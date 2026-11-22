#!/usr/bin/env python3
"""Isolated native replay smoke; run through mise after a desktop debug build."""
import json
import os
from pathlib import Path
import selectors
import socket
import subprocess
import tempfile
import concurrent.futures
import threading
import time

with tempfile.TemporaryDirectory(prefix="rmux-events-") as directory:
    config = Path(directory) / "config.json"
    config.write_text('{"confirm_close":"never"}')
    env = {k: v for k, v in os.environ.items() if not k.startswith("RMUX_")}
    endpoint = Path(directory) / "rpc"
    env.update(RMUX_CONFIG=str(config), RMUX_SHELL="/usr/bin/true", RMUX_SOCKET_DIR=str(endpoint))
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
            initial = call("events.read")
            assert sum(e["kind"] == "surface.created" and e["surface_id"] == surface for e in initial["events"]) == 1
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
            cli = subprocess.run([str(Path("target/debug/rmuxctl").resolve()), "read-events",
                                  "--after", page["cursor"], "--limit", "16", "--wait-ms", "0"],
                                 env=env, capture_output=True, timeout=8)
            assert cli.returncode == 0, cli.stderr.decode()
            cli_page = json.loads(cli.stdout)["result"]
            assert cli_page["cursor"] == page["cursor"] and cli_page["events"] == []
            start = time.monotonic()
            timed = call("events.read", {"after": page["cursor"], "wait_ms": 100})
            assert timed["events"] == [] and time.monotonic() - start >= 0.05
            ready = threading.Event()
            def wait_for_event():
                with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as client:
                    client.settimeout(5)
                    client.connect(str(endpoint / "rpc.sock"))
                    request = {"jsonrpc": "2.0", "id": 1, "method": "events.read", "params": {"after": timed["cursor"], "wait_ms": 2000}}
                    client.sendall((json.dumps(request) + "\n").encode())
                    ready.set()
                    with client.makefile("rb") as reader:
                        return json.loads(reader.readline(65536))["result"]
            with concurrent.futures.ThreadPoolExecutor(max_workers=1) as executor:
                future = executor.submit(wait_for_event)
                assert ready.wait(2)
                time.sleep(0.05)
                assert not future.done(), "empty event wait returned before an event"
                start = time.monotonic()
                call("notification.send", {"surface_id": surface, "title": "Wake waiting client"})
                assert time.monotonic() - start < 1, "event wait blocked the shared dispatcher"
                assert any(e["kind"] == "notification.received" for e in future.result(timeout=3)["events"])
            assert call("events.read", {"after": "0-0:0"})["gap"] == "restart"
            window = call("window.list")["windows"][0]["id"]
            original_workspace = call("workspace.list")["workspaces"][0]["id"]
            boundary = call("events.read")["cursor"]
            call("workspace.create", {"window_id": window, "name": "Event lifetime fixture"})
            destination = next(w["id"] for w in call("workspace.list")["workspaces"] if w["id"] != original_workspace)
            created_surface = next(s["id"] for s in call("surface.list")["surfaces"] if s["id"] != surface)
            created = call("events.read", {"after": boundary})
            assert sum(e["kind"] == "surface.created" and e["surface_id"] == created_surface for e in created["events"]) == 1
            call("surface.move_to_workspace", {"surface_id": created_surface, "workspace_id": original_workspace})
            moved = call("events.read", {"after": created["cursor"]})
            assert not any(e["kind"] in ("surface.created", "surface.closed") and e["surface_id"] == created_surface for e in moved["events"])
            # Keep another workspace alive so the application can answer after
            # closing both original terminal entities.
            call("workspace.create", {"window_id": window, "name": "Survivor"})
            boundary = call("events.read")["cursor"]
            call("workspace.close", {"workspace_id": original_workspace})
            closed = call("events.read", {"after": boundary})
            for target in (surface, created_surface):
                assert sum(e["kind"] == "surface.closed" and e["surface_id"] == target for e in closed["events"]) == 1, closed
            call("system.quit")
            assert process.wait(timeout=5) == 0
            print("PASS replay, bounded waiting, concurrent mutation and surface lifetimes")
        finally:
            if process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait(timeout=5)
            selector.close()
