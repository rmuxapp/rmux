"""Check an existing Bash without reading user startup files or installing tools."""
import os
from pathlib import Path
import shlex
import subprocess
import tempfile
from urllib.parse import quote

integration = Path(__file__).resolve().parent.parent / "shell-integration/bash/rmux.bash"
environment = dict(os.environ, TERM_PROGRAM="rmux", RMUX_SHELL_INTEGRATION="1")
environment.pop("BASH_ENV", None)

def run(commands, env=environment):
    return subprocess.run(["bash", "--noprofile", "--norc", "--noediting", "-i"],
                          input=(commands + "\nexit 0\n").encode(),
                          stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                          env=env, timeout=10, check=True).stdout

with tempfile.TemporaryDirectory(prefix="rmux-bash-") as temporary:
    directory = Path(temporary) / "space 世界 %;\x1b"
    directory.mkdir()
    source = f"source {shlex.quote(str(integration))}"
    output = run("\n".join([
        "PS1='fixture> '",
        "PROMPT_COMMAND='printf \"old-hook\\n\" # preserve comment'",
        source, source,
        f"cd -- {shlex.quote(str(directory))}",
        "printf 'HOOK:%s:END\\n' \"$PROMPT_COMMAND\"",
        "PS1='replacement> '",
        f"cd -- {shlex.quote(temporary)}",
        "printf 'same-directory-prompt\\n'",
        f"cd -- {shlex.quote(str(directory))}",
        "printf 'ready\\n'",
    ]))
    expected = b"\x1b]7;file://localhost" + quote(str(directory), safe="/._~-").encode() + b"\x07"
    other_directory = b"\x1b]7;file://localhost" + quote(temporary, safe="/._~-").encode() + b"\x07"
    assert output.count(other_directory) >= 2, "unchanged directories still emit reports"
    assert output.count(expected) >= 2, "returning to a previous directory refreshes the encoded path"
    assert expected in output, "OSC 7 must encode UTF-8, spaces and controls"
    assert b"old-hook\r\n" in output or b"old-hook\n" in output
    assert b"\n_rmux_bash_precmd:END" in output, "hook must append after a newline"
    assert b"\n_rmux_bash_precmd\n_rmux_bash_precmd:END" not in output
    assert b"\x1b]133;A\x07" in output and b"\x1b]133;B\x07" in output
    assert b"replacement> \x1b]133;B\x07" in output
    assert b"\x1b]133;B\x07\x1b]133;B\x07" not in output
    status_output = run("\n".join([
        "PS1='status> '",
        "PROMPT_COMMAND='printf \"USER-STATUS:%s\\n\" \"$?\"'",
        source,
        "false",
        "(exit 7)",
        "true",
    ]))
    assert b"USER-STATUS:1" in status_output and b"USER-STATUS:7" in status_output
    for status in [0, 1, 7]:
        assert f"\x1b]133;D;{status}\x07".encode() in status_output, "user prompt hooks must not replace command status"
    version = subprocess.check_output(["bash", "--noprofile", "--norc", "-c",
                                       "printf '%s %s' ${BASH_VERSINFO[0]} ${BASH_VERSINFO[1]}"],
                                      env=environment).decode().split()
    supports_ps0 = tuple(map(int, version)) >= (4, 4)
    # Indexed PROMPT_COMMAND execution is supported by Bash 5.1+. Older
    # versions still let us verify preservation of the user's array structure.
    for exported in (False, True):
        arrays = run("\n".join([
            "PROMPT_COMMAND=('printf \"FIRST:%s\\n\" \"$?\"' 'printf \"SECOND\\n\"')",
            "export PROMPT_COMMAND" if exported else ":",
            source, source,
            "printf 'ARRAY:%s:%s:%s:END\\n' \"${#PROMPT_COMMAND[@]}\" \"${PROMPT_COMMAND[0]}\" \"${PROMPT_COMMAND[3]}\"",
            "false",
        ]))
        assert b"ARRAY:4:_rmux_bash_capture_status:_rmux_bash_precmd:END" in arrays
        if tuple(map(int, version)) >= (5, 1):
            assert b"FIRST:1\nSECOND\n" in arrays or b"FIRST:1\r\nSECOND\r\n" in arrays
            assert b"\x1b]133;D;1\x07" in arrays
    preexec = run("\n".join([
        "PS0='user-before>'",
        source, source,
        "printf 'PREEXEC:%s:END\\n' \"$PS0\"",
        "printf 'RUNTIME-%s\\n' okay",
    ]))
    marker = b"\x1b]133;C\x07"
    expected_ps0 = (marker if supports_ps0 else b"") + b"user-before>"
    assert b"PREEXEC:" + expected_ps0 + b":END" in preexec
    assert marker + marker not in preexec, "re-sourcing must not duplicate PS0"
    if supports_ps0:
        assert marker + b"user-before>RUNTIME-okay" in preexec, "PS0 must precede command output"
    if not supports_ps0:
        assert marker not in preexec, "old Bash must keep PS0 unchanged"
    disabled = run(source, dict(environment, RMUX_SHELL_INTEGRATION="0"))
    assert b"\x1b]133;" not in disabled and b"\x1b]7;" not in disabled
print("bash integration: scalar/exported-array hooks, idempotence, prompt replacement, encoded cwd, command status, version-gated PS0 preservation and opt-out passed")
