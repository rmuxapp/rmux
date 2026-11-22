#!/usr/bin/env python3
"""Isolated native workspace.open acceptance; never connects to the user's app."""
import json
import os
from pathlib import Path
import selectors
import shlex
import subprocess
import tempfile
import time

binary = Path('target/debug/rmux').resolve()
with tempfile.TemporaryDirectory(prefix='rmux-open-smoke-') as temporary:
    root = Path(temporary)
    config = root / 'config.json'
    config.write_text(json.dumps({'confirm_close': 'never'}))
    env = {key: value for key, value in os.environ.items() if not key.startswith('RMUX_')}
    env.update(RMUX_CONFIG=str(config), RMUX_SHELL='/bin/sh', MTL_HUD_ENABLED='0')
    with tempfile.TemporaryFile() as stderr:
        process = subprocess.Popen([str(binary), '--rpc-stdio-live', '-d', str(root)], env=env,
                                   stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=stderr)
        selector = selectors.DefaultSelector()
        selector.register(process.stdout, selectors.EVENT_READ)
        sequence = 0
        pending = bytearray()
        def send(method, params=None):
            global sequence
            sequence += 1
            process.stdin.write((json.dumps({'jsonrpc':'2.0','id':sequence,'method':method,'params':params or {}})+'\n').encode())
            process.stdin.flush()
            deadline = time.monotonic() + 8
            while b'\n' not in pending:
                remaining = deadline - time.monotonic()
                assert remaining > 0 and selector.select(remaining), f'RPC timeout: {method}'
                chunk = os.read(process.stdout.fileno(), 65536)
                assert chunk, f'RPC closed: {method}'
                pending.extend(chunk)
            line, _, rest = pending.partition(b'\n')
            pending[:] = rest
            reply = json.loads(line)
            assert reply['id'] == sequence, reply
            return reply
        def result(method, params=None):
            reply = send(method, params)
            assert 'result' in reply, reply
            return reply['result']
        def identity():
            return {item['id']: item['process_id'] for item in result('surface.list')['surfaces']}
        try:
            assert 'workspace.open' in result('system.capabilities')['methods']
            opened = result('workspace.open', {'name':'Editor bridge 世界','cwd':str(root)})
            assert opened['selected'] and opened['reused'] is False
            surface = next(row for row in result('surface.list')['surfaces'] if row['workspace_id'] == opened['id'])
            marker = root / 'shell-state'
            result('surface.send_text', {'surface_id':surface['id'], 'text':"bridge_token='kept 世界'\n"})
            before = identity()
            for _ in range(2):
                reused = result('workspace.open', {'name':'Editor bridge 世界','cwd':'/missing-create-only-directory'})
                assert reused['id'] == opened['id'] and reused['reused']
                assert identity() == before
            command = 'printf "%s\\n%s\\n%s" "$bridge_token" "$$" "$PWD" > ' + shlex.quote(str(marker)) + '\n'
            result('surface.send_text', {'surface_id':surface['id'], 'text':command})
            deadline = time.monotonic() + 5
            while not marker.exists() and time.monotonic() < deadline:
                time.sleep(0.02)
            lines = marker.read_text().splitlines()
            assert lines[:2] == ['kept 世界', str(surface['process_id'])], lines
            assert Path(lines[2]).resolve() == root.resolve(), lines
            other = result('window.create')['id']
            duplicate = result('workspace.create', {'window_id':other,'name':'Editor bridge 世界','cwd':str(root)})
            assert send('workspace.open', {'name':'Editor bridge 世界'})['error']['code'] == -32006
            scoped = result('workspace.open', {'name':'Editor bridge 世界','window_id':other})
            assert scoped['id'] == duplicate['id'] and scoped['reused']
            result('workspace.create', {'window_id':other,'name':'Editor bridge 世界','cwd':str(root)})
            assert send('workspace.open', {'name':'Editor bridge 世界','window_id':other})['error']['code'] == -32006
            print('PASS: create once, reuse stable IDs/PIDs/Unicode shell state/cwd; global and same-window ambiguity rejected; explicit window lookup works.')
        finally:
            if process.poll() is None:
                try:
                    send('system.quit')
                    process.wait(timeout=5)
                except Exception:
                    process.terminate()
                    process.wait(timeout=5)
            selector.close()
            process.stdin.close()
            process.stdout.close()
