#!/usr/bin/env python3
"""The product's explicit native release recipe; hosts never run this build."""
import json
import os
from pathlib import Path
import platform
import subprocess
import sys

root, output = (Path(p).resolve() for p in sys.argv[1:])
cfg = json.loads((root / '.chrono-harness/release/build.json').read_text())
if cfg['schema'] != 'chrono-release-build/v1':
    raise ValueError('unsupported release build schema')
env = dict(os.environ, RUSTUP_TOOLCHAIN=cfg['rust_toolchain'], CARGO_PROFILE_RELEASE_STRIP='symbols')
subprocess.run(['rustup', 'toolchain', 'install', cfg['rust_toolchain'], '--profile', 'minimal'], env=env, check=True)
versions = {tool: subprocess.check_output([tool, '--version'], env=env, text=True).strip() for tool in ['cargo','rustc']}
if not versions['rustc'].startswith('rustc ' + cfg['rust_toolchain'] + ' '):
    raise ValueError('unexpected compiler')
for manifest in cfg['manifests']:
    subprocess.run(['cargo', 'build', '--release', '--locked', '--manifest-path', manifest], cwd=root, env=env, check=True)
subprocess.run(['cargo', 'build', '--locked', '--manifest-path', 'crates/distribution/Cargo.toml'], cwd=root, env=env, check=True)
subprocess.run(['cargo', 'test', '--locked', '--manifest-path', 'crates/distribution-tests/Cargo.toml'], cwd=root, env=env, check=True)
subprocess.run([str(root / 'crates/distribution/target/release/chrono-distribution'), 'pack', '--root', str(root), '--plan', str(root / '.chrono-harness/release/plan.json'), '--output', str(output)], cwd=root, env=env, check=True)
(output / 'build.json').write_text(json.dumps({'schema':'chrono-native-build/v1','system':platform.system(),'machine':platform.machine(),'versions':versions,'source_commit':subprocess.check_output(['git','rev-parse','HEAD'],cwd=root,text=True).strip(),'manifests':cfg['manifests']},indent=2)+'\n')
