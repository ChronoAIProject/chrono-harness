"""Real Python child IO used by the native adapter's dedicated Python tests."""
import sys
import time


mode = sys.argv[1]
if mode == "echo":
    sys.stdout.buffer.write(b"\x00\xff" + sys.stdin.buffer.read())
    sys.stderr.buffer.write(b"original stderr")
elif mode == "exit":
    sys.stderr.buffer.write(b"real failure")
    sys.exit(7)
elif mode == "overflow":
    sys.stdout.buffer.write(b"x" * 1000)
elif mode == "timeout":
    time.sleep(10)
else:
    raise ValueError("unknown native adapter child mode: " + mode)
