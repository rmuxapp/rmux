#!/usr/bin/env python3
"""Check hidden-output suppression and repaint on return in an isolated native app.

Requires a desktop capable of drawing the fixture window; not a screenshot or
input-to-display latency test. Configuration and shells are temporary.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=Path("target/debug/rmux"))
    parser.add_argument("--cli", type=Path, default=Path("target/debug/rmuxctl"))
    args = parser.parse_args()
    binary, cli = args.binary.resolve(strict=True), args.cli.resolve(strict=True)
    with tempfile.TemporaryDirectory(prefix="rmux-render-smoke-") as directory:
        root = Path(directory)
        shell = root / "shell"
        shell.write_text('#!/bin/sh\ntrap "exit 0" HUP TERM\nprintf "ready\\n"\nwhile IFS= read -r line; do printf "received: %s\\n" "$line"; done\n')
        shell.chmod(0o700)
        config = root / "config.json"
        config.write_text(json.dumps({"confirm_close": "never", "cursor_blink": False, "restore_session": False}))
        endpoint = root / "rpc/rpc.sock"
        env = {key: value for key, value in os.environ.items() if not key.startswith("RMUX_")}
        env.update(RMUX_CONFIG=str(config), RMUX_SHELL=str(shell), RMUX_SOCKET_DIR=str(endpoint.parent),
                   RMUX_RENDER_METRICS=str(root / "metrics.jsonl"))
        with tempfile.TemporaryFile() as log:
            app = subprocess.Popen([str(binary)], env=env, stdin=subprocess.DEVNULL,
                                   stdout=subprocess.DEVNULL, stderr=log)

            def call(method, **params):
                reply = subprocess.run([str(cli), "--socket", str(endpoint), method, json.dumps(params)],
                                       check=True, capture_output=True, text=True, timeout=10)
                result = json.loads(reply.stdout)
                assert "result" in result, result
                return result["result"]

            def until(read, predicate, message):
                deadline = time.monotonic() + 10
                while True:
                    assert app.poll() is None, "fixture app exited"
                    value = read()
                    if predicate(value):
                        return value
                    if time.monotonic() >= deadline:
                        raise AssertionError(f"{message}: {value}")
                    time.sleep(.05)

            try:
                until(endpoint.exists, bool, "socket did not start")
                surfaces = until(lambda: call("surface.list")["surfaces"], bool, "no terminal")
                terminal = surfaces[0]

                def metrics():
                    return next(row["render_metrics"] for row in call("surface.list")["surfaces"] if row["id"] == terminal["id"])

                until(metrics, lambda row: row["paint_calls"] > 0, "fixture must be drawable")
                call("workspace.create", window_id=terminal["window_id"], name="Visibility survivor", focus=True)
                hidden = until(metrics, lambda row: row["cache_releases"] > 0, "hidden snapshot not released")
                call("surface.send_text", surface_id=terminal["id"], text="hidden-output-世界\n")
                skipped = until(metrics, lambda row: row["hidden_output_invalidations_skipped"] > hidden["hidden_output_invalidations_skipped"], "hidden output was not suppressed")
                assert skipped["paint_calls"] == hidden["paint_calls"], "hidden terminal painted"
                call("surface.focus", surface_id=terminal["id"])
                recovered = until(metrics, lambda row: row["paint_calls"] > skipped["paint_calls"] and row["rows_rebuilt"] > skipped["rows_rebuilt"], "showing terminal did not rebuild and paint")
                text = call("surface.read_text", surface_id=terminal["id"], scope="viewport", max_bytes=4096)["text"]
                assert "received: hidden-output-世界" in text, text
                call("surface.send_text", surface_id=terminal["id"], text="visible-output\n")
                continued = until(metrics, lambda row: row["paint_calls"] > recovered["paint_calls"] and row["parsed_output_bytes"] > recovered["parsed_output_bytes"] and row["rows_rebuilt"] > recovered["rows_rebuilt"], "visible output stopped repainting")
                call("system.quit")
                app.wait(timeout=10)
                assert app.returncode == 0
                print(json.dumps({"status": "passed", "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
                                  "hidden": hidden, "suppressed": skipped, "recovered": recovered, "continued": continued}))
            finally:
                if app.poll() is None:
                    app.terminate()
                    try:
                        app.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        app.kill()
                        app.wait(timeout=5)


if __name__ == "__main__":
    main()
