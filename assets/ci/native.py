# chrono-ci: owned native-adoption/v1
"""Generated native acquisition/forwarding adapter. Host declarations select all inputs."""
import argparse
import datetime
import calendar
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import selectors
import signal
import time
import re
import shutil

BOUND = 64 * 1024 * 1024


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def addressed(path):
    p = Path(path)
    if p.is_absolute() or not p.parts or any(x in ('..', '.') for x in p.parts) or p.as_posix() != path:
        raise ValueError('invalid addressed path: ' + str(path))
    return p


def raw_file(root, path):
    p = addressed(path)
    target = root
    for part in p.parts:
        target = target / part
        if target.is_symlink():
            raise ValueError('symlink in original path')
    if not target.is_file() or target.stat().st_size > BOUND:
        raise ValueError('missing/unbounded JSON original: ' + str(path))
    with target.open('rb') as f:
        raw = f.read(BOUND + 1)
    if len(raw) > BOUND:
        raise ValueError('JSON original exceeds bound')
    return raw


def write(root, path, raw, replace=False):
    target = root / addressed(path)
    target.parent.mkdir(parents=True, exist_ok=True)
    # Validate parents without requiring the destination to exist.
    for p in [target, *target.parents]:
        if p == root.parent:
            break
        if p.is_symlink():
            raise ValueError('symlink publication')
    if target.exists() and not replace:
        if target.read_bytes() != raw:
            raise ValueError('original publication collision')
        return
    with tempfile.NamedTemporaryFile(dir=target.parent, delete=False) as f:
        f.write(raw)
        temp = Path(f.name)
    os.replace(temp, target)


def encoded(value):
    return json.dumps(value, separators=(',', ':'), ensure_ascii=False).encode()


def retain(root, directory, prefix, raw):
    path = directory + prefix + '-' + digest(raw) + '.json'
    write(root, path, raw)
    return {'path': path, 'sha256': digest(raw)}


def file_digest(path):
    hash = hashlib.sha256()
    with Path(path).open('rb') as f:
        for block in iter(lambda: f.read(65536), b''):
            hash.update(block)
    return hash.hexdigest()


def process(root, argv, stdin, env, directory, prefix, timeout, limit, credential_environment=()):
    # Actual child environment is distinct from the runner's outer producer environment.
    configured_argv = list(argv)
    chosen = argv[0] if '/' in argv[0] else shutil.which(argv[0], path=env.get('PATH', ''))
    if chosen is None:
        raise ValueError('declared child executable not found in actual PATH')
    executable = (root / chosen).resolve()
    executable_sha256 = file_digest(executable)
    argv = [str(executable), *argv[1:]]
    # Drain bounded original bytes while enforcing the registered process deadline.
    with tempfile.TemporaryFile() as original_stdin:
        original_stdin.write(stdin)
        original_stdin.seek(0)
        p = subprocess.Popen(argv, cwd=root, stdin=original_stdin, env=env,
                             stdout=subprocess.PIPE, stderr=subprocess.PIPE, start_new_session=True)
        def stop_group():
            if p.poll() is not None:
                return
            try:
                os.killpg(p.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            except PermissionError:
                # macOS can deny signalling a group after its leader has exited.
                if p.poll() is None:
                    p.kill()
        selector = selectors.DefaultSelector()
        selector.register(p.stdout, selectors.EVENT_READ, 'stdout')
        selector.register(p.stderr, selectors.EVENT_READ, 'stderr')
        buffers = {'stdout': bytearray(), 'stderr': bytearray()}
        deadline, failure = time.monotonic() + timeout, None
        while selector.get_map():
            if time.monotonic() >= deadline and failure is None:
                failure = 'timeout'
                stop_group()
            for key, _ in selector.select(0.1):
                chunk = os.read(key.fileobj.fileno(), 65536)
                if not chunk:
                    selector.unregister(key.fileobj)
                    key.fileobj.close()
                    continue
                buffer = buffers[key.data]
                remaining = limit - len(buffer)
                buffer.extend(chunk[:remaining])
                if len(chunk) > remaining and failure is None:
                    failure = 'output bound exceeded'
                    stop_group()
        selector.close()
        code = p.wait()
        stdout, stderr = bytes(buffers['stdout']), bytes(buffers['stderr'])
    input_ref = retain(root, directory, prefix + '-stdin', stdin)
    out_ref = retain(root, directory, prefix + '-stdout', stdout)
    err_ref = retain(root, directory, prefix + '-stderr', stderr)
    # Credential values are never republished; their actual omission is explicit.
    if file_digest(executable) != executable_sha256:
        failure = 'child executable changed during execution'
    receipt = {'schema': 'chrono-native-forwarding/v1', 'argv': argv, 'configured_argv': configured_argv, 'cwd': str(root),
               'executable': {'path': str(executable), 'sha256': executable_sha256},
               'environment': {k: digest(v.encode()) for k, v in env.items() if k not in credential_environment},
               'omitted_credentials': list(credential_environment),
               'original_environment_sha256': digest(encoded(env)),
               'environment_representation': 'sha256', 'stdin': input_ref, 'stdout': out_ref,
               'stderr': err_ref, 'exit_code': code, 'failure': failure}
    receipt_ref = retain(root, directory, prefix + '-process', encoded(receipt))
    if failure or code:
        raise ValueError('child failed; original receipt retained at ' + receipt_ref['path'])
    return stdout, [input_ref, out_ref, err_ref, receipt_ref]


def selection(provider, unit):
    workflow = provider['units'][unit] if unit else provider['collection']
    context = provider['full_contexts']['units'][unit] if unit else provider['full_contexts']['collection']
    return workflow, context, workflow['artifact_directory'] + 'preparation/'


def observations(root, provider, unit):
    workflow, _, directory = selection(provider, unit)
    keys = ['GITHUB_EVENT_NAME', 'GITHUB_EVENT_PATH', 'CHRONO_WORKFLOW_REVISION',
            'GITHUB_REPOSITORY', 'GITHUB_RUN_ID', 'GITHUB_RUN_ATTEMPT']
    actual = {key: os.environ[key] for key in keys}
    with Path(actual['GITHUB_EVENT_PATH']).open('rb') as f:
        payload_raw = f.read(BOUND + 1)
    if len(payload_raw) > BOUND:
        raise ValueError('event payload bound')
    payload = retain(root, directory, 'event-payload', payload_raw)
    report = {'schema': 'chrono-native-acquisition/v1', 'observed': actual, 'payload': payload}
    original = retain(root, directory, 'acquisition', encoded(report))
    write(root, workflow['artifact_directory'] + 'acquisition.json', encoded(original), True)
    return report, original


def endpoint_args(root, config, provider, observed, unit, command='prepare-endpoints'):
    args = [provider['collection']['generator'], command, '--host-root', str(root), '--config', config,
            '--event', observed['GITHUB_EVENT_NAME'], '--payload', observed['GITHUB_EVENT_PATH'],
            '--workflow-revision', observed['CHRONO_WORKFLOW_REVISION']]
    if unit:
        args += ['--unit', unit]
    return args


def moment(value):
    # Python 3.9 accepts only some fraction lengths; Rust emits exact RFC3339 nanoseconds.
    match = re.fullmatch(r'(\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d)(?:\.(\d{1,9}))?(?:Z|\+00:00)', value)
    if match is None:
        raise ValueError('native context time must be UTC')
    date = datetime.datetime.fromisoformat(match[1])
    return calendar.timegm(date.timetuple()) * 1000000000 + int((match[2] or '').ljust(9, '0'))


def freshness(root, provider, ctx, now, endpoints):
    identity = endpoints['workflow_policy']
    raw = raw_file(root, identity['path'])
    if digest(raw) != identity['sha256']:
        raise ValueError('observed workflow policy drift')
    workflow = json.loads(raw)
    started, observed = moment(ctx['branch_started_at']), moment(ctx['observed_at'])
    current = moment(now.isoformat())
    if observed < started or current < observed or current - started > workflow['staleness']['max_age_hours'] * 3600000000000:
        raise ValueError('stale/future shared context observation')


def publish(root, config, provider, extension):
    report, acquisition = observations(root, provider, None)
    actual = report['observed']
    _, context_path, directory = selection(provider, None)
    raw, processes = process(root, endpoint_args(root, config, provider, actual, None), b'', dict(os.environ), directory,
                             'event-endpoints', provider['gather']['timeout_seconds'], provider['gather']['output_limit_bytes'], provider['gather']['credential_environment'])
    endpoints = json.loads(raw)
    birth = endpoints['lineage']['birth']
    lineage_raw = raw_file(root, extension['lineage']['path'])
    if digest(lineage_raw) != extension['lineage']['sha256']:
        raise ValueError('original lineage byte drift')
    payload = json.loads(raw_file(root, report['payload']['path']))
    if actual['GITHUB_EVENT_NAME'] == 'push':
        roles = [role for prefix, role in extension['push_roles'].items() if payload['ref'].startswith(prefix)]
        if len(roles) != 1:
            raise ValueError('missing/ambiguous host push role')
        role = roles[0]
    elif actual['GITHUB_EVENT_NAME'] == 'pull_request':
        role = extension['pull_request_role']
    else:
        raise ValueError('shared native seed requires push/pull_request')
    integration_raw = None
    evidence = extension['integration_evidence']
    if role == 'delivery' and extension.get('integration_evidence_path'):
        evidence_path = extension['integration_evidence_path']
        if (root / evidence_path).exists():
            integration_raw = raw_file(root, evidence_path)
            evidence = digest(integration_raw)
            if extension['integration_evidence'] is not None and extension['integration_evidence'] != evidence:
                raise ValueError('declared integration handoff digest differs')
        else:
            evidence = None
    observed = datetime.datetime.now(datetime.timezone.utc)
    ctx = {'schema_version': 2, 'base': endpoints['base'], 'candidate': endpoints['candidate'],
           'dev_tip': endpoints['base'], 'branch_ref': birth['branch_ref'], 'fork_point': birth['base'],
           'branch_started_at': birth['branch_started_at'], 'observed_at': observed.isoformat().replace('+00:00', 'Z'),
           'operation': 'validate.delta', 'run_kind': role, 'integration_evidence': evidence,
           'retained_inputs': extension['retained_inputs']}
    freshness(root, provider, ctx, observed, endpoints)
    seed = encoded(ctx)
    write(root, context_path, seed, True)
    seed_dir = extension['seed_directory']
    write(root, seed_dir + 'context.json', seed)
    write(root, seed_dir + 'lineage.json', lineage_raw)
    write(root, seed_dir + 'endpoints.json', raw)
    write(root, seed_dir + 'payload.json', raw_file(root, report['payload']['path']))
    if integration_raw is not None:
        write(root, seed_dir + 'integration.json', integration_raw)
    binding = {'schema': 'chrono-native-seed/v1', 'event': actual['GITHUB_EVENT_NAME'],
               'repository': actual['GITHUB_REPOSITORY'], 'base': endpoints['base'], 'candidate': endpoints['candidate'],
               'workflow': provider['collection']['workflow_path'], 'workflow_revision': actual['CHRONO_WORKFLOW_REVISION'],
               'run': int(actual['GITHUB_RUN_ID']), 'attempt': int(actual['GITHUB_RUN_ATTEMPT']),
               'context_sha256': digest(seed), 'lineage_sha256': digest(lineage_raw), 'integration_sha256': evidence,
               'payload_sha256': report['payload']['sha256'], 'endpoints_sha256': digest(raw),
               'acquisition': acquisition, 'processes': processes}
    original_map = {}
    for original in [acquisition, report['payload'], *processes]:
        raw_original = raw_file(root, original['path'])
        file = 'originals/' + original['sha256']
        write(root, seed_dir + file, raw_original)
        original_map[original['path']] = {'file': file, 'sha256': original['sha256']}
    binding['originals'] = original_map
    write(root, seed_dir + 'binding.json', encoded(binding))


def acquire(root, config, provider, extension, unit):
    report, _ = observations(root, provider, unit)
    _, _, directory = selection(provider, unit)
    args = endpoint_args(root, config, provider, report['observed'], unit, 'acquire-seed')
    args += ['--repository', report['observed']['GITHUB_REPOSITORY']]
    process(root, args, b'', dict(os.environ), directory, 'seed-acquisition',
            provider['gather']['wait_seconds'] + provider['gather']['timeout_seconds'], BOUND, provider['gather']['credential_environment'])
    _, context_path, _ = selection(provider, unit)
    freshness(root, provider, json.loads(raw_file(root, context_path)), datetime.datetime.now(datetime.timezone.utc),
              json.loads(raw_file(root, provider['units'][unit]['artifact_directory'] + 'seed/endpoints.json')))


def composition(root, provider, extension, p, directory, env):
    manifest = json.loads(raw_file(root, provider['gather']['manifest_path']))
    sources = [{'pair': pair} for pair in extension['composition_sources']]
    for item in manifest['reports']:
        raw = raw_file(root, item['path'])
        if digest(raw) != item['sha256']:
            raise ValueError('gather report identity drift')
        report = json.loads(raw)
        context_address = report['request']['context']['path']
        # Context raw is small; original snapshot and all business blobs stay addressed.
        ctx = artifact_json(root, report['artifacts'], context_address, item.get('artifacts'))
        pair_path = ctx['retained_inputs']
        entry = report['artifacts'][pair_path]
        sources.append({'pair': {'path': pair_path, 'sha256': entry['sha256']},
                        'transport': item.get('artifacts'), 'artifacts': report['artifacts']})
    composition_manifest = retain(root, directory, 'composition-manifest', encoded({'schema': 'chrono-input-composition/v1', 'sources': sources}))
    # Immutable output selected by original manifest digest; the common path is a current projection.
    output = '.chrono-harness/state/inputs/composed-' + composition_manifest['sha256'] + '.json'
    receipt = output + '.receipt.json'
    args = [extension['inputs_program'], 'compose', '--host-root', str(root), '--config', p['profile'],
            '--base', p['base'], '--candidate', p['candidate'], '--manifest', composition_manifest['path'],
            '--output', output, '--receipt', receipt]
    _, refs = process(root, args, b'', env, directory, 'composition', 900, BOUND, provider['gather']['credential_environment'])
    write(root, extension['retained_inputs'], raw_file(root, output), True)
    provenance = json.loads(raw_file(root, receipt))
    originals = [composition_manifest, *refs]
    for path in [output, receipt, extension['retained_inputs']]:
        originals.append(retain(root, directory, 'composition-original', raw_file(root, path)))
    for source in provenance['originals']:
        originals.append(retain(root, directory, 'constituent', raw_file(root, source['retained']['path'])))
    return originals


def artifact_json(root, artifacts, address, transport):
    a = artifacts[address]
    if 'hex' in a:
        raw = bytes.fromhex(a['hex'])
    else:
        if a['schema'] != 'chrono-retained-blob/v1':
            raise ValueError('external original schema')
        path = a['storage']
        if transport:
            if not path.startswith(transport['source_directory']):
                raise ValueError('external original outside transport root')
            path = transport['directory'] + path[len(transport['source_directory']):]
        raw = raw_file(root, path)
    if digest(raw) != a['sha256'] or len(raw) != a['length']:
        raise ValueError('external original identity')
    return json.loads(raw)


def forward(root, config, provider, extension):
    stdin = sys.stdin.buffer.read(BOUND + 1)
    if len(stdin) > BOUND:
        raise ValueError('input request bound')
    req = json.loads(stdin)
    if Path(req['host_root']).resolve() != root or req['source'] != 'ci':
        raise ValueError('forwarding source/root mismatch')
    unit = req['selection'].get('unit')
    workflow, context_path, directory = selection(provider, unit)
    acquisition_ref = json.loads(raw_file(root, workflow['artifact_directory'] + 'acquisition.json'))
    acquisition_raw = raw_file(root, acquisition_ref['path'])
    if digest(acquisition_raw) != acquisition_ref['sha256']:
        raise ValueError('acquisition original drift')
    acquisition = json.loads(acquisition_raw)
    actual = acquisition['observed']
    payload = acquisition['payload']
    if digest(raw_file(root, payload['path'])) != payload['sha256']:
        raise ValueError('event original drift')
    env = dict(os.environ)
    # Supply the actual retained records to the child's existing named-env interface.
    env.update({key: actual[key] for key in ['GITHUB_EVENT_NAME', 'CHRONO_WORKFLOW_REVISION', 'GITHUB_REPOSITORY']})
    env['GITHUB_EVENT_PATH'] = str(root / payload['path'])
    endpoints_path = workflow['artifact_directory'] + 'seed/endpoints.json' if unit else extension['seed_directory'] + 'endpoints.json'
    freshness(root, provider, json.loads(raw_file(root, context_path)), datetime.datetime.now(datetime.timezone.utc),
              json.loads(raw_file(root, endpoints_path)))
    args = [provider['collection']['generator'], 'check-inputs', '--config', config,
            '--event-env', 'GITHUB_EVENT_NAME', '--payload-env', 'GITHUB_EVENT_PATH',
            '--revision-env', 'CHRONO_WORKFLOW_REVISION', '--repository-env', 'GITHUB_REPOSITORY']
    policy = json.loads(raw_file(root, req['effective_config']))
    credentials = sorted(set(provider['gather']['credential_environment']) | set(policy['environment'].get('credential_environment', [])))
    stdout, refs = process(root, args, stdin, env, directory, 'forward', 900, BOUND, credentials)
    p = json.loads(stdout)
    original_report = json.loads(raw_file(root, p['evidence']['report_path']))
    # Preserve the child report's manifest identity used by PreparedCheck's actual consumer.
    report = dict(original_report)
    originals = list(p['originals']) + refs + [acquisition_ref, payload]
    seed_directory = workflow['artifact_directory'] + 'seed/' if unit else extension['seed_directory']
    for name in ['context.json', 'binding.json', 'lineage.json', 'payload.json', 'endpoints.json']:
        originals.append(retain(root, directory, 'seed-original', raw_file(root, seed_directory + name)))
    seed_binding = json.loads(raw_file(root, seed_directory + 'binding.json'))
    if seed_binding.get('integration_sha256') is not None and extension.get('integration_evidence_path'):
        raw_integration = raw_file(root, seed_directory + 'integration.json')
        if digest(raw_integration) != seed_binding['integration_sha256']:
            raise ValueError('original integration handoff drift')
        originals.append(retain(root, directory, 'seed-integration-original', raw_integration))
    for original in seed_binding['originals'].values():
        raw_original = raw_file(root, seed_directory + original['file'])
        if digest(raw_original) != original['sha256']:
            raise ValueError('seed original publication closure drift')
        originals.append(retain(root, directory, 'seed-producer-original', raw_original))
    if req['selection']['kind'] == 'collect':
        originals += composition(root, provider, extension, p, directory, env)
    report['forwarding'] = {'acquisition': acquisition_ref, 'process': refs[-1], 'child_report': p['evidence']}
    by_path = {}
    for original in originals:
        if original['path'] in by_path and by_path[original['path']] != original:
            raise ValueError('original address conflict')
        by_path[original['path']] = original
    originals = list(by_path.values())
    report['originals'] = originals
    outer = retain(root, directory, 'adapter-result', encoded(report))
    p['originals'] = originals + [outer]
    p['evidence'] = dict(p['evidence'], report_path=outer['path'], report_sha256=outer['sha256'])
    sys.stdout.buffer.write(encoded(p) + b'\n')


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('operation', choices=['publish', 'acquire', 'forward'])
    parser.add_argument('--config', required=True)
    parser.add_argument('--unit')
    args = parser.parse_args()
    root = Path.cwd().resolve()
    provider = json.loads(raw_file(root, args.config))
    extension = provider['native_adoption']
    if extension['schema'] != 'chrono-native-adoption/v1':
        raise ValueError('native adoption extension schema')
    if args.operation == 'publish':
        publish(root, args.config, provider, extension)
    elif args.operation == 'acquire':
        if not args.unit:
            raise ValueError('seed acquisition needs registered unit')
        acquire(root, args.config, provider, extension, args.unit)
    else:
        forward(root, args.config, provider, extension)


if __name__ == '__main__':
    try:
        main()
    except (ValueError, KeyError, OSError, subprocess.SubprocessError) as error:
        print('E_NATIVE_ADOPTION:', error, file=sys.stderr)
        sys.exit(2)
