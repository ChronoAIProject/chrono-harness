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


class NativeRuntimeLocations(unittest.TestCase):
    """Own /usr's Python components, never /usr; keep split runtimes and errors."""

    def setUp(self):
        path = SOURCE.parent / "inputs/python_probe.py"
        spec = importlib.util.spec_from_file_location("native_runtime_locations", path)
        self.probe = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.probe)

    def test_linux_prefix_is_not_the_runtime_and_split_components_are_retained(self):
        paths = {"stdlib": "/usr/lib/python3.12", "platstdlib": "/usr/lib64/python3.12"}
        config = {"DESTSHARED": "/usr/lib64/python3.12/lib-dynload",
                  "Py_ENABLE_SHARED": 1, "LIBDIR": "/usr/lib/x86_64-linux-gnu",
                  "LDLIBRARY": "libpython3.12.so.1.0"}
        with mock.patch.object(self.probe.sys, "base_prefix", "/usr"), \
                mock.patch.object(self.probe.sysconfig, "get_path", side_effect=paths.get), \
                mock.patch.object(self.probe.sysconfig, "get_config_var", side_effect=config.get):
            self.assertEqual(self.probe.location(["root"]), "/usr/lib/python3.12")
            self.assertEqual(self.probe.location(["platstdlib"]), "/usr/lib64/python3.12")
            self.assertEqual(self.probe.location(["extensions"]), config["DESTSHARED"])
            self.assertEqual(self.probe.location(["shared-library"]),
                             "/usr/lib/x86_64-linux-gnu/libpython3.12.so.1.0")

    def test_framework_and_static_runtime_ownership_and_missing_config(self):
        config = {"PYTHONFRAMEWORK": "Python3", "PYTHONFRAMEWORKPREFIX": "/Developer/Library/Frameworks",
                  "VERSION": "3.9"}
        with mock.patch.object(self.probe.sysconfig, "get_config_var", side_effect=config.get):
            self.assertEqual(self.probe.location(["shared-library"]),
                             "/Developer/Library/Frameworks/Python3.framework/Versions/3.9/Python3")
            config.clear()
            self.assertEqual(self.probe.location(["shared-library"]), os.path.realpath(sys.executable))
            with self.assertRaises(ValueError):
                self.probe.location(["extensions"])
            config.update(Py_ENABLE_SHARED=1, LIBDIR="/lib", LDLIBRARY="../outside.so")
            with self.assertRaises(ValueError):
                self.probe.location(["shared-library"])
        with mock.patch.object(self.probe.sysconfig, "get_path", return_value=None):
            with self.assertRaises(ValueError):
                self.probe.location(["root"])

    def test_configuration_locations_respect_private_homes_without_reading_contents(self):
        with mock.patch.dict(os.environ, {"HOME": "/home/fixture"}, clear=True):
            self.assertEqual(self.probe.location(["home"]), "/home/fixture")
            self.assertEqual(self.probe.location(["home-parent"]), "/home")
            self.assertEqual(self.probe.location(["cargo-home"]), "/home/fixture/.cargo")
            self.assertEqual(self.probe.location(["rustup-home"]), "/home/fixture/.rustup")
            with mock.patch.dict(os.environ, {"CARGO_HOME": "/private/cargo", "RUSTUP_HOME": "/private/rustup"}):
                self.assertEqual(self.probe.location(["cargo-home"]), "/private/cargo")
                self.assertEqual(self.probe.location(["rustup-home"]), "/private/rustup")
            with mock.patch.dict(os.environ, {"RUSTUP_HOME": "relative"}):
                self.assertEqual(self.probe.location(["rustup-home"]), "relative")

    def test_actual_cli_selects_this_interpreters_stdlib_and_rejects_unknown_mode(self):
        import sysconfig
        path = SOURCE.parent / "inputs/python_probe.py"
        result = subprocess.run([sys.executable, "-B", str(path), "root"],
                                capture_output=True, pass_fds=PASS_FDS)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.decode(), sysconfig.get_path("stdlib") + "\n")
        rejected = subprocess.run([sys.executable, "-B", str(path), "unknown"],
                                  capture_output=True, pass_fds=PASS_FDS)
        self.assertNotEqual(rejected.returncode, 0)
        self.assertEqual(rejected.stdout, b"")
        self.assertIn(b"usage:", rejected.stderr)


class NativeAcquisitionForwarding(unittest.TestCase):
    """Behavior of the host composite's Bash forwarding; Shell/Python pairing.

    Expectations precede execution: preserve binary streams and the real exit,
    including interpreter lookup failure before a bootstrap report exists.
    Reentry must preserve originals and must not launch the child again.
    This fixture supplies an external command, not a native bootstrap result.
    """

    def setUp(self):
        self.root = Path(tempfile.mkdtemp(prefix="native acquisition forwarding "))
        self.retain_failed_fixture = False
        self.addCleanup(self.cleanup_fixture)
        source = (SOURCE / "native-acquisition/action.yml").read_text()
        # Exercise the production Bash function, supplying only its child argv.
        # The surrounding workflow/action metadata is forwarding, not a test oracle.
        body = source.split("      run: |\n", 1)[1].split("    - name:", 1)[0]
        body = "\n".join(line[8:] for line in body.splitlines())
        self.capture = body.split("\nretain ", 1)[0] + '\nretain "$@"\n'
        self.output = self.root / "original evidence"
        event = self.root / "event.json"
        self.event_bytes = b' {"fixture": "original event bytes"}\n'
        event.write_bytes(self.event_bytes)
        self.environment = dict(os.environ, GITHUB_EVENT_PATH=str(event),
                                GITHUB_RUN_ID="fixture-run", GITHUB_RUN_ATTEMPT="1",
                                GITHUB_JOB="acquire", GITHUB_SHA="fixture-candidate",
                                GITHUB_REF="refs/heads/integration/q3-native-acquisition",
                                CHRONO_NATIVE_WORKFLOW_REVISION="fixture-workflow",
                                GITHUB_EVENT_NAME="push", RUNNER_OS="fixture-os",
                                RUNNER_ARCH="fixture-arch")

    def cleanup_fixture(self):
        if not self.retain_failed_fixture:
            shutil.rmtree(self.root)

    def invoke(self, argv):
        return subprocess.run(["/bin/bash", "--noprofile", "--norc", "-e", "-o", "pipefail",
                               "-c", self.capture, "native-acquisition", str(self.output), *argv],
                              cwd=self.root, env=self.environment, pass_fds=PASS_FDS,
                              capture_output=True)

    def test_success_and_original_failure_preserve_binary_streams_and_exit(self):
        for code in (0, 17):
            with self.subTest(code=code):
                self.output = self.root / ("original evidence " + str(code))
                argv = [sys.executable, "-B", str(SOURCE / "fixtures/bootstrap-operation.py"),
                        "fail", str(code)]
                result = self.invoke(argv)
                self.assertEqual(result.returncode, code, result)
                self.assertEqual((self.output / "job.exit").read_bytes(), str(code).encode() + b"\n")
                self.assertEqual((self.output / "job.stdout").read_bytes(), b"failure stdout\x00\xff\n")
                self.assertEqual((self.output / "job.stderr").read_bytes(), b"failure stderr\xfe\n")
                self.assertEqual((self.output / "job.argv").read_bytes(),
                                 b"\0".join(os.fsencode(arg) for arg in argv) + b"\0")
                self.assertEqual((self.output / "event.json").read_bytes(), self.event_bytes)
                self.assertIn(b"candidate\tfixture-candidate\n",
                              (self.output / "job.identity.tsv").read_bytes())
                self.assertFalse((self.output / "report.json").exists())

    def test_missing_interpreter_preserves_shell_failure_without_a_bootstrap_report(self):
        result = self.invoke([str(self.root / "missing interpreter"), "-B", "bootstrap.py"])
        try:
            self.assertEqual(result.returncode, 127, result)
            self.assertEqual((self.output / "job.exit").read_bytes(), b"127\n")
            self.assertEqual((self.output / "job.stdout").read_bytes(), b"")
            self.assertIn(b"missing interpreter", (self.output / "job.stderr").read_bytes())
            self.assertFalse((self.output / "report.json").exists())
        except Exception:
            # Keep the actual inner originals and outer shell streams until the
            # caller finishes debugging/acceptance; successful fixtures are removed.
            self.retain_failed_fixture = True
            print("native acquisition forwarding fixture originals: " + str(self.root), flush=True)
            (self.root / "retain.bash").write_text(self.capture)
            (self.root / "shell.stdout").write_bytes(result.stdout)
            (self.root / "shell.stderr").write_bytes(result.stderr)
            (self.root / "shell.exit").write_text(str(result.returncode) + "\n")
            raise

    def test_reentry_preserves_originals_and_does_not_run_another_child(self):
        first = self.invoke([sys.executable, "-B", str(SOURCE / "fixtures/bootstrap-operation.py"),
                             "fail", "17"])
        self.assertEqual(first.returncode, 17, first)
        originals = {path.name: path.read_bytes() for path in self.output.iterdir()}
        marker = self.root / "must not be produced"
        second = self.invoke([sys.executable, "-B", str(SOURCE / "fixtures/bootstrap-operation.py"),
                              "write", str(marker), "new child"])
        self.assertNotEqual(second.returncode, 0, second)
        self.assertFalse(marker.exists())
        self.assertEqual({path.name: path.read_bytes() for path in self.output.iterdir()}, originals)

    def test_partial_original_event_is_not_overwritten_or_relabelled(self):
        self.output.mkdir()
        original = self.output / "event.json"
        original.write_bytes(b"retained partial original event\n")
        marker = self.root / "must not be produced"
        result = self.invoke([sys.executable, "-B", str(SOURCE / "fixtures/bootstrap-operation.py"),
                              "write", str(marker), "new child"])
        self.assertNotEqual(result.returncode, 0, result)
        self.assertFalse(marker.exists())
        self.assertEqual(original.read_bytes(), b"retained partial original event\n")
        self.assertFalse((self.output / "job.exit").exists())


class Bootstrap(unittest.TestCase):
    def test_native_input_tool_probe_preserves_selection_and_missing_tool_failure(self):
        probe = SOURCE.parent / "inputs/python_probe.py"
        env = dict(os.environ, PATH=str(Path(sys.executable).parent))
        selected = subprocess.run([sys.executable, "-B", str(probe), "tool", Path(sys.executable).name],
                                  env=env, capture_output=True)
        self.assertEqual(selected.returncode, 0, selected.stderr)
        self.assertEqual(selected.stdout.decode().strip(), os.path.realpath(sys.executable))
        missing = subprocess.run([sys.executable, "-B", str(probe), "tool", "chrono-missing-native-tool"],
                                 env=dict(env, PATH=""), capture_output=True)
        self.assertNotEqual(missing.returncode, 0)
        self.assertEqual(missing.stdout, b"")
        self.assertIn(b"missing selected tool", missing.stderr)

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
        # Synthetic bindings own compiler selection; conflict cases set overrides explicitly.
        for key in ("RUSTC", "RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER"):
            self.environment.pop(key, None)
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
        self.assertEqual(result.returncode, 0, result)

    def test_subprocess_failure_diagnostic_preserves_command_exit_and_binary_streams(self):
        argv = [sys.executable, "-B", str(self.root / OPERATION), "fail", "7"]
        result = subprocess.run(argv, capture_output=True, pass_fds=PASS_FDS)
        with self.assertRaises(AssertionError) as failed:
            self.succeeded(result)
        message = str(failed.exception)
        for expected in (repr(argv), "returncode=7", repr(b"failure stdout\x00\xff\n"),
                         repr(b"failure stderr\xfe\n")):
            self.assertIn(expected, message)

    def git(self, *args):
        result = subprocess.run(["git", *args], cwd=self.root, pass_fds=PASS_FDS,
                                capture_output=True)
        self.succeeded(result)
        return result.stdout.decode().strip()

    def evidence(self):
        return json.loads(self.state_path.read_bytes())

    def calls(self):
        return json.loads((self.tools / "calls.json").read_bytes())

    def observation(self, commit, tree, dirty):
        return {"commit": commit, "tree": tree, "dirty": dirty, "error": None}

    def acquisition_config(self, mode="acquisition"):
        built = "crates/inputs/target/debug/fixture-inputs"
        installed = ".chrono-harness/bin/chrono-worktree"
        original = ".chrono-harness/state/native/fixture/acquisition.json"
        interface = self.ci / "fixtures/acquirer"
        interface.write_bytes(b"#!" + os.fsencode(sys.executable) + b"\n" +
                              (self.root / OPERATION).read_bytes())
        interface.chmod(0o755)
        self.environment["PYTHONPATH"] = str(self.ci.parent)
        self.environment.pop("CHRONO_WORKTREE_BOOTSTRAP", None)
        declarations = self.declarations("copy-executable", str(interface), built)
        declarations["projects"][0]["actions"]["acquire"] = {
            "operation": "acquire.inputs", "tool": "inputs",
            "argv": ["acquisition", original, installed] if mode == "acquisition" else ["fail", "7"],
        }
        self.write_json(self.registry, declarations)
        config = self.binding_config()
        config["tools"]["inputs"] = built
        config["operations"] = ["make.tool", "acquire.inputs"]
        config["install"] = [{"source": built, "destination": installed}]
        config["toolchain_provision"] = {
            "profile": "default", "rustup_home": ".chrono-harness/state/native/fixture/rustup",
            "cargo_home": ".chrono-harness/state/native/fixture/cargo",
        }
        config["participation"] = {
            "coordinator_root": "git-main-worktree", "program": installed, "git": "git",
            "config": ".chrono-harness/native/fixture/worktree.json",
        }
        self.write_json(self.ci / "bootstrap.json", config)
        self.git("init", "-q")
        (self.root / ".gitignore").write_text(
            "/crates/inputs/target/\n/.chrono-harness/bin/\n/.chrono-harness/state/\n/mock-tools/\n")
        self.git("config", "user.email", "fixture@example.invalid")
        self.git("config", "user.name", "Fixture")
        self.git("add", ".")
        self.git("commit", "-qm", "first native acquisition fixture")
        return config, built, installed, original

    def test_first_main_seed_acquires_with_built_tool_before_install_and_without_full_policy(self):
        config, built, installed, original = self.acquisition_config()
        self.assertFalse((self.root / config["participation"]["config"]).exists())
        self.succeeded(self.invoke())  # invoke deliberately runs outside HOST_ROOT
        report = self.evidence()
        self.assertEqual(report["source"]["state"], "clean")
        self.assertEqual([r["operation"] for r in report["operations"]],
                         ["make.tool", "acquire.inputs"])
        acquisition = report["operations"][1]
        self.assertEqual(acquisition["argv"][0], str(self.root / built))
        self.assertEqual(acquisition["cwd"], str(self.root))
        raw = (self.root / original).read_bytes()
        observed = json.loads(raw)
        self.assertFalse(observed["installed_before"], "acquisition required its future owner")
        self.assertEqual(observed["environment"], {
            "RUSTUP_HOME": str(self.root / config["toolchain_provision"]["rustup_home"]),
            "CARGO_HOME": str(self.root / config["toolchain_provision"]["cargo_home"]),
            "RUSTC": str(self.tools / "rustc"),
        })
        stdout = (self.root / acquisition["stdout"]["path"]).read_bytes()
        self.assertEqual(json.loads(stdout)["fixture_original"], {
            "path": original, "sha256": hashlib.sha256(raw).hexdigest()})
        self.assertEqual(acquisition["stdout"]["sha256"], hashlib.sha256(stdout).hexdigest())
        self.assertEqual((self.root / installed).read_bytes(), (self.root / built).read_bytes())
        self.assertEqual(acquisition["executable_sha256"], report["installed"][0]["sha256"])
        self.assertFalse((self.root / config["participation"]["config"]).exists())

        # Shared consumers validate a fixed producer location, never their own cwd.
        import bootstrap_shared
        bootstrap_shared.validate_execution(self.root, config, report)
        changed = json.loads(json.dumps(report))
        changed["operations"][1]["argv"][0] = str(self.root / installed)
        with self.assertRaisesRegex(ValueError, "operation execution differs"):
            bootstrap_shared.validate_execution(self.root, config, changed)

    def test_first_seed_acquisition_failure_retains_originals_and_stops_installation(self):
        config, built, installed, original = self.acquisition_config("fail")
        result = self.invoke()
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse((self.root / installed).exists())
        self.assertTrue((self.root / built).is_file())
        self.assertFalse((self.root / original).exists())
        reports = list(self.state_path.parent.glob("bootstrap-*/report.json"))
        self.assertEqual(len(reports), 1)
        report = json.loads(reports[0].read_bytes())
        self.assertEqual(report["status"], "failed")
        self.assertEqual(report["installed"], [])
        acquisition = report["operations"][1]
        self.assertEqual(acquisition["exit_code"], 7)
        for channel, expected in [("stdout", b"failure stdout\x00\xff\n"),
                                  ("stderr", b"failure stderr\xfe\n")]:
            self.assertEqual((self.root / acquisition[channel]["path"]).read_bytes(), expected)
            self.assertEqual(acquisition[channel]["sha256"], hashlib.sha256(expected).hexdigest())

    def test_existing_owner_refusal_does_not_fall_back_to_main_seed(self):
        config, built, installed, original = self.acquisition_config("fail")
        owner = self.root / installed
        owner.parent.mkdir(parents=True)
        shutil.copy2(self.ci / "fixtures/acquirer", owner)
        result = self.invoke()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"bootstrap ownership refused", result.stderr)
        self.assertFalse((self.root / built).exists())
        self.assertFalse((self.root / original).exists())
        self.assertFalse((self.tools / "calls.json").exists())

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
    elif sys.argv[1:] == ["--real-fixture"]:
        from bootstrap_shared_tests import RealFixtureStartup
        unittest.main(argv=[sys.argv[0]], defaultTest="RealFixtureStartup")
    else:
        from bootstrap_shared_tests import SharedStartup, AttemptCustody, ManagedAttemptCustody, ParentSourceAcquisition, RunnerConsumption, FixtureHostPolicy
        unittest.main()
