"""Check isolated bootstrap selection without installing or changing a toolchain."""
import importlib.util
import io
import json
import os
from pathlib import Path
import subprocess
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
        config = dict(self.config, schema="chrono-bootstrap/v1", projects=".chrono-harness/projects.json")
        (ci / "isolated.json").write_text(json.dumps(config))
        (ci.parent / "projects.json").write_text('{"projects":[],"scripts":[]}')
        failure = subprocess.CalledProcessError(17, ["rustup", "toolchain", "install"])
        with mock.patch.object(bootstrap.sys, "argv", ["bootstrap.py", str(self.root), ".chrono-harness/ci/isolated.json"]), \
                mock.patch.object(bootstrap, "participate"), \
                mock.patch.object(bootstrap, "observe_source"), \
                mock.patch.object(bootstrap, "toolchain_environment", side_effect=failure), \
                mock.patch.object(bootstrap.sys, "stderr", io.StringIO()):
            with self.assertRaises(SystemExit) as raised:
                bootstrap.main()
        self.assertEqual(raised.exception.code, 17)

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


if __name__ == "__main__":
    unittest.main()
