#!/usr/bin/env python3
"""Focused native RPC history smoke; starts and cleans up its own app only."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time

binary = Path('target/debug/rmux').resolve()
cli = Path('target/debug/rmuxctl').resolve()
with tempfile.TemporaryDirectory(prefix='rmux-focus-history-') as directory:
    root = Path(directory)
    config = root / 'config.json'
    config.write_text(json.dumps({'confirm_close': 'never'}))
    def pane(*names):
        return {'kind': 'pane', 'active_tab': 0,
                'tabs': [{'label': name, 'directory': directory} for name in names]}
    layout = root / 'layout.json'
    layout.write_text(json.dumps({'version': 1, 'sidebar': True, 'active_workspace': 0,
        'workspaces': [
            {'name': 'First', 'active_pane': 0, 'layout': {
                'kind': 'split', 'axis': 'Columns', 'weights': [1, 1],
                'children': [pane('a', 'b'), pane('c')]}},
            {'name': 'Second', 'active_pane': 0, 'layout': pane('d')},
        ]}))
    endpoint = root / 'rpc/rpc.sock'
    env = {k: v for k, v in os.environ.items() if not k.startswith('RMUX_')}
    env.update(RMUX_CONFIG=str(config), RMUX_SOCKET_DIR=str(endpoint.parent),
               RMUX_SHELL='/usr/bin/false', MTL_HUD_ENABLED='0')
    with (root / 'app.log').open('w') as log:
        app = subprocess.Popen([str(binary), '--layout', str(layout)], env=env,
                               stdin=subprocess.DEVNULL, stdout=log, stderr=log)
        try:
            deadline = time.monotonic() + 10
            while not endpoint.exists():
                assert app.poll() is None, 'isolated app exited before socket startup'
                if time.monotonic() > deadline:
                    raise TimeoutError('isolated app did not expose its socket')
                time.sleep(.02)
            def call(method, **params):
                reply = subprocess.run([str(cli), '--socket', str(endpoint), method, json.dumps(params)],
                                       check=True, capture_output=True, text=True, timeout=8)
                response = json.loads(reply.stdout)
                assert 'result' in response, response
                return response['result']
            window = call('window.list')['windows'][0]['id']
            surfaces = call('surface.list')['surfaces']
            assert len(surfaces) == 4, surfaces
            a, b, c, d = [surface['id'] for surface in surfaces]
            def focus(surface):
                call('surface.focus', surface_id=surface)
            def navigate(direction, expected):
                result = call('window.focus_history', window_id=window, direction=direction)
                assert result['moved'] == (expected is not None), result
                assert result['surface_id'] == expected, result
                if expected is not None:
                    panes = call('pane.list')['panes']
                    assert any(pane['selected_surface_id'] == expected for pane in panes), panes
                    target = next(s for s in surfaces if s['id'] == expected)
                    current = call('window.list')['windows'][0]
                    assert current['selected_workspace_id'] == target['workspace_id'], current
            for surface in [a, b, c, d]:
                focus(surface)
            navigate('back', c)
            navigate('back', b)
            navigate('forward', c)
            focus(a)
            navigate('forward', None)
            navigate('back', c)
            navigate('forward', a)
            call('workspace.select', workspace_id=surfaces[-1]['workspace_id'])
            navigate('back', a)
            navigate('forward', d)
            call('system.quit')
            app.wait(timeout=5)
            print('PASS: cross-tab/pane/workspace history, forward traversal, branch and boundary no-op')
        finally:
            if app.poll() is None:
                app.terminate()
                try:
                    app.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    app.kill()
                    app.wait(timeout=5)
