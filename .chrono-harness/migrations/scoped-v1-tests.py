#!/usr/bin/env python3
"""Dedicated decoder tests using fixed actual repository registrations as historical input."""
import copy
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import unittest

ROOT = Path(__file__).resolve().parents[2]
SOURCE = Path(__file__).with_name("scoped-v1.py")
SPEC = importlib.util.spec_from_file_location("decoder", SOURCE)
DECODER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(DECODER)
BASE = "4f08aef7ab40d7b0a3fb6ba42af2620600f18d98"


def blob(path):
    return subprocess.check_output(["git", "-C", str(ROOT), "show", BASE + ":" + path])


def request():
    config_path = ".chrono-harness/config.json"
    config = json.loads(blob(config_path))
    paths = [config_path, *config["registries"].values()]
    raw = {p: list(blob(p)) for p in paths}
    profile = json.loads((ROOT / ".chrono-harness/workflow.json").read_text())["historical_profiles"][0]
    profile_bytes = blob(profile["profile_path"])
    return {"schema": "chrono-historical-decode/v1", "config_path": config_path,
            "original": {p: json.loads(bytes(b)) for p, b in raw.items()},
            "original_bytes": raw, "profile": profile,
            "profile_bytes": list(profile_bytes), "profile_value": json.loads(profile_bytes)}


class DecoderTests(unittest.TestCase):
    def test_adopted_host_interpreter_matches_registered_version(self):
        # Host policy validation belongs to the host's registered script suite;
        # portable product fixtures independently adopt their executing platform.
        cfg = json.loads((ROOT / ".chrono-harness/config.json").read_text())
        tool = next(t for t in cfg["tools"] if t["id"] == "python3")
        self.assertTrue(Path(tool["program"]).is_absolute())
        observed = subprocess.run([tool["program"], *tool["version_argv"]],
                                  env={}, capture_output=True)
        self.assertEqual(observed.returncode, 0, observed.stderr)
        self.assertEqual(observed.stdout.decode().strip(), tool["expected_version"])

    def test_real_old_records_edges_costs_and_obligations_survive(self):
        req = request()
        out = DECODER.convert(req)
        old_fm = req["original"][".chrono-harness/FILEMAP.json"]
        new_fm = out["values"][".chrono-harness/FILEMAP.json"]
        for key in ["files", "project_edges", "test_costs", "cost_models"]:
            self.assertEqual(old_fm[key], new_fm[key])
        for test, ops in req["profile_value"]["policy"]["bindings"].items():
            self.assertEqual(ops, new_fm["execution_plans"][test]["operations"])
        self.assertEqual(out["historical"]["test:ci-verify"][0][1],
                         req["profile"]["legacy_records"][0]["definition"])

    def test_wrong_profile_version_bytes_and_legacy_record_rejected(self):
        for edit in [lambda r: r["profile_value"].update(schema="unknown"),
                     lambda r: r["original"][".chrono-harness/FILEMAP.json"].update(schema_version=2),
                     lambda r: r["profile"].get("legacy_records")[0].update(id="missing"),
                     lambda r: r.update(profile_bytes=list(b"{}"))]:
            req = request()
            edit(req)
            with self.assertRaises(ValueError):
                DECODER.convert(req)

    def test_actual_process_preserves_input_and_failure_exit(self):
        req = request()
        run = subprocess.run([sys.executable, str(SOURCE)], input=json.dumps(req).encode(), capture_output=True, cwd="/")
        self.assertEqual(run.returncode, 0, run.stderr)
        self.assertEqual(json.loads(run.stdout), DECODER.convert(req))
        req["profile_value"]["schema"] = "wrong"
        bad = subprocess.run([sys.executable, str(SOURCE)], input=json.dumps(req).encode(), capture_output=True)
        self.assertNotEqual(bad.returncode, 0)
        self.assertIn(b"unsupported version/profile", bad.stderr)


if __name__ == "__main__":
    unittest.main(verbosity=2)
