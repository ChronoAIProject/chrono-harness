"""Provide the declared Cargo/rustup interface without changing an installed SDK."""
import json
import os
from pathlib import Path
import sys

root = Path(__file__).parent
name, args = Path(sys.argv[0]).name, sys.argv[1:]
record = root / "calls.json"
calls = json.loads(record.read_text()) if record.exists() else []
calls.append([name, *args])
record.write_text(json.dumps(calls))
if name == "rustup":
    if args == ["run", "1.95.0", "cargo", "--version"]:
        sys.exit(0 if (root / "cargo-ready").exists() else 1)
    if args == ["run", "1.95.0", "rustfmt", "--version"]:
        sys.exit(0 if (root / "fmt-ready").exists() else 1)
    if args == ["toolchain", "install", "1.95.0", "--profile", "minimal", "--component", "rustfmt"]:
        (root / "cargo-ready").touch()
        (root / "fmt-ready").touch()
    elif args == ["component", "add", "--toolchain", "1.95.0", "rustfmt"]:
        if (root / "fail-component").exists():
            sys.exit(8)
        (root / "fmt-ready").touch()
    else:
        sys.exit(99)
elif name in ["cargo", "rustc"] and args == ["--version"] and os.environ.get("RUSTUP_TOOLCHAIN") == "1.95.0":
    print(name + " 1.95.0 (fixture)")
else:
    sys.exit(99)
