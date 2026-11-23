#!/usr/bin/env python3
"""Isolated native buffer RPC check; no user config, clipboard or live shell."""
import json
import os
from pathlib import Path
import subprocess
import tempfile


def main():
    binary = Path("target/debug/rmux").resolve()
    content = "build 世界\n" * 1000
    operations = [
        ("system.capabilities", {}),
        ("buffer.set", {"text": content}),
        ("buffer.set", {"name": "taken", "text": "keep"}),
        ("buffer.rename", {"new_name": "taken"}),
        ("buffer.get", {}),
        ("buffer.rename", {"new_name": "renamed 世界"}),
        ("buffer.get", {"name": "renamed 世界"}),
        ("buffer.get", {}),
        ("buffer.rename", {"name": "renamed 世界", "new_name": "renamed 世界"}),
        ("buffer.rename", {"name": "renamed 世界", "new_name": "bad\n"}),
        ("buffer.list", {}),
    ]
    requests = [{"jsonrpc": "2.0", "id": index, "method": method, "params": params}
                for index, (method, params) in enumerate(operations)]
    requests += [
        {"jsonrpc": "2.0", "method": "buffer.rename", "params": {"name": "renamed 世界", "new_name": "notified"}},
        {"jsonrpc": "2.0", "id": 11, "method": "buffer.get", "params": {"name": "notified"}},
        {"jsonrpc": "2.0", "id": 12, "method": "buffer.get", "params": {"name": "taken"}},
        {"jsonrpc": "2.0", "method": "system.quit"},
    ]
    with tempfile.TemporaryDirectory(prefix="rmux-buffer-smoke-") as directory:
        config = Path(directory) / "config.json"
        config.write_text(json.dumps({"confirm_close": "never"}))
        env = {key: value for key, value in os.environ.items() if not key.startswith("RMUX_")}
        env.update(RMUX_CONFIG=str(config), RMUX_SHELL="/usr/bin/false")
        result = subprocess.run([str(binary), "--rpc-stdio-live"], env=env, timeout=10,
                                input="".join(json.dumps(request, ensure_ascii=False) + "\n" for request in requests).encode(),
                                capture_output=True)
        assert result.returncode == 0, result.stderr.decode()
        responses = [json.loads(line) for line in result.stdout.splitlines()]
        assert [response["id"] for response in responses] == list(range(13)), responses
        assert "buffer.rename" in json.dumps(responses[0]["result"])
        assert responses[3]["error"]["code"] == -32008
        assert responses[4]["result"]["text"] == content
        assert responses[5]["result"] == {"name": "renamed 世界", "previous_name": "default", "bytes": len(content.encode()), "changed": True}
        assert responses[6]["result"]["text"] == content
        assert responses[7]["error"]["code"] == -32008
        assert responses[8]["result"]["changed"] is False
        assert responses[9]["error"]["code"] == -32602
        assert responses[10]["result"]["bytes"] == len(content.encode()) + 4
        assert responses[11]["result"]["text"] == content
        assert responses[12]["result"]["text"] == "keep"
    print("Native buffer rename passed: capabilities, collision preservation, Unicode content, no-op, invalid name, accounting and notification routing. UI unverified.")


if __name__ == "__main__":
    main()
