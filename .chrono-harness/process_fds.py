"""Host-owned forwarding of explicit process capabilities through Python subprocesses."""
import os


def inherited_fds():
    raw = os.environ.get("CHRONO_PROCESS_FDS")
    if raw is None:
        return ()
    fds = tuple(int(value) for value in raw.split(","))
    if not fds or len(set(fds)) != len(fds) or any(fd < 3 for fd in fds):
        raise ValueError("invalid process ownership descriptors")
    for fd in fds:
        os.fstat(fd)
        os.set_inheritable(fd, False)
    return fds
