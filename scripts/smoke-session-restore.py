#!/usr/bin/env python3
"""Bounded Unix native multi-window restore check; no user configuration touched."""
import json
import os
from pathlib import Path
import selectors
import subprocess
import tempfile
import time
from urllib.parse import quote


def main(automatic=False, preset=False):
    binary = Path("target/debug/rmux").resolve()
    with tempfile.TemporaryDirectory(prefix="rmux-session-smoke-") as directory:
        root = Path(directory)
        (root / "config.json").write_text("{}")
        shell = root / "shell"
        shell.write_text('#!/bin/sh\ntrap "exit 0" HUP TERM\nprintf "%s" "$$" > "$RMUX_TEST_DIR/$$.pid"\nwhile IFS= read -r line; do\n  if [ "$line" = report ]; then printf "\\033]7;file://localhost%s\\007" "$RMUX_TEST_URL"; fi\ndone\n')
        shell.write_text(shell.read_text().replace("while IFS=", 'pwd > "$RMUX_TEST_DIR/$$.cwd"\nwhile IFS='))
        shell.chmod(0o700)
        def pane(*labels):
            return {"kind": "pane", "active_tab": len(labels) - 1,
                    "tabs": [{"label": label, "directory": directory} for label in labels]}
        def workspace(name, layout, active=0):
            return {"name": name, "description": "restored metadata", "icon": "⚙",
                    "color": 0x7aa2f7, "pinned": True, "active_pane": active, "layout": layout}
        def window(workspace):
            return {"version": 1, "sidebar": True, "sidebar_width": 240,
                    "active_workspace": 0, "workspaces": [workspace]}
        data = {"version": 1, "active_window": 1, "windows": [
            window(workspace("First 世界", pane("one", "two"))),
            window(workspace("Second", {"kind": "split", "axis": "Rows", "weights": [1, 3],
                                       "children": [pane("three"), pane("four") ]}, 1)),
        ]}
        session = root / "session.json"
        expected_group = {"id": 17, "name": "Build 世界", "members": [0],
                          "collapsed": True, "pinned": True, "generated_anchor": False,
                          "color": 0xabcdef, "icon": "📂", "external_id": "fixture-build"}
        data["windows"][0]["groups"] = [expected_group]
        data["windows"][1]["workspaces"][0]["zoomed_pane"] = 1
        session.write_text(json.dumps(data))
        if automatic:
            (root / "config.json").write_text(json.dumps({"session_file": str(session), "confirm_close": "never"}))
        if preset:
            (root / "config.json").write_text(json.dumps({"layout_presets":[
                {"name":"Fixture 世界", "path":str(session), "session":True, "cwd":directory}
            ]}))
            data["windows"][0]["workspaces"][0]["layout"]["tabs"][0]["directory"] = None
            session.write_text(json.dumps(data))
        env = {key: value for key, value in os.environ.items() if not key.startswith("RMUX_")}
        env.update(RMUX_CONFIG=str(root / "config.json"), RMUX_SHELL=str(shell), RMUX_TEST_DIR=directory)
        reported = root / "reported 世界"
        reported.mkdir()
        (reported / ".git").mkdir()
        (reported / ".git" / "HEAD").write_text("ref: refs/heads/initial\n")
        env["RMUX_TEST_URL"] = quote(str(reported))
        checked = subprocess.run([str(binary), "--check-session", str(session)], env=env,
                                 capture_output=True, timeout=5)
        assert checked.returncode == 0, checked.stderr.decode()
        assert not list(root.glob("*.pid")), "structural check launched shells"
        saved_tab = data["windows"][1]["workspaces"][0]["layout"]["children"][1]["tabs"][0]
        saved_tab["directory"] = str(root / "missing-directory")
        session.write_text(json.dumps(data))
        rejected = subprocess.run([str(binary), "--session", str(session)], env=env,
                                  capture_output=True, timeout=5)
        assert rejected.returncode == 2 and b"working directory" in rejected.stderr, rejected.stderr.decode()
        assert not list(root.glob("*.pid")), "later-window preflight failure started earlier shells"
        saved_tab["directory"] = directory
        session.write_text(json.dumps(data))
        with tempfile.TemporaryFile() as stderr:
            launch = [str(binary), "--rpc-stdio-live"] if automatic else [str(binary), "--session", str(session), "--rpc-stdio-live"]
            if preset:
                launch = [str(binary), "--preset", "Fixture 世界", "--rpc-stdio-live"]
            process = subprocess.Popen(launch,
                                       env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                       stderr=stderr, bufsize=0)
            selector = selectors.DefaultSelector()
            selector.register(process.stdout, selectors.EVENT_READ)
            pending = bytearray()
            tracked_pids = set()
            def call(method, params=None, expected_error=None):
                request = {"jsonrpc": "2.0", "id": method, "method": method}
                if params is not None: request["params"] = params
                process.stdin.write((json.dumps(request) + "\n").encode())
                deadline = time.monotonic() + 5
                while b"\n" not in pending:
                    remaining = deadline - time.monotonic()
                    assert remaining > 0 and selector.select(remaining), f"timeout: {method}"
                    chunk = os.read(process.stdout.fileno(), 65536)
                    assert chunk, f"closed stream: {method}"
                    pending.extend(chunk)
                line, _, tail = pending.partition(b"\n")
                pending[:] = tail
                response = json.loads(line)
                assert response["id"] == method, response
                if expected_error is not None:
                    assert response["error"]["code"] == expected_error, response
                    return response["error"]
                return response["result"]
            try:
                windows = call("window.list")["windows"]
                workspaces = call("workspace.list")["workspaces"]
                surfaces = call("surface.list")["surfaces"]
                assert len(windows) == len(workspaces) == 2 and len(surfaces) == 4
                named = {item["name"]: item for item in workspaces}
                assert set(named) == {"First 世界", "Second"}
                def check_group_discovery(windows, workspaces, first_name):
                    first = next(item for item in workspaces if item["name"] == first_name)
                    second = next(item for item in workspaces if item["name"] == "Second")
                    assert first["group_id"] == "workspace_group:17" and first["group_anchor"]
                    assert second["group_id"] is None and not second["group_anchor"]
                    groups = [group for win in windows for group in win["groups"]]
                    assert groups == [{"id": "workspace_group:17", "window_id": first["window_id"],
                                       "name": "Build 世界", "color": "#abcdef", "icon": "📂", "external_id": "fixture-build",
                                       "pinned": True, "collapsed": True, "generated_anchor": False, "cwd": None,
                                       "anchor_workspace_id": first["id"], "workspace_ids": [first["id"]]}], groups
                check_group_discovery(windows, workspaces, "First 世界")
                repeated = call("workspace.group.create", {"window_id": named["First 世界"]["window_id"], "name": "Ignored retry", "external_id": "fixture-build"})
                assert repeated["id"] == "workspace_group:17" and repeated["name"] == "Build 世界"
                assert all(item["pinned"] and item["icon"] == "⚙" and item["color"] == "#7aa2f7" for item in workspaces)
                assert all(item["description"] == "restored metadata" for item in workspaces)
                assert named["Second"]["layout"]["axis"] == "rows"
                assert named["Second"]["layout"]["weights"] == [1, 3]
                assert named["Second"]["zoomed_pane_id"] == named["Second"]["layout"]["children"][1]["pane_id"]
                assert sorted(item["title"] for item in surfaces) == ["four", "one", "three", "two"]
                first_tabs = [item for item in surfaces if item["workspace_id"] == named["First 世界"]["id"]]
                assert next(item for item in first_tabs if item["selected"])["title"] == "two"
                pids = {item["process_id"] for item in surfaces}
                assert len(pids) == 4 and None not in pids
                tracked_pids.update(pids)
                for pid in pids: os.kill(pid, 0)
                if preset:
                    for pid in pids:
                        cwd = root / f"{pid}.cwd"
                        deadline = time.monotonic() + 3
                        while not cwd.exists():
                            assert time.monotonic() < deadline, "preset shell cwd not reported"
                            time.sleep(0.01)
                        assert Path(cwd.read_text().strip()).resolve() == root.resolve()
                    assert json.loads(session.read_text()) == data, "preset import modified its source file"

                assert not any(item["process_exited"] for item in surfaces)
                group_target = {"window_id": named["First 世界"]["window_id"], "group_id": "workspace_group:17"}
                call("workspace.group.ungroup", {**group_target, "remove_generated_anchor": True}, expected_error=-32005)
                changed = call("workspace.group.update", {**group_target, "name": "Updated 世界", "collapsed": False, "pinned": False})
                assert changed["name"] == "Updated 世界" and not changed["collapsed"] and not changed["pinned"]
                call("workspace.group.update", {**group_target, "name": "Build 世界", "collapsed": True, "pinned": True})
                check_group_discovery(call("window.list")["windows"], call("workspace.list")["workspaces"], "First 世界")
                if not automatic:
                    second_id = named["Second"]["id"]
                    call("workspace.move_to_window", {"workspace_id": second_id, "window_id": group_target["window_id"]})
                    call("workspace.select", {"workspace_id": named["First 世界"]["id"]})
                    before_failure = call("window.list")["windows"]
                    before_failure_workspaces = call("workspace.list")["workspaces"]
                    call("workspace.group.create", {"window_id": group_target["window_id"], "name": "Bad cwd", "cwd": str(root / "does-not-exist"), "external_id": "failed-create"}, expected_error=-32005)
                    assert call("window.list")["windows"] == before_failure, "failed creation changed group state or selection"
                    assert call("workspace.list")["workspaces"] == before_failure_workspaces, "failed creation changed workspaces"
                    membership = {**group_target, "workspace_id": second_id}
                    added = call("workspace.group.add", membership)["group"]
                    call("workspace.group.update", {**group_target, "collapsed": True})
                    hidden_terminal = next(item for item in surfaces if item["workspace_id"] == second_id and item["title"] == "three")
                    call("surface.focus", {"surface_id": hidden_terminal["id"]})
                    revealed = next(group for win in call("window.list")["windows"] for group in win["groups"] if group["id"] == group_target["group_id"])
                    assert not revealed["collapsed"], "terminal focus left its workspace hidden"
                    selected_workspace = next(item for item in call("workspace.list")["workspaces"] if item["id"] == second_id)
                    assert selected_workspace["selected"]
                    assert selected_workspace["zoomed_pane_id"] == hidden_terminal["pane_id"], "terminal focus did not follow zoom"

                    call("workspace.group.update", {**group_target, "collapsed": True})
                    moved_tab = first_tabs[0]
                    call("surface.move_to_workspace", {"surface_id": moved_tab["id"], "workspace_id": second_id})
                    revealed = next(group for win in call("window.list")["windows"] for group in win["groups"] if group["id"] == group_target["group_id"])
                    assert not revealed["collapsed"], "tab transfer left destination hidden"
                    moved = next(item for item in call("surface.list")["surfaces"] if item["id"] == moved_tab["id"])
                    assert moved["workspace_id"] == second_id and moved["process_id"] == moved_tab["process_id"]
                    call("surface.move_to_workspace", {"surface_id": moved_tab["id"], "workspace_id": named["First 世界"]["id"]})

                    assert added["workspace_ids"] == [named["First 世界"]["id"], second_id]
                    anchored = call("workspace.group.set_anchor", membership)["group"]
                    assert anchored["anchor_workspace_id"] == second_id
                    assert anchored["workspace_ids"] == [second_id, named["First 世界"]["id"]]
                    removed = call("workspace.group.remove", membership)["group"]
                    assert removed["anchor_workspace_id"] == named["First 世界"]["id"]
                    assert removed["workspace_ids"] == [named["First 世界"]["id"]]
                    ungrouped = call("workspace.group.ungroup", group_target)
                    assert ungrouped["ungrouped"] and ungrouped["workspace_ids"] == [named["First 世界"]["id"]]
                    assert all(not win["groups"] for win in call("window.list")["windows"])
                    assert all(item["group_id"] is None for item in call("workspace.list")["workspaces"])
                    create_params = {"window_id": group_target["window_id"], "name": "Regrouped", "workspace_ids": [second_id, named["First 世界"]["id"]], "external_id": "regroup"}
                    created = call("workspace.group.create", create_params)
                    assert created["workspace_ids"] == create_params["workspace_ids"] and not created["generated_anchor"]
                    assert call("workspace.group.create", create_params)["id"] == created["id"]
                    active_before = next(win for win in call("window.list")["windows"] if win["id"] == group_target["window_id"])["selected_workspace_id"]
                    call("workspace.group.ungroup", {"window_id": group_target["window_id"], "group_id": created["id"]})
                    flattened = call("workspace.list")["workspaces"]
                    assert [item["id"] for item in flattened] == create_params["workspace_ids"], "ungroup lost member order"
                    assert next(item["id"] for item in flattened if item["selected"]) == active_before, "ungroup changed selection"
                retained = call("surface.list")["surfaces"]
                assert {(item["id"], item["process_id"]) for item in retained} == {(item["id"], item["process_id"]) for item in surfaces}, "group mutation restarted terminals"
                if not automatic:
                    group_directory = root / "project 世界"
                    group_directory.mkdir()
                    fresh_params = {"window_id": group_target["window_id"], "name": "Fresh", "external_id": "new-shell", "cwd": str(group_directory)}
                    fresh = call("workspace.group.create", fresh_params)
                    assert fresh["generated_anchor"] and len(fresh["workspace_ids"]) == 1
                    assert call("workspace.group.create", fresh_params)["id"] == fresh["id"]
                    after = call("surface.list")["surfaces"]
                    assert len(after) == 5, "idempotent creation spawned extra shells"
                    tracked_pids.update(item["process_id"] for item in after)
                    expected_members = fresh["workspace_ids"][:]
                    for placement in ("end", "top", "after_current"):
                        added = call("workspace.group.new_workspace", {"window_id": group_target["window_id"], "group_id": fresh["id"], "placement": placement})
                        if placement == "end": expected_members.append(added["workspace_id"])
                        else: expected_members.insert(1, added["workspace_id"])
                        discovered = next(group for win in call("window.list")["windows"] for group in win["groups"] if group["id"] == fresh["id"])
                        assert discovered["workspace_ids"] == expected_members, (placement, discovered)
                    after = call("surface.list")["surfaces"]
                    assert len(after) == 8
                    call("workspace.group.ungroup", {"window_id": group_target["window_id"], "group_id": fresh["id"], "remove_generated_anchor": True}, expected_error=-32005)
                    tracked_pids.update(item["process_id"] for item in after)
                    deadline = time.monotonic() + 3
                    for terminal in after:
                        cwd_file = root / f'{terminal["process_id"]}.cwd'
                        while not cwd_file.exists():
                            assert time.monotonic() < deadline, "fixture shell did not start"
                            time.sleep(0.01)
                        actual_cwd = Path(cwd_file.read_text().strip())
                        expected_cwd = group_directory if terminal["workspace_id"] in expected_members else root
                        assert actual_cwd.resolve() == expected_cwd.resolve(), f"new group shell cwd {actual_cwd} != {expected_cwd}"
                    disposable = call("workspace.group.create", {"window_id": group_target["window_id"], "name": "Cleanup fixture"})
                    disposable_surfaces = [item for item in call("surface.list")["surfaces"] if item["workspace_id"] == disposable["anchor_workspace_id"]]
                    tracked_pids.update(item["process_id"] for item in disposable_surfaces)
                    cleaned = call("workspace.group.ungroup", {"window_id": group_target["window_id"], "group_id": disposable["id"], "remove_generated_anchor": True})
                    assert cleaned["closed_workspace_id"] == disposable["anchor_workspace_id"]
                    assert len(call("surface.list")["surfaces"]) == 8
                if automatic:
                    def saved_tabs(node):
                        if node["kind"] == "pane": return node["tabs"]
                        return [tab for child in node["children"] for tab in saved_tabs(child)]
                    def session_tabs(saved):
                        return [tab for win in saved["windows"] for workspace in win["workspaces"] for tab in saved_tabs(workspace["layout"])]
                    call("workspace.rename", {"workspace_id": named["First 世界"]["id"], "name": "Checkpoint 世界"})
                    deadline = time.monotonic() + 4
                    while True:
                        checkpoint = json.loads(session.read_text())
                        if "Checkpoint 世界" in {item["workspaces"][0]["name"] for item in checkpoint["windows"]}: break
                        assert time.monotonic() < deadline, "change-triggered checkpoint not written"
                        time.sleep(0.02)
                    assert all(tab["directory"] == directory for tab in session_tabs(checkpoint)), "explicit startup directories lost without OSC reports"
                    assert next(win for win in checkpoint["windows"] if win["workspaces"][0]["name"] == "Checkpoint 世界")["groups"] == [expected_group], "group state lost during capture"
                    assert next(win["workspaces"][0] for win in checkpoint["windows"] if win["workspaces"][0]["name"] == "Second")["zoomed_pane"] == 1
                    call("surface.send_text", {"surface_id": surfaces[0]["id"], "text": "report\n"})
                    def wait_branch(expected):
                        deadline = time.monotonic() + 3
                        while True:
                            target = next(item for item in call("surface.list")["surfaces"] if item["id"] == surfaces[0]["id"])
                            if target["git_branch"] == expected: return
                            assert time.monotonic() < deadline, ("Git context did not refresh", expected, target)
                            time.sleep(0.01)
                    wait_branch("initial")
                    (reported / ".git" / "HEAD").write_text("ref: refs/heads/switched-世界\n")
                    call("surface.send_text", {"surface_id": surfaces[0]["id"], "text": "report\n"})
                    wait_branch("switched-世界")
                    deadline = time.monotonic() + 4
                    while True:
                        checkpoint = json.loads(session.read_text())
                        if str(reported) in {tab["directory"] for tab in session_tabs(checkpoint)}: break
                        assert time.monotonic() < deadline, "reported directory not checkpointed"
                        time.sleep(0.02)
                    # Kill only this isolated test app: no normal quit callback
                    # can rewrite the completed change-triggered checkpoint.
                    competitor = subprocess.run(launch, env=env, capture_output=True, timeout=5,
                        input=b'{"jsonrpc":"2.0","id":1,"method":"window.list"}\n{"jsonrpc":"2.0","method":"system.quit"}\n')
                    assert competitor.returncode == 0, competitor.stderr.decode()
                    assert b"recovery disabled" in competitor.stderr, competitor.stderr.decode()
                    competitor_windows = json.loads(competitor.stdout.splitlines()[0])["result"]["windows"]
                    assert len(competitor_windows) == 1, "competing process restored an already-owned session"
                    retained_checkpoint = json.loads(session.read_text())
                    assert {item["workspaces"][0]["name"] for item in retained_checkpoint["windows"]} == {"Checkpoint 世界", "Second"}, "competing process overwrote checkpoint"
                    checkpoint_bytes = session.read_bytes()
                    previous_pids = set(pids)
                    process.kill()
                    process.wait(timeout=3)
                    selector.unregister(process.stdout)
                    process.stdin.close()
                    process.stdout.close()
                    assert session.read_bytes() == checkpoint_bytes
                    process = subprocess.Popen(launch, env=env, stdin=subprocess.PIPE,
                                               stdout=subprocess.PIPE, stderr=stderr, bufsize=0)
                    selector.register(process.stdout, selectors.EVENT_READ)
                    pending.clear()
                    recovered_windows = call("window.list")["windows"]
                    recovered_workspaces = call("workspace.list")["workspaces"]
                    check_group_discovery(call("window.list")["windows"], recovered_workspaces, "Checkpoint 世界")
                    recovered_surfaces = call("surface.list")["surfaces"]
                    assert len(recovered_windows) == 2
                    assert {item["name"] for item in recovered_workspaces} == {"Checkpoint 世界", "Second"}
                    recovered_split = next(item for item in recovered_workspaces if item["name"] == "Second")["layout"]
                    assert recovered_split["axis"] == "rows" and recovered_split["weights"] == [1, 3]
                    zoomed = next(item for item in recovered_workspaces if item["name"] == "Second")["zoomed_pane_id"]
                    assert zoomed == recovered_split["children"][1]["pane_id"]
                    call("pane.zoom.set", {"pane_id": zoomed, "zoomed": False})
                    unzoomed = next(item for item in call("workspace.list")["workspaces"] if item["name"] == "Second")
                    assert unzoomed["zoomed_pane_id"] is None and unzoomed["layout"] == recovered_split
                    assert sorted(item["title"] for item in recovered_surfaces) == ["four", "one", "three", "two"]
                    recovered_pids = {item["process_id"] for item in recovered_surfaces}
                    assert len(recovered_pids) == 4 and None not in recovered_pids
                    tracked_pids.update(recovered_pids)
                    assert recovered_pids.isdisjoint(previous_pids), "recovery should create fresh shells"
                    for pid in recovered_pids: os.kill(pid, 0)
                    # A normal quit exercises capture before teardown and the
                    # awaited checkpoint writer, unlike terminating the process.
                    process.stdin.write(b'{"jsonrpc":"2.0","method":"system.quit"}\n')
                    assert process.wait(timeout=5) == 0
                    saved = json.loads(session.read_text())
                    assert next(win for win in saved["windows"] if win["workspaces"][0]["name"] == "Checkpoint 世界")["groups"] == [expected_group], "group state lost after forced-stop restore"
                    assert len(saved["windows"]) == 2
                    assert {item["workspaces"][0]["name"] for item in saved["windows"]} == {"Checkpoint 世界", "Second"}
                    assert all(item["sidebar_width"] == 240 for item in saved["windows"])
                    assert all(item["geometry"]["width"] > 0 and item["geometry"]["height"] > 0
                               and item["geometry"]["mode"] == "windowed" for item in saved["windows"])
                    assert str(reported) in {tab["directory"] for tab in session_tabs(saved)}
                    assert saved != data, "checkpoint did not include effective captured tab settings"
            except Exception:
                stderr.seek(0)
                print(stderr.read(4096).decode(errors="replace"))
                raise
            finally:
                selector.close()
                if process.poll() is None: process.terminate()
                try: process.wait(timeout=3)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait(timeout=3)
                process.stdin.close()
                process.stdout.close()
                pids = tracked_pids | {int(path.stem) for path in root.glob("*.pid")}
                deadline = time.monotonic() + 3
                while pids and time.monotonic() < deadline:
                    for pid in list(pids):
                        try: os.kill(pid, 0)
                        except ProcessLookupError: pids.remove(pid)
                    if pids: time.sleep(0.01)
                assert not pids, f"fixture shells not reaped: {pids}"
    print(f"Session restore passed (preset={preset}, automatic={automatic}, forced_stop_relaunch={automatic}): headless validation, all-window directory preflight, two windows, metadata, weighted splits, tab selection, fresh shells and cleanup. OS window activation/visual focus unverified.")


def check_fresh_recovery():
    binary = Path("target/debug/rmux").resolve()
    with tempfile.TemporaryDirectory(prefix="rmux-fresh-recovery-") as directory:
        root = Path(directory)
        session = root / "new/nested/session.json"
        config = root / "config.json"
        config.write_text(json.dumps({"session_file": str(session), "confirm_close": "never"}))
        env = {key: value for key, value in os.environ.items() if not key.startswith("RMUX_")}
        env.update(RMUX_CONFIG=str(config), RMUX_SHELL="/usr/bin/false")
        request = b'{"jsonrpc":"2.0","id":1,"method":"workspace.list"}\n{"jsonrpc":"2.0","method":"system.quit"}\n'
        def launch():
            result = subprocess.run([str(binary), "--rpc-stdio-live"], env=env,
                                    capture_output=True, timeout=8, input=request)
            assert result.returncode == 0, result.stderr.decode()
            assert b"recovery disabled" not in result.stderr, result.stderr.decode()
            assert b"could not recover session" not in result.stderr, result.stderr.decode()
            return json.loads(result.stdout.splitlines()[0])["result"]["workspaces"]
        assert not session.parent.exists()
        assert len(launch()) == 1
        saved = json.loads(session.read_text())
        assert len(saved["windows"]) == 1
        assert len(saved["windows"][0]["workspaces"]) == 1
        if os.name == "posix":
            assert session.parent.stat().st_mode & 0o077 == 0
            assert session.parent.parent.stat().st_mode & 0o077 == 0
        saved["windows"][0]["workspaces"][0]["name"] = "Fresh recovery 世界"
        session.write_text(json.dumps(saved))
        restored = launch()
        assert len(restored) == 1 and restored[0]["name"] == "Fresh recovery 世界", restored
        assert json.loads(session.read_text())["windows"][0]["workspaces"][0]["name"] == "Fresh recovery 世界"
    print("Fresh recovery passed: missing nested folders, private Unix permissions, normal-Quit checkpoint and relaunch metadata. OS focus/visuals unverified.")


def check_corrupt_fallback():
    binary = Path("target/debug/rmux").resolve()
    with tempfile.TemporaryDirectory(prefix="rmux-corrupt-session-") as directory:
        root = Path(directory)
        session = root / "session.json"
        config = root / "config.json"
        config.write_text(json.dumps({"session_file": str(session), "confirm_close": "never"}))
        env = {key: value for key, value in os.environ.items() if not key.startswith("RMUX_")}
        env.update(RMUX_CONFIG=str(config), RMUX_SHELL="/usr/bin/false")
        for original in [b'{"version":', b'{"version":99,"windows":[],"active_window":null}']:
            session.write_bytes(original)
            result = subprocess.run([str(binary), "--rpc-stdio-live"], env=env,
                capture_output=True, timeout=5,
                input=b'{"jsonrpc":"2.0","id":1,"method":"window.list"}\n{"jsonrpc":"2.0","method":"system.quit"}\n')
            assert result.returncode == 0, result.stderr.decode()
            assert b"could not recover session" in result.stderr, result.stderr.decode()
            windows = json.loads(result.stdout.splitlines()[0])["result"]["windows"]
            assert len(windows) == 1 and windows[0]["workspace_count"] == 1
            assert session.read_bytes() == original, "fallback quit replaced failed recovery file"
    print("Corrupt/unsupported session fallback passed: fresh window launches and normal Quit preserves original bytes.")


if __name__ == "__main__":
    import sys
    if sys.argv[1:] == ["--preset"]:
        main(preset=True)
    elif sys.argv[1:] == ["--fresh-recovery"]:
        check_fresh_recovery()
    elif not sys.argv[1:]:
        main()
        main(automatic=True)
        check_corrupt_fallback()
        check_fresh_recovery()
    else:
        raise SystemExit("Usage: smoke-session-restore.py [--preset | --fresh-recovery]")
