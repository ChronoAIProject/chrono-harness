"""Execute one explicit build fixture operation inside the temporary host."""
from pathlib import Path
import json
import os
import subprocess
import sys
import time

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from process_fds import inherited_fds

mode = sys.argv[1]
if mode == "custody":
    outcome = sys.argv[2]
    for name in ("crates/runner/target", ".chrono-harness/startup"):
        path = Path(name)
        path.mkdir(parents=True)
        (path / "payload").write_bytes(b"reconstructible output")
    state = Path(".chrono-harness/state")
    state.mkdir(exist_ok=True)
    (state / "custody-original").write_bytes(b"original evidence")
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
