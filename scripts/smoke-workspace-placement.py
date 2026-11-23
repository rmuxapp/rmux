#!/usr/bin/env python3
"""Native RPC check for insertion order, failed creation and selection continuity."""
import argparse
import json
import os
from pathlib import Path
import socket
import subprocess
import tempfile
import time

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--live-shell", action="store_true", help="Check layout mutations with persistent shell state (end-placement scenario only)")
args = parser.parse_args()
binary = Path("target/debug/rmux").resolve()
for mode in (["end"] if args.live_shell else ["end", "top", "after_current"]):
    with tempfile.TemporaryDirectory(prefix="rmux-placement-") as temporary:
        root = Path(temporary).resolve()
        config = root / "config.json"
        config.write_text(json.dumps({"new_workspace_placement": mode, "confirm_close": "never"}))
        endpoint = root / "rpc"
        env = {k: v for k, v in os.environ.items() if not k.startswith("RMUX_")}
        env.update(RMUX_CONFIG=str(config), RMUX_SHELL="/usr/bin/false", RMUX_SOCKET_DIR=str(endpoint))
        if args.live_shell:
            fixture = root / "fixture-shell"
            fixture.write_text('#!/bin/sh\ntrap "exit 0" HUP TERM\nprintf "alive-%s" "$$" > "$RMUX_TEST_PROCESS_DIR/$$.ready"\ntoken=initial\nwhile IFS= read -r line; do\n  case "$line" in\n    set:*) token=${line#set:} ;;\n    probe) printf "%s" "$token" > "$RMUX_TEST_PROCESS_DIR/$$.state" ;;\n  esac\ndone\n')
            fixture.chmod(0o700)
            env.update(RMUX_SHELL=str(fixture), RMUX_TEST_PROCESS_DIR=str(root))

        def wait_file(path, expected):
            deadline = time.monotonic() + 3
            while time.monotonic() < deadline:
                if path.exists() and path.read_text() == expected:
                    return
                time.sleep(.01)
            raise AssertionError(f"fixture did not report expected state in {path.name}")

        with tempfile.TemporaryFile() as log:
            app = subprocess.Popen([str(binary)], env=env, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=log)
            try:
                deadline = time.monotonic() + 8
                while not (endpoint / "rpc.sock").exists():
                    if app.poll() is not None or time.monotonic() >= deadline:
                        log.seek(0)
                        raise AssertionError("socket startup failed: " + log.read().decode(errors="replace"))
                    time.sleep(.02)
                with socket.socket(socket.AF_UNIX) as connection:
                    connection.settimeout(8)
                    connection.connect(str(endpoint / "rpc.sock"))
                    stream = connection.makefile("rwb", buffering=0)
                    def call(method, fail=False, notify=False, **params):
                        request = dict(jsonrpc="2.0", method=method, params=params)
                        if not notify:
                            request["id"] = 1
                        stream.write((json.dumps(request) + "\n").encode())
                        if notify:
                            return
                        reply = json.loads(stream.readline())
                        assert reply.get("id") == 1, "notification unexpectedly produced a response"
                        if fail:
                            assert "error" in reply, reply
                            return
                        assert "error" not in reply, reply
                        return reply["result"]
                    window = call("window.list")["windows"][0]["id"]
                    def rows():
                        return call("workspace.list", window_id=window)["workspaces"]
                    original = rows()[0]["id"]
                    second = call("workspace.create", window_id=window, name="Second")["id"]
                    before = rows()
                    call("workspace.create", fail=True, window_id=window, cwd=str(root / "missing"))
                    assert [(r["id"], r["selected"]) for r in rows()] == [(r["id"], r["selected"]) for r in before]
                    third = call("workspace.create", window_id=window, name="Third")["id"]
                    expected = {"end": [original, second, third], "top": [third, second, original], "after_current": [original, third, second]}[mode]
                    assert [r["id"] for r in rows()] == expected, rows()
                    assert [r["id"] for r in rows() if r["selected"]] == [original], rows()
                    fourth = call("workspace.create", window_id=window, name="Focused", focus=True)["id"]
                    assert [r["id"] for r in rows() if r["selected"]] == [fourth], rows()
                    call("workspace.close", workspace_id=fourth)
                    assert [r["id"] for r in rows()] == expected, rows()
                    if mode == "end":
                        pane = call("pane.list", workspace_id=original)["panes"][0]["id"]
                        assert call("pane.resize", pane_id=pane, direction="right")["changed"] is False
                        call("pane.split", notify=True, pane_id=pane, direction="right")
                        assert len(call("pane.list", workspace_id=original)["panes"]) == 2, "split notification was ignored"
                        def identity():
                            return [(r["id"], r.get("process_id")) for r in call("surface.list", workspace_id=original)["surfaces"]]
                        surfaces = identity()
                        if args.live_shell:
                            for surface, pid in surfaces:
                                wait_file(root / f"{pid}.ready", f"alive-{pid}")
                                call("surface.send_text", surface_id=surface, text=f"set:retained-{pid}-世界\nprobe\n")
                                wait_file(root / f"{pid}.state", f"retained-{pid}-世界")
                        selected = [r["id"] for r in rows() if r["selected"]]
                        assert call("pane.resize", pane_id=pane, direction="right")["changed"] is True
                        call("pane.resize", notify=True, pane_id=pane, direction="right")
                        layout = next(r["layout"] for r in rows() if r["id"] == original)
                        assert abs(layout["weights"][0] - 1.2) < 1e-5 and abs(layout["weights"][1] - .8) < 1e-5, layout
                        call("pane.resize", notify=True, pane_id=pane, direction="invalid")
                        assert call("system.ping")["pong"] is True
                        assert identity() == surfaces
                        assert [r["id"] for r in rows() if r["selected"]] == selected
                        call("pane.resize", fail=True, pane_id=pane, direction="diagonal")
                        call("pane.resize", fail=True, pane_id="999999999", direction="left")
                        call("pane.zoom.set", pane_id=pane, zoomed=True)
                        call("pane.resize", fail=True, pane_id=pane, direction="left")
                        assert next(r["layout"] for r in rows() if r["id"] == original) == layout
                        call("pane.zoom.set", pane_id=pane, zoomed=False)
                        assert call("pane.resize", pane_id=pane, direction="left")["changed"] is True
                        print("PASS: pane.resize weights, identity, selection, single-pane no-op and rejection paths")
                        selected = [r["id"] for r in rows() if r["selected"]]
                        # Split before an existing pane on both axes, preserving identities.
                        for direction, axis in [("left", "columns"), ("up", "rows")]:
                            created = call("pane.split", pane_id=pane, direction=direction)
                            tree = next(r["layout"] for r in rows() if r["id"] == original)
                            def containing_pair(node):
                                if "children" not in node:
                                    return None
                                if [child.get("pane_id") for child in node["children"]] == [created["pane_id"], pane]:
                                    return node
                                return next((found for child in node["children"] if (found := containing_pair(child))), None)
                            pair = containing_pair(tree)
                            assert pair and pair["axis"] == axis and pair["weights"] == [1.0, 1.0], tree
                            current = identity()
                            assert all(item in current for item in surfaces)
                            assert [r["id"] for r in rows() if r["selected"]] == selected
                        print("PASS: left/up splits preserve nested order, identities and workspace selection")
                        # Swap identities only; retain every split axis and weight.
                        before_swap = next(r["layout"] for r in rows() if r["id"] == original)
                        pane_ids = [r["id"] for r in call("pane.list", workspace_id=original)["panes"]]
                        first, other = pane_ids[0], pane_ids[-1]
                        def exchanged(node):
                            result = dict(node)
                            if "pane_id" in result:
                                result["pane_id"] = {first: other, other: first}.get(result["pane_id"], result["pane_id"])
                            if "children" in result:
                                result["children"] = [exchanged(child) for child in result["children"]]
                            return result
                        assert call("pane.swap", pane_id=first, other_pane_id=other)["changed"] is True
                        assert next(r["layout"] for r in rows() if r["id"] == original) == exchanged(before_swap)
                        assert [r["id"] for r in rows() if r["selected"]] == selected
                        assert call("pane.swap", pane_id=first, other_pane_id=first)["changed"] is False
                        foreign = call("pane.list", workspace_id=second)["panes"][0]["id"]
                        call("pane.swap", fail=True, pane_id=first, other_pane_id=foreign)
                        call("pane.swap", fail=True, pane_id=first, other_pane_id="999999999")
                        call("pane.swap", notify=True, pane_id=first, other_pane_id=other)
                        assert next(r["layout"] for r in rows() if r["id"] == original) == before_swap
                        assert [r["id"] for r in call("pane.list", workspace_id=original)["panes"]] == pane_ids
                        print("PASS: pane swap exchanges only identities, preserves selection and rejects foreign panes")
                        before_equalize = identity()
                        call("pane.zoom.set", pane_id=pane, zoomed=True)
                        call("workspace.equalize", fail=True, workspace_id=original)
                        call("pane.swap", fail=True, pane_id=first, other_pane_id=other)
                        call("pane.zoom.set", pane_id=pane, zoomed=False)
                        selected = [r["id"] for r in rows() if r["selected"]]
                        assert call("workspace.equalize", workspace_id=original)["changed"] is True
                        def balanced(node):
                            return "children" not in node or (all(w == 1 for w in node["weights"]) and all(balanced(c) for c in node["children"]))
                        assert balanced(next(r["layout"] for r in rows() if r["id"] == original))
                        assert call("workspace.equalize", workspace_id=original)["changed"] is False
                        call("workspace.equalize", notify=True, workspace_id=original)
                        assert call("system.ping")["pong"] is True
                        assert identity() == before_equalize
                        assert [r["id"] for r in rows() if r["selected"]] == selected
                        call("workspace.equalize", fail=True, workspace_id="999999999")
                        call("workspace.equalize", fail=True, workspace_id=original, direction="right")
                        print("PASS: workspace equalize balances nested weights, is idempotent and preserves identities/selection")
                        if args.live_shell:
                            current = call("surface.list", workspace_id=original)["surfaces"]
                            assert all(not item["process_exited"] for item in current)
                            for item in current:
                                pid = item["process_id"]
                                wait_file(root / f"{pid}.ready", f"alive-{pid}")
                                os.kill(pid, 0)
                            for surface, pid in surfaces:
                                state = root / f"{pid}.state"
                                state.unlink()
                                call("surface.send_text", surface_id=surface, text="probe\n")
                                wait_file(state, f"retained-{pid}-世界")
                            print("PASS: running shell PIDs and Unicode state survive resize, left/up splits, swap and equalize")
                    if mode == "after_current":
                        group = call("workspace.group.create", window_id=window, name="Placement", workspace_ids=[second, original])
                        call("workspace.select", workspace_id=original)
                        after_group = call("workspace.create", window_id=window, name="After group")["id"]
                        assert [r["id"] for r in rows()] == [third, second, original, after_group], rows()
                        call("workspace.group.update", window_id=window, group_id=group["id"], pinned=True)
                        after_pin = call("workspace.create", window_id=window, name="After pinned group")["id"]
                        assert [r["id"] for r in rows()] == [second, original, after_pin, third, after_group], rows()
                        assert [r["id"] for r in rows() if r["selected"]] == [original], rows()
            finally:
                if app.poll() is None:
                    app.terminate()
                try:
                    app.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    app.kill()
                    app.wait(timeout=5)
                if args.live_shell:
                    # Observe only fixture-owned processes; never signal a reused PID.
                    deadline = time.monotonic() + 5
                    pending = [int(path.stem) for path in root.glob("*.ready")]
                    while pending:
                        alive = []
                        for pid in pending:
                            try:
                                os.kill(pid, 0)
                                alive.append(pid)
                            except ProcessLookupError:
                                pass
                        pending = alive
                        assert not pending or time.monotonic() < deadline, f"fixture children remain: {pending}"
                        if pending:
                            time.sleep(.01)
                    print("PASS: all fixture-owned shells exited after app shutdown")
    print(f"PASS: {mode} insertion, failed creation, background/focused selection and close" + (", group and pinned-group ordering" if mode == "after_current" else ""))
