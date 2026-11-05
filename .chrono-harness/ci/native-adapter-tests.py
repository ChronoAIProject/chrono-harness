"""Dedicated Python tests of the shipped adapter and its genuine Rust bridge."""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import shutil
import sys
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
SOURCE = Path(__file__).resolve().parent
arguments = argparse.ArgumentParser()
arguments.add_argument("--interpreter", required=True)
configured, test_arguments = arguments.parse_known_args()
INTERPRETER = configured.interpreter
DIRECTORY = ".chrono-harness/state/collection/preparation/"
spec = importlib.util.spec_from_file_location("native_adapter", ROOT / "assets/ci/native.py")
native = importlib.util.module_from_spec(spec)
spec.loader.exec_module(native)


class NativeAdapter(unittest.TestCase):
    def test_utc_fraction_boundaries(self):
        for text, nanoseconds in [("0", 0), ("1", 100000000), ("12345", 123450000),
                                  ("123456", 123456000), ("123456789", 123456789)]:
            with self.subTest(fraction=text):
                self.assertEqual(native.moment("2026-01-01T00:00:00." + text + "Z"),
                                 1767225600000000000 + nanoseconds)
        with self.assertRaises(ValueError):
            native.moment("2026-01-01T00:00:00+08:00")

    def test_native_forwarding_resolves_declared_path_once_and_retains_real_bounded_outcomes(self):
        owner = ROOT / ".chrono-harness/state/native-adapter-tests"
        owner.mkdir(parents=True, exist_ok=True)
        root = Path(tempfile.mkdtemp(prefix="native adapter \u03bb ", dir=owner)).resolve()
        try:
            self.exercise_transport(root)
        except BaseException:
            print("Original failed native adapter fixture: " + str(root), file=sys.stderr)
            print("Retain until this test's failure diagnosis and acceptance are complete.",
                  file=sys.stderr)
            raise
        else:
            # Only this producer's fresh fixture is removed, after all real
            # bridge children joined and every assertion consumed its originals.
            shutil.rmtree(root)

    def exercise_transport(self, root):
        provider = json.loads((SOURCE / "fixtures/native-adapter-provider.json").read_bytes())
        provider["native_adoption"]["interpreter"] = INTERPRETER
        program = Path(INTERPRETER).name
        provider["native_adoption"]["inputs_program"] = program
        config = ".chrono-harness/ci/units.json"
        config_path = root / config
        config_path.parent.mkdir(parents=True)
        config_path.write_text(json.dumps(provider))
        (root / ".chrono-harness/config.json").write_text(json.dumps({
            "environment": {"credential_environment": ["SECRET"]}}))
        bridge = root / provider["collection"]["generator"]
        bridge.parent.mkdir(parents=True)
        # This unmodified executable is the existing registered build.ci output.
        # A symlink avoids retaining another copy of the candidate tool.
        bridge.symlink_to(ROOT / "crates/ci/target/debug/chrono-ci")
        native.NATIVE_PROVIDER = provider
        native.NATIVE_CONFIG = config
        # Use the declared invocation, even if a launcher reports another sys.executable.
        env = {"PATH": str(Path(INTERPRETER).parent), "BUSINESS_VALUE": "actual value",
               "SECRET": "actual credential"}
        fixture = str(SOURCE / "fixtures/native-adapter-child.py")
        argv = [program, "-B", fixture, "echo"]
        raw, refs = native.process(root, argv, b"original input", env, DIRECTORY,
                                   "good", 5, 1024, ["SECRET"])
        self.assertEqual(raw, b"\x00\xfforiginal input")
        receipt = json.loads((root / refs[-1]["path"]).read_bytes())
        self.assertEqual(receipt["argv"][0], str(Path(env["PATH"]) / program))
        self.assertEqual(receipt["configured_argv"][0], program)
        self.assertEqual(receipt["executable"]["sha256"],
                         hashlib.sha256(Path(receipt["argv"][0]).read_bytes()).hexdigest())
        self.assertEqual(receipt["environment"]["BUSINESS_VALUE"],
                         hashlib.sha256(b"actual value").hexdigest())
        self.assertNotIn("SECRET", receipt["environment"])
        self.assertEqual(receipt["omitted_credentials"], ["SECRET"])
        self.assertEqual((root / receipt["stdin"]["path"]).read_bytes(), b"original input")
        self.assertEqual((root / receipt["stdout"]["path"]).read_bytes(), raw)
        self.assertEqual((root / receipt["stderr"]["path"]).read_bytes(), b"original stderr")
        with self.assertRaises(ValueError) as undeclared:
            native.process(root, [receipt["argv"][0], "-B", fixture, "echo"], b"", env,
                           DIRECTORY, "undeclared", 5, 1024, ["SECRET"])
        self.assertEqual(str(undeclared.exception),
                         "E_CI: native-process differs from registered acquisition operations/bounds")
        for prefix, timeout, expected in [("exit", 5, None),
                                           ("overflow", 5, "output bound exceeded"),
                                           ("timeout", 0.1, "timeout")]:
            with self.assertRaises(ValueError) as failed:
                native.process(root, [program, "-B", fixture, prefix], b"", env,
                               DIRECTORY, prefix, timeout, 32, ["SECRET"])
            original = str(failed.exception).split("retained at ")[1]
            receipt = json.loads((root / original).read_bytes())
            self.assertEqual(receipt["failure"], expected)
            self.assertLessEqual(len((root / receipt["stdout"]["path"]).read_bytes()), 32)
            if prefix == "exit":
                self.assertEqual(receipt["exit_code"], 7)
                self.assertEqual((root / receipt["stderr"]["path"]).read_bytes(), b"real failure")


if __name__ == "__main__":
    unittest.main(argv=[sys.argv[0], *test_arguments], verbosity=2)
