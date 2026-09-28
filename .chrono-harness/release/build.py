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
    for project in projects['projects'] + projects['scripts']:
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
    if os.path.lexists(output):
        raise ValueError('release output already exists')
    env = dict(os.environ, RUSTUP_TOOLCHAIN=cfg['rust_toolchain'], CARGO_PROFILE_RELEASE_STRIP='symbols')
    report = {
        'schema': 'chrono-native-build/v3', 'status': 'failed',
        'system': platform.system(), 'machine': platform.machine(),
        'source_commit': None, 'versions': {}, 'manifests': cfg['manifests'],
        'verification_operations': cfg['verification_operations'],
        'processes': [], 'failure': None,
    }
    phase = 'source-identity'

    def run(label, argv, operation=None):
        nonlocal phase
        phase = label
        process = subprocess.run(argv, cwd=root, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        report['processes'].append({
            'phase': phase, 'operation': operation, 'argv': argv, 'cwd': str(root),
            'exit_code': process.returncode,
            'stdout_bytes': list(process.stdout), 'stderr_bytes': list(process.stderr),
            'stdout_sha256': hashlib.sha256(process.stdout).hexdigest(),
            'stderr_sha256': hashlib.sha256(process.stderr).hexdigest(),
        })
        sys.stdout.buffer.write(process.stdout)
        sys.stdout.buffer.flush()
        sys.stderr.buffer.write(process.stderr)
        sys.stderr.buffer.flush()
        if process.returncode != 0:
            raise subprocess.CalledProcessError(process.returncode, argv)
        return process.stdout

    exit_code = 0
    try:
        source = run('source-identity', [tools['git'], 'rev-parse', 'HEAD']).decode('utf-8').strip()
        if len(source) not in (40, 64) or any(c not in '0123456789abcdef' for c in source):
            raise ValueError('invalid source commit observation')
        report['source_commit'] = source
        run('toolchain-install', [tools['rustup'], 'toolchain', 'install', cfg['rust_toolchain'], '--profile', 'minimal'])
        for tool in ['cargo', 'rustc']:
            report['versions'][tool] = run('compiler-version', [tools[tool], '--version']).decode('utf-8').strip()
        if not report['versions']['rustc'].startswith('rustc ' + cfg['rust_toolchain'] + ' '):
            raise ValueError('unexpected compiler')
        for manifest in cfg['manifests']:
            run('build', [tools['cargo'], 'build', '--release', '--locked', '--manifest-path', manifest])
        for operation, _tool, argv in operations:
            run('verification', argv, operation)
        run('package', [str(root / 'crates/distribution/target/release/chrono-distribution'), 'pack', '--root', str(root), '--plan', str(root / '.chrono-harness/release/plan.json'), '--output', str(output)])
        report['status'] = 'passed'
    except (subprocess.CalledProcessError, ValueError, OSError) as error:
        actual_exit = error.returncode if isinstance(error, subprocess.CalledProcessError) else None
        report['failure'] = {'phase': phase, 'message': str(error), 'exit_code': actual_exit}
        exit_code = (actual_exit if actual_exit > 0 else 128 - actual_exit) if actual_exit is not None else 2
        print(str(error), file=sys.stderr)
    try:
        output.mkdir(parents=True, exist_ok=True)
        with (output / 'build.json').open('x') as stream:
            stream.write(json.dumps(report, indent=2) + '\n')
    except OSError as error:
        print('cannot retain native build evidence: ' + str(error), file=sys.stderr)
        # Reporting cannot replace an already observed child failure with success.
        return exit_code or 2
    return exit_code


if __name__ == '__main__':
    try:
        if len(sys.argv) != 3:
            raise ValueError('usage: build.py ROOT ABSENT_OUTPUT')
        sys.exit(main(Path(sys.argv[1]).resolve(), Path(sys.argv[2]).absolute()))
    except (ValueError, OSError) as error:
        print(str(error), file=sys.stderr)
        sys.exit(2)
