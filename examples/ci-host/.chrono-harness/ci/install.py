#!/usr/bin/env python3
"""Copy an explicit, previously installed tool distribution; no source checkout needed."""
from pathlib import Path
import shutil
import sys

if len(sys.argv) != 3:
    raise SystemExit("usage: install.py HOST_ROOT INSTALLED_TOOLS_DIRECTORY")
root, distribution = (Path(value).resolve() for value in sys.argv[1:])
destination = root / ".chrono-harness/bin"
destination.mkdir(parents=True, exist_ok=True)
for name in ("chrono-harness", "chrono-judge-ci", "chrono-ci"):
    shutil.copy2(distribution / name, destination / name)
