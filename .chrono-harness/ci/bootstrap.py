#!/usr/bin/env python3
"""Build only explicitly registered bootstrap operations, then install declared tools."""
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import shutil
import subprocess
import sys
import tempfile
import threading
import time

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
        if ("owner_upgrade" in config or "owner_stage" in config) and not PASS_FDS:
            raise ValueError("bootstrap participation requires original ownership descriptors")
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
    if anchor == root and "owner_upgrade" in config:
        import bootstrap_owner
        bootstrap_owner.upgrade(root, config_path, config)
    result = subprocess.run([str(owner), "bootstrap", "--host-root", str(root), "--config", adoption["config"], "--bootstrap-config", config_path.as_posix()], pass_fds=PASS_FDS, capture_output=True)
    try:
        report = json.loads(result.stdout)
        process = report["managed_process"]
    except (ValueError, KeyError):
        raise ValueError("bootstrap ownership refused: " + result.stdout.decode("utf-8", "replace") + result.stderr.decode("utf-8", "replace"))
    sys.stdout.buffer.write(bytes(process["stdout_bytes"]))
    sys.stderr.buffer.write(bytes(process["stderr_bytes"]))
    print("bootstrap ownership original report: " + str(anchor / report["report_path"]),
          file=sys.stderr)
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


def binding_registry_path(root, config):
    declared = config["tool_bindings_registry"]
    if not isinstance(declared, str):
        raise ValueError("invalid bootstrap toolchain registry path")
    path = Path(declared)
    if (path.is_absolute() or ".." in path.parts or path.as_posix() != declared
            or path.parts[:2] != (".chrono-harness", "ci")):
        raise ValueError("invalid bootstrap toolchain registry path")
    target = root
    for part in path.parts:
        target /= part
        if target.is_symlink():
            raise ValueError("bootstrap toolchain registry contains a symlink")
    return target


def binding_declarations(root, config, read_input=None):
    """Read source expectations before probing or provisioning the selected SDK."""
    native = {"system": platform.system(), "machine": platform.machine()}
    if "tool_bindings_registry" in config:
        if "tool_bindings" in config or "native_platform" in config:
            raise ValueError("conflicting bootstrap binding declarations")
        path = binding_registry_path(root, config)
        registry = json.loads(path.read_bytes() if read_input is None else
                              read_input(root, config["tool_bindings_registry"]))
        if (registry.get("schema") != "chrono-bootstrap-toolchains/v1"
                or registry.get("rust_toolchain") != config["rust_toolchain"]):
            raise ValueError("bootstrap toolchain registry differs")
        entry = registry.get("platforms", {}).get(native["system"] + "/" + native["machine"])
        if not isinstance(entry, dict) or not isinstance(entry.get("tools"), dict):
            raise ValueError("unsupported or missing native bootstrap registration")
        if any(not isinstance(item, dict) for item in entry["tools"].values()):
            raise ValueError("invalid native executable binding")
        declared = {name: {"sha256": item.get("sha256"), "version": item.get("version"),
                           "allow_relocation": True}
                    for name, item in entry["tools"].items()}
    else:
        declared = config.get("tool_bindings")
        if config.get("require_tool_bindings"):
            if declared is None:
                raise ValueError("bootstrap requires declared Cargo/rustc bindings")
            if config.get("native_platform") != native:
                raise ValueError("unsupported or missing native bootstrap registration")
    if declared is None:
        return None
    if not isinstance(declared, dict) or set(declared) != {"cargo", "rustc"}:
        raise ValueError("tool_bindings must declare cargo and rustc")
    for tool, item in declared.items():
        if (not isinstance(item, dict)
                or set(item) - {"path", "sha256", "version", "allow_relocation"}
                or ("path" in item and not isinstance(item["path"], str))
                or ("path" not in item and "tool_bindings_registry" not in config)
                or not isinstance(item.get("sha256"), str)
                or not re.fullmatch(r"[0-9a-f]{64}", item["sha256"])
                or not isinstance(item.get("version"), str)
                or type(item.get("allow_relocation", False)) is not bool):
            raise ValueError("invalid " + tool + " executable binding")
    return declared


def registration_paths(config_path, config):
    paths = [config_path, config["projects"], ".chrono-harness/ci/bootstrap.py"]
    if "tool_bindings_registry" in config:
        paths.append(config["tool_bindings_registry"])
    if "owner_upgrade" in config or "owner_stage" in config:
        paths.extend([".chrono-harness/ci/bootstrap_owner.py", ".chrono-harness/ci/bootstrap_shared.py"])
    if "owner_upgrade" in config:
        paths.append(config["owner_upgrade"])
    return paths


def declared_bindings(root, config, env):
    """Resolve and validate the explicit Cargo/rustc selection contract.

    The declaration is intentionally separate from observations: its digest and
    version are checked against the selected files before any registered action
    runs.  `allow_relocation` permits an equivalent rustup installation to move
    while retaining the observed local path in the report.
    """
    declared = binding_declarations(root, config)
    if declared is None:
        return None
    toolchain = config["rust_toolchain"]
    selected = {}
    for tool in ("cargo", "rustc"):
        item = declared[tool]
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
        if not item.get("allow_relocation", False):
            expected_path = Path(item["path"])
            if not expected_path.is_absolute():
                expected_path = root / expected_path
            if selected_path != expected_path:
                raise ValueError("bound " + tool + " relocation is not permitted")
        configured = config.get("tools", {}).get(tool)
        if configured is None and "tool_bindings_registry" not in config:
            raise ValueError("tool_bindings requires configured " + tool)
        # In a relocatable legacy declaration the repeated original path is a
        # historical location, not a second current selector. Never execute it.
        retired = item.get("allow_relocation", False) and configured == item.get("path")
        if configured is not None and not retired:
            configured_path = shutil.which(configured, path=env.get("PATH")) if not Path(configured).is_absolute() else configured
            if configured_path is None or not Path(configured_path).is_file():
                raise ValueError("configured " + tool + " executable is missing")
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


def provision_environment(root, config):
    """Resolve the host's opt-in private selection before any installer effect."""
    env = dict(os.environ)
    if "environment" in config:
        values = config["environment"]
        if (not isinstance(values, dict) or any(
                not isinstance(k, str) or not k or "=" in k or "\0" in k
                or k.startswith("CHRONO_") or not isinstance(v, str) or "\0" in v
                for k, v in values.items())):
            raise ValueError("invalid bootstrap environment selection")
        env.update(values)
    if "host_environment" in config:
        values = config["host_environment"]
        if (not isinstance(values, dict) or not values or any(not isinstance(v, str) for v in values.values())
                or set(values) - {"HOME", "CARGO_HOME", "TMPDIR"}
                or set(values) & set(config.get("environment", {}))):
            raise ValueError("invalid bootstrap host environment selection")
        env.update({key: str(host_path(root, value, ".chrono-harness/state/"))
                    for key, value in values.items()})
    declaration = config.get("toolchain_provision")
    if "toolchain_provision" in config:
        if (not isinstance(declaration, dict)
                or set(declaration) != {"profile", "rustup_home", "cargo_home"}
                or declaration["profile"] != "default"):
            raise ValueError("invalid isolated toolchain provision declaration")
        if any(not isinstance(declaration[key], str) for key in ("rustup_home", "cargo_home")):
            raise ValueError("isolated toolchain homes must be host-state paths")
        homes = [host_path(root, declaration[key], ".chrono-harness/state/")
                 for key in ("rustup_home", "cargo_home")]
        if homes[0] == homes[1] or any(a in b.parents for a, b in
                                      [(homes[0], homes[1]), (homes[1], homes[0])]):
            raise ValueError("isolated Rustup/Cargo homes must be disjoint")
        env.update(RUSTUP_HOME=str(homes[0]), CARGO_HOME=str(homes[1]))
        # The opt-in selection replaces the inherited compiler; the existing
        # declaration still checks and supplies the selected compiler's bytes.
        env["RUSTC"] = ""
    return env


def toolchain_environment(config, root=None):
    root = root or Path.cwd()
    binding_declarations(root, config)
    env = provision_environment(root, config)
    toolchain = config["rust_toolchain"]
    if "toolchain_provision" in config:
        # Delegate component production and metadata to Rustup. The official
        # default profile includes the six components required by the local
        # retained inventory; minimal + rustfmt cannot supply that inventory.
        subprocess.run(["rustup", "toolchain", "install", toolchain, "--profile", "default",
                        "--no-self-update"], cwd=root, env=env, pass_fds=PASS_FDS, check=True)
    probe = subprocess.run(["rustup", "run", toolchain, "cargo", "--version"], cwd=root, env=env,
                           pass_fds=PASS_FDS, capture_output=True)
    if probe.returncode:
        if "toolchain_provision" in config:
            raise ValueError("isolated default toolchain Cargo probe failed: " + str(probe.returncode))
        subprocess.run(["rustup", "toolchain", "install", toolchain, "--profile", "minimal", "--component", "rustfmt"], cwd=root, env=env, pass_fds=PASS_FDS, check=True)
    fmt_probe = subprocess.run(["rustup", "run", toolchain, "rustfmt", "--version"], cwd=root, env=env,
                               pass_fds=PASS_FDS, capture_output=True)
    if fmt_probe.returncode:
        if "toolchain_provision" in config:
            raise ValueError("isolated default toolchain rustfmt probe failed: " + str(fmt_probe.returncode))
        subprocess.run(["rustup", "component", "add", "--toolchain", toolchain, "rustfmt"], cwd=root, env=env, pass_fds=PASS_FDS, check=True)
    env.update(RUSTUP_TOOLCHAIN=toolchain, CARGO_TERM_COLOR="never")
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


def registered_operations(projects):
    operations = {}
    for project in projects["projects"] + projects["scripts"]:
        for action in project["actions"].values():
            key = action["operation"]
            if key in operations:
                raise ValueError("duplicate operation: " + key)
            operations[key] = action
    return operations


def operation_sequence(config):
    """Explicit prerequisites run in the caller domain before SDK selection."""
    prerequisites = config.get("prerequisites", [])
    if (not isinstance(prerequisites, list)
            or any(not isinstance(name, str) or not name for name in prerequisites)
            or len(set(prerequisites)) != len(prerequisites)
            or set(prerequisites) & set(config["operations"])):
        raise ValueError("invalid bootstrap prerequisites")
    return prerequisites + config["operations"]


def run_operation(root, directory, index, operation, action, executable, env):
    """Retain original bytes while forwarding them to the existing bounded owner.

    Lifecycle descriptors, process ownership, time and output limits remain with
    the registered invoking owner. Streams are forwarded as they arrive, so
    retaining them does not hide output from that owner or buffer it in memory.
    """
    argv = [executable, *action["argv"]]
    record = {"operation": operation, "tool": action["tool"], "argv": argv,
              "cwd": str(root), "executable_sha256": file_digest(Path(executable))}
    errors = []

    def forward(pipe, channel, destination):
        path = directory / (str(index) + "." + channel)
        try:
            with path.open("xb", buffering=0) as output:
                while True:
                    chunk = pipe.read1(65536)
                    if not chunk:
                        break
                    output.write(chunk)
                    destination.write(chunk)
                    destination.flush()
            record[channel] = {"path": path.relative_to(root).as_posix(),
                               "sha256": file_digest(path), "size": path.stat().st_size}
        except (OSError, ValueError) as error:
            errors.append(error)
        finally:
            pipe.close()

    started = time.monotonic()
    with subprocess.Popen(argv, cwd=root, env=env, pass_fds=PASS_FDS,
                          stdout=subprocess.PIPE, stderr=subprocess.PIPE) as process:
        record["pid"] = process.pid
        workers = [threading.Thread(target=forward, args=(pipe, channel, destination))
                   for pipe, channel, destination in [(process.stdout, "stdout", sys.stdout.buffer),
                                                       (process.stderr, "stderr", sys.stderr.buffer)]]
        for worker in workers:
            worker.start()
        record["exit_code"] = process.wait()
        for worker in workers:
            worker.join()
    record["duration_seconds"] = time.monotonic() - started
    if errors:
        raise errors[0]
    return record


def source_identity(before, after):
    if before["error"] or after["error"]:
        state = "unavailable"
    elif before != after:
        state = "changed"
    elif before["dirty"]:
        state = "dirty"
    else:
        state = "clean"
    return {"state": state, "commit": before["commit"] if state == "clean" else None,
            "tree": before["tree"] if state == "clean" else None, "before": before, "after": after}


def main():
    publication = len(sys.argv) == 6 and sys.argv[3] == "--owner-publication"
    if len(sys.argv) not in (2, 3) and not publication:
        raise ValueError("usage: bootstrap.py HOST_ROOT [REGISTERED_CONFIG]")
    root = Path(sys.argv[1]).resolve()
    config_path = Path(sys.argv[2]) if len(sys.argv) >= 3 else Path(".chrono-harness/ci/bootstrap.json")
    if config_path.is_absolute() or ".." in config_path.parts or config_path.parts[:2] != (".chrono-harness", "ci"):
        raise ValueError("bootstrap config must be an explicit host CI path")
    config = json.loads((root / config_path).read_text())
    if publication:
        import bootstrap_owner
        bootstrap_owner.publication(root, sys.argv[4], sys.argv[5])
        return
    report_destination(root, config)
    if "rust_incremental" in config and type(config["rust_incremental"]) is not bool:
        raise ValueError("rust_incremental must be a boolean")
    if type(config.get("require_tool_bindings", False)) is not bool:
        raise ValueError("require_tool_bindings must be a boolean")
    binding_declarations(root, config)
    provision_environment(root, config)
    operation_sequence(config)
    participate(root, config_path, config)
    previous_owner = None
    if "owner_stage" in config:
        import bootstrap_owner
        bootstrap_owner.require_capabilities(root)
        previous_owner = bootstrap_owner.file_binding(
            host_path(root, config["owner_stage"]["selected"], ".chrono-harness/bin/"))
    source_before = observe_source(root)
    if config["schema"] != "chrono-bootstrap/v1":
        raise ValueError("unsupported bootstrap schema")
    projects = json.loads((root / config["projects"]).read_text())
    operations = registered_operations(projects)
    prerequisites = config.get("prerequisites", [])
    for name in prerequisites:
        if name not in operations or operations[name]["tool"] not in config["tools"]:
            raise ValueError("bootstrap prerequisite requires a configured tool: " + name)
    report = report_destination(root, config)
    report.parent.mkdir(parents=True, exist_ok=True)
    directory = Path(tempfile.mkdtemp(prefix="bootstrap-", dir=report.parent))
    result = {"schema": "chrono-bootstrap-result/v1", "config": config_path.as_posix(),
              "host_root": str(root), "versions": {}, "status": "failed",
              "registrations": {path: file_digest(root / path) for path in
                                registration_paths(config_path.as_posix(), config)},
              "environment": {},
              "operations": [], "installed": []}
    try:
        # A verifier of an installed profile must independently select its
        # current caller domain. The enclosing build selection is not yet active.
        for index, operation in enumerate(prerequisites):
            action = operations[operation]
            executable = config["tools"][action["tool"]]
            if not Path(executable).is_absolute() and "/" in executable:
                executable = str(root / executable)
            executable = shutil.which(executable, path=os.environ.get("PATH")) or executable
            record = run_operation(root, directory, index, operation, action, executable, dict(os.environ))
            result["operations"].append(record)
            if record["exit_code"]:
                raise subprocess.CalledProcessError(record["exit_code"], record["argv"])
        # Only after prerequisites succeed may the selected build homes and SDK
        # have effects. Ordinary participation already protects all these jobs.
        for path in config.get("host_environment", {}).values():
            host_path(root, path, ".chrono-harness/state/").mkdir(parents=True, exist_ok=True)
        env, versions, bindings = toolchain_environment(config, root)
        result["versions"] = versions
        result["environment"] = {key: env.get(key) for key in ["RUSTUP_TOOLCHAIN", "CARGO_INCREMENTAL",
                                                               "RUSTC", "RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER"]}
        if bindings:
            result["executables"] = [bindings[tool] for tool in ("cargo", "rustc")]
            result["selection"] = {"toolchain": config["rust_toolchain"],
                                   "cargo": bindings["cargo"]["path"],
                                   "rustc": bindings["rustc"]["path"], "compiler": env["RUSTC"]}
        if "toolchain_provision" in config:
            result["toolchain_provision"] = config["toolchain_provision"]
            result["environment"].update({key: env[key] for key in ("RUSTUP_HOME", "CARGO_HOME")})
        if "host_environment" in config:
            result["host_environment"] = config["host_environment"]
            result["environment"].update({key: env[key] for key in config["host_environment"]})
        for index, operation in enumerate(config["operations"], start=len(prerequisites)):
            action = operations[operation]
            executable = (bindings[action["tool"]]["path"] if bindings and action["tool"] in bindings
                          else config["tools"][action["tool"]])
            if not Path(executable).is_absolute() and "/" in executable:
                executable = str(root / executable)
            executable = shutil.which(executable, path=env.get("PATH")) or executable
            record = run_operation(root, directory, index, operation, action, executable, env)
            result["operations"].append(record)
            if record["exit_code"]:
                raise subprocess.CalledProcessError(record["exit_code"], record["argv"])
        if bindings and declared_bindings(root, config, env) != bindings:
            raise ValueError("bound executable changed during bootstrap")
        for item in config["install"]:
            source, target = root / item["source"], root / item["destination"]
            if not source.is_file():
                raise ValueError("missing built tool: " + str(source))
            if item["destination"] == config.get("participation", {}).get("program"):
                import bootstrap_owner
                bootstrap_owner.check_shared_install(root, source, target)
                if target.exists() and file_digest(source) == file_digest(target):
                    result["installed"].append({"path": item["destination"], "sha256": file_digest(target)})
                    continue
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, target)
            result["installed"].append({"path": item["destination"], "sha256": file_digest(target)})
        if previous_owner is not None:
            bootstrap_owner.stage(root, config, result, previous_owner)
        result["status"] = "passed"
    except (ValueError, KeyError, OSError, subprocess.CalledProcessError) as error:
        result["error"] = str(error)
        raise
    finally:
        result["source"] = source_identity(source_before, observe_source(root))
        raw = json.dumps(result, indent=2) + "\n"
        (directory / "report.json").write_text(raw)
        print("bootstrap original report: " + str(directory / "report.json"), file=sys.stderr)
    report.write_text(raw)
    if previous_owner is not None:
        print("chrono-owner-stage-result " + json.dumps({
            "path": (directory / "report.json").relative_to(root).as_posix(),
            "sha256": file_digest(directory / "report.json")}))


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, OSError, subprocess.CalledProcessError) as error:
        print("E_BOOTSTRAP:", error, file=sys.stderr)
        sys.exit(error.returncode if isinstance(error, subprocess.CalledProcessError)
                 and 0 < error.returncode <= 255 else 1)
