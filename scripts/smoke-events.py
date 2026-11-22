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
from urllib.parse import quote

with tempfile.TemporaryDirectory(prefix="rmux-events-") as directory:
    config = Path(directory) / "config.json"
    config.write_text('{"confirm_close":"never"}')
    env = {k: v for k, v in os.environ.items() if not k.startswith("RMUX_")}
    endpoint = Path(directory) / "rpc"
    reported_directory = Path(directory) / "reported 世界"
    reported_directory.mkdir()
    shell = Path(directory) / "fixture-shell"
    shell.write_text('#!/bin/sh\ntrap "exit 0" HUP TERM\nwhile IFS= read -r line; do\n  case "$line" in\n    bell) printf "\\007\\007\\007" ;;\n    metadata) printf "\\033]7501;state=working:id=build\\007\\033]9;4;1;42\\007\\033]2;Replay title 世界\\007\\033]7;file://localhost%s\\007" "$RMUX_TEST_URL" ;;\n  esac\ndone\n')
    shell.chmod(0o700)
    env.update(RMUX_CONFIG=str(config), RMUX_SHELL=str(shell), RMUX_SOCKET_DIR=str(endpoint), RMUX_TEST_URL=quote(str(reported_directory)))
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
            capabilities = call("system.capabilities")
            assert "events.read" in capabilities["methods"]
            event_api = capabilities["events"]
            assert event_api["version"] == 1 and event_api["max_page_records"] == 128
            assert event_api["max_wait_ms"] == 3000 and not event_api["push_subscriptions"]
            assert "surface.directory_changed" in event_api["kinds"]
            assert "surface.focus_changed" in event_api["kinds"]
            assert "surface.location_invalidated" in event_api["kinds"]
            assert "workspace.created" in event_api["kinds"]
            assert "workspace_id" in event_api["filters"]
            surfaces = call("surface.list")["surfaces"]
            assert all(isinstance(item["input_focused"], bool) for item in surfaces)
            surface = surfaces[0]["id"]
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
                                  "--after", page["cursor"], "--limit", "16", "--wait-ms", "0",
                                  "--surface", surface, "--kind", "notification.received"],
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
                    request = {"jsonrpc": "2.0", "id": 1, "method": "events.read", "params": {"after": timed["cursor"], "wait_ms": 2000, "surface_id": surface, "kinds": ["notification.received"]}}
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
            boundary = call("events.read")["cursor"]
            call("surface.send_text", {"surface_id": surface, "text": "metadata\n"})
            metadata_events = []
            deadline = time.monotonic() + 3
            while time.monotonic() < deadline:
                page = call("events.read", {"after": boundary, "wait_ms": 100})
                metadata_events.extend(page["events"])
                boundary = page["cursor"]
                if {"surface.title_changed", "surface.directory_changed", "surface.status_changed", "surface.progress_changed"} <= {e["kind"] for e in metadata_events}:
                    break
            for kind in ("surface.title_changed", "surface.directory_changed", "surface.status_changed", "surface.progress_changed"):
                assert sum(e["kind"] == kind and e["surface_id"] == surface for e in metadata_events) == 1, metadata_events
            current = next(s for s in call("surface.list")["surfaces"] if s["id"] == surface)
            assert current["title"] == "Replay title 世界" and current["directory_label"] == str(reported_directory), current
            assert current["program_status"]["state"] == "working" and current["program_status"]["id"] == "build"
            assert current["progress"] == {"state": "running", "percent": 42}
            call("surface.send_text", {"surface_id": surface, "text": "metadata\n"})
            duplicate = call("events.read", {"after": boundary, "wait_ms": 100})
            assert not any(e["kind"] in ("surface.title_changed", "surface.directory_changed") for e in duplicate["events"])
            # OSC terminator BEL bytes above must not count as terminal bells.
            assert current["bell_revision"] == "0", current
            boundary = call("events.read")["cursor"]
            call("surface.send_text", {"surface_id": surface, "text": "bell\n"})
            bell_events = []
            deadline = time.monotonic() + 3
            while time.monotonic() < deadline:
                page = call("events.read", {"after": boundary, "wait_ms": 100, "surface_id": surface, "kinds": ["surface.bell"]})
                boundary = page["cursor"]
                bell_events.extend(page["events"])
                current = next(s for s in call("surface.list")["surfaces"] if s["id"] == surface)
                if current["bell_revision"] == "3" and bell_events:
                    break
            assert current["bell_revision"] == "3" and bell_events, (current, bell_events)
            assert all(e["kind"] == "surface.bell" and e["surface_id"] == surface for e in bell_events)
            assert call("events.read", {"after": boundary, "wait_ms": 50, "kinds": ["surface.bell"]})["events"] == []
            window = call("window.list")["windows"][0]["id"]
            original_workspace = call("workspace.list")["workspaces"][0]["id"]
            boundary = call("events.read")["cursor"]
            call("workspace.create", {"window_id": window, "name": "Event lifetime fixture"})
            destination = next(w["id"] for w in call("workspace.list")["workspaces"] if w["id"] != original_workspace)
            created_surface = next(s["id"] for s in call("surface.list")["surfaces"] if s["id"] != surface)
            created = call("events.read", {"after": boundary})
            assert sum(e["kind"] == "surface.created" and e["surface_id"] == created_surface for e in created["events"]) == 1
            workspace_events = call("events.read", {"after": boundary, "workspace_id": destination})
            assert [e["kind"] for e in workspace_events["events"]] == ["workspace.created"], workspace_events
            assert all("surface_id" not in e for e in workspace_events["events"])

            original_pid = next(s["process_id"] for s in call("surface.list")["surfaces"] if s["id"] == created_surface)
            call("window.create")
            other = next(w for w in call("workspace.list")["workspaces"] if w["window_id"] != window)
            for target_workspace, target_window in [(other["id"], other["window_id"]), (original_workspace, window)]:
                boundary = call("events.read")["cursor"]
                call("surface.move_to_workspace", {"surface_id": created_surface, "workspace_id": target_workspace})
                moved = call("events.read", {"after": boundary})
                assert not any(e["kind"] in ("surface.created", "surface.closed") and e["surface_id"] == created_surface for e in moved["events"])
                locations = call("events.read", {"after": boundary, "surface_id": created_surface, "kinds": ["surface.location_invalidated"]})
                assert locations["events"], moved
                moved_surface = next(s for s in call("surface.list")["surfaces"] if s["id"] == created_surface)
                assert moved_surface["workspace_id"] == target_workspace, moved_surface
                assert moved_surface["window_id"] == target_window, moved_surface
                assert moved_surface["process_id"] == original_pid, moved_surface
                os.kill(original_pid, 0)
                # A real output change proves the transferred entity still
                # owns working input/output, beyond a retained PID snapshot.
                bell_boundary = int(moved_surface["bell_revision"])
                call("surface.send_text", {"surface_id": created_surface, "text": "bell\n"})
                deadline = time.monotonic() + 3
                while True:
                    current = next(s for s in call("surface.list")["surfaces"] if s["id"] == created_surface)
                    if int(current["bell_revision"]) == bell_boundary + 3:
                        break
                    assert time.monotonic() < deadline, "transferred terminal did not respond"
                    time.sleep(.01)
            assert destination not in [w["id"] for w in call("workspace.list")["workspaces"]], "empty source workspace survived transfer"
            workspace_events = call("events.read", {"after": created["cursor"], "workspace_id": destination})
            assert [e["kind"] for e in workspace_events["events"]] == ["workspace.closed"], workspace_events
            # Keep another workspace alive so the application can answer after
            # closing both original terminal entities.
            call("workspace.create", {"window_id": window, "name": "Survivor"})
            boundary = call("events.read")["cursor"]
            call("workspace.close", {"workspace_id": original_workspace})
            closed = call("events.read", {"after": boundary})
            workspace_events = call("events.read", {"after": boundary, "workspace_id": original_workspace})
            assert [e["kind"] for e in workspace_events["events"]] == ["workspace.closed"], workspace_events
            for target in (surface, created_surface):
                assert sum(e["kind"] == "surface.closed" and e["surface_id"] == target for e in closed["events"]) == 1, closed
            call("system.quit")
            assert process.wait(timeout=5) == 0
            errors.seek(0)
            assert b"native quit confirmation unavailable" not in errors.read(), "native quit hook did not install"
            print("PASS replay, bounded waiting, concurrent mutation, cross-window transfer and surface lifetimes")
        finally:
            if process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait(timeout=5)
            selector.close()
