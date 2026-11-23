#!/usr/bin/env python3
"""Exercise configured CLI command argv, environment and cwd through a real PTY."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time


def run(binary):
    with tempfile.TemporaryDirectory(prefix="rmux-command-") as directory:
        root = Path(directory).resolve()
        project = root / "project 世界"
        project.mkdir()
        output = root / "result.json"
        fixture = root / "probe.py"
        fixture.write_text("import json, os, pathlib, sys\npathlib.Path(sys.argv[1]).write_text(json.dumps({'argv': sys.argv[2:], 'cwd': os.getcwd(), 'value': os.environ['RMUX_COMMAND_TEST'], 'empty': os.environ['RMUX_COMMAND_EMPTY']}))\n")
        config = root / "config.json"
        env = {key: value for key, value in os.environ.items() if not key.startswith("RMUX_")}
        env.update(RMUX_CONFIG=str(config), RMUX_SHELL="/usr/bin/false", RMUX_COMMAND_TEST="inherited")
        for cwd in [str(project), None]:
            output.unlink(missing_ok=True)
            config.write_text(json.dumps({"commands": [{"name": "Probe 世界", "argv": [sys.executable, str(fixture), str(output), "", "two words", "$(literal)", "世界"], "cwd": cwd, "env": {"RMUX_COMMAND_TEST": "configured 世界", "RMUX_COMMAND_EMPTY": ""}}]}))
            with tempfile.TemporaryFile() as stderr:
                process = subprocess.Popen([str(binary), "--command", "Probe 世界"], cwd=root, env=env, stdout=subprocess.DEVNULL, stderr=stderr)
                try:
                    deadline = time.monotonic() + 8
                    while not output.exists() and time.monotonic() < deadline and process.poll() is None:
                        time.sleep(0.02)
                    assert output.exists(), "configured command did not write its probe"
                    # The child exits immediately after writing, before rmux is terminated.
                    time.sleep(0.05)
                    result = json.loads(output.read_text())
                    assert result == {"argv": ["", "two words", "$(literal)", "世界"], "cwd": cwd or str(root), "value": "configured 世界", "empty": ""}, result
                finally:
                    if process.poll() is None:
                        process.terminate()
                    try:
                        process.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        process.kill()
                        process.wait(timeout=5)
        missing = subprocess.run([str(binary), "--command", "missing"], env=env, capture_output=True, timeout=5)
        assert missing.returncode == 2 and b"command is not configured" in missing.stderr
    print("PASS: configured command literal argv, Unicode/empty env, configured/inherited cwd, missing name")


if __name__ == "__main__":
    run(Path(sys.argv[1] if len(sys.argv) > 1 else "target/debug/rmux").resolve())
