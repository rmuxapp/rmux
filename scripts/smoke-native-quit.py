"""Run the production AppKit quit bridge in an isolated app with a deadline."""
import subprocess
import sys
if sys.platform != "darwin":
    raise SystemExit("This probe requires macOS")
result = subprocess.run(["target/debug/examples/native-quit-probe"],
                        capture_output=True, text=True, timeout=10)
assert result.returncode == 0, result.stderr
assert "PASS AppKit" in result.stdout, (result.stdout, result.stderr)
print(result.stdout.strip())
