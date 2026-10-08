#!/usr/bin/env python3
"""Exercise the real Python bootstrap entry with registered operations and SDK fixtures."""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
import zlib

sys.dont_write_bytecode = True
SOURCE = Path(__file__).resolve().parent
sys.path.insert(0, str(SOURCE.parent))
from process_fds import inherited_fds

PASS_FDS = inherited_fds()
OPERATION = ".chrono-harness/ci/fixtures/bootstrap-operation.py"


class LifecycleTransport(unittest.TestCase):
    def test_original_bytes_and_legacy_streams_and_corruption(self):
        spec = importlib.util.spec_from_file_location("bootstrap_transport", SOURCE / "bootstrap.py")
        owner = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(owner)
        for data in (b"", bytes(range(256)) * 4096, b"original error\xff\x00"):
            original = {"stdout_bytes": list(data), "stdout_sha256": hashlib.sha256(data).hexdigest()}
            self.assertEqual(owner.process_stream(original, "stdout"), data)
            for version in (1, 2):
                process = {"encoding": "chrono-retained-process/v" + str(version),
                           "stdout_sha256": original["stdout_sha256"]}
                if version == 1:
                    process["stdout_hex"] = data.hex()
                else:
                    process.update(stdout_zlib_hex=zlib.compress(data).hex(), stdout_length=len(data))
                self.assertEqual(owner.process_stream(process, "stdout"), data)
                corrupt = dict(process, stdout_sha256="0" * 64)
                with self.assertRaises(ValueError):
                    owner.process_stream(corrupt, "stdout")
                if version == 2:
                    for change in ({"stdout_length": len(data) + 1},
                                   {"stdout_zlib_hex": process["stdout_zlib_hex"] + "00"},
                                   {"stdout_length": 64 * 1024 * 1024 + 1},
                                   {"stdout_bytes": []}):
                        with self.assertRaises(ValueError):
                            owner.process_stream(dict(process, **change), "stdout")


class Bootstrap(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory(prefix="bootstrap host ")
        self.addCleanup(directory.cleanup)
        self.root = Path(directory.name).resolve()
        self.ci = self.root / ".chrono-harness/ci"
        self.ci.mkdir(parents=True)
        shutil.copyfile(SOURCE / "bootstrap.py", self.ci / "bootstrap.py")
        shutil.copyfile(SOURCE.parent / "process_fds.py", self.ci.parent / "process_fds.py")
        (self.ci / "fixtures").mkdir()
        shutil.copyfile(SOURCE / "fixtures/bootstrap-operation.py", self.root / OPERATION)
        self.config = {
            "schema": "chrono-bootstrap/v1", "projects": ".chrono-harness/projects.json",
            "rust_toolchain": "1.95.0", "tools": {"python": sys.executable},
            "operations": ["make.tool"],
            "install": [{"source": "built-tool", "destination": ".chrono-harness/bin/tool"}],
        }
        self.write_json(self.ci / "bootstrap.json", self.config)
        self.registry = self.root / ".chrono-harness/projects.json"
        self.declare("write", "built-tool", "binary")
        self.tools = self.root / "mock-tools"
        self.tools.mkdir()
        sdk = b"#!" + os.fsencode(sys.executable) + b"\n" + (SOURCE / "fixtures/bootstrap-tool.py").read_bytes()
        for name in ["rustup", "cargo", "rustc"]:
            path = self.tools / name
            path.write_bytes(sdk)
            path.chmod(0o755)
        (self.tools / "cargo-ready").touch()
        self.environment = dict(os.environ)
        self.environment["PATH"] = str(self.tools) + os.pathsep + os.environ["PATH"]
        self.environment["GIT_TRACE2_EVENT"] = str(self.tools / "git-trace.jsonl")
        self.state_path = self.root / ".chrono-harness/state/bootstrap.json"

    def write_json(self, path, value):
        path.write_text(json.dumps(value))

    def declarations(self, *args):
        return {"projects": [{"actions": {"build": {
            "operation": "make.tool", "tool": "python", "argv": [OPERATION, *args],
        }}}], "scripts": []}

    def declare(self, *args):
        self.write_json(self.registry, self.declarations(*args))

    def invoke(self, selected=None):
        args = [sys.executable, "-B", str(self.ci / "bootstrap.py"), str(self.root)]
        if selected is not None:
            args.append(selected)
        return subprocess.run(args, cwd=tempfile.gettempdir(), env=self.environment,
                              pass_fds=PASS_FDS, capture_output=True)

    def succeeded(self, result):
        self.assertEqual(result.returncode, 0, (result.stdout, result.stderr))

    def git(self, *args):
        result = subprocess.run(["git", *args], cwd=self.root, pass_fds=PASS_FDS,
                                capture_output=True, check=True)
        return result.stdout.decode().strip()

    def evidence(self):
        return json.loads(self.state_path.read_bytes())

    def calls(self):
        return json.loads((self.tools / "calls.json").read_bytes())

    def observation(self, commit, tree, dirty):
        return {"commit": commit, "tree": tree, "dirty": dirty, "error": None}

    def test_selected_report_survives_a_later_bootstrap(self):
        selected = dict(self.config, report_path=".chrono-harness/state/cache/bootstrap.json")
        self.write_json(self.ci / "cache.json", selected)
        self.succeeded(self.invoke(".chrono-harness/ci/cache.json"))
        retained = self.root / selected["report_path"]
        self.assertTrue(retained.is_file(), "selected bootstrap evidence was not retained")
        original = retained.read_bytes()
        result = json.loads(original)
        self.assertEqual(result["config"], ".chrono-harness/ci/cache.json")
        self.assertEqual(result["installed"][0]["sha256"], hashlib.sha256(b"binary").hexdigest())
        self.assertFalse(self.state_path.exists(), "selected report overwrote the default slot")

        self.declare("write", "built-tool", "later binary")
        self.succeeded(self.invoke())
        self.assertEqual(retained.read_bytes(), original)
        self.assertEqual(self.evidence()["config"], ".chrono-harness/ci/bootstrap.json")
        self.assertEqual(self.evidence()["installed"][0]["sha256"],
                         hashlib.sha256(b"later binary").hexdigest())

    def test_invalid_report_location_stops_before_tools(self):
        for value in [None, 1, [], "", "/tmp/bootstrap.json", "bootstrap.json",
                      ".chrono-harness/ci/overwrite.json", ".chrono-harness/state/../source",
                      ".chrono-harness/state", ".chrono-harness/state/", ".chrono-harness/state//a.json"]:
            with self.subTest(value=value):
                self.write_json(self.ci / "bootstrap.json", dict(self.config, report_path=value))
                result = self.invoke()
                self.assertNotEqual(result.returncode, 0)
                self.assertIn(b"bootstrap report must be an explicit host state file", result.stderr)
                self.assertFalse((self.tools / "calls.json").exists())

    def test_report_write_failure_is_not_success(self):
        selected = dict(self.config, report_path=".chrono-harness/state/blocked/result.json")
        self.write_json(self.ci / "bootstrap.json", selected)
        blocked = self.root / ".chrono-harness/state/blocked"
        blocked.parent.mkdir()
        blocked.write_bytes(b"original evidence")
        result = self.invoke()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"E_BOOTSTRAP:", result.stderr)
        self.assertEqual(blocked.read_bytes(), b"original evidence")
        self.assertFalse(self.state_path.exists())

    def test_report_symlinks_do_not_redirect_evidence(self):
        state = self.state_path.parent
        state.mkdir()
        other = self.root / "other"
        other.mkdir()
        source = other / "source"
        source.write_bytes(b"keep source")
        for is_directory in [True, False]:
            with self.subTest(is_directory=is_directory):
                alias = state / "alias"
                alias.symlink_to(other if is_directory else source)
                selected = ".chrono-harness/state/alias" + ("/source" if is_directory else "")
                self.write_json(self.ci / "bootstrap.json", dict(self.config, report_path=selected))
                result = self.invoke()
                self.assertNotEqual(result.returncode, 0)
                self.assertIn(b"bootstrap report path must not contain a symlink", result.stderr)
                self.assertFalse((self.tools / "calls.json").exists())
                self.assertEqual(source.read_bytes(), b"keep source")
                alias.unlink()

    def test_registered_incremental_reaches_child_and_result(self):
        self.declare("environment", "built-tool", "CARGO_INCREMENTAL")
        for declared, ambient, expected in [(True, "0", "1"), (False, "1", "0"), (None, "0", "0")]:
            with self.subTest(declared=declared):
                config = dict(self.config)
                if declared is not None:
                    config["rust_incremental"] = declared
                self.write_json(self.ci / "bootstrap.json", config)
                self.environment["CARGO_INCREMENTAL"] = ambient
                self.succeeded(self.invoke())
                child = json.loads((self.root / ".chrono-harness/bin/tool").read_bytes())
                self.assertEqual(child["CARGO_INCREMENTAL"], expected)
                self.assertEqual(self.evidence()["environment"]["CARGO_INCREMENTAL"], expected)

    def test_invalid_incremental_declared_value_stops_before_tools(self):
        for value in [None, 0, 1, "true", [], {}]:
            with self.subTest(value=value):
                self.write_json(self.ci / "bootstrap.json", dict(self.config, rust_incremental=value))
                result = self.invoke()
                self.assertNotEqual(result.returncode, 0)
                self.assertIn(b"rust_incremental must be a boolean", result.stderr)
                self.assertFalse((self.tools / "calls.json").exists())
                self.assertFalse(self.state_path.exists())

    def test_missing_distribution_declaration_stops_before_source_operations(self):
        self.write_json(self.ci / "bootstrap.json", dict(
            self.config, distribution={"manifest": ".chrono-harness/distribution-release.json"}))
        result = self.invoke()
        self.assertNotEqual(result.returncode, 0, "missing immutable inputs were ignored")
        self.assertFalse((self.root / "built-tool").exists())
        self.assertFalse(self.state_path.exists())

    def source_equals(self, expected):
        # Compare JSON types too: Python alone equates False with numeric zero.
        self.assertEqual(json.dumps(self.evidence()["source"], sort_keys=True),
                         json.dumps(expected, sort_keys=True))

    def test_registered_operations_and_failure_propagation(self):
        self.succeeded(self.invoke())
        self.assertTrue((self.tools / "fmt-ready").exists(), "minimal SDK lacks rustfmt")
        self.assertIn(["rustup", "component", "add", "--toolchain", "1.95.0", "rustfmt"], self.calls())
        self.assertEqual((self.root / ".chrono-harness/bin/tool").read_bytes(), b"binary")
        state = self.evidence()
        self.assertEqual(state["installed"][0]["sha256"], hashlib.sha256(b"binary").hexdigest())
        self.assertEqual(state["schema"], "chrono-bootstrap-result/v1")
        self.assertEqual(state["versions"]["cargo"], "cargo 1.95.0 (fixture)")
        self.assertEqual(state["versions"]["rustc"], "rustc 1.95.0 (fixture)")
        self.assertEqual(state["source"]["state"], "unavailable")
        self.assertIsNone(state["source"]["commit"])
        self.assertIsNone(state["source"]["tree"])
        for phase in ["before", "after"]:
            observed = state["source"][phase]
            self.assertIsNone(observed["commit"])
            self.assertIsNone(observed["tree"])
            self.assertIsNone(observed["dirty"])
            self.assertIsInstance(observed["error"], str)
            self.assertTrue(observed["error"])

        (self.root / ".gitignore").write_text(
            "/built-tool\n/selected-tool\n/.chrono-harness/bin/\n/.chrono-harness/state/\n/mock-tools/\n")
        (self.root / "product-source").write_text("source")
        self.git("init", "-q")
        self.git("config", "user.email", "fixture@example.invalid")
        self.git("config", "user.name", "Fixture")
        self.git("add", ".")
        self.git("commit", "-qm", "bootstrap source")
        commit, tree = self.git("rev-parse", "HEAD"), self.git("rev-parse", "HEAD^{tree}")
        trace = self.tools / "git-trace.jsonl"
        trace.write_text("")
        self.succeeded(self.invoke())
        observed = self.observation(commit, tree, False)
        self.source_equals({
            "state": "clean", "commit": commit, "tree": tree, "before": observed, "after": observed})
        acquisitions = sum(json.loads(line)["event"] == "start" for line in trace.read_text().splitlines())
        self.assertEqual(acquisitions, 8, "each source observation reads root/HEAD, fixed tree, dirt and live HEAD")

        for file, staged in [("product-source", False), ("product-source", True), ("untracked-source", False)]:
            with self.subTest(file=file, staged=staged):
                (self.root / file).write_text("dirty source")
                if staged:
                    self.git("add", file)
                self.succeeded(self.invoke())
                observed = self.observation(commit, tree, True)
                self.source_equals({
                    "state": "dirty", "commit": None, "tree": None, "before": observed, "after": observed})
                if file == "product-source":
                    self.git("restore", "--staged", "--worktree", file)
                else:
                    (self.root / file).unlink()

        selected = dict(self.config, operations=["make.selected"], install=[
            {"source": "selected-tool", "destination": ".chrono-harness/bin/selected"}])
        self.write_json(self.ci / "selected profile.json", selected)
        declarations = self.declarations("fail", "99")
        declarations["projects"][0]["actions"]["selected"] = {
            "operation": "make.selected", "tool": "python",
            "argv": [OPERATION, "write", "selected-tool", "selected"],
        }
        self.write_json(self.registry, declarations)
        (self.root / "built-tool").unlink()
        self.succeeded(self.invoke(".chrono-harness/ci/selected profile.json"))
        self.assertFalse((self.root / "built-tool").exists())
        self.assertEqual((self.root / ".chrono-harness/bin/selected").read_bytes(), b"selected")
        self.assertEqual((self.root / ".chrono-harness/bin/tool").read_bytes(), b"binary")
        self.assertEqual(state["config"], ".chrono-harness/ci/bootstrap.json")
        selected_state = self.state_path.read_bytes()
        selected_result = json.loads(selected_state)
        self.assertEqual(selected_result["config"], ".chrono-harness/ci/selected profile.json")
        self.assertEqual(len(selected_result["installed"]), 1)
        self.assertEqual(selected_result["installed"][0]["sha256"], hashlib.sha256(b"selected").hexdigest())
        before_calls = self.calls()
        for invalid in ["/tmp/config.json", ".chrono-harness/ci/../outside.json", "outside.json"]:
            with self.subTest(invalid=invalid):
                rejected = self.invoke(invalid)
                self.assertNotEqual(rejected.returncode, 0)
                self.assertIn(b"explicit host CI path", rejected.stderr)
                self.assertEqual(self.calls(), before_calls, "invalid config launched a tool")
                self.assertEqual(self.state_path.read_bytes(), selected_state)

        self.declare("move-head", "built-tool", "binary")
        self.git("add", ".")
        self.git("commit", "-qm", "moving operation")
        before_commit, before_tree = self.git("rev-parse", "HEAD"), self.git("rev-parse", "HEAD^{tree}")
        self.succeeded(self.invoke())
        after_commit, after_tree = self.git("rev-parse", "HEAD"), self.git("rev-parse", "HEAD^{tree}")
        self.assertNotEqual(before_commit, after_commit)
        self.assertNotEqual(before_tree, after_tree)
        self.source_equals({
            "state": "changed", "commit": None, "tree": None,
            "before": self.observation(before_commit, before_tree, False),
            "after": self.observation(after_commit, after_tree, False),
        })

        self.declare("lose-git", "built-tool", "binary")
        self.git("add", ".")
        self.git("commit", "-qm", "unavailable operation")
        before_commit, before_tree = self.git("rev-parse", "HEAD"), self.git("rev-parse", "HEAD^{tree}")
        self.succeeded(self.invoke())
        unavailable = self.evidence()["source"]
        self.assertEqual(unavailable["state"], "unavailable")
        self.assertIsNone(unavailable["commit"])
        self.assertIsNone(unavailable["tree"])
        self.assertEqual(json.dumps(unavailable["before"], sort_keys=True),
                         json.dumps(self.observation(before_commit, before_tree, False), sort_keys=True))
        for key in ["commit", "tree", "dirty"]:
            self.assertIsNone(unavailable["after"][key])
        self.assertIsInstance(unavailable["after"]["error"], str)
        (self.root / ".saved-git").rename(self.root / ".git")

        last_success = self.state_path.read_bytes()
        self.declare("fail", "9")
        failed = self.invoke()
        self.assertNotEqual(failed.returncode, 0)
        self.assertIn(b"exit status 9", failed.stderr)
        self.assertEqual(self.state_path.read_bytes(), last_success)
        self.declare("write", "built-tool", "binary")
        (self.tools / "cargo-ready").unlink()
        (self.tools / "fmt-ready").unlink()
        self.succeeded(self.invoke())
        self.assertIn(["rustup", "toolchain", "install", "1.95.0", "--profile", "minimal", "--component", "rustfmt"], self.calls())
        (self.tools / "fmt-ready").unlink()
        (self.tools / "fail-component").touch()
        (self.root / "built-tool").unlink()
        self.assertNotEqual(self.invoke().returncode, 0)
        self.assertFalse((self.root / "built-tool").exists(), "operations ran despite component failure")


if __name__ == "__main__":
    from bootstrap_shared_tests import SharedStartup, DistributionStartup

    unittest.main()
