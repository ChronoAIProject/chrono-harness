#!/usr/bin/env python3
"""The product's explicit native release recipe; hosts never run this build."""
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import stat
import signal
import subprocess
import tempfile
import sys
import tarfile
import time
import uuid
from concurrent.futures import ThreadPoolExecutor, as_completed

EVIDENCE_BOUND = 64 * 1024 * 1024


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
        digest = hashlib.sha256()
        size = 0
        with file.open('rb') as stream:
            for block in iter(lambda: stream.read(1024 * 1024), b''):
                digest.update(block)
                size += len(block)
        return {'path': name, 'sha256': digest.hexdigest(),
                'size': size, 'mode': stat.S_IMODE(info.st_mode), 'error': None}
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


def retain_failure_evidence(output, phase, stdout, stderr):
    """Carry an explicitly retained native child receipt into the artifact."""
    marker = b'original native publication process evidence: '
    source = None
    for stream in (stdout, stderr):
        for line in stream.splitlines():
            if marker in line:
                source = Path(line.split(marker, 1)[1].decode('utf-8', 'strict').strip())
                break
        if source is not None:
            break
    if source is None:
        return None
    temp_root = Path(tempfile.gettempdir()).resolve()
    try:
        resolved = source.resolve(strict=True)
    except OSError:
        return {'status': 'unavailable', 'reason': 'retained directory disappeared'}
    if (not source.is_absolute() or source.is_symlink() or
            not resolved.name.startswith('chrono-native-publication-failure-') or
            resolved.parent != temp_root or not resolved.is_dir()):
        return {'status': 'unavailable', 'reason': 'retained directory is outside the native fixture boundary'}
    source = resolved
    destination = output / 'failure-evidence' / phase
    destination.mkdir(parents=True, exist_ok=True)
    files = [source / 'stdout', source / 'stderr', source / 'process.json']
    preparation = source / 'state' / 'collection' / 'preparation'
    if preparation.is_dir():
        files.extend(p for p in sorted(preparation.rglob('*')) if p.is_file())
    retained, used = [], 0
    for path in files:
        relative = path.relative_to(source)
        if path.is_symlink() or path.resolve() != path or not path.is_file():
            retained.append({'source': relative.as_posix(), 'status': 'omitted',
                             'reason': 'not a regular retained file'})
            continue
        size = path.stat().st_size
        if size > EVIDENCE_BOUND - used:
            retained.append({'source': relative.as_posix(), 'status': 'omitted',
                             'reason': 'bounded evidence limit', 'bytes': size})
            continue
        with path.open('rb') as stream:
            raw = stream.read(EVIDENCE_BOUND - used + 1)
        if len(raw) != size:
            retained.append({'source': relative.as_posix(), 'status': 'omitted',
                             'reason': 'source changed during retention', 'bytes': len(raw)})
            continue
        target = destination / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(raw)
        used += len(raw)
        retained.append({'source': relative.as_posix(),
                         'path': target.relative_to(output).as_posix(),
                         'status': 'retained', 'bytes': len(raw),
                         'sha256': hashlib.sha256(raw).hexdigest()})
    return {'status': 'retained' if all(row['status'] == 'retained' for row in retained) else 'partial',
            'source': source.name, 'files': retained,
            'bytes': used, 'bound_bytes': EVIDENCE_BOUND}


def preflight(root, required_tools=None, required_manifests=None):
    cfg = json_file(root / '.chrono-harness/release/build.json')
    if cfg['schema'] not in ('chrono-release-build/v2', 'chrono-release-build/v3', 'chrono-release-build/v4'):
        raise ValueError('unsupported release build schema')
    manifests = cfg['manifests']
    if not isinstance(manifests, list) or not manifests or len(set(manifests)) != len(manifests):
        raise ValueError('release manifests must be nonempty and unique')
    for manifest in manifests:
        declared = path(root, manifest)
        if (required_manifests is None or manifest in required_manifests) and not declared.is_file():
            raise ValueError('missing release manifest: ' + manifest)
    tools = {}
    for name, program in cfg['tools'].items():
        if not isinstance(program, str) or not program:
            raise ValueError('invalid tool binding: ' + name)
        if required_tools is not None and name not in required_tools:
            continue
        executable = program if Path(program).is_absolute() or '/' not in program else str(path(root, program))
        resolved = shutil.which(executable)
        if resolved is None:
            raise ValueError('unavailable tool: ' + name)
        tools[name] = os.path.abspath(resolved)
    for name in (['rustup', 'cargo', 'rustc', 'git'] if required_tools is None else required_tools):
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
        if action['tool'] not in cfg['tools']:
            raise ValueError('missing tool binding: ' + action['tool'])
        argv = action['argv']
        if not isinstance(argv, list) or any(not isinstance(a, str) or '\0' in a for a in argv):
            raise ValueError('invalid operation argv: ' + operation)
        operations.append((operation, action['tool'], [tools.get(action['tool'], cfg['tools'][action['tool']]), *argv]))
    return cfg, tools, operations, staging_plan(root, cfg)


def legacy_main(root, output):
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
        record = {
            'phase': phase, 'operation': operation, 'argv': argv, 'cwd': str(root),
            'exit_code': process.returncode,
            'stdout_bytes': list(process.stdout), 'stderr_bytes': list(process.stderr),
            'stdout_sha256': hashlib.sha256(process.stdout).hexdigest(),
            'stderr_sha256': hashlib.sha256(process.stderr).hexdigest(),
        }
        if process.returncode != 0:
            try:
                nested = retain_failure_evidence(output, phase, process.stdout, process.stderr)
            except (OSError, UnicodeError, ValueError) as error:
                nested = {'status': 'unavailable', 'reason': str(error)}
            if nested is not None:
                record['failure_evidence'] = nested
        report['processes'].append(record)
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


# v4 is an explicit host recipe. The Rust projection only transports its outputs.
def sha_file(file):
    digest = hashlib.sha256()
    with file.open('rb') as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b''):
            digest.update(block)
    return digest.hexdigest()


def json_file(file):
    def unique(pairs):
        result = {}
        for key, value in pairs:
            if key in result:
                raise ValueError('duplicate release receipt key: ' + key)
            result[key] = value
        return result
    return json.loads(file.read_bytes(), object_pairs_hook=unique)


def write_json(file, value):
    file.parent.mkdir(parents=True, exist_ok=True)
    with file.open('x') as stream:
        stream.write(json.dumps(value, indent=2) + '\n')


def units_plan(root, cfg, operations, staging):
    if cfg['schema'] != 'chrono-release-build/v4':
        raise ValueError('independent scopes require release build v4')
    plan = json_file(path(root, staging['release_plan']))
    units = cfg['units']
    builds, tests, ids, directories = {}, {}, set(), []
    if not isinstance(units, list) or not units:
        raise ValueError('release units must be explicit and nonempty')
    for unit in units:
        id0 = unit['id']
        if not isinstance(id0, str) or not id0 or not all(c.isascii() and (c.isalnum() or c in '_-') for c in id0) or id0 in ids:
            raise ValueError('invalid or duplicate release unit')
        ids.add(id0)
        if unit['kind'] == 'build':
            if set(unit) != {'id', 'kind', 'manifest', 'asset', 'needs', 'handoff'} or unit['needs']:
                raise ValueError('build units need one explicit manifest/asset and no dependencies')
            if unit['manifest'] not in cfg['manifests'] or unit['asset'] not in plan['assets']:
                raise ValueError('unknown build manifest or asset')
            builds[id0] = unit
        elif unit['kind'] == 'verify':
            if set(unit) != {'id', 'kind', 'operation', 'needs', 'handoff'} or unit['operation'] not in cfg['verification_operations']:
                raise ValueError('verification needs one original registered operation')
            tests[id0] = unit
        else:
            raise ValueError('unknown release unit kind')
        directory = unit['handoff']
        plain_path(root, directory)
        if any(overlaps(directory, other) for other in directories + list(plan['assets'].values()) + [r['destination'] for r in staging['bindings']]):
            raise ValueError('overlapping release handoff directory')
        directories.append(directory)
    if sorted(u['manifest'] for u in builds.values()) != sorted(cfg['manifests']) or sorted(u['asset'] for u in builds.values()) != sorted(plan['assets']):
        raise ValueError('build units must cover each manifest and asset exactly once')
    if sorted(u['operation'] for u in tests.values()) != sorted(cfg['verification_operations']):
        raise ValueError('verification units must cover each unfiltered operation exactly once')
    if set(r['asset'] for r in staging['bindings']) != set(plan['assets']):
        raise ValueError('release v4 retains staging observations for all assets')
    for unit in tests.values():
        if not isinstance(unit['needs'], list) or sorted(unit['needs']) != sorted(builds):
            raise ValueError('verification must explicitly depend on the complete build vector')
    collector = cfg['collector']
    if set(collector) != {'id', 'needs', 'package_asset', 'dependency_metadata'} or collector['id'] in ids or sorted(collector['needs']) != sorted(ids) or collector['package_asset'] not in plan['assets']:
        raise ValueError('collector needs exact build/test membership and package asset')
    plain_path(root, collector['dependency_metadata'])
    if any(overlaps(collector['dependency_metadata'], d) for d in directories):
        raise ValueError('collector metadata overlaps handoff')
    if set(cfg['unit_costs']) != ids | {collector['id']} or any(not isinstance(v, str) or not v for v in cfg['unit_costs'].values()):
        raise ValueError('release units need explicit cost registrations')
    if set(cfg['limits']) != {'unit_seconds', 'platform_seconds'} or any(type(v) is not int or v <= 0 for v in cfg['limits'].values()):
        raise ValueError('release limits must be registered positive seconds')
    if type(cfg['local_workers']) is not int or not 1 <= cfg['local_workers'] <= len(units):
        raise ValueError('local_workers must be a bounded positive concurrency')
    for platform0, bindings in cfg['native_jobs'].items():
        if set(bindings) != ids | {collector['id']} or len(set(bindings.values())) != len(bindings):
            raise ValueError('native jobs need explicit exact unit bindings')
    return plan, builds, tests


class UnitExecution:
    def __init__(self, root, output, cfg, tools, unit):
        self.root, self.output, self.cfg, self.tools = root, output, cfg, tools
        self.env = dict(os.environ, RUSTUP_TOOLCHAIN=cfg['rust_toolchain'], CARGO_PROFILE_RELEASE_STRIP='symbols')
        self.phase = 'source-identity'
        self.report = {'schema': 'chrono-release-unit/v1', 'unit': unit, 'status': 'failed',
                       'started_ns': time.time_ns(), 'finished_ns': None, 'context': None,
                       'lineage': None, 'environment': {k: self.env.get(k) for k in ['PATH', 'RUSTUP_TOOLCHAIN', 'CARGO_PROFILE_RELEASE_STRIP', 'SDKROOT', 'MACOSX_DEPLOYMENT_TARGET', 'CARGO_TARGET_DIR', 'RUSTFLAGS']}, 'processes': [], 'failure': None, 'selected': {}, 'assets': {}}

    def run(self, phase, argv, operation=None):
        self.phase = phase
        before = time.time_ns()
        process = subprocess.run(argv, cwd=self.root, env=self.env, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        record = {'phase': phase, 'operation': operation, 'argv': argv, 'cwd': str(self.root),
                  'started_ns': before, 'finished_ns': time.time_ns(), 'exit_code': process.returncode}
        prefix = 'processes/' + str(len(self.report['processes']))
        for name, raw in [('stdout', process.stdout), ('stderr', process.stderr)]:
            file = self.output / (prefix + '.' + name)
            file.parent.mkdir(parents=True, exist_ok=True)
            file.write_bytes(raw)
            record[name] = {'path': file.relative_to(self.output).as_posix(), 'size': len(raw),
                            'sha256': hashlib.sha256(raw).hexdigest()}
        if process.returncode:
            nested = retain_failure_evidence(self.output, phase, process.stdout, process.stderr)
            if nested is not None:
                record['failure_evidence'] = nested
        self.report['processes'].append(record)
        sys.stdout.buffer.write(process.stdout); sys.stdout.buffer.flush()
        sys.stderr.buffer.write(process.stderr); sys.stderr.buffer.flush()
        if process.returncode:
            raise subprocess.CalledProcessError(process.returncode, argv)
        return process.stdout

    def context(self):
        git = self.tools['git']
        source = self.run('source-identity', [git, 'rev-parse', 'HEAD']).decode().strip()
        tree = self.run('source-tree', [git, 'rev-parse', 'HEAD^{tree}']).decode().strip()
        for oid in [source, tree]:
            if len(oid) not in (40, 64) or any(c not in '0123456789abcdef' for c in oid):
                raise ValueError('invalid source identity')
        if self.run('source-clean', [git, 'status', '--porcelain', '--untracked-files=normal']):
            raise ValueError('release source must be a fixed clean candidate')
        inputs = ['.chrono-harness/release/build.py', '.chrono-harness/release/build.json',
                  self.cfg['projects'], self.cfg['consumer_staging']['release_plan'],
                  self.cfg['consumer_staging']['host_config']]
        self.report['execution_tools'] = {n: executable_identity(Path(p)) for n, p in self.tools.items()}
        self.report['context'] = {'source_commit': source, 'source_tree': tree,
                                  'system': platform.system(), 'machine': platform.machine(),
                                  'recipe': {n: sha_file(plain_path(self.root, n)) for n in inputs},
                                  'rust_toolchain': self.cfg['rust_toolchain'],
                                  'rust_components': self.cfg['rust_components']}
        run = os.environ.get('CHRONO_RELEASE_RUN')
        attempt = os.environ.get('CHRONO_RELEASE_ATTEMPT')
        job = os.environ.get('CHRONO_RELEASE_JOB')
        if not run or not attempt or not job:
            raise ValueError('missing original run/attempt/job binding')
        if not attempt.isdecimal() or int(attempt) < 1:
            raise ValueError('invalid producing attempt')
        local = os.environ.get('CHRONO_RELEASE_LOCAL') == '1'
        if not local:
            matching = [p for p, bindings in self.cfg['native_jobs'].items()
                        if bindings.get(self.report['unit']['id']) == job]
            if len(matching) != 1:
                raise ValueError('unregistered native producing job')
        raw_dependencies = os.environ.get('CHRONO_RELEASE_DEPENDENCIES')
        if raw_dependencies is None:
            raise ValueError('missing original dependency input')
        dependency_file = self.output / 'dependency-context.json'
        dependency_file.parent.mkdir(parents=True, exist_ok=True)
        raw = raw_dependencies.encode('utf-8')
        dependency_file.write_bytes(raw)
        self.report['dependency_input'] = {'path': dependency_file.name, 'size': len(raw), 'sha256': hashlib.sha256(raw).hexdigest(), 'source': 'local-outcomes' if local else 'native-needs'}
        self.report['lineage'] = {'kind': 'local' if local else 'github', 'run_id': run,
                                  'attempt': attempt, 'job': job}

    def toolchain(self):
        components = [a for c in self.cfg['rust_components'] for a in ['--component', c]]
        self.run('toolchain-install', [self.tools['rustup'], 'toolchain', 'install', self.cfg['rust_toolchain'], '--profile', 'minimal', *components])
        versions = {}
        for name in ['cargo', 'rustc']:
            versions[name] = self.run('compiler-version', [self.tools[name], '--version']).decode().strip()
        if not versions['rustc'].startswith('rustc ' + self.cfg['rust_toolchain'] + ' '):
            raise ValueError('unexpected compiler')
        active = {}
        for name in ['cargo', 'rustc']:
            actual = self.run('compiler-executable', [self.tools['rustup'], 'which', '--toolchain', self.cfg['rust_toolchain'], name]).decode().strip()
            if not Path(actual).is_absolute():
                raise ValueError('compiler executable selector returned a nonabsolute path')
            active[name] = executable_identity(Path(actual))
        self.report['toolchain'] = {'versions': versions, 'tools': {n: executable_identity(Path(p)) for n, p in self.tools.items()}, 'active_compilers': active}

    def save(self, error=None):
        finished = time.time_ns()
        if error is None and finished - self.report['started_ns'] > self.cfg['limits']['unit_seconds'] * 1000000000:
            self.phase = 'unit-span'
            error = ValueError('registered release unit span exceeded')
        if 'platform_started_ns' in self.report:
            self.report['platform_span_ns'] = finished - self.report['platform_started_ns']
            if error is None and self.report['platform_span_ns'] > self.cfg['limits']['platform_seconds'] * 1000000000:
                self.phase = 'platform-span'
                error = ValueError('registered release platform span exceeded')
        code = 0
        if error is None:
            self.report['status'] = 'passed'
        else:
            original = error.returncode if isinstance(error, subprocess.CalledProcessError) else None
            self.report['failure'] = {'phase': self.phase, 'message': str(error), 'exit_code': original}
            code = (original if original > 0 else 128 - original) if original is not None else 2
            print(str(error), file=sys.stderr)
        self.report['finished_ns'] = finished
        write_json(self.output / 'receipt.json', self.report)
        return code


def executable_identity(file):
    info = file.stat()
    if not stat.S_ISREG(info.st_mode) or not info.st_mode & 0o111:
        raise ValueError('not a regular executable: ' + str(file))
    return {'path': str(file), 'sha256': sha_file(file), 'size': info.st_size, 'mode': stat.S_IMODE(info.st_mode)}


def asset_identity(root, value):
    observed = identity(root, value)
    if observed['error'] is not None:
        raise ValueError('unavailable release asset: ' + value)
    return observed


def dependency_selection(execution, ids):
    raw = os.environ.get('CHRONO_RELEASE_DEPENDENCIES')
    if raw is None:
        raise ValueError('missing current dependency outcomes')
    selected = json.loads(raw)
    local = execution.report['lineage']['kind'] == 'local'
    bindings = {id0: id0 for id0 in ids} if local else next(
        b for b in execution.cfg['native_jobs'].values() if b.get(execution.report['unit']['id']) == execution.report['lineage']['job'])
    if set(selected) != {bindings[id0] for id0 in ids}:
        raise ValueError('current dependency membership differs from registered needs')
    result = {}
    for id0 in ids:
        row = selected[bindings[id0]]
        if not isinstance(row, dict):
            raise ValueError('invalid current dependency row: ' + id0)
        if row.get('result') != 'success':
            raise ValueError('current dependency failed/cancelled/skipped: ' + id0)
        outputs = row.get('outputs', {})
        if not isinstance(outputs, dict):
            raise ValueError('invalid selected dependency outputs: ' + id0)
        if not all(isinstance(outputs.get(k), str) and outputs[k] for k in ['artifact_id', 'run_id', 'attempt']):
            raise ValueError('missing exact selected artifact identity: ' + id0)
        if not outputs['attempt'].isdecimal() or not 1 <= int(outputs['attempt']) <= int(execution.report['lineage']['attempt']) or outputs['run_id'] != execution.report['lineage']['run_id']:
            raise ValueError('selected artifact has incompatible run/attempt: ' + id0)
        if not local and not outputs['artifact_id'].isdecimal():
            raise ValueError('artifact ID must be an exact native ID')
        result[id0] = {'job': bindings[id0], **{k: outputs[k] for k in ['artifact_id', 'run_id', 'attempt']}}
    execution.report['selected'] = result
    return result


def read_receipt(execution, unit, selection):
    directory = plain_path(execution.root, unit['handoff'])
    receipt_file = plain_path(directory, 'receipt.json')
    receipt = json_file(receipt_file)
    if not isinstance(receipt, dict):
        raise ValueError('selected receipt must be an object')
    if receipt.get('schema') != 'chrono-release-unit/v1' or receipt.get('status') != 'passed' or receipt.get('failure') is not None or receipt.get('unit') != unit or receipt.get('context') != execution.report['context']:
        raise ValueError('incompatible or failed selected receipt: ' + unit['id'])
    expected = {'kind': execution.report['lineage']['kind'], 'run_id': selection['run_id'],
                'attempt': selection['attempt'], 'job': selection['job']}
    if receipt.get('lineage') != expected:
        raise ValueError('selected artifact lineage mismatch: ' + unit['id'])
    receipt_sha = sha_file(receipt_file)
    if execution.report['lineage']['kind'] == 'local' and selection['artifact_id'] != receipt_sha:
        raise ValueError('selected local original receipt digest mismatch')
    if type(receipt.get('started_ns')) is not int or type(receipt.get('finished_ns')) is not int or not 0 < receipt['started_ns'] <= receipt['finished_ns']:
        raise ValueError('missing original unit timing')
    if receipt['finished_ns'] - receipt['started_ns'] > execution.cfg['limits']['unit_seconds'] * 1000000000:
        raise ValueError('selected original unit exceeded registered bound')
    processes = receipt['processes']
    if not processes or any(type(p['exit_code']) is not int or p['exit_code'] != 0 for p in processes):
        raise ValueError('selected receipt contains original process failure')
    for process in processes:
        for stream in ['stdout', 'stderr']:
            declaration = process[stream]
            file = plain_path(directory, declaration['path'])
            if file.stat().st_size != declaration['size'] or sha_file(file) != declaration['sha256']:
                raise ValueError('original process stream mismatch')
    dependency_input = receipt['dependency_input']
    dependency_file = plain_path(directory, dependency_input['path'])
    if dependency_file.stat().st_size != dependency_input['size'] or sha_file(dependency_file) != dependency_input['sha256']:
        raise ValueError('original dependency input bytes changed')
    original_dependencies = json_file(dependency_file)
    if set(original_dependencies) != {v['job'] for v in receipt['selected'].values()}:
        raise ValueError('original selected dependency membership differs')
    for selected in receipt['selected'].values():
        observed = original_dependencies[selected['job']]
        if observed.get('result') != 'success' or any(observed['outputs'].get(k) != selected[k] for k in ['artifact_id', 'run_id', 'attempt']):
            raise ValueError('selected dependency differs from original native input')
    source_phases = [('source-identity', receipt['context']['source_commit']), ('source-tree', receipt['context']['source_tree']), ('source-final', receipt['context']['source_commit']), ('tree-final', receipt['context']['source_tree'])]
    for phase, expected in source_phases:
        original = [p for p in processes if p['phase'] == phase]
        if len(original) != 1 or plain_path(directory, original[0]['stdout']['path']).read_bytes().decode().strip() != expected:
            raise ValueError('original source observation differs from selected receipt')
    for phase in ['source-clean', 'source-clean-final']:
        original = [p for p in processes if p['phase'] == phase]
        if len(original) != 1 or plain_path(directory, original[0]['stdout']['path']).read_bytes():
            raise ValueError('selected source has no original clean observation')
    if 'toolchain' in receipt:
        for phase, names, field in [('compiler-version', ['cargo', 'rustc'], 'versions'), ('compiler-executable', ['cargo', 'rustc'], 'active_compilers')]:
            original = [p for p in processes if p['phase'] == phase]
            if len(original) != 2:
                raise ValueError('missing original compiler observations')
            for process, name in zip(original, names):
                value = receipt['toolchain'][field][name]
                expected = value if field == 'versions' else value['path']
                if plain_path(directory, process['stdout']['path']).read_bytes().decode().strip() != expected:
                    raise ValueError('original compiler observation differs from selected receipt')
    return directory, receipt, receipt_sha


def verify_toolchain(receipt, cfg):
    value = receipt.get('toolchain', {})
    required = {'git', 'rustup', 'cargo', 'rustc'}
    if receipt['unit']['kind'] == 'verify':
        projects = receipt.get('action_tool')
        if not isinstance(projects, str) or projects not in cfg['tools']:
            raise ValueError('missing original verification action tool')
        required.add(projects)
    if not value.get('versions', {}).get('rustc', '').startswith('rustc ' + cfg['rust_toolchain'] + ' ') or set(value.get('tools', {})) != required:
        raise ValueError('missing original selected toolchain evidence')
    for name, identity0 in value['tools'].items():
        if not isinstance(identity0.get('path'), str) or not Path(identity0['path']).is_absolute() or not isinstance(identity0.get('size'), int) or identity0['size'] <= 0 or not isinstance(identity0.get('mode'), int) or not identity0['mode'] & 0o111 or not isinstance(identity0.get('sha256'), str) or len(identity0['sha256']) != 64 or any(c not in '0123456789abcdef' for c in identity0['sha256']):
            raise ValueError('invalid original tool executable identity: ' + name)
    install = [p for p in receipt['processes'] if p['phase'] == 'toolchain-install']
    components = [a for c in cfg['rust_components'] for a in ['--component', c]]
    if len(install) != 1 or install[0]['argv'] != [value['tools']['rustup']['path'], 'toolchain', 'install', cfg['rust_toolchain'], '--profile', 'minimal', *components]:
        raise ValueError('selected receipt toolchain installation differs from recipe')
    versions = [p for p in receipt['processes'] if p['phase'] == 'compiler-version']
    if len(versions) != 2 or [p['argv'] for p in versions] != [[value['tools'][n]['path'], '--version'] for n in ['cargo', 'rustc']]:
        raise ValueError('missing original compiler version invocations')
    selectors = [p for p in receipt['processes'] if p['phase'] == 'compiler-executable']
    if set(value.get('active_compilers', {})) != {'cargo', 'rustc'} or len(selectors) != 2 or [p['argv'] for p in selectors] != [[value['tools']['rustup']['path'], 'which', '--toolchain', cfg['rust_toolchain'], n] for n in ['cargo', 'rustc']]:
        raise ValueError('missing original active compiler identities/selectors')
    for name in ['cargo', 'rustc']:
        identity0 = value['active_compilers'][name]
        if not Path(identity0['path']).is_absolute() or identity0['size'] <= 0 or not identity0['mode'] & 0o111 or len(identity0['sha256']) != 64:
            raise ValueError('invalid original active compiler executable')
    return value


def toolchain_fingerprint(receipt):
    value = receipt['toolchain']
    return json.dumps({'versions': value['versions'], 'tools': {n: {k: i[k] for k in ['sha256', 'size', 'mode']} for n, i in value['tools'].items() if n in {'git', 'rustup', 'cargo', 'rustc'}}, 'active_compilers': {n: {k: i[k] for k in ['sha256', 'size', 'mode']} for n, i in value['active_compilers'].items()}}, sort_keys=True)


def import_build(execution, unit, selection, plan):
    directory, receipt, receipt_sha = read_receipt(execution, unit, selection)
    verify_toolchain(receipt, execution.cfg)
    original = [p for p in receipt['processes'] if p['phase'] == 'build']
    if len(original) != 1 or original[0]['argv'] != [receipt['toolchain']['tools']['cargo']['path'], 'build', '--release', '--locked', '--manifest-path', unit['manifest']]:
        raise ValueError('selected build recipe/action differs')
    archive = plain_path(directory, 'executable.tar')
    transport = receipt['transport']
    if archive.stat().st_size != transport['size'] or sha_file(archive) != transport['sha256']:
        raise ValueError('selected executable archive mismatch')
    expected = receipt['assets']
    if set(expected) != {unit['asset']} or expected[unit['asset']]['path'] != plan['assets'][unit['asset']]:
        raise ValueError('build asset membership/path mismatch')
    destination = plain_path(execution.root, plan['assets'][unit['asset']])
    destination.parent.mkdir(parents=True, exist_ok=True)
    with tarfile.open(archive, 'r:') as package:
        members = package.getmembers()
        if len(members) != 1 or members[0].name != 'executable' or not members[0].isfile() or members[0].size != expected[unit['asset']]['size'] or members[0].mode != expected[unit['asset']]['mode']:
            raise ValueError('invalid executable archive member/path/mode')
        with package.extractfile(members[0]) as source, tempfile.NamedTemporaryFile(dir=destination.parent, delete=False) as target:
            temporary = Path(target.name)
            shutil.copyfileobj(source, target, 1024 * 1024)
        try:
            temporary.chmod(members[0].mode)
            if not same_identity(expected[unit['asset']], asset_identity(temporary.parent, temporary.name)):
                raise ValueError('transported executable identity mismatch')
            os.replace(temporary, destination)
        finally:
            if temporary.exists(): temporary.unlink()
    return {'receipt_sha256': receipt_sha, 'transport': transport, 'identity': expected[unit['asset']], 'selection': selection}, receipt


def check_staging(execution, staging):
    tracked = execution.run('consumer-tracking', [execution.tools['git'], '--literal-pathspecs', 'ls-files', '-z', '--', *[r['destination'] for r in staging['bindings']]])
    if tracked:
        raise ValueError('release consumer destination is tracked by Git')
    staging['observations'] = []
    execution.report['consumer_staging'] = staging
    try:
        stage(execution.root, staging)
    finally:
        intact = observe(execution.root, staging, 'staged')
    if not intact:
        raise ValueError('release staging identity mismatch')


def package_identity(execution, output, plan):
    release = json_file(output / 'release.json')
    context = execution.report['context']
    if release['schema'] != 'chrono-release/v1' or release['version'] != plan['version'] or release['source_commit'] != context['source_commit'] or release['source_tree'] != context['source_tree'] or len(release['platforms']) != 1:
        raise ValueError('final package source/version/platform mismatch')
    expected_platform = {'Darwin': 'macos', 'Linux': 'linux'}.get(context['system'], context['system'].lower()) + '-' + {'arm64': 'aarch64', 'aarch64': 'aarch64', 'x86_64': 'x86_64'}.get(context['machine'], context['machine'])
    if set(release['platforms']) != {expected_platform}:
        raise ValueError('final package native platform mismatch')
    members = next(iter(release['platforms'].values()))
    expected_files = {'release.json'} | {a + '-' + expected_platform for a in plan['assets']}
    if {p.name for p in output.iterdir()} != expected_files:
        raise ValueError('final package file membership mismatch')
    if set(members) != set(plan['assets']):
        raise ValueError('final package asset membership mismatch')
    observations = {}
    for asset, declaration in members.items():
        if declaration['file'] != asset + '-' + expected_platform:
            raise ValueError('final package asset filename mismatch')
        got = asset_identity(output, declaration['file'])
        expected = execution.report['assets'][asset]['identity']
        if not same_identity(expected, got) or got['sha256'] != declaration['sha256'] or got['size'] != declaration['size']:
            raise ValueError('final package asset byte/mode mismatch: ' + asset)
        observations[asset] = got
    execution.report['package'] = {'release_sha256': sha_file(output / 'release.json'), 'assets': observations}



def retain_original_receipt(execution, id0, directory, receipt, expected_sha):
    target = execution.output / 'originals' / id0
    target.mkdir(parents=True)
    shutil.copy2(plain_path(directory, 'receipt.json'), target / 'receipt.json')
    if sha_file(target / 'receipt.json') != expected_sha:
        raise ValueError('selected receipt changed during final retention')
    files = {'receipt.json': {'sha256': sha_file(target / 'receipt.json'), 'size': (target / 'receipt.json').stat().st_size}}
    dependency = receipt['dependency_input']
    shutil.copy2(plain_path(directory, dependency['path']), target / dependency['path'])
    if sha_file(target / dependency['path']) != dependency['sha256']:
        raise ValueError('original dependency input changed during retention')
    files[dependency['path']] = dependency
    for process in receipt['processes']:
        for stream in ['stdout', 'stderr']:
            declaration = process[stream]
            source = plain_path(directory, declaration['path'])
            destination = plain_path(target, declaration['path'])
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, destination)
            if sha_file(destination) != declaration['sha256'] or destination.stat().st_size != declaration['size']:
                raise ValueError('original stream changed during final receipt retention')
            files[declaration['path']] = declaration
    execution.report.setdefault('originals', {})[id0] = files


def scoped_main(root, output, scope):
    declaration = json_file(root / '.chrono-harness/release/build.json')
    selected_unit = None if scope == '--collect' else next((u for u in declaration.get('units', []) if u['id'] == scope), None)
    required = {'git'} if scope == '--collect' else {'git', 'rustup', 'cargo', 'rustc'}
    if selected_unit is not None and selected_unit.get('kind') == 'verify':
        projects = json_file(path(root, declaration['projects']))
        actions = [a for p in projects['projects'] + projects['scripts'] for a in p['actions'].values() if a['operation'] == selected_unit['operation']]
        if len(actions) != 1:
            raise ValueError('ambiguous or missing selected verification action')
        required.add(actions[0]['tool'])
    required_manifests = {selected_unit['manifest']} if selected_unit is not None and selected_unit.get('kind') == 'build' else set()
    cfg, tools, operations, staging = preflight(root, required, required_manifests)
    plan, builds, tests = units_plan(root, cfg, operations, staging)
    unit = cfg['collector'] if scope == '--collect' else next((u for u in cfg['units'] if u['id'] == scope), None)
    if unit is None:
        raise ValueError('unknown explicit release unit: ' + scope)
    if os.path.lexists(output):
        raise ValueError('release output already exists')
    execution = UnitExecution(root, output, cfg, tools, unit)
    if unit.get('kind') == 'verify':
        execution.report['action_tool'] = next(o[1] for o in operations if o[0] == unit['operation'])
    error = None
    try:
        execution.context()
        if scope != '--collect':
            execution.toolchain()
        if unit.get('kind') == 'build':
            execution.run('build', [tools['cargo'], 'build', '--release', '--locked', '--manifest-path', unit['manifest']])
            observed = asset_identity(root, plan['assets'][unit['asset']])
            output.mkdir(parents=True, exist_ok=True)
            with tarfile.open(output / 'executable.tar', 'x:') as archive:
                archive.add(plain_path(root, observed['path']), arcname='executable', recursive=False)
            execution.report['assets'] = {unit['asset']: observed}
            archive = output / 'executable.tar'
            execution.report['transport'] = {'path': 'executable.tar', 'size': archive.stat().st_size, 'sha256': sha_file(archive)}
        else:
            execution.phase = 'selected-dependencies'
            selected = dependency_selection(execution, unit['needs'])
            build_receipts = {}
            original_receipts = {}
            for id0, build in builds.items():
                execution.phase = 'selected-build:' + id0
                lineage, receipt = import_build(execution, build, selected[id0], plan)
                execution.report['assets'][build['asset']] = lineage
                build_receipts[id0] = receipt
                original_receipts[id0] = (plain_path(root, build['handoff']), receipt)
            producer_versions = {toolchain_fingerprint(r) for r in build_receipts.values()}
            if len(producer_versions) != 1:
                raise ValueError('selected producer toolchain versions differ')
            if scope != '--collect' and toolchain_fingerprint(execution.report) not in producer_versions:
                raise ValueError('verification toolchain differs from selected producers')
            check_staging(execution, staging)
            if scope == '--collect':
                native_metadata = plain_path(root, cfg['collector']['dependency_metadata'])
                metadata = json_file(native_metadata)
                original_metadata = output / 'dependency-file.json'
                shutil.copy2(native_metadata, original_metadata)
                execution.report['dependency_file'] = {'path': original_metadata.name, 'size': original_metadata.stat().st_size, 'sha256': sha_file(original_metadata), 'source_path': cfg['collector']['dependency_metadata']}
                if json_file(original_metadata) != metadata:
                    raise ValueError('registered dependency metadata changed during retention')
                if metadata != json.loads(os.environ['CHRONO_RELEASE_DEPENDENCIES']):
                    raise ValueError('native current dependency metadata differs from selected downloads')
                for id0, test in tests.items():
                    execution.phase = 'selected-verification:' + id0
                    directory, receipt, receipt_sha = read_receipt(execution, test, selected[id0])
                    verify_toolchain(receipt, cfg)
                    if toolchain_fingerprint(receipt) not in producer_versions:
                        raise ValueError('selected verification toolchain differs from producers')
                    original_receipts[id0] = (directory, receipt)
                    if receipt['assets'] != execution.report['assets']:
                        raise ValueError('selected binary or producer lineage changed after verification: ' + id0)
                    action = next(o for o in operations if o[0] == test['operation'])
                    runs = [p for p in receipt['processes'] if p['phase'] == 'verification']
                    if receipt['action_tool'] != action[1]:
                        raise ValueError('selected verification tool differs from registered action')
                    expected_argv = [receipt['toolchain']['tools'][action[1]]['path'], *action[2][1:]]
                    if len(runs) != 1 or runs[0]['operation'] != test['operation'] or runs[0]['argv'] != expected_argv:
                        raise ValueError('verification did not execute exact original registered action')
                    observations = receipt['consumer_staging']['observations']
                    if [o['phase'] for o in observations] != ['staged', 'before-verification', 'after-verification'] or not all(o['matches'] for o in observations):
                        raise ValueError('missing complete before/after release staging observations')
                    expected_files = {r['source'] for r in staging['bindings']} | {r['destination'] for r in staging['bindings']}
                    expected_identities = {r['source']:r['expected'] for r in staging['bindings']}
                    expected_identities.update({r['destination']:r['expected'] for r in staging['bindings']})
                    for snapshot in observations:
                        if {f['path'] for f in snapshot['files']} != expected_files or any(not same_identity(expected_identities[f['path']], f) for f in snapshot['files']):
                            raise ValueError('verification staging vector mismatch')
                    execution.report.setdefault('verifications', {})[id0] = {'receipt_sha256': receipt_sha, 'selection': selected[id0]}
                for id0, (directory, receipt) in original_receipts.items():
                    expected_sha = execution.report['assets'][builds[id0]['asset']]['receipt_sha256'] if id0 in builds else execution.report['verifications'][id0]['receipt_sha256']
                    retain_original_receipt(execution, id0, directory, receipt, expected_sha)
                execution.report['platform_started_ns'] = min(r['started_ns'] for r in build_receipts.values())
                if not observe(root, staging, 'before-package'):
                    raise ValueError('changed consumer before package')
                try:
                    execution.run('package', [str(plain_path(root, plan['assets'][cfg['collector']['package_asset']])), 'pack', '--root', str(root), '--plan', str(root / staging['release_plan']), '--output', str(output / 'package')])
                finally:
                    intact = observe(root, staging, 'after-package')
                if not intact:
                    raise ValueError('changed consumer after package')
                execution.phase = 'package-identity'
                package_identity(execution, output / 'package', plan)
                for filename in ['release.json', *[d['path'] for d in execution.report['package']['assets'].values()]]:
                    file = plain_path(output / 'package', filename)
                    if (output / file.name).exists():
                        raise ValueError('package output collision')
                    os.replace(file, output / file.name)
                (output / 'package').rmdir()
            else:
                operation, _tool, argv = next(o for o in operations if o[0] == unit['operation'])
                if not observe(root, staging, 'before-verification', operation):
                    raise ValueError('changed consumer before verification')
                try:
                    execution.run('verification', argv, operation)
                finally:
                    intact = observe(root, staging, 'after-verification', operation)
                if not intact:
                    raise ValueError('changed release consumer after verification')
        # Every unit must still refer to the same fixed source after its effects.
        if execution.run('source-clean-final', [tools['git'], 'status', '--porcelain', '--untracked-files=normal']) or execution.run('source-final', [tools['git'], 'rev-parse', 'HEAD']).decode().strip() != execution.report['context']['source_commit'] or execution.run('tree-final', [tools['git'], 'rev-parse', 'HEAD^{tree}']).decode().strip() != execution.report['context']['source_tree']:
            raise ValueError('fixed release source changed during unit')
    except (ValueError, OSError, KeyError, TypeError, AttributeError, IndexError, tarfile.TarError, subprocess.CalledProcessError) as caught:
        error = caught
    return execution.save(error)


def local_main(root, output):
    cfg, tools, operations, staging = preflight(root)
    _plan, builds, tests = units_plan(root, cfg, operations, staging)
    if os.path.lexists(output):
        raise ValueError('release output already exists')
    # Fixed HEAD is the only candidate copied; no mutable shared build/consumer roots.
    source = subprocess.check_output([tools['git'], 'rev-parse', 'HEAD'], cwd=root).decode().strip()
    if subprocess.check_output([tools['git'], 'status', '--porcelain', '--untracked-files=normal'], cwd=root):
        raise ValueError('local release requires a fixed clean candidate')
    output.mkdir(parents=True)
    run_id = uuid.uuid4().hex
    metadata = {'schema': 'chrono-release-local/v1', 'source_commit': source,
                'started_ns': time.time_ns(), 'finished_ns': None, 'units': {}, 'status': 'failed', 'driver_tools': {'git': executable_identity(Path(tools['git'])), 'python': executable_identity(Path(sys.executable))}}
    start = time.monotonic()
    script_name = '.chrono-harness/release/build.py'
    roots = output / 'roots'
    receipts = output / 'units'
    deadline_seconds = cfg['limits']['platform_seconds']

    def invoke(id0, dependencies, collect=False):
        unit_root = roots / id0
        logs = output / 'driver-processes' / id0
        logs.mkdir(parents=True)
        commands = [[tools['git'], 'clone', '--no-hardlinks', '--no-checkout', str(root), str(unit_root)],
                    [tools['git'], '-C', str(unit_root), 'checkout', '--detach', source]]
        result = {'processes': [], 'result': 'failure', 'outputs': {}}
        def process(argv, env=None):
            remaining = deadline_seconds - (time.monotonic() - start)
            if remaining <= 0:
                raise ValueError('local release platform span exceeded registered bound')
            process_started = time.time_ns()
            proc = subprocess.Popen(argv, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, start_new_session=True)
            timed_out = False
            try:
                stdout, stderr = proc.communicate(timeout=min(remaining, cfg['limits']['unit_seconds']))
            except subprocess.TimeoutExpired:
                timed_out = True
                os.killpg(proc.pid, signal.SIGKILL)
                stdout, stderr = proc.communicate()
            record = {'argv': argv, 'exit_code': proc.returncode, 'started_ns': process_started, 'finished_ns': time.time_ns(), 'timed_out': timed_out}
            for stream, raw in [('stdout', stdout), ('stderr', stderr)]:
                file = logs / (str(len(result['processes'])) + '.' + stream)
                file.write_bytes(raw)
                record[stream] = {'path': file.relative_to(output).as_posix(), 'size': len(raw), 'sha256': hashlib.sha256(raw).hexdigest()}
            result['processes'].append(record)
            return proc.returncode
        try:
            for argv in commands:
                if process(argv):
                    return id0, result
            for need in dependencies:
                declaration = next(u for u in cfg['units'] if u['id'] == need)
                target = plain_path(unit_root, declaration['handoff'])
                shutil.copytree(receipts / need, target)
            env = dict(os.environ, CHRONO_RELEASE_LOCAL='1', CHRONO_RELEASE_RUN=run_id,
                       CHRONO_RELEASE_ATTEMPT='1', CHRONO_RELEASE_JOB=id0,
                       CHRONO_RELEASE_DEPENDENCIES=json.dumps(dependencies))
            if collect:
                write_json(plain_path(unit_root, cfg['collector']['dependency_metadata']), dependencies)
            argv = [sys.executable, str(unit_root / script_name), str(unit_root), str(receipts / id0),
                    *(['--collect'] if collect else ['--unit', id0])]
            code = process(argv, env)
            if code == 0:
                receipt = json_file(receipts / id0 / 'receipt.json')
                if receipt['status'] != 'passed':
                    raise ValueError('successful wrapper has no successful original receipt')
                result['result'] = 'success'
                result['outputs'] = {'artifact_id': sha_file(receipts / id0 / 'receipt.json'),
                                     'run_id': run_id, 'attempt': '1'}
        except (ValueError, OSError, KeyError, subprocess.TimeoutExpired) as error:
            result['error'] = str(error)
        finally:
            # Roots are disposable products of this invocation; keep receipts and raw streams.
            if unit_root.is_dir(): shutil.rmtree(unit_root)
        return id0, result

    outcomes = {}
    try:
        with ThreadPoolExecutor(max_workers=cfg['local_workers']) as pool:
            futures = [pool.submit(invoke, id0, {}) for id0 in builds]
            for future in as_completed(futures):
                id0, result = future.result(); outcomes[id0] = result
            dependencies = {id0: {'result': r['result'], 'outputs': r['outputs']} for id0, r in outcomes.items()}
            futures = [pool.submit(invoke, id0, dependencies) for id0 in tests]
            for future in as_completed(futures):
                id0, result = future.result(); outcomes[id0] = result
        dependencies = {id0: {'result': r['result'], 'outputs': r['outputs']} for id0, r in outcomes.items()}
        id0, result = invoke(cfg['collector']['id'], dependencies, True)
        outcomes[id0] = result
        if result['result'] == 'success':
            accepted = receipts / id0
            receipt = json_file(accepted / 'receipt.json')
            for declaration in receipt['package']['assets'].values():
                source_file = plain_path(accepted, declaration['path'])
                destination = plain_path(output, declaration['path'])
                shutil.copy2(source_file, destination)
                if not same_identity(declaration, asset_identity(output, declaration['path'])):
                    raise ValueError('accepted local package changed during handoff')
            retained = {p[stream]['path']: p[stream] for p in receipt['processes'] for stream in ['stdout', 'stderr']}
            retained[receipt['dependency_input']['path']] = receipt['dependency_input']
            retained[receipt['dependency_file']['path']] = receipt['dependency_file']
            retained.update({'originals/' + id1 + '/' + name: declaration for id1, files in receipt['originals'].items() for name, declaration in files.items()})
            for name, declaration in retained.items():
                source_file, destination = plain_path(accepted, name), plain_path(output, name)
                destination.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(source_file, destination)
                if sha_file(destination) != declaration['sha256'] or destination.stat().st_size != declaration['size']:
                    raise ValueError('original local evidence changed during handoff')
            for name in ['release.json', 'receipt.json']:
                shutil.copy2(accepted / name, output / name)
            if sha_file(output / 'release.json') != receipt['package']['release_sha256']:
                raise ValueError('local release manifest changed during handoff')
            metadata['status'] = 'passed'
    finally:
        metadata['units'] = outcomes
        metadata['finished_ns'] = time.time_ns()
        write_json(output / 'local.json', metadata)
    return 0 if metadata['status'] == 'passed' else 2


def main(root, output, scope=None):
    cfg = json_file(root / '.chrono-harness/release/build.json')
    if scope is not None:
        return scoped_main(root, output, scope)
    if cfg['schema'] == 'chrono-release-build/v4':
        return local_main(root, output)
    return legacy_main(root, output)


if __name__ == '__main__':
    try:
        if len(sys.argv) not in (3, 4, 5) or (len(sys.argv) == 4 and sys.argv[3] != '--collect') or (len(sys.argv) == 5 and sys.argv[3] != '--unit'):
            raise ValueError('usage: build.py ROOT ABSENT_OUTPUT [--unit ID | --collect]')
        scope = None if len(sys.argv) == 3 else ('--collect' if len(sys.argv) == 4 else sys.argv[4])
        sys.exit(main(Path(sys.argv[1]).resolve(), Path(sys.argv[2]).absolute(), scope))
    except (ValueError, OSError, KeyError, TypeError) as error:
        print(str(error), file=sys.stderr)
        sys.exit(2)
