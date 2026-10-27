#!/usr/bin/env python3
"""Select explicit Python runtime components and native configuration locations."""
import os
import sys
import shutil
import sysconfig

sys.dont_write_bytecode = True

def absolute(path):
    if not path or not os.path.isabs(path):
        raise ValueError("runtime configuration requires an absolute path")
    return path


def location(arguments):
    if arguments == ["executable"]:
        return os.path.realpath(absolute(sys.executable))
    if arguments in (["root"], ["platstdlib"]):
        # base_prefix may be /usr: it owns neither /usr nor its other runtimes.
        key = "stdlib" if arguments == ["root"] else "platstdlib"
        return absolute(sysconfig.get_path(key))
    if arguments == ["extensions"]:
        return absolute(sysconfig.get_config_var("DESTSHARED"))
    if arguments == ["shared-library"]:
        framework = sysconfig.get_config_var("PYTHONFRAMEWORK")
        if framework:
            return os.path.join(
                absolute(sysconfig.get_config_var("PYTHONFRAMEWORKPREFIX")),
                framework + ".framework", "Versions",
                sysconfig.get_config_var("VERSION"), framework,
            )
        if sysconfig.get_config_var("Py_ENABLE_SHARED"):
            library = sysconfig.get_config_var("LDLIBRARY")
            if not library or os.path.basename(library) != library:
                raise ValueError("invalid Python shared library name")
            return os.path.join(absolute(sysconfig.get_config_var("LIBDIR")), library)
        # A static interpreter carries its runtime in the separately bound image.
        return os.path.realpath(absolute(sys.executable))
    if arguments == ["home"]:
        return absolute(os.environ["HOME"])
    if arguments == ["home-parent"]:
        return os.path.dirname(absolute(os.environ["HOME"]))
    if arguments == ["cargo-home"]:
        return os.environ.get("CARGO_HOME") or os.path.join(os.environ["HOME"], ".cargo")
    if arguments == ["rustup-home"]:
        return os.environ.get("RUSTUP_HOME") or os.path.join(os.environ["HOME"], ".rustup")
    if len(arguments) == 2 and arguments[0] == "tool":
        selected = shutil.which(arguments[1])
        if selected is None:
            raise ValueError("missing selected tool: " + arguments[1])
        return os.path.realpath(selected)
    raise ValueError("usage: python_probe.py executable|root|platstdlib|extensions|shared-library|"
                     "home|home-parent|cargo-home|rustup-home|tool NAME")


if __name__ == "__main__":
    try:
        print(location(sys.argv[1:]))
    except (ValueError, KeyError, TypeError) as error:
        sys.exit(str(error))
