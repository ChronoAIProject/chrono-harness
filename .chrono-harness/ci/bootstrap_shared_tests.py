#!/usr/bin/env python3
"""Transport real bootstrap fixture outputs, then exercise consumer rejection."""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time
import unittest

sys.dont_write_bytecode = True
SOURCE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("bootstrap_fixture", SOURCE / "bootstrap-tests.py")
fixture = importlib.util.module_from_spec(spec)
spec.loader.exec_module(fixture)
CONFIG = ".chrono-harness/ci/bootstrap-shared.json"


class SharedStartup(unittest.TestCase):
    def setUp(self):
        self.host = fixture.Bootstrap()
        self.host.setUp()
        self.addCleanup(self.host.doCleanups)
        self.root = self.host.root
        shutil.copyfile(SOURCE / "bootstrap_shared.py", self.host.ci / "bootstrap_shared.py")
        self.host.config["install"].append({"source": "built-second", "destination": ".chrono-harness/bin/second"})
        self.host.write_json(self.host.ci / "bootstrap.json", self.host.config)
        for name in ["built-tool", "built-second"]:
            path = self.root / name
            path.write_bytes(b"second")
            path.chmod(0o755)
        self.shared = {"schema": "chrono-host-startup/v1", "directory": ".chrono-harness/startup/",
                       "profiles": {"core": {"config": ".chrono-harness/ci/bootstrap.json", "build": True}},
                       "consumers": {"unit_fixture": ["core"]}}
        self.host.write_json(self.root / CONFIG, self.shared)
        (self.root / ".gitignore").write_text("/built-*\n/.chrono-harness/bin/\n/.chrono-harness/state/\n/.chrono-harness/startup/\n/mock-tools/\n")
        (self.root / "product-source").write_text("original")
        self.host.git("init", "-q")
        self.host.git("config", "user.email", "fixture@example.invalid")
        self.host.git("config", "user.name", "Fixture")
        self.host.git("add", ".")
        self.host.git("commit", "-qm", "source")
        self.environment = self.host.environment
        self.output = self.host.tools / "github-output"
        self.output.touch()
        self.environment["GITHUB_OUTPUT"] = str(self.output)
        self.transfer = self.root / self.shared["directory"]

    def invoke(self, command, *args, root=None):
        root = root or self.root
        return subprocess.run([sys.executable, "-B", str(root / ".chrono-harness/ci/bootstrap_shared.py"),
                               str(root), CONFIG, command, *args], env=self.environment,
                              pass_fds=fixture.PASS_FDS, capture_output=True)

    def produce(self):
        self.host.succeeded(self.invoke("produce"))
        self.environment["CHRONO_STARTUP_BINDING"] = self.output.read_text().strip().split("=", 1)[1]
        self.assertEqual(self.environment["CHRONO_STARTUP_BINDING"],
                         hashlib.sha256((self.transfer / "manifest.json").read_bytes()).hexdigest())

    def clone_consumer(self):
        directory = tempfile.TemporaryDirectory(prefix="startup consumer ")
        self.addCleanup(directory.cleanup)
        root = Path(directory.name).resolve() / "host"
        subprocess.run(["git", "clone", "-q", str(self.root), str(root)], check=True,
                       capture_output=True, pass_fds=fixture.PASS_FDS)
        shutil.copytree(self.transfer, root / self.shared["directory"])
        return root

    def test_import_validates_originals_without_building_and_verify_detects_installed_drift(self):
        self.produce()
        original = self.host.state_path.read_bytes()
        root = self.clone_consumer()
        for path in (root / self.shared["directory"] / "tools").iterdir():
            path.chmod(0o644)  # Native artifact transfer does not preserve execute bits.
        self.host.succeeded(self.invoke("install", "unit_fixture", root=root))
        for name, expected in [("tool", b"binary"), ("second", b"second")]:
            path = root / ".chrono-harness/bin" / name
            self.assertEqual(path.read_bytes(), expected)
            self.assertEqual(path.stat().st_mode & 0o777, 0o755)
        self.assertFalse((root / "built-tool").exists(), "consumer ran a producer operation")
        result = json.loads((root / ".chrono-harness/state/bootstrap.json").read_bytes())
        self.assertEqual(result["provenance"]["kind"], "shared-startup")
        self.assertEqual(result["provenance"]["report_sha256"], hashlib.sha256(original).hexdigest())
        self.host.succeeded(self.invoke("verify", "core", root=root))
        (root / ".chrono-harness/bin/tool").write_bytes(b"changed")
        failed = self.invoke("verify", "core", root=root)
        self.assertNotEqual(failed.returncode, 0)
        self.assertIn(b"installed startup tool differs", failed.stderr)

    def test_bad_binding_source_platform_profile_or_late_asset_never_partially_installs(self):
        self.produce()
        for defect in ["binding", "missing-binding", "dirty", "source", "platform", "profile", "late-asset", "symlink"]:
            with self.subTest(defect=defect):
                root = self.clone_consumer()
                transfer = root / self.shared["directory"]
                manifest_path = transfer / "manifest.json"
                manifest = json.loads(manifest_path.read_bytes())
                binding = self.environment["CHRONO_STARTUP_BINDING"]
                if defect == "binding": self.environment["CHRONO_STARTUP_BINDING"] = "0" * 64
                elif defect == "missing-binding": self.environment.pop("CHRONO_STARTUP_BINDING")
                elif defect == "dirty": (root / "product-source").write_text("different")
                elif defect == "source":
                    subprocess.run(["git", "-C", str(root), "-c", "user.name=Fixture", "-c", "user.email=f@example.invalid", "commit", "--allow-empty", "-qm", "other"], check=True, pass_fds=fixture.PASS_FDS)
                elif defect == "profile": (root / ".chrono-harness/ci/bootstrap.json").write_text("{}")
                elif defect == "platform":
                    manifest["platform"] = {"system": "different", "machine": "different"}
                    manifest_path.write_text(json.dumps(manifest))
                    self.environment["CHRONO_STARTUP_BINDING"] = hashlib.sha256(manifest_path.read_bytes()).hexdigest()
                elif defect == "late-asset": (transfer / "tools/1").write_bytes(b"damaged")
                elif defect == "symlink":
                    (transfer / "tools/1").unlink()
                    (transfer / "tools/1").symlink_to(root / "product-source")
                target = root / ".chrono-harness/bin/tool"
                target.parent.mkdir()
                target.write_bytes(b"keep")
                result = self.invoke("install", "unit_fixture", root=root)
                self.assertNotEqual(result.returncode, 0, result.stdout)
                self.assertIn(b"E_STARTUP:", result.stderr)
                self.assertEqual(target.read_bytes(), b"keep")
                self.assertFalse((root / ".chrono-harness/state/startup-install.json").exists())
                self.environment["CHRONO_STARTUP_BINDING"] = binding

    def test_publication_failure_and_unknown_consumer_are_not_success(self):
        self.environment["GITHUB_OUTPUT"] = str(self.host.tools)
        result = self.invoke("produce")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"E_STARTUP:", result.stderr)
        self.assertEqual(self.output.read_bytes(), b"")
        self.environment["CHRONO_STARTUP_BINDING"] = hashlib.sha256((self.transfer / "manifest.json").read_bytes()).hexdigest()
        root = self.clone_consumer()
        result = self.invoke("install", "unknown", root=root)
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse((root / ".chrono-harness/bin").exists())
        state = root / ".chrono-harness/state"
        state.mkdir()
        (state / "startup-install.json").mkdir()
        self.assertNotEqual(self.invoke("install", "unit_fixture", root=root).returncode, 0)

    def test_dirty_producer_and_existing_transfer_are_rejected(self):
        (self.root / "product-source").write_text("dirty")
        failed = self.invoke("produce")
        self.assertNotEqual(failed.returncode, 0)
        self.assertFalse(self.transfer.exists())
        self.host.git("restore", "product-source")
        self.produce()
        original = (self.transfer / "manifest.json").read_bytes()
        self.assertNotEqual(self.invoke("produce").returncode, 0)
        self.assertEqual((self.transfer / "manifest.json").read_bytes(), original)

    def test_shared_install_and_verify_recheck_executable_binding(self):
        bindings = self.bind_tools()
        self.produce()
        root = self.clone_consumer()
        self.host.succeeded(self.invoke("install", "unit_fixture", root=root))
        self.host.succeeded(self.invoke("verify", "core", root=root))
        report = json.loads((root / ".chrono-harness/state/bootstrap.json").read_bytes())
        self.assertEqual([row["sha256"] for row in report["executables"]],
                         [bindings[name]["sha256"] for name in ["cargo", "rustc"]])

    def bind_tools(self, relocate=False):
        bindings = {}
        for tool in ["cargo", "rustc"]:
            path = self.host.tools / tool
            bindings[tool] = {"path": str(path),
                              "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
                              "version": tool + " 1.95.0 (fixture)", "allow_relocation": relocate}
        self.host.config["tools"].update({"cargo": str(self.host.tools / "cargo"),
                                           "rustc": str(self.host.tools / "rustc")})
        self.host.config["tool_bindings"] = bindings
        self.host.write_json(self.host.ci / "bootstrap.json", self.host.config)
        self.host.git("add", ".chrono-harness/ci/bootstrap.json")
        self.host.git("commit", "-qm", "bind bootstrap executables")
        return bindings

    def test_equivalent_relocation_preserves_producer_and_consumer_paths(self):
        self.bind_tools(relocate=True)
        self.produce()
        original = self.host.state_path.read_bytes()
        root = self.clone_consumer()
        local = root / "mock-tools"
        shutil.copytree(self.host.tools, local)
        self.environment["PATH"] = str(local) + os.pathsep + os.environ["PATH"]
        self.host.succeeded(self.invoke("install", "unit_fixture", root=root))
        self.host.succeeded(self.invoke("verify", "core", root=root))
        retained = (root / ".chrono-harness/state/startup-originals/core.json").read_bytes()
        self.assertEqual(retained, original)
        report = json.loads((root / ".chrono-harness/state/startup-install.json").read_bytes())
        self.assertEqual([row["path"] for row in report["local_executables"]["core"]],
                         [str(local / name) for name in ("cargo", "rustc")])
        self.assertNotEqual(json.loads(retained)["executables"][0]["path"], str(local / "cargo"))

    def test_missing_altered_or_contradictory_execution_originals_reject_before_install(self):
        self.bind_tools()
        self.produce()
        for defect in ["executables", "selection", "compiler", "operations", "argv", "exit", "stream", "stream-missing"]:
            with self.subTest(defect=defect):
                root = self.clone_consumer()
                transfer = root / self.shared["directory"]
                manifest_path = transfer / "manifest.json"
                manifest = json.loads(manifest_path.read_bytes())
                original_path = transfer / manifest["profiles"]["core"]["report"]["path"]
                report = json.loads(original_path.read_bytes())
                if defect in ("executables", "selection", "operations"):
                    del report[defect]
                elif defect == "compiler": report["environment"]["RUSTC"] = "/different/rustc"
                elif defect == "argv": report["operations"][0]["argv"][-1] = "different"
                elif defect == "exit": report["operations"][0]["exit_code"] = 9
                elif defect == "stream": (transfer / report["operations"][0]["stdout"]["path"]).write_bytes(b"altered")
                elif defect == "stream-missing": (transfer / report["operations"][0]["stderr"]["path"]).unlink()
                original_path.write_text(json.dumps(report))
                manifest["profiles"]["core"]["report"]["sha256"] = hashlib.sha256(original_path.read_bytes()).hexdigest()
                manifest_path.write_text(json.dumps(manifest))
                self.environment["CHRONO_STARTUP_BINDING"] = hashlib.sha256(manifest_path.read_bytes()).hexdigest()
                target = root / ".chrono-harness/bin/tool"
                target.parent.mkdir()
                target.write_bytes(b"keep")
                rejected = self.invoke("install", "unit_fixture", root=root)
                self.assertNotEqual(rejected.returncode, 0, rejected.stdout)
                self.assertIn(b"E_STARTUP:", rejected.stderr)
                self.assertEqual(target.read_bytes(), b"keep")
                self.assertFalse((root / ".chrono-harness/state/startup-install.json").exists())

    def test_same_version_different_local_bytes_reject_install_and_verify(self):
        self.bind_tools(relocate=True)
        self.produce()
        root = self.clone_consumer()
        self.host.succeeded(self.invoke("install", "unit_fixture", root=root))
        fresh = self.clone_consumer()
        target = fresh / ".chrono-harness/bin/tool"
        target.parent.mkdir()
        target.write_bytes(b"keep")
        cargo = self.host.tools / "cargo"
        cargo.write_bytes(cargo.read_bytes() + b"\n# same version, changed executable\n")
        for command, arg, consumer in [("install", "unit_fixture", fresh), ("verify", "core", root)]:
            result = self.invoke(command, arg, root=consumer)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn(b"bound cargo identity mismatch", result.stderr)
        self.assertEqual(target.read_bytes(), b"keep")


class RealSharedStartup(unittest.TestCase):
    """Explicit native acceptance mode; the ordinary fixture suite stays small."""

    def test_registered_core_shared_and_runner_consumer(self):
        source_root = SOURCE.parents[1]
        sys.path.insert(0, str(SOURCE))
        import bootstrap
        observed = bootstrap.observe_source(source_root)
        self.assertIsNone(observed["error"])
        self.assertIs(observed["dirty"], False, "real consumption requires committed clean candidate source")
        evidence_owner = source_root / ".chrono-harness/state/bootstrap-real"
        evidence_owner.mkdir(parents=True, exist_ok=True)
        evidence = Path(tempfile.mkdtemp(prefix="attempt-", dir=evidence_owner))
        root = evidence / "fixture"
        environment = dict(os.environ)
        for key in ("CHRONO_WORKTREE_BOOTSTRAP", "CHRONO_WORKTREE_CHECK", "CHRONO_CHECK_SOURCE"):
            environment.pop(key, None)
        environment["GITHUB_OUTPUT"] = str(evidence / "github-output")
        filemap = json.loads((source_root / ".chrono-harness/FILEMAP.json").read_bytes())
        deadline = time.monotonic() + filemap["execution_plans"]["test:host-bootstrap-tests"]["timeout_seconds"]
        host = json.loads((source_root / ".chrono-harness/config.json").read_bytes())
        tools = {row["id"]: row["program"] for row in host["tools"]}
        commands = []

        def run(name, argv, cwd):
            stdout, stderr = evidence / (name + ".stdout"), evidence / (name + ".stderr")
            started = time.monotonic()
            with stdout.open("xb") as out, stderr.open("xb") as err:
                process = subprocess.run(argv, cwd=cwd, env=environment, pass_fds=fixture.PASS_FDS,
                                         stdout=out, stderr=err, timeout=max(0.001, deadline - started))
            commands.append({"name": name, "argv": argv, "cwd": str(cwd), "exit_code": process.returncode,
                             "duration_seconds": time.monotonic() - started,
                             "stdout": str(stdout), "stderr": str(stderr)})
            (evidence / "commands.json").write_text(json.dumps(commands, indent=2) + "\n")
            self.assertEqual(process.returncode, 0, name + ": " + stderr.read_text(errors="replace")[-3000:])

        print("real bootstrap consumer originals: " + str(evidence), flush=True)
        run("clone", [tools["git"], "clone", "--no-local", "--quiet", str(source_root), str(root)], source_root)
        self.assertEqual(bootstrap.observe_source(root), observed)
        registry = json.loads((root / ".chrono-harness/projects.json").read_bytes())
        actions = bootstrap.registered_operations(registry)

        def registered(operation):
            action = actions[operation]
            run(operation, [tools[action["tool"]], *action["argv"]], root)

        registered("host.bootstrap-cache")
        registered("host.bootstrap-shared-produce")
        binding = (evidence / "github-output").read_text().strip().split("=", 1)[1]
        environment["CHRONO_STARTUP_BINDING"] = binding
        registered("host.bootstrap-shared-install-unit_runner")
        units = json.loads((root / ".chrono-harness/ci/units.json").read_bytes())
        run("runner-startup", units["units"]["runner"]["bootstrap"], root)
        run("runner-check", [*host["canonical_check"]["argv"], "--unit", "runner"], root)

        def original(descriptor):
            raw = (root / descriptor["path"]).read_bytes()
            self.assertEqual(hashlib.sha256(raw).hexdigest(), descriptor["sha256"])
            return raw

        check_config = json.loads((root / ".chrono-harness/ci/check.json").read_bytes())
        reference = json.loads((root / check_config["policy"]["units"]["runner"]["report_path"]).read_bytes())
        report = json.loads(original(reference["original"]))
        self.assertEqual(report["judge"]["exit_code"], 0)
        self.assertIsNone(report["judge"]["failure"])
        response = json.loads(original(report["judge"]["stdout_original"]))
        original(report["judge"]["stderr_original"])
        self.assertEqual(response["status"], "passed")
        self.assertEqual(response["evidence"]["candidate"], observed["commit"])
        runner = [row for row in response["evidence"]["executed"] if row["operation"] == "test.runner-tests"]
        self.assertEqual(len(runner), 1, "startup alone cannot satisfy the later independent Cargo action")
        self.assertEqual(runner[0]["process"]["exit_code"], 0)
        self.assertIsNone(runner[0]["process"]["failure"])
        self.assertIn("test result: ok", runner[0]["process"]["stdout"])
        # Retain exact producer/report/process bytes after every child has joined.
        # Fixture source/build caches have no remaining consumer on success.
        shutil.copytree(root / ".chrono-harness/state", evidence / "state")
        shutil.copytree(root / ".chrono-harness/startup", evidence / "startup")
        result = {"source": observed, "binding": binding, "commands": commands,
                  "fixture": str(root), "retained_state": str(evidence / "state"),
                  "retained_startup": str(evidence / "startup")}
        (evidence / "result.json").write_text(json.dumps(result, indent=2) + "\n")
        shutil.rmtree(root)


if __name__ == "__main__":
    unittest.main(defaultTest="SharedStartup")
