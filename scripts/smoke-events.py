#!/usr/bin/env python3
"""Isolated native replay smoke; run through mise after a desktop debug build."""
import json
import os
from pathlib import Path
import selectors
import subprocess
import tempfile
import time

with tempfile.TemporaryDirectory(prefix="rmux-events-") as directory:
    config = Path(directory) / "config.json"
    config.write_text('{"confirm_close":"never"}')
    env = {k: v for k, v in os.environ.items() if not k.startswith("RMUX_")}
    env.update(RMUX_CONFIG=str(config), RMUX_SHELL="/usr/bin/true")
    with tempfile.TemporaryFile() as errors:
        process = subprocess.Popen([str(Path("target/debug/rmux").resolve()), "--rpc-stdio-live"],
                                   env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                   stderr=errors, bufsize=0)
        selector = selectors.DefaultSelector()
        selector.register(process.stdout, selectors.EVENT_READ)
        pending = bytearray()
        sequence = 0

        def call(method, params=None):
            global sequence
            sequence += 1
            request = {"jsonrpc": "2.0", "id": sequence, "method": method}
            if params is not None:
                request["params"] = params
            process.stdin.write((json.dumps(request) + "\n").encode())
            deadline = time.monotonic() + 8
            while b"\n" not in pending:
                remaining = deadline - time.monotonic()
                assert remaining > 0 and selector.select(remaining), f"timeout: {method}"
                data = os.read(process.stdout.fileno(), 65536)
                assert data, f"closed: {method}"
                pending.extend(data)
            line, _, rest = pending.partition(b"\n")
            pending[:] = rest
            response = json.loads(line)
            assert response["id"] == sequence and "result" in response, response
            return response["result"]

        try:
            assert "events.read" in call("system.capabilities")["methods"]
            surface = call("surface.list")["surfaces"][0]["id"]
            before = call("events.read")["cursor"]
            call("notification.send", {"surface_id": surface, "title": "Replay 世界", "body": "private body"})
            received = call("events.read", {"after": before, "limit": 1})
            while received["events"] and received["events"][0]["kind"] != "notification.received":
                received = call("events.read", {"after": received["cursor"], "limit": 1})
            event = received["events"][0]
            assert event["surface_id"] == surface and received["gap"] is None
            assert "private body" not in json.dumps(received)
            call("notification.dismiss", {"surface_id": surface, "revision": int(event["notification_id"])})
            page = call("events.read", {"after": received["cursor"]})
            assert any(e["kind"] == "notification.dismissed" and e["notification_id"] == event["notification_id"] for e in page["events"])
            assert call("events.read", {"after": page["cursor"]})["events"] == []
            assert call("events.read", {"after": "0-0:0"})["gap"] == "restart"
            call("system.quit")
            assert process.wait(timeout=5) == 0
            print("PASS native notification event replay, dismissal, empty replay and restart gap")
        finally:
            if process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait(timeout=5)
            selector.close()
