#!/usr/bin/env python3
"""Regenerate the checked-in Finder layout (no Finder automation needed in CI).

Run: mise run generate-dmg-layout
Only this maintenance command needs ds-store; release packaging copies the file.
"""
from pathlib import Path
from ds_store import DSStore

output = Path(__file__).resolve().parents[1] / 'assets/macos/dmg-layout.dsstore'
output.unlink(missing_ok=True)
with DSStore.open(str(output), 'w+') as store:
    store['.']['bwsp'] = {
        'ShowStatusBar': False, 'ShowToolbar': False, 'ShowSidebar': False,
        'ShowPathbar': False, 'ShowTabView': False,
        'WindowBounds': '{{200, 200}, {560, 320}}',
    }
    store['.']['icvp'] = {
        'viewOptionsVersion': 1, 'backgroundType': 1,
        'backgroundColorRed': 1.0, 'backgroundColorGreen': 1.0,
        'backgroundColorBlue': 1.0, 'iconSize': 96.0, 'textSize': 14.0,
        'gridSpacing': 100.0, 'gridOffsetX': 0.0, 'gridOffsetY': 0.0,
        'arrangeBy': 'none', 'labelOnBottom': True,
        'showItemInfo': False, 'showIconPreview': True,
    }
    store['.']['vSrn'] = ('long', 1)
    store['.']['vstl'] = ('type', b'icnv')
    store['rmux.app']['Iloc'] = (140, 140)
    store['Applications']['Iloc'] = (420, 140)
with DSStore.open(str(output), 'r') as store:
    assert store['rmux.app']['Iloc'] == (140, 140)
    assert store['Applications']['Iloc'] == (420, 140)
    assert store['.']['icvp']['arrangeBy'] == 'none'
print(f'Wrote and verified {output}')
