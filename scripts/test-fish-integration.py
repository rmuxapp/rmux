"""Hook fixture for an existing fish; no user configuration or installations."""
import os
import re
from pathlib import Path
import shutil
import subprocess
import tempfile
from urllib.parse import unquote

fish = shutil.which("fish")
if fish is None:
    raise SystemExit("fish is not installed; this test does not install tools")
integration = Path(__file__).resolve().parent.parent / "shell-integration/fish/rmux.fish"
subprocess.run([fish, "--no-config", "--no-execute", str(integration)], check=True)
version = tuple(map(int, re.search(r"(\d+)\.(\d+)", subprocess.check_output([fish, "--version"], text=True)).groups()))
native_markers = version >= (3, 7) or subprocess.run([fish, "--no-config", "--command", "status test-feature mark-prompt"],
                               stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode in (0, 1)


def quote(text):
    return "'" + str(text).replace("\\", "\\\\").replace("'", "\\'") + "'"


with tempfile.TemporaryDirectory(prefix="rmux-fish-") as temporary:
    directory = Path(temporary) / "space 世界 %;\x1b"
    directory.mkdir()
    source = "source " + quote(integration)
    commands = "\n".join([
        "function fish_prompt; printf 'user-prompt'; end",
        "function user_hook --on-event fish_prompt; printf 'user-hook'; end",
        source, source,
        "cd " + quote(directory),
        "emit fish_prompt",
        "fish_prompt",
        "emit fish_preexec",
        "false; emit fish_postexec",
        "emit fish_prompt",
    ])

    def run(enabled, features=()):
        env = dict(os.environ, TERM_PROGRAM="rmux", RMUX_SHELL_INTEGRATION=enabled,
                   XDG_CONFIG_HOME=temporary, XDG_DATA_HOME=temporary)
        return subprocess.run([fish, "--no-config", "--interactive", *features, "--command", commands],
                              stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                              env=env, check=True, timeout=10).stdout

    output = run("1")
    assert b"user-prompt" in output and b"user-hook" in output
    if native_markers:
        assert b"\x1b]133;" not in output and b"\x1b]7;" not in output, "native fish must not gain duplicate rmux hooks"
    else:
        paths = [part.split(b"\x07", 1)[0] for part in output.split(b"\x1b]7;file://localhost")[1:]]
        assert paths and all(b"\x1b" not in path for path in paths)
        assert any(unquote(path.decode()) == str(directory) for path in paths)
        assert b"user-prompt" in output and b"user-hook" in output
        assert output.count(b"\x1b]133;A\x07") == 2, "re-sourcing must not duplicate hooks"
        assert b"\x1b]133;C\x07" in output
        assert b"\x1b]133;D;1\x07" in output
    if version >= (4, 0):
        disabled_native = run("1", ("--features=no-mark-prompt",))
        assert b"\x1b]133;" not in disabled_native, "explicitly disabled native markers must stay disabled"
    disabled = run("0")
    assert b"\x1b]133;" not in disabled and b"\x1b]7;" not in disabled
    # Exercise the startup ordering used by prepare_fish, including user hooks
    # and opt-out set inside config.fish rather than only the parent environment.
    config = Path(temporary) / "fish"
    config.mkdir(exist_ok=True)
    for enabled in (True, False):
        (config / "config.fish").write_text(
            "set -g RMUX_USER_CONFIG loaded\n"
            "function fish_prompt; printf 'custom-prompt'; end\n"
            + ("" if enabled else "set -gx RMUX_SHELL_INTEGRATION 0\n")
        )
        for login in (False, True):
            env = dict(os.environ, HOME=temporary, TERM_PROGRAM="rmux",
                       XDG_CONFIG_HOME=temporary, XDG_DATA_HOME=temporary,
                       RMUX_SHELL_INTEGRATION="1", RMUX_FISH_WRAPPER=str(integration.parent))
            result = subprocess.run(
                [fish, "--interactive", *(["--login"] if login else []),
                 "--init-command", 'source "$RMUX_FISH_WRAPPER/rmux.fish"; set -e RMUX_FISH_WRAPPER',
                 "--command", "printf '%s|' $RMUX_USER_CONFIG; fish_prompt; "
                 "emit fish_prompt; set -q RMUX_FISH_WRAPPER; and exit 9; exit 0"],
                env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=True, timeout=10,
            ).stdout
            assert b"loaded|custom-prompt" in result
            assert (b"\x1b]133;A\x07" in result) == (enabled and not native_markers)
print("fish hooks and automatic startup: cwd, user configuration, login modes, idempotence, status and opt-out passed")
