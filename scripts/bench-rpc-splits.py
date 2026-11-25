#!/usr/bin/env python3
"""Bounded native macOS split/RSS baseline; no rendering-latency claim."""
import json
import argparse
import hashlib
import os
from pathlib import Path
import platform
import statistics
import sys
import subprocess
import tempfile
import time
import bench_processes


def cpu_seconds(pid):
    value = subprocess.run(["/bin/ps", "-o", "time=", "-p", str(pid)],
                           check=True, capture_output=True, text=True).stdout.strip()
    days, clock = value.split("-", 1) if "-" in value else ("0", value)
    seconds = 0.0
    for part in clock.split(":"):
        seconds = seconds * 60 + float(part)
    return int(days) * 86400 + seconds


def sample(panes, iteration, binary, cli, settle_seconds, measure_close=False, idle_seconds=0, process_tree=False, measure_startup=False, workload="idle", visibility="visible", disable_blink=False, render_metrics=False, control_directory=None):
    with tempfile.TemporaryDirectory(prefix="rmux-split-bench-") as directory:
        root = Path(directory)
        idle_directory = root / "idle-survivor"
        idle_directory.mkdir()
        shell = root / "shell"
        shell.write_text('#!/bin/sh\ntrap "exit 0" HUP TERM\nwhile IFS= read -r line; do :; done\n')
        if measure_startup:
            shell.write_text(shell.read_text().replace('while IFS=', "printf 'rmux-benchmark-shell-ready\\n'\nwhile IFS="))
        if workload == "busy":
            # A bounded producer: 32 lines at 20 Hz per terminal. Each child runs
            # the same workload; CPU accounting includes these producers.
            shell.write_text(f"#!{sys.executable}\nimport os,sys,time\n"
                             f"if os.path.realpath(os.getcwd()) == {str(idle_directory.resolve())!r}:\n"
                             " print('rmux-benchmark-idle-ready', flush=True); sys.stdin.buffer.read(); sys.exit(0)\n"
                             "chunk = ('benchmark output 0123456789 世界 ' * 2 + '\\r\\n') * 32\n"
                             "while True:\n sys.stdout.write(chunk); sys.stdout.flush(); time.sleep(0.05)\n")
        shell.chmod(0o700)
        config = root / "config.json"
        preferences = {"confirm_close": "never", "session_file": str(root / "session.json")}
        if disable_blink:
            preferences["cursor_blink"] = False
        config.write_text(json.dumps(preferences))
        endpoint = root / "rpc/rpc.sock"
        env = {key: value for key, value in os.environ.items() if not key.startswith("RMUX_")}
        env.update(RMUX_CONFIG=str(config), RMUX_SHELL=str(shell), RMUX_SOCKET_DIR=str(endpoint.parent))

        if render_metrics:
            env["RMUX_RENDER_METRICS"] = str(root / "render-metrics.jsonl")

        def call(method, **params):
            reply = subprocess.run([str(cli), "--socket", str(endpoint), method, json.dumps(params)],
                                   check=True, capture_output=True, text=True, timeout=10)
            value = json.loads(reply.stdout)
            assert "result" in value, value
            return value["result"]

        with tempfile.TemporaryFile() as log:
            launched_at = time.perf_counter_ns()
            app = subprocess.Popen([str(binary)], env=env, stdin=subprocess.DEVNULL,
                                   stdout=subprocess.DEVNULL, stderr=log)
            try:
                deadline = time.monotonic() + 10
                while not endpoint.exists():
                    assert app.poll() is None, "native app exited"
                    if time.monotonic() > deadline:
                        raise TimeoutError("native app socket did not start")
                    time.sleep(.02)
                socket_ready_ms = (time.perf_counter_ns() - launched_at) / 1_000_000
                while True:
                    initial = call("surface.list")["surfaces"]
                    if initial: break
                    if time.monotonic() > deadline: raise TimeoutError("first terminal did not register")
                    time.sleep(.02)
                surface = initial[0]
                first_surface_ms = (time.perf_counter_ns() - launched_at) / 1_000_000
                startup = None
                if measure_startup:
                    while True:
                        output = call("surface.read_text", surface_id=surface["id"], max_bytes=4096)
                        if "rmux-benchmark-shell-ready" in output["text"]: break
                        if time.monotonic() > deadline: raise TimeoutError("fixture shell output did not become readable")
                        time.sleep(.02)
                    startup = {"socket_ready_ms": socket_ready_ms, "first_surface_ms": first_surface_ms,
                               "first_shell_output_ms": (time.perf_counter_ns() - launched_at) / 1_000_000}
                work = surface["workspace_id"]
                fixture_name = f"rmux benchmark {app.pid} · {workload} {visibility} · {panes} terminals"
                if control_directory is not None:
                    call("workspace.rename", workspace_id=work, name=fixture_name)
                leaves = [surface["pane_id"]]
                timings = []
                # Breadth-first balanced splits avoid measuring a deeply nested chain.
                for index in range(panes - 1):
                    target = leaves.pop(0)
                    start = time.perf_counter_ns()
                    created = call("pane.split", pane_id=target,
                                   direction="right" if index % 2 == 0 else "down")
                    timings.append((time.perf_counter_ns() - start) / 1_000_000)
                    leaves.extend([target, created["pane_id"]])
                surfaces = call("surface.list", workspace_id=work)["surfaces"]
                assert len(surfaces) == panes
                assert all(not row["process_exited"] for row in surfaces)
                assert len({row["process_id"] for row in surfaces}) == panes
                original_tree = bench_processes.tree(app.pid) if process_tree else None
                if visibility == "hidden":
                    # The measured terminals keep running in an inactive workspace.
                    # The one visible idle survivor is counted in total app/tree cost.
                    hidden_survivor = call("workspace.create", window_id=surface["window_id"],
                                          name=f"Idle view · {fixture_name}" if control_directory is not None else "hidden-benchmark-survivor", focus=True,
                                          cwd=str(idle_directory))
                    if workload == "busy":
                        # macOS temporary paths can be aliases (/var vs /private/var).
                        # Prove the foreground survivor reached its idle branch,
                        # rather than silently measuring an extra busy terminal.
                        deadline = time.monotonic() + 10
                        while True:
                            idle_surface = call("surface.list", workspace_id=hidden_survivor["id"])["surfaces"][0]
                            idle_text = call("surface.read_text", surface_id=idle_surface["id"], scope="history", lines=32, max_bytes=65536)["text"]
                            assert "benchmark output" not in idle_text, "foreground survivor is unexpectedly busy"
                            if "rmux-benchmark-idle-ready" in idle_text: break
                            if time.monotonic() > deadline: raise TimeoutError("foreground survivor did not become idle")
                            time.sleep(.02)
                # Verify scenario routing and real output before the quiet CPU
                # interval. These RPC reads are deliberately outside the sample.
                selected = next(row for row in call("workspace.list")["workspaces"] if row["selected"])
                assert (selected["id"] == work) == (visibility != "hidden")
                if workload == "busy":
                    for terminal in surfaces:
                        output = call("surface.read_text", surface_id=terminal["id"], scope="history", lines=32, max_bytes=65536)
                        assert "benchmark output" in output["text"], output
                ui_verification = None
                if control_directory is not None:
                    control_directory.mkdir(parents=True, exist_ok=True)
                    token = f"{workload}-{visibility}-{panes}-{iteration}"
                    ready = control_directory / f"{token}.ready.json"
                    proceed = control_directory / f"{token}.continue"
                    if ready.exists() or proceed.exists():
                        raise RuntimeError(f"Use a fresh control directory; gate already exists: {token}")
                    ready.write_text(json.dumps({"pid": app.pid, "binary": str(binary),
                                                 "window_id": surface["window_id"], "visibility": visibility,
                                                 "fixture_name": fixture_name}))
                    deadline = time.monotonic() + 180
                    while not proceed.exists():
                        if app.poll() is not None: raise RuntimeError("app exited while waiting for UI verification")
                        if time.monotonic() > deadline: raise TimeoutError("UI verification gate timed out")
                        time.sleep(.05)
                    ui_verification = json.loads(proceed.read_text())
                    assert ui_verification["pid"] == app.pid
                    assert ui_verification["visibility"] == visibility
                    assert ui_verification["fixture_name"] == fixture_name, "acknowledge the name observed in the native window"
                    if app.poll() is not None: raise RuntimeError("verified fixture exited before sampling")
                time.sleep(settle_seconds)
                rss = int(subprocess.run(["/bin/ps", "-o", "rss=", "-p", str(app.pid)],
                                         check=True, capture_output=True, text=True).stdout.strip())
                result = {"panes": panes, "iteration": iteration, "split_ms": timings,
                          "split_total_ms": sum(timings), "app_rss_kib": rss,
                          "workload": workload, "visibility": visibility, "ui_verification": ui_verification, "cursor_blink_disabled": disable_blink,
                          "measured_shell_pids": [item["process_id"] for item in surfaces],
                          "target_output_lines_per_second_per_terminal": 640 if workload == "busy" else 0}
                if startup is not None: result["startup"] = startup
                before_tree = bench_processes.tree(app.pid) if process_tree else None
                if before_tree is not None:
                    result["process_tree"] = before_tree
                if render_metrics:
                    result["render_before"] = call("surface.list", workspace_id=work)["surfaces"]
                    assert all("render_metrics" in row for row in result["render_before"]), "binary lacks RPC render counters"
                if idle_seconds:
                    before_cpu = cpu_seconds(app.pid)
                    before_wall = time.monotonic()
                    time.sleep(idle_seconds)
                    after_cpu = cpu_seconds(app.pid)
                    elapsed = time.monotonic() - before_wall
                    result["sample_wall_seconds"] = elapsed
                    result["sample_app_cpu_seconds"] = max(0, after_cpu - before_cpu)
                    result["sample_app_cpu_percent_one_core"] = 100 * result["sample_app_cpu_seconds"] / elapsed
                    if process_tree:
                        after_tree = bench_processes.tree(app.pid)
                        result["sample_process_tree_end"] = after_tree
                        result["sample_retained_process_cpu"] = bench_processes.retained_cpu(before_tree, after_tree)
                if render_metrics:
                    result["render_after"] = call("surface.list", workspace_id=work)["surfaces"]
                if measure_close:
                    # Keep one fresh terminal alive so closing the measured workspace
                    # does not also close the last window/application.
                    survivor = hidden_survivor if visibility == "hidden" else call("workspace.create", window_id=surface["window_id"],
                                    name="close-benchmark-survivor", focus=True, cwd=str(idle_directory))
                    call("workspace.close", workspace_id=work)
                    deadline = time.monotonic() + 10
                    while True:
                        remaining = call("surface.list")["surfaces"]
                        if len(remaining) == 1 and remaining[0]["workspace_id"] == survivor["id"]:
                            break
                        if time.monotonic() > deadline:
                            raise TimeoutError("closed workspace surfaces remained registered")
                        time.sleep(.02)
                    time.sleep(settle_seconds)
                    result["after_close_app_rss_kib"] = int(subprocess.run(
                        ["/bin/ps", "-o", "rss=", "-p", str(app.pid)],
                        check=True, capture_output=True, text=True).stdout.strip())
                    result["after_close_surfaces"] = len(remaining)
                    if process_tree:
                        result["after_close_process_tree"] = bench_processes.tree(app.pid)
                        result["after_close_original_children_alive"] = bench_processes.survivors(original_tree)
                call("system.quit")
                app.wait(timeout=10)
                assert app.returncode == 0
                if process_tree:
                    result["after_quit_original_children_alive"] = bench_processes.survivors(original_tree)
                return result
            finally:
                if app.poll() is None:
                    app.terminate()
                    try:
                        app.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        app.kill()
                        app.wait(timeout=5)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=Path("target/debug/rmux"))
    parser.add_argument("--cli", type=Path, default=Path("target/debug/rmuxctl"))
    parser.add_argument("--label", default="local-debug")
    parser.add_argument("--settle-seconds", type=float, default=0)
    parser.add_argument("--measure-startup", action="store_true", help="Measure socket, terminal registration and parsed fixture output; not presentation latency")
    parser.add_argument("--measure-close", action="store_true")
    parser.add_argument("--process-tree", action="store_true", help="Include child process RSS and identity-aware CPU/teardown samples")
    parser.add_argument("--idle-seconds", type=float, default=0)
    parser.add_argument("--panes", type=int, nargs="+", default=[1, 8, 32])
    parser.add_argument("--iterations", type=int, default=3)
    parser.add_argument("--workload", choices=["idle", "busy"], default="idle")
    parser.add_argument("--visibility", choices=["visible", "hidden", "minimized"], default="visible")
    parser.add_argument("--disable-cursor-blink", action="store_true")
    parser.add_argument("--render-metrics", action="store_true", help="Requires a binary with opt-in RPC render counters")
    parser.add_argument("--control-directory", type=Path, help="Pause each scenario for native UI verification; create the matching .continue file after observing the requested state")
    args = parser.parse_args()
    if args.visibility == "minimized" and args.control_directory is None:
        parser.error("minimized measurements require a control directory and native UI verification")
    if args.measure_startup and args.workload != "idle":
        parser.error("startup marker measurement requires the idle workload")
    if not 1 <= args.iterations <= 20:
        parser.error("iterations must be between 1 and 20")
    if any(not 1 <= panes <= 32 for panes in args.panes):
        parser.error("pane counts must be between 1 and 32")
    if not 0 <= args.settle_seconds <= 60:
        parser.error("settle-seconds must be between 0 and 60")
    if not 0 <= args.idle_seconds <= 60:
        parser.error("idle-seconds must be between 0 and 60")
    binary, cli = args.binary.resolve(strict=True), args.cli.resolve(strict=True)
    with binary.open("rb") as stream:
        binary_sha256 = hashlib.file_digest(stream, "sha256").hexdigest()
    assert platform.system() == "Darwin", "This baseline currently targets macOS"
    revision = subprocess.run(["git", "rev-parse", "--short", "HEAD"], check=True,
                              capture_output=True, text=True).stdout.strip()
    for panes in args.panes:
        samples = [sample(panes, iteration, binary, cli, args.settle_seconds, args.measure_close, args.idle_seconds, args.process_tree, args.measure_startup, args.workload, args.visibility, args.disable_cursor_blink, args.render_metrics, args.control_directory) for iteration in range(args.iterations)]
        timings = [value for row in samples for value in row["split_ms"]]
        print(json.dumps({"harness_revision": revision, "harness_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(), "binary_label": args.label,
                          "binary_sha256": binary_sha256, "settle_seconds": args.settle_seconds,
                          "platform": platform.platform(),
                          "panes": panes, "samples": samples,
                          "split_p50_ms": statistics.median(timings) if timings else None,
                          "split_max_ms": max(timings) if timings else None,
                          "app_rss_median_kib": statistics.median(row["app_rss_kib"] for row in samples)}), flush=True)
