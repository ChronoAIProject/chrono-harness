"""Check isolated bootstrap selection without installing or changing a toolchain."""
import importlib.util
import io
import json
import os
from pathlib import Path
import runpy
import subprocess
import shutil
import sys
import tempfile
import unittest
from unittest import mock


SOURCE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("bootstrap", SOURCE / "bootstrap.py")
bootstrap = importlib.util.module_from_spec(spec)
spec.loader.exec_module(bootstrap)


class IsolatedProvision(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory(prefix="isolated bootstrap ")
        self.addCleanup(directory.cleanup)
        self.root = Path(directory.name).resolve()
        self.config = {
            "rust_toolchain": "1.95.0", "tools": {},
            "toolchain_provision": {
                "profile": "default",
                "rustup_home": ".chrono-harness/state/private-rust/rustup",
                "cargo_home": ".chrono-harness/state/private-rust/cargo",
            },
        }

    def test_default_profile_uses_private_homes_for_every_child(self):
        original = dict(os.environ)
        calls = []

        def run(argv, **kwargs):
            calls.append((argv, kwargs))
            return subprocess.CompletedProcess(argv, 0, b"", b"")

        with mock.patch.object(bootstrap.subprocess, "run", side_effect=run), \
                mock.patch.object(bootstrap, "declared_bindings", return_value=None), \
                mock.patch.object(bootstrap.subprocess, "check_output", side_effect=[
                    "cargo 1.95.0 (fixture)\n", "rustc 1.95.0 (fixture)\n"]):
            env, versions, bindings = bootstrap.toolchain_environment(self.config, self.root)
        self.assertEqual(calls[0][0], ["rustup", "toolchain", "install", "1.95.0",
                                     "--profile", "default", "--no-self-update"])
        self.assertEqual(versions["rustc"], "rustc 1.95.0 (fixture)")
        self.assertIsNone(bindings)
        for _, kwargs in calls:
            self.assertEqual(kwargs["env"]["RUSTUP_HOME"],
                             str(self.root / self.config["toolchain_provision"]["rustup_home"]))
            self.assertEqual(kwargs["env"]["CARGO_HOME"],
                             str(self.root / self.config["toolchain_provision"]["cargo_home"]))
        self.assertEqual(env["RUSTUP_HOME"], calls[0][1]["env"]["RUSTUP_HOME"])
        self.assertEqual(dict(os.environ), original, "selection changed the worker environment")

    def test_source_bootstrap_selects_its_environment_before_private_production(self):
        values = {"CARGO_HOME": "/source/cargo", "RUSTUP_HOME": "/source/rustup",
                  "RUSTC": "/source/rustc", "RUSTFLAGS": "--sysroot=/source/sysroot"}
        original = dict(os.environ)
        env = bootstrap.provision_environment(self.root, {"environment": values})
        for key, value in values.items():
            self.assertEqual(env[key], value)
        self.assertEqual(dict(os.environ), original)
        private = bootstrap.provision_environment(self.root, dict(self.config, environment=values))
        self.assertEqual(private["RUSTUP_HOME"],
                         str(self.root / self.config["toolchain_provision"]["rustup_home"]))
        self.assertEqual(private["RUSTC"], "")

    def test_invalid_bootstrap_environment_fails_before_installer_effects(self):
        for values in [None, [], {"": "x"}, {"bad=key": "x"}, {"RUSTC": None},
                       {"CHRONO_PROCESS_FDS": "7"}, {"RUSTC": "bad\0value"}]:
            with self.subTest(values=values), mock.patch.object(bootstrap.subprocess, "run") as run:
                with self.assertRaisesRegex(ValueError, "environment selection"):
                    bootstrap.toolchain_environment(dict(self.config, environment=values), self.root)
                run.assert_not_called()

    def test_host_home_rebinding_preserves_the_selected_rustup_installation(self):
        config = {"host_environment": {"HOME": ".chrono-harness/cache/private-home",
                                      "CARGO_HOME": ".chrono-harness/cache/private-cargo"}}
        for inherited, expected in [({"HOME": "/worker"}, "/worker/.rustup"),
                                    ({"HOME": "/worker", "RUSTUP_HOME": "/sdk/rustup"}, "/sdk/rustup")]:
            with self.subTest(inherited=inherited), mock.patch.dict(os.environ, inherited, clear=True):
                original = dict(os.environ)
                env = bootstrap.provision_environment(self.root, config)
                self.assertEqual(env["RUSTUP_HOME"], expected)
                self.assertEqual(env["HOME"], str(self.root / config["host_environment"]["HOME"]))
                self.assertEqual(env["CARGO_HOME"], str(self.root / config["host_environment"]["CARGO_HOME"]))
                self.assertEqual(dict(os.environ), original)
                self.assertFalse((self.root / ".chrono-harness").exists())
        with mock.patch.dict(os.environ, {"HOME": "/worker"}, clear=True):
            explicit = dict(config, environment={"RUSTUP_HOME": "/selected/rustup"})
            self.assertEqual(bootstrap.provision_environment(self.root, explicit)["RUSTUP_HOME"], "/selected/rustup")
            private = bootstrap.provision_environment(self.root, dict(self.config, **config))
            self.assertEqual(private["RUSTUP_HOME"], str(self.root / self.config["toolchain_provision"]["rustup_home"]))

    def test_install_failure_stops_before_selection_and_preserves_exit(self):
        failure = subprocess.CalledProcessError(17, ["rustup", "toolchain", "install"])
        with mock.patch.object(bootstrap.subprocess, "run", side_effect=failure), \
                mock.patch.object(bootstrap, "declared_bindings") as select:
            with self.assertRaises(subprocess.CalledProcessError) as raised:
                bootstrap.toolchain_environment(self.config, self.root)
        self.assertEqual(raised.exception.returncode, 17)
        select.assert_not_called()

    def test_entry_preserves_the_isolated_installers_nonzero_exit(self):
        ci = self.root / ".chrono-harness/ci"
        ci.mkdir(parents=True)
        config = dict(self.config, schema="chrono-bootstrap/v1", projects=".chrono-harness/projects.json",
                      entrypoint={"tool": "python3", "script": ".chrono-harness/ci/bootstrap.py"},
                      tools={"python3": sys.executable}, operations=["fixture.after-install"],
                      install=[{"source": ".chrono-harness/ci/bootstrap.py",
                                "destination": ".chrono-harness/bin/fixture"}])
        (ci / "isolated.json").write_text(json.dumps(config))
        (ci.parent / "projects.json").write_text(json.dumps({"projects": [], "scripts": [{
            "actions": {"execute": {"operation": "fixture.after-install", "tool": "python3",
                                    "argv": ["--version"]}}}]}))
        shutil.copyfile(SOURCE / "bootstrap.py", ci / "bootstrap.py")
        failure = subprocess.CalledProcessError(17, ["rustup", "toolchain", "install"])
        with mock.patch.object(bootstrap.sys, "argv", ["bootstrap.py", str(self.root), ".chrono-harness/ci/isolated.json"]), \
                mock.patch.object(bootstrap.subprocess, "run", side_effect=failure) as installer, \
                mock.patch.object(bootstrap.subprocess, "check_output", side_effect=
                                  subprocess.CalledProcessError(128, ["git"], stderr=b"fixture host")), \
                mock.patch.object(bootstrap.subprocess, "Popen", side_effect=
                                  AssertionError("operation ran after installer failure")) as operation, \
                mock.patch.object(bootstrap.shutil, "copy2", side_effect=
                                  AssertionError("installation ran after installer failure")) as copy, \
                mock.patch.object(bootstrap.sys, "stderr", io.StringIO()):
            with self.assertRaises(SystemExit) as raised:
                runpy.run_path(str(SOURCE / "bootstrap.py"), run_name="__main__")
        self.assertEqual(raised.exception.code, 17)
        installer.assert_called_once()
        self.assertEqual(installer.call_args[0][0], ["rustup", "toolchain", "install", "1.95.0",
                                                   "--profile", "default", "--no-self-update"])
        for key, field in [("RUSTUP_HOME", "rustup_home"), ("CARGO_HOME", "cargo_home")]:
            self.assertEqual(installer.call_args[1]["env"][key],
                             str(self.root / config["toolchain_provision"][field]))
        operation.assert_not_called()
        copy.assert_not_called()
        reports = list((self.root / ".chrono-harness/state").glob("bootstrap-*/report.json"))
        self.assertEqual(len(reports), 1)
        report = json.loads(reports[0].read_text())
        self.assertEqual(report["status"], "failed")
        self.assertEqual(report["error"], str(failure))
        self.assertEqual(report["operations"], [])
        self.assertEqual(report["installed"], [])
        self.assertEqual(report["versions"], {})
        self.assertEqual(report["environment"], {})
        self.assertFalse((self.root / ".chrono-harness/state/bootstrap.json").exists())

    def test_invalid_or_overlapping_homes_fail_before_any_child(self):
        for update in [
            {"profile": "complete"}, {"rustup_home": "/tmp/global-rust"},
            {"rustup_home": ".chrono-harness/state/../global"},
            {"cargo_home": self.config["toolchain_provision"]["rustup_home"]},
            {"cargo_home": self.config["toolchain_provision"]["rustup_home"] + "/cargo"},
            {"extra": True},
        ]:
            with self.subTest(update=update), mock.patch.object(bootstrap.subprocess, "run") as run:
                config = dict(self.config, toolchain_provision={
                    **self.config["toolchain_provision"], **update})
                with self.assertRaises(ValueError):
                    bootstrap.toolchain_environment(config, self.root)
                run.assert_not_called()

    def test_symlink_home_fails_before_any_child(self):
        state = self.root / ".chrono-harness/state"
        state.mkdir(parents=True)
        (state / "private-rust").symlink_to(self.root, target_is_directory=True)
        with mock.patch.object(bootstrap.subprocess, "run") as run:
            with self.assertRaises(ValueError):
                bootstrap.toolchain_environment(self.config, self.root)
            run.assert_not_called()

    def test_malformed_selection_fails_before_any_child(self):
        for declaration in [None, {}, [], {**self.config["toolchain_provision"], "cargo_home": None}]:
            with self.subTest(declaration=declaration), mock.patch.object(bootstrap.subprocess, "run") as run:
                with self.assertRaises(ValueError):
                    bootstrap.toolchain_environment(dict(self.config, toolchain_provision=declaration), self.root)
                run.assert_not_called()

    def test_failed_default_probe_does_not_fall_back_or_retry_install(self):
        for probe in (1, 2):
            calls = []

            def run(argv, **kwargs):
                calls.append(argv)
                return subprocess.CompletedProcess(argv, 19 if len(calls) == probe + 1 else 0)

            with self.subTest(probe=probe), mock.patch.object(bootstrap.subprocess, "run", side_effect=run), \
                    mock.patch.object(bootstrap, "declared_bindings") as select:
                with self.assertRaisesRegex(ValueError, "probe failed: 19"):
                    bootstrap.toolchain_environment(self.config, self.root)
                self.assertEqual(sum(argv[1:3] == ["toolchain", "install"] for argv in calls), 1)
                select.assert_not_called()


class NativeCargoDomains(unittest.TestCase):
    """Host selection expectations; the fixture is not Cargo/native acceptance.

    Ordinary native consumers use their registered locked Cargo domain.
    """

    def setUp(self):
        directory = tempfile.TemporaryDirectory(prefix="native cargo domains ")
        self.addCleanup(directory.cleanup)
        self.root = Path(directory.name).resolve()
        self.originals = self.root / "originals"
        self.originals.mkdir()
        self.index = 0
        self.actions = bootstrap.registered_operations(
            json.loads((SOURCE.parent / "projects.json").read_text()))
        self.base_environment = {"PATH": os.environ["PATH"], "HOME": "/Users/runner",
                                 "CARGO_HOME": "/Users/runner/.cargo",
                                 "FIXTURE_DOMAIN_ROOT": str(self.root / "domains")}

    def native_config(self, entry):
        # This test supplies the already registered native identity as input;
        # production selection/drift validation remains the Rust owner's check.
        selector = json.loads((SOURCE.parents[1] / entry).read_text())
        rows = selector["platforms"]["macos-aarch64"]["configurations"]
        expected_path = ".chrono-harness/git/pinned-macos.json"
        selected = [row for row in rows if row["path"] == expected_path]
        self.assertEqual(len(selected), 1)
        return json.loads((SOURCE.parents[1] / selected[0]["path"]).read_text())

    def selected_environment(self, values, inherited=None, bootstrap_profile=False):
        inherited = inherited or self.base_environment
        if bootstrap_profile:
            with mock.patch.dict(os.environ, inherited, clear=True):
                return bootstrap.provision_environment(self.root, {"environment": values})
        # Supply the registered guard/observer values to the external interface
        # fixture. This is host contract data, not a Python replacement for the
        # Rust registration/environment judge.
        return dict(inherited, **values)

    def cargo(self, operation, environment, mode="build"):
        action = self.actions[operation]
        manifest = action["argv"][action["argv"].index("--manifest-path") + 1]
        for name in (manifest, str(Path(manifest).with_name("Cargo.lock"))):
            target = self.root / name
            target.parent.mkdir(parents=True, exist_ok=True)
            if not target.exists():
                shutil.copyfile(SOURCE.parents[1] / name, target)
        fixture = {"tool": "python3", "argv": [
            str(SOURCE / "fixtures/bootstrap-operation.py"), "cargo-domain", mode, manifest]}
        result = bootstrap.run_operation(self.root, self.originals, self.index,
                                         operation, fixture, sys.executable, environment)
        self.index += 1
        return result

    def test_ordinary_native_consumers_keep_the_ordinary_locked_domain(self):
        config = self.native_config(".chrono-harness/config.json")
        self.assertNotIn("CARGO_NET_OFFLINE", config["environment"]["inherit"])
        environment = self.selected_environment(config["environment"]["values"])
        self.assertEqual(environment["CARGO_HOME"], "/Users/runner/.cargo")
        self.assertNotEqual(environment.get("CARGO_NET_OFFLINE"), "true")
        # The five distinct locked manifests in the 21 original failing checks.
        for operation in ("build.runner", "build.cache", "build.distribution",
                          "build.instructions", "build.judge-routes"):
            with self.subTest(operation=operation):
                result = self.cargo(operation, environment)
                self.assertEqual(result["exit_code"], 0)
                observed = json.loads((self.root / result["stdout"]["path"]).read_bytes())
                self.assertEqual(observed["cargo_home"], "/Users/runner/.cargo")
                self.assertFalse(observed["offline"])
        # The terminal managed originals identify this same ordinary cache
        # prerequisite inside RealFixtureStartup; retain its registered profile.
        cache = json.loads((SOURCE / "bootstrap-cache.json").read_text())
        self.assertEqual(cache["operations"], ["build.cache"])
        child_environment = self.selected_environment(cache.get("environment", {}), environment,
                                                      bootstrap_profile=True)
        self.assertEqual(self.cargo(cache["operations"][0], child_environment)["exit_code"], 0)
        self.assertEqual(child_environment["CARGO_HOME"], "/Users/runner/.cargo")
        self.assertNotEqual(child_environment.get("CARGO_NET_OFFLINE"), "true")




if __name__ == "__main__":
    unittest.main()
