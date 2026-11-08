#!/usr/bin/env python3
"""Transport real bootstrap fixture outputs, then exercise consumer rejection."""
import hashlib
from concurrent.futures import ThreadPoolExecutor
from contextlib import contextmanager
import importlib.util
import json
import os
import platform
from pathlib import Path
import selectors
import shutil
import socket
import subprocess
import sys
import tempfile
import time
import unittest
import zlib

sys.dont_write_bytecode = True
SOURCE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("bootstrap_fixture", SOURCE / "bootstrap-tests.py")
fixture = importlib.util.module_from_spec(spec)
spec.loader.exec_module(fixture)
CONFIG = ".chrono-harness/ci/bootstrap-shared.json"
LIFECYCLE_PROGRAM = SOURCE.parents[1] / ".chrono-harness/bin/chrono-worktree"


def artifact_directory(name):
    return Path(os.environ["CHRONO_ARTIFACT_DIRECTORY"]) / name


def fixture_host_config(root):
    """Use an already admitted policy when preparing a current fixture host.

    Keep its exact expected Git identity: observing different bytes never edits
    a binding or falls back to another platform's historical policy.
    """
    value = json.loads((root / ".chrono-harness/config.json").read_bytes())
    if value.get("schema") == "chrono-git-configs/v3":
        operating_system = {"Darwin": "macos", "Linux": "linux"}.get(platform.system())
        architecture = {"arm64": "aarch64", "aarch64": "aarch64", "x86_64": "x86_64"}.get(platform.machine())
        host = value["platforms"].get(str(operating_system) + "-" + str(architecture))
        if host is None:
            raise ValueError("fixture has no registered native Git platform")
        observed = hashlib.sha256(Path(host["program"]).read_bytes()).hexdigest()
        matches = [row for row in host["configurations"] if row["sha256"] == observed]
        if len(matches) != 1:
            raise ValueError("fixture has no unique registered Git executable identity")
        value = json.loads((root / matches[0]["path"]).read_bytes())
    return value


class FixtureHostPolicy(unittest.TestCase):
    def test_exact_registered_identity_selects_current_policy_and_preserves_expected_bytes(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / ".chrono-harness").mkdir()
            program = root / "fixture-git"
            program.write_bytes(b"current admitted Git")
            policy = {"expected": hashlib.sha256(program.read_bytes()).hexdigest()}
            (root / "current.json").write_text(json.dumps(policy))
            (root / "historical.json").write_text(json.dumps({"expected": "old admitted Git"}))
            selector = {"schema": "chrono-git-configs/v3", "platforms": {"macos-aarch64": {
                "program": str(program), "configurations": [
                    {"sha256": "0" * 64, "path": "historical.json", "registry_path": "historical.json"},
                    {"sha256": policy["expected"], "path": "current.json", "registry_path": "current.json"}]}}}
            path = root / ".chrono-harness/config.json"
            path.write_text(json.dumps(selector))
            original = path.read_bytes()
            with fixture.mock.patch.object(platform, "system", return_value="Darwin"), fixture.mock.patch.object(platform, "machine", return_value="arm64"):
                self.assertEqual(fixture_host_config(root), policy)
                program.write_bytes(b"unadmitted replacement")
                with self.assertRaisesRegex(ValueError, "no unique registered Git"):
                    fixture_host_config(root)
            self.assertEqual(path.read_bytes(), original)

    def test_unknown_platform_has_no_historical_fallback(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / ".chrono-harness").mkdir()
            (root / ".chrono-harness/config.json").write_text(json.dumps({
                "schema": "chrono-git-configs/v3", "platforms": {}}))
            with self.assertRaisesRegex(ValueError, "no registered native Git platform"):
                fixture_host_config(root)


def acquire_parent_source(source, root, git, run):
    """Acquire current endpoint trees, without traversing unrelated old blobs.

    check-inputs reads physical commit parent headers (also at shallow tips),
    then fetches the configured target. Its scoped runner consumes candidate and
    base snapshots, not the intervening historical file versions. Keep that
    genuine remote: the parent checkout's local dev branch may be stale/absent.
    """
    run("clone", [git, "clone", "--no-local", "-q", "--depth=1", "--single-branch",
                  "--no-tags", str(source), str(root)], source)
    remote = json.loads((root / ".chrono-harness/worktree.json").read_bytes())["remote"]
    branch = json.loads((root / ".chrono-harness/workflow.json").read_bytes())["target_branch"]
    url = run("source-origin", [git, "-C", str(source), "remote", "get-url", remote], source).decode().strip()
    if ":" not in url and not os.path.isabs(url):
        url = str((source / url).resolve())
    if remote != "origin":
        run("fixture-remote", [git, "-C", str(root), "remote", "rename", "origin", remote], root)
    run("fixture-origin", [git, "-C", str(root), "remote", "set-url", remote, url], root)
    run("fixture-base", [git, "-C", str(root), "fetch", "--depth=1", "--no-tags",
                         "--no-write-fetch-head", "--", remote,
                         "refs/heads/" + branch + ":refs/remotes/" + remote + "/" + branch], root)


class ParentSourceAcquisition(unittest.TestCase):
    """Required endpoint bytes and physical parents, independent of old blobs."""

    def setUp(self):
        owner = artifact_directory("bootstrap-custody-tests")
        owner.mkdir(parents=True, exist_ok=True)
        self.directory = Path(tempfile.mkdtemp(prefix="acquisition-", dir=owner)).resolve()
        print("acquisition originals: " + str(self.directory), flush=True)
        self.commands = []
        self.remote, self.source = self.directory / "remote", self.directory / "source"
        self.remote.mkdir()
        self.git(self.remote, "init", "-q", "-b", "dev")
        self.git(self.remote, "config", "user.name", "Fixture")
        self.git(self.remote, "config", "user.email", "fixture@example.invalid")
        (self.remote / "payload").write_bytes(b"irrelevant historical bytes")
        self.commit(self.remote, "history")
        self.old_blob = self.git(self.remote, "rev-parse", "HEAD:payload").decode().strip()
        harness = self.remote / ".chrono-harness"
        harness.mkdir()
        (harness / "worktree.json").write_text(json.dumps({"remote": "origin"}))
        (harness / "workflow.json").write_text(json.dumps({"target_branch": "dev"}))
        (self.remote / "payload").write_bytes(b"required base snapshot")
        self.commit(self.remote, "dev")
        self.base = self.git(self.remote, "rev-parse", "HEAD").decode().strip()
        self.command("source-clone", ["/usr/bin/git", "clone", "--no-local", "-q",
                                  str(self.remote), str(self.source)], self.directory)
        self.git(self.source, "checkout", "-qb", "candidate")
        self.git(self.source, "config", "user.name", "Fixture")
        self.git(self.source, "config", "user.email", "fixture@example.invalid")
        (self.source / "payload").write_bytes(b"required candidate snapshot")
        self.commit(self.source, "candidate")
        self.candidate = self.git(self.source, "rev-parse", "HEAD").decode().strip()
        # One private pack lets omit remove exactly one promised fixture object.
        self.git(self.source, "repack", "-ad")
        self.git(self.source, "config", "remote.origin.promisor", "true")
        self.git(self.source, "config", "remote.origin.partialclonefilter", "blob:none")

    def command(self, name, argv, cwd):
        result = subprocess.run(argv, cwd=cwd, capture_output=True, pass_fds=fixture.PASS_FDS,
                                env=dict(os.environ, GIT_NO_LAZY_FETCH="1"))
        stem = self.directory / (str(len(self.commands)) + "-" + name)
        stem.with_suffix(".stdout").write_bytes(result.stdout)
        stem.with_suffix(".stderr").write_bytes(result.stderr)
        self.commands.append({"argv": argv, "cwd": str(cwd), "exit_code": result.returncode,
                              "stdout": str(stem.with_suffix(".stdout")),
                              "stderr": str(stem.with_suffix(".stderr"))})
        (self.directory / "commands.json").write_text(json.dumps(self.commands, indent=2))
        self.assertEqual(result.returncode, 0, result)
        return result.stdout

    def git(self, root, *args):
        return self.command("git", ["/usr/bin/git", "-C", str(root), *args], root)

    def commit(self, root, message):
        self.git(root, "add", ".")
        self.git(root, "commit", "-qm", message)

    def omit(self, root, oid):
        # Only this newly created fixture object is removed; keep the original
        # bytes for failure diagnosis alongside the exact process originals.
        objects = root / ".git/objects"
        packs = list((objects / "pack").glob("*.pack"))
        for pack in packs:
            raw = pack.read_bytes()
            saved = self.directory / (root.name + "-" + pack.name)
            saved.write_bytes(raw)
            pack.rename(pack.with_suffix(".saved"))
            result = subprocess.run(["/usr/bin/git", "-C", str(root), "unpack-objects"],
                                    input=raw, capture_output=True, pass_fds=fixture.PASS_FDS)
            self.assertEqual(result.returncode, 0, result)
        path = objects / oid[:2] / oid[2:]
        path.rename(self.directory / (root.name + "-missing-" + oid))

    def test_irrelevant_missing_history_preserves_candidate_and_real_dev_inputs(self):
        self.omit(self.source, self.old_blob)
        root = self.directory / "consumer"
        acquire_parent_source(self.source, root, "/usr/bin/git", self.command)
        self.assertEqual(self.git(root, "rev-parse", "HEAD").decode().strip(), self.candidate)
        self.assertEqual(self.git(root, "cat-file", "commit", "HEAD"),
                         self.git(self.source, "cat-file", "commit", "HEAD"))
        self.assertIn(("parent " + self.base + "\n").encode(), self.git(root, "cat-file", "commit", "HEAD"))
        self.assertEqual((root / "payload").read_bytes(), b"required candidate snapshot")
        self.assertEqual(self.git(root, "remote", "get-url", "origin").decode().strip(), str(self.remote))
        # Use the unchanged input producer's subsequent fetch shape.
        self.git(root, "fetch", "--no-tags", "--no-write-fetch-head", "--", "origin",
                 "refs/heads/dev:refs/chrono-harness/check/fixture")
        self.assertEqual(self.git(root, "rev-parse", "refs/chrono-harness/check/fixture").decode().strip(), self.base)
        self.assertEqual(self.git(root, "show", self.base + ":payload"), b"required base snapshot")
        self.assertEqual(self.git(root, "show", self.base + ":.chrono-harness/workflow.json"),
                         (self.remote / ".chrono-harness/workflow.json").read_bytes())
        self.assertEqual(self.git(root, "ls-tree", "-r", "HEAD"),
                         self.git(self.source, "ls-tree", "-r", "HEAD"))
        with self.assertRaises(AssertionError):
            self.git(root, "cat-file", "-e", self.old_blob)

    def test_missing_required_candidate_fails_with_original_diagnostic(self):
        oid = self.git(self.source, "rev-parse", "HEAD:payload").decode().strip()
        self.omit(self.source, oid)
        with self.assertRaises(AssertionError) as failed:
            acquire_parent_source(self.source, self.directory / "consumer", "/usr/bin/git", self.command)
        for expected in ("returncode=", "stdout=", "stderr=", "fatal:"):
            self.assertIn(expected, str(failed.exception))

    def test_missing_genuine_dev_fails_after_candidate_acquisition(self):
        self.git(self.remote, "branch", "-m", "unavailable-dev")
        with self.assertRaises(AssertionError) as failed:
            acquire_parent_source(self.source, self.directory / "consumer", "/usr/bin/git", self.command)
        self.assertIn("refs/heads/dev", str(failed.exception))
        self.assertEqual((self.directory / "consumer/payload").read_bytes(), b"required candidate snapshot")

    def test_missing_required_base_snapshot_is_not_clone_success(self):
        oid = self.git(self.remote, "rev-parse", "HEAD:payload").decode().strip()
        self.omit(self.remote, oid)
        with self.assertRaises(AssertionError) as failed:
            acquire_parent_source(self.source, self.directory / "consumer", "/usr/bin/git", self.command)
        for expected in ("fetch", "refs/heads/dev", "returncode=", "stdout=", "stderr="):
            self.assertIn(expected, str(failed.exception))
        self.assertEqual((self.directory / "consumer/payload").read_bytes(), b"required candidate snapshot")


class SharedStartup(unittest.TestCase):
    def setUp(self):
        self.host = fixture.Bootstrap()
        self.host.setUp()
        self.addCleanup(self.host.doCleanups)
        self.root = self.host.root
        shutil.copyfile(SOURCE / "bootstrap_shared.py", self.host.ci / "bootstrap_shared.py")
        self.host.config["install"].append({"source": "built-second", "destination": ".chrono-harness/bin/second"})
        self.host.write_json(self.host.ci / "bootstrap-core.json", self.host.config)
        for name in ["built-tool", "built-second"]:
            path = self.root / name
            path.write_bytes(b"second")
            path.chmod(0o644 if name == "built-second" else 0o755)
        self.shared = {"schema": "chrono-host-startup/v1", "directory": ".chrono-harness/startup/",
                       "profiles": {"core": {"config": ".chrono-harness/ci/bootstrap-core.json", "build": True}},
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

    def test_produce_uses_registered_profile_entrypoint_before_any_report_or_transfer(self):
        self.host.config["entrypoint"]["tool"] = "refusing-producer"
        self.host.config["tools"]["refusing-producer"] = "/usr/bin/false"
        self.host.write_json(self.host.ci / "bootstrap-core.json", self.host.config)
        self.host.git("add", ".chrono-harness/ci/bootstrap-core.json")
        self.host.git("commit", "-qm", "registered refusing producer")
        result = self.invoke("produce")
        self.assertNotEqual(result.returncode, 0, result)
        self.assertIn(b"/usr/bin/false", result.stderr)
        self.assertFalse(self.transfer.exists())
        self.assertFalse(self.host.state_path.exists())

    def clone_consumer(self):
        directory = tempfile.TemporaryDirectory(prefix="startup consumer ")
        self.addCleanup(directory.cleanup)
        root = Path(directory.name).resolve() / "host"
        result = subprocess.run(["git", "clone", "-q", str(self.root), str(root)],
                                capture_output=True, pass_fds=fixture.PASS_FDS)
        self.host.succeeded(result)
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
            self.assertEqual(path.stat().st_mode & 0o777, 0o644 if name == "second" else 0o755)
        self.assertFalse((root / "built-tool").exists(), "consumer ran a producer operation")
        result = json.loads((root / ".chrono-harness/state/bootstrap.json").read_bytes())
        self.assertEqual(result["provenance"]["kind"], "shared-startup")
        self.assertEqual(result["provenance"]["report_sha256"], hashlib.sha256(original).hexdigest())
        self.host.succeeded(self.invoke("verify", "core", root=root))
        data_file = root / ".chrono-harness/bin/second"
        data_file.chmod(0o755)
        self.assertNotEqual(self.invoke("verify", "core", root=root).returncode, 0)
        data_file.chmod(0o644)
        self.host.succeeded(self.invoke("verify", "core", root=root))
        (root / ".chrono-harness/bin/tool").write_bytes(b"changed")
        failed = self.invoke("verify", "core", root=root)
        self.assertNotEqual(failed.returncode, 0)
        self.assertIn(b"installed startup tool differs", failed.stderr)

    def test_bad_binding_source_platform_profile_or_late_asset_never_partially_installs(self):
        self.produce()
        for defect in ["binding", "missing-binding", "dirty", "source", "platform", "profile", "late-asset", "symlink", "zero-mode", "special-mode", "boolean-mode"]:
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
                elif defect == "profile": (root / ".chrono-harness/ci/bootstrap-core.json").write_text("{}")
                elif defect == "platform":
                    manifest["platform"] = {"system": "different", "machine": "different"}
                    manifest_path.write_text(json.dumps(manifest))
                    self.environment["CHRONO_STARTUP_BINDING"] = hashlib.sha256(manifest_path.read_bytes()).hexdigest()
                elif defect == "late-asset": (transfer / "tools/1").write_bytes(b"damaged")
                elif defect == "symlink":
                    (transfer / "tools/1").unlink()
                    (transfer / "tools/1").symlink_to(root / "product-source")
                elif defect in ["zero-mode", "special-mode", "boolean-mode"]:
                    manifest["profiles"]["core"]["files"][1]["mode"] = {
                        "zero-mode": 0, "special-mode": 0o1000, "boolean-mode": True}[defect]
                    manifest_path.write_text(json.dumps(manifest))
                    self.environment["CHRONO_STARTUP_BINDING"] = hashlib.sha256(manifest_path.read_bytes()).hexdigest()
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
        raw = (self.transfer / "manifest.json").read_bytes()
        retained = self.root / (".chrono-harness/state/startup-originals/manifest-" +
                                hashlib.sha256(raw).hexdigest() + ".json")
        self.assertEqual(retained.read_bytes(), raw)

    def test_manifest_original_survives_transfer_disposal_after_consumer_failure(self):
        self.produce()
        raw = (self.transfer / "manifest.json").read_bytes()
        binding = self.environment["CHRONO_STARTUP_BINDING"]
        retained = self.root / (".chrono-harness/state/startup-originals/manifest-" + binding + ".json")
        self.host.succeeded(self.invoke("install", "unit_fixture"))
        self.host.succeeded(self.invoke("verify", "core"))
        self.assertEqual(retained.read_bytes(), raw)
        (self.root / ".chrono-harness/bin/tool").write_bytes(b"consumer drift")
        failed = self.invoke("verify", "core")
        self.assertNotEqual(failed.returncode, 0)
        self.assertIn(b"installed startup tool differs", failed.stderr)
        # Only this test's reconstructible transfer is disposed, after its users.
        shutil.rmtree(self.transfer)
        self.assertEqual(retained.read_bytes(), raw)
        self.assertEqual(hashlib.sha256(retained.read_bytes()).hexdigest(), binding)
        manifest = json.loads(retained.read_bytes())
        original = self.root / ".chrono-harness/state/startup-originals/core.json"
        self.assertEqual(manifest["source"], json.loads(original.read_bytes())["source"]["after"])
        self.assertEqual(manifest["profiles"]["core"]["report"]["sha256"],
                         hashlib.sha256(original.read_bytes()).hexdigest())

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
        self.host.write_json(self.host.ci / "bootstrap-core.json", self.host.config)
        self.host.git("add", ".chrono-harness/ci/bootstrap-core.json")
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
        self.host.write_json(self.host.ci / "bootstrap-core.json", self.host.config)
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

    A fresh primary fixture participates in the existing main-cache owner before
    production. Source and original evidence retain their paths after disposal.
    """

    def __init__(self, owner, source, deadline, output_limit, recovery_owner=None, recovery_environment=None):
        owner.mkdir(parents=True, exist_ok=True)
        self.evidence = owner / "pending-attempt"
        # Preserve every settled original in place. Atomic mkdir reserves the
        # first unresolved slot even when two invocations arrive together.
        number = 1
        while True:
            try:
                self.evidence.mkdir()
                break
            except FileExistsError:
                try:
                    try:
                        self.require_settled(self.evidence)
                    except (OSError, ValueError, KeyError, TypeError):
                        if recovery_owner is None:
                            raise
                        self.recover_setup(self.evidence, recovery_owner, recovery_environment, deadline, output_limit)
                        self.require_settled(self.evidence)
                except (OSError, ValueError, KeyError, TypeError) as error:
                    raise ValueError("unresolved real-consumer attempt; protected custody: " + str(self.evidence)) from error
                number += 1
                self.evidence = owner / ("pending-attempt-" + str(number))
        self.initialize(source, deadline, output_limit)

    def initialize(self, source, deadline, output_limit, root=None):
        self.deadline, self.output_limit = deadline, output_limit
        self.result = {"source": source, "fixture": str(self.evidence / "fixture" if root is None else root),
                       "status": "running", "commands": [],
                       "custody": {"status": "protected-pending", "descendants_quiescent": None,
                                   "disposal": "not-requested",
                                   "reason": "No owner release/disposal receipt yet.",
                                   "consumers": ["real-consumer assertions", "failure diagnosis", "caller acceptance and review"],
                                   "capacity": "one unresolved attempt; no automatic retry or age deletion"}}
        self.publish()

    @staticmethod
    def original(descriptor, root=Path("/")):
        path = root / descriptor["path"]
        if path.is_symlink() or ("identity" in descriptor and descriptor["identity"] !=
                                str(path.stat().st_dev) + ":" + str(path.stat().st_ino)):
            raise ValueError("owner original physical identity differs")
        raw = path.read_bytes()
        if hashlib.sha256(raw).hexdigest() != descriptor["sha256"] or (
                "size" in descriptor and len(raw) != descriptor["size"]):
            raise ValueError("owner original bytes differ")
        return raw

    @classmethod
    def require_settled(cls, evidence):
        if evidence.is_symlink():
            raise ValueError("attempt ownership is unknown")
        result = json.loads((evidence / "result.json").read_bytes())
        if result["status"] not in ("passed", "failed"):
            raise ValueError("producer completion is unresolved")
        custody, root = result["custody"], Path(result["fixture"])
        completion = evidence / "consumer-completion.json"
        if completion.exists():
            cls.require_consumer_completion(evidence, result)
            return
        if result["status"] == "failed" and len(result["commands"]) == 1 and result["commands"][0]["name"] == "clone":
            cls.require_failed_clone(evidence, result)
            return
        for descriptor in custody.get("required_originals", []):
            cls.original(descriptor)
        reports = {}
        for command in result["commands"]:
            code = command["exit_code"]
            if not isinstance(code, int) or command["status"] != ("passed" if code == 0 else "failed"):
                raise ValueError("producer command has not joined with its original outcome")
            if code != 0 and (result["status"] != "failed" or command["name"] != "managed-use"):
                raise ValueError("producer setup/release did not succeed")
            raw = cls.original(command["stdout"])
            cls.original(command["stderr"])
            reports[command["name"]] = raw
        if "managed_report" not in custody:
            cls.require_setup_release(evidence, result)
            return
        managed = cls.original(custody["managed_report"])
        released = cls.original(custody["release_report"])
        if reports.get("managed-use") != managed or not any(
                name.startswith("owner-release-") and raw == released for name, raw in reports.items()):
            raise ValueError("owner reports are not retained command originals")
        report = json.loads(managed)
        command = next(row for row in result["commands"] if row["name"] == "managed-use")
        process = report["managed_process"]
        if result["status"] == "passed":
            if command["exit_code"] != 0 or report["status"] != "used" or process["exit_code"] != 0 or process["failure"] is not None:
                raise ValueError("managed producer success differs from its original")
        elif command["exit_code"] == 0 or report["status"] != "failed" or not isinstance(process["exit_code"], int):
            raise ValueError("managed producer failure lacks a joined original")
        # Release only the cache-attempt admission slot after exclusive disposal.
        # Failed originals and their consumers keep their outcomes and identities.
        cls.verify_disposal(root, custody["declared_outputs"], json.loads(released))

    @classmethod
    def require_failed_clone(cls, evidence, result):
        """A joined clone failure produced no managed output to release.

        Keep the failed result, streams and partial source in place. This only
        settles admission; it claims neither cache disposal nor task success.
        """
        command, custody = result["commands"][0], result["custody"]
        root, argv = Path(result["fixture"]), command["argv"]
        if (any(key in custody for key in ("owner_executable", "managed_report", "declared_outputs", "release_report"))
                or type(command["exit_code"]) is not int or command["exit_code"] <= 0 or command["status"] != "failed"
                or result["source"].get("dirty") is not False or result["source"].get("error") is not None
                or root != evidence / "fixture" or root.is_symlink() or root.resolve() != root
                or len(argv) != 9 or not Path(argv[0]).is_absolute()
                or argv[1:7] != ["clone", "--no-local", "-q", "--depth=1", "--single-branch", "--no-tags"]
                or argv[7:] != [command["cwd"], str(root)] or not Path(command["cwd"]).is_absolute()
                or custody["disposal"] != "not-requested" or custody["descendants_quiescent"] is not None):
            raise ValueError("failed clone has no unambiguous joined pre-management original")
        cls.original(command["stdout"])
        cls.original(command["stderr"])

    @classmethod
    def require_consumer_completion(cls, evidence, result):
        """Admission consumes the owner's terminal receipt after original use ends.

        This never certifies a new release, repairs missing producer bytes, or
        permits another assertion to consume a reclaimed descriptor.
        """
        # Command originals remain admission inputs for both ordinary attempts
        # and setup recovery after their report-consumption obligation ends.
        for command in result["commands"]:
            code = command["exit_code"]
            if not isinstance(code, int) or command["status"] != ("passed" if code == 0 else "failed"):
                raise ValueError("producer command has no joined original outcome")
            cls.original(command["stdout"])
            cls.original(command["stderr"])
        completion = json.loads((evidence / "consumer-completion.json").read_bytes())
        expected = evidence / "result.json"
        if (completion["schema"] != "chrono-real-attempt-consumer-completion/v1"
                or completion["state"] != "assertions-completed"
                or completion["attempt"]["path"] != str(expected)
                or cls.original(completion["attempt"]) != expected.read_bytes()
                or result["custody"]["disposal"] != "verified-absent"):
            raise ValueError("assertion completion does not bind this exact settled attempt")
        declarations = result["custody"]["consumer_declarations"]
        if set(completion["releases"]) != set(declarations) or not declarations:
            raise ValueError("assertion completion does not account all admitted consumers")
        for consumer, receipt in completion["releases"].items():
            declaration = declarations[consumer]
            value = json.loads(cls.original(declaration))
            domain = json.loads(cls.original(receipt["domain"]))
            release = json.loads(cls.original(receipt["owner"]))
            if (value["consumer"] != consumer or domain != {
                    "consumer": consumer, "declaration_sha256": declaration["sha256"], "state": "released"}
                    or release["schema"] != "chrono-retention-consumer-release/v1"
                    or release["consumer"] != consumer
                    or release["declaration_sha256"] != declaration["sha256"]
                    or release["receipt_sha256"] != receipt["domain"]["sha256"]
                    or release["effects"] != "consumer-root-release-only"
                    or len(value["originals"]) != release["original_count"]
                    or cls.original(receipt["exit"]) != b"0\n"
                    or cls.original(receipt["stderr"]) != b""):
                raise ValueError("exact original consumer release receipt differs")

    @classmethod
    def setup_source(cls, evidence, result):
        import bootstrap
        if result["status"] != "failed" or any(key in result["custody"] for key in
                ("owner_executable", "managed_report", "declared_outputs", "release_report")):
            raise ValueError("not a failed attempt before owner management")
        names = {"clone", "source-origin", "fixture-remote", "fixture-origin", "fixture-base"}
        for command in result["commands"]:
            if command["name"] not in names or command["status"] != "passed" or command["exit_code"] != 0:
                raise ValueError("setup command has no joined successful original")
            cls.original(command["stdout"])
            cls.original(command["stderr"])
        root = Path(result["fixture"])
        if (result["source"].get("dirty") is not False or result["source"].get("error") is not None
                or root.resolve() != root or root.is_symlink() or bootstrap.observe_source(root) != result["source"]):
            raise ValueError("setup fixture source/identity differs from its original")
        return root

    @classmethod
    def verify_setup_release(cls, root, outputs, report, source):
        if report.get("status") != "finished" or report.get("source_root") != str(root):
            raise ValueError("missing actual setup owner release")
        released = json.loads(cls.original(report["main_cache_release"], root))
        if (released.get("schema") != "chrono-main-cache-release/v1" or released.get("path") != str(root)
                or released.get("head") != source["commit"] or released.get("task_completion") != "not-claimed"):
            raise ValueError("setup release identity differs")
        for name in outputs:
            if (root / name).exists() or (root / name).is_symlink():
                raise ValueError("setup fixture output has unknown production: " + name)

    @classmethod
    def require_setup_release(cls, evidence, result):
        root = cls.setup_source(evidence, result)
        recovered = json.loads((evidence / "setup-recovery/result.json").read_bytes())
        if (recovered["status"] != "passed" or recovered["source"] != result["source"]
                or recovered["fixture"] != str(root) or recovered["setup_original"]["path"] != str(evidence / "result.json")
                or cls.original(recovered["setup_original"]) != (evidence / "result.json").read_bytes()):
            raise ValueError("setup recovery has no bound terminal original")
        if (evidence / "setup-recovery/consumer-completion.json").exists():
            cls.require_consumer_completion(evidence / "setup-recovery", recovered)
            return
        commands = recovered["commands"]
        if len(commands) != 2 or commands[0]["name"] != "owner-import" or not commands[1]["name"].startswith("owner-release-"):
            raise ValueError("setup recovery commands differ")
        for command in commands:
            if command["exit_code"] != 0 or command["status"] != "passed":
                raise ValueError("setup recovery did not join successfully")
            cls.original(command["stdout"])
            cls.original(command["stderr"])
        custody = recovered["custody"]
        for descriptor in custody.get("required_originals", []):
            cls.original(descriptor)
        imported_raw = cls.original(custody["import_report"])
        if imported_raw != cls.original(commands[0]["stdout"]):
            raise ValueError("setup import is not its command original")
        imported = json.loads(imported_raw)
        if imported["status"] != "imported" or imported["source_root"] != str(root) or "managed_process" in imported:
            raise ValueError("setup recovery has no unproduced owner import")
        released = cls.original(custody["release_report"])
        if cls.original(commands[1]["stdout"]) != released:
            raise ValueError("setup release is not its command original")
        outputs = [row["path"] for row in json.loads((root / ".chrono-harness/cleanup.json").read_bytes())["artifacts"]
                   if row["disposition"] == "dispose"]
        if custody["declared_outputs"] != outputs:
            raise ValueError("setup recovery output declaration differs")
        cls.verify_setup_release(root, outputs, json.loads(released), result["source"])

    @classmethod
    def recover_setup(cls, evidence, program, environment, deadline, output_limit):
        import bootstrap
        original = evidence / "result.json"
        result = json.loads(original.read_bytes())
        root = cls.setup_source(evidence, result)
        recovery = cls.__new__(cls)
        recovery.evidence = evidence / "setup-recovery"
        try:
            recovery.evidence.mkdir()
        except FileExistsError:
            cls.require_setup_release(evidence, result)
            return
        recovery.initialize(result["source"], deadline, output_limit, root)
        recovery.result["setup_original"] = {"path": str(original), "sha256": bootstrap.file_digest(original),
                                              "size": original.stat().st_size}
        recovery.publish()
        try:
            recovery.manage(root, program, environment)
            recovery.run("owner-import", [str(program), "import", "--host-root", str(root), "--path", str(root),
                         "--report-consumer", recovery.consumer_name("owner-import")],
                         root, environment, output_limit=recovery.owner_output_limit)
            if recovery.retain_owner_report("owner-import", "import_report") is None:
                raise ValueError("setup import lacks its actual owner original")
            recovery.dispose(setup_only=True)
        except BaseException as error:
            recovery.finish("failed", repr(error))
            raise
        recovery.finish("passed")

    def publish(self):
        path = self.evidence / "result.json"
        temporary = path.with_suffix(".json.tmp")
        temporary.write_text(json.dumps(self.result, indent=2) + "\n")
        os.replace(temporary, path)

    def __enter__(self):
        return self

    def __exit__(self, kind, error, traceback):
        try:
            if hasattr(self, "lifecycle"):
                self.dispose()
        except Exception as custody_error:
            self.result["custody"]["error"] = repr(custody_error)
            if kind is None:
                self.finish("failed", repr(custody_error))
                raise
        self.finish("passed" if kind is None else "failed", None if error is None else repr(error))

    def manage(self, root, program, environment):
        import bootstrap
        self.root, self.environment = root, environment
        policy = json.loads((root / ".chrono-harness/cleanup.json").read_bytes())
        if policy.get("main_cache_only") is not True:
            raise ValueError("fixture has not adopted main-cache ownership")
        self.outputs = [row["path"] for row in policy["artifacts"] if row["disposition"] == "dispose"]
        self.require_absent_outputs()
        # Invoke the caller-built owner against the fresh fixture. Do not install
        # a running owner into the destination that core bootstrap will replace.
        self.lifecycle = str(program)
        self.owner_output_limit = fixture_host_config(root)["protocol"]["stdout_limit_bytes"]
        self.result["fixture"] = str(root)
        self.result["custody"]["owner_executable"] = {"path": str(program), "sha256": bootstrap.file_digest(program)}
        self.result["custody"]["declared_outputs"] = self.outputs
        self.publish()

    def require_absent_outputs(self):
        for name in self.outputs:
            path = self.root / name
            if path.exists() or path.is_symlink():
                raise ValueError("pre-existing fixture output has unknown ownership: " + name)

    def managed_argv(self, operation):
        return [self.lifecycle, "use", "--host-root", str(self.root), "--path", str(self.root),
                "--operation", operation, "--report-consumer", self.consumer_name("managed-use")]

    def consumer_name(self, name):
        return "real-attempt:" + hashlib.sha256(str(self.evidence).encode()).hexdigest() + ":" + name

    def managed(self, operation):
        self.require_absent_outputs()
        try:
            # Reserve time for the same owner's release and disposal. On an outer
            # timeout, surviving inherited leases still refuse unsafe disposal.
            self.run("managed-use", self.managed_argv(operation), self.root, self.environment,
                     timeout=max(0.001, self.deadline - time.monotonic() - 15), output_limit=self.owner_output_limit)
        finally:
            self.retain_owner_report("managed-use", "managed_report")

    def owner_report(self, stdout):
        import bootstrap
        report = json.loads(stdout.read_bytes())
        source_root = report.get("source_root")
        if not isinstance(source_root, str):
            raise ValueError("lifecycle report source root missing")
        source = Path(source_root)
        if not source.is_absolute() or source != source.resolve():
            raise ValueError("lifecycle report source root is not canonical")
        report_path = report.get("report_path")
        if not isinstance(report_path, str):
            raise ValueError("lifecycle report path missing")
        relative = Path(report_path)
        if (relative.is_absolute() or ".." in relative.parts
                or not report_path.startswith(".chrono-harness/state/")):
            raise ValueError("lifecycle report path is outside registered state")
        path = source / relative
        if path != path.resolve():
            raise ValueError("lifecycle report path is not canonical")
        # The actual owner's original remains authoritative; stdout is retained too.
        if json.loads(path.read_bytes()) != report:
            raise ValueError("lifecycle stdout differs from original report")
        descriptor = {"path": str(path), "sha256": bootstrap.file_digest(path), "size": path.stat().st_size,
                      "identity": str(path.stat().st_dev) + ":" + str(path.stat().st_ino)}
        # Publication transfers this actual caller's root while its producer
        # lease is still held. No post-acquisition pin can close that interval.
        declaration = report["report_consumer"]
        if declaration["consumer"] != self.consumer_name(stdout.stem):
            raise ValueError("lifecycle original consumer differs")
        expected = declaration["originals"][0]
        if expected["path"] != report_path or expected["identity"] != descriptor["identity"] + ":file":
            raise ValueError("lifecycle consumer original identity differs")
        self.retain_receipt_closure(source, report)
        directory = source / ".chrono-harness/state/real-attempt-consumers" / hashlib.sha256(declaration["consumer"].encode()).hexdigest()
        directory.mkdir(parents=True, exist_ok=True)
        target = directory / "declaration.json"
        raw = (json.dumps(declaration, sort_keys=True) + "\n").encode()
        if target.exists() and target.read_bytes() != raw:
            raise ValueError("consumer declaration original differs")
        if not target.exists():
            target.write_bytes(raw)
        self.result["custody"].setdefault("consumer_declarations", {})[declaration["consumer"]] = {
            "path": str(target), "sha256": hashlib.sha256(raw).hexdigest(), "size": len(raw)}
        return descriptor

    def retain_receipt_closure(self, root, report):
        """Only the typed receipt members consumed by custody assertions."""
        custody = self.result["custody"]
        for key in ("recovery_intent", "fetch_intent", "cache_intent", "main_cache_release"):
            if report.get(key):
                relative = report[key]
                raw = self.original(relative, root)
                path = root / relative["path"]
                descriptor = dict(relative, path=str(path), size=len(raw),
                                  identity=str(path.stat().st_dev) + ":" + str(path.stat().st_ino))
                originals = custody.setdefault("required_originals", [])
                if descriptor not in originals:
                    originals.append(descriptor)
        for item in report.get("drain", []):
            if item.get("receipt"):
                relative = item["receipt"]
                raw = self.original(relative, root)
                path = root / relative["path"]
                descriptor = dict(relative, path=str(path), size=len(raw),
                                  identity=str(path.stat().st_dev) + ":" + str(path.stat().st_ino))
                originals = custody.setdefault("required_originals", [])
                if descriptor not in originals:
                    originals.append(descriptor)
                self.retain_receipt_closure(root, json.loads(raw))

    def release_consumers(self, verify_setup=None):
        """Called by the assertion owner only after its successful final read.

        Failed assertions/acceptance keep their declarations. Cache release is
        independent and never certifies completion of this original consumer.
        """
        if verify_setup is not None:
            self.require_settled(verify_setup)
        else:
            # Assertion completion is distinct from admission settlement: an
            # expected earlier live-consumer refusal is still original evidence.
            # Only the latest actual release may prove current cache disposal.
            custody = self.result["custody"]
            if custody["disposal"] != "verified-absent" or self.result["status"] not in ("passed", "failed"):
                raise ValueError("assertion consumer still has unresolved producer/disposal")
            for command in self.result["commands"]:
                if not isinstance(command["exit_code"], int):
                    raise ValueError("assertion consumer has unjoined command")
                for channel in ("stdout", "stderr"):
                    self.original(command[channel])
            for descriptor in custody.get("required_originals", []):
                self.original(descriptor)
            self.original(custody["managed_report"])
            self.verify_disposal(self.root, self.outputs, json.loads(self.original(custody["release_report"])))
        releases = {}
        def binding(path):
            raw = path.read_bytes()
            return {"path": str(path), "sha256": hashlib.sha256(raw).hexdigest(), "size": len(raw)}
        for consumer, descriptor in self.result["custody"].get("consumer_declarations", {}).items():
            raw = self.original(descriptor)
            path = Path(descriptor["path"])
            root = self.root
            declaration = {"path": str(path.relative_to(root)), "sha256": descriptor["sha256"]}
            state = {"consumer": consumer, "declaration_sha256": descriptor["sha256"], "state": "released"}
            receipt = path.parent / "assertions-released.json"
            encoded = (json.dumps(state, sort_keys=True) + "\n").encode()
            if receipt.exists() and receipt.read_bytes() != encoded:
                raise ValueError("consumer release original differs")
            if not receipt.exists():
                receipt.write_bytes(encoded)
            request = {"operation": "release-consumer", "declaration": declaration,
                       "receipt": {"path": str(receipt.relative_to(root)), "sha256": hashlib.sha256(encoded).hexdigest()},
                       "pointer": ""}
            request_path = path.parent / "release-request.json"
            request_path.write_text(json.dumps(request))
            # Separate originals: do not rewrite a settled attempt's command
            # history or make an earlier cache release certify this release.
            with (path.parent / "release.stdout").open("xb") as out, (path.parent / "release.stderr").open("xb") as err:
                result = subprocess.run([str(SOURCE.parents[1] / ".chrono-harness/bin/chrono-harness"),
                    "retention", "--host-root", str(root), "--request", str(request_path.relative_to(root))],
                    cwd=root, env=self.environment, pass_fds=fixture.PASS_FDS, stdout=out, stderr=err,
                    timeout=max(0.001, self.deadline - time.monotonic()))
            (path.parent / "release.exit").write_text(str(result.returncode) + "\n")
            if result.returncode:
                raise ValueError("original consumer release failed: " + str(path.parent))
            releases[consumer] = {"domain": binding(receipt), "owner": binding(path.parent / "release.stdout"),
                                  "stderr": binding(path.parent / "release.stderr"), "exit": binding(path.parent / "release.exit")}
        completion = {"schema": "chrono-real-attempt-consumer-completion/v1", "state": "assertions-completed",
                      "attempt": binding(self.evidence / "result.json"), "releases": releases}
        target = self.evidence / "consumer-completion.json"
        with target.open("x") as output:
            output.write(json.dumps(completion, indent=2) + "\n")
        self.consumers_released = True

    def retain_owner_report(self, name, key):
        custody = self.result["custody"]
        try:
            descriptor = self.owner_report(self.evidence / (name + ".stdout"))
        except (OSError, ValueError, KeyError, TypeError) as error:
            # Diagnostic collection must not replace the command's own failure.
            # Old command streams/reports remain retained; none proves this release.
            custody[key + "_error"] = repr(error)
            custody.pop(key, None)
            self.publish()
            return None
        custody[key] = descriptor
        custody.pop(key + "_error", None)
        self.publish()
        return descriptor

    def dispose(self, setup_only=False):
        custody = self.result["custody"]
        custody.update(status="protected-pending", disposal="requested", descendants_quiescent=None,
                       reason="No verified release/disposal receipt for this request.")
        self.publish()
        name = "owner-release-" + str(len(self.result["commands"]))
        try:
            try:
                self.run(name, [self.lifecycle, "finish", "--host-root", str(self.root),
                               "--path", str(self.root), "--artifacts-only", "--report-consumer", self.consumer_name(name)], self.root, self.environment,
                         output_limit=self.owner_output_limit)
            finally:
                released = self.retain_owner_report(name, "release_report")
            if released is None:
                raise ValueError("owner release has no valid original report; preserve pending attempt")
            if setup_only:
                self.verify_setup_release(self.root, self.outputs, json.loads(self.original(released)), self.result["source"])
            else:
                self.verify_disposal(self.root, self.outputs, json.loads(self.original(released)))
        except Exception as error:
            custody["error"] = repr(error)
            self.publish()
            raise
        custody.update(status="outputs-disposed-evidence-retained", disposal="verified-absent",
                       descendants_quiescent=True, reason="Owner acquired exclusive enrollment lease and verified declared output absence.")
        custody.pop("error", None)
        self.publish()

    @classmethod
    def verify_disposal(cls, root, outputs, report):
        if report.get("status") != "finished" or not report.get("main_cache_release"):
            raise ValueError("missing actual main-cache release receipt")
        def receipt(descriptor):
            return json.loads(cls.original(descriptor, root))
        receipt(report["main_cache_release"])
        effects = {}
        for item in report.get("drain", []):
            if item.get("path") != str(root) or "receipt" not in item:
                continue
            original = receipt(item["receipt"])
            if original.get("status") != "cleaned" or original.get("protected_historical_artifacts"):
                raise ValueError("owner has not disposed all freshly owned outputs")
            intent = receipt(original["cache_intent"])
            if set(intent["binding"]["artifacts"]) != set(outputs):
                raise ValueError("owner output ownership differs")
            effects.update({row["path"]: row for row in original["artifact_disposals"]})
        if set(effects) != set(outputs) or any(
                row["status"] not in ("verified-absent", "already-absent") or row["bytes_after"] != 0
                or (root / name).exists() or (root / name).is_symlink()
                for name, row in effects.items()):
            raise ValueError("no complete registered disposal evidence; preserve pending attempt")

    def finish(self, status, error=None):
        self.result["status"] = status
        if error is not None:
            self.result["error"] = error
        self.publish()

    def run(self, name, argv, cwd, environment, timeout=None, output_limit=None):
        if any(row["name"] == name for row in self.result["commands"]):
            raise ValueError("duplicate real-consumer command")
        stdout, stderr = self.evidence / (name + ".stdout"), self.evidence / (name + ".stderr")
        row = {"name": name, "argv": argv, "cwd": str(cwd), "exit_code": None,
               "status": "running", "ownership_fds": list(fixture.PASS_FDS),
               "output_limit_bytes": self.output_limit if output_limit is None else output_limit}
        self.result["commands"].append(row)
        self.publish()
        started = time.monotonic()
        try:
            with stdout.open("xb") as out, stderr.open("xb") as err:
                process = subprocess.run(argv, cwd=cwd, env=environment, pass_fds=fixture.PASS_FDS,
                                         stdout=out, stderr=err, timeout=max(0.001, min(self.deadline - started, timeout if timeout is not None else self.deadline - started)))
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
            self.retain_streams(row, stdout, stderr)
        size = sum(command[channel]["size"] for command in self.result["commands"]
                   for channel in ("stdout", "stderr") if channel in command
                   and command["output_limit_bytes"] == row["output_limit_bytes"])
        if size > row["output_limit_bytes"]:
            raise AssertionError("real-consumer original streams exceed registered output bound")
        if process.returncode:
            raise AssertionError(
                f"{name}: exit_code={process.returncode}\n"
                f"stdout ({stdout}):\n{stdout.read_text(errors='replace')[-3000:]}\n"
                f"stderr ({stderr}):\n{stderr.read_text(errors='replace')[-3000:]}")

    def retain_streams(self, row, stdout, stderr):
        for channel, path in (("stdout", stdout), ("stderr", stderr)):
            if path.is_file():
                import bootstrap
                row[channel] = {"path": str(path), "sha256": bootstrap.file_digest(path),
                                "size": path.stat().st_size}
        self.publish()


class AttemptCustody(unittest.TestCase):
    def failed_clone(self, directory):
        source = Path(directory).resolve()
        observed = {"commit": "a" * 40, "tree": "b" * 40, "dirty": False, "error": None}
        attempt = RealAttempt(source / "evidence", observed, time.monotonic() + 10, 1048576)
        root = Path(attempt.result["fixture"])
        root.mkdir()
        (root / "partial-source").write_bytes(b"preserve partial clone")
        with self.assertRaisesRegex(AssertionError, "clone: exit_code=128"):
            attempt.run("clone", ["/usr/bin/git", "clone", "--no-local", "-q", "--depth=1",
                        "--single-branch", "--no-tags", str(source), str(root)], source, dict(os.environ))
        attempt.finish("failed", "joined clone failure")
        return attempt

    def test_joined_failed_clone_retains_source_and_failure_before_next_attempt(self):
        with tempfile.TemporaryDirectory() as directory:
            attempt = self.failed_clone(directory)
            original = (attempt.evidence / "result.json").read_bytes()
            RealAttempt.require_settled(attempt.evidence)
            next_attempt = RealAttempt(attempt.evidence.parent, {}, time.monotonic() + 10, 1048576)
            self.assertNotEqual(next_attempt.evidence, attempt.evidence)
            self.assertEqual((attempt.evidence / "result.json").read_bytes(), original)
            self.assertEqual((Path(attempt.result["fixture"]) / "partial-source").read_bytes(), b"preserve partial clone")
            self.assertEqual(attempt.result["custody"]["disposal"], "not-requested")
            self.assertIsNone(attempt.result["custody"]["descendants_quiescent"])

    def test_failed_clone_admission_refuses_unknown_outcomes_management_and_changed_originals(self):
        for mode in ("timeout", "signal", "managed", "extra-command", "other-command", "argv", "fixture", "stream"):
            with self.subTest(mode=mode), tempfile.TemporaryDirectory() as directory:
                attempt = self.failed_clone(directory)
                command = attempt.result["commands"][0]
                if mode == "timeout":
                    command.update(status="timeout", exit_code=None)
                elif mode == "signal":
                    command["exit_code"] = -9
                elif mode == "managed":
                    attempt.result["custody"]["owner_executable"] = {"path": "owner"}
                elif mode == "extra-command":
                    attempt.result["commands"].append(dict(command, name="managed-use"))
                elif mode == "other-command":
                    command["name"] = "managed-use"
                elif mode == "argv":
                    command["argv"][1] = "checkout"
                elif mode == "fixture":
                    attempt.result["fixture"] = str(attempt.evidence.parent)
                else:
                    Path(command["stderr"]["path"]).write_bytes(b"changed")
                attempt.publish()
                with self.assertRaises((ValueError, OSError, KeyError, TypeError)):
                    RealAttempt.require_settled(attempt.evidence)

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
                    with self.assertRaises((AssertionError, subprocess.TimeoutExpired)) as failure:
                        attempt.run("command", argv, Path(directory), dict(os.environ))
                    if status == "failed":
                        message = str(failure.exception)
                        self.assertIn("exit_code=7", message)
                        for channel in ("stdout", "stderr"):
                            self.assertIn(str(attempt.evidence / ("command." + channel)), message)
                            self.assertIn("failure " + channel, message)
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


class ManagedAttemptCustody(unittest.TestCase):
    """Independent expectations against the existing real lifecycle binary."""

    def run(self, result=None):
        self.attempts = []
        result = super().run(result)
        failed = any(case is self or getattr(case, "test_case", None) is self
                     for case, error in result.errors + result.failures)
        if not failed:
            try:
                for attempt in self.attempts:
                    if attempt.result["custody"]["disposal"] == "verified-absent" and not getattr(attempt, "consumers_released", False):
                        attempt.release_consumers()
                    if getattr(attempt, "consumers_released", False):
                        RealAttempt.require_settled(attempt.evidence)
                    recovery_path = attempt.evidence / "setup-recovery/result.json"
                    if recovery_path.is_file():
                        recovered = RealAttempt.__new__(RealAttempt)
                        recovered.evidence = recovery_path.parent
                        recovered.result = json.loads(recovery_path.read_bytes())
                        recovered.root = Path(recovered.result["fixture"])
                        recovered.deadline, recovered.environment = attempt.deadline, attempt.fixture_environment
                        if recovered.result["status"] == "passed":
                            # The failed setup's replay consumer, rather than a
                            # managed-use consumer, owns this completed recovery.
                            if not (recovered.evidence / "consumer-completion.json").exists():
                                recovered.release_consumers(verify_setup=attempt.evidence)
                            RealAttempt.require_settled(attempt.evidence)
            except Exception:
                result.addError(self, sys.exc_info())
        return result

    def test_original_consumer_spans_publication_acquisition_disposal_and_release(self):
        root, attempt = self.host("success")
        attempt.run("managed-use", attempt.managed_argv("fixture.custody"), root, attempt.environment,
                    output_limit=attempt.owner_output_limit)
        # A real owner transition before acquisition falsifies a post-read-only
        # pin. Its original maintenance report records the actual effects.
        request = root / ".chrono-harness/state/real-attempt-maintain.json"
        request.write_text(json.dumps({"operation": "maintain"}))
        maintenance = [str(SOURCE.parents[1] / ".chrono-harness/bin/chrono-harness"),
                       "retention", "--host-root", str(root), "--request", str(request.relative_to(root))]
        attempt.run("before-acquisition", maintenance,
                    root, attempt.environment, output_limit=attempt.owner_output_limit)
        descriptor = attempt.retain_owner_report("managed-use", "managed_report")
        self.assertIsNotNone(descriptor)
        original = RealAttempt.original(descriptor)
        with attempt:
            pass
        attempt.run("after-disposal", maintenance,
                    root, attempt.environment, output_limit=attempt.owner_output_limit)
        self.assertEqual(RealAttempt.original(descriptor), original)
        RealAttempt.require_settled(attempt.evidence)
        self.assertFalse((root / "crates/runner/target").exists())
        self.assertTrue((root / ".git/index").is_file())
        for changed in (dict(descriptor, sha256="0" * 64), dict(descriptor, identity="0:0"),
                        dict(descriptor, path=str(root / "absent-original"))):
            with self.assertRaises((ValueError, OSError)):
                RealAttempt.original(changed)
        attempt.release_consumers()
        with (attempt.evidence / "after-consumer-release.stdout").open("xb") as out, (attempt.evidence / "after-consumer-release.stderr").open("xb") as err:
            maintained = subprocess.run(maintenance, cwd=root, env=attempt.environment,
                pass_fds=fixture.PASS_FDS, stdout=out, stderr=err,
                timeout=max(0.001, attempt.deadline - time.monotonic()))
        (attempt.evidence / "after-consumer-release.exit").write_text(str(maintained.returncode) + "\n")
        self.assertEqual(maintained.returncode, 0)
        round_result = json.loads((attempt.evidence / "after-consumer-release.stdout").read_bytes())
        # A finite maintenance round may stop before deleting any bytes. The
        # independent retention suite owns eventual disposal; this caller checks
        # its real result for errors and preserves any original still retained.
        self.assertEqual(round_result["accounting_errors"], [])
        for declaration in attempt.result["custody"]["consumer_declarations"].values():
            release = json.loads((Path(declaration["path"]).parent / "release.stdout").read_bytes())
            self.assertEqual(release["declaration_sha256"], declaration["sha256"])
            self.assertEqual(release["effects"], "consumer-root-release-only")
        # A 25ms host maintenance round may stop before an 8MiB report's cursor.
        # The dedicated runner obligation verifies exact released closure disposal;
        # no test deadline or retention-round budget is enlarged here.
        if Path(descriptor["path"]).exists():
            self.assertEqual(RealAttempt.original(descriptor), original)
        RealAttempt.require_settled(attempt.evidence)
        completion = attempt.evidence / "consumer-completion.json"
        raw = completion.read_bytes()
        tampered = json.loads(raw)
        tampered["attempt"]["sha256"] = "0" * 64
        completion.write_text(json.dumps(tampered))
        with self.assertRaises(ValueError):
            RealAttempt.require_settled(attempt.evidence)
        completion.write_bytes(raw)
        admitted = RealAttempt(attempt.evidence.parent, {}, time.monotonic() + 5, 1048576)
        self.assertNotEqual(admitted.evidence, attempt.evidence)

    def test_independent_host_rebinds_inherited_tools(self):
        acquire = acquire_parent_source

        def inherited_host(source, root, git, run):
            acquire(source, root, git, run)
            policy_path = root / ".chrono-harness/worktree.json"
            policy = json.loads(policy_path.read_bytes())
            policy["git"].update(program="/absent/parent/git", expected_version="parent Git", sha256="0" * 64)
            policy_path.write_text(json.dumps(policy))
            config_path = root / ".chrono-harness/config.json"
            config = fixture_host_config(root)
            next(row for row in config["tools"] if row["id"] == "python3")["program"] = "/absent/parent/python3"
            config_path.write_text(json.dumps(config))

        with fixture.mock.patch(__name__ + ".acquire_parent_source", side_effect=inherited_host):
            root, attempt = self.host("success")
        with attempt:
            attempt.managed("fixture.custody")
        report = json.loads(RealAttempt.original(attempt.result["custody"]["managed_report"]))
        self.assertEqual(report["tool"]["path"], "/usr/bin/git")
        self.assertEqual(report["tool"]["sha256"], hashlib.sha256(Path("/usr/bin/git").read_bytes()).hexdigest())
        version = subprocess.run(["/usr/bin/git", "--version"], capture_output=True, check=True,
                                 pass_fds=fixture.PASS_FDS)
        import bootstrap
        self.assertEqual(bootstrap.retained_process(report["tool"]["version"])["stdout_bytes"], version.stdout)
        assertions = json.loads((root / ".chrono-harness/state/custody-assertions.json").read_bytes())
        self.assertEqual(assertions["python_executable"], sys.executable)
        self.assertEqual(report["managed_process"]["argv"][0], sys.executable)
        self.assertEqual(report["tool"]["version"]["environment"], attempt.environment)
        # Even a zero-exit release needs its own original: an older successful
        # report must not certify a later request whose report cannot be read.
        original_release = attempt.result["custody"]["release_report"]
        original_bytes = RealAttempt.original(original_release)
        with fixture.mock.patch.object(attempt, "owner_report", side_effect=FileNotFoundError("missing current release")):
            with self.assertRaisesRegex(ValueError, "owner release has no valid original report"):
                with attempt:
                    pass
        self.assertEqual(RealAttempt.original(original_release), original_bytes)
        self.assertEqual(attempt.result["commands"][-1]["exit_code"], 0)
        self.assertEqual(attempt.result["custody"]["status"], "protected-pending")
        self.assertIsNone(attempt.result["custody"]["descendants_quiescent"])
        self.assertNotIn("release_report", attempt.result["custody"])
        with self.assertRaisesRegex(ValueError, "unresolved real-consumer attempt"):
            RealAttempt(attempt.evidence.parent, {}, time.monotonic() + 5, 1048576)

    def test_rejected_owner_keeps_primary_failure_and_pending_custody(self):
        for consumer in ("managed", "live"):
            with self.subTest(consumer=consumer):
                root, attempt = self.host("hold" if consumer == "live" else "success")
                policy_path = root / ".chrono-harness/worktree.json"
                policy = json.loads(policy_path.read_bytes())
                policy["git"]["sha256"] = "0" * 64
                policy_path.write_text(json.dumps(policy))
                for args in (["add", str(policy_path)], ["-c", "user.name=Fixture", "-c", "user.email=fixture@example.invalid",
                                                        "commit", "-qm", "reject fixture owner before enrollment"]):
                    subprocess.run(["/usr/bin/git", "-C", str(root), *args], capture_output=True,
                                   check=True, pass_fds=fixture.PASS_FDS)
                import bootstrap
                attempt.result["source"] = bootstrap.observe_source(root)
                attempt.publish()
                if consumer == "managed":
                    # A missing owner original must not replace the real nonzero
                    # command, just as empty stdout must not do so on native CI.
                    with fixture.mock.patch.object(attempt, "owner_report", side_effect=FileNotFoundError("missing owner original")):
                        with self.assertRaisesRegex(AssertionError, r"managed-use: exit_code=2[\s\S]*E_WORKTREE: Git executable digest mismatch") as failed:
                            with attempt:
                                attempt.managed("fixture.custody")
                else:
                    with self.assertRaisesRegex(AssertionError, "live consumer readiness.*Git executable digest mismatch") as failed:
                        with attempt:
                            with self.live_consumer(root, attempt):
                                self.fail("rejected owner reached consumer")
                    self.assertFalse((root / ".chrono-harness/state/custody-release").exists())
                result = json.loads((attempt.evidence / "result.json").read_bytes())
                self.assertEqual(result["error"], repr(failed.exception))
                self.assertEqual(result["status"], "failed")
                self.assertEqual(result["custody"]["status"], "protected-pending")
                self.assertIsNone(result["custody"]["descendants_quiescent"])
                self.assertNotIn("release_report", result["custody"])
                self.assertIn("Git executable digest mismatch", result["custody"]["error"])
                for command in result["commands"]:
                    self.assertNotEqual(command["exit_code"], 0)
                    self.assertEqual(command["status"], "failed")
                    self.assertIn(b"Git executable digest mismatch", RealAttempt.original(command["stderr"]))
                    self.assertEqual(RealAttempt.original(command["stdout"]), b"")
                with self.assertRaisesRegex(ValueError, "unresolved real-consumer attempt"):
                    RealAttempt(attempt.evidence.parent, {}, time.monotonic() + 5, 1048576)

    def test_clone_failure_exposes_original_subprocess_diagnostic(self):
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory).resolve()
            with fixture.mock.patch(__name__ + ".SOURCE", source / ".chrono-harness/ci"):
                with self.assertRaises(AssertionError) as failed:
                    self.host("clone-failure")
            message = str(failed.exception)
            for expected in ("args=['/usr/bin/git', 'clone', '--no-local', '-q'", str(source),
                             "returncode=128", "stdout=b''", "stderr=b", "fatal:"):
                self.assertIn(expected, message)

    def host(self, mode, manage=True):
        base = artifact_directory("bootstrap-custody-tests")
        base.mkdir(parents=True, exist_ok=True)
        directory = Path(tempfile.mkdtemp(prefix=mode + "-", dir=base)).resolve()
        root = directory / "fixture"
        # This is an independent host, not a replica of the developer's tools.
        environment = {key: os.environ[key] for key in ("PATH", "HOME", "SSH_AUTH_SOCK") if key in os.environ}
        environment["GIT_TERMINAL_PROMPT"] = "0"
        git = "/usr/bin/git"
        def acquire(name, argv, cwd):
            result = subprocess.run(argv, cwd=cwd, env=environment, capture_output=True, pass_fds=fixture.PASS_FDS)
            self.assertEqual(result.returncode, 0, result)
            return result.stdout
        acquire_parent_source(SOURCE.parents[1], root, git, acquire)
        policy_path = root / ".chrono-harness/worktree.json"
        policy = json.loads(policy_path.read_bytes())
        policy["environment"] = {"inherit": ["PATH", "HOME", "SSH_AUTH_SOCK"], "values": {"GIT_TERMINAL_PROMPT": "0"}}
        policy["schema"] = "chrono-worktree-config/v2"
        policy["git"] = {"program": git,
                         "expected_version": acquire("git-version", [git, "--version"], root).decode().strip(),
                         "sha256": hashlib.sha256(Path(git).read_bytes()).hexdigest()}
        policy_path.write_text(json.dumps(policy))
        shutil.copyfile(SOURCE / "fixtures/bootstrap-operation.py", root / fixture.OPERATION)
        config_path = root / ".chrono-harness/config.json"
        config = fixture_host_config(root)
        config["protocol"]["timeout_seconds"] = 1 if mode == "timeout" else 20
        next(row for row in config["tools"] if row["id"] == "python3")["program"] = sys.executable
        config_path.write_text(json.dumps(config))
        projects_path = root / ".chrono-harness/projects.json"
        projects = json.loads(projects_path.read_bytes())
        script = next(row for row in projects["scripts"] if row["id"] == "host-bootstrap-tests")
        script["actions"]["custody-fixture"] = {"operation": "fixture.custody", "tool": "python3",
            "argv": ["-B", fixture.OPERATION, "custody", mode]}
        projects_path.write_text(json.dumps(projects))
        for args in (["add", "."], ["-c", "user.name=Fixture", "-c", "user.email=fixture@example.invalid", "commit", "-qm", "custody fixture"]):
            acquire("commit", [git, "-C", str(root), *args], root)
        import bootstrap
        attempt = RealAttempt(directory / "evidence", bootstrap.observe_source(root), time.monotonic() + 40, 1048576)
        self.attempts.append(attempt)
        if manage:
            attempt.manage(root, LIFECYCLE_PROGRAM, environment)
        else:
            attempt.result["fixture"] = str(root)
            attempt.publish()
        attempt.fixture_environment = environment
        print("custody fixture originals: " + str(attempt.evidence), flush=True)
        return root, attempt

    def test_failed_setup_recovers_actual_owner_release_without_changing_original(self):
        root, attempt = self.host("success", manage=False)
        attempt.finish("failed", "pre-management input assertion")
        original = (attempt.evidence / "result.json").read_bytes()
        with self.assertRaisesRegex(ValueError, "unresolved real-consumer attempt"):
            RealAttempt(attempt.evidence.parent, {}, time.monotonic() + 10, 1048576)

        owner = LIFECYCLE_PROGRAM
        admitted = RealAttempt(attempt.evidence.parent, {}, time.monotonic() + 40, 1048576,
                               recovery_owner=owner, recovery_environment=attempt.fixture_environment)
        self.assertNotEqual(admitted.evidence, attempt.evidence)
        self.assertEqual((attempt.evidence / "result.json").read_bytes(), original)
        recovery_path = attempt.evidence / "setup-recovery/result.json"
        recovery_raw = recovery_path.read_bytes()
        recovery = json.loads(recovery_raw)
        self.assertEqual(recovery["status"], "passed")
        self.assertEqual([row["name"] for row in recovery["commands"]], ["owner-import", "owner-release-1"])
        release = json.loads(RealAttempt.original(recovery["custody"]["release_report"]))
        receipt = json.loads(RealAttempt.original(release["main_cache_release"], root))
        self.assertEqual(receipt["task_completion"], "not-claimed")
        self.assertNotIn("managed_process", release)
        self.assertFalse((root / "crates/runner/target").exists())
        self.assertTrue((root / ".git").is_dir())
        self.assertTrue((root / fixture.OPERATION).is_file())
        for defect in ("missing-release", "changed-stream", "unknown-command", "false-source"):
            with self.subTest(defect=defect):
                changed = json.loads(recovery_raw)
                if defect == "missing-release":
                    del changed["custody"]["release_report"]
                elif defect == "changed-stream":
                    changed["commands"][1]["stdout"]["sha256"] = "0" * 64
                elif defect == "unknown-command":
                    changed["commands"][0]["name"] = "unjoined-consumer"
                else:
                    changed["setup_original"]["sha256"] = "0" * 64
                recovery_path.write_text(json.dumps(changed))
                with self.assertRaises((ValueError, KeyError)):
                    RealAttempt.require_settled(attempt.evidence)
                self.assertEqual((attempt.evidence / "result.json").read_bytes(), original)
        recovery_path.write_bytes(recovery_raw)
        RealAttempt.require_settled(attempt.evidence)
        with self.assertRaisesRegex(ValueError, "unresolved real-consumer attempt"):
            RealAttempt(attempt.evidence.parent, {}, time.monotonic() + 10, 1048576)

        # End the real setup-report obligation, then exercise later admission
        # against the original command streams that remain its inputs.
        completed = RealAttempt.__new__(RealAttempt)
        completed.evidence, completed.result = recovery_path.parent, recovery
        completed.root, completed.environment = root, attempt.fixture_environment
        completed.deadline = attempt.deadline
        completed.release_consumers(verify_setup=attempt.evidence)
        RealAttempt.require_settled(attempt.evidence)
        stream = Path(recovery["commands"][0]["stdout"]["path"])
        raw = stream.read_bytes()
        try:
            stream.write_bytes(raw + b"changed after consumer release")
            with self.assertRaises(ValueError):
                RealAttempt.require_settled(attempt.evidence)
        finally:
            stream.write_bytes(raw)
        RealAttempt.require_settled(attempt.evidence)

    def test_setup_recovery_preserves_running_changed_and_unmanaged_production(self):
        for defect in ("running", "unknown-command", "source-drift", "unknown-output"):
            with self.subTest(defect=defect):
                root, attempt = self.host("success", manage=False)
                if defect == "unknown-command":
                    attempt.run("unmanaged-production", [sys.executable, "-B", str(SOURCE / "fixtures/bootstrap-operation.py"),
                                "fail", "0"], root, attempt.fixture_environment)
                elif defect == "source-drift":
                    (root / fixture.OPERATION).write_bytes(b"changed source")
                elif defect == "unknown-output":
                    (root / "crates/runner/target").mkdir(parents=True)
                if defect != "running":
                    attempt.finish("failed", "unresolved original setup")
                original = (attempt.evidence / "result.json").read_bytes()
                with self.assertRaises((ValueError, AssertionError)):
                    RealAttempt(attempt.evidence.parent, {}, time.monotonic() + 40, 1048576,
                                recovery_owner=LIFECYCLE_PROGRAM,
                                recovery_environment=attempt.fixture_environment)
                self.assertEqual((attempt.evidence / "result.json").read_bytes(), original)
                self.assertEqual(sorted(path.name for path in attempt.evidence.parent.iterdir()), ["pending-attempt"])

    def test_owned_success_failure_timeout_dispose_outputs_keep_source_and_originals(self):
        for mode in ("success", "failure", "timeout"):
            with self.subTest(mode=mode):
                root, attempt = self.host(mode)
                source = (root / fixture.OPERATION).read_bytes()
                with attempt:
                    if mode == "success":
                        attempt.managed("fixture.custody")
                    else:
                        with self.assertRaises(AssertionError):
                            attempt.managed("fixture.custody")
                # A drain may happen as soon as use returns. Consume only
                # retained assertion evidence after that protection ends.
                assertions = json.loads((root / ".chrono-harness/state/custody-assertions.json").read_bytes())
                self.assertTrue(assertions["ownership_fds"])
                self.assertEqual(assertions["verified_payloads"],
                                 ["crates/runner/target/payload", ".chrono-harness/startup/payload"])
                self.assertEqual((root / ".chrono-harness/state/custody-original").read_bytes(), b"original evidence")
                managed = json.loads(Path(attempt.result["custody"]["managed_report"]["path"]).read_bytes())
                process = managed["managed_process"]
                self.assert_producer_streams(attempt, process)
                if mode != "timeout":
                    self.assertEqual(process["exit_code"], 0 if mode == "success" else 7)
                if mode == "timeout":
                    self.assertIsNotNone(process["failure"])
                else:
                    self.assertIsNone(process["failure"])
                custody = attempt.result["custody"]
                self.assertEqual(custody["status"], "outputs-disposed-evidence-retained")
                self.assertFalse((root / "crates/runner/target").exists())
                self.assertFalse((root / ".chrono-harness/startup").exists())
                self.assertEqual((root / fixture.OPERATION).read_bytes(), source)
                self.assertEqual((root / ".chrono-harness/state/custody-original").read_bytes(), b"original evidence")
                released = json.loads(Path(custody["release_report"]["path"]).read_bytes())
                self.assertTrue(released["main_cache_release"])
                effects = [effect for row in released["drain"] if "receipt" in row
                           for effect in json.loads(attempt.original(row["receipt"], root)).get("artifact_disposals", [])]
                effect = next(row for row in effects if row["path"] == "crates/runner/target/")
                self.assertEqual(effect["status"], "verified-absent")
                self.assertGreater(effect["bytes_before"], 0)
                self.assertEqual(effect["bytes_after"], 0)
                self.assertEqual(attempt.result["custody"]["descendants_quiescent"], True)
                attempt.finish("passed" if mode == "success" else "failed")
                originals = {path: path.read_bytes() for path in attempt.evidence.rglob("*") if path.is_file()}
                for defect in ("running", "false-outcome", "interrupted", "unknown", "missing-release", "altered-release", "missing-stream", "labels-only"):
                    with self.subTest(defect=defect), tempfile.TemporaryDirectory() as directory:
                        owner = Path(directory)
                        pending = owner / "pending-attempt"
                        pending.mkdir()
                        retained = json.loads((attempt.evidence / "result.json").read_bytes())
                        if defect == "false-outcome":
                            retained["status"] = "failed" if mode == "success" else "passed"
                        elif defect in ("running", "interrupted", "unknown"):
                            retained["status"] = defect
                        elif defect == "missing-release":
                            del retained["custody"]["release_report"]
                        elif defect == "altered-release":
                            retained["custody"]["release_report"]["sha256"] = "0" * 64
                        elif defect == "missing-stream":
                            retained["commands"][0]["stdout"]["path"] = str(owner / "absent")
                        else:
                            retained["status"] = "passed"
                            for command in retained["commands"]:
                                command.update(status="passed", exit_code=0)
                            if mode == "success":
                                retained["commands"] = []
                        raw = json.dumps(retained).encode()
                        (pending / "result.json").write_bytes(raw)
                        with self.assertRaisesRegex(ValueError, "unresolved real-consumer attempt"):
                            RealAttempt(owner, {}, time.monotonic() + 5, 1048576)
                        self.assertEqual((pending / "result.json").read_bytes(), raw)
                        self.assertEqual(list(owner.iterdir()), [pending])
                with ThreadPoolExecutor(max_workers=2) as workers:
                    entrants = [workers.submit(RealAttempt, attempt.evidence.parent, {},
                                               time.monotonic() + 5, 1048576) for _ in range(2)]
                admitted = [entry.result() for entry in entrants if entry.exception() is None]
                rejected = [entry.exception() for entry in entrants if entry.exception() is not None]
                self.assertEqual(len(admitted), 1)
                self.assertEqual(len(rejected), 1)
                self.assertIsInstance(rejected[0], ValueError)
                self.assertIn("unresolved real-consumer attempt", str(rejected[0]))
                next_attempt = admitted[0]
                self.assertNotEqual(next_attempt.evidence, attempt.evidence)
                with self.assertRaisesRegex(ValueError, "unresolved real-consumer attempt"):
                    RealAttempt(attempt.evidence.parent, {}, time.monotonic() + 5, 1048576)
                for path, raw in originals.items():
                    self.assertEqual(path.read_bytes(), raw)
                attempt.release_consumers()

    def assert_producer_streams(self, attempt, process):
        import bootstrap
        process = bootstrap.retained_process(process)
        self.assertEqual(process["argv"][0], sys.executable)
        for channel in ("stdout", "stderr"):
            expected = ("custody " + channel + "\n").encode()
            self.assertEqual(bytes(process[channel + "_bytes"]), expected)
            self.assertEqual(process[channel + "_sha256"], hashlib.sha256(expected).hexdigest())
        for row in attempt.result["commands"]:
            for channel in ("stdout", "stderr"):
                self.assertEqual(len(RealAttempt.original(row[channel])), row[channel]["size"])

    @contextmanager
    def live_consumer(self, root, attempt):
        # The actual registered owner, not a mocked lock, protects this consumer.
        command = attempt.managed_argv("fixture.custody")
        stdout, stderr = attempt.evidence / "managed-use.stdout", attempt.evidence / "managed-use.stderr"
        row = {"name": "managed-use", "argv": command, "cwd": str(root), "exit_code": None,
               "status": "running", "ownership_fds": list(fixture.PASS_FDS),
               "output_limit_bytes": attempt.owner_output_limit}
        attempt.result["commands"].append(row)
        attempt.publish()
        started = time.monotonic()
        listener = socket.socket()
        completed, completion = socket.socketpair()
        with listener, completed, completion, ThreadPoolExecutor(max_workers=1) as workers, \
                stdout.open("xb") as out, stderr.open("xb") as err:
            listener.bind(("127.0.0.1", 0))
            listener.listen(1)
            host, port = listener.getsockname()
            channel_path = root / ".chrono-harness/state/custody-channel.json"
            channel_path.parent.mkdir(parents=True, exist_ok=True)
            with channel_path.open("x") as address:
                json.dump({"host": host, "port": port}, address)
            child = subprocess.Popen(command, cwd=root, env=attempt.environment,
                                     pass_fds=fixture.PASS_FDS, stdout=out, stderr=err)
            def join_child():
                try:
                    return child.wait()
                finally:
                    completion.sendall(b"E")

            terminal = workers.submit(join_child)
            channel = None
            try:
                # Admission belongs to the managed owner. Wait for its actual
                # consumer or its joined original exit under this attempt's
                # existing deadline, rather than a separate pre-admission timer.
                with selectors.DefaultSelector() as notification:
                    notification.register(listener, selectors.EVENT_READ)
                    notification.register(completed, selectors.EVENT_READ)
                    events = notification.select(max(0, attempt.deadline - time.monotonic()))
                if not events or any(key.fileobj is completed for key, _ in events):
                    self.fail("live consumer readiness: " + stderr.read_text(errors="replace")[-3000:])
                channel, _ = listener.accept()
                channel.settimeout(max(0.001, min(15, attempt.deadline - time.monotonic())))
                self.assertEqual(channel.recv(1), b"R", "live consumer readiness channel")
                self.assertFalse(terminal.done(), "live consumer exited before protected assertions")
                yield
            finally:
                primary_error = sys.exc_info()[1]
                finalize_error = None
                try:
                    if channel is not None:
                        channel.sendall(b"X")
                except Exception as error:
                    attempt.result["custody"]["live_finalize_error"] = repr(error)
                    finalize_error = error
                finally:
                    if channel is not None:
                        channel.close()
                try:
                    terminal.result(timeout=25)
                except Exception as error:
                    attempt.result["custody"]["live_join_error"] = repr(error)
                    finalize_error = finalize_error or error
                finally:
                    row.update(exit_code=child.returncode, duration_seconds=time.monotonic() - started,
                               status="unresolved" if child.returncode is None else "passed" if child.returncode == 0 else "failed")
                    attempt.retain_streams(row, stdout, stderr)
                    attempt.retain_owner_report("managed-use", "managed_report")
                if primary_error is None and finalize_error is not None:
                    raise finalize_error
            self.assertEqual(child.returncode, 0)

    def test_live_consumer_and_preexisting_output_remain_protected(self):
        root, attempt = self.host("hold")
        with attempt:
            with self.live_consumer(root, attempt):
                with self.assertRaises((AssertionError, ValueError)):
                    attempt.dispose()
                assertions = json.loads((root / ".chrono-harness/state/custody-assertions.json").read_bytes())
                self.assertEqual(len(assertions["verified_payloads"]), 2)
                self.assertEqual(attempt.result["custody"]["status"], "protected-pending")
            self.assertTrue((root / ".chrono-harness/state/custody-release").is_file())
        report = json.loads(RealAttempt.original(attempt.result["custody"]["managed_report"]))
        self.assertEqual(report["managed_process"]["exit_code"], 0)
        self.assertIsNone(report["managed_process"]["failure"])
        self.assert_producer_streams(attempt, report["managed_process"])
        self.assertFalse((root / "crates/runner/target").exists())
        attempt.finish("passed")
        other, pending = self.host("success")
        output = other / "crates/runner/target"
        output.mkdir(parents=True)
        (output / "historical").write_bytes(b"unowned")
        with self.assertRaisesRegex(ValueError, "pre-existing"):
            pending.managed("fixture.custody")
        self.assertEqual((output / "historical").read_bytes(), b"unowned")
        pending.finish("failed", "unowned pre-existing output refused")


class BootstrapCommandMocks(unittest.TestCase):
    """Test producer orchestration with the external command boundary mocked."""

    def setUp(self):
        import bootstrap_shared
        self.shared = bootstrap_shared
        self.temporary = tempfile.TemporaryDirectory(prefix="bootstrap command mocks ")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.observed = {"commit": "a" * 40, "dirty": False, "error": None}
        self.output = self.root / "github-output"
        self.profile = {"entrypoint": {"tool": "bash", "script": ".chrono-harness/ci/bootstrap-native.sh"},
                        "tools": {"bash": "/registered/bash"},
                        "install": [{"destination": ".chrono-harness/bin/tool"}]}
        self.config = {"directory": ".chrono-harness/startup/", "profiles": {
            "core": {"config": ".chrono-harness/ci/bootstrap-core.json", "build": True},
            "cache": {"config": ".chrono-harness/ci/bootstrap-cache.json", "build": False}}}
        self.profiles = {"core": (self.profile, "b" * 64), "cache": ({"install": []}, "c" * 64)}
        self.argv = ["/registered/bash", str(self.root / self.profile["entrypoint"]["script"]),
                     str(self.root), self.config["profiles"]["core"]["config"]]
        self.command = self.patch(self.shared.subprocess, "run")
        self.patch(self.shared, "source", return_value=self.observed)
        self.patch(self.shared, "native_platform", return_value={"os": "mock", "arch": "mock"})
        self.original = self.patch(self.shared, "original_report", return_value=b'{"operations":[]}')
        environment = fixture.mock.patch.dict(os.environ, {"GITHUB_OUTPUT": str(self.output)})
        environment.start()
        self.addCleanup(environment.stop)

    def patch(self, *args, **kwargs):
        patcher = fixture.mock.patch.object(*args, **kwargs)
        result = patcher.start()
        self.addCleanup(patcher.stop)
        return result

    def produce(self):
        return self.shared.produce(self.root, CONFIG, self.config, self.profiles, "d" * 64)

    def test_registered_command_success_exports_mock_producer_bytes(self):
        def installed(*args, **kwargs):
            tool = self.root / ".chrono-harness/bin/tool"
            tool.parent.mkdir(parents=True)
            tool.write_bytes(b"mock producer output")
            tool.chmod(0o755)
        self.command.side_effect = installed
        self.produce()
        self.command.assert_called_once_with(self.argv, check=True, pass_fds=fixture.PASS_FDS)
        transfer = self.root / self.config["directory"]
        raw = (transfer / "manifest.json").read_bytes()
        self.assertEqual(self.output.read_text(), "binding=" + hashlib.sha256(raw).hexdigest() + "\n")
        manifest = json.loads(raw)
        self.assertEqual(manifest["source"], self.observed)
        item = manifest["profiles"]["core"]["files"][0]
        self.assertEqual((transfer / item["path"]).read_bytes(), b"mock producer output")
        self.assertEqual(manifest["profiles"]["cache"]["files"], [])

    def test_command_failure_prevents_export_and_preserves_original_exception(self):
        failures = [subprocess.CalledProcessError(17, self.argv, output=b"partial", stderr=b"mock failure"),
                    subprocess.TimeoutExpired(self.argv, 3, output=b"partial", stderr=b"mock timeout")]
        for failure in failures:
            with self.subTest(failure=type(failure).__name__):
                self.command.reset_mock()
                self.command.side_effect = failure
                with self.assertRaises(type(failure)) as caught:
                    self.produce()
                self.assertIs(caught.exception, failure)
                self.command.assert_called_once_with(self.argv, check=True, pass_fds=fixture.PASS_FDS)
                self.original.assert_not_called()
                self.assertFalse((self.root / self.config["directory"]).exists())
                self.assertFalse(self.output.exists())

    def test_source_change_after_command_prevents_export(self):
        self.patch(self.shared, "source", side_effect=[self.observed, dict(self.observed, dirty=True)])
        with self.assertRaisesRegex(ValueError, "source changed during production"):
            self.produce()
        self.command.assert_called_once_with(self.argv, check=True, pass_fds=fixture.PASS_FDS)
        self.original.assert_not_called()
        self.assertFalse(self.output.exists())


def runner_execution_process(evidence, row):
    """Read the versioned Rust report interface, preserving original byte checks."""
    schema = evidence.get("execution_evidence_schema")
    if schema != "chrono-ci-executions/v2" or "process" in row:
        raise AssertionError("unsupported or duplicate runner execution evidence")
    process = dict(row["receipt"]["process"])
    encoding = process.get("encoding")
    if encoding not in ("chrono-retained-process/v1", "chrono-retained-process/v2"):
        raise AssertionError("unsupported runner process encoding")
    for stream in ("stdout", "stderr"):
        if stream in process or stream + "_bytes" in process:
            raise AssertionError("conflicting runner stream aliases")
        if encoding == "chrono-retained-process/v1":
            raw = bytes.fromhex(process[stream + "_hex"])
        else:
            size = process[stream + "_length"]
            if type(size) is not int or not 0 <= size <= 64 * 1024 * 1024:
                raise AssertionError("runner original stream bound")
            decoder = zlib.decompressobj()
            raw = decoder.decompress(bytes.fromhex(process[stream + "_zlib_hex"]), size + 1)
            if len(raw) != size or not decoder.eof or decoder.unused_data or decoder.unconsumed_tail:
                raise AssertionError("runner original stream length or encoding differs")
        if len(raw) > 64 * 1024 * 1024 or hashlib.sha256(raw).hexdigest() != process[stream + "_sha256"]:
            raise AssertionError("runner original stream identity differs")
        process[stream] = raw.decode("utf-8", errors="replace")
    return process


def consume_runner_operation(response, candidate, registered):
    """Consume selected execution or explicitly exercise the registered action.

    The normal runner response is immutable evidence, including valid no-work.
    This fixture acceptance action never supplies a replacement DELTA report.
    """
    evidence = response["evidence"]
    if response["status"] != "passed" or evidence["candidate"] != candidate:
        raise AssertionError("runner check did not pass for the current candidate")
    if evidence["blocked"]:
        raise AssertionError("runner check has blocked operations")
    runner = [row for row in evidence["executed"] if row["operation"] == "test.runner-tests"]
    if "test:runner-tests" not in evidence["selected"]:
        if runner:
            raise AssertionError("unselected runner tests have executed operations")
        command = registered("test.runner-tests")
        if command["name"] != "test.runner-tests" or command["status"] != "passed" or command["exit_code"] != 0:
            raise AssertionError("independent runner tests did not pass")
        stdout = RealAttempt.original(command["stdout"])
        RealAttempt.original(command["stderr"])
        if b"test result: ok" not in stdout:
            raise AssertionError("independent runner tests did not pass")
        return {"operation": "test.runner-tests", "evidence": "registered-action", "command": command["name"]}
    if len(runner) != 1:
        raise AssertionError("startup alone cannot satisfy the later independent Cargo action")
    process = runner_execution_process(evidence, runner[0])
    if process["exit_code"] != 0 or process["failure"] is not None or "test result: ok" not in process["stdout"]:
        raise AssertionError("independent runner tests did not pass")
    return {"operation": "test.runner-tests", "evidence": "runner-check"}


class RunnerConsumption(unittest.TestCase):
    """Independent selection expectations and real Python subprocess effects."""

    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix="runner consumption ")
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.attempt = RealAttempt(self.root / "evidence", {}, time.monotonic() + 10, 1048576)
        self.response = {"status": "passed", "evidence": {
            "base": "same-fixture-source", "candidate": "same-fixture-source",
            "changed_paths": [], "selected": [], "executed": [], "blocked": []}}
        self.outcome = "success"

    def registered(self, operation):
        self.assertEqual(operation, "test.runner-tests")
        self.attempt.run(operation, [sys.executable, "-B", str(SOURCE / "fixtures/bootstrap-operation.py"),
                                    "runner-consumer", self.outcome], self.root, dict(os.environ))
        return self.attempt.result["commands"][-1]

    def consume(self):
        return consume_runner_operation(self.response, "same-fixture-source", self.registered)

    def selected(self):
        process = {"exit_code": 0, "failure": None}
        for stream, raw in (("stdout", b"test result: ok. 1 passed; 0 failed"), ("stderr", b"")):
            process[stream + "_zlib_hex"] = zlib.compress(raw).hex()
            process[stream + "_length"] = len(raw)
            process[stream + "_sha256"] = hashlib.sha256(raw).hexdigest()
        process["encoding"] = "chrono-retained-process/v2"
        self.response["evidence"].update(base="earlier-fixture-source", changed_paths=["runner-source"],
            execution_evidence_schema="chrono-ci-executions/v2", selected=["test:runner-tests"],
            executed=[{"operation": "test.runner-tests", "receipt": {"process": process}}])

    def test_same_commit_no_work_stays_unchanged_and_runs_registered_consumer(self):
        before = json.dumps(self.response, sort_keys=True)
        result = self.consume()
        self.assertEqual(json.dumps(self.response, sort_keys=True), before)
        self.assertEqual(result, {"operation": "test.runner-tests", "evidence": "registered-action",
                                  "command": "test.runner-tests"})
        self.assertEqual((self.root / "runner-consumed").read_bytes(), b"success")
        self.assertEqual(len(self.attempt.result["commands"]), 1)
        row = self.attempt.result["commands"][0]
        self.assertEqual(row["exit_code"], 0)
        self.assertIn(b"test result: ok", RealAttempt.original(row["stdout"]))
        self.assertEqual(RealAttempt.original(row["stderr"]), b"runner consumer stderr\n")

    def test_unselected_nonempty_delta_also_requires_actual_consumption(self):
        self.response["evidence"].update(base="earlier-fixture-source", changed_paths=["documentation"])
        self.assertEqual(self.consume()["evidence"], "registered-action")
        self.assertEqual((self.root / "runner-consumed").read_bytes(), b"success")

    def test_selected_success_reuses_execution_without_duplicate_action(self):
        self.selected()
        self.assertEqual(self.consume(), {"operation": "test.runner-tests", "evidence": "runner-check"})
        self.assertEqual(self.attempt.result["commands"], [])
        self.assertFalse((self.root / "runner-consumed").exists())

    def test_selected_runner_with_identifiers_reuses_original_runner_execution(self):
        self.selected()
        evidence = self.response["evidence"]
        evidence["selected"].insert(0, "test:identifiers-tests")
        evidence["executed"].insert(0, {"operation": "test.identifiers-tests"})
        before = json.dumps(self.response, sort_keys=True)
        self.assertEqual(self.consume(), {"operation": "test.runner-tests", "evidence": "runner-check"})
        self.assertEqual(json.dumps(self.response, sort_keys=True), before)
        self.assertEqual(self.attempt.result["commands"], [])

    def test_only_identifiers_selected_still_runs_registered_runner_consumer(self):
        self.response["evidence"].update(selected=["test:identifiers-tests"],
            executed=[{"operation": "test.identifiers-tests"}])
        before = json.dumps(self.response, sort_keys=True)
        self.assertEqual(self.consume()["evidence"], "registered-action")
        self.assertEqual(json.dumps(self.response, sort_keys=True), before)
        self.assertEqual((self.root / "runner-consumed").read_bytes(), b"success")
        self.assertEqual(len(self.attempt.result["commands"]), 1)

    def test_current_single_receipt_preserves_originals_and_rejects_drift(self):
        self.selected()
        evidence = self.response["evidence"]
        row = evidence["executed"][0]
        process = row["receipt"]["process"]
        before = json.dumps(self.response, sort_keys=True)
        self.assertEqual(self.consume()["evidence"], "runner-check")
        self.assertEqual(json.dumps(self.response, sort_keys=True), before)
        self.assertEqual(self.attempt.result["commands"], [])
        process["stdout_sha256"] = "0" * 64
        with self.assertRaisesRegex(AssertionError, "identity differs"):
            self.consume()

    def test_missing_duplicate_failed_or_unselected_execution_never_substitutes(self):
        for defect in ("missing", "duplicate", "exit", "failure", "no-tests", "unselected", "blocked",
                       "check-failed", "source-drift", "missing-schema", "duplicate-process"):
            with self.subTest(defect=defect):
                self.selected()
                self.response["status"] = "passed"
                self.response["evidence"].update(candidate="same-fixture-source", blocked=[])
                evidence = self.response["evidence"]
                process = evidence["executed"][0]["receipt"]["process"]
                if defect == "missing": evidence["executed"] = []
                elif defect == "duplicate": evidence["executed"] *= 2
                elif defect == "exit": process["exit_code"] = 7
                elif defect == "failure": process["failure"] = "timeout"
                elif defect == "no-tests":
                    raw = b"startup only"
                    process["stdout_zlib_hex"] = zlib.compress(raw).hex()
                    process["stdout_length"] = len(raw)
                    process["stdout_sha256"] = hashlib.sha256(raw).hexdigest()
                elif defect == "missing-schema": del evidence["execution_evidence_schema"]
                elif defect == "duplicate-process": evidence["executed"][0]["process"] = process
                elif defect == "unselected": evidence["selected"] = []
                elif defect == "blocked": evidence["blocked"] = ["test.runner-tests"]
                elif defect == "check-failed": self.response["status"] = "failed"
                elif defect == "source-drift": evidence["candidate"] = "different-fixture-source"
                with self.assertRaises(AssertionError):
                    self.consume()
                self.assertEqual(self.attempt.result["commands"], [])

    def test_direct_failure_retains_exit_and_original_streams(self):
        self.outcome = "failure"
        with self.assertRaises(AssertionError):
            self.consume()
        row = self.attempt.result["commands"][0]
        self.assertEqual(row["status"], "failed")
        self.assertEqual(row["exit_code"], 7)
        self.assertEqual(RealAttempt.original(row["stdout"]), b"runner consumer failure\n")
        self.assertEqual(RealAttempt.original(row["stderr"]), b"runner consumer stderr\n")

    def test_zero_exit_without_test_result_is_not_consumption(self):
        self.outcome = "no-tests"
        with self.assertRaisesRegex(AssertionError, "independent runner tests did not pass"):
            self.consume()
        self.assertEqual(self.attempt.result["commands"][0]["exit_code"], 0)


if __name__ == "__main__":
    unittest.main(defaultTest="SharedStartup")
