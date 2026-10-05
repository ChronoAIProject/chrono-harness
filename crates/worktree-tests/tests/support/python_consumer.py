"""Invoke real Python consumers through their existing interfaces."""
import importlib.util
from pathlib import Path
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
else:
    raise ValueError("unknown registered Python consumer: " + route)
