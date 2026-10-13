#!/usr/bin/env python3
"""Transport real bootstrap fixture outputs, then exercise consumer rejection."""
import hashlib
import importlib.util
import json
import os
import platform
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

    def test_shared_relocation_with_absent_and_stale_original_paths(self):
        self.bind_tools(relocate=True)
        self.produce()
        root = self.clone_consumer()
        local = root / "mock-tools"
        shutil.copytree(self.host.tools, local)
        self.environment["PATH"] = str(local) + os.pathsep + os.environ["PATH"]
        for tool in ("cargo", "rustc"):
            (self.host.tools / tool).write_bytes(b"retired non-executable bytes")
        self.host.succeeded(self.invoke("install", "unit_fixture", root=root))
        for tool in ("cargo", "rustc"):
            (self.host.tools / tool).unlink()
        self.host.succeeded(self.invoke("verify", "core", root=root))
        # A current effective compiler conflict still fails before installation.
        fresh = self.clone_consumer()
        self.environment["RUSTC"] = str(self.host.tools / "rustc")
        rejected = self.invoke("install", "unit_fixture", root=fresh)
        self.assertNotEqual(rejected.returncode, 0)
        self.assertIn(b"inherited compiler selection differs", rejected.stderr)
        self.assertFalse((fresh / ".chrono-harness/bin").exists())

    def test_native_shared_registry_binds_original_report_and_preflight(self):
        bindings = self.bind_tools(relocate=True)
        path = ".chrono-harness/ci/native.json"
        data = {"schema": "chrono-bootstrap-toolchains/v1", "rust_toolchain": "1.95.0",
                "platforms": {platform.system() + "/" + platform.machine(): {
                    "target": "fixture", "tools": {name: {"sha256": item["sha256"], "version": item["version"]}
                                                    for name, item in bindings.items()}}}}
        self.host.write_json(self.root / path, data)
        del self.host.config["tool_bindings"]
        self.host.config["tools"].pop("cargo")
        self.host.config["tools"].pop("rustc")
        self.host.config.update(tool_bindings_registry=path, require_tool_bindings=True)
        self.host.write_json(self.host.ci / "bootstrap.json", self.host.config)
        self.host.git("add", ".chrono-harness/ci")
        self.host.git("commit", "-qm", "native expectations")
        self.produce()
        root = self.clone_consumer()
        original = json.loads((self.transfer / "reports/core.json").read_bytes())
        self.assertEqual(original["registrations"][path], hashlib.sha256((root / path).read_bytes()).hexdigest())
        self.host.succeeded(self.invoke("install", "unit_fixture", root=root))
        self.host.succeeded(self.invoke("verify", "core", root=root))
        fresh = self.clone_consumer()
        transfer = fresh / self.shared["directory"]
        original_path = transfer / "reports/core.json"
        original["registrations"].pop(path)
        original_path.write_text(json.dumps(original))
        manifest_path = transfer / "manifest.json"
        manifest = json.loads(manifest_path.read_bytes())
        manifest["profiles"]["core"]["report"]["sha256"] = hashlib.sha256(original_path.read_bytes()).hexdigest()
        manifest_path.write_text(json.dumps(manifest))
        self.environment["CHRONO_STARTUP_BINDING"] = hashlib.sha256(manifest_path.read_bytes()).hexdigest()
        rejected = self.invoke("install", "unit_fixture", root=fresh)
        self.assertNotEqual(rejected.returncode, 0)
        self.assertIn(b"startup execution registrations differ", rejected.stderr)
        self.assertFalse((fresh / ".chrono-harness/bin").exists())

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


class RealAttempt:
    """One producer-owned attempt; its result is evidence, never a cleanup ledger.

    The lifecycle owner currently exposes retained-producer admission only as a
    Rust API. Until a registered Python-callable route exists, keep the original
    fixture and refuse another attempt in this slot, including after interruption.
    """

    def __init__(self, owner, source, deadline, output_limit):
        owner.mkdir(parents=True, exist_ok=True)
        self.evidence = owner / "pending-attempt"
        try:
            self.evidence.mkdir()
        except FileExistsError:
            raise ValueError("unresolved real-consumer attempt; protected custody: " + str(self.evidence))
        self.deadline, self.output_limit = deadline, output_limit
        self.result = {"source": source, "fixture": str(self.evidence / "fixture"),
                       "status": "running", "commands": [],
                       "custody": {"status": "protected-pending", "descendants_quiescent": None,
                                   "disposal": "not-requested",
                                   "reason": "RetainedArtifact begin/seal/release has no registered Python-callable lifecycle operation; no disposal ownership established.",
                                   "consumers": ["real-consumer assertions", "failure diagnosis", "caller acceptance and review"],
                                   "capacity": "one unresolved attempt; no automatic retry or age deletion"}}
        self.publish()

    def publish(self):
        path = self.evidence / "result.json"
        temporary = path.with_suffix(".json.tmp")
        temporary.write_text(json.dumps(self.result, indent=2) + "\n")
        os.replace(temporary, path)

    def __enter__(self):
        return self

    def __exit__(self, kind, error, traceback):
        self.finish("passed" if kind is None else "failed", None if error is None else repr(error))

    def finish(self, status, error=None):
        self.result["status"] = status
        if error is not None:
            self.result["error"] = error
        self.publish()

    def run(self, name, argv, cwd, environment):
        if any(row["name"] == name for row in self.result["commands"]):
            raise ValueError("duplicate real-consumer command")
        stdout, stderr = self.evidence / (name + ".stdout"), self.evidence / (name + ".stderr")
        row = {"name": name, "argv": argv, "cwd": str(cwd), "exit_code": None,
               "status": "running", "ownership_fds": list(fixture.PASS_FDS)}
        self.result["commands"].append(row)
        self.publish()
        started = time.monotonic()
        try:
            with stdout.open("xb") as out, stderr.open("xb") as err:
                process = subprocess.run(argv, cwd=cwd, env=environment, pass_fds=fixture.PASS_FDS,
                                         stdout=out, stderr=err, timeout=max(0.001, self.deadline - started))
            row["exit_code"] = process.returncode
            row["status"] = "passed" if process.returncode == 0 else "failed"
        except subprocess.TimeoutExpired:
            row["status"] = "timeout"
            raise
        except BaseException as error:
            row["status"] = "interrupted" if isinstance(error, (KeyboardInterrupt, SystemExit)) else "launch-error"
            row["error"] = repr(error)
            raise
        finally:
            row["duration_seconds"] = time.monotonic() - started
            for channel, path in (("stdout", stdout), ("stderr", stderr)):
                if path.is_file():
                    import bootstrap
                    row[channel] = {"path": str(path), "sha256": bootstrap.file_digest(path),
                                    "size": path.stat().st_size}
            self.publish()
        size = sum(command[channel]["size"] for command in self.result["commands"]
                   for channel in ("stdout", "stderr") if channel in command)
        if size > self.output_limit:
            raise AssertionError("real-consumer original streams exceed registered output bound")
        if process.returncode:
            raise AssertionError(name + ": " + stderr.read_text(errors="replace")[-3000:])


class AttemptCustody(unittest.TestCase):
    def test_terminal_success_failure_and_timeout_keep_originals_and_refuse_accumulation(self):
        for mode, code, status in [("fail", "0", "passed"), ("fail", "7", "failed"),
                                   ("timeout", "0", "timeout")]:
            with self.subTest(status=status), tempfile.TemporaryDirectory() as directory:
                owner = Path(directory) / "evidence"
                observed = {"commit": "a" * 40, "tree": "b" * 40, "dirty": False, "error": None}
                attempt = RealAttempt(owner, observed, time.monotonic() + (0.8 if status == "timeout" else 10), 1048576)
                argv = [sys.executable, "-B", str(SOURCE / "fixtures/bootstrap-operation.py"), mode, code]
                if status == "passed":
                    attempt.run("command", argv, Path(directory), dict(os.environ))
                    attempt.finish("passed")
                else:
                    with self.assertRaises((AssertionError, subprocess.TimeoutExpired)):
                        attempt.run("command", argv, Path(directory), dict(os.environ))
                    attempt.finish("failed")
                result = json.loads((attempt.evidence / "result.json").read_bytes())
                row = result["commands"][0]
                self.assertEqual(row["status"], status)
                self.assertEqual(row["exit_code"], None if status == "timeout" else int(code))
                self.assertEqual(row["ownership_fds"], list(fixture.PASS_FDS))
                for channel, ending in [("stdout", b"\x00\xff\n"), ("stderr", b"\xfe\n")]:
                    original = row[channel]
                    raw = Path(original["path"]).read_bytes()
                    self.assertEqual(raw, (b"timeout " if mode == "timeout" else b"failure ") + channel.encode() + ending)
                    self.assertEqual(hashlib.sha256(raw).hexdigest(), original["sha256"])
                    self.assertEqual(len(raw), original["size"])
                self.assertEqual(result["source"], observed)
                self.assertEqual(result["custody"]["status"], "protected-pending")
                self.assertIs(result["custody"]["descendants_quiescent"], None)
                original_result = (attempt.evidence / "result.json").read_bytes()
                with self.assertRaisesRegex(ValueError, "unresolved real-consumer attempt"):
                    RealAttempt(owner, observed, time.monotonic() + 10, 1048576)
                self.assertEqual((attempt.evidence / "result.json").read_bytes(), original_result)

    def test_interrupted_and_historical_attempts_are_preserved(self):
        with tempfile.TemporaryDirectory() as directory:
            owner = Path(directory)
            historical = owner / "attempt-historical"
            historical.mkdir()
            (historical / "original").write_bytes(b"foreign or historical failure")
            attempt = RealAttempt(owner, {"commit": "a" * 40}, time.monotonic() + 10, 1048576)
            self.assertEqual(json.loads((attempt.evidence / "result.json").read_bytes())["status"], "running")
            with self.assertRaisesRegex(ValueError, "unresolved real-consumer attempt"):
                RealAttempt(owner, {}, time.monotonic() + 10, 1048576)
            self.assertEqual((historical / "original").read_bytes(), b"foreign or historical failure")


class RealSharedStartup(unittest.TestCase):
    """Explicit native acceptance mode; the ordinary fixture suite stays small."""

    def test_registered_core_shared_and_runner_consumer(self):
        source_root = SOURCE.parents[1]
        sys.path.insert(0, str(SOURCE))
        import bootstrap
        observed = bootstrap.observe_source(source_root)
        self.assertIsNone(observed["error"])
        self.assertIs(observed["dirty"], False, "real consumption requires committed clean candidate source")
        filemap = json.loads((source_root / ".chrono-harness/FILEMAP.json").read_bytes())
        bounds = filemap["execution_plans"]["test:host-bootstrap-tests"]
        deadline = time.monotonic() + bounds["timeout_seconds"]
        with RealAttempt(source_root / ".chrono-harness/state/bootstrap-real", observed,
                         deadline, bounds["output_limit_bytes"]) as attempt:
            evidence, root = attempt.evidence, Path(attempt.result["fixture"])
            environment = dict(os.environ)
            for key in ("CHRONO_WORKTREE_BOOTSTRAP", "CHRONO_WORKTREE_CHECK", "CHRONO_CHECK_SOURCE"):
                environment.pop(key, None)
            environment["GITHUB_OUTPUT"] = str(evidence / "github-output")
            host = json.loads((source_root / ".chrono-harness/config.json").read_bytes())
            tools = {row["id"]: row["program"] for row in host["tools"]}

            def run(name, argv, cwd):
                attempt.run(name, argv, cwd, environment)

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
            attempt.result["binding"] = binding
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
            # Originals remain at their original paths. No copied reports, forged
            # lifecycle enrollment, or unchecked recursive fixture deletion.
            attempt.result["runner_report"] = reference["original"]



if __name__ == "__main__":
    unittest.main(defaultTest="SharedStartup")
