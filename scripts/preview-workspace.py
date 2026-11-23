#!/usr/bin/env python3
"""Isolated native visual fixture. All project/status content is synthetic."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile

parser = argparse.ArgumentParser()
parser.add_argument("--binary", default="target/debug/rmux")
parser.add_argument("--scene", choices=["workspace", "overflow", "compact"], default="workspace")
parser.add_argument("--check", action="store_true")
parser.add_argument("--colors", action="store_true", help="Show saved workspace and tab color examples")
parser.add_argument("--theme", default="dark", choices=["dark", "light", "solarized_dark", "solarized_light", "nord", "catppuccin_mocha", "catppuccin_latte"])
args = parser.parse_args()
binary = Path(args.binary).resolve()
with tempfile.TemporaryDirectory(prefix="rmux-visual-") as temporary:
    root = Path(temporary)
    config = root / "config.json"
    config.write_text(json.dumps({"font_size": 13, "confirm_close": "never", "terminal_theme": args.theme}))
    shell = root / "preview-shell"
    shell.write_text('''#!/bin/sh
trap 'exit 0' HUP TERM
printf '\\033]7;file://localhost%s\\007' "$PWD"
printf '\\033[1;36mrmux visual fixture\\033[0m\\n\\n'
printf 'Project: %s\\n' "${PWD##*/}"
printf 'Synthetic content for layout comparison.\\n'
printf 'No build, agent or network service is running.\\n\\n'
printf '\\033[32m  ✓ Layout loaded\\n  ✓ Unicode: café 世界\\033[0m\\n\\n'
case "$PWD" in
  *review*) printf '\\033]777;notify;Review ready;Synthetic notification preview for sidebar spacing.\\007' ;;
esac
printf '$ '
while IFS= read -r line; do
  printf 'Preview input: %s\\n$ ' "$line"
done
''')
    shell.chmod(0o700)
    def project(name, branch):
        directory = root / name
        (directory / ".git").mkdir(parents=True)
        (directory / ".git/HEAD").write_text(f"ref: refs/heads/{branch}\n")
        return str(directory)
    app = project("workspace-app", "feature/workspace-layout")
    review = project("review-ready", "review/terminal-navigation")
    docs = project("documentation", "main")
    def pane(directory, labels):
        return {"kind": "pane", "active_tab": 0,
                "tabs": [{"label": label, "directory": directory} for label in labels]}
    def workspace(name, description, layout, pinned=False):
        return {"name": name, "description": description, "pinned": pinned,
                "active_pane": 0, "layout": layout}
    main = {"kind": "split", "axis": "Columns", "weights": [2, 1], "children": [
        pane(app, ["Editor", "Tests", "Long terminal title that should truncate cleanly"]),
        {"kind": "split", "axis": "Rows", "weights": [1, 1], "children": [pane(app, ["Build output"]), pane(app, ["Server logs"])]}]}
    if args.scene == "overflow":
        main = pane(app, [f"Terminal {i:02} — long context title" for i in range(1, 13)])
    layout = {"version": 1, "sidebar": True, "sidebar_width": 280,
              "active_workspace": 0,
              "geometry": {"x": 80, "y": 80, "width": 1280 if args.scene != "compact" else 800,
                           "height": 800 if args.scene != "compact" else 540},
              "workspaces": [workspace("Workspace layout", "Nested panes and terminal tabs", main, True),
                             workspace("Review ready", "Unread message and branch context", pane(review, ["Review"])),
                             workspace("Documentation", "A quieter workspace", pane(docs, ["Notes"]))],
              "groups": [{"id": 1, "name": "Product", "members": [0, 1], "collapsed": False,
                          "pinned": True, "generated_anchor": False}]}
    if args.colors:
        layout["groups"][0]["color"] = 0x7aa2f7
        layout["workspaces"][1]["color"] = 0x9ece6a
        layout["workspaces"][2]["color"] = 0xe0af68
        tabs = main["tabs"] if main["kind"] == "pane" else main["children"][0]["tabs"]
        for tab, color in zip(tabs, [0x7aa2f7, 0xf7768e, 0x9ece6a]):
            tab["color"] = color
    saved = root / "layout.json"
    saved.write_text(json.dumps(layout))
    env = {k: v for k, v in os.environ.items() if not k.startswith("RMUX_")}
    env.update(RMUX_CONFIG=str(config), RMUX_SHELL=str(shell), MTL_HUD_ENABLED="0")
    subprocess.run([str(binary), "--check-layout", str(saved)], env=env, check=True)
    if not args.check:
        print("Synthetic preview: close its window to finish. Your configuration is unchanged.", flush=True)
        process = subprocess.Popen([str(binary), "--layout", str(saved), "--rpc-stdio-live"],
                                   env=env, stdin=subprocess.PIPE, stdout=subprocess.DEVNULL, text=True)
        try:
            process.wait()
        except KeyboardInterrupt:
            if process.poll() is None:
                try:
                    process.stdin.write('{"jsonrpc":"2.0","id":1,"method":"system.quit"}\n')
                    process.stdin.flush()
                except BrokenPipeError:
                    pass
                process.wait(timeout=10)
        finally:
            process.stdin.close()
