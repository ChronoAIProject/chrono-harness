"""Execute one explicit build fixture operation inside the temporary host."""
from pathlib import Path
import hashlib
import json
import os
import shutil
import subprocess
import sys
import time

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from process_fds import inherited_fds

mode = sys.argv[1]
if mode == "copy-executable":
    destination = Path(sys.argv[3])
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(sys.argv[2], destination)
    sys.exit(0)
if mode == "acquisition":
    # A Python interface fixture, not a native observation or admission receipt.
    output = Path(sys.argv[2])
    output.parent.mkdir(parents=True, exist_ok=True)
    raw = json.dumps({"installed_before": Path(sys.argv[3]).exists(),
                      "environment": {key: os.environ.get(key) for key in
                                      ("RUSTUP_HOME", "CARGO_HOME", "RUSTC")}}).encode()
    with output.open("xb") as stream:
        stream.write(raw)
    print(json.dumps({"fixture_original": {"path": output.as_posix(),
                                          "sha256": hashlib.sha256(raw).hexdigest()}}))
    sys.exit(0)
if mode == "runner-consumer":
    outcome = sys.argv[2]
    Path("runner-consumed").write_bytes(outcome.encode())
    print("test result: ok. 1 passed; 0 failed" if outcome == "success" else "runner consumer " + outcome)
    print("runner consumer stderr", file=sys.stderr)
    sys.exit(7 if outcome == "failure" else 0)
if mode == "custody":
    outcome = sys.argv[2]
    ownership_fds = inherited_fds()
    if not ownership_fds:
        raise AssertionError("custody assertions require inherited owner protection")
    verified_payloads = []
    for name in ("crates/runner/target", ".chrono-harness/startup"):
        path = Path(name)
        path.mkdir(parents=True)
        (path / "payload").write_bytes(b"reconstructible output")
        if (path / "payload").read_bytes() != b"reconstructible output":
            raise AssertionError("custody payload differs: " + name)
        verified_payloads.append((path / "payload").as_posix())
    state = Path(".chrono-harness/state")
    state.mkdir(exist_ok=True)
    (state / "custody-original").write_bytes(b"original evidence")
    # The parent consumes this original after use returns; payload reads must
    # finish here, while the inherited owner lease is still held, on every exit.
    with (state / "custody-assertions.json").open("x") as output:
        json.dump({"ownership_fds": list(ownership_fds),
                   "python_executable": sys.executable,
                   "verified_payloads": verified_payloads}, output)
    print("custody stdout", flush=True)
    print("custody stderr", file=sys.stderr, flush=True)
    (state / "custody-ready").touch()
    if outcome == "timeout":
        time.sleep(30)
    if outcome == "hold":
        until = time.monotonic() + 15
        while not (state / "custody-release").exists() and time.monotonic() < until:
            time.sleep(0.02)
    sys.exit(7 if outcome == "failure" else 0)
if mode == "timeout":
    sys.stdout.buffer.write(b"timeout stdout\x00\xff\n")
    sys.stdout.buffer.flush()
    sys.stderr.buffer.write(b"timeout stderr\xfe\n")
    sys.stderr.buffer.flush()
    time.sleep(30)
    sys.exit(0)
if mode == "fail":
    sys.stdout.buffer.write(b"failure stdout\x00\xff\n")
    sys.stderr.buffer.write(b"failure stderr\xfe\n")
    sys.exit(int(sys.argv[2]))
if mode == "environment":
    Path(sys.argv[2]).write_text(json.dumps({name: os.environ.get(name) for name in sys.argv[3:]}))
    sys.exit(0)
if mode == "mutate":
    Path(os.environ["BOUND_CARGO"]).write_bytes(b"replaced executable")
    sys.exit(0)
if mode == "move-head":
    Path("product-source").write_text("moved")
    for args in [["add", "product-source"], ["commit", "-qm", "moved"]]:
        subprocess.run(["git", *args], check=True, pass_fds=inherited_fds())
elif mode == "lose-git":
    Path(".git").rename(".saved-git")
elif mode != "write":
    sys.exit(99)
Path(sys.argv[2]).write_bytes(sys.argv[3].encode("utf-8"))
