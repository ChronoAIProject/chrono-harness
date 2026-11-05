#!/usr/bin/env python3
"""Exercise coverage failures through actual listing subprocesses and original receipts."""
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location("inventory", Path(__file__).with_name("workflow-inventory.py"))
inventory = importlib.util.module_from_spec(spec)
spec.loader.exec_module(inventory)


class Coverage(unittest.TestCase):
    def test_product_and_test_imports_preserve_sources_without_bytecode(self):
        source = Path(__file__).resolve().parent
        sources = [".chrono-harness/process_fds.py", ".chrono-harness/ci/workflow-inventory.py",
                   ".chrono-harness/ci/workflow-inventory-tests.py"]
        for script in sources[1:]:
            with self.subTest(script=script):
                root = Path(tempfile.mkdtemp(prefix="inventory import "))
                try:
                    for path in sources:
                        destination = root / path
                        destination.parent.mkdir(parents=True, exist_ok=True)
                        shutil.copyfile(source.parents[1] / path, destination)
                    environment = dict(os.environ)
                    for key in ("PYTHONDONTWRITEBYTECODE", "PYTHONPYCACHEPREFIX", "CHRONO_PROCESS_FDS"):
                        environment.pop(key, None)
                    loaded = subprocess.run(
                        [sys.executable, str(source / "fixtures/source-import.py"), script],
                        cwd=root, env=environment, capture_output=True, pass_fds=inventory.PASS_FDS)
                    (root / "stdout.bin").write_bytes(loaded.stdout)
                    (root / "stderr.bin").write_bytes(loaded.stderr)
                    self.assertEqual(loaded.returncode, 0, loaded.stderr)
                    for parent in (".chrono-harness", ".chrono-harness/ci"):
                        self.assertFalse((root / parent / "__pycache__").exists(), parent)
                    for path in sources:
                        self.assertEqual((root / path).read_bytes(), (source.parents[1] / path).read_bytes())
                except BaseException:
                    print("retained inventory import fixture: " + str(root), file=sys.stderr)
                    raise
                else:
                    shutil.rmtree(root)

    def fixture(self, root, mode):
        script = root / "list.py"
        script.write_bytes((Path(__file__).resolve().parent / "fixtures/inventory-listing.py").read_bytes())
        groups = {"t": "left", "opaque": "right"}
        actions = {key: {"operation": key, "tool": "python", "argv": [str(script), mode, key]}
                   for key in ["all", "left", "right"]}
        (root / "projects.json").write_text(json.dumps({"projects": [{"id": "t", "test_groups": groups, "actions": actions}]}))
        config = {"schema": "chrono-workflow-inventory/v1", "projects": "projects.json", "project": "t",
                  "groups": groups, "unfiltered_action": "all", "tools": {"python": sys.executable},
                  "binaries": {key: ["cases"] for key in actions}, "report_directory": "reports",
                  "timeout_seconds": 5, "output_limit_bytes": 4096}
        if mode == "drift": config["groups"] = {"t": "right", "opaque": "left"}
        (root / "config.json").write_text(json.dumps(config))

    def test_real_listings_and_coverage_failures(self):
        for mode, expected in [("ok", None), ("failed", "failed listing"), ("empty", "empty"),
                               ("overlap", "overlapping"), ("omit", "omission"),
                               ("ignored", "ignored"), ("drift", "binding drift")]:
            with self.subTest(mode=mode), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                self.fixture(root, mode)
                if expected:
                    with self.assertRaisesRegex(ValueError, expected):
                        inventory.validate(root, Path("config.json"))
                else:
                    report = inventory.validate(root, Path("config.json"))
                    self.assertEqual(report["complete"], ["cases::a", "cases::b"])
                if mode == "failed":
                    receipts = json.loads((root / "reports/receipts.json").read_text())
                    self.assertEqual(receipts[-1]["exit_code"], 7)
                    self.assertEqual((root / receipts[-1]["stderr"]).read_text(), "original failure\n")

    def test_duplicate_or_truncated_listing_fails(self):
        for output in ["a: test\na: test\n2 tests, 0 benchmarks\n", "a: test\n", "a: test\n0 tests, 0 benchmarks\n"]:
            with self.assertRaises(ValueError):
                inventory.parse_listing(output, "Running tests/cases.rs (target/cases-123abc)\n", ["cases"])


if __name__ == "__main__":
    unittest.main()
