#!/usr/bin/env python3
"""Build only explicitly registered bootstrap operations, then install declared tools."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys


def main():
    if len(sys.argv) != 2:
        raise ValueError("usage: bootstrap.py HOST_ROOT")
    root = Path(sys.argv[1]).resolve()
    config = json.loads((root / ".chrono-harness/ci/bootstrap.json").read_text())
    if config["schema"] != "chrono-bootstrap/v1":
        raise ValueError("unsupported bootstrap schema")
    projects = json.loads((root / config["projects"]).read_text())
    operations = {}
    for project in projects["projects"] + projects["scripts"]:
        for action in project["actions"].values():
            key = action["operation"]
            if key in operations:
                raise ValueError("duplicate operation: " + key)
            operations[key] = action
    toolchain = config["rust_toolchain"]
    probe = subprocess.run(["rustup", "run", toolchain, "cargo", "--version"], capture_output=True)
    if probe.returncode:
        subprocess.run(["rustup", "toolchain", "install", toolchain, "--profile", "minimal", "--component", "rustfmt"], check=True)
    env = dict(os.environ, RUSTUP_TOOLCHAIN=toolchain, CARGO_TERM_COLOR="never")
    versions = {}
    for tool in ["cargo", "rustc"]:
        versions[tool] = subprocess.check_output([tool, "--version"], env=env, text=True).strip()
    if not versions["rustc"].startswith("rustc " + toolchain + " "):
        raise ValueError("unexpected rustc version")
    for operation in config["operations"]:
        action = operations[operation]
        subprocess.run([config["tools"][action["tool"]], *action["argv"]], cwd=root, env=env, check=True)
    installed = []
    for item in config["install"]:
        source, target = root / item["source"], root / item["destination"]
        if not source.is_file():
            raise ValueError("missing built tool: " + str(source))
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source, target)
        installed.append({"path": item["destination"], "sha256": hashlib.sha256(target.read_bytes()).hexdigest()})
    state = root / ".chrono-harness/state"
    state.mkdir(parents=True, exist_ok=True)
    (state / "bootstrap.json").write_text(json.dumps({"schema": "chrono-bootstrap-result/v1", "versions": versions, "installed": installed}, indent=2) + "\n")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, OSError, subprocess.CalledProcessError) as error:
        print("E_BOOTSTRAP:", error, file=sys.stderr)
        sys.exit(1)
