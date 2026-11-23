# SSH connections

[← User guide](../user-guide.md)

## On this page

- [Connect with SSH](#connect-with-ssh)
- [SSH from scripts](#ssh-from-scripts)
- [SSH tabs and splits](#ssh-tabs-and-splits)

## Connect with SSH

**File → SSH → Connect via SSH…** (also in the command palette, custom action
`connect_ssh`) opens a destination editor. Enter an SSH config alias, hostname,
or `user@host`, then press Enter or Connect to launch a new terminal workspace.
For an explicit port, use `ssh://user@host:2222` or
`ssh://user@[2001:db8::1]:2222`. Ports must be 1–65535. SSH URLs support no
passwords, paths or percent-encoded user information. rmux converts their user
and port to separate SSH arguments and passes the host after `--`.
rmux runs the existing `ssh` executable directly; it does not invoke a shell to
interpret the input. Use your SSH configuration for identities, proxies and
default ports. The SSH client handles configuration, host-key
verification, agent/key use and password prompts inside the terminal.

**File → SSH → SSH Hosts…** (palette/custom action `ssh_hosts`) reads concrete aliases
from `~/.ssh/config` on demand and opens a searchable picker. It also lists SSH
session targets and restored saved targets from the first 256 terminals in this
window, ahead of config aliases. Saved targets are labelled **Not connected**;
session targets describe the launched destination, not verified authentication.
Exact duplicates are merged while username case is preserved. These targets remain
available if the config cannot be read; no separate connection-history file is kept.
Enter opens a new SSH workspace directly. **Shift+Enter** or **Edit destination**
opens the selected target in the destination editor with its text selected, so you
can change the user/host/port before connecting. Editing does not alter your SSH
config or saved target. Escape closes the picker/editor without connecting;
Refresh rereads the configuration, and Enter SSH destination opens the direct editor.

Discovery follows Include paths (absolute, relative to `~/.ssh`, and `~/`) and
filename wildcards in lexical order. Nested includes and symlink cycles are handled.
It reads at most 64 KiB per file / 256 KiB total, attempts at most 64 files with eight
levels of includes, scans at most 2,048 directory entries, and returns at most 256
unique aliases. Read errors in includes retain other available aliases; the picker
marks incomplete discovery. Host patterns/negations are skipped. Context-dependent
Include tokens, environment variables and `~otheruser` paths are not expanded.
Host/Match conditions are not evaluated: this is an alias inventory, so conditional
includes may supply aliases that OpenSSH will not enable for a particular connection.

No Match commands execute and IdentityFile directives are not followed. Missing configuration yields an empty picker with
manual entry available. Cancellation and the shared four-file-worker limit apply;
no idle watcher runs. OpenSSH still reads its full configuration when connecting,
so aliases outside this discovery subset can be entered manually.
Destinations are limited to 255 ASCII bytes, without spaces or command options.
Native macOS fixture checks cover key authentication, a configured SSH proxy,
an encrypted-key passphrase prompt without echo, and rejection of changed host
keys or unauthorized keys. Account-password and agent flows remain unverified.
RPC workspace creation (local or SSH) and interactive SSH launch prepare the PTY, initial colors and first terminal
snapshot on a background worker. Color changes during startup are applied before
the workspace is inserted. Custom-command launches share this preparation path.

The SSH client must already be available on PATH; launch errors appear in rmux,
and connection/authentication errors appear in the terminal. This first remote
slice does not install a relay, resume disconnected sessions, browse remote files,
or automatically reconnect. Live authentication/platform acceptance is pending.
Sessions launched by this SSH action do not expose a local working directory:
remote shell directory reports cannot drive local file browsing, git metadata,
or new-shell directory inheritance. When the remote shell emits OSC 7, the sidebar
shows **SSH · destination · /path**, and surface JSON provides `remote_directory` separately
from local directory metadata. This is display text, not a local filesystem path;
remote file browsing and automatic remote shell integration remain unimplemented.

**Copy Working Directory** copies the most recently reported remote path as text.
It reports an error if no remote path is known. SSH context menus disable
**Reveal Working Directory**, since a remote path is not a local folder.
Remote tab titles retain an **SSH** prefix when renamed by you or the shell.
The sidebar shows the destination before any directory report arrives; after the
client exits it says **SSH ended**, marking a retained path as **Last**. An SSH
label denotes a launched client, not successful authentication. The full destination
and path remain available in the sidebar tooltip when the row is truncated.

The requested SSH destination also remains available in tab tooltips and terminal
switcher search after renaming a tab. Surface JSON includes `ssh_destination`
(null for local terminals). It identifies the launch target, not a confirmed
connection or authentication result, and does not enable automatic reconnect.
Newly saved layouts, sessions and recovery checkpoints retain the SSH destination.
Restoration still starts a fresh **Local** shell; its tooltip identifies the saved
SSH target as **Not connected**. Surface JSON exposes that target separately as
`saved_ssh_destination`. Open **Connect via SSH…** from that tab to pre-fill the
target, then press Connect to start a new SSH workspace. In the next release,
a **Connect SSH** banner also offers this action directly, keeping the restored
local shell open. No connection starts until you choose it. Re-saving retains the
target. Older snapshots without this field cannot recover an SSH destination
from an arbitrary tab title.

## SSH from scripts

CLI: `rmuxctl new-workspace --window WINDOW_ID --ssh user@host --focus true`
creates the same kind of SSH terminal. `--ssh` also accepts an SSH URL; `--name`
overrides the default workspace label. Focus and OS activation remain opt-in.
Raw `workspace.create` accepts optional `ssh_destination` alongside its existing
parameters. SSH process startup runs off the UI thread, with window/busy/capacity
checks before preparation and publication. Success returns the created workspace,
not confirmation of remote authentication; interact with its terminal for prompts.
This is an rmux extension to workspace creation, not cmux `workspace.ssh.open`
compatibility. A native macOS no-network fixture verifies CLI/RPC startup,
argument delivery, rollback, teardown and saved-target recovery (`mise run smoke-ssh`).

`mise run smoke-ssh-loopback` also checks config aliases, successful public-key
authentication, changed-host-key rejection and unauthorized-key rejection with
existing macOS OpenSSH, temporary keys and a server bound to 127.0.0.1.
Interactive password/agent/proxy flows and visual/platform acceptance remain pending.

## SSH tabs and splits

Interactive splits and new tabs inherit the active tab's explicit SSH destination,
starting a separate SSH client with its own authentication and an SSH-labelled tab.
They do not reuse remote working directories as local paths or resume remote processes.
A restored **Local / Not connected** tab starts local siblings until explicitly
reconnected. Use **File → Tabs → New Local Tab** (also in the palette, custom action `new_local_tab`)
to start a local shell in the same pane, or New Workspace for a separate workspace.
New Local Tab uses the local cached directory when available, otherwise the app's
initial directory; it never inherits a remote path or saved SSH destination.

It shares the normal background startup/cancel controls and has no default shortcut.
The tab-bar plus tooltip says New SSH Tab when the active tab supplies an SSH target.
RPC `pane.split` creates a local shell by default; its optional `ssh_destination`
explicitly starts SSH, as does `workspace.create.ssh_destination`.
Interactive split commands prepare the terminal and its first frame in the background and show
“Starting split terminal…” while pending. Each workspace permits one pending
split. Completion splits the original pane; switching focus during launch keeps
your newer focus. Closing the original pane or transferring the workspace to
another window discards the pending result. Startup failure leaves existing
panes intact.

The split inherits the source tab's font at request time, even if you switch
tabs during startup. Inherited defaults follow settings changes made while it
starts; custom font choices are retained.
Use the pending indicator's cancel control, Cancel Split Terminal in the
File menu/palette, or a custom `cancel_split` binding to discard a pending split.
The indicator changes to Canceling until background work finishes; another split
can then start. Cancellation does not interrupt an OS spawn already in progress.
