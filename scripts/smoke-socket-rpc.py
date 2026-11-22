#!/usr/bin/env python3
"""Isolated local-socket RPC lifecycle check; never attaches to user app instances."""
import argparse
import json
import os
from pathlib import Path
import socket
import subprocess
import tempfile
import time

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--app", type=Path, default=Path("target/debug/rmux"))
parser.add_argument("--client", type=Path, default=Path("target/debug/rmuxctl"))
parser.add_argument("--config-socket", action="store_true", help="Enable the app endpoint through Settings JSON, without RMUX_SOCKET_DIR")
parser.add_argument("--endpoint-only", action="store_true", help="Check endpoint discovery and normal-quit cleanup only")
options = parser.parse_args()

with tempfile.TemporaryDirectory(prefix="rmux-socket-") as tmp:
    root = Path(tmp)
    endpoint = root / "endpoint"
    config = root / "config.json"
    settings = {"confirm_close": "never"}
    if options.config_socket:
        settings["socket_directory"] = str(endpoint)
    config.write_text(json.dumps(settings))
    env = {k: v for k, v in os.environ.items() if not k.startswith("RMUX_")}
    shell = root / "fixture-shell"
    shell.write_text('#!/bin/sh\nprintf "fixture-dir:%s\\n" "${PWD##*/}"\ntrap "exit 0" HUP TERM\nwhile IFS= read -r line; do\n  case "$line" in\n    notify) printf "\\033]777;notify;Shell;After\\007" ;;\n    burst) printf "\\033]9;First burst\\007\\033]99;;Second burst\\007" ;;\n    ping) printf "ordinary output\\n" ;;\n  esac\ndone\n')
    shell.chmod(0o700)
    env.update(RMUX_SOCKET_DIR=str(endpoint), RMUX_CONFIG=str(config), RMUX_SHELL=str(shell))
    with tempfile.TemporaryFile() as log:
        app_env = dict(env)
        if options.config_socket:
            app_env.pop("RMUX_SOCKET_DIR")
        app = subprocess.Popen([str(options.app.resolve())], env=app_env, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=log)
        clients = []
        try:
            deadline = time.monotonic() + 5
            while not (endpoint / "rpc.sock").exists():
                assert app.poll() is None, "test app exited before socket became ready"
                assert time.monotonic() < deadline, "socket startup timed out"
                time.sleep(.01)
            def connect():
                client = socket.socket(socket.AF_UNIX)
                client.settimeout(5)
                client.connect(str(endpoint / "rpc.sock"))
                clients.append(client)
                return client.makefile("rwb", buffering=0)
            def call(stream, method, params=None):
                stream.write((json.dumps({"jsonrpc":"2.0", "id":1, "method":method, "params":params or {}}) + "\n").encode())
                result = json.loads(stream.readline())
                assert "error" not in result, result
                return result["result"]
            cli = str(options.client.resolve())
            discovery = subprocess.run([cli, "system.capabilities"], env=env, capture_output=True, text=True, timeout=8)
            assert discovery.returncode == 0, discovery.stderr
            if options.endpoint_only:
                stream = connect()
                assert call(stream, "window.list")["windows"]
                stream.write(b'{"jsonrpc":"2.0","id":2,"method":"system.quit"}\n')
                app.wait(timeout=5)
                stream.close()
                assert not endpoint.exists(), "normal quit left socket directory behind"
                print("PASS: endpoint startup, CLI discovery, window lookup and normal-quit cleanup")
                raise SystemExit(0)
            assert json.loads(discovery.stdout)["result"]["mode"] == "live"
            rejected = subprocess.run([cli, "--socket", str(endpoint / "rpc.sock"), "nonexistent.method"], capture_output=True, text=True, timeout=8)
            assert rejected.returncode == 1, rejected.stderr
            assert json.loads(rejected.stdout)["error"]["code"] == -32601
            first, second = connect(), connect()
            assert call(first, "system.capabilities")["mode"] == "live"
            original = call(first, "workspace.list")["workspaces"][0]
            renamed = subprocess.run([cli, "rename-workspace", "--workspace", original["id"], "Socket 世界"], env=env, capture_output=True, text=True, timeout=8)
            assert renamed.returncode == 0, renamed.stderr
            listed = subprocess.run([cli, "list-workspaces"], env=env, capture_output=True, text=True, timeout=8)
            assert listed.returncode == 0, listed.stderr
            assert json.loads(listed.stdout)["result"]["workspaces"][0]["name"] == "Socket 世界"
            assert call(second, "workspace.list")["workspaces"][0]["name"] == "Socket 世界"
            call(first, "window.create")
            all_workspaces = call(first, "workspace.list")["workspaces"]
            assert len(all_workspaces) == 2, all_workspaces
            scoped = call(first, "workspace.list", {"window_id": original["window_id"]})["workspaces"]
            assert [row["id"] for row in scoped] == [original["id"]]
            panels = subprocess.run([cli, "list-panels", "--window", original["window_id"], "--workspace", original["id"]], env=env, capture_output=True, text=True, timeout=8)
            assert panels.returncode == 0, panels.stderr
            surfaces = json.loads(panels.stdout)["result"]["surfaces"]
            assert surfaces and all(row["workspace_id"] == original["id"] for row in surfaces)
            surface = surfaces[0]["id"]
            # Route the selected-text wrap option through the real CLI, socket,
            # UI target lookup and background exporter. This fixture has no selection.
            selection_read = subprocess.run([cli, "read-screen", "--surface", surface, "--selection", "--preserve-wraps"], env=env, capture_output=True, text=True, timeout=8)
            assert selection_read.returncode == 0, (selection_read.stdout, selection_read.stderr)
            selection_result = json.loads(selection_read.stdout)["result"]
            assert selection_result["scope"] == "selection" and selection_result["text"] == "", selection_result
            assert selection_result["truncated"] is False, selection_result
            for extra in [
                {"preserve_wraps": True},
                {"scope": "history", "preserve_wraps": True},
                {"scope": "selection", "preserve_wraps": "true"},
                {"scope": "selection", "preserve_wraps": None},
            ]:
                first.write((json.dumps({"jsonrpc":"2.0", "id":"invalid-wrap", "method":"surface.read_text", "params":{"surface_id":surface, **extra}}) + "\n").encode())
                rejected_read = json.loads(first.readline())
                assert rejected_read["id"] == "invalid-wrap" and rejected_read["error"]["code"] == -32602, rejected_read
            assert call(second, "surface.read_text", {"surface_id":surface,"scope":"selection"})["text"] == ""
            pane_id = surfaces[0]["pane_id"]
            original_pid = surfaces[0]["process_id"]
            before_selection = [(row["id"], row["selected"]) for row in call(first, "workspace.list")["workspaces"]]
            split_command = subprocess.run([cli, "split-pane", "--pane", pane_id, "--direction", "right", "--cwd", str(root), "--focus", "false"], env=env, capture_output=True, text=True, timeout=8)
            assert split_command.returncode == 0, (split_command.stdout, split_command.stderr)
            split = json.loads(split_command.stdout)["result"]
            nested = call(first, "pane.split", {"pane_id":split["pane_id"],"direction":"down"})
            assert split["workspace_id"] == nested["workspace_id"] == original["id"]
            panes = call(first, "pane.list", {"workspace_id":original["id"]})["panes"]
            assert [pane["id"] for pane in panes] == [pane_id, split["pane_id"], nested["pane_id"]], panes
            after = call(first, "surface.list", {"workspace_id":original["id"]})["surfaces"]
            assert next(row for row in after if row["id"] == surface)["process_id"] == original_pid
            deadline = time.monotonic() + 3
            while True:
                text = call(first, "surface.read_text", {"surface_id":nested["surface_id"]})["text"]
                if "fixture-dir:" + root.name in text: break
                assert time.monotonic() < deadline, text
                time.sleep(.01)
            workspaces = call(first, "workspace.list")["workspaces"]
            assert [(row["id"], row["selected"]) for row in workspaces] == before_selection
            tree = next(row for row in workspaces if row["id"] == original["id"])["layout"]
            assert tree["axis"] == "columns" and tree["children"][1]["axis"] == "rows", tree
            call(first, "pane.zoom.set", {"pane_id":pane_id,"zoomed":True})
            before_failure = call(first, "pane.list", {"workspace_id":original["id"]})
            failed_split = subprocess.run([cli, "pane.split", json.dumps({"pane_id":pane_id,"direction":"down","cwd":str(root / "missing")})], env=env, capture_output=True, text=True, timeout=8)
            assert failed_split.returncode == 1, (failed_split.stdout, failed_split.stderr)
            assert call(first, "pane.list", {"workspace_id":original["id"]}) == before_failure
            call(first, "pane.zoom.set", {"pane_id":pane_id,"zoomed":False})
            # Zoom/unzoom can focus the original terminal. Make the unread
            # precondition explicit instead of relying on OS window activation.
            call(first, "surface.focus", {"surface_id":nested["surface_id"]})
            deadline = time.monotonic() + 3
            while True:
                target = next(row for row in call(first, "surface.list", {"workspace_id":original["id"]})["surfaces"] if row["id"] == surface)
                if not target["input_focused"]: break
                assert time.monotonic() < deadline, "notification target did not lose input focus"
                time.sleep(.01)
            notification = subprocess.run([cli, "notify", "--surface", surface, "--title", "Build 世界", "--body", "Complete"], env=env, capture_output=True, text=True, timeout=8)
            assert notification.returncode == 0, (notification.stdout, notification.stderr)
            old_revision = json.loads(notification.stdout)["result"]["revision"]
            newest = call(first, "notification.send", {"surface_id":surface,"body":"Review ready"})
            assert newest["revision"] > old_revision
            retained = call(first, "notification.list", {"workspace_id":original["id"]})["notifications"]
            assert [message["revision"] for message in retained] == [newest["revision"], old_revision], retained
            assert all(message["unread"] for message in retained), retained
            selection_before_read = [(row["id"], row["selected"]) for row in call(first, "workspace.list")["workspaces"]]
            acknowledgement = subprocess.run([cli, "read-notification", "--surface", surface, "--revision", str(old_revision)], env=env, capture_output=True, text=True, timeout=8)
            assert acknowledgement.returncode == 0, (acknowledgement.stdout, acknowledgement.stderr)
            assert json.loads(acknowledgement.stdout)["result"]["marked_read"]
            assert not call(first, "notification.mark_read", {"surface_id":surface,"revision":old_revision})["marked_read"]
            assert not call(first, "notification.mark_read", {"surface_id":surface,"revision":18446744073709551615})["marked_read"]
            acknowledged = call(first, "notification.list", {"workspace_id":original["id"]})["notifications"]
            assert len(acknowledged) == 2, acknowledged
            by_revision = {message["revision"]: message for message in acknowledged}
            assert not by_revision[old_revision]["unread"] and by_revision[newest["revision"]]["unread"], acknowledged
            assert by_revision[old_revision]["title"] == "Build 世界", acknowledged
            assert [(row["id"], row["selected"]) for row in call(first, "workspace.list")["workspaces"]] == selection_before_read
            dismissal = subprocess.run([cli, "dismiss-notification", "--surface", surface, "--revision", str(old_revision)], env=env, capture_output=True, text=True, timeout=8)
            assert dismissal.returncode == 0, (dismissal.stdout, dismissal.stderr)
            assert json.loads(dismissal.stdout)["result"]["dismissed"]
            assert not call(first, "notification.dismiss", {"surface_id":surface,"revision":old_revision})["dismissed"]
            messages = call(first, "notification.list", {"workspace_id":original["id"]})["notifications"]
            assert len(messages) == 1 and messages[0]["body"] == "Review ready", messages
            assert call(first, "notification.dismiss", {"surface_id":surface,"revision":newest["revision"]})["dismissed"]
            assert call(first, "notification.list", {"workspace_id":original["id"]})["notifications"] == []
            call(first, "surface.send_text", {"surface_id":surface,"text":"notify"})
            entered = subprocess.run([cli, "send-key", "--surface", surface, "Enter"], env=env, capture_output=True, text=True, timeout=8)
            assert entered.returncode == 0, (entered.stdout, entered.stderr)
            deadline = time.monotonic() + 3
            while True:
                messages = call(first, "notification.list", {"workspace_id":original["id"]})["notifications"]
                if messages and messages[0]["title"] == "Shell": break
                assert time.monotonic() < deadline, "shell notification did not reach UI"
                time.sleep(.01)
            assert messages[0]["revision"] > newest["revision"]
            replacement = call(first, "notification.send", {"surface_id":surface,"body":"RPC after OSC"})
            pasted = subprocess.run([cli, "send", "--surface", surface, "--paste", "ping\n"], env=env, capture_output=True, text=True, timeout=8)
            assert pasted.returncode == 0, (pasted.stdout, pasted.stderr)
            assert json.loads(pasted.stdout)["result"]["paste"] is True
            deadline = time.monotonic() + 3
            while "ordinary output" not in call(first, "surface.read_text", {"surface_id":surface})["text"]:
                assert time.monotonic() < deadline, "automation paste was not delivered"
                time.sleep(.01)
            messages = call(first, "notification.list", {"workspace_id":original["id"]})["notifications"]
            assert messages[0]["body"] == "RPC after OSC" and messages[0]["revision"] == replacement["revision"], messages

            call(first, "surface.send_text", {"surface_id":surface,"text":"burst\n"})
            deadline = time.monotonic() + 3
            while True:
                messages = call(first, "notification.list", {"workspace_id":original["id"]})["notifications"]
                if len(messages) >= 4: break
                assert time.monotonic() < deadline, "notification burst lost history"
                time.sleep(.01)
            assert messages[0]["title"] == "Second burst", messages
            assert messages[1]["body"] == "First burst", messages

            other = next(row for row in all_workspaces if row["id"] != original["id"])
            assert call(first, "pane.list", {"window_id": other["window_id"], "workspace_id": original["id"]})["panes"] == []
            invalid = subprocess.run([cli, "list-panels", "--workspace", "not-an-id"], env=env, capture_output=True, text=True, timeout=8)
            assert invalid.returncode == 1 and json.loads(invalid.stdout)["error"]["code"] == -32602
            created = subprocess.run([cli, "new-workspace", "--window", original["window_id"], "--name", "Background 世界", "--cwd", str(root)], env=env, capture_output=True, text=True, timeout=8)
            assert created.returncode == 0, (created.stdout, created.stderr)
            workspace = json.loads(created.stdout)["result"]
            assert workspace["name"] == "Background 世界" and not workspace["selected"], workspace
            scoped = call(first, "workspace.list", {"window_id": original["window_id"]})["workspaces"]
            assert len(scoped) == 2 and next(row for row in scoped if row["id"] == original["id"])["selected"]
            focused = call(first, "workspace.create", {"window_id": original["window_id"], "name":"Focused", "focus":True})
            assert focused["selected"]
            before_failure = call(first, "workspace.list", {"window_id":original["window_id"]})
            failed = subprocess.run([cli, "new-workspace", "--window", original["window_id"], "--cwd", str(root / "missing")], env=env, capture_output=True, text=True, timeout=8)
            assert failed.returncode == 1, (failed.stdout, failed.stderr)
            assert call(first, "workspace.list", {"window_id":original["window_id"]}) == before_failure
            closing_surfaces = call(first, "surface.list", {"workspace_id": workspace["id"]})["surfaces"]
            closed = subprocess.run([cli, "close-workspace", "--workspace", workspace["id"]], env=env, capture_output=True, text=True, timeout=8)
            assert closed.returncode == 0, (closed.stdout, closed.stderr)
            assert json.loads(closed.stdout)["result"]["window_closed"] is False
            remaining = call(first, "workspace.list", {"window_id":original["window_id"]})["workspaces"]
            assert len(remaining) == 2 and next(row for row in remaining if row["id"] == focused["id"])["selected"]
            assert call(first, "workspace.close", {"workspace_id":other["id"]})["window_closed"] is True
            assert len(call(first, "window.list")["windows"]) == 1
            call(first, "workspace.close", {"workspace_id":focused["id"]})
            remaining = call(first, "workspace.list")["workspaces"]
            assert len(remaining) == 1 and remaining[0]["id"] == original["id"] and remaining[0]["selected"]
            deadline = time.monotonic() + 3
            for terminal in closing_surfaces:
                pid = terminal["process_id"]
                if pid is None: continue
                while True:
                    try: os.kill(pid, 0)
                    except ProcessLookupError: break
                    assert time.monotonic() < deadline, "closed workspace shell was not reaped"
                    time.sleep(.01)
            first.close()
            clients[0].close()
            third = connect()
            assert call(third, "workspace.list")["workspaces"][0]["id"] == original["id"]
            second.write(b'{"jsonrpc":"2.0","id":2,"method":"system.quit"}\n')
            app.wait(timeout=5)
            assert not endpoint.exists(), "normal quit left socket directory behind"
            second.close()
            third.close()
            print("Socket RPC passed: bundled/client discovery and error exits, private endpoint, two clients, shared UI mutation, cross-window filters, targeted nested splits and failure atomicity, selected-text wrap routing/validation, notification acknowledgement/history and OSC coexistence, workspace creation/close and shell cleanup, reconnect, normal-quit cleanup.")
        finally:
            for client in clients: client.close()
            if app.poll() is None:
                app.terminate()
                try: app.wait(timeout=3)
                except subprocess.TimeoutExpired:
                    app.kill(); app.wait(timeout=3)
            if app.returncode:
                log.seek(0)
                print(log.read().decode(errors="replace"))
