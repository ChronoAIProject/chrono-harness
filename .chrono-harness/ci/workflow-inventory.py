#!/usr/bin/env python3
"""Host-owned coverage guard: observe registered Cargo/libtest lists, never select dependencies."""
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from process_fds import inherited_fds

PASS_FDS = inherited_fds()
import time


def parse_listing(stdout, stderr, binaries):
    observed = re.findall(r"Running [^\n]*\(([^\n]+)\)", stderr)
    observed = [re.sub(r"-[0-9a-f]+$", "", Path(path).name) for path in observed]
    if observed != binaries:
        raise ValueError("listing binaries differ: " + repr(observed))
    suites = []
    names = []
    for line in stdout.splitlines():
        if line.endswith(": test"):
            names.append(line[:-6])
        elif re.fullmatch(r"\d+ tests?, \d+ benchmarks?", line):
            count, benches = map(int, re.findall(r"\d+", line))
            if benches or count != len(names) or len(set(names)) != len(names):
                raise ValueError("malformed/duplicate libtest listing")
            suites.append(names)
            names = []
        elif line.strip():
            raise ValueError("unexpected libtest listing: " + line)
    if names or len(suites) != len(binaries):
        raise ValueError("incomplete libtest listing")
    return {binary + "::" + name for binary, suite in zip(binaries, suites) for name in suite}


def validate_sets(complete, groups, ignored):
    if ignored:
        raise ValueError("ignored tests cannot establish complete coverage")
    if not complete or any(not group for group in groups.values()):
        raise ValueError("empty inventory/group")
    union = set()
    for identity, group in groups.items():
        if union & group:
            raise ValueError("overlapping inventory group: " + identity)
        union |= group
    if union != complete:
        raise ValueError("inventory omission/addition: " + repr(sorted(union ^ complete)))


def validate(root, config_path):
    config_bytes = (root / config_path).read_bytes()
    config = json.loads(config_bytes)
    if config["schema"] != "chrono-workflow-inventory/v1":
        raise ValueError("unsupported inventory schema")
    projects_bytes = (root / config["projects"]).read_bytes()
    projects = json.loads(projects_bytes)
    rows = [row for row in projects["projects"] if row["id"] == config["project"]]
    if len(rows) != 1 or rows[0].get("test_groups") != config["groups"]:
        raise ValueError("inventory group binding drift")
    row = rows[0]
    state = root / config["report_directory"]
    state.mkdir(parents=True, exist_ok=True)
    receipts = []

    def listing(action_key, ignored=False):
        action = row["actions"][action_key]
        argv = [config["tools"][action["tool"]], *action["argv"]]
        if "--" not in argv:
            argv.append("--")
        argv += (["--ignored"] if ignored else []) + ["--list"]
        label = action_key + ("-ignored" if ignored else "")
        stdout_path, stderr_path = state / (label + ".stdout"), state / (label + ".stderr")
        start = time.monotonic()
        with stdout_path.open("wb") as stdout, stderr_path.open("wb") as stderr:
            try:
                process = subprocess.run(argv, cwd=root, stdout=stdout, stderr=stderr,
                                         pass_fds=PASS_FDS, timeout=config["timeout_seconds"])
                exit_code, error = process.returncode, None
            except subprocess.TimeoutExpired:
                exit_code, error = 124, "listing timeout"
        output_size, error_size = stdout_path.stat().st_size, stderr_path.stat().st_size
        if max(output_size, error_size) > config["output_limit_bytes"]:
            receipts.append({"action": action_key, "argv": argv, "exit_code": exit_code,
                             "error": "listing output bound exceeded", "stdout": str(stdout_path.relative_to(root)),
                             "stderr": str(stderr_path.relative_to(root))})
            (state / "receipts.json").write_text(json.dumps(receipts, indent=2) + "\n")
            raise ValueError("listing output bound exceeded")
        output, errors = stdout_path.read_bytes(), stderr_path.read_bytes()
        receipt = {"action": action_key, "operation": action["operation"], "argv": argv,
                   "exit_code": exit_code, "error": error, "duration_seconds": time.monotonic() - start,
                   "stdout": str(stdout_path.relative_to(root)), "stderr": str(stderr_path.relative_to(root)),
                   "stdout_sha256": hashlib.sha256(output).hexdigest(),
                   "stderr_sha256": hashlib.sha256(errors).hexdigest()}
        receipts.append(receipt)
        (state / "receipts.json").write_text(json.dumps(receipts, indent=2) + "\n")
        if exit_code:
            raise ValueError("failed listing: " + label + " exit " + str(exit_code))
        return parse_listing(output.decode(), errors.decode(), config["binaries"][action_key])

    complete = listing(config["unfiltered_action"])
    ignored = listing(config["unfiltered_action"], ignored=True)
    groups = {identity: listing(action) for identity, action in config["groups"].items()}
    validate_sets(complete, groups, ignored)
    report = {"schema": config["schema"], "project": config["project"],
              "config_sha256": hashlib.sha256(config_bytes).hexdigest(),
              "projects_sha256": hashlib.sha256(projects_bytes).hexdigest(),
              "complete": sorted(complete), "groups": {key: sorted(value) for key, value in groups.items()},
              "receipts": receipts}
    (state / "inventory.json").write_text(json.dumps(report, indent=2) + "\n")
    return report


if __name__ == "__main__":
    try:
        report = validate(Path(sys.argv[1]).resolve(), Path(sys.argv[2]))
        print(json.dumps({"complete": len(report["complete"]),
                          "groups": {key: len(value) for key, value in report["groups"].items()}}))
    except (ValueError, KeyError, OSError, UnicodeError) as error:
        print("E_WORKFLOW_INVENTORY:", error, file=sys.stderr)
        sys.exit(1)
