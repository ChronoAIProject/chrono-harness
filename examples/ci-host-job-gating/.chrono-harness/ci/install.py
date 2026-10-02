"""Install the host's explicit existing tool distribution."""
from pathlib import Path
import shutil
import sys
root, distribution = (Path(value).resolve() for value in sys.argv[1:])
destination = root / ".chrono-harness/bin"
destination.mkdir(parents=True, exist_ok=True)
for name in ("chrono-harness", "chrono-judge-ci", "chrono-ci", "chrono-worktree"):
    shutil.copy2(distribution / name, destination / name)
