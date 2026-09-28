#!/usr/bin/env python3
"""The product's explicit native release recipe; hosts never run this build."""
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import stat
import subprocess
import tempfile
import sys


def path(root, value):
    if not isinstance(value, str) or not value or value.startswith('/') or any(
        part in ('', '.', '..') for part in value.split('/')
    ):
        raise ValueError('expected a literal root-relative path')
    return root / value


def plain_path(root, value):
    result = path(root, value)
    current = root
    for part in result.relative_to(root).parts:
        current = current / part
        if current.is_symlink():
            raise ValueError('symlink in release consumer path: ' + value)
    return result


def overlaps(a, b):
    return a == b or a.startswith(b + '/') or b.startswith(a + '/')


def staging_plan(root, cfg):
    if cfg['schema'] == 'chrono-release-build/v2':
        if 'consumer_staging' in cfg or 'rust_components' in cfg:
            raise ValueError('consumer_staging and rust_components require release build v3')
        return None
    components = cfg['rust_components']
    if not isinstance(components, list) or any(not isinstance(c, str) or not c or '\0' in c for c in components) or len(set(components)) != len(components):
        raise ValueError('rust_components must list unique nonempty component names')
    declaration = cfg['consumer_staging']
    plan = json.loads(path(root, declaration['release_plan']).read_text())
    if plan['schema'] != 'chrono-release-plan/v1' or not plan['assets']:
        raise ValueError('unsupported or empty consumer release plan')
    sources = list(plan['assets'].values())
    for source in sources:
        plain_path(root, source)
    host = json.loads(path(root, declaration['host_config']).read_text())
    artifacts = host['artifacts']
    bindings = declaration['bindings']
    if not isinstance(bindings, list) or not bindings:
        raise ValueError('consumer bindings must be nonempty')
    rows, assets, destinations = [], set(), []
    for binding in bindings:
        asset = binding['asset']
        if asset not in plan['assets'] or asset in assets:
            raise ValueError('unknown or duplicate consumer asset: ' + asset)
        assets.add(asset)
        targets = binding['destinations']
        if not isinstance(targets, list) or not targets:
            raise ValueError('consumer destinations must be nonempty')
        for destination in targets:
            plain_path(root, destination)
            declarations = [a for a in artifacts if (
                destination.startswith(a['path']) if a['path'].endswith('/')
                else destination == a['path']
            )]
            if len(declarations) != 1 or declarations[0]['tracked'] is not False:
                raise ValueError('consumer destination needs one untracked artifact: ' + destination)
            path(root, declarations[0]['path'].removesuffix('/'))
            if any(overlaps(destination, other) for other in destinations + sources):
                raise ValueError('overlapping consumer destination: ' + destination)
            destinations.append(destination)
            rows.append({'asset': asset, 'source': plan['assets'][asset], 'destination': destination})
    return {'release_plan': declaration['release_plan'], 'host_config': declaration['host_config'], 'bindings': rows}


def identity(root, name):
    try:
        file = plain_path(root, name)
        info = file.stat()
        if not stat.S_ISREG(info.st_mode) or not info.st_mode & 0o111:
            raise ValueError('not a regular executable')
        raw = file.read_bytes()
        return {'path': name, 'sha256': hashlib.sha256(raw).hexdigest(),
                'size': len(raw), 'mode': stat.S_IMODE(info.st_mode), 'error': None}
    except (ValueError, OSError) as error:
        return {'path': name, 'sha256': None, 'size': None, 'mode': None, 'error': str(error)}


def same_identity(a, b):
    return a['error'] is None and b['error'] is None and all(
        a[k] == b[k] for k in ['sha256', 'size', 'mode'])


def stage(root, staging):
    # Destination declarations and Git tracking are checked before build effects.
    # Recheck filesystem paths at the actual copy; replace only the explicit artifact.
    for row in staging['bindings']:
        observed = identity(root, row['source'])
        row['expected'] = observed
        if observed['error'] is not None:
            raise ValueError('unavailable release consumer source: ' + row['source'])
        source = plain_path(root, row['source'])
        destination = plain_path(root, row['destination'])
        destination.parent.mkdir(parents=True, exist_ok=True)
        with tempfile.NamedTemporaryFile(dir=destination.parent, delete=False) as temporary:
            temp = Path(temporary.name)
        try:
            shutil.copy2(source, temp)
            os.replace(temp, destination)
        finally:
            if temp.exists():
                temp.unlink()


def observe(root, staging, phase, operation=None):
    files, matches = {}, True
    for row in staging['bindings']:
        for name in [row['source'], row['destination']]:
            if name not in files:
                files[name] = identity(root, name)
            expected = row.get('expected')
            matches = matches and expected is not None and same_identity(expected, files[name])
    staging['observations'].append({'phase': phase, 'operation': operation,
                                    'matches': matches, 'files': list(files.values())})
    return matches


def preflight(root):
    cfg = json.loads((root / '.chrono-harness/release/build.json').read_text())
    if cfg['schema'] not in ('chrono-release-build/v2', 'chrono-release-build/v3'):
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
    return cfg, tools, operations, staging_plan(root, cfg)


def main(root, output):
    cfg, tools, operations, staging = preflight(root)
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
    if staging is not None:
        report['schema'] = 'chrono-native-build/v4'
        report['consumer_staging'] = staging
        staging['observations'] = []
    phase = 'source-identity'

    def checked_snapshot(label, operation=None):
        nonlocal phase
        phase = label
        if not observe(root, staging, label, operation):
            raise ValueError('release consumer identity changed or unavailable: ' + label)

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
        if staging is not None:
            tracked = run('consumer-tracking', [tools['git'], '--literal-pathspecs', 'ls-files', '-z', '--',
                          *[row['destination'] for row in staging['bindings']]])
            if tracked:
                raise ValueError('release consumer destination is tracked by Git')
        component_args = [arg for component in cfg.get('rust_components', []) for arg in ['--component', component]]
        run('toolchain-install', [tools['rustup'], 'toolchain', 'install', cfg['rust_toolchain'], '--profile', 'minimal', *component_args])
        for tool in ['cargo', 'rustc']:
            report['versions'][tool] = run('compiler-version', [tools[tool], '--version']).decode('utf-8').strip()
        if not report['versions']['rustc'].startswith('rustc ' + cfg['rust_toolchain'] + ' '):
            raise ValueError('unexpected compiler')
        for manifest in cfg['manifests']:
            run('build', [tools['cargo'], 'build', '--release', '--locked', '--manifest-path', manifest])
        if staging is not None:
            phase = 'consumer-staging'
            try:
                stage(root, staging)
            finally:
                observe(root, staging, 'staged')
            checked_snapshot('before-verification')
        for operation, _tool, argv in operations:
            if staging is not None:
                checked_snapshot('before-verification', operation)
            try:
                run('verification', argv, operation)
            finally:
                # Observe even when the child failed; never mask its original exit.
                intact = staging is None or observe(root, staging, 'after-verification', operation)
            if not intact:
                phase = 'after-verification'
                raise ValueError('release consumer identity changed after ' + operation)
        if staging is not None:
            checked_snapshot('before-package')
        release_plan = staging['release_plan'] if staging is not None else '.chrono-harness/release/plan.json'
        try:
            run('package', [str(root / 'crates/distribution/target/release/chrono-distribution'), 'pack', '--root', str(root), '--plan', str(root / release_plan), '--output', str(output)])
        finally:
            intact = staging is None or observe(root, staging, 'after-package')
        if not intact:
            phase = 'after-package'
            raise ValueError('release consumer identity changed after package')
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
