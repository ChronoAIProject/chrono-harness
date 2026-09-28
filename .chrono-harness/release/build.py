#!/usr/bin/env python3
"""The product's explicit native release recipe; hosts never run this build."""
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys


def path(root, value):
    if not isinstance(value, str) or not value or value.startswith('/') or any(
        part in ('', '.', '..') for part in value.split('/')
    ):
        raise ValueError('expected a literal root-relative path')
    return root / value


def preflight(root):
    cfg = json.loads((root / '.chrono-harness/release/build.json').read_text())
    if cfg['schema'] != 'chrono-release-build/v2':
        raise ValueError('unsupported release build schema')
    manifests = cfg['manifests']
    if not isinstance(manifests, list) or not manifests or len(set(manifests)) != len(manifests):
        raise ValueError('release manifests must be nonempty and unique')
    for manifest in manifests:
        if not path(root, manifest).is_file():
            raise ValueError('missing release manifest: ' + manifest)
    tools = {}
    for name, program in cfg['tools'].items():
        if not isinstance(program, str) or not program:
            raise ValueError('invalid tool binding: ' + name)
        executable = program if Path(program).is_absolute() or '/' not in program else str(path(root, program))
        resolved = shutil.which(executable)
        if resolved is None:
            raise ValueError('unavailable tool: ' + name)
        tools[name] = os.path.abspath(resolved)
    for name in ['rustup', 'cargo', 'rustc', 'git']:
        if name not in tools:
            raise ValueError('missing tool binding: ' + name)
    projects = json.loads(path(root, cfg['projects']).read_text())
    actions = {}
    for project in projects['projects']:
        for action in project['actions'].values():
            operation = action['operation']
            if operation in actions:
                raise ValueError('ambiguous registered operation: ' + operation)
            actions[operation] = action
    selected = cfg['verification_operations']
    if not isinstance(selected, list) or not selected or len(set(selected)) != len(selected):
        raise ValueError('verification operations must be nonempty and unique')
    operations = []
    for operation in selected:
        if operation not in actions:
            raise ValueError('unknown registered operation: ' + operation)
        action = actions[operation]
        if action['tool'] not in tools:
            raise ValueError('missing tool binding: ' + action['tool'])
        argv = action['argv']
        if not isinstance(argv, list) or any(not isinstance(a, str) or '\0' in a for a in argv):
            raise ValueError('invalid operation argv: ' + operation)
        operations.append((operation, action['tool'], [tools[action['tool']], *argv]))
    return cfg, tools, operations


def main(root, output):
    cfg, tools, operations = preflight(root)
    env = dict(os.environ, RUSTUP_TOOLCHAIN=cfg['rust_toolchain'], CARGO_PROFILE_RELEASE_STRIP='symbols')
    subprocess.run([tools['rustup'], 'toolchain', 'install', cfg['rust_toolchain'], '--profile', 'minimal'], cwd=root, env=env, check=True)
    versions = {tool: subprocess.check_output([tools[tool], '--version'], cwd=root, env=env, text=True).strip() for tool in ['cargo', 'rustc']}
    if not versions['rustc'].startswith('rustc ' + cfg['rust_toolchain'] + ' '):
        raise ValueError('unexpected compiler')
    for manifest in cfg['manifests']:
        subprocess.run([tools['cargo'], 'build', '--release', '--locked', '--manifest-path', manifest], cwd=root, env=env, check=True)
    verified = []
    for operation, tool, argv in operations:
        process = subprocess.run(argv, cwd=root, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        sys.stdout.buffer.write(process.stdout)
        sys.stdout.buffer.flush()
        sys.stderr.buffer.write(process.stderr)
        sys.stderr.buffer.flush()
        if process.returncode != 0:
            raise subprocess.CalledProcessError(process.returncode, argv)
        verified.append({
            'operation': operation, 'tool': tool, 'argv': argv, 'cwd': str(root),
            'exit_code': process.returncode,
            'stdout_bytes': list(process.stdout), 'stderr_bytes': list(process.stderr),
            'stdout_sha256': hashlib.sha256(process.stdout).hexdigest(),
            'stderr_sha256': hashlib.sha256(process.stderr).hexdigest(),
        })
    subprocess.run([str(root / 'crates/distribution/target/release/chrono-distribution'), 'pack', '--root', str(root), '--plan', str(root / '.chrono-harness/release/plan.json'), '--output', str(output)], cwd=root, env=env, check=True)
    (output / 'build.json').write_text(json.dumps({
        'schema': 'chrono-native-build/v2', 'system': platform.system(), 'machine': platform.machine(),
        'versions': versions, 'source_commit': subprocess.check_output([tools['git'], 'rev-parse', 'HEAD'], cwd=root, env=env, text=True).strip(),
        'manifests': cfg['manifests'], 'verification': verified,
    }, indent=2) + '\n')


if __name__ == '__main__':
    try:
        main(*(Path(p).resolve() for p in sys.argv[1:]))
    except subprocess.CalledProcessError as error:
        print(str(error), file=sys.stderr)
        sys.exit(error.returncode if error.returncode > 0 else 128 - error.returncode)
