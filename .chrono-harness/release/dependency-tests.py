#!/usr/bin/env python3
"""Release recipe contracts, verified by its paired Python test script."""
from copy import deepcopy
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import threading
import unittest
from types import SimpleNamespace

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location('release_build', HERE / 'build.py')
RECIPE = importlib.util.module_from_spec(SPEC)
exec(compile((HERE / 'build.py').read_bytes(), str(HERE / 'build.py'), 'exec'), RECIPE.__dict__)


class RecipeTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix='release recipe λ ')
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.plan = {'schema': 'chrono-release-plan/v1', 'version': 'fixture',
                     'assets': {'a': 'out/a', 'b': 'out/b'}}
        self.staging = {'release_plan': 'plan.json', 'host_config': 'host.json',
                        'bindings': [{'asset': name, 'source': 'out/' + name,
                                      'destination': 'bin/' + name} for name in ['a', 'b']]}
        self.cfg = {'schema': 'chrono-release-build/v5', 'manifests': ['a/Cargo.toml', 'b/Cargo.toml'],
                    'verification_operations': ['test.none', 'test.a'], 'local_workers': 2,
                    'limits': {'unit_seconds': 30, 'platform_seconds': 120},
                    'units': [
                        {'id': 'build_a', 'kind': 'build', 'manifest': 'a/Cargo.toml', 'asset': 'a', 'needs': [], 'handoff': 'handoff/a'},
                        {'id': 'build_b', 'kind': 'build', 'manifest': 'b/Cargo.toml', 'asset': 'b', 'needs': [], 'handoff': 'handoff/b'},
                        {'id': 'test_none', 'kind': 'verify', 'operation': 'test.none', 'needs': [], 'handoff': 'handoff/none', 'rust_toolchain': False},
                        {'id': 'test_a', 'kind': 'verify', 'operation': 'test.a', 'needs': ['build_a'], 'handoff': 'handoff/test_a', 'rust_toolchain': False}],
                    'collector': {'id': 'collect', 'needs': ['build_a', 'build_b', 'test_none', 'test_a'],
                                  'package_asset': 'a', 'dependency_metadata': 'handoff/metadata.json'}}
        ids = self.cfg['collector']['needs'] + ['collect']
        self.cfg['unit_costs'] = dict.fromkeys(ids, 'unmeasured')
        self.cfg['native_jobs'] = {'fixture': {name: name for name in ids}}
        self.write('plan.json', self.plan)

    def write(self, name, value):
        file = self.root / name
        file.parent.mkdir(parents=True, exist_ok=True)
        file.write_text(json.dumps(value))

    def plan_units(self):
        return RECIPE.units_plan(self.root, self.cfg, [], self.staging)

    def test_v5_allows_empty_and_subset_dependencies(self):
        _, builds, tests = self.plan_units()
        self.assertEqual(set(builds), {'build_a', 'build_b'})
        self.assertEqual(tests['test_none']['needs'], [])
        self.assertEqual(tests['test_a']['needs'], ['build_a'])

    def test_consumer_contract_rejects_added_or_missing_binary_dependencies(self):
        self.cfg['verification_consumers'] = {
            'test_none': {'need': 'Exercise source library without release executables.', 'release_assets': []},
            'test_a': {'need': 'Run asset a through its external command interface.', 'release_assets': ['a']},
        }
        self.plan_units()
        for index, needs in [(2, ['build_a']), (3, []), (3, ['build_a', 'build_b'])]:
            with self.subTest(index=index, needs=needs):
                original = self.cfg['units'][index]['needs']
                self.cfg['units'][index]['needs'] = needs
                try:
                    with self.assertRaisesRegex(ValueError, 'E_RELEASE_CONSUMPTION'):
                        self.plan_units()
                finally:
                    self.cfg['units'][index]['needs'] = original

    def test_consumer_contract_requires_known_unique_assets_and_a_concrete_need(self):
        bad = [None, [], {'unknown': {'need': 'test', 'release_assets': []}},
               {'test_none': {'need': '', 'release_assets': []}},
               {'test_none': {'need': 'test', 'release_assets': ['missing']}},
               {'test_none': {'need': 'test', 'release_assets': ['a', 'a']}},
               {'test_none': {'need': 'test', 'release_assets': 'a'}},
               {'test_none': {'need': 'test', 'release_assets': [], 'extra': True}}]
        for contract in bad:
            with self.subTest(contract=contract):
                self.cfg['verification_consumers'] = contract
                with self.assertRaisesRegex(ValueError, 'E_RELEASE_CONSUMPTION'):
                    self.plan_units()

    def test_registered_library_consumers_reject_the_complete_build_vector(self):
        root = HERE.parent.parent
        cfg, _, operations, staging = RECIPE.preflight(root, {'git'}, set())
        complete = [u['id'] for u in cfg['units'] if u['kind'] == 'build']
        checked = []
        for name, contract in cfg['verification_consumers'].items():
            if contract['release_assets']:
                continue
            changed = deepcopy(cfg)
            next(u for u in changed['units'] if u['id'] == name)['needs'] = complete
            with self.subTest(unit=name):
                with self.assertRaisesRegex(ValueError, 'E_RELEASE_CONSUMPTION'):
                    RECIPE.units_plan(root, changed, operations, staging)
            checked.append(name)
        self.assertTrue(checked, 'registered real library consumers must exercise the regression')

    def test_missing_consumer_explanation_is_visible_without_claiming_unnecessary(self):
        plan, builds, tests = self.plan_units()
        rows = RECIPE.dependency_contracts(self.cfg, plan, builds, tests)
        self.assertEqual(rows['test_a']['status'], 'unverified')
        self.assertEqual(rows['test_a']['findings'], ['W_RELEASE_CONSUMPTION_UNVERIFIED'])
        self.assertEqual(rows['test_a']['release_assets'], ['a'])
        self.assertEqual(rows['test_none']['findings'], [])
        self.assertIsNone(rows['test_a']['need'])

    def test_failure_evidence_requires_explicit_owned_nonoverlapping_directories(self):
        self.write('host.json', {'artifacts': [{'path': 'state/', 'tracked': False}]})
        self.cfg['failure_evidence'] = {'test.none': ['state/nested/']}
        self.plan_units()
        for value in [None, [], {'missing': ['state/']}, {'test.none': 'state/'},
                      {'test.none': ['state']}, {'test.none': ['source/']},
                      {'test.none': ['state/', 'state/nested/']},
                      {'test.none': ['../state/']}, {'test.none': ['state/', 'state/']}]:
            with self.subTest(value=value):
                self.cfg['failure_evidence'] = value
                with self.assertRaisesRegex(ValueError, 'failure evidence'):
                    self.plan_units()
        self.cfg['failure_evidence'] = {'test.none': ['state/nested/']}
        self.write('host.json', {'artifacts': [{'path': 'state/', 'tracked': True}]})
        with self.assertRaisesRegex(ValueError, 'failure evidence'):
            self.plan_units()

    def test_failure_evidence_does_not_reinterpret_v4(self):
        self.cfg['schema'] = 'chrono-release-build/v4'
        self.cfg['failure_evidence'] = {}
        for unit in self.cfg['units'][2:]:
            del unit['rust_toolchain']
            unit['needs'] = ['build_a', 'build_b']
        with self.assertRaisesRegex(ValueError, 'failure evidence'):
            self.plan_units()

    def test_registered_recipe_has_a_valid_explicit_plan(self):
        root = HERE.parent.parent
        cfg, _, operations, staging = RECIPE.preflight(root, {'git'}, set())
        RECIPE.units_plan(root, cfg, operations, staging)

    def test_v4_keeps_complete_vector_requirement(self):
        self.cfg['schema'] = 'chrono-release-build/v4'
        for unit in self.cfg['units'][2:]:
            del unit['rust_toolchain']
        with self.assertRaisesRegex(ValueError, 'complete build vector'):
            self.plan_units()
        for unit in self.cfg['units'][2:]:
            unit['needs'] = ['build_b', 'build_a']
        self.plan_units()

    def test_v5_requires_an_explicit_boolean_toolchain_choice(self):
        unit = self.cfg['units'][2]
        for value in [None, 0, 1, 'rust', [], {}]:
            with self.subTest(value=value):
                unit['rust_toolchain'] = value
                with self.assertRaisesRegex(ValueError, 'explicit boolean rust_toolchain'):
                    self.plan_units()
        del unit['rust_toolchain']
        with self.assertRaisesRegex(ValueError, 'explicit boolean rust_toolchain'):
            self.plan_units()

    def test_v5_rejects_duplicate_unknown_and_non_build_dependencies(self):
        for needs in [['build_a', 'build_a'], ['missing'], ['test_none'], 'build_a', [None]]:
            with self.subTest(needs=needs):
                self.cfg['units'][3]['needs'] = needs
                with self.assertRaisesRegex(ValueError, 'unique declared build'):
                    self.plan_units()

    def test_collector_must_still_require_every_unit(self):
        self.cfg['collector']['needs'].remove('test_none')
        with self.assertRaisesRegex(ValueError, 'exact build/test membership'):
            self.plan_units()

    def test_empty_staging_does_not_query_all_git_files(self):
        calls = []
        execution = SimpleNamespace(root=self.root, tools={'git': 'git'}, report={},
                                    run=lambda *args: calls.append(args))
        selected = RECIPE.selected_staging(self.staging, {})
        RECIPE.check_staging(execution, selected)
        self.assertEqual(calls, [])
        self.assertEqual(selected['observations'], [{'phase': 'staged', 'operation': None, 'matches': True, 'files': []}])
        self.assertNotIn('observations', self.staging)

    def test_collection_rejects_extra_asset_and_lineage_substitution(self):
        _, builds, tests = self.plan_units()
        identities = {n: {'path': 'out/' + n, 'sha256': n * 64, 'size': 1, 'mode': 493, 'error': None} for n in ['a', 'b']}
        assets = {n: {'identity': identities[n], 'selection': {'job': 'build_' + n, 'artifact_id': n}} for n in ['a', 'b']}
        for row in self.staging['bindings']:
            row['expected'] = identities[row['asset']]
        files = [dict(identities['a'], path=p) for p in ['out/a', 'bin/a']]
        receipt = {'assets': {'a': assets['a']}, 'selected': {'build_a': assets['a']['selection']},
                   'consumer_staging': {'observations': [
                       {'phase': phase, 'matches': True, 'files': deepcopy(files)}
                       for phase in ['staged', 'before-verification', 'after-verification']]}}
        RECIPE.verify_consumer(receipt, tests['test_a'], builds, assets, self.staging)
        mutations = [
            lambda r: r['assets'].update(b=assets['b']),
            lambda r: r['selected']['build_a'].update(artifact_id='other'),
            lambda r: r['consumer_staging']['observations'][2]['files'][0].update(sha256='0' * 64),
            lambda r: r['consumer_staging']['observations'][1]['files'].append(files[0]),
            lambda r: r['consumer_staging']['observations'].pop()]
        for mutate in mutations:
            altered = deepcopy(receipt)
            mutate(altered)
            with self.assertRaises(ValueError):
                RECIPE.verify_consumer(altered, tests['test_a'], builds, assets, self.staging)

    def test_zero_dependency_receipt_cannot_claim_a_selected_producer(self):
        _, builds, tests = self.plan_units()
        receipt = {'assets': {}, 'selected': {}, 'consumer_staging': {'observations': [
            {'phase': phase, 'matches': True, 'files': []}
            for phase in ['staged', 'before-verification', 'after-verification']]}}
        RECIPE.verify_consumer(receipt, tests['test_none'], builds, {}, self.staging)
        receipt['selected']['build_a'] = {}
        with self.assertRaisesRegex(ValueError, 'lineage changed'):
            RECIPE.verify_consumer(receipt, tests['test_none'], builds, {}, self.staging)

    def test_ready_units_do_not_wait_for_unrelated_builds(self):
        # The blocking build can complete only after both independent verifiers run.
        release = threading.Event()
        build_started = threading.Event()
        lock = threading.Lock()
        seen, active, peak = set(), 0, 0
        outcomes = {}
        def invoke(name, dependencies):
            nonlocal active, peak
            with lock:
                active += 1
                peak = max(peak, active)
                seen.add(name)
            try:
                if name == 'build_a':
                    self.assertTrue(build_started.wait(5), 'independent builds did not overlap')
                if name == 'build_b':
                    build_started.set()
                    self.assertTrue(release.wait(5), 'unrelated build blocked ready verification')
                if name == 'test_none':
                    self.assertEqual(dependencies, {})
                if name == 'test_a':
                    self.assertEqual(set(dependencies), {'build_a'})
                    self.assertEqual(dependencies['build_a']['result'], 'success')
                    self.assertIn('test_none', seen)
                    release.set()
                return name, {'result': 'success', 'outputs': {'artifact_id': name}}
            finally:
                with lock:
                    active -= 1
        RECIPE.run_local_units(self.cfg, invoke, outcomes)
        self.assertEqual(set(outcomes), set(self.cfg['collector']['needs']))
        self.assertEqual(peak, 2)

    def test_failed_dependency_is_preserved_and_does_not_block_independent_test(self):
        outcomes = {}
        received = {}
        def invoke(name, dependencies):
            received[name] = dependencies
            return name, {'result': 'failure' if name == 'build_a' else 'success', 'outputs': {}}
        RECIPE.run_local_units(self.cfg, invoke, outcomes)
        self.assertEqual(received['test_a'], {'build_a': {'result': 'failure', 'outputs': {}}})
        self.assertEqual(received['test_none'], {})
        self.assertEqual(outcomes['test_none']['result'], 'success')

    def create_host(self):
        for name in ['cargo', 'rustc', 'rustup', 'verifier']:
            target = self.root / 'tools' / name
            target.parent.mkdir(exist_ok=True)
            shutil.copy2(HERE / 'fixtures/tool.py', target)
            target.chmod(0o755)
        target = self.root / '.chrono-harness/release/build.py'
        target.parent.mkdir(parents=True)
        shutil.copy2(HERE / 'build.py', target)
        self.cfg.update(rust_toolchain='1.94.0', rust_components=[], projects='projects.json',
                        tools={n: 'tools/' + n for n in ['cargo', 'rustc', 'rustup']},
                        consumer_staging={'release_plan': 'plan.json', 'host_config': 'host.json',
                                          'bindings': [{'asset': n, 'destinations': ['bin/' + n]} for n in ['a', 'b']]})
        self.cfg['tools']['git'] = 'git'
        self.cfg['tools']['python3'] = sys.executable
        self.write('.chrono-harness/release/build.json', self.cfg)
        self.write('host.json', {'artifacts': [{'path': 'bin/', 'tracked': False}]})
        self.write('projects.json', {'projects': [], 'scripts': [
            {'actions': {n: {'operation': 'test.' + n, 'tool': 'python3', 'argv': ['tools/verifier', 'verify']} for n in ['none', 'a']}}]})
        for manifest in self.cfg['manifests']:
            file = self.root / manifest
            file.parent.mkdir()
            file.write_text('fixture manifest')
        (self.root / '.gitignore').write_text('out/\nbin/\nhandoff/\n')
        for argv in [['init', '-q'], ['add', '.'], ['-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid', 'commit', '-qm', 'fixture']]:
            subprocess.run(['git', *argv], cwd=self.root, check=True, capture_output=True)

    def invoke_unit(self, name, dependencies, extra_environment=None):
        output = self.root.parent / (self.root.name + '-' + name)
        self.addCleanup(lambda: shutil.rmtree(output) if output.exists() else None)
        env = dict(os.environ, PYTHONDONTWRITEBYTECODE='1', CHRONO_RELEASE_LOCAL='1',
                   CHRONO_RELEASE_RUN='fixture', CHRONO_RELEASE_ATTEMPT='1', CHRONO_RELEASE_JOB=name,
                   CHRONO_RELEASE_DEPENDENCIES=json.dumps(dependencies))
        env.update(extra_environment or {})
        scope = ['--collect'] if name == 'collect' else ['--unit', name]
        result = subprocess.run([sys.executable, '-B', str(HERE / 'build.py'), str(self.root), str(output), *scope],
                                env=env, capture_output=True, timeout=30)
        self.assertEqual(result.returncode, 0, result.stderr.decode(errors='replace'))
        receipt = json.loads((output / 'receipt.json').read_text())
        self.assertEqual(receipt['status'], 'passed')
        return output, receipt

    def test_real_scopes_and_collector_with_subset_and_empty_dependencies(self):
        self.cfg['verification_consumers'] = {
            'test_none': {'need': 'Run verifier without a released executable.', 'release_assets': []},
        }
        self.create_host()
        selections = {}
        receipts = {}
        for name in ['test_none', 'build_a', 'test_a', 'build_b']:
            unit = next(u for u in self.cfg['units'] if u['id'] == name)
            deps = {need: selections[need] for need in unit['needs']}
            output, receipt = self.invoke_unit(name, deps)
            receipts[name] = receipt
            selections[name] = {'result': 'success', 'outputs': {'artifact_id': RECIPE.sha_file(output / 'receipt.json'), 'run_id': 'fixture', 'attempt': '1'}}
            shutil.copytree(output, self.root / unit['handoff'])
        self.write(self.cfg['collector']['dependency_metadata'], selections)
        output, receipt = self.invoke_unit('collect', selections)
        self.assertEqual(receipts['test_none']['assets'], {})
        self.assertEqual(receipts['test_none']['dependency_contracts']['test_none']['status'], 'declared')
        self.assertEqual(receipts['test_a']['dependency_contracts']['test_a']['findings'], ['W_RELEASE_CONSUMPTION_UNVERIFIED'])
        self.assertEqual(receipt['dependency_contracts']['test_a']['status'], 'unverified')
        self.assertEqual(set(receipts['test_a']['assets']), {'a'})
        self.assertEqual(set(receipt['verifications']), {'test_none', 'test_a'})
        for name in ['test_none', 'test_a']:
            self.assertNotIn('toolchain', receipts[name])
            self.assertEqual(set(receipts[name]['execution_tools']), {'git', 'python3'})
            self.assertFalse(any(p['phase'] in {'toolchain-install', 'compiler-version', 'compiler-executable'} for p in receipts[name]['processes']))
        self.assertEqual(receipt['platform_started_ns'], receipts['test_none']['started_ns'])
        self.assertTrue((output / 'release.json').is_file())
        for name in self.cfg['collector']['needs']:
            self.assertTrue((output / 'originals' / name / 'receipt.json').is_file())

    def test_no_toolchain_verifier_runs_with_unavailable_compiler_bindings(self):
        self.create_host()
        for name in ['cargo', 'rustc', 'rustup']:
            self.cfg['tools'][name] = 'missing/' + name
        self.write('.chrono-harness/release/build.json', self.cfg)
        subprocess.run(['git', 'add', '.'], cwd=self.root, check=True, capture_output=True)
        subprocess.run(['git', '-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid', 'commit', '-qm', 'explicit unavailable compiler bindings'], cwd=self.root, check=True, capture_output=True)
        environment = {'RUSTUP_TOOLCHAIN': 'foreign-toolchain-choice', 'CARGO_PROFILE_RELEASE_STRIP': 'false'}
        _, receipt = self.invoke_unit('test_none', {}, environment)
        self.assertEqual(set(receipt['execution_tools']), {'git', 'python3'})
        self.assertNotIn('toolchain', receipt)
        for key, value in environment.items():
            self.assertEqual(receipt['environment'][key], value)
        RECIPE.verify_toolchain(receipt, self.cfg)
        for mutate in [
                lambda r: r.update(toolchain={}),
                lambda r: r['execution_tools'].pop('python3'),
                lambda r: r['execution_tools']['python3'].update(sha256='invalid'),
                lambda r: r['processes'].append({'phase': 'toolchain-install'})]:
            altered = deepcopy(receipt)
            mutate(altered)
            with self.assertRaises(ValueError):
                RECIPE.verify_toolchain(altered, self.cfg)

    def test_explicit_rust_toolchain_is_observed_without_asset_dependencies(self):
        self.cfg['units'][2]['rust_toolchain'] = True
        self.create_host()
        _, receipt = self.invoke_unit('test_none', {})
        self.assertEqual(receipt['assets'], {})
        self.assertEqual(set(receipt['toolchain']['tools']), {'git', 'python3', 'cargo', 'rustc', 'rustup'})
        self.assertEqual(receipt['toolchain']['versions']['rustc'], 'rustc 1.94.0 (fixture)')
        self.assertEqual(len([p for p in receipt['processes'] if p['phase'] == 'toolchain-install']), 1)
        RECIPE.verify_toolchain(receipt, self.cfg)


if __name__ == '__main__':
    unittest.main()
