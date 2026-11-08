"""Original report helpers still consumed by this host's Python owner adapter.

Shared produce/install/verify now belong to the Rust bootstrap entrypoint.
"""
import hashlib
import json
from pathlib import Path
import sys

sys.dont_write_bytecode = True
import bootstrap


def digest(data):
    return hashlib.sha256(data).hexdigest()


def safe(root, declared, prefix=None):
    if not isinstance(declared, str) or not declared or any(c in declared for c in "\0\r\n"):
        raise ValueError("invalid startup path")
    path = Path(declared)
    if (path.is_absolute() or ".." in path.parts or path.as_posix() != declared
            or prefix is not None and not declared.startswith(prefix)):
        raise ValueError("invalid startup path: " + declared)
    target = root
    for part in path.parts:
        target /= part
        if target.is_symlink():
            raise ValueError("startup path contains a symlink: " + declared)
    return target


def read(root, declared, prefix=None):
    path = safe(root, declared, prefix)
    if not path.is_file():
        raise ValueError("startup input is not a regular file: " + declared)
    return path.read_bytes()


def original_report(root, item, profile, observed, read_input=read):
    path = bootstrap.report_destination(root, profile)
    data = path.read_bytes()
    report = json.loads(data)
    identity = report["source"]
    if (report["schema"] != "chrono-bootstrap-result/v1" or report["config"] != item["config"]
            or "provenance" in report or identity["state"] != "clean"
            or identity["commit"] != observed["commit"] or identity["tree"] != observed["tree"]
            or identity["before"] != observed or identity["after"] != observed):
        raise ValueError("startup requires original clean producer reports")
    expected = [{"path": i["destination"], "sha256": digest(read(root, i["destination"]))}
                for i in profile["install"]]
    if report["installed"] != expected:
        raise ValueError("startup producer report differs from installed tools")
    validate_execution(root, profile, report, read_input)
    validate_streams(root, report)
    return data


def validate_execution(root, profile, report, read_input=read):
    """Bind original producer facts to this source's declarations, never to probes."""
    if report.get("status") != "passed" or not Path(report.get("host_root", "")).is_absolute():
        raise ValueError("startup requires a successful original execution")
    expected_registrations = {path: digest(read_input(root, path)) for path in
                              bootstrap.registration_paths(report["config"], profile)}
    if report.get("registrations") != expected_registrations:
        raise ValueError("startup execution registrations differ")
    if "host_environment" in profile:
        # This is fixed producer data: bind its declared location literally.
        # Admission of current writable homes belongs to provision_environment.
        if (report.get("host_environment") != profile["host_environment"]
                or any(report.get("environment", {}).get(key) != str(
                    bootstrap.host_path(Path(report["host_root"]), path))
                    for key, path in profile["host_environment"].items())):
            raise ValueError("startup producer host environment differs")
    bindings = bootstrap.binding_declarations(root, profile, read_input)
    selected = {}
    if bindings is not None:
        rows = report.get("executables", [])
        if len(rows) != 2:
            raise ValueError("startup producer executable bindings are missing")
        for name, row in zip(("cargo", "rustc"), rows):
            if (row.get("sha256") != bindings[name]["sha256"]
                    or row.get("version") != bindings[name]["version"]
                    or not Path(row.get("path", "")).is_absolute()):
                raise ValueError("startup producer executable binding differs")
            selected[name] = row["path"]
            if not bindings[name].get("allow_relocation", False):
                declared_path = Path(bindings[name]["path"])
                if not declared_path.is_absolute():
                    declared_path = Path(report["host_root"]) / declared_path
                if row["path"] != str(declared_path):
                    raise ValueError("startup producer relocation is not permitted")
        if report.get("selection") != dict(selected, toolchain=profile["rust_toolchain"], compiler=selected["rustc"]):
            raise ValueError("startup producer selection differs")
        if report["versions"] != {name: bindings[name]["version"] for name in bindings}:
            raise ValueError("startup producer versions differ")
        expected_environment = {"RUSTUP_TOOLCHAIN": profile["rust_toolchain"], "RUSTC": selected["rustc"],
                                "RUSTC_WRAPPER": "", "RUSTC_WORKSPACE_WRAPPER": ""}
        if "rust_incremental" in profile:
            expected_environment["CARGO_INCREMENTAL"] = "1" if profile["rust_incremental"] else "0"
        if any(report.get("environment", {}).get(key) != value for key, value in expected_environment.items()):
            raise ValueError("startup producer compiler environment differs")
    operations = bootstrap.registered_operations(json.loads(read_input(root, profile["projects"])))
    actual = report.get("operations", [])
    if [row["operation"] for row in actual] != bootstrap.operation_sequence(profile):
        raise ValueError("startup producer operations differ")
    for row in actual:
        action = operations[row["operation"]]
        executable = (selected[action["tool"]] if action["tool"] in selected
                      else profile["tools"][action["tool"]])
        if not Path(executable).is_absolute() and "/" in executable:
            executable = str(Path(report["host_root"]) / executable)
        if (row["tool"] != action["tool"] or row["argv"] != [executable, *action["argv"]]
                or row["cwd"] != report["host_root"] or type(row["exit_code"]) is not int
                or row["exit_code"] != 0 or type(row["pid"]) is not int or row["pid"] <= 0):
            raise ValueError("startup producer operation execution differs")
        if action["tool"] in selected and row["executable_sha256"] != bindings[action["tool"]]["sha256"]:
            raise ValueError("startup operation executable differs")


def stream_descriptors(report):
    return [row[channel] for row in report["operations"] for channel in ("stdout", "stderr")]


def validate_streams(storage, report):
    paths = set()
    for item in stream_descriptors(report):
        raw = read(storage, item["path"], ".chrono-harness/state/")
        if item["path"] in paths or digest(raw) != item["sha256"] or len(raw) != item["size"]:
            raise ValueError("startup original process stream differs")
        paths.add(item["path"])


def write_new(path, data):
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("xb") as output:
        output.write(data)


def encode(value):
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode()
