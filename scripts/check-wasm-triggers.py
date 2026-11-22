#!/usr/bin/env python3
"""Keep WASM trigger exclusions honest against rustc's compiled dependency file."""
from pathlib import Path
import re
import shlex
import sys

root = Path(__file__).resolve().parent.parent
web = root / "tools/web-probe"
workflow = (root / ".github/workflows/shared-demo.yml").read_text()
excluded = re.findall(r"^\s+- '!([^*?']+)'\s*$", workflow, re.MULTILINE)
depfile = Path(sys.argv[1])
dependencies = set()
for token in shlex.split(depfile.read_text().replace("\\\n", "")):
    path = Path(token.removesuffix(":"))
    path = (web / path).resolve() if not path.is_absolute() else path.resolve()
    if path.is_relative_to(root):
        dependencies.add(path.relative_to(root).as_posix())
if not any(path.startswith("crates/rmux-desktop/src/") for path in dependencies):
    raise SystemExit("No shared desktop dependencies found; check the supplied rustc .d file")
conflicts = sorted(set(excluded) & dependencies)
if conflicts:
    raise SystemExit("WASM compiles files excluded from its build triggers: " + ", ".join(conflicts))
print(f"PASS {len(excluded)} native exclusions checked against compiled WASM inputs")
