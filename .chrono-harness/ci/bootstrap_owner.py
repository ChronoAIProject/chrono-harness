"""Bootstrap construction and original qualification, under the worktree owner.

This module never interprets a lifecycle ledger or acquires a lease. The worktree
owner supplies construction SH and keeps its descriptors alive through this
module's child processes. Rust owns stage validation, byte publication and adoption.
"""
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile

import bootstrap
import bootstrap_shared as shared

STAGES = ".chrono-harness/bin/owner-stages/"
CURRENT = ".chrono-harness/state/bootstrap-owner-current.json"


def file_binding(path):
    path = Path(path)
    if path.is_symlink() or not path.is_file():
        raise ValueError("owner executable must be a physical regular file")
    before = path.stat()
    digest = bootstrap.file_digest(path)
    after = path.stat()
    if (before.st_dev, before.st_ino, before.st_size, before.st_mtime_ns) != (
            after.st_dev, after.st_ino, after.st_size, after.st_mtime_ns):
        raise ValueError("owner executable changed during observation")
    return {"identity": str(after.st_dev) + ":" + str(after.st_ino),
            "sha256": digest, "size": after.st_size}


def require_capabilities(root):
    if os.environ.get("CHRONO_WORKTREE_BOOTSTRAP") != str(root) or not bootstrap.PASS_FDS:
        raise ValueError("owner operation requires original ownership descriptors")
    for fd in bootstrap.PASS_FDS:
        os.fstat(fd)


def committed_reader(root, source):
    if (set(source) != {"commit", "tree"} or any(
            not isinstance(source[k], str) or not re.fullmatch(r"[0-9a-f]{40}", source[k])
            for k in source)):
        raise ValueError("owner qualification requires an immutable producer commit and tree")
    def git(*args):
        return subprocess.check_output(["git", "--no-optional-locks", "--no-replace-objects",
                                        "-C", str(root), *args], stderr=subprocess.PIPE,
                                       pass_fds=bootstrap.PASS_FDS)
    if git("rev-parse", source["commit"] + "^{tree}").decode().strip() != source["tree"]:
        raise ValueError("immutable producer tree differs")
    def read_input(unused_root, path, prefix=None):
        shared.safe(root, path, prefix)
        return git("show", source["commit"] + ":" + path)
    return read_input


def qualified_selected_original(root, reference, selected):
    """Qualify installed input from its actual producer, before construction."""
    selected_before = file_binding(selected)
    current_path = shared.safe(root, CURRENT)
    if current_path.exists():
        current = json.loads(current_path.read_bytes())
        if current.get("installed") == selected_before:
            return qualified_publication(root, current, selected)
    path = shared.safe(root, reference, ".chrono-harness/state/")
    original = path.read_bytes()
    report = json.loads(original)
    identity = report["source"]
    source = {key: identity[key] for key in ("commit", "tree")}
    observed = dict(source, dirty=False, error=None)
    if (identity != bootstrap.source_identity(observed, observed)
            or report.get("host_root") != str(root)):
        raise ValueError("selected owner requires its original clean producer in this host")
    read_input = committed_reader(root, source)
    profile_path = report["config"]
    profile = json.loads(read_input(root, profile_path, ".chrono-harness/ci/"))
    if bootstrap.report_destination(root, profile) != path:
        raise ValueError("selected owner original differs from its producer destination")
    destination = selected.relative_to(root).as_posix()
    installed = [item for item in profile["install"] if item["destination"] == destination]
    if len(installed) != 1:
        raise ValueError("selected owner was not uniquely installed by this producer")
    validated = shared.original_report(
        root, {"config": profile_path}, profile, observed, read_input)
    if validated != original or file_binding(selected) != selected_before:
        raise ValueError("selected owner or its original changed during qualification")
    return {"schema": "chrono-bootstrap-selected-original/v1", "path": reference,
            "sha256": shared.digest(original), "source": source, "selected": selected_before}


def qualified_publication(root, current, selected):
    """Read fixed producer data; never execute an older bootstrap or judge."""
    receipt = current["receipt"]
    raw = shared.read(root, receipt["path"], ".chrono-harness/state/automatic-cleanup-v2/")
    if (shared.digest(raw) != receipt["sha256"]
            or json.loads(raw) != {k: v for k, v in current.items() if k != "receipt"}
            or current.get("schema") != "chrono-worktree-owner-adoption/v1"):
        raise ValueError("selected owner adoption original differs")
    report, _ = stage_report(root, current["stage_result"], current["stage_sha256"],
                             producer_commit=current["source_commit"])
    record = report["owner_stage"]
    staged = shared.safe(root, record["staged"]["path"], STAGES)
    publication = current["publication"]
    installed = file_binding(selected)
    expected = {"schema": "chrono-bootstrap-owner-publication/v1", "stage": record,
                "installed": installed, "adoption": "not-claimed", "task_completion": "not-claimed"}
    intent = {"schema": "chrono-bootstrap-owner-publication-intent/v1",
              "stage": record, "replacement": installed}
    if (record["selected"] != selected.relative_to(root).as_posix()
            or current["installed"] != installed or publication != expected
            or installed["sha256"] != record["staged"]["sha256"]
            or current["original_owner"] != record["previous"]
            or file_binding(staged.parent / "previous-owner") != record["previous"]
            or json.loads(shared.read(root, (staged.parent / "publication-intent.json").relative_to(root).as_posix())) != intent
            or json.loads(shared.read(root, (staged.parent / "publication-result.json").relative_to(root).as_posix())) != expected):
        raise ValueError("selected owner publication originals differ")
    if file_binding(selected) != installed:
        raise ValueError("selected owner changed during publication qualification")
    return {"schema": "chrono-bootstrap-selected-original/v1", "path": current["stage_result"],
            "sha256": current["stage_sha256"],
            "source": {k: report["source"][k] for k in ("commit", "tree")}, "selected": installed}


def check_shared_install(root, source, target, adoption):
    if os.environ.get("CHRONO_WORKTREE_BOOTSTRAP") == str(root):
        anchor = bootstrap.coordinator_root(root, adoption)
        local = bootstrap.host_path(root, adoption["program"], ".chrono-harness/bin/")
        if target != local:
            raise ValueError("lifecycle install destination differs from participation")
        current = file_binding(target) if target.exists() else None
        if anchor != root:
            selected = bootstrap.host_path(anchor, adoption["program"], ".chrono-harness/bin/")
            coordinator = file_binding(selected)
            if current is None or current["identity"] != coordinator["identity"]:
                return
        if current is None or file_binding(source)["sha256"] != current["sha256"]:
            raise ValueError("changed lifecycle executable requires exclusive owner deployment")


def reuse_construction(root, config, result, observed):
    """Consume the current owner's actual construction instead of building it twice."""
    names = config.get("reuse_owner_operations", [])
    if (not isinstance(names, list) or any(not isinstance(name, str) for name in names)
            or len(set(names)) != len(names) or not set(names) <= set(config["operations"])
            or (names and "owner_upgrade" not in config)):
        raise ValueError("invalid owner construction reuse declaration")
    if not names:
        return {}, None
    adoption = config["participation"]
    if bootstrap.coordinator_root(root, adoption) != root:
        return {}, None
    current_path = shared.safe(root, CURRENT)
    if not current_path.exists():
        return {}, None  # A physical main's first source build still seeds its owner.
    current = json.loads(current_path.read_bytes())
    selected = shared.safe(root, adoption["program"], ".chrono-harness/bin/")
    if current.get("source_commit") != observed["commit"]:
        raise ValueError("owner construction must belong to the current candidate")
    qualified_publication(root, current, selected)
    report, profile = stage_report(root, current["stage_result"], current["stage_sha256"])
    for field in ("projects", "rust_toolchain", "rust_incremental", "require_tool_bindings",
                  "tool_bindings_registry", "tool_bindings", "native_platform",
                  "host_environment", "environment", "toolchain_provision"):
        if profile.get(field) != config.get(field):
            raise ValueError("owner construction selection differs: " + field)
    for field in ("environment", "executables", "selection", "versions", "host_environment"):
        if report.get(field) != result.get(field):
            raise ValueError("owner construction observation differs: " + field)
    rows = {row["operation"]: row for row in report["operations"]}
    if not set(names) <= set(rows):
        raise ValueError("owner construction is missing a declared operation")
    for tool in sorted({rows[name]["tool"] for name in names}):
        if profile.get("tools", {}).get(tool) != config.get("tools", {}).get(tool):
            raise ValueError("owner construction selection differs: tool " + tool)
    source = shared.safe(root, report["owner_stage"]["staged"]["path"], STAGES)
    reused = {name: dict(rows[name], reused_from={"path": current["stage_result"],
                                                "sha256": current["stage_sha256"]})
              for name in names}
    return reused, source


def stage(root, config, result, previous):
    require_capabilities(root)
    declaration = config["owner_stage"]
    selected = shared.safe(root, declaration["selected"], ".chrono-harness/bin/")
    if file_binding(selected) != previous:
        raise ValueError("selected owner changed during construction")
    source = shared.safe(root, declaration["source"])
    before = file_binding(source)
    parent = shared.safe(root, STAGES[:-1])
    parent.mkdir(parents=True, exist_ok=True)
    directory = Path(tempfile.mkdtemp(prefix="stage-", dir=parent))
    target = directory / "chrono-worktree"
    with source.open("rb") as input_stream, target.open("xb") as output:
        shutil.copyfileobj(input_stream, output)
        os.fchmod(output.fileno(), 0o755)
    if file_binding(source) != before or file_binding(target)["sha256"] != before["sha256"]:
        raise ValueError("built owner changed during staging")
    result["owner_stage"] = {"selected": declaration["selected"], "previous": previous,
                             "staged": dict(file_binding(target), path=target.relative_to(root).as_posix())}


def stage_report(root, reference, expected_sha=None, producer_commit=None):
    path = shared.safe(root, reference, ".chrono-harness/state/bootstrap-")
    raw = path.read_bytes()
    if expected_sha is not None and shared.digest(raw) != expected_sha:
        raise ValueError("owner stage original report differs")
    report = json.loads(raw)
    read_input = shared.read
    if producer_commit is None:
        observed = bootstrap.observe_source(root)
    else:
        source = {k: report["source"][k] for k in ("commit", "tree")}
        if source["commit"] != producer_commit or report.get("host_root") != str(root):
            raise ValueError("selected owner stage producer differs")
        read_input = committed_reader(root, source)
        observed = dict(source, dirty=False, error=None)
    profile = json.loads(read_input(root, report["config"], ".chrono-harness/ci/"))
    if (report.get("source") != bootstrap.source_identity(observed, observed)
            or observed["dirty"] is not False or observed["error"] is not None
            or not profile.get("owner_stage") or report["installed"]):
        raise ValueError("owner stage requires the original current clean source")
    shared.validate_execution(root, profile, report, read_input)
    shared.validate_streams(root, report)
    record = report["owner_stage"]
    staged = shared.safe(root, record["staged"]["path"], STAGES)
    if file_binding(staged) != {k: v for k, v in record["staged"].items() if k != "path"}:
        raise ValueError("retained owner stage changed")
    if record["selected"] != profile["owner_stage"]["selected"]:
        raise ValueError("owner stage selected destination differs")
    return report, profile


def run_owner(root, program, arguments, directory, name):
    env = dict(os.environ)
    consumer = "bootstrap-owner:" + directory.name + ":" + name
    result = subprocess.run([str(program), *arguments, "--report-consumer", consumer], pass_fds=bootstrap.PASS_FDS,
                            env=env, capture_output=True)
    for channel in ("stdout", "stderr"):
        shared.write_new(directory / (name + "." + channel), getattr(result, channel))
    shared.write_new(directory / (name + ".exit"), (str(result.returncode) + "\n").encode())
    sys = __import__("sys")
    sys.stderr.buffer.write(result.stderr)
    try:
        report = json.loads(result.stdout)
    except ValueError:
        sys.stdout.buffer.write(result.stdout)
        raise subprocess.CalledProcessError(result.returncode or 1, result.args)
    declaration = report.get("report_consumer")
    originals = declaration.get("originals") if isinstance(declaration, dict) else None
    if (not isinstance(declaration, dict)
            or declaration.get("schema") != "chrono-retention-consumer/v1"
            or declaration.get("consumer") != consumer
            or not isinstance(report.get("report_path"), str)
            or not isinstance(originals, list) or len(originals) != 1
            or not isinstance(originals[0], dict)
            or originals[0].get("path") != report["report_path"]):
        raise ValueError("owner phase report consumer is missing or differs from the original")
    shared.write_new(directory / (name + "-consumer.json"), shared.encode(declaration))
    process = report.get("managed_process")
    if process is not None:
        process = bootstrap.retained_process(process)
        sys.stdout.buffer.write(bytes(process["stdout_bytes"]))
        sys.stderr.buffer.write(bytes(process["stderr_bytes"]))
    if result.returncode or (process is not None and (process["failure"] is not None or process["exit_code"])):
        raise subprocess.CalledProcessError(
            process["exit_code"] if process is not None and process["exit_code"] else result.returncode or 1,
            result.args)
    return report


def upgrade(root, config_path, config):
    declaration = json.loads(shared.read(root, config["owner_upgrade"], ".chrono-harness/ci/"))
    if (set(declaration) != {"schema", "selected_original", "stage_config"}
            or declaration["schema"] != "chrono-bootstrap-owner-upgrade/v1"):
        raise ValueError("unsupported owner upgrade declaration")
    shared.safe(root, declaration["selected_original"], ".chrono-harness/state/")
    shared.safe(root, declaration["stage_config"], ".chrono-harness/ci/")
    observed = bootstrap.observe_source(root)
    if observed["error"] or observed["dirty"] is not False:
        raise ValueError("owner upgrade requires clean committed candidate source")
    selected = shared.safe(root, config["participation"]["program"], ".chrono-harness/bin/")
    current_path = shared.safe(root, CURRENT)
    if current_path.exists():
        current = json.loads(current_path.read_bytes())
        if (current.get("schema") == "chrono-worktree-owner-adoption/v1"
                and current.get("source_commit") == observed["commit"]
                and current.get("installed") == file_binding(selected)):
            qualified_publication(root, current, selected)
            return
    original = bootstrap.report_destination(root, config)
    if original.exists():
        previous = json.loads(original.read_bytes())
        if previous.get("source") == bootstrap.source_identity(observed, observed):
            shared.original_report(root, {"config": config_path.as_posix()}, config, observed)
            # A real current-source seed/ordinary producer already installed the
            # exact declared set. Its original execution, not a marker, binds it.
            return
    qualification = qualified_selected_original(root, declaration["selected_original"], selected)
    original = shared.safe(root, qualification["path"], ".chrono-harness/state/").read_bytes()
    data = selected.read_bytes()
    if (shared.digest(original) != qualification["sha256"]
            or file_binding(selected) != qualification["selected"]
            or shared.digest(data) != qualification["selected"]["sha256"]
            or len(data) != qualification["selected"]["size"]):
        raise ValueError("qualified selected owner changed before construction")
    parent = shared.safe(root, ".chrono-harness/state/bootstrap-owner-upgrade")
    parent.mkdir(parents=True, exist_ok=True)
    directory = Path(tempfile.mkdtemp(prefix="attempt-", dir=parent))
    shared.write_new(directory / "selected-producer.json", original)
    shared.write_new(directory / "selected-qualification.json", shared.encode(qualification))
    carrier = directory / "construction-owner"
    with carrier.open("xb") as output:
        output.write(data)
        os.fchmod(output.fileno(), 0o755)
    if (file_binding(selected) != qualification["selected"]
            or file_binding(carrier)["sha256"] != qualification["selected"]["sha256"]):
        raise ValueError("qualified selected owner changed before construction")
    shared.write_new(directory / "upgrade.json", shared.encode(declaration))
    common = ["--host-root", str(root), "--config", config["participation"]["config"]]
    run_owner(root, carrier, ["migrate", *common, "--path", str(root)], directory, "migration")
    built = run_owner(root, carrier, ["bootstrap", *common, "--bootstrap-config", declaration["stage_config"]],
                      directory, "construction")
    stdout = bootstrap.retained_process(built["managed_process"])["stdout_bytes"]
    prefix = b"chrono-owner-stage-result "
    last = stdout.splitlines()[-1]
    if not last.startswith(prefix):
        raise ValueError("construction lacks its original stage result")
    descriptor = json.loads(last[len(prefix):])
    report, _ = stage_report(root, descriptor["path"], descriptor["sha256"])
    # run_owner has joined the original construction process before requesting EX.
    staged = shared.safe(root, report["owner_stage"]["staged"]["path"], STAGES)
    run_owner(root, staged, ["deploy-owner", *common, "--bootstrap-config", declaration["stage_config"],
                             "--stage-result", descriptor["path"], "--stage-sha256", descriptor["sha256"]],
              directory, "deployment")
    adopted = json.loads(current_path.read_bytes())
    if adopted.get("source_commit") != observed["commit"] or adopted.get("installed") != file_binding(selected):
        raise ValueError("owner deployment did not publish current adoption")
