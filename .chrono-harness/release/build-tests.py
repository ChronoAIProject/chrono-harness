#!/usr/bin/env python3
"""Independent behavioral fixtures for the registered release recipe owner."""
import hashlib
import importlib.util
import io
import contextlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import tempfile
import unittest

SCRIPT = Path(__file__).with_name('build.py')
ROOT = SCRIPT.parent.parent.parent
BUILD = '.chrono-harness/release/build.json'
PLAN = '.chrono-harness/release/plan.json'

def sha(p):
    return hashlib.sha256(p.read_bytes()).hexdigest()

def put(p, value):
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(json.dumps(value, indent=2) + '\n')

def retain_failed_fixture(test, directory):
    destination=os.environ.get('CHRONO_TEST_FAILURE_DIRECTORY')
    result=test._outcome.result
    failed=any(case is test and error is not None for case,error in getattr(test._outcome,'errors',[]))
    failed=failed or any(case is test for case,_ in result.failures+result.errors)
    if destination and failed:
        target=Path(destination)/test.id()
        target.parent.mkdir(parents=True,exist_ok=True)
        shutil.copytree(directory,target,symlinks=True)
        print('Original failed fixture: '+str(target),file=sys.stderr)

TOOLS = r'''#!/usr/bin/env python3
import json, os, sys, shutil, subprocess
from pathlib import Path
name=Path(sys.argv[0]).name
with open(os.environ['CALLS'],'a') as f:f.write(json.dumps({'tool':name,'argv':sys.argv[1:],'cwd':str(Path.cwd())})+'\n')
fault=os.environ.get('FAULT','')
if name=='git':
 if 'clone' in sys.argv:
  shutil.copytree(sys.argv[-2],sys.argv[-1],ignore=shutil.ignore_patterns('handoffs','products','consumer','__pycache__'))
 elif 'checkout' in sys.argv:pass
 elif 'status' in sys.argv:
  if fault=='dirty':print(' M tracked')
 elif 'ls-files' in sys.argv:
  if fault=='tracked':sys.stdout.buffer.write(b'consumer/a\0')
 else:print(('b' if 'HEAD^{tree}' in sys.argv else 'a')*40)
elif name=='rustup' and sys.argv[1:2]==['which']:print(Path.cwd()/'tools'/sys.argv[-1])
elif name=='rustc':print('rustc 1.95.0 fixture')
elif name=='cargo':
 if '--version' in sys.argv:print('cargo fixture')
 elif sys.argv[1]=='build':
  if fault=='build':sys.stderr.buffer.write(b'build failure\xfe');sys.exit(41)
  cfg=json.loads(Path('.chrono-harness/release/build.json').read_text())
  plan=json.loads(Path('.chrono-harness/release/plan.json').read_text())
  u=next(u for u in cfg['units'] if u.get('manifest')==sys.argv[-1])
  target=Path(plan['assets'][u['asset']]);target.parent.mkdir(parents=True,exist_ok=True)
  target.write_bytes(Path('producer-'+u['asset']).read_bytes());target.chmod(0o755)
elif name=='probe':
 for p in ['consumer/a','consumer/distribution']:assert Path(p).stat().st_mode&0o111
 subprocess.run(['consumer/a'],check=True)
 if fault=='destination-change':Path('consumer/a').write_bytes(b'changed')
 if fault=='destination-mode':Path('consumer/a').chmod(0o644)
 if fault=='destination-missing':Path('consumer/a').unlink()
 if fault=='source-change':Path('products/a').write_bytes(b'changed')
 sys.stdout.buffer.write(b'original verification\xff\n');sys.stderr.buffer.write(b'original diagnostic\xfe\n')
 if fault=='verify':sys.exit(73)
'''
PACK = r'''#!/usr/bin/env python3
import hashlib,json,os,platform,shutil,sys
from pathlib import Path
with open(os.environ['CALLS'],'a') as f:f.write(json.dumps({'tool':'package','argv':sys.argv[1:],'cwd':str(Path.cwd())})+'\n')
if os.environ.get('FAULT')=='pack':sys.stderr.buffer.write(b'pack failure\xfe');sys.exit(41)
p=Path(sys.argv[sys.argv.index('--output')+1]);assert not p.exists();p.mkdir()
plan=json.loads(Path(sys.argv[sys.argv.index('--plan')+1]).read_text())
platform0={'Darwin':'macos','Linux':'linux'}[platform.system()]+'-'+{'arm64':'aarch64','aarch64':'aarch64','x86_64':'x86_64'}[platform.machine()]
assets={}
for name,source in plan['assets'].items():
 file=name+'-'+platform0;shutil.copy2(source,p/file);raw=(p/file).read_bytes()
 assets[name]={'file':file,'size':len(raw),'sha256':hashlib.sha256(raw).hexdigest()}
(p/'release.json').write_text(json.dumps({'schema':'chrono-release/v1','version':plan['version'],'source_commit':'a'*40,'source_tree':'b'*40,'platforms':{platform0:assets}}))
if os.environ.get('FAULT')=='package-extra':(p/'undeclared').write_bytes(b'wrong')
if os.environ.get('FAULT')=='package-mode':(p/next(iter(assets.values()))['file']).chmod(0o644)
if os.environ.get('FAULT')=='package-change':(p/next(iter(assets.values()))['file']).write_bytes(b'changed')
'''

class Fixture:
    def __init__(self):
        self.temp=tempfile.TemporaryDirectory(prefix='release unit λ ')
        self.root=Path(self.temp.name)/'source with spaces';self.root.mkdir()
        self.calls=Path(self.temp.name)/'calls.jsonl'
        self.cfg={'schema':'chrono-release-build/v4','rust_toolchain':'1.95.0','rust_components':[],
                  'manifests':['registered/a.toml','registered/distribution.toml'],
                  'projects':'registered/projects.json','tools':{n:'tools/'+n for n in ['git','cargo','rustc','rustup','probe']},
                  'verification_operations':['original.first','original.second'],
                  'consumer_staging':{'release_plan':PLAN,'host_config':'registered/host.json','bindings':[{'asset':'a','destinations':['consumer/a']},{'asset':'distribution','destinations':['consumer/distribution']}]},
                  'units':[{'id':'build_a','kind':'build','manifest':'registered/a.toml','asset':'a','needs':[],'handoff':'handoffs/build_a'},
                           {'id':'build_distribution','kind':'build','manifest':'registered/distribution.toml','asset':'distribution','needs':[],'handoff':'handoffs/build_distribution'},
                           {'id':'verify_first','kind':'verify','operation':'original.first','needs':['build_a','build_distribution'],'handoff':'handoffs/verify_first'},
                           {'id':'verify_second','kind':'verify','operation':'original.second','needs':['build_a','build_distribution'],'handoff':'handoffs/verify_second'}],
                  'collector':{'id':'package','needs':['build_a','build_distribution','verify_first','verify_second'],'package_asset':'distribution','dependency_metadata':'dependencies.json'},
                  'unit_costs':{n:'unmeasured' for n in ['build_a','build_distribution','verify_first','verify_second','package']},'limits':{'unit_seconds':2700,'platform_seconds':2700},
                  'native_jobs':{'fixture':{n:'native_'+n for n in ['build_a','build_distribution','verify_first','verify_second','package']}},'local_workers':2}
        self.plan={'schema':'chrono-release-plan/v1','version':'fixture-v1','assets':{'a':'products/a','distribution':'products/distribution'}}
        put(self.root/BUILD,self.cfg);put(self.root/PLAN,self.plan)
        put(self.root/'registered/host.json',{'artifacts':[{'path':'consumer/','tracked':False}]})
        put(self.root/'registered/projects.json',{'projects':[],'scripts':[{'actions':{'first':{'operation':'original.first','tool':'probe','argv':['first','space value','$literal','λ']},'second':{'operation':'original.second','tool':'probe','argv':['second']}}}]})
        target=self.root/'.chrono-harness/release/build.py';target.write_bytes(SCRIPT.read_bytes())
        for n in ['git','cargo','rustc','rustup','probe']:
            p=self.root/('tools/'+n);p.parent.mkdir(exist_ok=True);p.write_text(TOOLS);p.chmod(0o755)
        for m in self.cfg['manifests']:(self.root/m).write_text('manifest')
        (self.root/'producer-a').write_bytes(b'#!/bin/sh\nprintf "actual produced bytes\\n"\n')
        (self.root/'producer-distribution').write_text(PACK)
        self.outcomes={};self.receipts={};self.invocations=0

    def close(self):self.temp.cleanup()
    def calls_read(self):return [json.loads(line) for line in self.calls.read_text().splitlines()] if self.calls.exists() else []
    def edit(self, f):f(self.cfg);put(self.root/BUILD,self.cfg)
    def run(self,id0,fault='',attempt='1',selection=None,native=False):
        output=Path(self.temp.name)/('output-'+id0+'-'+str(len(self.receipts)))
        needs=next(u for u in self.cfg['units']+[self.cfg['collector']] if u['id']==id0)['needs']
        selected={n:self.outcomes[n] for n in needs} if selection is None else selection
        if native:selected={self.cfg['native_jobs']['fixture'][n]:r for n,r in selected.items()}
        env=dict(os.environ,CALLS=str(self.calls),FAULT=fault,CHRONO_RELEASE_LOCAL='0' if native else '1',CHRONO_RELEASE_RUN='123',CHRONO_RELEASE_ATTEMPT=attempt,CHRONO_RELEASE_JOB='native_'+id0 if native else id0,CHRONO_RELEASE_DEPENDENCIES=json.dumps(selected))
        if id0=='package':put(self.root/'dependencies.json',selected)
        command=[sys.executable,str(SCRIPT),str(self.root),str(output),*(['--collect'] if id0=='package' else ['--unit',id0])]
        process=subprocess.run(command,env=env,capture_output=True)
        original=Path(self.temp.name)/'driver-processes'/str(self.invocations);self.invocations+=1
        original.mkdir(parents=True)
        (original/'stdout.bin').write_bytes(process.stdout);(original/'stderr.bin').write_bytes(process.stderr)
        put(original/'process.json',{'argv':command,'exit_code':process.returncode})
        if output.exists():
            receipt=json.loads((output/'receipt.json').read_text());self.receipts[str(len(self.receipts))]=(output,receipt)
        if id0!='package' and output.exists():
            handoff=self.root/next(u['handoff'] for u in self.cfg['units'] if u['id']==id0)
            if handoff.exists():shutil.rmtree(handoff)
            shutil.copytree(output,handoff)
            self.outcomes[id0]={'result':'success' if process.returncode==0 else 'failure','outputs':{'artifact_id':'567' if native else sha(output/'receipt.json'),'run_id':'123','attempt':attempt}}
        return process,output
    def builds(self,native=False):
        for id0 in ['build_a','build_distribution']:
            p,_=self.run(id0,native=native);assert p.returncode==0,p.stderr
    def tests(self,native=False):
        for id0 in ['verify_first','verify_second']:
            p,_=self.run(id0,native=native);assert p.returncode==0,p.stderr
    def ready(self):self.builds();self.tests()
    def reseal(self,id0,change):
        directory=self.root/next(u['handoff'] for u in self.cfg['units'] if u['id']==id0)
        file=directory/'receipt.json';r=json.loads(file.read_text());change(r);put(file,r)
        self.outcomes[id0]['outputs']['artifact_id']=sha(file)

class ReleaseUnits(unittest.TestCase):
    def setUp(self):self.f=Fixture();self.addCleanup(self.f.close)
    def tearDown(self):retain_failed_fixture(self,Path(self.f.temp.name))
    def passed(self,p):self.assertEqual(p.returncode,0,p.stderr.decode(errors='replace'))
    def test_producer_verifier_collector_execute_distinct_original_commands(self):
        f=self.f;f.ready();before=f.calls_read();p,out=f.run('package');self.passed(p)
        new=f.calls_read()[len(before):]
        self.assertEqual([c['tool'] for c in new if c['tool']!='git'],['package'])
        self.assertEqual(len([c for c in before if c['tool']=='cargo' and c['argv'][0]=='build']),2)
        self.assertEqual([c['argv'] for c in before if c['tool']=='probe'],[['first','space value','$literal','λ'],['second']])
        receipt=json.loads((out/'receipt.json').read_text());self.assertEqual(set(receipt['verifications']),{'verify_first','verify_second'})
        self.assertEqual(set(receipt['package']['assets']),{'a','distribution'})
        self.assertTrue((out/'release.json').exists())
        self.assertEqual(json.loads((out/receipt['dependency_input']['path']).read_text()),f.outcomes)
        self.assertEqual(json.loads((out/receipt['dependency_file']['path']).read_text()),f.outcomes)
        for directory,r in f.receipts.values():
            for process in r['processes']:
                for stream in ['stdout','stderr']:
                    data=process[stream];raw=(directory/data['path']).read_bytes()
                    self.assertEqual(hashlib.sha256(raw).hexdigest(),data['sha256']);self.assertEqual(len(raw),data['size'])
    def test_local_full_driver_uses_isolated_roots_declared_handoffs_and_same_units(self):
        f=self.f;output=Path(f.temp.name)/'full-output'
        p=subprocess.run([sys.executable,str(SCRIPT),str(f.root),str(output)],env=dict(os.environ,CALLS=str(f.calls)),capture_output=True)
        self.passed(p)
        report=json.loads((output/'local.json').read_text());self.assertEqual(report['status'],'passed')
        self.assertEqual(set(report['units']),{'build_a','build_distribution','verify_first','verify_second','package'})
        calls=f.calls_read();builds=[c for c in calls if c['tool']=='cargo' and c['argv'][0]=='build'];tests=[c for c in calls if c['tool']=='probe']
        self.assertEqual(len(builds),2);self.assertEqual(len(tests),2)
        self.assertEqual(len({c['cwd'] for c in builds+tests}),4)
        self.assertEqual(len([c for c in calls if c['tool']=='package']),1)
        self.assertTrue((output/'units/package/release.json').exists());self.assertTrue((output/'release.json').exists())
        self.assertEqual(list((output/'roots').iterdir()),[])
        receipt=json.loads((output/'receipt.json').read_text())
        for process in receipt['processes']:
            for stream in ['stdout','stderr']:
                d=process[stream];self.assertEqual(sha(output/d['path']),d['sha256'])
        for id0,files in receipt['originals'].items():
            for name,d in files.items():self.assertEqual(sha(output/'originals'/id0/name),d['sha256'])
    def test_real_distribution_packs_the_accepted_original_asset_vector(self):
        f=self.f
        installer=ROOT/'crates/distribution/target/debug/chrono-distribution'
        self.assertTrue(installer.is_file(),'build the registered distribution prerequisite')
        (f.root/'producer-distribution').write_bytes(installer.read_bytes())
        # This repository is only test data. No source-lane Git identity is changed.
        real_git=shutil.which('git');f.cfg['tools']['git']=real_git
        f.cfg['collector']['package_asset']='chrono-distribution'
        f.cfg['units'][1]['asset']='chrono-distribution'
        f.cfg['consumer_staging']['bindings'][1]['asset']='chrono-distribution'
        f.plan['assets']['chrono-distribution']=f.plan['assets'].pop('distribution')
        (f.root/'producer-distribution').rename(f.root/'producer-chrono-distribution')
        put(f.root/BUILD,f.cfg);put(f.root/PLAN,f.plan)
        (f.root/'.gitignore').write_text('handoffs/\nproducts/\nconsumer/\ndependencies.json\n')
        env=dict(os.environ,GIT_CONFIG_NOSYSTEM='1',GIT_CONFIG_GLOBAL=os.devnull)
        for argv in [['init','-q'],['add','--','.'],['-c','user.name=Fixture','-c','user.email=fixture@example.invalid','commit','-qm','release fixture']]:
            result=subprocess.run([real_git,*argv],cwd=f.root,env=env,capture_output=True);self.assertEqual(result.returncode,0,result.stderr)
        f.ready();before=f.calls_read();p,out=f.run('package');self.passed(p)
        release=json.loads((out/'release.json').read_text())
        members=next(iter(release['platforms'].values()));self.assertEqual(set(members),{'a','chrono-distribution'})
        for asset,declaration in members.items():
            self.assertEqual(sha(out/declaration['file']),sha(f.root/f.plan['assets'][asset]))
        self.assertEqual(f.calls_read(),before)
    def test_native_selected_original_attempt_can_be_carried(self):
        f=self.f;f.builds(True);f.tests(True);p,out=f.run('package',attempt='2',native=True);self.passed(p)
        r=json.loads((out/'receipt.json').read_text());self.assertEqual(r['lineage']['attempt'],'2')
        self.assertEqual({v['attempt'] for v in r['selected'].values()},{'1'})
    def test_original_non_utf8_failure_exit_and_sibling_independence(self):
        f=self.f;f.builds();p,out=f.run('verify_first','verify');self.assertEqual(p.returncode,73)
        r=json.loads((out/'receipt.json').read_text());last=next(p for p in r['processes'] if p['phase']=='verification')
        self.assertEqual((out/last['stderr']['path']).read_bytes(),b'original diagnostic\xfe\n')
        self.assertEqual(r['failure']['exit_code'],73)
        p,_=f.run('verify_second');self.passed(p)
        before=f.calls_read();p,_=f.run('package');self.assertEqual(p.returncode,2)
        self.assertFalse(any(c['tool']=='package' for c in f.calls_read()[len(before):]))
    def test_stale_pass_cannot_mask_current_failed_cancelled_skipped_dependency(self):
        f=self.f;f.ready()
        for outcome in ['failure','cancelled','skipped']:
            selected=json.loads(json.dumps(f.outcomes));selected['verify_first']['result']=outcome
            p,_=f.run('package',selection=selected);self.assertEqual(p.returncode,2)
        self.assertFalse(any(c['tool']=='package' for c in f.calls_read()))
    def test_selected_binary_rebuild_invalidates_old_verification(self):
        f=self.f;f.ready();(f.root/'producer-a').write_bytes(b'#!/bin/sh\nprintf "different\\n"\n')
        p,_=f.run('build_a',attempt='2');self.passed(p)
        p,_=f.run('package',attempt='2');self.assertEqual(p.returncode,2)
        self.assertIn(b'lineage changed',p.stderr)
    def test_missing_corrupted_transport_stream_and_lineage_are_rejected(self):
        f=self.f;f.ready()
        for case in ['archive','stream','dependency','lineage','source','action','observations','mode']:
            files={p:p.read_bytes() for p in (f.root/'handoffs').rglob('*') if p.is_file()}
            try:
                if case=='archive':(f.root/'handoffs/build_a/executable.tar').write_bytes(b'broken')
                elif case=='stream':(f.root/'handoffs/verify_first/processes/0.stdout').write_bytes(b'changed')
                elif case=='dependency':(f.root/'handoffs/verify_first/dependency-context.json').write_bytes(b'{}')
                elif case=='lineage':f.reseal('verify_first',lambda r:r['lineage'].update(attempt='2'))
                elif case=='source':f.reseal('verify_first',lambda r:r['context'].update(source_tree='c'*40))
                elif case=='action':f.reseal('verify_first',lambda r:next(p for p in r['processes'] if p['phase']=='verification').update(argv=['unregistered']))
                elif case=='mode':f.reseal('build_a',lambda r:r['assets']['a'].update(mode=0o644))
                else:f.reseal('verify_first',lambda r:r['consumer_staging'].update(observations=[]))
                p,_=f.run('package');self.assertEqual(p.returncode,2,case)
            finally:
                for path,raw in files.items():path.write_bytes(raw)
                for id0 in f.outcomes:
                    file=f.root/next(u['handoff'] for u in f.cfg['units'] if u['id']==id0)/'receipt.json'
                    f.outcomes[id0]['outputs']['artifact_id']=sha(file)
        shutil.rmtree(f.root/'handoffs/build_a');p,_=f.run('package');self.assertEqual(p.returncode,2)
    def test_before_after_all_assets_reject_mutation_absence_and_mode(self):
        f=self.f;f.builds()
        for fault in ['destination-change','source-change','destination-mode','destination-missing']:
            p,out=f.run('verify_first',fault);self.assertEqual(p.returncode,2,fault)
            r=json.loads((out/'receipt.json').read_text());o=r['consumer_staging']['observations']
            self.assertEqual([s['phase'] for s in o],['staged','before-verification','after-verification'])
            self.assertFalse(o[-1]['matches'])
            self.assertEqual(len(o[-1]['files']),4)
    def test_package_failure_and_final_byte_mismatch_preserve_failed_receipts(self):
        f=self.f;f.ready()
        for fault,code in [('pack',41),('package-change',2),('package-mode',2),('package-extra',2)]:
            p,out=f.run('package',fault);self.assertEqual(p.returncode,code)
            r=json.loads((out/'receipt.json').read_text());self.assertEqual(r['status'],'failed')
    def test_invalid_membership_paths_and_duplicate_operations_fail_before_effects(self):
        f=self.f;original=json.loads(json.dumps(f.cfg))
        mutations=[lambda c:c['units'].pop(),lambda c:c['units'][2].update(needs=['build_a']),lambda c:c['units'][0].update(handoff='../escape'),lambda c:c['units'][0].update(asset='unknown'),lambda c:c['units'][1].update(asset='a'),lambda c:c['units'][3].update(operation='original.first'),lambda c:c['collector'].update(needs=['build_a'])]
        for change in mutations:
            f.cfg=json.loads(json.dumps(original));f.edit(change)
            output=Path(f.temp.name)/'bad-out'
            p=subprocess.run([sys.executable,str(SCRIPT),str(f.root),str(output),'--unit','build_a'],capture_output=True)
            self.assertEqual(p.returncode,2);self.assertFalse(output.exists());self.assertEqual(f.calls_read(),[])
        f.cfg=original;put(f.root/BUILD,original)
    def test_absent_output_and_dirty_source(self):
        f=self.f;out=Path(f.temp.name)/'existing';out.mkdir()
        p=subprocess.run([sys.executable,str(SCRIPT),str(f.root),str(out),'--unit','build_a'],capture_output=True)
        self.assertEqual(p.returncode,2);self.assertEqual(f.calls_read(),[])
        p,out=f.run('build_a','dirty');self.assertEqual(p.returncode,2)
        self.assertFalse(any(c['tool']=='cargo' for c in f.calls_read()))
    def test_literal_archive_member_cannot_escape_or_hide_additional_files(self):
        f=self.f;f.builds()
        archive=f.root/'handoffs/build_a/executable.tar'
        with tarfile.open(archive,'w:') as t:t.add(f.root/'producer-a',arcname='../escape')
        f.reseal('build_a',lambda r:r['transport'].update(sha256=sha(archive),size=archive.stat().st_size))
        p,_=f.run('verify_first');self.assertEqual(p.returncode,2)
        self.assertFalse((f.root/'escape').exists())
    def test_large_executable_remains_a_streamed_external_mode_preserving_archive(self):
        f=self.f;producer=f.root/'producer-a'
        with producer.open('ab') as stream:
            stream.write(b'exit 0\n')
            for _ in range(65):stream.write(b'\0'*(1024*1024))
        f.ready();p,out=f.run('package');self.passed(p)
        r=json.loads((out/'receipt.json').read_text());observed=r['package']['assets']['a']
        self.assertGreater(observed['size'],64*1024*1024);self.assertEqual(observed['mode'],0o755)
        self.assertLess((f.root/'handoffs/build_a/receipt.json').stat().st_size,65536)
        self.assertEqual(sha(out/observed['path']),sha(producer))
    def test_selected_identity_and_dependency_metadata_fail_closed(self):
        f=self.f;f.ready()
        for case in ['membership','run','future','missing']:
            selected=json.loads(json.dumps(f.outcomes))
            if case=='membership':selected.pop('verify_second')
            elif case=='run':selected['build_a']['outputs']['run_id']='other'
            elif case=='future':selected['build_a']['outputs']['attempt']='2'
            else:selected['build_a']['outputs'].pop('artifact_id')
            p,_=f.run('package',selection=selected);self.assertEqual(p.returncode,2,case)
        self.assertFalse(any(c['tool']=='package' for c in f.calls_read()))
    def test_original_build_failure_does_not_cancel_independent_producer(self):
        f=self.f;p,out=f.run('build_a','build');self.assertEqual(p.returncode,41)
        r=json.loads((out/'receipt.json').read_text());self.assertEqual(r['failure']['exit_code'],41)
        p,_=f.run('build_distribution');self.passed(p)
        p,_=f.run('verify_first');self.assertEqual(p.returncode,2)
        self.assertFalse(any(c['tool']=='probe' for c in f.calls_read()))
    def test_build_scope_requires_only_its_explicit_manifest(self):
        f=self.f;(f.root/'registered/distribution.toml').unlink();(f.root/'tools/probe').unlink()
        p,_=f.run('build_a');self.passed(p)
        (f.root/'registered/a.toml').unlink();p,out=f.run('build_a')
        self.assertEqual(p.returncode,2);self.assertFalse(out.exists())
    def test_collector_requires_no_available_build_or_test_sdk(self):
        f=self.f;f.ready()
        for name in ['cargo','rustc','rustup','probe']:(f.root/'tools'/name).unlink()
        for manifest in f.cfg['manifests']:(f.root/manifest).unlink()
        before=f.calls_read();p,_=f.run('package');self.passed(p)
        self.assertEqual([c['tool'] for c in f.calls_read()[len(before):] if c['tool']!='git'],['package'])
    def test_host_retains_all_original_unfiltered_actions_and_explicit_native_edges(self):
        cfg=json.loads((ROOT/BUILD).read_text());projects=json.loads((ROOT/cfg['projects']).read_text());projection=json.loads((ROOT/'.chrono-harness/ci/release.json').read_text())
        self.assertEqual(len(cfg['manifests']),15);self.assertEqual(len(cfg['verification_operations']),15)
        actions={a['operation']:a for p in projects['projects']+projects['scripts'] for a in p['actions'].values()}
        for u in cfg['units']:
            if u['kind']=='verify':
                self.assertIn(u['operation'],cfg['verification_operations'])
                action=actions[u['operation']]
                self.assertEqual(action['argv'],['test','--locked','--manifest-path',next(p['manifest'] for p in projects['projects'] if p['actions'].get('execute',{}).get('operation')==u['operation'])])
        self.assertEqual(len(projection['jobs']),62)
        jobs={j['id']:j for j in projection['jobs']}
        for mapping in cfg['native_jobs'].values():
            for u in cfg['units']+[cfg['collector']]:
                j=jobs[mapping[u['id']]]
                self.assertEqual(j['needs'],[mapping[n] for n in u['needs']]);self.assertEqual(j['timeout_minutes'],45)
                self.assertEqual([d['job'] for d in j['downloads']],j['needs'])

class UnitFailureRetention(unittest.TestCase):
    def setUp(self):
        spec=importlib.util.spec_from_file_location('release_recipe',SCRIPT)
        self.recipe=importlib.util.module_from_spec(spec);spec.loader.exec_module(self.recipe)
        self.temp=tempfile.TemporaryDirectory(prefix='release failure retention ')
        self.addCleanup(self.temp.cleanup)
        self.root=Path(self.temp.name);self.output=self.root/'output';self.output.mkdir()

    def tearDown(self):retain_failed_fixture(self,self.root)

    def failed_child(self,stderr):
        execution=self.recipe.UnitExecution(self.root,self.output,
            {'rust_toolchain':'1.95.0','limits':{'unit_seconds':2700,'platform_seconds':2700}}, {}, {'id':'verify'})
        stdout=b'original stdout\xff\n'
        code='import sys;from pathlib import Path;stdout='+repr(stdout)+';stderr='+repr(stderr)+';Path("child.stdout").write_bytes(stdout);Path("child.stderr").write_bytes(stderr);sys.stdout.buffer.write(stdout);sys.stderr.buffer.write(stderr);sys.exit(73)'
        stdout_capture=io.TextIOWrapper(io.BytesIO(),encoding='utf-8')
        stderr_capture=io.TextIOWrapper(io.BytesIO(),encoding='utf-8')
        with contextlib.redirect_stdout(stdout_capture),contextlib.redirect_stderr(stderr_capture):
            with self.assertRaises(subprocess.CalledProcessError) as caught:
                execution.run('verification',[sys.executable,'-c',code],'original.verify')
            self.assertEqual(caught.exception.output,stdout)
            self.assertEqual(caught.exception.stderr,stderr)
            result=execution.save(caught.exception)
        stdout_capture.flush();stderr_capture.flush()
        self.assertEqual(result,73)
        self.assertEqual(stdout_capture.buffer.getvalue(),stdout)
        self.assertTrue(stderr_capture.buffer.getvalue().startswith(stderr))
        self.assertEqual(execution.report['failure']['exit_code'],73)
        self.assertEqual(execution.report['failure']['phase'],'verification')
        self.assertEqual(len(execution.report['processes']),1)
        record=execution.report['processes'][0];self.assertEqual(record['exit_code'],73)
        for name,raw in [('stdout',stdout),('stderr',stderr)]:
            retained=record[name]
            self.assertEqual(retained['size'],len(raw));self.assertEqual(retained['sha256'],hashlib.sha256(raw).hexdigest())
            if 'path' in retained:self.assertEqual((self.output/retained['path']).read_bytes(),raw)
            else:
                self.assertEqual(retained['status'],'unavailable')
                self.assertEqual(bytes(retained['bytes']),raw)
        return execution.report,stderr_capture.buffer.getvalue()

    def test_child_failure_survives_stream_write_failure_with_actual_bytes(self):
        (self.output/'processes/0.stderr').mkdir(parents=True)
        report,diagnostic=self.failed_child(b'original stderr\xfe\n')
        record=report['processes'][0]
        self.assertNotIn('path',record['stderr'])
        self.assertEqual(record['stdout']['path'],'processes/0.stdout')
        receipt=json.loads((self.output/'receipt.json').read_bytes())
        self.assertEqual(receipt['failure']['exit_code'],73)
        self.assertTrue(receipt['secondary_failures'])
        self.assertIn(b'stderr retention',diagnostic)

    def test_child_failure_survives_native_marker_copy_failure(self):
        with tempfile.TemporaryDirectory(prefix='chrono-native-publication-failure-') as retained:
            for name in ['stdout','stderr','process.json']:(Path(retained)/name).write_bytes(b'original nested evidence')
            (self.output/'failure-evidence').write_bytes(b'blocked auxiliary directory')
            report,diagnostic=self.failed_child(b'original native publication process evidence: '+retained.encode()+b'\n')
        self.assertEqual(report['processes'][0]['failure_evidence']['status'],'unavailable')
        self.assertTrue(report['secondary_failures'])
        self.assertIn(b'native marker retention',diagnostic)
        self.assertFalse((self.output/'failure-evidence').is_dir())

    def test_child_failure_survives_non_utf8_native_marker(self):
        report,diagnostic=self.failed_child(b'original native publication process evidence: /tmp/invalid-\xff\n')
        self.assertEqual(report['processes'][0]['failure_evidence']['status'],'unavailable')
        self.assertTrue(report['secondary_failures'])
        self.assertIn(b'native marker retention',diagnostic)

    def test_child_failure_survives_receipt_and_stream_publication_failure(self):
        (self.output/'receipt.json').mkdir()
        (self.output/'processes/0.stderr').mkdir(parents=True)
        report,diagnostic=self.failed_child(b'original stderr\xfe\n')
        self.assertEqual(report['status'],'failed')
        self.assertFalse((self.output/'receipt.json').is_file())
        self.assertIn(b'cannot retain release unit evidence',diagnostic)
        self.assertTrue(any(f['component']=='receipt publication' for f in report['secondary_failures']))

    def test_child_failure_survives_receipt_publication_failure(self):
        (self.output/'receipt.json').mkdir()
        report,diagnostic=self.failed_child(b'original stderr\xfe\n')
        self.assertEqual(report['status'],'failed')
        self.assertFalse((self.output/'receipt.json').is_file())
        self.assertIn(b'cannot retain release unit evidence',diagnostic)
        self.assertTrue(any(f['component']=='receipt publication' for f in report['secondary_failures']))

    def test_successful_child_cannot_pass_when_receipt_publication_fails(self):
        (self.output/'receipt.json').mkdir()
        execution=self.recipe.UnitExecution(self.root,self.output,
            {'rust_toolchain':'1.95.0','limits':{'unit_seconds':2700,'platform_seconds':2700}}, {}, {'id':'verify'})
        execution.run('verification',[sys.executable,'-c','pass'],'original.verify')
        self.assertEqual(execution.save(),2)
        self.assertEqual(execution.report['status'],'failed')
        self.assertIsNone(execution.report['failure']['exit_code'])
        self.assertFalse((self.output/'receipt.json').is_file())

if __name__=='__main__':unittest.main()
