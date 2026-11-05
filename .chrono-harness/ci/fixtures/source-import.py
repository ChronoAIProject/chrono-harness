"""Import one real source with bytecode enabled at interpreter entry."""
import runpy
import sys

sys.dont_write_bytecode = False
sys.pycache_prefix = None
runpy.run_path(sys.argv[1], run_name="consumer")
