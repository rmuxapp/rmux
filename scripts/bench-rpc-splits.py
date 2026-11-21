#!/usr/bin/env python3
"""Bounded native macOS split/RSS baseline; no rendering-latency claim."""
import json
import argparse
import hashlib
import os
from pathlib import Path
import platform
import statistics
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


def sample(panes, iteration, binary, cli, settle_seconds, measure_close=False, idle_seconds=0, process_tree=False, measure_startup=False):
    with tempfile.TemporaryDirectory(prefix="rmux-split-bench-") as directory:
        root = Path(directory)
        shell = root / "shell"
        shell.write_text('#!/bin/sh\ntrap "exit 0" HUP TERM\nwhile IFS= read -r line; do :; done\n')
        if measure_startup:
            shell.write_text(shell.read_text().replace('while IFS=', "printf 'rmux-benchmark-shell-ready\\n'\nwhile IFS="))
        shell.chmod(0o700)
        config = root / "config.json"
        config.write_text(json.dumps({"confirm_close": "never", "session_file": str(root / "session.json")}))
        endpoint = root / "rpc/rpc.sock"
        env = {key: value for key, value in os.environ.items() if not key.startswith("RMUX_")}
        env.update(RMUX_CONFIG=str(config), RMUX_SHELL=str(shell), RMUX_SOCKET_DIR=str(endpoint.parent))

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
                time.sleep(settle_seconds)
                rss = int(subprocess.run(["/bin/ps", "-o", "rss=", "-p", str(app.pid)],
                                         check=True, capture_output=True, text=True).stdout.strip())
                result = {"panes": panes, "iteration": iteration, "split_ms": timings,
                          "split_total_ms": sum(timings), "app_rss_kib": rss}
                if startup is not None: result["startup"] = startup
                before_tree = bench_processes.tree(app.pid) if process_tree else None
                if before_tree is not None:
                    result["process_tree"] = before_tree
                if idle_seconds:
                    before_cpu = cpu_seconds(app.pid)
                    before_wall = time.monotonic()
                    time.sleep(idle_seconds)
                    after_cpu = cpu_seconds(app.pid)
                    elapsed = time.monotonic() - before_wall
                    result["idle_wall_seconds"] = elapsed
                    result["idle_app_cpu_seconds"] = max(0, after_cpu - before_cpu)
                    result["idle_app_cpu_percent_one_core"] = 100 * result["idle_app_cpu_seconds"] / elapsed
                    if process_tree:
                        after_tree = bench_processes.tree(app.pid)
                        result["idle_process_tree_end"] = after_tree
                        result["idle_retained_process_cpu"] = bench_processes.retained_cpu(before_tree, after_tree)
                if measure_close:
                    # Keep one fresh terminal alive so closing the measured workspace
                    # does not also close the last window/application.
                    survivor = call("workspace.create", window_id=surface["window_id"],
                                    name="close-benchmark-survivor", focus=True)
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
                        result["after_close_original_children_alive"] = bench_processes.survivors(before_tree)
                call("system.quit")
                app.wait(timeout=10)
                assert app.returncode == 0
                if process_tree:
                    result["after_quit_original_children_alive"] = bench_processes.survivors(before_tree)
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
    args = parser.parse_args()
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
        samples = [sample(panes, iteration, binary, cli, args.settle_seconds, args.measure_close, args.idle_seconds, args.process_tree, args.measure_startup) for iteration in range(args.iterations)]
        timings = [value for row in samples for value in row["split_ms"]]
        print(json.dumps({"harness_revision": revision, "binary_label": args.label,
                          "binary_sha256": binary_sha256, "settle_seconds": args.settle_seconds,
                          "platform": platform.platform(),
                          "panes": panes, "samples": samples,
                          "split_p50_ms": statistics.median(timings) if timings else None,
                          "split_max_ms": max(timings) if timings else None,
                          "app_rss_median_kib": statistics.median(row["app_rss_kib"] for row in samples)}), flush=True)
