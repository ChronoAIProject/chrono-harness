"""Execute one explicit build fixture operation inside the temporary host."""
from pathlib import Path
import json
import os
import subprocess
import sys

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from process_fds import inherited_fds

mode = sys.argv[1]
if mode == "fail":
    sys.exit(int(sys.argv[2]))
if mode == "environment":
    Path(sys.argv[2]).write_text(json.dumps({name: os.environ.get(name) for name in sys.argv[3:]}))
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
