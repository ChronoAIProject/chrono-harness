#!/usr/bin/env python3
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
