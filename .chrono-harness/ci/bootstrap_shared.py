#!/usr/bin/env python3
"""Share this host's registered bootstrap results between primary CI checkouts."""
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import subprocess
import sys
import tempfile

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


def source(root):
    if not (root / ".git").is_dir() or (root / ".git").is_symlink():
        raise ValueError("shared startup requires a primary checkout")
    observed = bootstrap.observe_source(root)
    if observed["error"] or observed["dirty"] is not False:
        raise ValueError("shared startup requires clean source: " + str(observed))
    return observed


def native_platform():
    return {"system": platform.system(), "machine": platform.machine()}


def registration(root, path):
    raw = read(root, path, ".chrono-harness/ci/")
    config = json.loads(raw)
    if (set(config) != {"schema", "directory", "profiles", "consumers"}
            or config["schema"] != "chrono-host-startup/v1"
            or not config["profiles"] or not config["consumers"]):
        raise ValueError("invalid host startup registration")
    directory = config["directory"]
    if not isinstance(directory, str) or not directory.endswith('/'):
        raise ValueError("startup directory must have a trailing slash")
    safe(root, directory[:-1], ".chrono-harness/")
    if directory.startswith((".chrono-harness/state/", ".chrono-harness/bin/", ".chrono-harness/ci/")) or directory == ".chrono-harness/":
        raise ValueError("startup transfer overlaps source, tools or evidence")
    destinations, reports = set(), set()
    profiles = {}
    for name, item in config["profiles"].items():
        if (not re.fullmatch(r"[A-Za-z0-9_-]+", name) or set(item) != {"config", "build"}
                or type(item["build"]) is not bool):
            raise ValueError("invalid startup profile")
        data = read(root, item["config"], ".chrono-harness/ci/")
        profile = json.loads(data)
        if profile["schema"] != "chrono-bootstrap/v1" or not profile["install"]:
            raise ValueError("startup profile must install declared tools")
        report = bootstrap.report_destination(root, profile)
        if report in reports:
            raise ValueError("startup profiles must have separate reports")
        reports.add(report)
        for installed in bootstrap.installations(root, profile):
            target = safe(root, installed["destination"], ".chrono-harness/bin/")
            if target in destinations:
                raise ValueError("startup profiles must have separate tools")
            destinations.add(target)
        profiles[name] = (profile, digest(data))
    for selected in config["consumers"].values():
        if (not isinstance(selected, list) or not selected or len(set(selected)) != len(selected)
                or any(name not in profiles for name in selected)):
            raise ValueError("startup consumer requires explicit profiles")
    return config, profiles, digest(raw)


def original_report(root, item, profile, observed):
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
    if "distribution" in profile:
        original = read(root, ".chrono-harness/state/distribution.json")
        bootstrap.validate_distribution(root, profile, original)
        if report.get("distribution") != bootstrap.distribution_binding(original):
            raise ValueError("startup distribution receipt differs from producer binding")
    elif "distribution" in report:
        raise ValueError("unregistered startup distribution receipt")
    return data


def write_new(path, data):
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("xb") as output:
        output.write(data)


def encode(value):
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode()


def produce(root, path, config, profiles, config_sha):
    observed = source(root)
    transfer = safe(root, config["directory"][:-1])
    if transfer.exists():
        raise ValueError("startup transfer already exists")
    for item in config["profiles"].values():
        if item["build"]:
            subprocess.run([sys.executable, "-B", str(root / ".chrono-harness/ci/bootstrap.py"),
                            str(root), item["config"]], check=True, pass_fds=bootstrap.PASS_FDS)
    if source(root) != observed:
        raise ValueError("startup source changed during production")
    state = safe(root, ".chrono-harness/state")
    state.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="startup-export-", dir=state) as temporary:
        staging = Path(temporary) / "artifact"
        staging.mkdir()
        manifest = {"schema": "chrono-host-startup-artifact/v1", "source": observed,
                    "platform": native_platform(), "config": path, "config_sha256": config_sha,
                    "profiles": {}}
        index = 0
        for name, item in config["profiles"].items():
            profile, profile_sha = profiles[name]
            report = original_report(root, item, profile, observed)
            report_path = "reports/" + name + ".json"
            write_new(staging / report_path, report)
            entry = {"config": item["config"], "config_sha256": profile_sha,
                     "report": {"path": report_path, "sha256": digest(report)}, "files": []}
            if "distribution" in profile:
                original = read(root, ".chrono-harness/state/distribution.json")
                receipt_path = "reports/" + name + "-distribution.json"
                write_new(staging / receipt_path, original)
                entry["distribution_report"] = {"path": receipt_path, "sha256": digest(original)}
            for installed in bootstrap.installations(root, profile):
                target = safe(root, installed["destination"])
                data, mode = target.read_bytes(), target.stat().st_mode & 0o777
                if not mode & 0o111:
                    raise ValueError("startup tool is not executable")
                asset = "tools/" + str(index)
                index += 1
                write_new(staging / asset, data)
                entry["files"].append({"path": asset, "destination": installed["destination"],
                                       "sha256": digest(data), "size": len(data), "mode": mode})
            manifest["profiles"][name] = entry
        raw = encode(manifest)
        write_new(staging / "manifest.json", raw)
        if source(root) != observed:
            raise ValueError("startup source changed before publication")
        staging.rename(transfer)
    # A failed output publication must fail the producer step and prevent use.
    with Path(os.environ["GITHUB_OUTPUT"]).open("a") as output:
        output.write("binding=" + digest(raw) + "\n")


def validate_artifact(root, path, config, profiles, config_sha, binding):
    observed = source(root)
    if not isinstance(binding, str) or not re.fullmatch(r"[0-9a-f]{64}", binding):
        raise ValueError("missing or invalid startup producer binding")
    transfer = safe(root, config["directory"][:-1])
    raw = read(transfer, "manifest.json")
    if digest(raw) != binding:
        raise ValueError("startup manifest differs from producer binding")
    manifest = json.loads(raw)
    if (manifest["schema"] != "chrono-host-startup-artifact/v1"
            or manifest["config"] != path or manifest["config_sha256"] != config_sha
            or manifest["source"] != observed or manifest["platform"] != native_platform()
            or set(manifest["profiles"]) != set(profiles)):
        raise ValueError("startup source, platform or registration differs")
    payloads, originals = {}, {}
    seen = set()
    for name, (profile, profile_sha) in profiles.items():
        entry = manifest["profiles"][name]
        if entry["config"] != config["profiles"][name]["config"] or entry["config_sha256"] != profile_sha:
            raise ValueError("startup profile differs")
        if [f["destination"] for f in entry["files"]] != [f["destination"] for f in bootstrap.installations(root, profile)]:
            raise ValueError("startup install set differs")
        original = read(transfer, entry["report"]["path"], "reports/")
        if digest(original) != entry["report"]["sha256"]:
            raise ValueError("startup original report differs")
        report = json.loads(original)
        if (report["schema"] != "chrono-bootstrap-result/v1" or report["config"] != entry["config"]
                or "provenance" in report or report["source"]["state"] != "clean"
                or report["source"]["before"] != observed or report["source"]["after"] != observed
                or report["source"]["commit"] != observed["commit"] or report["source"]["tree"] != observed["tree"]
                or report["installed"] != [{"path": f["destination"], "sha256": f["sha256"]}
                                           for f in entry["files"][:len(profile["install"])]]):
            raise ValueError("startup original report is not bound to this source and payload")
        originals[name] = original
        for item in entry["files"]:
            data = read(transfer, item["path"], "tools/")
            if (item["path"] in seen or digest(data) != item["sha256"] or len(data) != item["size"]
                    or type(item["mode"]) is not int or not 0 < item["mode"] <= 0o777
                    or not item["mode"] & 0o111):
                raise ValueError("startup tool bytes or metadata differ")
            seen.add(item["path"])
            payloads[item["destination"]] = data
        if "distribution" in profile:
            receipt = entry["distribution_report"]
            raw_receipt = read(transfer, receipt["path"], "reports/")
            if (digest(raw_receipt) != receipt["sha256"]
                    or report.get("distribution") != bootstrap.distribution_binding(raw_receipt)):
                raise ValueError("startup distribution original report differs")
            bootstrap.validate_distribution(root, profile, raw_receipt, payloads)
        elif "distribution_report" in entry or "distribution" in report:
            raise ValueError("unregistered startup distribution receipt")
    return manifest, payloads, originals


def imported_report(original, binding):
    report = json.loads(original)
    report["provenance"] = {"kind": "shared-startup", "binding": binding, "report_sha256": digest(original)}
    return encode(report)


def replace(path, data, mode=0o644):
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, temporary = tempfile.mkstemp(prefix=".startup-", dir=path.parent)
    try:
        with os.fdopen(fd, "wb") as output:
            output.write(data)
            os.fchmod(output.fileno(), mode)
        os.replace(temporary, path)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)


def install(root, path, config, profiles, config_sha, consumer):
    if consumer not in config["consumers"]:
        raise ValueError("unknown startup consumer")
    binding = os.environ.get("CHRONO_STARTUP_BINDING")
    manifest, payloads, originals = validate_artifact(root, path, config, profiles, config_sha, binding)
    selected = config["consumers"][consumer]
    report_path = safe(root, ".chrono-harness/state/startup-install.json")
    if report_path.exists():
        raise ValueError("startup installation already has a result")
    # Check all files, original reports and destinations before replacing a tool.
    writes = []
    for name in selected:
        profile = profiles[name][0]
        for item in manifest["profiles"][name]["files"]:
            writes.append((safe(root, item["destination"]), payloads[item["destination"]], item["mode"]))
        writes.append((bootstrap.report_destination(root, profile), imported_report(originals[name], binding), 0o644))
        writes.append((safe(root, ".chrono-harness/state/startup-originals/" + name + ".json"), originals[name], 0o644))
        if "distribution" in profile:
            original = read(safe(root, config["directory"][:-1]),
                            manifest["profiles"][name]["distribution_report"]["path"], "reports/")
            # Preserve installer bytes and its release-source identity without relabelling.
            writes.append((safe(root, ".chrono-harness/state/distribution.json"), original, 0o644))
            writes.append((safe(root, ".chrono-harness/state/startup-originals/" + name + "-distribution.json"), original, 0o644))
    if any(target.exists() and not target.is_file() for target, _, _ in writes):
        raise ValueError("startup destination is not a file")
    versions = {}
    for name in selected:
        profile = profiles[name][0]
        if profile["rust_toolchain"] not in versions:
            _, observed = bootstrap.toolchain_environment(profile)
            versions[profile["rust_toolchain"]] = observed
        if versions[profile["rust_toolchain"]] != json.loads(originals[name])["versions"]:
            raise ValueError("startup consumer toolchain differs")
    if source(root) != manifest["source"]:
        raise ValueError("startup consumer source changed before installation")
    for target, data, mode in writes:
        replace(target, data, mode)
    replace(report_path, encode({"schema": "chrono-host-startup-install/v1", "binding": binding,
                                "consumer": consumer, "profiles": selected, "source": manifest["source"]}))


def verify(root, path, config, profiles, config_sha, name):
    result = json.loads(read(root, ".chrono-harness/state/startup-install.json"))
    if (result["schema"] != "chrono-host-startup-install/v1" or name not in result["profiles"]
            or config["consumers"].get(result["consumer"]) != result["profiles"]):
        raise ValueError("startup profile was not installed for the declared consumer")
    manifest, _, originals = validate_artifact(root, path, config, profiles, config_sha, result["binding"])
    if result["source"] != manifest["source"]:
        raise ValueError("startup installation source differs")
    for item in manifest["profiles"][name]["files"]:
        target = safe(root, item["destination"])
        if digest(target.read_bytes()) != item["sha256"] or target.stat().st_mode & 0o777 != item["mode"]:
            raise ValueError("installed startup tool differs: " + item["destination"])
    if bootstrap.report_destination(root, profiles[name][0]).read_bytes() != imported_report(originals[name], result["binding"]):
        raise ValueError("installed startup report differs")
    if "distribution" in profiles[name][0]:
        original = read(safe(root, config["directory"][:-1]),
                        manifest["profiles"][name]["distribution_report"]["path"], "reports/")
        if read(root, ".chrono-harness/state/distribution.json") != original:
            raise ValueError("installed startup distribution report differs")


def main():
    if len(sys.argv) not in (4, 5):
        raise ValueError("usage: bootstrap_shared.py HOST CONFIG produce|install CONSUMER|verify PROFILE")
    root, path, command = Path(sys.argv[1]).resolve(), sys.argv[2], sys.argv[3]
    config, profiles, config_sha = registration(root, path)
    if command == "produce" and len(sys.argv) == 4:
        produce(root, path, config, profiles, config_sha)
    elif command in ("install", "verify") and len(sys.argv) == 5:
        {"install": install, "verify": verify}[command](root, path, config, profiles, config_sha, sys.argv[4])
    else:
        raise ValueError("invalid startup command")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, TypeError, OSError, subprocess.CalledProcessError) as error:
        print("E_STARTUP:", error, file=sys.stderr)
        sys.exit(1)
