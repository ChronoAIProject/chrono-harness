#!/usr/bin/env python3
"""Exercise the real Python bootstrap entry with registered operations and SDK fixtures."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

sys.dont_write_bytecode = True
SOURCE = Path(__file__).resolve().parent
sys.path.insert(0, str(SOURCE.parent))
from process_fds import inherited_fds

PASS_FDS = inherited_fds()
OPERATION = ".chrono-harness/ci/fixtures/bootstrap-operation.py"


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
    unittest.main()
