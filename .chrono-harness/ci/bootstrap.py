#!/usr/bin/env python3
"""Build only explicitly registered bootstrap operations, then install declared tools."""
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from process_fds import inherited_fds

PASS_FDS = inherited_fds()


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


def file_digest(path):
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def executable_identity(path, tool, env):
    path = Path(path)
    if not path.is_file() or path.is_symlink():
        raise ValueError("bound executable is not a regular file: " + str(path))
    probe = subprocess.run([str(path), "--version"], env=env, pass_fds=PASS_FDS,
                           capture_output=True, text=True)
    if probe.returncode:
        raise ValueError("bound " + tool + " version failed: " + str(probe.returncode))
    version = probe.stdout.strip()
    if not version.startswith(tool + " "):
        raise ValueError("bound " + tool + " version mismatch: " + version)
    return {"path": str(path), "sha256": file_digest(path), "version": version}


def declared_bindings(root, config, env):
    """Resolve and validate the explicit Cargo/rustc selection contract.

    The declaration is intentionally separate from observations: its digest and
    version are checked against the selected files before any registered action
    runs.  `allow_relocation` permits an equivalent rustup installation to move
    while retaining the observed local path in the report.
    """
    declared = config.get("tool_bindings")
    if declared is None:
        return None
    if set(declared) != {"cargo", "rustc"}:
        raise ValueError("tool_bindings must declare cargo and rustc")
    toolchain = config["rust_toolchain"]
    selected = {}
    for tool in ("cargo", "rustc"):
        item = declared[tool]
        if (not isinstance(item, dict)
                or set(item) - {"path", "sha256", "version", "allow_relocation"}
                or not isinstance(item.get("path"), str)
                or not isinstance(item.get("sha256"), str)
                or not re.fullmatch(r"[0-9a-f]{64}", item["sha256"])
                or not isinstance(item.get("version"), str)
                or type(item.get("allow_relocation", False)) is not bool):
            raise ValueError("invalid " + tool + " executable binding")
        try:
            selected_path = Path(subprocess.check_output(
                ["rustup", "which", "--toolchain", toolchain, tool],
                cwd=root, env=env, pass_fds=PASS_FDS, stderr=subprocess.PIPE,
            ).decode("utf-8").strip())
        except (OSError, subprocess.CalledProcessError, UnicodeDecodeError) as error:
            raise ValueError("cannot resolve bound " + tool + ": " + str(error))
        if not selected_path.is_absolute():
            raise ValueError("rustup selected non-absolute " + tool + " path")
        if not selected_path.is_file() or selected_path.is_symlink():
            raise ValueError("bound executable is not a regular file: " + str(selected_path))
        if file_digest(selected_path) != item["sha256"]:
            raise ValueError("bound " + tool + " identity mismatch")
        observed = executable_identity(selected_path, tool, env)
        if observed["sha256"] != item["sha256"] or observed["version"] != item["version"]:
            raise ValueError("bound " + tool + " identity mismatch")
        expected_path = Path(item["path"])
        if not expected_path.is_absolute():
            expected_path = root / expected_path
        if expected_path.exists():
            expected = executable_identity(expected_path, tool, env)
            if expected["sha256"] != observed["sha256"] or expected["version"] != observed["version"]:
                raise ValueError("declared " + tool + " path differs from selected executable")
        elif not item.get("allow_relocation", False):
            raise ValueError("declared " + tool + " executable is missing")
        configured = config.get("tools", {}).get(tool)
        if configured is None:
            raise ValueError("tool_bindings requires configured " + tool)
        configured_path = shutil.which(configured) if not Path(configured).is_absolute() else configured
        if configured_path is None:
            if not item.get("allow_relocation", False):
                raise ValueError("configured " + tool + " executable is missing")
        else:
            configured_identity = executable_identity(configured_path, tool, env)
            if (configured_identity["sha256"] != observed["sha256"]
                    or configured_identity["version"] != observed["version"]):
                raise ValueError("configured " + tool + " selection differs from binding")
        selected[tool] = observed
    if selected["cargo"]["version"].split(" ", 2)[1] != toolchain:
        raise ValueError("Cargo toolchain selection mismatch")
    if not selected["rustc"]["version"].startswith("rustc " + toolchain + " "):
        raise ValueError("rustc toolchain selection mismatch")
    inherited_rustc = env.get("RUSTC")
    if inherited_rustc and Path(inherited_rustc).resolve() != Path(selected["rustc"]["path"]).resolve():
        raise ValueError("inherited compiler selection differs from binding")
    for wrapper in ("RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER"):
        if env.get(wrapper):
            raise ValueError("inherited compiler wrapper differs from binding: " + wrapper)
    return selected


def toolchain_environment(config, root=None):
    toolchain = config["rust_toolchain"]
    probe = subprocess.run(["rustup", "run", toolchain, "cargo", "--version"], cwd=root,
                           pass_fds=PASS_FDS, capture_output=True)
    if probe.returncode:
        subprocess.run(["rustup", "toolchain", "install", toolchain, "--profile", "minimal", "--component", "rustfmt"], cwd=root, pass_fds=PASS_FDS, check=True)
    fmt_probe = subprocess.run(["rustup", "run", toolchain, "rustfmt", "--version"], cwd=root,
                               pass_fds=PASS_FDS, capture_output=True)
    if fmt_probe.returncode:
        subprocess.run(["rustup", "component", "add", "--toolchain", toolchain, "rustfmt"], cwd=root, pass_fds=PASS_FDS, check=True)
    env = dict(os.environ, RUSTUP_TOOLCHAIN=toolchain, CARGO_TERM_COLOR="never")
    if "rust_incremental" in config:
        env["CARGO_INCREMENTAL"] = "1" if config["rust_incremental"] else "0"
    bindings = declared_bindings(root or Path.cwd(), config, env)
    versions = {}
    for tool in ["cargo", "rustc"]:
        executable = bindings[tool]["path"] if bindings else tool
        versions[tool] = subprocess.check_output([executable, "--version"], cwd=root, env=env, pass_fds=PASS_FDS, text=True).strip()
    if not versions["rustc"].startswith("rustc " + toolchain + " "):
        raise ValueError("unexpected rustc version")
    if bindings:
        env["RUSTC"] = bindings["rustc"]["path"]
        env["RUSTC_WRAPPER"] = ""
        env["RUSTC_WORKSPACE_WRAPPER"] = ""
    return env, versions, bindings


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
    if type(config.get("require_tool_bindings", False)) is not bool:
        raise ValueError("require_tool_bindings must be a boolean")
    if config.get("require_tool_bindings") and "tool_bindings" not in config:
        raise ValueError("bootstrap requires declared Cargo/rustc bindings")
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
    env, versions, bindings = toolchain_environment(config, root)
    for operation in config["operations"]:
        action = operations[operation]
        executable = config["tools"][action["tool"]]
        if bindings and action["tool"] in bindings:
            executable = bindings[action["tool"]]["path"]
        subprocess.run([executable, *action["argv"]], cwd=root, env=env, pass_fds=PASS_FDS, check=True)
    if bindings:
        # A successful child must not have persistently replaced the selected
        # compiler or Cargo executable before installation/report publication.
        current = declared_bindings(root, config, env)
        if {tool: current[tool]["sha256"] for tool in current} != {
                tool: bindings[tool]["sha256"] for tool in bindings}:
            raise ValueError("bound executable changed during bootstrap")
    installed = []
    for item in config["install"]:
        source, target = root / item["source"], root / item["destination"]
        if not source.is_file():
            raise ValueError("missing built tool: " + str(source))
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source, target)
        installed.append({"path": item["destination"], "sha256": hashlib.sha256(target.read_bytes()).hexdigest()})
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
    result = {"schema": "chrono-bootstrap-result/v1", "config": config_path.as_posix(),
              "source": source_identity, "versions": versions,
              "environment": {key: env.get(key) for key in ["RUSTUP_TOOLCHAIN", "CARGO_INCREMENTAL"]},
              "installed": installed}
    if bindings:
        result["executables"] = [bindings[tool] for tool in ("cargo", "rustc")]
        result["selection"] = {"toolchain": config["rust_toolchain"],
                                "cargo": bindings["cargo"]["path"],
                                "rustc": bindings["rustc"]["path"],
                                "compiler": env["RUSTC"]}
    report.write_text(json.dumps(result, indent=2) + "\n")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, OSError, subprocess.CalledProcessError) as error:
        print("E_BOOTSTRAP:", error, file=sys.stderr)
        sys.exit(1)
