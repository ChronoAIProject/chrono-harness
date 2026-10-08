#!/usr/bin/env python3
"""Build only explicitly registered bootstrap operations, then install declared tools."""
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from process_fds import inherited_fds

PASS_FDS = inherited_fds()


def host_path(root, declared, prefix=".chrono-harness/"):
    path = Path(declared)
    if (not isinstance(declared, str) or path.is_absolute() or ".." in path.parts
            or path.as_posix() != declared or not declared.startswith(prefix)):
        raise ValueError("invalid bootstrap host path")
    target = root
    for part in path.parts:
        target /= part
        if target.is_symlink():
            raise ValueError("bootstrap host path contains a symlink")
    return target


def distribution_spec(root, config):
    """Read the adopted installer contract; do not implement download/installation."""
    if "distribution" not in config:
        return None
    declaration = config["distribution"]
    if set(declaration) != {"manifest"}:
        raise ValueError("invalid bootstrap distribution declaration")
    raw = host_path(root, declaration["manifest"]).read_bytes()
    release = json.loads(raw)
    lock = json.loads(host_path(root, ".chrono-harness/distribution.json").read_bytes())
    system = {"Darwin": "macos", "Linux": "linux"}.get(platform.system())
    arch = {"arm64": "aarch64", "aarch64": "aarch64", "x86_64": "x86_64"}.get(platform.machine())
    key = str(system) + "-" + str(arch)
    assets = release["platforms"][key]
    if (lock["schema"] != "chrono-install/v1" or release["schema"] != "chrono-release/v1"
            or lock["version"] != release["version"] or not lock["install"]
            or lock["manifest"] != {"file": "release.json", "size": len(raw),
                                    "sha256": hashlib.sha256(raw).hexdigest()}
            or lock["installers"][key] != assets["chrono-distribution"]):
        raise ValueError("bootstrap distribution manifest/installer binding differs")
    installed, destinations = [], set()
    for name, destination in sorted(lock["install"].items()):
        host_path(root, destination, ".chrono-harness/bin/")
        if destination in destinations:
            raise ValueError("duplicate distribution destination")
        destinations.add(destination)
        asset = assets[name]
        if asset["file"] != name + "-" + key:
            raise ValueError("bootstrap distribution asset platform differs")
        installed.append({"name": name, "path": destination,
                          "sha256": asset["sha256"], "size": asset["size"]})
    if any(item["destination"] in destinations for item in config["install"]):
        raise ValueError("source install overlaps immutable enforcement executable")
    return {"schema": "chrono-install-result/v1", "version": release["version"],
            "source_commit": release["source_commit"], "source_tree": release["source_tree"],
            "platform": key, "manifest": lock["manifest"], "installed": installed}


def validate_distribution(root, config, raw, payloads=None):
    expected = distribution_spec(root, config)
    if expected is None or json.loads(raw) != expected:
        raise ValueError("bootstrap requires original pinned distribution receipt")
    for item in expected["installed"]:
        if payloads is None:
            target = host_path(root, item["path"], ".chrono-harness/bin/")
            data = target.read_bytes()
            if not target.stat().st_mode & 0o111:
                raise ValueError("distribution executable lacks execute permission")
        else:
            data = payloads[item["path"]]
        if len(data) != item["size"] or hashlib.sha256(data).hexdigest() != item["sha256"]:
            raise ValueError("distribution executable differs: " + item["path"])
    return expected


def distribution_binding(raw):
    return {"receipt_path": ".chrono-harness/state/distribution.json",
            "receipt_sha256": hashlib.sha256(raw).hexdigest()}


def installations(root, config):
    selected = list(config["install"])
    release = distribution_spec(root, config)
    if release is not None:
        selected += [{"destination": item["path"]} for item in release["installed"]]
    return selected


def report_destination(root, config):
    declared = config.get("report_path", ".chrono-harness/state/bootstrap.json")
    if not isinstance(declared, str) or any(c in declared for c in "\x00\r\n"):
        raise ValueError("bootstrap report must be an explicit host state file")
    path = Path(declared)
    if (path.is_absolute() or ".." in path.parts or len(path.parts) < 3
            or path.parts[:2] != (".chrono-harness", "state") or path.as_posix() != declared):
        raise ValueError("bootstrap report must be an explicit host state file")
    target = root
    for part in path.parts:
        target = target / part
        if target.is_symlink():
            raise ValueError("bootstrap report path must not contain a symlink")
    return target


def observe_source(root):
    """Observe this host's source repository without requiring one for development."""
    observed = {"commit": None, "tree": None, "dirty": None, "error": None}

    def git(*args):
        return subprocess.check_output(
            ["git", "--no-optional-locks", "-C", str(root), *args],
            stderr=subprocess.PIPE, pass_fds=PASS_FDS,
        ).decode("utf-8", "surrogateescape").strip()

    try:
        source_root, commit = git(
            "rev-parse", "--show-toplevel", "--verify", "HEAD",
        ).rsplit("\n", 1)
        if Path(source_root).resolve() != root:
            raise ValueError("host root is not the Git source root")
        observed["commit"] = commit
        observed["tree"] = git("rev-parse", "--verify", observed["commit"] + "^{tree}")
        observed["dirty"] = bool(git(
            "status", "--porcelain=v1", "--untracked-files=normal", "--ignore-submodules=none",
        ))
        if git("rev-parse", "--verify", "HEAD") != observed["commit"]:
            raise ValueError("source HEAD changed during observation")
    except subprocess.CalledProcessError as error:
        observed["error"] = error.stderr.decode("utf-8", "replace").strip() or str(error)
    except (OSError, ValueError) as error:
        observed["error"] = str(error)
    return observed


def participate(root, config_path, config):
    adoption = config.get("participation")
    if adoption is None:
        return False
    marker = os.environ.get("CHRONO_WORKTREE_BOOTSTRAP")
    if marker is not None:
        if Path(marker) != root:
            raise ValueError("bootstrap participation root mismatch")
        return False
    if set(adoption) != {"coordinator_root", "program", "git", "config"} or adoption["coordinator_root"] != "git-main-worktree":
        raise ValueError("unsupported bootstrap participation declaration")
    program = Path(adoption["program"])
    if program.is_absolute() or ".." in program.parts or program.parts[:2] != (".chrono-harness", "bin"):
        raise ValueError("bootstrap participant must be a registered host binary")
    inventory = subprocess.check_output([adoption["git"], "--no-optional-locks", "-C", str(root), "worktree", "list", "--porcelain", "-z"], pass_fds=PASS_FDS)
    first = inventory.split(b"\0", 1)[0]
    if not first.startswith(b"worktree "):
        raise ValueError("coordinator Git owner inventory is missing")
    anchor = Path(os.fsdecode(first[len(b"worktree "):]))
    owner = anchor / program
    if anchor == root and (root / ".git").is_dir() and not owner.exists():
        # Git main cannot enroll as a disposable linked attachment; it may seed tools.
        return False
    result = subprocess.run([str(owner), "bootstrap", "--host-root", str(root), "--config", adoption["config"], "--bootstrap-config", config_path.as_posix()], pass_fds=PASS_FDS, capture_output=True)
    try:
        report = json.loads(result.stdout)
        process = report["managed_process"]
    except (ValueError, KeyError):
        raise ValueError("bootstrap ownership refused: " + result.stdout.decode("utf-8", "replace") + result.stderr.decode("utf-8", "replace"))
    sys.stdout.buffer.write(bytes(process["stdout_bytes"]))
    sys.stderr.buffer.write(bytes(process["stderr_bytes"]))
    if report["status"] != "used" and report.get("managed_command_failed") is not True:
        raise ValueError("bootstrap lifecycle failed: " + str(report.get("error")) +
                         "; original report " + str(report.get("report_path")))
    if process["failure"] is not None:
        raise ValueError("bootstrap process failed: " + process["failure"] +
                         "; original report " + str(report.get("report_path")))
    sys.exit(process["exit_code"] if 0 <= process["exit_code"] <= 255 else 1)


def toolchain_environment(config):
    toolchain = config["rust_toolchain"]
    probe = subprocess.run(["rustup", "run", toolchain, "cargo", "--version"], pass_fds=PASS_FDS, capture_output=True)
    if probe.returncode:
        subprocess.run(["rustup", "toolchain", "install", toolchain, "--profile", "minimal", "--component", "rustfmt"], pass_fds=PASS_FDS, check=True)
    fmt_probe = subprocess.run(["rustup", "run", toolchain, "rustfmt", "--version"], pass_fds=PASS_FDS, capture_output=True)
    if fmt_probe.returncode:
        subprocess.run(["rustup", "component", "add", "--toolchain", toolchain, "rustfmt"], pass_fds=PASS_FDS, check=True)
    env = dict(os.environ, RUSTUP_TOOLCHAIN=toolchain, CARGO_TERM_COLOR="never")
    if "rust_incremental" in config:
        env["CARGO_INCREMENTAL"] = "1" if config["rust_incremental"] else "0"
    versions = {}
    for tool in ["cargo", "rustc"]:
        versions[tool] = subprocess.check_output([tool, "--version"], env=env, pass_fds=PASS_FDS, text=True).strip()
    if not versions["rustc"].startswith("rustc " + toolchain + " "):
        raise ValueError("unexpected rustc version")
    return env, versions


def main():
    if len(sys.argv) not in (2, 3):
        raise ValueError("usage: bootstrap.py HOST_ROOT [REGISTERED_CONFIG]")
    root = Path(sys.argv[1]).resolve()
    config_path = Path(sys.argv[2]) if len(sys.argv) == 3 else Path(".chrono-harness/ci/bootstrap.json")
    if config_path.is_absolute() or ".." in config_path.parts or config_path.parts[:2] != (".chrono-harness", "ci"):
        raise ValueError("bootstrap config must be an explicit host CI path")
    config = json.loads((root / config_path).read_text())
    report_destination(root, config)
    if "rust_incremental" in config and type(config["rust_incremental"]) is not bool:
        raise ValueError("rust_incremental must be a boolean")
    distribution_spec(root, config)
    participate(root, config_path, config)
    source_before = observe_source(root)
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
    env, versions = toolchain_environment(config)
    for operation in config["operations"]:
        action = operations[operation]
        subprocess.run([config["tools"][action["tool"]], *action["argv"]], cwd=root, env=env, pass_fds=PASS_FDS, check=True)
    installed = []
    for item in config["install"]:
        source, target = root / item["source"], root / item["destination"]
        if not source.is_file():
            raise ValueError("missing built tool: " + str(source))
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source, target)
        installed.append({"path": item["destination"], "sha256": hashlib.sha256(target.read_bytes()).hexdigest()})
    distribution = None
    if "distribution" in config:
        # The generated launcher and the existing Rust installer own actual installation.
        launcher = host_path(root, ".chrono-harness/install.py")
        subprocess.run([sys.executable, "-B", str(launcher), str(root)],
                       check=True, pass_fds=PASS_FDS)
        original = host_path(root, ".chrono-harness/state/distribution.json").read_bytes()
        validate_distribution(root, config, original)
        distribution = distribution_binding(original)
    source_after = observe_source(root)
    if source_before["error"] or source_after["error"]:
        source_state = "unavailable"
    elif source_before != source_after:
        source_state = "changed"
    elif source_before["dirty"]:
        source_state = "dirty"
    else:
        source_state = "clean"
    # Only clean, agreeing observations bind the installed result to a source commit.
    source_identity = {
        "state": source_state,
        "commit": source_before["commit"] if source_state == "clean" else None,
        "tree": source_before["tree"] if source_state == "clean" else None,
        "before": source_before,
        "after": source_after,
    }
    report = report_destination(root, config)
    report.parent.mkdir(parents=True, exist_ok=True)
    result = {"schema": "chrono-bootstrap-result/v1", "config": config_path.as_posix(), "source": source_identity, "versions": versions, "environment": {key: env.get(key) for key in ["RUSTUP_TOOLCHAIN", "CARGO_INCREMENTAL"]}, "installed": installed}
    if distribution is not None:
        result["distribution"] = distribution
    report.write_text(json.dumps(result, indent=2) + "\n")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, OSError, subprocess.CalledProcessError) as error:
        print("E_BOOTSTRAP:", error, file=sys.stderr)
        sys.exit(1)
