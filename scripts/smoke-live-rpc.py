#!/usr/bin/env python3
"""Short Unix native live-RPC smoke check; requires a usable GUI session.

Run through mise after building rmux. Opens one test window with /usr/bin/false
as its shell; terminates only the process it starts. Never attaches to other apps.
"""
import json
import argparse
import os
from pathlib import Path
import selectors
import subprocess
import sys
import tempfile
import time


def run(binary, live_shell=False):
    with tempfile.TemporaryDirectory(prefix="rmux-rpc-smoke-") as directory:
        config = Path(directory) / "config.json"
        config.write_text("{}")
        def pane(*labels):
            return {"kind": "pane", "active_tab": 0,
                    "tabs": [{"label": label, "directory": directory} for label in labels]}
        layout = Path(directory) / "layout.json"
        layout.write_text(json.dumps({
            "version": 1, "sidebar": True, "active_workspace": 0,
            "workspaces": [
                {"name": "RPC split", "active_pane": 0,
                 "layout": {"kind": "split", "axis": "Columns", "weights": [2, 1],
                            "children": [pane("first", "second"), pane("third")]}},
                {"name": "RPC other", "active_pane": 0, "layout": pane("fourth")},
            ],
        }))
        env = {key: value for key, value in os.environ.items() if not key.startswith("RMUX_")}
        env.update(RMUX_CONFIG=str(config), RMUX_SHELL="/usr/bin/false")
        if live_shell:
            fixture = Path(directory) / "fixture-shell"
            fixture.write_text('#!/bin/sh\ntrap "exit 0" HUP TERM\nprintf "alive-%s" "$$" > "$RMUX_TEST_PROCESS_DIR/$$.ready"\ntoken=initial\nwhile IFS= read -r line; do\n  case "$line" in\n    fail) exit 7 ;;\n    set:*) token=${line#set:} ;;\n    probe) printf "%s" "$token" > "$RMUX_TEST_PROCESS_DIR/$$.state"; printf "reported-state:%s\\n" "$token" ;;\n  esac\ndone\n')
            fixture.chmod(0o700)
            env.update(RMUX_SHELL=str(fixture), RMUX_TEST_PROCESS_DIR=directory)
        child_pids = set()
        with tempfile.TemporaryFile() as stderr:
            process = subprocess.Popen([str(binary), "--rpc-stdio-live", "--layout", str(layout)], env=env,
                                       stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                       stderr=stderr, bufsize=0)
            selector = selectors.DefaultSelector()
            selector.register(process.stdout, selectors.EVENT_READ)
            pending = bytearray()
            sequence = 0
            checks = []

            def wait_file(path, expected):
                deadline = time.monotonic() + 3
                while time.monotonic() < deadline:
                    if path.exists() and path.read_text() == expected:
                        return
                    time.sleep(0.01)
                actual = path.read_text() if path.exists() else "<missing>"
                raise AssertionError(f"fixture state mismatch in {path.name}: expected {expected!r}, got {actual!r}")

            def send(method, params=None, notification=False):
                nonlocal sequence
                sequence += 1
                request = {"jsonrpc": "2.0", "method": method}
                if not notification:
                    request["id"] = sequence
                if params is not None:
                    request["params"] = params
                process.stdin.write((json.dumps(request) + "\n").encode())
                if notification:
                    return
                deadline = time.monotonic() + 8
                while b"\n" not in pending:
                    remaining = deadline - time.monotonic()
                    if remaining <= 0 or not selector.select(remaining):
                        raise RuntimeError(f"timed out waiting for {method}")
                    chunk = os.read(process.stdout.fileno(), 65536)
                    if not chunk:
                        raise RuntimeError(f"RPC stream closed during {method}")
                    pending.extend(chunk)
                line, _, rest = pending.partition(b"\n")
                pending[:] = rest
                response = json.loads(line)
                assert response["id"] == sequence, response
                return response

            try:
                capabilities = send("system.capabilities")["result"]
                assert capabilities["mode"] == "live"
                assert "surface.focus" in capabilities["methods"]
                windows = send("window.list")["result"]["windows"]
                workspaces = send("workspace.list")["result"]["workspaces"]
                panes = send("pane.list")["result"]["panes"]
                surfaces = send("surface.list")["result"]["surfaces"]
                assert (len(windows), len(workspaces), len(panes), len(surfaces)) == (1, 2, 3, 4)
                window, workspace, pane, surface = windows[0], workspaces[0], panes[0], surfaces[0]
                assert workspace["window_id"] == window["id"]
                assert pane["workspace_id"] == surface["workspace_id"] == workspace["id"]
                assert surface["pane_id"] == workspace["layout"]["children"][0]["pane_id"] == pane["id"]
                assert workspace["layout"]["axis"] == "columns"
                assert workspace["layout"]["weights"] == [2, 1]
                assert pane["selected_surface_id"] == surface["id"]
                checks.append("live discovery and cross-level identity")
                rename_target = workspaces[1]["id"]
                assert send("workspace.rename", {"workspace_id": rename_target, "name": "  \u0007 \u0007  "})["error"]["code"] == -32602
                assert send("workspace.rename", {"workspace_id": "missing", "name": "name"})["error"]["code"] == -32004
                renamed = send("workspace.rename", {"workspace_id": rename_target, "name": "  計画\n v2  "})["result"]
                assert renamed == {"workspace_id": rename_target, "name": "計画 v2"}
                after_rename = send("workspace.list")["result"]["workspaces"]
                assert next(item for item in after_rename if item["id"] == rename_target)["name"] == "計画 v2"
                assert [item["id"] for item in after_rename if item["selected"]] == [workspace["id"]]
                checks.append("rename inactive workspace with normalized Unicode name without selecting it")
                original_ids = {item["id"] for item in surfaces}
                for target in surfaces[1:]:
                    assert send("surface.focus", {"surface_id": target["id"]})["result"]["selected"]
                    current = send("surface.list")["result"]["surfaces"]
                    assert {item["id"] for item in current} == original_ids
                    assert next(item for item in current if item["id"] == target["id"])["selected"]
                    assert not any(item["selected"] for item in current if item["pane_id"] == target["pane_id"] and item["id"] != target["id"])
                    selected = [item["id"] for item in send("workspace.list")["result"]["workspaces"] if item["selected"]]
                    assert selected == [target["workspace_id"]]
                checks.append("switch inactive tab, sibling pane and other workspace without replacing identities")
                assert send("pane.zoom.set", {"pane_id": pane["id"]})["error"]["code"] == -32602
                assert send("pane.zoom.set", {"pane_id": "missing", "zoomed": True})["error"]["code"] == -32004
                for _ in range(2):
                    assert send("pane.zoom.set", {"pane_id": pane["id"], "zoomed": True})["result"]["zoomed"]
                sibling = next(item for item in panes if item["workspace_id"] == workspace["id"] and item["id"] != pane["id"])
                assert not send("pane.zoom.set", {"pane_id": sibling["id"], "zoomed": False})["result"]["zoomed"]
                def current_workspace():
                    return next(item for item in send("workspace.list")["result"]["workspaces"] if item["id"] == workspace["id"])
                assert current_workspace()["zoomed_pane_id"] == pane["id"]
                sibling_surface = next(item for item in surfaces if item["pane_id"] == sibling["id"])
                assert send("surface.focus", {"surface_id": sibling_surface["id"]})["result"]["selected"]
                assert current_workspace()["zoomed_pane_id"] == sibling["id"]
                assert not send("pane.zoom.set", {"pane_id": sibling["id"], "zoomed": False})["result"]["zoomed"]
                restored = current_workspace()
                assert restored["zoomed_pane_id"] is None
                assert restored["layout"] == workspace["layout"]
                assert {item["id"] for item in send("surface.list")["result"]["surfaces"]} == original_ids
                checks.append("idempotent zoom, focus follows sibling, unzoom preserves layout and IDs")
                before_swap = send("surface.list")["result"]["surfaces"]
                identities = {item["id"]: (item["pane_id"], item.get("process_id")) for item in before_swap}
                assert send("pane.zoom.set", {"pane_id": pane["id"], "zoomed": True})["result"]["zoomed"]
                swap = {"pane_id": pane["id"], "other_pane_id": sibling["id"]}
                assert "error" in send("pane.swap", swap)
                assert current_workspace()["zoomed_pane_id"] == pane["id"]
                assert current_workspace()["layout"] == restored["layout"]
                assert not send("pane.zoom.set", {"pane_id": pane["id"], "zoomed": False})["result"]["zoomed"]
                assert send("pane.swap", swap)["result"]["changed"]
                swapped = current_workspace()
                assert swapped["zoomed_pane_id"] is None
                assert swapped["layout"] != restored["layout"]
                after_swap = send("surface.list")["result"]["surfaces"]
                assert {item["id"]: (item["pane_id"], item.get("process_id")) for item in after_swap} == identities
                assert send("pane.swap", swap)["result"]["changed"]
                assert current_workspace()["layout"] == restored["layout"]
                checks.append("pane swap rejects zoom without mutation, preserves surface/pane/process identities and round-trips layout")
                assert send("workspace.select", {"workspace_id": workspace["id"]})["result"]["selected"]
                assert send("surface.focus", {"surface_id": surface["id"]})["result"]["selected"]
                assert send("surface.focus", {"surface_id": "missing"})["error"]["code"] == -32004
                assert send("workspace.select", {"workspace_id": "missing"})["error"]["code"] == -32004
                checks.append("selection and stale-ID errors")
                send("surface.focus", {"surface_id": surface["id"]}, notification=True)
                assert send("system.ping")["result"]["pong"]
                assert send("surface.list")["result"]["surfaces"][0]["selected"]
                checks.append("notification has no response; subsequent request remains aligned")
                assert send("window.create", {"activate": "yes"})["error"]["code"] == -32602
                created = send("window.create", {"activate": False})["result"]
                assert created["id"] != window["id"]
                assert len(send("window.list")["result"]["windows"]) == 2
                all_surfaces = send("surface.list")["result"]["surfaces"]
                new_surfaces = [item for item in all_surfaces if item["window_id"] == created["id"]]
                assert len(new_surfaces) == 1
                assert original_ids.issubset({item["id"] for item in all_surfaces})
                remote = new_surfaces[0]
                assert remote["workspace_id"] == created["selected_workspace_id"]
                assert send("surface.focus", {"surface_id": remote["id"]})["result"]["workspace_id"] == remote["workspace_id"]
                assert send("surface.focus", {"surface_id": surfaces[-1]["id"]})["result"]["workspace_id"] == surfaces[-1]["workspace_id"]
                after = {item["id"]: item for item in send("window.list")["result"]["windows"]}
                assert after[window["id"]]["selected_workspace_id"] == surfaces[-1]["workspace_id"]
                assert after[created["id"]]["selected_workspace_id"] == remote["workspace_id"]
                checks.append("create second window and route surface focus across windows")
                all_ids = {item["id"] for item in all_surfaces}
                original_processes = {item["id"]: item["process_id"] for item in all_surfaces}
                if live_shell:
                    child_pids.update(original_processes.values())
                    assert None not in child_pids and len(child_pids) == len(all_surfaces)
                    for pid in child_pids:
                        marker = Path(directory) / f"{pid}.ready"
                        wait_file(marker, f"alive-{pid}")
                        os.kill(pid, 0)
                    assert not any(item["process_exited"] for item in all_surfaces)
                    assert send("surface.send_text", {"surface_id": surface["id"], "text": "x" * 16385})["error"]["code"] == -32602
                    assert send("surface.send_text", {"surface_id": "missing", "text": "probe\n"})["error"]["code"] == -32004
                    before_input = send("window.list")["result"]
                    for surface_id, pid in original_processes.items():
                        text = f"set:state-{pid}-世界\nprobe\n"
                        assert send("surface.send_text", {"surface_id": surface_id, "text": text})["result"]["queued_bytes"] == len(text.encode())
                        try:
                            wait_file(Path(directory) / f"{pid}.state", f"state-{pid}-世界")
                        except AssertionError as error:
                            screen = send("surface.read_text", {"surface_id": surface_id, "scope": "screen", "max_bytes": 4096})
                            raise AssertionError(f"{error}; fixture screen: {screen}") from error
                    assert send("window.list")["result"] == before_input
                    assert [(item["id"], item["selected"]) for item in send("surface.list")["result"]["surfaces"]] == [(item["id"], item["selected"]) for item in all_surfaces]
                    checks.append("UTF-8 input reaches each PTY without changing workspace/tab selection")
                # Choose a multi-tab source so this round trip retains the
                # original layout for the workspace-transfer assertions below.
                candidate = next(item for item in all_surfaces if sum(other["pane_id"] == item["pane_id"] for other in all_surfaces) > 1)
                tab_move = {"surface_id": candidate["id"], "workspace_id": remote["workspace_id"]}
                assert send("surface.move_to_workspace", {**tab_move, "workspace_id": "missing"})["error"]["code"] == -32004
                assert send("surface.move_to_workspace", {**tab_move, "surface_id": "missing"})["error"]["code"] == -32004
                for _ in range(2):
                    assert send("surface.move_to_workspace", tab_move)["result"] == tab_move
                transferred = send("surface.list")["result"]["surfaces"]
                assert {item["id"] for item in transferred} == all_ids
                moved_tab = next(item for item in transferred if item["id"] == candidate["id"])
                assert moved_tab["workspace_id"] == remote["workspace_id"] and moved_tab["window_id"] == created["id"]
                assert moved_tab["process_id"] == candidate["process_id"] and moved_tab["selected"]
                tab_return = {"surface_id": candidate["id"], "workspace_id": candidate["workspace_id"]}
                assert send("surface.move_to_workspace", tab_return)["result"] == tab_return
                assert {item["id"]: item["process_id"] for item in send("surface.list")["result"]["surfaces"]} == original_processes
                checks.append("cross-window tab round trip retains identities/PIDs, selects destination and rejects stale IDs")
                move = {"workspace_id": workspace["id"], "window_id": created["id"]}
                assert send("workspace.move_to_window", {"workspace_id": workspace["id"], "window_id": "missing"})["error"]["code"] == -32004
                for _ in range(2):
                    assert send("workspace.move_to_window", move)["result"] == move
                moved = current_workspace()
                assert moved["window_id"] == created["id"] and moved["layout"] == workspace["layout"]
                moved_surfaces = send("surface.list")["result"]["surfaces"]
                assert {item["id"] for item in moved_surfaces} == all_ids
                assert all(item["window_id"] == created["id"] for item in moved_surfaces if item["workspace_id"] == workspace["id"])
                assert len(send("workspace.list")["result"]["workspaces"]) == 3
                assert send("workspace.move_to_window", {"workspace_id": workspaces[1]["id"], "window_id": created["id"]})["result"]["window_id"] == created["id"]
                remaining_windows = send("window.list")["result"]["windows"]
                assert len(remaining_windows) == 1 and remaining_windows[0]["id"] == created["id"]
                assert {item["id"] for item in send("surface.list")["result"]["surfaces"]} == all_ids
                assert send("surface.focus", {"surface_id": surface["id"]})["result"]["selected"]
                checks.append("move populated and last workspaces, retain layout/IDs and remove empty source window")
                if live_shell:
                    current = send("surface.list")["result"]["surfaces"]
                    assert {item["id"]: item["process_id"] for item in current} == original_processes
                    assert not any(item["process_exited"] for item in current)
                    for pid in child_pids:
                        os.kill(pid, 0)
                    for surface_id, pid in original_processes.items():
                        state = Path(directory) / f"{pid}.state"
                        state.unlink()
                        assert send("surface.send_text", {"surface_id": surface_id, "text": "probe\n"})["result"]["queued_bytes"] == 6
                        wait_file(state, f"state-{pid}-世界")
                        deadline = time.monotonic() + 3
                        while True:
                            snapshot = send("surface.read_text", {"surface_id": surface_id})
                            # Viewport reads preserve visual row breaks, including
                            # soft wraps in the narrow split pane.
                            if "result" in snapshot and f"reported-state:state-{pid}-世界" in snapshot["result"]["text"].replace("\n", ""):
                                assert snapshot["result"]["scope"] == "viewport"
                                assert not snapshot["result"]["truncated"]
                                break
                            assert "error" not in snapshot or snapshot["error"]["code"] == -32007
                            assert time.monotonic() < deadline, snapshot
                            time.sleep(0.01)
                        deadline = time.monotonic() + 3
                        while True:
                            history = send("surface.read_text", {"surface_id": surface_id, "scope": "history"})
                            if "result" in history:
                                assert history["result"]["scope"] == "history"
                                assert f"reported-state:state-{pid}-世界" in history["result"]["text"]
                                assert not history["result"]["truncated"]
                                break
                            assert history["error"]["code"] == -32007 and time.monotonic() < deadline, history
                            time.sleep(0.01)
                    short = send("surface.read_text", {"surface_id": surface["id"], "max_bytes": 5})["result"]
                    assert len(short["text"].encode()) <= 5 and short["truncated"]
                    assert send("surface.read_text", {"surface_id": surface["id"], "max_bytes": 0})["error"]["code"] == -32602
                    history_short = send("surface.read_text", {"surface_id": surface["id"], "scope": "history", "max_bytes": 5})["result"]
                    assert len(history_short["text"].encode()) <= 5 and history_short["truncated"]
                    assert send("surface.read_text", {"surface_id": surface["id"], "scope": "invalid"})["error"]["code"] == -32602
                    checks.append("background history reads unwrap Unicode output and enforce byte limits and valid scopes")
                    selection = send("surface.read_text", {"surface_id": surface["id"], "scope": "selection", "max_bytes": 5})["result"]
                    assert selection["scope"] == "selection" and selection["text"] == "" and not selection["truncated"]
                    checks.append("background selection read returns empty text when no selection exists")
                    deadline = time.monotonic() + 3
                    while True:
                        memory = send("surface.memory_usage", {"surface_id": surface["id"]})
                        if "result" in memory:
                            memory = memory["result"]
                            assert memory["surface_id"] == surface["id"] and memory["scope"] == "ghostty_screen_storage"
                            assert memory["primary_resident_bytes"] > 0
                            assert memory["virtual_bytes"] >= memory["primary_resident_bytes"] + memory["alternate_resident_bytes"]
                            break
                        assert memory["error"]["code"] == -32007 and time.monotonic() < deadline, memory
                        time.sleep(0.01)
                    checks.append("on-demand background memory sample reports addressed Ghostty screen storage")
                    checks.append("viewport reads return live Unicode output with explicit bounded truncation")
                    checks.append("five running fixture-shell PIDs survive both workspace transfers")
                    checks.append("shell variable values survive transfer and remain reachable by targeted input")
                # Exercise removal of the sole tab/pane/workspace/window.
                last_window = send("window.create", {"activate": False})["result"]
                last_tab = next(item for item in send("surface.list")["result"]["surfaces"] if item["window_id"] == last_window["id"])
                if live_shell:
                    last_pid = last_tab["process_id"]
                    child_pids.add(last_pid)
                    wait_file(Path(directory) / f"{last_pid}.ready", f"alive-{last_pid}")
                    assert "result" in send("surface.send_text", {"surface_id": last_tab["id"], "text": "set:last-tab-世界\nprobe\n"})
                    wait_file(Path(directory) / f"{last_pid}.state", "last-tab-世界")
                last_move = {"surface_id": last_tab["id"], "workspace_id": workspace["id"]}
                assert send("surface.move_to_workspace", last_move)["result"] == last_move
                windows_after = send("window.list")["result"]["windows"]
                assert len(windows_after) == 1 and windows_after[0]["id"] == created["id"]
                assert last_window["selected_workspace_id"] not in {item["id"] for item in send("workspace.list")["result"]["workspaces"]}
                tabs_after = send("surface.list")["result"]["surfaces"]
                assert {item["id"] for item in tabs_after} == all_ids | {last_tab["id"]}
                retained = next(item for item in tabs_after if item["id"] == last_tab["id"])
                assert retained["process_id"] == last_tab["process_id"] and retained["workspace_id"] == workspace["id"]
                if live_shell:
                    assert not retained["process_exited"]
                    (Path(directory) / f"{last_pid}.state").unlink()
                    assert "result" in send("surface.send_text", {"surface_id": last_tab["id"], "text": "probe\n"})
                    wait_file(Path(directory) / f"{last_pid}.state", "last-tab-世界")
                assert send("surface.move_to_workspace", {"surface_id": last_tab["id"], "workspace_id": last_window["selected_workspace_id"]})["error"]["code"] == -32004
                checks.append("moving the sole tab closes its source window/workspace and preserves the shell and state")
                def merge_and_check(source_id, destination_id):
                    before = {item["id"]: item for item in send("workspace.list")["result"]["workspaces"]}
                    source, destination = before[source_id], before[destination_id]
                    terminals = send("surface.list")["result"]["surfaces"]
                    # Use a hidden tab: focusing the merged workspace must not
                    # acknowledge messages in a tab the user has not opened.
                    hidden = next(item for item in terminals
                                  if item["workspace_id"] in (source_id, destination_id) and not item["selected"])
                    assert "result" in send("notification.send", {
                        "surface_id": hidden["id"], "title": "Merge unread 世界", "body": source_id})
                    messages_before = [item for item in send("notification.list")["result"]["notifications"]
                                       if item["surface_id"] == hidden["id"]]
                    assert messages_before and all(item["unread"] for item in messages_before)
                    params = {"workspace_id": source_id, "target_workspace_id": destination_id}
                    assert send("workspace.merge", params)["result"] == {**params, "merged": True}
                    after = {item["id"]: item for item in send("workspace.list")["result"]["workspaces"]}
                    assert set(after) == set(before) - {source_id}
                    merged = after[destination_id]
                    assert merged["layout"]["axis"] == "columns"
                    assert merged["layout"]["children"] == [destination["layout"], source["layout"]]
                    assert merged["zoomed_pane_id"] is None
                    retained = send("surface.list")["result"]["surfaces"]
                    def tab_positions(items):
                        return {item["id"]: (item["pane_id"], item["index"], item["selected"]) for item in items}
                    assert tab_positions(retained) == tab_positions(terminals)
                    messages_after = [item for item in send("notification.list")["result"]["notifications"]
                                      if item["surface_id"] == hidden["id"]]
                    assert messages_after == [{**item, "workspace_id": destination_id,
                                               "window_id": destination["window_id"]} for item in messages_before]
                    assert {item["id"]: item["process_id"] for item in retained} == {item["id"]: item["process_id"] for item in terminals}
                    moved_ids = {item["id"] for item in terminals if item["workspace_id"] in (source_id, destination_id)}
                    assert all(item["workspace_id"] == destination_id and item["window_id"] == destination["window_id"] for item in retained if item["id"] in moved_ids)
                    selected = next(item for item in send("window.list")["result"]["windows"] if item["id"] == destination["window_id"])
                    assert selected["selected_workspace_id"] == destination_id
                    assert send("workspace.merge", params)["error"]["code"] == -32004
                    assert {item["id"]: item for item in send("workspace.list")["result"]["workspaces"]} == after

                merge_and_check(workspaces[1]["id"], workspace["id"])
                checks.append("same-window merge preserves layouts, tab positions/selections and hidden-tab unread messages; stale source rejected without mutation")
                merge_window = send("window.create", {"activate": False})["result"]
                merge_target = merge_window["selected_workspace_id"]
                merge_tab = next(item for item in send("surface.list")["result"]["surfaces"] if item["workspace_id"] == merge_target)
                expected_states = {**{pid: f"state-{pid}-世界" for pid in original_processes.values()}, last_tab["process_id"]: "last-tab-世界"}
                if live_shell:
                    child_pids.add(merge_tab["process_id"])
                    wait_file(Path(directory) / f'{merge_tab["process_id"]}.ready', f'alive-{merge_tab["process_id"]}')
                    expected_states[merge_tab["process_id"]] = "initial"
                merge_and_check(workspace["id"], merge_target)
                merge_and_check(remote["workspace_id"], merge_target)
                remaining = send("window.list")["result"]["windows"]
                assert len(remaining) == 1 and remaining[0]["id"] == merge_window["id"]
                if live_shell:
                    for item in send("surface.list")["result"]["surfaces"]:
                        pid = item["process_id"]
                        assert not item["process_exited"]
                        state = Path(directory) / f"{pid}.state"
                        state.unlink(missing_ok=True)
                        assert "result" in send("surface.send_text", {"surface_id": item["id"], "text": "probe\n"})
                        wait_file(state, expected_states[pid])
                    checks.append("seven shell PIDs and Unicode variables survive same/cross-window merges and source-window closure")
                checks.append("cross-window merge preserves layouts, tab positions/selections and hidden-tab unread messages, and removes the emptied source window")
                if live_shell:
                    closing_window = send("window.create", {"activate": False})["result"]
                    sole_tab = next(item for item in send("surface.list")["result"]["surfaces"] if item["window_id"] == closing_window["id"])
                    child_pids.add(sole_tab["process_id"])
                    wait_file(Path(directory) / f'{sole_tab["process_id"]}.ready', f'alive-{sole_tab["process_id"]}')
                    assert "result" in send("surface.send_text", {"surface_id": sole_tab["id"], "text": "\x04"})
                    deadline = time.monotonic() + 5
                    while any(item["id"] == closing_window["id"] for item in send("window.list")["result"]["windows"]):
                        assert time.monotonic() < deadline, "last shell exit left an empty window"
                        time.sleep(0.02)
                    checks.append("Ctrl+D in the sole tab removes its empty pane, workspace and window")
                    before_exit = send("surface.list")["result"]["surfaces"]
                    failed = before_exit[0]
                    # Ensure even the newest fixture is past the two-second startup guard.
                    time.sleep(2.1)
                    assert "result" in send("surface.send_text", {"surface_id": failed["id"], "text": "fail\n"})
                    deadline = time.monotonic() + 5
                    while True:
                        current = send("surface.list")["result"]["surfaces"]
                        if all(item["id"] != failed["id"] for item in current):
                            break
                        assert time.monotonic() < deadline, "established shell with nonzero status did not close"
                        time.sleep(0.02)
                    # Canonical PTY input turns Ctrl+D on an empty line into EOF.
                    exited = before_exit[1]
                    assert "result" in send("surface.send_text", {"surface_id": exited["id"], "text": "\x04"})
                    deadline = time.monotonic() + 5
                    while True:
                        current = send("surface.list")["result"]["surfaces"]
                        if all(item["id"] != exited["id"] for item in current):
                            break
                        assert time.monotonic() < deadline, "Ctrl+D left a finished tab open"
                        time.sleep(0.02)
                    assert {item["id"] for item in current} == {item["id"] for item in before_exit} - {exited["id"], failed["id"]}
                    checks.append("established failed shells and Ctrl+D close their tabs; unrelated tabs survive")
                process.stdin.close()
                time.sleep(0.1)
                assert process.poll() is None, "EOF unexpectedly closed the GUI"
                checks.append("stdin EOF leaves GUI running")
            except Exception:
                stderr.seek(0)
                print(stderr.read(4096).decode(errors="replace"), file=sys.stderr)
                raise
            finally:
                selector.close()
                if process.poll() is None:
                    process.terminate()
                    try:
                        process.wait(timeout=3)
                    except subprocess.TimeoutExpired:
                        process.kill()
                        process.wait(timeout=3)
                process.stdout.close()
                if not process.stdin.closed:
                    process.stdin.close()
                deadline = time.monotonic() + 3
                while child_pids and time.monotonic() < deadline:
                    for pid in list(child_pids):
                        try:
                            os.kill(pid, 0)
                        except ProcessLookupError:
                            child_pids.remove(pid)
                    if child_pids:
                        time.sleep(0.01)
                assert not child_pids, f"fixture children did not exit after test app termination: {child_pids}"
            checks.append("all tracked fixture children exited after app termination")
            print(json.dumps({"status": "passed", "checks": checks,
                              "limitations": "no visual, OS activation or latency acceptance"}))


if __name__ == "__main__":
    if os.name != "posix" or not Path("/usr/bin/false").exists():
        raise SystemExit("This smoke harness requires Unix and /usr/bin/false")
    parser = argparse.ArgumentParser()
    parser.add_argument("binary", nargs="?", default="target/debug/rmux")
    parser.add_argument("--live-shell", action="store_true")
    args = parser.parse_args()
    run(Path(args.binary).resolve(), args.live_shell)
