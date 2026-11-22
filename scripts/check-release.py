#!/usr/bin/env python3
"""Reject incomplete or reused release versions before compiling platform builds."""
import argparse
from pathlib import Path
import subprocess
import tomllib


def check(root, remote=True):
    version = tomllib.loads((root / 'Cargo.toml').read_text())['workspace']['package']['version']
    tag = 'v' + version
    notes = root / 'docs/releases' / f'{tag}.md'
    if not notes.is_file() or not notes.read_text().startswith(f'# rmux {version}\n'):
        raise SystemExit(f'Missing or mismatched release notes: {notes}')
    for name in ('Cargo.lock', 'tools/web-probe/Cargo.lock'):
        packages = tomllib.loads((root / name).read_text())['package']
        for package in packages:
            if package['name'].startswith('rmux-') and package['name'] != 'rmux-web-probe':
                if package['version'] != version:
                    raise SystemExit(f'{name}: {package["name"]} is not version {version}')
    if remote:
        result = subprocess.run(['git', 'ls-remote', '--exit-code', 'origin', f'refs/tags/{tag}'],
                                cwd=root, capture_output=True, text=True)
        if result.returncode == 0:
            raise SystemExit(f'{tag} already exists; do not rebuild or replace a published version')
        if result.returncode != 2:
            raise SystemExit('Cannot verify remote release tag; check GitHub connectivity and credentials')
    print(f'Release preflight passed for {tag}')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--local', action='store_true', help='Check files without querying GitHub')
    args = parser.parse_args()
    check(Path(__file__).resolve().parents[1], remote=not args.local)
