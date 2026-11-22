"""Exercise shell-approved capture without user startup files or real histories."""
import os
from pathlib import Path
import re
import shlex
import subprocess
import tempfile

repo = Path(__file__).resolve().parents[1]

def capture(shell, enabled=True):
    with tempfile.TemporaryDirectory(prefix='rmux-history-capture-') as temporary:
        history = Path(temporary) / 'history'
        if shell == 'bash':
            command = ['/bin/bash', '--noprofile', '--norc', '--noediting', '-i']
            settings = 'set -o history; HISTCONTROL=ignorespace; HISTIGNORE="echo OMIT_ME"'
            script = repo / 'shell-integration/bash/rmux.bash'
        else:
            command = ['/bin/zsh', '-d', '-f', '-i']
            settings = 'setopt hist_ignore_space; SAVEHIST=100'
            script = repo / 'shell-integration/zsh/rmux.zsh'
        commands = [
            f'HISTFILE={shlex.quote(str(history))}; HISTSIZE=200; {settings}',
            f'source {shlex.quote(str(script))}',
            'echo ACCEPTED_世界',
            ' echo PRIVATE_SPACE',
            ' echo PRIVATE_SECOND',
            'echo OMIT_ME',
            "printf '%s\\n' 'MULTI\nLINE'",
            'false',
            'echo FINAL_ACCEPTED',
            'HISTFILE=/dev/null',
            'echo PRIVATE_DISABLED',
            'exit 0',
        ]
        env = {**os.environ, 'TERM_PROGRAM': 'rmux', 'RMUX_SHELL_INTEGRATION': '1',
               'RMUX_SESSION_HISTORY': '1' if enabled else '0'}
        env.pop('BASH_ENV', None)
        result = subprocess.run(command, input=('\n'.join(commands)+'\n').encode(),
                                stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                                env=env, timeout=15, check=True)
        records = [bytes.fromhex(encoded.decode()).decode() for encoded in
                   re.findall(rb'\x1b\]9999;rmux-history;([0-9a-f]+)\x07',result.stdout)]
        if not enabled:
            assert not records, (shell, records)
            return
        assert 'echo ACCEPTED_世界' in records, (shell, records)
        assert 'echo FINAL_ACCEPTED' in records, (shell, records)
        assert 'false' in records, (shell, records)
        assert any('MULTI\nLINE' in entry for entry in records), (shell, records)
        if shell == 'bash':
            assert 'echo OMIT_ME' not in records, records
        assert not any('PRIVATE_' in entry for entry in records), (shell, records)
        assert not any('source ' in entry for entry in records), (shell, records)
        assert b'\x1b]133;D;1\x07' in result.stdout, 'capture changed exit status'
        assert not history.exists() or b'9999;rmux-history' not in history.read_bytes()

for shell in ('bash', 'zsh'):
    capture(shell)
    capture(shell, False)
print('shell history capture: exclusions, Unicode, initial/global history, opt-out and exit status passed')

# An explicit empty shell-history list invalidates rmux's retained copy, and
# numbering can restart without hiding the next accepted command.
for shell in ('bash', 'zsh'):
    with tempfile.TemporaryDirectory(prefix='rmux-history-clear-') as temporary:
        history = Path(temporary) / 'history'
        history.touch()
        script = repo / f'shell-integration/{shell}/rmux.{"bash" if shell == "bash" else "zsh"}'
        if shell == 'bash':
            argv = ['/bin/bash', '--noprofile', '--norc', '--noediting', '-i']
            setup, reset = 'set -o history; HISTCONTROL=ignorespace', 'history -c'
        else:
            argv = ['/bin/zsh', '-d', '-f', '-i']
            setup, reset = 'SAVEHIST=100; setopt hist_ignore_space', 'fc -p "$HISTFILE" 200 100'
        lines = [f'HISTFILE={shlex.quote(str(history))}; HISTSIZE=200; {setup}',
                 f'source {shlex.quote(str(script))}', 'echo BEFORE_CLEAR', reset,
                 'echo AFTER_CLEAR', 'exit 0']
        env = {**os.environ, 'TERM_PROGRAM':'rmux', 'RMUX_SHELL_INTEGRATION':'1', 'RMUX_SESSION_HISTORY':'1'}
        env.pop('BASH_ENV', None)
        output = subprocess.run(argv, input=('\n'.join(lines)+'\n').encode(), env=env,
                                stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=15, check=True).stdout
        reports = re.findall(rb'\x1b\]9999;rmux-history;([^\x07]*)\x07',output)
        decoded = [None if entry == b'clear' else bytes.fromhex(entry.decode()).decode() for entry in reports]
        assert decoded == ['echo BEFORE_CLEAR', None, 'echo AFTER_CLEAR'], (shell, decoded)
print('shell history clearing: explicit reset and resumed capture passed')
