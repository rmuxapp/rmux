"""Focused integration check using an existing zsh; never reads user rc files."""
import os
from pathlib import Path
import shlex
import subprocess
import tempfile
from urllib.parse import quote


integration = Path(__file__).resolve().parent.parent / "shell-integration/zsh/rmux.zsh"
with tempfile.TemporaryDirectory(prefix="rmux-zsh-") as temporary:
    directory = Path(temporary) / "space 世界 %;\x1b"
    directory.mkdir()
    commands = "\n".join([
        "PS1='fixture> '",
        "original_precmd() { :; }; original_preexec() { :; }",
        "precmd_functions=(original_precmd); preexec_functions=(original_preexec)",
        f"source {shlex.quote(str(integration))}",
        f"source {shlex.quote(str(integration))}",
        "printf 'HOOKS:%s:%s\\n' ${#precmd_functions} ${#preexec_functions}",
        f"cd -- {shlex.quote(str(directory))}",
        "printf 'command-output\\n'",
        f"cd -- {shlex.quote(temporary)}",
        "printf 'same-directory-prompt\\n'",
        f"cd -- {shlex.quote(str(directory))}",
        "false",
        "exit 0",
        "",
    ])
    environment = dict(os.environ, TERM_PROGRAM="rmux", RMUX_SHELL_INTEGRATION="1")
    result = subprocess.run(["zsh", "-d", "-f", "-i"], input=commands.encode(),
                            stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                            env=environment, timeout=10, check=True)
    output = result.stdout
    expected_directory = b"\x1b]7;file://localhost" + quote(str(directory), safe="/._~-").encode() + b"\x07"
    other_directory = b"\x1b]7;file://localhost" + quote(temporary, safe="/._~-").encode() + b"\x07"
    assert output.count(other_directory) >= 2, "unchanged directories still emit reports"
    assert output.count(expected_directory) >= 2, "returning to a previous directory refreshes the encoded path"
    assert expected_directory in output, "OSC 7 must encode UTF-8, spaces and controls"
    assert b"HOOKS:2:2" in output, "sourcing twice must preserve and not duplicate hooks"
    assert b"\x1b]133;C\x07command-output" in output, "output marker precedes command output"
    assert b"\x1b]133;D;1\x07" in output, "nonzero exit status must survive hook chaining"
    assert b"\x1b]133;A\x07" in output and b"\x1b]133;B\x07" in output
    assert b"\x1b]133;B\x07\x1b]133;B\x07" not in output, "prompt suffix must not accumulate"
    environment["RMUX_SHELL_INTEGRATION"] = "0"
    disabled = subprocess.run(["zsh", "-d", "-f", "-i"],
                              input=f"source {shlex.quote(str(integration))}\nexit\n".encode(),
                              stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                              env=environment, timeout=10, check=True)
    assert b"\x1b]133;" not in disabled.stdout and b"\x1b]7;" not in disabled.stdout
print("zsh integration: hooks, markers, exit status, encoded directory and opt-out passed")
