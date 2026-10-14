#!/usr/bin/env python3
"""Expose the selected Python/Cargo runtime locations to the input observer."""
import os
import sys
import shutil

sys.dont_write_bytecode = True

if sys.argv[1:] == ["executable"]:
    print(os.path.realpath(sys.executable))
elif sys.argv[1:] == ["root"]:
    print(sys.base_prefix)
elif sys.argv[1:] == ["cargo-home"]:
    print(os.environ.get("CARGO_HOME") or os.path.join(os.environ["HOME"], ".cargo"))
elif len(sys.argv) == 3 and sys.argv[1] == "tool":
    selected = shutil.which(sys.argv[2])
    if selected is None:
        sys.exit("missing selected tool: " + sys.argv[2])
    print(os.path.realpath(selected))
else:
    sys.exit("usage: python_probe.py executable|root|cargo-home|tool NAME")
