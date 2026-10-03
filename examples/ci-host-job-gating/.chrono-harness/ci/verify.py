"""Exercise the registered generator's actual verification command."""
import subprocess
subprocess.run([".chrono-harness/bin/chrono-ci", "verify", "--host-root", ".", "--config", ".chrono-harness/ci/units.json"], check=True)
