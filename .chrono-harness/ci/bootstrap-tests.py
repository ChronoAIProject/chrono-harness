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
from unittest import mock

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

    def binding_config(self):
        bindings = {}
        for tool in ["cargo", "rustc"]:
            path = self.tools / tool
            bindings[tool] = {
                "path": str(path),
                "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
                "version": tool + " 1.95.0 (fixture)",
            }
        config = dict(self.config)
        config["tools"] = {"python": sys.executable, "cargo": str(self.tools / "cargo"),
                            "rustc": str(self.tools / "rustc")}
        config["tool_bindings"] = bindings
        return config

    def test_relocation_ignores_only_retired_declaration_paths(self):
        config = self.binding_config()
        for name in ("cargo", "rustc"):
            config["tool_bindings"][name]["allow_relocation"] = True
        local = self.root / "relocated"
        shutil.copytree(self.tools, local)
        self.environment["PATH"] = str(local) + os.pathsep + os.environ["PATH"]
        for state in ("stale", "absent"):
            with self.subTest(state=state):
                for name in ("cargo", "rustc"):
                    old = self.tools / name
                    if state == "stale": old.write_bytes(b"retired, not executable")
                    else: old.unlink()
                self.write_json(self.ci / "bootstrap.json", config)
                self.succeeded(self.invoke())
                self.assertEqual(self.evidence()["selection"]["cargo"], str(local / "cargo"))
        # A separately configured current selection remains authoritative.
        other = self.root / "other-current"
        other.mkdir()
        cargo = other / "cargo"
        cargo.write_bytes((local / "cargo").read_bytes() + b"\n# different bytes, same version\n")
        cargo.chmod(0o755)
        config["tools"]["cargo"] = str(cargo)
        self.write_json(self.ci / "bootstrap.json", config)
        rejected = self.invoke()
        self.assertNotEqual(rejected.returncode, 0)
        self.assertIn(b"configured cargo selection differs", rejected.stderr)
        config["tools"]["cargo"] = str(self.root / "missing-current-cargo")
        self.write_json(self.ci / "bootstrap.json", config)
        rejected = self.invoke()
        self.assertNotEqual(rejected.returncode, 0)
        self.assertIn(b"configured cargo executable is missing", rejected.stderr)

    def test_exact_path_rejects_equal_byte_relocation(self):
        config = self.binding_config()
        local = self.root / "relocated"
        shutil.copytree(self.tools, local)
        self.environment["PATH"] = str(local) + os.pathsep + os.environ["PATH"]
        self.write_json(self.ci / "bootstrap.json", config)
        rejected = self.invoke()
        self.assertNotEqual(rejected.returncode, 0)
        self.assertIn(b"relocation is not permitted", rejected.stderr)
        self.assertFalse((self.root / "built-tool").exists())

    def test_native_registry_selects_independent_tuple_and_rejects_missing_before_tools(self):
        spec = importlib.util.spec_from_file_location("native_bootstrap", SOURCE / "bootstrap.py")
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        registry = {"schema": "chrono-bootstrap-toolchains/v1", "rust_toolchain": "1.95.0",
                    "platforms": {}}
        for system, machine, byte in [("Darwin", "arm64", "a"), ("Linux", "x86_64", "b")]:
            registry["platforms"][system + "/" + machine] = {
                "target": "fixture", "tools": {name: {"sha256": byte * 64,
                    "version": name + " 1.95.0 (fixture)"} for name in ("cargo", "rustc")}}
        path = ".chrono-harness/ci/native.json"
        self.write_json(self.root / path, registry)
        config = dict(self.config, require_tool_bindings=True, tool_bindings_registry=path)
        for system, machine, byte in [("Darwin", "arm64", "a"), ("Linux", "x86_64", "b")]:
            with mock.patch.object(module.platform, "system", return_value=system), mock.patch.object(module.platform, "machine", return_value=machine):
                actual = module.binding_declarations(self.root, config)
                self.assertEqual(actual["cargo"]["sha256"], byte * 64)
                self.assertEqual(actual["rustc"]["version"], "rustc 1.95.0 (fixture)")
        for defect in ("unknown", "missing", "conflict"):
            with self.subTest(defect=defect), mock.patch.object(module.platform, "system", return_value="Linux"), mock.patch.object(module.platform, "machine", return_value="x86_64"), mock.patch.object(module.subprocess, "run") as run:
                candidate = dict(config)
                data = json.loads(json.dumps(registry))
                if defect == "unknown": data["platforms"].pop("Linux/x86_64")
                elif defect == "missing": candidate["tool_bindings_registry"] += ".missing"
                else: candidate["tool_bindings"] = self.binding_config()["tool_bindings"]
                self.write_json(self.root / path, data)
                with self.assertRaises((ValueError, OSError)):
                    module.toolchain_environment(candidate, self.root)
                run.assert_not_called()

    def test_bound_executables_are_selected_and_reported(self):
        config = self.binding_config()
        config["operations"] = ["make.cargo"]
        self.declare("write", "built-tool", "bound")
        declarations = self.declarations("write", "built-tool", "bound")
        declarations["projects"][0]["actions"]["cargo"] = {
            "operation": "make.cargo", "tool": "cargo", "argv": ["build-fixture", "built-tool"],
        }
        self.write_json(self.registry, declarations)
        self.write_json(self.ci / "bootstrap.json", config)
        self.succeeded(self.invoke())
        state = self.evidence()
        self.assertEqual([row["version"] for row in state["executables"]],
                         ["cargo 1.95.0 (fixture)", "rustc 1.95.0 (fixture)"])
        self.assertEqual(state["selection"]["compiler"], str(self.tools / "rustc"))
        self.assertIn(["cargo", "build-fixture", "built-tool"], self.calls())
        self.assertIn(["rustc", "compile-fixture", "built-tool"], self.calls())
        child = json.loads((self.root / ".chrono-harness/bin/tool").read_bytes())
        self.assertEqual(child, {"compiler": str(self.tools / "rustc"), "wrapper": "", "workspace_wrapper": ""})
        operation = state["operations"][0]
        self.assertEqual(operation["operation"], "make.cargo")
        self.assertEqual(operation["argv"], [str(self.tools / "cargo"), "build-fixture", "built-tool"])
        self.assertEqual(operation["exit_code"], 0)
        self.assertEqual((self.root / operation["stdout"]["path"]).read_bytes(), b"cargo original stdout\n")
        self.assertEqual((self.root / operation["stderr"]["path"]).read_bytes(), b"cargo original stderr\n")

    def test_failed_operation_retains_original_process_without_installing(self):
        self.declare("fail", "9")
        failed = self.invoke()
        self.assertNotEqual(failed.returncode, 0)
        self.assertFalse(self.state_path.exists())
        originals = list((self.root / ".chrono-harness/state").glob("bootstrap-*/report.json"))
        self.assertEqual(len(originals), 1)
        report = json.loads(originals[0].read_bytes())
        self.assertEqual(report["status"], "failed")
        operation = report["operations"][0]
        self.assertEqual(operation["exit_code"], 9)
        for channel, expected in [("stdout", b"failure stdout\x00\xff\n"), ("stderr", b"failure stderr\xfe\n")]:
            raw = (self.root / operation[channel]["path"]).read_bytes()
            self.assertEqual(raw, expected)
            self.assertEqual(hashlib.sha256(raw).hexdigest(), operation[channel]["sha256"])
        self.assertFalse((self.root / ".chrono-harness/bin/tool").exists())

    def test_binding_mismatch_rejects_before_registered_operation(self):
        config = self.binding_config()
        config["tool_bindings"]["rustc"]["sha256"] = "0" * 64
        self.write_json(self.ci / "bootstrap.json", config)
        result = self.invoke()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"bound rustc identity mismatch", result.stderr)
        self.assertFalse((self.root / "built-tool").exists())

    def test_required_binding_declaration_cannot_be_omitted(self):
        config = dict(self.config, require_tool_bindings=True)
        self.write_json(self.ci / "bootstrap.json", config)
        result = self.invoke()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"requires declared Cargo/rustc bindings", result.stderr)
        self.assertFalse((self.tools / "calls.json").exists())

    def test_unsupported_native_registration_and_compiler_override_reject_before_build(self):
        config = self.binding_config()
        for defect in ("missing-native", "unsupported-native", "compiler", "wrapper", "workspace-wrapper"):
            with self.subTest(defect=defect):
                candidate = dict(config)
                environment = dict(self.environment)
                if defect in ("missing-native", "unsupported-native"):
                    candidate["require_tool_bindings"] = True
                    if defect == "unsupported-native":
                        candidate["native_platform"] = {"system": "unsupported", "machine": "unsupported"}
                else:
                    key = {"compiler": "RUSTC", "wrapper": "RUSTC_WRAPPER", "workspace-wrapper": "RUSTC_WORKSPACE_WRAPPER"}[defect]
                    self.environment[key] = "/different/compiler"
                self.write_json(self.ci / "bootstrap.json", candidate)
                rejected = self.invoke()
                self.assertNotEqual(rejected.returncode, 0)
                self.assertFalse((self.root / "built-tool").exists())
                self.environment = environment

    def test_persistent_bound_executable_drift_rejects_after_operation(self):
        config = self.binding_config()
        config["operations"] = ["make.mutate"]
        declarations = self.declarations("mutate")
        declarations["projects"][0]["actions"]["mutate"] = {
            "operation": "make.mutate", "tool": "python", "argv": [OPERATION, "mutate"],
        }
        self.write_json(self.registry, declarations)
        (self.root / "built-tool").write_bytes(b"keep")
        self.environment["BOUND_CARGO"] = str(self.tools / "cargo")
        self.write_json(self.ci / "bootstrap.json", config)
        result = self.invoke()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"bound cargo identity mismatch", result.stderr)
        self.assertFalse(self.state_path.exists())
        originals = list((self.root / ".chrono-harness/state").glob("bootstrap-*/report.json"))
        self.assertEqual(len(originals), 1)
        failed = json.loads(originals[0].read_bytes())
        self.assertEqual(failed["status"], "failed")
        self.assertEqual(failed["operations"][0]["exit_code"], 0,
                         "a successful child does not waive persistent executable drift")
        self.assertEqual(failed["executables"][0]["sha256"], config["tool_bindings"]["cargo"]["sha256"])

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
    if sys.argv[1:] == ["--real-consumer"]:
        from bootstrap_shared_tests import RealSharedStartup
        unittest.main(argv=[sys.argv[0]], defaultTest="RealSharedStartup")
    else:
        from bootstrap_shared_tests import SharedStartup, AttemptCustody
        unittest.main()
