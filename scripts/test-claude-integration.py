#!/usr/bin/env python3
"""Exercise session-local Claude hooks with a fake CLI in a real Unix PTY."""
import errno
import json
import os
from pathlib import Path
import pty
import select
import signal
import shlex
import subprocess
import sys
import tempfile
import time

CLIENT = Path(__file__).resolve().parents[1] / 'target/debug/rmuxctl'
with tempfile.TemporaryDirectory(prefix='rmux-claude-test-') as directory:
    root = Path(directory)
    fake = root / 'claude'
    fake.write_text(f'''#!{sys.executable}
import json, os, signal, subprocess, sys, time
from pathlib import Path
if 'RMUX_CLAUDE_EVENTS' not in os.environ:
    print('PASSTHROUGH', json.dumps(sys.argv[1:]), flush=True)
    sys.exit(7)
settings = json.loads(sys.argv[2])
Path(os.environ['FIXTURE_ROOT'], 'events-path').write_text(os.environ['RMUX_CLAUDE_EVENTS'])
def report(event):
    command = settings['hooks'][event][0]['hooks'][0]['command']
    subprocess.run(command, shell=True, input=json.dumps({{'hook_event_name':event}}), text=True, check=True)
def interrupt(*args):
    print('INTERRUPTED', flush=True)
signal.signal(signal.SIGINT, interrupt)
for event in ['SessionStart', 'UserPromptSubmit', 'PermissionRequest', 'PostToolUse', 'Stop']:
    report(event)
    time.sleep(.2)
print('READY', flush=True)
time.sleep(1)
report('SessionEnd')
sys.exit(7)
''')
    fake.chmod(0o700)
    def run(args):
        pid, fd = pty.fork()
        if pid == 0:
            os.environ.update(PATH=str(root) + os.pathsep + os.environ['PATH'], TERM_PROGRAM='rmux', FIXTURE_ROOT=str(root))
            os.execv(str(CLIENT), [str(CLIENT), 'claude', *args])
        output = bytearray()
        sent = False
        deadline = time.monotonic() + 10
        try:
            while time.monotonic() < deadline:
                if select.select([fd], [], [], .1)[0]:
                    try:
                        data = os.read(fd, 65536)
                    except OSError as error:
                        if error.errno == errno.EIO: break
                        raise
                    if not data: break
                    output.extend(data)
                    if b'READY' in output and not sent:
                        os.killpg(pid, signal.SIGINT)
                        sent = True
            else:
                os.killpg(pid, signal.SIGKILL)
                raise AssertionError('Claude supervisor timed out')
            _, status = os.waitpid(pid, 0)
            assert os.waitstatus_to_exitcode(status) == 7, (status, output)
            return bytes(output)
        finally:
            os.close(fd)
    output = run([])
    for state in ('idle', 'working', 'blocked', 'done', 'clear'):
        assert f'state={state}:id=rmux-claude'.encode() in output, output
    assert b'INTERRUPTED' in output, output
    assert b'777;notify;Claude;Response ready' in output
    assert not Path((root / 'events-path').read_text()).exists()
    for args in (['-p', 'hello'], ['--settings', '{"permissions":{}}'], ['--settings={}', 'hello']):
        output = run(args)
        assert b'PASSTHROUGH' in output and b'7501' not in output, output
    environment = dict(os.environ, PATH=str(root) + os.pathsep + os.environ['PATH'], TERM_PROGRAM='rmux', RMUX_CLI=str(CLIENT), RMUX_SHELL_INTEGRATION='1')
    project = CLIENT.parents[2]
    for shell, flags in [('bash', ['--noprofile', '--norc', '-ic']), ('zsh', ['-df', '-ic'])]:
        integration = project / f'shell-integration/{shell}/rmux.{shell}'
        source = f'source {shlex.quote(str(integration))}; '
        result = subprocess.run([f'/bin/{shell}', *flags, source + "claude -p 'hello spaced'"], env=environment, capture_output=True, text=True)
        assert result.returncode == 7 and 'hello spaced' in result.stdout, result
        result = subprocess.run([f'/bin/{shell}', *flags, "alias claude='printf alias-kept'; " + source + 'alias claude'], env=environment, capture_output=True, text=True)
        assert result.returncode == 0 and 'alias-kept' in result.stdout, result
print('Claude hook lifecycle, Ctrl+C, cleanup, exit status, and passthrough passed')
