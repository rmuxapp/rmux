#!/usr/bin/env python3
"""Install the compiled Apple icon and its generated plist keys before signing."""
from pathlib import Path
import plistlib
import shutil
import subprocess
import sys

root = Path(__file__).resolve().parents[1]
subprocess.run([sys.executable, str(root / 'scripts/make-macos-icon.py'), '--check'], check=True)
bundle = Path(sys.argv[1])
resources = bundle / 'Contents/Resources'
resources.mkdir(parents=True, exist_ok=True)
for source in [root / 'docs/brand/rmux.icns', root / 'docs/brand/macos/Assets.car']:
    shutil.copyfile(source, resources / source.name)
path = bundle / 'Contents/Info.plist'
info = plistlib.loads(path.read_bytes())
info.update(plistlib.loads((root / 'docs/brand/macos/icon-info.plist').read_bytes()))
path.write_bytes(plistlib.dumps(info))
