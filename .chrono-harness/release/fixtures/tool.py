#!/usr/bin/env python3
"""Minimal external compiler/package interfaces for the release recipe tests."""
import hashlib
import json
from pathlib import Path
import platform
import shutil
import subprocess
import sys


def main():
    executable = Path(sys.argv[0]).resolve()
    args = sys.argv[1:]
    if executable.name in ('cargo', 'rustc') and args == ['--version']:
        print(executable.name + ' 1.94.0 (fixture)')
    elif executable.name == 'rustup' and args[:1] == ['which']:
        print(executable.parent / args[-1])
    elif executable.name == 'rustup' and args[:2] == ['toolchain', 'install']:
        pass
    elif executable.name == 'cargo' and args[:1] == ['build']:
        name = Path(args[args.index('--manifest-path') + 1]).parent.name
        destination = Path('out') / name
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(executable, destination)
    elif args[:1] == ['verify']:
        print('verification ran without unpublished assets')
    elif args[:1] == ['pack']:
        root = Path(args[args.index('--root') + 1])
        output = Path(args[args.index('--output') + 1])
        plan = json.loads(Path(args[args.index('--plan') + 1]).read_text())
        output.mkdir()
        system = {'Darwin': 'macos', 'Linux': 'linux'}[platform.system()]
        machine = {'arm64': 'aarch64', 'aarch64': 'aarch64', 'x86_64': 'x86_64'}[platform.machine()]
        target = system + '-' + machine
        assets = {}
        for name, source in plan['assets'].items():
            destination = output / (name + '-' + target)
            shutil.copy2(root / source, destination)
            raw = destination.read_bytes()
            assets[name] = {'file': destination.name, 'sha256': hashlib.sha256(raw).hexdigest(), 'size': len(raw)}
        def git(value):
            return subprocess.check_output(['git', 'rev-parse', value], cwd=root).decode().strip()
        release = {'schema': 'chrono-release/v1', 'version': plan['version'],
                   'source_commit': git('HEAD'), 'source_tree': git('HEAD^{tree}'),
                   'platforms': {target: assets}}
        (output / 'release.json').write_text(json.dumps(release))
    else:
        raise ValueError('unsupported fixture invocation: ' + repr(args))


if __name__ == '__main__':
    main()
