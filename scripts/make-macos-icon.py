#!/usr/bin/env python3
"""Compile the macOS Icon Composer source with an already installed Apple SDK.

Generated assets are committed so release runners do not need a newer Xcode.
--check verifies provenance without requiring Apple tools (also used by bundling).
"""
import argparse
import hashlib
import json
from pathlib import Path
import plistlib
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / 'docs/brand/rmux.icon'
ASSETS = ROOT / 'docs/brand/macos'
OUTPUTS = [ASSETS / 'Assets.car', ASSETS / 'icon-info.plist', ROOT / 'docs/brand/rmux.icns']
MANIFEST = ASSETS / 'generated.json'


def hashes():
    files = sorted(path for path in SOURCE.rglob('*') if path.is_file()) + OUTPUTS
    return {str(path.relative_to(ROOT)): hashlib.sha256(path.read_bytes()).hexdigest() for path in files}


def check():
    manifest = json.loads(MANIFEST.read_text())
    minimum = json.loads((ROOT / 'release/macos.json').read_text())['minimum_system_version']
    if manifest['minimum_system_version'] != minimum or manifest['sha256'] != hashes():
        raise SystemExit('macOS icon source or generated assets changed; run mise exec -- python scripts/make-macos-icon.py')
    info = plistlib.loads((ASSETS / 'icon-info.plist').read_bytes())
    assert info == {'CFBundleIconFile': 'rmux', 'CFBundleIconName': 'rmux'}, info
    assert OUTPUTS[0].stat().st_size > 0
    assert OUTPUTS[2].read_bytes().startswith(b'icns')
    print('macOS icon source, compiled assets, and bundle metadata match')


def generate():
    version = subprocess.check_output(['xcodebuild', '-version'], text=True).strip()
    if int(version.splitlines()[0].split()[1].split('.')[0]) < 26:
        raise SystemExit('Icon Composer compilation requires an existing Xcode 26 or later. No tools were installed.')
    minimum = json.loads((ROOT / 'release/macos.json').read_text())['minimum_system_version']
    with tempfile.TemporaryDirectory(prefix='rmux-icon-') as directory:
        output = Path(directory)
        subprocess.run(['xcrun', 'actool', str(SOURCE), '--compile', str(output),
                        '--output-partial-info-plist', str(output / 'icon-info.plist'),
                        '--app-icon', 'rmux', '--platform', 'macosx',
                        '--minimum-deployment-target', minimum,
                        '--output-format', 'human-readable-text', '--warnings', '--errors'], check=True)
        records = json.loads(subprocess.check_output(['xcrun', 'assetutil', '--info', str(output / 'Assets.car')], text=True))
        sizes = {r.get('PixelWidth') for r in records if r.get('AssetType') == 'Icon Image'}
        assert {32, 64, 128, 256, 512, 1024} <= sizes, sizes
        appearances = {r.get('Appearance') for r in records if r.get('AssetType') == 'IconImageStack'}
        assert {'NSAppearanceNameAqua', 'NSAppearanceNameDarkAqua', 'ISAppearanceTintable'} <= appearances, appearances
        ASSETS.mkdir(parents=True, exist_ok=True)
        for destination in OUTPUTS:
            shutil.copyfile(output / destination.name, destination)
    MANIFEST.write_text(json.dumps({'compiler': version, 'minimum_system_version': minimum,
                                   'sha256': hashes()}, indent=2) + '\n')
    check()


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    check() if args.check else generate()
