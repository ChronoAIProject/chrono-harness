#!/usr/bin/env python3
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
