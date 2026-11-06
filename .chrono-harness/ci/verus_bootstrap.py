#!/usr/bin/env python3
"""Fetch, verify, and install the registered Verus distribution.

The bootstrap owner invokes each phase as a separate registered operation.  The
archive and digest are inputs from bootstrap-verus.json; this script never
discovers a latest release or accepts a caller-supplied URL.
"""
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import platform
import shutil
import sys
import tempfile
import urllib.error
import urllib.request
import zipfile


def fail(message):
    raise ValueError(message)


def load(root, config_path):
    path = root / config_path
    config = json.loads(path.read_bytes())
    declaration = config.get("verus")
    if not isinstance(declaration, dict):
        fail("missing Verus declaration")
    key = platform.system() + "/" + platform.machine()
    entry = declaration.get("platforms", {}).get(key)
    if not isinstance(entry, dict):
        fail("unsupported Verus host: " + key)
    required = {"version", "url", "archive", "sha256"}
    if set(entry) != required or any(not isinstance(entry[name], str) or not entry[name]
                                     for name in required):
        fail("invalid Verus archive declaration")
    if len(entry["sha256"]) != 64 or any(c not in "0123456789abcdef" for c in entry["sha256"]):
        fail("invalid Verus archive digest")
    if not entry["url"].startswith("https://github.com/verus-lang/verus/releases/download/"):
        fail("Verus archive URL is outside the registered release origin")
    archive = Path(entry["archive"])
    cache = Path(declaration.get("cache_directory", ".chrono-harness/cache/bootstrap-verus"))
    install = Path(declaration.get("install_directory", ".chrono-harness/bin/verus"))
    if archive.is_absolute() or len(archive.parts) != 1 or ".." in archive.parts:
        fail("invalid Verus archive path")
    for name, value in (("cache_directory", cache), ("install_directory", install)):
        if value.is_absolute() or ".." in value.parts or value.as_posix() != str(value):
            fail("invalid Verus " + name + " path")
        if not value.as_posix().startswith(".chrono-harness/"):
            fail("Verus " + name + " must be under .chrono-harness")
    return entry, root / cache, root / install


def digest(path):
    value = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            value.update(chunk)
    return value.hexdigest()


def archive_path(cache, declaration):
    path = cache / declaration["archive"]
    if path.parent != cache:
        fail("Verus archive must be directly in its cache directory")
    return path


def download(root, config_path):
    declaration, cache, install = load(root, config_path)
    del install
    cache.mkdir(parents=True, exist_ok=True)
    target = archive_path(cache, declaration)
    if target.is_file() and digest(target) == declaration["sha256"]:
        print(json.dumps({"phase": "download", "status": "reused", "path": str(target)}))
        return
    if target.exists():
        target.unlink()
    fd, temporary = tempfile.mkstemp(prefix="verus-", suffix=".partial", dir=cache)
    os.close(fd)
    temporary_path = Path(temporary)
    try:
        request = urllib.request.Request(
            declaration["url"], headers={"User-Agent": "chrono-harness-verus-bootstrap/1"})
        with urllib.request.urlopen(request, timeout=120) as response, temporary_path.open("wb") as output:
            while True:
                chunk = response.read(1024 * 1024)
                if not chunk:
                    break
                output.write(chunk)
        if digest(temporary_path) != declaration["sha256"]:
            fail("downloaded Verus archive digest mismatch")
        os.replace(temporary_path, target)
    finally:
        temporary_path.unlink(missing_ok=True)
    print(json.dumps({"phase": "download", "status": "downloaded", "path": str(target)}))


def verify(root, config_path):
    declaration, cache, install = load(root, config_path)
    del install
    target = archive_path(cache, declaration)
    if not target.is_file() or digest(target) != declaration["sha256"]:
        fail("Verus archive is absent or has the wrong digest")
    print(json.dumps({"phase": "verify", "version": declaration["version"],
                      "sha256": declaration["sha256"]}))


def safe_members(archive):
    members = []
    with zipfile.ZipFile(archive) as source:
        for info in source.infolist():
            path = PurePosixPath(info.filename)
            if path.is_absolute() or ".." in path.parts:
                fail("Verus archive contains an unsafe member")
            mode = (info.external_attr >> 16) & 0o170000
            if mode == 0o120000:
                fail("Verus archive contains a symlink")
            members.append(info)
    return members


def extract(root, config_path):
    declaration, cache, install = load(root, config_path)
    archive = archive_path(cache, declaration)
    if not archive.is_file() or digest(archive) != declaration["sha256"]:
        fail("Verus archive is absent or has the wrong digest")
    members = safe_members(archive)
    install.parent.mkdir(parents=True, exist_ok=True)
    temporary = Path(tempfile.mkdtemp(prefix="verus-install-", dir=install.parent))
    try:
        with zipfile.ZipFile(archive) as source:
            for info in members:
                target = temporary / PurePosixPath(info.filename)
                if info.is_dir():
                    target.mkdir(parents=True, exist_ok=True)
                    continue
                target.parent.mkdir(parents=True, exist_ok=True)
                with source.open(info) as input_stream, target.open("wb") as output:
                    shutil.copyfileobj(input_stream, output)
                target.chmod(info.external_attr >> 16 & 0o777 or 0o644)
        candidates = [path for path in temporary.rglob("verus") if path.is_file()]
        if len(candidates) != 1:
            fail("Verus archive must contain exactly one verus executable")
        staged = candidates[0].parent
        if install.exists():
            shutil.rmtree(install)
        os.replace(staged, install)
    finally:
        if temporary.exists():
            shutil.rmtree(temporary)
    print(json.dumps({"phase": "extract", "version": declaration["version"],
                      "path": str(install)}))


def main():
    if len(sys.argv) != 4:
        fail("usage: verus_bootstrap.py HOST_ROOT CONFIG_PATH PHASE")
    root = Path(sys.argv[1]).resolve()
    config_path = Path(sys.argv[2])
    if config_path.is_absolute() or ".." in config_path.parts or config_path.parts[:2] != (".chrono-harness", "ci"):
        fail("invalid Verus bootstrap config path")
    phase = sys.argv[3]
    if phase == "download":
        download(root, config_path)
    elif phase == "verify":
        verify(root, config_path)
    elif phase == "extract":
        extract(root, config_path)
    else:
        fail("unknown Verus bootstrap phase")


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, json.JSONDecodeError, urllib.error.URLError, zipfile.BadZipFile) as error:
        print("E_VERUS_BOOTSTRAP:", error, file=sys.stderr)
        sys.exit(1)
