"""Release recipe behavior, staging identities and original failure evidence."""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).with_name('build.py')
TOOL = Path(__file__).parent / 'fixtures/recipe-tool.py'
BUILD = '.chrono-harness/release/build.json'
PROJECTS = 'registered/actions.json'
PLAN = 'registered/release.json'
HOST = 'registered/host.json'
RELEASE = 'native build/chosen-tool'
CONSUMER = 'consumer/bin/tool'
RELEASE_BYTES = (Path(__file__).parent / 'fixtures/recipe-consumer.py').read_bytes()


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def write(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2) + '\n')


class Recipe:
    def __init__(self):
        self.temp = tempfile.TemporaryDirectory(prefix='release consumer λ ')
        self.root = Path(self.temp.name).resolve() / 'source with spaces'
        self.output = Path(self.temp.name) / 'native output'
        self.calls_path = Path(self.temp.name) / 'calls.jsonl'
        for tool in ['tools/rustup', 'tools/cargo', 'tools/rustc', 'tools/git',
                     'tools/probe', 'crates/distribution/target/release/chrono-distribution']:
            path = self.root / tool
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(TOOL.read_bytes())
            path.chmod(0o755)
        manifest = self.root / 'arbitrary layout/product.toml'
        manifest.parent.mkdir()
        manifest.write_text('fixture')
        write(self.root / BUILD, {
            'schema': 'chrono-release-build/v2', 'rust_toolchain': '1.95.0',
            'manifests': ['arbitrary layout/product.toml'], 'projects': PROJECTS,
            'tools': {name: 'tools/' + name for name in ['rustup', 'cargo', 'rustc', 'git', 'probe']},
            'verification_operations': ['prepare.custom', 'verify.custom', 'finish.custom']})
        write(self.root / PROJECTS, {
            'projects': [{'id': 'custom', 'actions': {
                'first': {'operation': 'prepare.custom', 'tool': 'probe',
                          'argv': ['prepare', 'a space', '$literal\n`text`']},
                'second': {'operation': 'verify.custom', 'tool': 'probe',
                           'argv': ['verify', '--declared']}}}],
            'scripts': [{'id': 'standalone', 'actions': {
                'run': {'operation': 'finish.custom', 'tool': 'probe', 'argv': []}}}]})

    def edit(self, path, change):
        target = self.root / path
        value = json.loads(target.read_bytes())
        change(value)
        write(target, value)

    def run(self, fault=''):
        env = dict(os.environ, CHRONO_RECIPE_CALLS=str(self.calls_path),
                   CHRONO_RECIPE_FAIL=fault)
        env['PATH'] = str(self.root / 'tools') + os.pathsep + env['PATH']
        return subprocess.run([sys.executable, '-B', str(SCRIPT), str(self.root), str(self.output)],
                              cwd='/', env=env, capture_output=True)

    def calls(self):
        return ([json.loads(row) for row in self.calls_path.read_text().splitlines()]
                if self.calls_path.exists() else [])


class RecipeTests(unittest.TestCase):
    def fixture(self, staged=False):
        f = Recipe()
        self.addCleanup(f.temp.cleanup)
        if staged:
            release = f.root / RELEASE
            release.parent.mkdir(parents=True)
            release.write_bytes(RELEASE_BYTES)
            release.chmod(0o755)
            write(f.root / PLAN, {'schema': 'chrono-release-plan/v1', 'version': 'fixture',
                                  'assets': {'chosen': RELEASE}})
            write(f.root / HOST, {'artifacts': [{'path': 'consumer/', 'tracked': False}]})
            f.edit(BUILD, lambda v: v.update({
                'schema': 'chrono-release-build/v3', 'rust_components': [],
                'consumer_staging': {'release_plan': PLAN, 'host_config': HOST,
                    'bindings': [{'asset': 'chosen', 'destinations': [CONSUMER]}]},
                'verification_operations': ['verify.consumer']}))
            write(f.root / PROJECTS, {'projects': [], 'scripts': [{'actions': {'execute': {
                'operation': 'verify.consumer', 'tool': 'probe',
                'argv': ['consumer', CONSUMER, RELEASE]}}}]})
        return f

    def evidence(self, f, staged=False):
        report = json.loads((f.output / 'build.json').read_bytes())
        self.assertEqual(report['schema'], 'chrono-native-build/v4' if staged else 'chrono-native-build/v3')
        if not staged:
            self.assertEqual(report['source_commit'], 'a' * 40)
        for process in report['processes']:
            if not staged:
                self.assertEqual(process['cwd'], str(f.root))
            for stream in ['stdout', 'stderr']:
                self.assertEqual(process[stream + '_sha256'], digest(bytes(process[stream + '_bytes'])))
        return report

    def test_release_recipe_runs_registered_operations_in_order_with_literal_arguments(self):
        f = self.fixture()
        out = f.run()
        self.assertEqual(out.returncode, 0, out.stderr)
        calls = f.calls()
        self.assertTrue(all(c['cwd'] == str(f.root) for c in calls))
        probes = [c for c in calls if c['tool'] == 'probe']
        self.assertEqual(len(probes), 3)
        self.assertEqual(probes[0]['argv'], ['prepare', 'a space', '$literal\n`text`'])
        self.assertEqual(probes[1]['argv'], ['verify', '--declared'])
        self.assertEqual(probes[2]['argv'], [])
        pack = next(i for i, c in enumerate(calls) if c['tool'] == 'chrono-distribution')
        self.assertEqual(calls[pack - 1]['tool'], 'probe')
        report = self.evidence(f)
        self.assertEqual(report['status'], 'passed')
        self.assertIsNone(report['failure'])
        verification = [p for p in report['processes'] if p['phase'] == 'verification']
        self.assertEqual(len(verification), 3)
        self.assertEqual(len(report['processes']), len(calls))
        self.assertEqual(report['processes'][-1]['phase'], 'package')
        for operation, actual in zip(['prepare.custom', 'verify.custom', 'finish.custom'], verification):
            self.assertEqual(actual['operation'], operation)
            self.assertEqual(actual['exit_code'], 0)
            self.assertEqual(actual['cwd'], str(f.root))
            self.assertEqual(actual['argv'][0], str(f.root / 'tools/probe'))
            self.assertEqual(bytes(actual['stdout_bytes']), b'actual stdout\xff\n')
            self.assertEqual(bytes(actual['stderr_bytes']), b'actual stderr\xfe\n')
            self.assertEqual(actual['stdout_sha256'], digest(b'actual stdout\xff\n'))
            self.assertEqual(actual['stderr_sha256'], digest(b'actual stderr\xfe\n'))

    def test_release_recipe_rejects_invalid_operations_before_building(self):
        for case in range(6):
            with self.subTest(case=case):
                f = self.fixture()
                if case == 0:
                    f.edit(BUILD, lambda v: v.update(verification_operations=['missing']))
                elif case == 1:
                    f.edit(BUILD, lambda v: v.update(verification_operations=['verify.custom', 'verify.custom']))
                elif case == 2:
                    f.edit(BUILD, lambda v: v['tools'].pop('probe'))
                elif case == 3:
                    f.edit(PROJECTS, lambda v: v['projects'][0]['actions']['second'].update(operation='prepare.custom'))
                elif case == 4:
                    f.edit(PROJECTS, lambda v: v['projects'][0]['actions']['second'].update(argv=['verify', 7]))
                else:
                    f.edit(BUILD, lambda v: v['tools'].update(probe='tools/missing'))
                out = f.run()
                self.assertNotEqual(out.returncode, 0)
                self.assertEqual(f.calls(), [])
                self.assertFalse(f.output.exists())

    def test_release_recipe_preserves_failed_operation_exit_and_never_packages(self):
        f = self.fixture()
        out = f.run('prepare')
        self.assertEqual(out.returncode, 73)
        self.assertEqual(f.calls()[-1]['argv'][0], 'prepare')
        self.assertFalse(any(c['tool'] == 'chrono-distribution' for c in f.calls()))
        self.assertEqual(self.evidence(f)['status'], 'failed')
        self.assertFalse((f.output / 'release.json').exists())
        self.assertIn(b'actual stdout\xff\n', out.stdout)
        self.assertIn(b'actual stderr\xfe\n', out.stderr)

    def test_release_recipe_retains_failed_operation_and_prior_processes(self):
        f = self.fixture()
        self.assertEqual(f.run('verify').returncode, 73)
        report = self.evidence(f)
        self.assertEqual(report['status'], 'failed')
        self.assertEqual(report['failure']['phase'], 'verification')
        self.assertEqual(report['failure']['exit_code'], 73)
        processes = report['processes']
        self.assertEqual(len(processes), len(f.calls()))
        last = processes[-1]
        self.assertEqual(last['operation'], 'verify.custom')
        self.assertEqual(last['exit_code'], 73)
        self.assertEqual(bytes(last['stdout_bytes']), b'actual stdout\xff\n')
        self.assertEqual(bytes(last['stderr_bytes']), b'actual stderr\xfe\n')
        self.assertTrue(any(p.get('operation') == 'prepare.custom' and p['exit_code'] == 0 for p in processes))
        self.assertFalse(any(p.get('operation') == 'finish.custom' or p['phase'] == 'package' for p in processes))
        self.assertFalse((f.output / 'release.json').exists())

    def test_release_recipe_carries_nested_native_failure_evidence_into_the_artifact(self):
        originals = {
            'state/collection/preparation/receipt.json': b'{"exit_code":73,"stderr":"original"}',
            'state/collection/preparation/stdin.bin': b'original input',
            'state/collection/preparation/stdout.bin': b'partial output',
            'state/collection/preparation/stderr.bin': b'original stderr',
            'process.json': b'{"exit_code":73}', 'stdout': b'fixture stdout', 'stderr': b'fixture stderr'}
        for spelling in ['evidence', 'evidence-alias']:
            with self.subTest(spelling=spelling):
                f = self.fixture()
                self.assertEqual(f.run(spelling).returncode, 73)
                report = self.evidence(f)
                process = report['processes'][-1]
                nested = process['failure_evidence']
                self.assertEqual(nested['status'], 'retained')
                self.assertEqual(report['status'], 'failed')
                self.assertEqual(process['exit_code'], 73)
                self.assertFalse((f.output / 'release.json').exists())
                for name, original in originals.items():
                    row = next(row for row in nested['files'] if row['source'] == name)
                    self.assertEqual(row['status'], 'retained')
                    self.assertEqual((f.output / row['path']).read_bytes(), original)
                    self.assertEqual(row['sha256'], digest(original))
                    self.assertEqual(row['bytes'], len(original))
                shutil.rmtree(Path(tempfile.gettempdir()) / nested['source'])

    def test_native_failure_retention_preserves_directory_and_byte_boundaries(self):
        spec = importlib.util.spec_from_file_location('tested_release_recipe', SCRIPT)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        retain, bound = module.retain_failure_evidence, module.EVIDENCE_BOUND
        with tempfile.TemporaryDirectory() as temporary, tempfile.TemporaryDirectory(prefix='chrono-native-publication-failure-') as original:
            outside, source = Path(temporary).resolve(), Path(original).resolve()
            output = outside / 'output'
            def marker(path):
                return b'original native publication process evidence: ' + str(path).encode() + b'\n'
            self.assertIsNone(retain(output, 'none', b'no marker', b''))
            invalid = outside / 'chrono-native-publication-failure-outside'
            invalid.mkdir()
            self.assertEqual(retain(output, 'outside', marker(invalid), b'')['status'], 'unavailable')
            leaf = outside / 'chrono-native-publication-failure-link'
            leaf.symlink_to(source, target_is_directory=True)
            self.assertEqual(retain(output, 'link', marker(leaf), b'')['status'], 'unavailable')
            (source / 'stdout').write_bytes(b'original\xff')
            (source / 'stderr').write_bytes(b'original\xfe')
            (source / 'process.json').write_text('{"exit_code":73}')
            preparation = source / 'state/collection/preparation'
            preparation.mkdir(parents=True)
            with (preparation / 'oversized').open('wb') as stream:
                stream.truncate(bound + 1)
            secret = outside / 'not-evidence'
            secret.write_bytes(b'not an original stream')
            (preparation / 'linked').symlink_to(secret)
            result = retain(output, 'bounded', b'', marker(source))
            self.assertEqual(result['status'], 'partial')
            self.assertEqual(result['bound_bytes'], bound)
            self.assertEqual(bound, 67108864)
            self.assertLessEqual(result['bytes'], bound)
            rows = {row['source']: row for row in result['files']}
            self.assertEqual(rows['state/collection/preparation/oversized']['reason'], 'bounded evidence limit')
            self.assertEqual(rows['state/collection/preparation/linked']['status'], 'omitted')
            for name in ['linked', 'oversized']:
                self.assertFalse((output / ('failure-evidence/bounded/state/collection/preparation/' + name)).exists())
            for name in ['stdout', 'stderr', 'process.json']:
                raw = (source / name).read_bytes()
                self.assertEqual((output / rows[name]['path']).read_bytes(), raw)
                self.assertEqual(rows[name]['sha256'], digest(raw))
            self.assertEqual(result['bytes'], sum(row.get('bytes', 0) for row in rows.values() if row['status'] == 'retained'))

    def test_release_recipe_retains_install_build_and_package_failures(self):
        for fault, phase in [('install', 'toolchain-install'), ('build', 'build'), ('pack', 'package')]:
            with self.subTest(fault=fault):
                f = self.fixture()
                self.assertEqual(f.run(fault).returncode, 41)
                report = self.evidence(f)
                self.assertEqual(report['status'], 'failed')
                self.assertEqual(report['failure']['phase'], phase)
                self.assertEqual(len(report['processes']), len(f.calls()))
                last = report['processes'][-1]
                self.assertEqual(last['phase'], phase)
                self.assertEqual(last['exit_code'], 41)
                self.assertEqual(bytes(last['stdout_bytes']), b'failed phase\xff\n')
                self.assertEqual(bytes(last['stderr_bytes']), b'original diagnostic\xfe\n')
                self.assertFalse((f.output / 'release.json').exists())

    def test_release_recipe_semantic_failure_keeps_successful_process_exit(self):
        f = self.fixture()
        self.assertEqual(f.run('version').returncode, 2)
        report = self.evidence(f)
        self.assertEqual(report['status'], 'failed')
        self.assertEqual(report['failure']['phase'], 'compiler-version')
        self.assertIsNone(report['failure']['exit_code'])
        self.assertEqual(report['processes'][-1]['exit_code'], 0)
        self.assertEqual(bytes(report['processes'][-1]['stdout_bytes']), b'rustc different-version\n')
        self.assertFalse(any(p['phase'] in ['build', 'package'] for p in report['processes']))

    def test_release_recipe_missing_program_and_existing_output_preserve_real_state(self):
        f = self.fixture()
        (f.root / 'crates/distribution/target/release/chrono-distribution').unlink()
        self.assertEqual(f.run().returncode, 2)
        report = self.evidence(f)
        self.assertEqual(report['status'], 'failed')
        self.assertEqual(report['failure']['phase'], 'package')
        self.assertIsNone(report['failure']['exit_code'])
        self.assertFalse(any(p['phase'] == 'package' for p in report['processes']))
        f = self.fixture()
        f.output.mkdir()
        (f.output / 'build.json').write_text('preexisting evidence')
        self.assertEqual(f.run().returncode, 2)
        self.assertEqual(f.calls(), [])
        self.assertEqual((f.output / 'build.json').read_text(), 'preexisting evidence')
        f = self.fixture()
        f.output.symlink_to(f.root / 'absent destination')
        self.assertEqual(f.run().returncode, 2)
        self.assertEqual(f.calls(), [])
        self.assertTrue(f.output.is_symlink())
        self.assertFalse((f.root / 'absent destination').exists())

    def test_release_recipe_retains_source_failure_without_inventing_identity(self):
        f = self.fixture()
        self.assertEqual(f.run('source').returncode, 41)
        report = json.loads((f.output / 'build.json').read_bytes())
        self.assertEqual(report['schema'], 'chrono-native-build/v3')
        self.assertEqual(report['status'], 'failed')
        self.assertIsNone(report['source_commit'])
        self.assertEqual(len(f.calls()), 1)
        self.assertEqual(len(report['processes']), 1)
        process = report['processes'][0]
        self.assertEqual(process['phase'], 'source-identity')
        self.assertEqual(process['exit_code'], 41)
        self.assertEqual(bytes(process['stdout_bytes']), b'failed phase\xff\n')
        self.assertEqual(process['stdout_sha256'], digest(b'failed phase\xff\n'))

    def test_release_consumers_execute_staged_release_bytes_with_observed_identities(self):
        f = self.fixture(staged=True)
        (f.root / 'consumer/bin').mkdir(parents=True)
        (f.root / CONSUMER).write_text('old debug executable')
        out = f.run()
        self.assertEqual(out.returncode, 0, out.stderr)
        self.assertIn(b'actual release consumer', out.stdout)
        self.assertEqual((f.root / CONSUMER).read_bytes(), RELEASE_BYTES)
        report = self.evidence(f, staged=True)
        self.assertEqual(report['status'], 'passed')
        observations = report['consumer_staging']['observations']
        for phase in ['staged', 'before-verification', 'after-verification', 'before-package', 'after-package']:
            snapshot = next(s for s in observations if s['phase'] == phase)
            self.assertIs(snapshot['matches'], True)
            for path in [RELEASE, CONSUMER]:
                file = next(f for f in snapshot['files'] if f['path'] == path)
                self.assertEqual(file['sha256'], digest(RELEASE_BYTES))
                self.assertEqual(file['size'], len(RELEASE_BYTES))
                self.assertEqual(file['mode'], 0o755)

    def test_release_consumers_reject_changed_source_destination_mode_or_absence(self):
        for fault in ['source-change', 'destination-change', 'destination-missing', 'destination-mode']:
            with self.subTest(fault=fault):
                f = self.fixture(staged=True)
                self.assertEqual(f.run(fault).returncode, 2)
                report = self.evidence(f, staged=True)
                self.assertEqual(report['status'], 'failed')
                self.assertTrue(any(s['phase'] == 'after-verification' and s['matches'] is False for s in report['consumer_staging']['observations']))
                self.assertFalse(any(c['tool'] == 'chrono-distribution' for c in f.calls()))
                self.assertEqual(report['processes'][-1]['exit_code'], 0)

    def test_release_consumers_retain_child_failure_and_changed_identity_together(self):
        f = self.fixture(staged=True)
        self.assertEqual(f.run('destination-change-failure').returncode, 73)
        report = self.evidence(f, staged=True)
        self.assertEqual(report['failure']['exit_code'], 73)
        self.assertEqual(report['failure']['phase'], 'verification')
        self.assertTrue(any(s['phase'] == 'after-verification' and s['matches'] is False for s in report['consumer_staging']['observations']))
        self.assertFalse(any(c['tool'] == 'chrono-distribution' for c in f.calls()))

    def test_release_consumers_reject_invalid_or_overlapping_registration_before_builds(self):
        for case in range(11):
            with self.subTest(case=case):
                f = self.fixture(staged=True)
                if case == 0:
                    f.edit(BUILD, lambda v: v.update(schema='chrono-release-build/v2'))
                elif case == 1:
                    f.edit(BUILD, lambda v: v['consumer_staging']['bindings'][0].update(asset='unknown'))
                elif case == 2:
                    f.edit(BUILD, lambda v: v['consumer_staging']['bindings'][0].update(destinations=['undeclared/tool']))
                elif case == 3:
                    f.edit(HOST, lambda v: v['artifacts'][0].update(tracked=True))
                elif case == 4:
                    f.edit(BUILD, lambda v: v['consumer_staging']['bindings'][0].update(destinations=[CONSUMER, CONSUMER]))
                elif case == 5:
                    f.edit(BUILD, lambda v: v['consumer_staging']['bindings'][0].update(destinations=[CONSUMER, 'consumer/bin/tool/child']))
                elif case == 6:
                    f.edit(PLAN, lambda v: v['assets'].update(other=CONSUMER))
                elif case == 7:
                    f.edit(BUILD, lambda v: v['consumer_staging']['bindings'][0].update(destinations=['consumer/../source']))
                elif case == 9:
                    f.edit(BUILD, lambda v: v.update(rust_components=['rustfmt', 'rustfmt']))
                elif case == 10:
                    f.edit(BUILD, lambda v: v.update(rust_components='rustfmt'))
                else:
                    (f.root / 'elsewhere').mkdir()
                    (f.root / 'consumer').symlink_to('elsewhere')
                self.assertEqual(f.run().returncode, 2)
                self.assertEqual(f.calls(), [])
                self.assertFalse(f.output.exists())

    def test_release_consumers_refuse_tracked_destinations_and_missing_release_before_execution(self):
        for fault in ['tracked', 'missing']:
            with self.subTest(fault=fault):
                f = self.fixture(staged=True)
                if fault == 'missing':
                    (f.root / RELEASE).unlink()
                self.assertEqual(f.run(fault).returncode, 2)
                self.assertEqual(self.evidence(f, staged=True)['status'], 'failed')
                self.assertFalse(any(c['tool'] in ['probe', 'chrono-distribution'] for c in f.calls()))
                self.assertFalse((f.root / CONSUMER).exists())
                if fault == 'tracked':
                    self.assertFalse(any(c['tool'] in ['cargo', 'rustup'] for c in f.calls()))

    def test_release_consumers_tracking_treats_registered_paths_as_literal_names(self):
        f = self.fixture(staged=True)
        git = shutil.which('git')
        self.assertIsNotNone(git)
        def run(args):
            out = subprocess.run([git] + args, cwd=f.root, capture_output=True)
            self.assertEqual(out.returncode, 0, out.stderr)
        run(['init', '-q'])
        (f.root / 'consumer/bin').mkdir(parents=True)
        (f.root / 'consumer/bin/other').write_text('preserved source')
        run(['add', '--', 'consumer/bin/other'])
        run(['-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid',
             'commit', '-qm', 'tracked neighbor'])
        def binding(v):
            v['tools']['git'] = git
            v['consumer_staging']['bindings'][0]['destinations'] = ['consumer/bin/*']
        f.edit(BUILD, binding)
        f.edit(PROJECTS, lambda v: v['scripts'][0]['actions']['execute']['argv'].__setitem__(1, 'consumer/bin/*'))
        out = f.run()
        self.assertEqual(out.returncode, 0, out.stderr)
        self.assertEqual((f.root / 'consumer/bin/*').read_bytes(), RELEASE_BYTES)
        self.assertEqual((f.root / 'consumer/bin/other').read_text(), 'preserved source')
        self.assertEqual(self.evidence(f, staged=True)['status'], 'passed')

    def test_release_consumers_observe_failed_pack_without_masking_its_exit(self):
        f = self.fixture(staged=True)
        self.assertEqual(f.run('pack').returncode, 41)
        report = self.evidence(f, staged=True)
        self.assertEqual(report['status'], 'failed')
        self.assertEqual(report['failure']['phase'], 'package')
        self.assertEqual(report['failure']['exit_code'], 41)
        last = report['consumer_staging']['observations'][-1]
        self.assertEqual(last['phase'], 'after-package')
        self.assertIs(last['matches'], False)
        process = report['processes'][-1]
        self.assertEqual(process['phase'], 'package')
        self.assertEqual(process['exit_code'], 41)
        self.assertEqual(bytes(process['stderr_bytes']), b'original diagnostic\xfe\n')


if __name__ == '__main__':
    unittest.main()
