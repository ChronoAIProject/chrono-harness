#!/usr/bin/env python3
"""Exercise coverage failures through actual listing subprocesses and original receipts."""
import importlib.util
import json
from pathlib import Path
import sys
import tempfile
import unittest

sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location("inventory", Path(__file__).with_name("workflow-inventory.py"))
inventory = importlib.util.module_from_spec(spec)
spec.loader.exec_module(inventory)


class Coverage(unittest.TestCase):
    def fixture(self, root, mode):
        script = root / "list.py"
        script.write_text("""import sys
mode, action = sys.argv[1:3]
if mode == 'failed' and action == 'right':
 print('original failure', file=sys.stderr); sys.exit(7)
names = {'all':['a','b'], 'left':['a'], 'right':['b']}[action]
if '--ignored' in sys.argv: names = ['a'] if mode == 'ignored' else []
elif mode == 'empty' and action == 'right': names = []
elif mode == 'overlap' and action == 'right': names = ['a','b']
elif mode == 'omit' and action == 'all': names = ['a','b','c']
print('Running tests/cases.rs (target/cases-123abc)', file=sys.stderr)
for name in names: print(name+': test')
print(str(len(names))+' tests, 0 benchmarks')
""")
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
