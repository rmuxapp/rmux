#!/usr/bin/env python3
"""Bundle dependency notices from the exact locally resolved Cargo packages."""
import json
import os
from pathlib import Path
import subprocess
import sys

metadata = json.loads(subprocess.check_output(['cargo', 'metadata', '--locked', '--format-version', '1', '--filter-platform', os.environ.get('RMUX_RELEASE_TARGET', 'aarch64-apple-darwin')]))
resolved = {node['id'] for node in metadata['resolve']['nodes']}
parts = ['rmux third-party notices\n\nDependency license declarations and available upstream license/notice files.\n']
for package in sorted(metadata['packages'], key=lambda p: (p['name'], p['version'])):
    if package['id'] not in resolved or package['name'].startswith('rmux-'):
        continue
    root = Path(package['manifest_path']).parent
    parts.append(f"\n{'=' * 72}\n{package['name']} {package['version']}\nLicense: {package.get('license') or 'See upstream license'}\nSource: {package.get('repository') or ''}\n")
    files = {p for p in root.iterdir() if p.is_file() and p.name.lower().startswith(('license', 'licence', 'copying', 'notice', 'copyright'))}
    if package.get('license_file'):
        files.add(root / package['license_file'])
    for path in sorted(files):
        if path.is_file():
            parts.append('\n' + path.name + '\n' + path.read_text(encoding='utf-8', errors='replace') + '\n')
repo = Path(__file__).resolve().parent.parent
parts.append('\nGhostty notices\n' + (repo / 'release/ghostty-notices.txt').read_text(encoding='utf-8'))
if os.environ.get('RMUX_RELEASE_TARGET') == 'wasm32-unknown-unknown':
    fonts = repo / 'tools/web-probe/assets'
    for name in ['FiraMono-LICENSE', 'Inter-OFL.txt', 'NotoSansCJK-OFL.txt', 'NotoSansCJK-README.md', 'NotoEmoji-OFL.txt', 'NotoEmoji-README.md']:
        parts.append('\nBrowser font notices: ' + name + '\n' + (fonts / name).read_text(encoding='utf-8'))
Path(sys.argv[1]).write_text(''.join(parts), encoding='utf-8')
