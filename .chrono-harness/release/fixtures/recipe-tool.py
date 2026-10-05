#!/usr/bin/env python3
import json, os, sys, subprocess, tempfile
from pathlib import Path
name = Path(sys.argv[0]).name
with open(os.environ['CHRONO_RECIPE_CALLS'], 'a') as log:
    log.write(json.dumps({'tool':name,'argv':sys.argv[1:],'cwd':str(Path.cwd())})+'\n')
failure = os.environ.get('CHRONO_RECIPE_FAIL')
if failure in ('evidence', 'evidence-alias') and name == 'probe':
    retained = Path(tempfile.mkdtemp(prefix='chrono-native-publication-failure-'))
    preparation = retained / 'state' / 'collection' / 'preparation'
    preparation.mkdir(parents=True)
    (preparation / 'receipt.json').write_bytes(b'{"exit_code":73,"stderr":"original"}')
    (preparation / 'stdin.bin').write_bytes(b'original input')
    (preparation / 'stdout.bin').write_bytes(b'partial output')
    (preparation / 'stderr.bin').write_bytes(b'original stderr')
    (retained / 'process.json').write_bytes(b'{"exit_code":73}')
    (retained / 'stdout').write_bytes(b'fixture stdout')
    (retained / 'stderr').write_bytes(b'fixture stderr')
    if failure == 'evidence-alias':
        alias = Path.cwd() / 'temporary-directory-alias'
        alias.symlink_to(retained.parent, target_is_directory=True)
        retained = alias / retained.name
    print('original native publication process evidence: ' + str(retained), file=sys.stderr)
    sys.exit(73)
if ((failure == 'source' and name == 'git') or
    (failure == 'install' and name == 'rustup') or
    (failure == 'build' and name == 'cargo' and sys.argv[1:2] == ['build']) or
    (failure == 'pack' and name == 'chrono-distribution')):
    sys.stdout.buffer.write(b'failed phase\xff\n')
    sys.stderr.buffer.write(b'original diagnostic\xfe\n')
    if failure == 'pack':
        if Path('consumer/bin/tool').exists(): Path('consumer/bin/tool').write_bytes(b'pack side-effect')
        Path(sys.argv[sys.argv.index('--output')+1]).mkdir()
    sys.exit(41)
if failure == 'version' and name == 'rustc':
    print('rustc different-version')
    sys.exit(0)
if name == 'rustc': print('rustc 1.95.0 fixture')
elif name == 'cargo' and sys.argv[1:] == ['--version']: print('cargo fixture')
elif name == 'git':
    if 'ls-files' in sys.argv[1:]:
        if failure == 'tracked': sys.stdout.buffer.write(b'consumer/bin/tool\0')
    else: print('a'*40)
elif name == 'probe':
    if sys.argv[1:2] == ['consumer']:
        subprocess.run([sys.argv[2]], check=True)
        if failure in ('destination-change', 'destination-change-failure'):
            Path(sys.argv[2]).write_bytes(b'overwritten consumer')
        if failure == 'source-change': Path(sys.argv[3]).write_bytes(b'overwritten release')
        if failure == 'destination-missing': Path(sys.argv[2]).unlink()
        if failure == 'destination-mode': Path(sys.argv[2]).chmod(0o644)
        if failure == 'destination-change-failure': sys.exit(73)
    sys.stdout.buffer.write(b'actual stdout\xff\n')
    sys.stderr.buffer.write(b'actual stderr\xfe\n')
    if len(sys.argv) > 1 and os.environ.get('CHRONO_RECIPE_FAIL') == sys.argv[1]: sys.exit(73)
elif name == 'chrono-distribution':
    Path(sys.argv[sys.argv.index('--output')+1]).mkdir()
