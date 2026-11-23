#!/usr/bin/env python3
"""Native macOS SSH authentication checks against a temporary loopback sshd."""
import getpass
import json
import os
from pathlib import Path
import socket
import subprocess
import tempfile
import time


def wait_for(check, message):
    deadline = time.monotonic() + 8
    while not check():
        if time.monotonic() >= deadline:
            raise AssertionError(message)
        time.sleep(.02)


def stop(process):
    if process is not None and process.poll() is None:
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=5)


def main():
    binary = Path("target/debug/rmux").resolve()
    cli = Path("target/debug/rmuxctl").resolve()
    with tempfile.TemporaryDirectory(prefix="rmux-ssh-loopback-") as directory:
        root = Path(directory)
        for name in ["host", "client", "encrypted"]:
            subprocess.run(["/usr/bin/ssh-keygen", "-q", "-t", "ed25519", "-N", "fixture-passphrase" if name == "encrypted" else "", "-f", str(root / name)],
                           check=True, capture_output=True)
        authorized = root / "authorized"
        authorized.write_text((root / "client.pub").read_text() + (root / "encrypted.pub").read_text())
        with socket.socket() as reservation:
            reservation.bind(("127.0.0.1", 0))
            port = reservation.getsockname()[1]
        server_config = root / "sshd_config"
        server_config.write_text(f"ListenAddress 127.0.0.1\nPort {port}\nHostKey {root}/host\nPidFile {root}/sshd.pid\nAuthorizedKeysFile {authorized}\nAllowUsers {getpass.getuser()}\nPasswordAuthentication no\nKbdInteractiveAuthentication no\nUsePAM no\nStrictModes no\nAllowTcpForwarding local\nPermitOpen 127.0.0.1:{port}\nForceCommand /usr/bin/printf rmux-loopback-authenticated\n")
        known = root / "known_hosts"
        correct_host = f"[127.0.0.1]:{port} " + (root / "host.pub").read_text()
        known.write_text(correct_host)
        client_config = root / "ssh_config"
        client_config.write_text(
            f'Host loopback-proxy\n ProxyCommand /usr/bin/ssh -F "{client_config}" -W %h:%p loopback-fixture\n'
            f"Host loopback-encrypted\n IdentityFile {root}/encrypted\n BatchMode no\n"
            f"Host loopback-fixture loopback-proxy\n IdentityFile {root}/client\n BatchMode yes\n"
            f"Host *\n HostName 127.0.0.1\n Port {port}\n User {getpass.getuser()}\n"
            f" IdentitiesOnly yes\n IdentityAgent none\n StrictHostKeyChecking yes\n"
            f" UserKnownHostsFile {known}\n GlobalKnownHostsFile /dev/null\n")
        wrapper = root / "ssh"
        wrapper.write_text('#!/bin/sh\nexec /usr/bin/ssh -F "$RMUX_TEST_SSH_CONFIG" "$@"\n')
        wrapper.chmod(0o700)
        config = root / "config.json"
        config.write_text(json.dumps({"confirm_close": "never", "session_file": str(root / "session.json")}))
        endpoint = root / "rpc/rpc.sock"
        env = {key: value for key, value in os.environ.items() if not key.startswith("RMUX_")}
        env.update(PATH=str(root), RMUX_CONFIG=str(config), RMUX_SHELL="/usr/bin/false",
                   RMUX_SOCKET_DIR=str(endpoint.parent), RMUX_TEST_SSH_CONFIG=str(client_config), SSH_ASKPASS_REQUIRE="never", LC_ALL="C")

        def call(method, **params):
            result = subprocess.run([str(cli), "--socket", str(endpoint), method, json.dumps(params)],
                                    capture_output=True, text=True, timeout=10)
            reply = json.loads(result.stdout)
            assert result.returncode == 0 and "result" in reply, (reply, result.stderr)
            return reply["result"]

        server = app = None
        with (root / "server.log").open("w+") as server_log, (root / "app.log").open("w+") as app_log:
            try:
                server = subprocess.Popen(["/usr/sbin/sshd", "-D", "-e", "-f", str(server_config)],
                                          stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=server_log)
                def listening():
                    assert server.poll() is None, "temporary sshd exited"
                    try:
                        with socket.create_connection(("127.0.0.1", port), timeout=.1):
                            return True
                    except OSError:
                        return False
                wait_for(listening, "loopback sshd did not listen")
                app = subprocess.Popen([str(binary)], env=env, stdin=subprocess.DEVNULL,
                                       stdout=subprocess.DEVNULL, stderr=app_log)
                wait_for(lambda: endpoint.exists() or app.poll() is not None, "native app did not listen")
                assert app.poll() is None, "native app exited"
                window = call("window.list")["windows"][0]["id"]

                def connection(expected, destination="loopback-fixture", passphrase=None):
                    workspace = call("workspace.create", window_id=window, ssh_destination=destination)["id"]
                    surface = call("surface.list", workspace_id=workspace)["surfaces"][0]
                    assert surface["ssh_destination"] == destination
                    if passphrase is not None:
                        wait_for(lambda: "Enter passphrase for key" in call("surface.read_text", surface_id=surface["id"])["text"],
                                 "SSH did not prompt for the encrypted key in its PTY")
                        call("surface.send_text", surface_id=surface["id"], text=passphrase + "\n")
                    wait_for(lambda: expected in call("surface.read_text", surface_id=surface["id"])["text"],
                             "SSH terminal did not report expected authentication outcome")
                    wait_for(lambda: call("surface.list", workspace_id=workspace)["surfaces"][0]["process_exited"],
                             "SSH client did not exit")
                    if passphrase is not None:
                        assert passphrase not in call("surface.read_text", surface_id=surface["id"])["text"], "passphrase was echoed"

                connection("rmux-loopback-authenticated")
                connection("rmux-loopback-authenticated", destination="loopback-proxy")
                connection("rmux-loopback-authenticated", destination="loopback-encrypted", passphrase="fixture-passphrase")
                known.write_text(f"[127.0.0.1]:{port} " + (root / "client.pub").read_text())
                connection("REMOTE HOST IDENTIFICATION HAS CHANGED")
                known.write_text(correct_host)
                authorized.write_text("")
                connection("Permission denied")
                server_log.flush()
                server_log.seek(0)
                assert server_log.read().count("Accepted publickey") == 4, "unexpected successful authentication"
                call("system.quit")
                app.wait(timeout=8)
                assert app.returncode == 0
            finally:
                stop(app)
                stop(server)
        print("PASS: native SSH alias/key authentication, configured proxy, interactive encrypted-key passphrase without echo, changed-host-key rejection and unauthorized-key rejection against isolated loopback sshd. Account passwords, agents and visuals unverified.")


if __name__ == "__main__":
    main()
