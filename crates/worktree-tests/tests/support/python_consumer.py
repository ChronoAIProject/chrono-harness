"""Invoke real Python consumers through their existing interfaces."""
import importlib.util
import json
from pathlib import Path
import subprocess
import sys

source = Path(sys.argv[1])
route = sys.argv[2]
spec = importlib.util.spec_from_file_location("consumer", source)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
root = Path.cwd()

if route == "bootstrap":
    sys.argv = [str(source), str(root), ".chrono-harness/ci/python-bootstrap.json"]
    module.main()
elif route == "workflow-inventory":
    module.validate(root, Path(".chrono-harness/state/list-config.json"))
elif route == "scoped-v1-tests":
    module.blob("opaque-declared-input")
elif route == "detached":
    config = json.loads(Path(".chrono-harness/state/python-consumer.json").read_text())
    subprocess.Popen(
        [config["child"], "detached-holder"],
        pass_fds=module.inherited_fds(), stdin=subprocess.DEVNULL,
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, start_new_session=True,
    )
else:
    raise ValueError("unknown registered Python consumer: " + route)
