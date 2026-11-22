# Ports, memory and error messages

[← User guide](../user-guide.md)

## On this page

- [Open listening ports](#open-listening-ports)
- [Inspect listening ports](#inspect-listening-ports)
- [Memory panel lifecycle](#memory-panel-lifecycle)
- [Dismiss error banners](#dismiss-error-banners)

## Open listening ports

In **Listening Ports**, select a row and press **O**, or click its external-link
button, to open it as HTTP in your default browser. Enter still copies the raw
listener address. Wildcard bindings (`0.0.0.0` and `::`) open through IPv4/IPv6
loopback. This is an explicit HTTP action, not protocol detection; for HTTPS or
other protocols, copy the address instead. Scoped IPv6 addresses remain copy-only.
Results are snapshots: use **R** to refresh after a server restarts.

## Inspect listening ports

Use **Show Listening Ports** in the palette or menu to inspect a snapshot of local
TCP listeners owned by the current terminal’s shell and descendants. Refresh
manually and copy an address from its row. In the panel, ↑/↓, Tab/Shift-Tab or Home/End select,
Page Up/Down moves by the visible list height, and hovering a truncated address
shows it in full.
Enter or Command-C (Control-C on Linux/Windows) copies, R refreshes, and Escape
closes and returns to the terminal. **Close Listening Ports** is also in
the palette; custom actions are `listening_ports` and `close_listening_ports`.
Refresh preserves the selected address and owning PID when both remain in the
results, even if rows move. If that listener disappears, selection returns to the
first result. While refreshing, the previous snapshot stays visible and is labeled
as such. Navigation during the scan determines which listener remains selected.
Copying does not move the list; an unchanged refresh keeps your scroll position.
Failed scans clear old results rather than presenting them as current.

The panel does not discover remote-host ports or assume that a listener serves
HTTP. Results can change immediately as processes and sockets come and go. A
refresh notice appears if process ownership changed during the scan. Closing
the panel cancels remaining scan work between OS reads; an OS call already in
progress can finish first. If an immediate refresh reports another scan running,
retry after it releases the scan slot.

## Memory panel lifecycle

Closing the Terminal Memory panel releases its cached process samples. Reopening
starts a fresh CPU baseline, so refresh again after the baseline interval to see
CPU percentages. A sample already inside an OS query may finish before cleanup.

## Dismiss error banners

Terminal, tab and split-operation errors appear in dismissible banners. Use the
copy icon to copy the full message or the close icon to dismiss it. Long messages
scroll inside the banner; dismissal does not retry the failed operation.
