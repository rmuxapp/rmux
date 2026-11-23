# Notifications and unread activity

[← User guide](../user-guide.md)

## On this page

- [Desktop notifications](#desktop-notifications)
- [Notification panel](#notification-panel)
- [Shell notifications and program status](#shell-notifications-and-program-status)
- [Unread indicators](#unread-indicators)
- [Mark all read](#mark-all-read)

## Desktop notifications

**Desktop notifications** in Settings enables optional OS banners for unfocused
terminals. Save settings to keep the choice (`"native_notifications": true` in
JSON); the default is off. **Show desktop banners** chooses between an unfocused
terminal and the entire app being in the background (`"notification_focus":
"unfocused_terminal"` or `"background_app"`). On macOS, use the bundled app and allow Notification
Center permission when prompted. Delivery failures appear in the terminal.
Banners are limited to one per second and a bounded queue; the in-app notification
history remains available independently. Turning the setting off skips queued
requests but does not recall a banner already submitted to the OS. On macOS and Linux, banner clicks route to the terminal’s current workspace/window unless
a modal interaction is active. Click listeners are capped at 16 and expire after
two minutes. Linux delivery depends on the desktop notification server supporting
actions; rmux requests banner expiry and bounds its own listener even when the
server ignores it. Native click behavior still needs validation. Windows click
routing, installer registration and platform acceptance remain pending.

## Notification panel

The notification panel lists messages newest-first across all workspaces in the
window, using arrival order rather than pane order. Keyboard selection remains
attached to the same message as newer messages arrive.

Use Mark read on a card, or R for the selected notification, to acknowledge only
that message without opening its terminal. It remains in All history. Other
messages and bell/task-status alerts are unaffected.

Notification previews show up to two title lines and four body lines. More/Less
expands one message at a time; press E to toggle the selected message. Expansion
does not mark it read.

Each notification card has a Copy button for its complete title and body. With the panel
focused, Command-C on macOS or Control-C on Linux/Windows copies the selected
message. Copying preserves unread state and does not open the source terminal.

The notification panel owns keyboard focus while open. Up/down, Page Up/Down,
and Tab/Shift-Tab navigate messages. Page keys use the visible panel height and
measured card sizes, advancing at least one message. Enter/Space closes the panel and opens the
selected terminal. Delete/Backspace dismisses the selected message, and Escape
closes the panel and restores its prior focus. Selection tracks terminal and message revision
as messages arrive; the panel and Settings are mutually exclusive.

## Shell notifications and program status

OSC 9, OSC 777 and OSC 99 title/body notification requests mark unfocused
terminals unread, like BEL. OSC 99 supports chunk assembly and Base64 text, with
up to 16 pending message IDs, 256 title bytes and 1024 body bytes. OSC 99
`o=always` (default), `o=unfocused` and `o=invisible` control whether a request
is accepted. Empty `p=?` queries return `p=title,body,?` and the supported `o`
values, preserving the query identifier and BEL/ST terminator. Queries do not
create notifications. Base64 text accepts padded or unpadded chunks, including encoding groups and
Unicode characters split across successive chunks.
Hidden tabs/workspaces, zoom-hidden panes and inactive OS windows
count as invisible. Suppressed requests are discarded, not replayed on blur.

OSC 9;4 progress reports show a percentage or Working/Paused/Error label in the
terminal tab. Remove reports and terminal reset clear it; no polling is used.
Process exit or terminal output closure clears running, indeterminate and paused
progress. Error progress remains visible until cleared or reset. Buffered activity
reports arriving after completion cannot restore a stale Working indicator.
OSC 7501 program status shows a tab label and hover details. Blocked work takes
priority, followed by errors, completed work, running work and idle state; `+N`
indicates additional records. Up to 64 records are kept per terminal. Shell prompt,
exit, reset and focus events clean up records according to their state.

View → Status and Notifications → Toggle Program Status (also in the palette) shows all records for the
focused terminal, including hierarchical IDs, inherited app names and messages.
If terminal output is busy, the panel opens with a Loading state and fills in
automatically; closing it cancels the pending load.
Hover a truncated task heading or app/title line for details. Each task's **Copy**
control copies its heading, app/title and complete message to the clipboard.
Opening Program Status focuses the panel. Use ↑/↓ or Home/End to select a task,
or Page Up/Down to move by the visible panel height, accounting for long messages.

Use C or the Copy shortcut to copy its complete details. Escape closes the panel
and returns to the terminal. Clicking a task also selects it; selection follows
its task ID as reports update.
View → Status and Notifications → Toggle Notifications (also in the palette) opens a panel with up to 32 recent
messages per live terminal, with separate read state for each message. All and Unread
filters show live counts; press U while the panel is focused to switch filters.
Arrow keys or Tab navigate the visible messages, Enter opens the selected terminal,
and Delete dismisses the selected message. Switching filters preserves selection
when that message remains visible. Click a message to focus its terminal, or Dismiss
to acknowledge it without switching focus. Panes with unread tabs show an amber
border. A new notification appears again after dismissal. Focusing a terminal or Mark All
Read acknowledges retained messages without deleting them. Dismiss All clears the
window’s retained messages while preserving bell/status alerts. History lasts for
the terminal’s lifetime; native OS notifications are not yet implemented. OSC 99
action/close reports, replacement IDs after delivery,
expiry, icons and buttons remain unsupported.

Cmd+N creates a workspace; click sidebar rows or use Cmd+Alt+Up/Down to switch.
Right-click a workspace for Rename, Description, Color, Icon, Pin/Unpin and
Close actions. This selects that workspace and clears sidebar multi-selection;
Close uses the normal confirmation policy. Arrows/Home/End navigate the menu,
Enter/Space activates, and Escape restores terminal focus. Outside click or
focus loss dismisses it. Show Workspace Actions in the palette also opens this
menu; `workspace_menu` is available for custom shortcut bindings.
Right-clicking a group header instead offers Rename Group, group Color/Icon,
Pin/Unpin Group, Ungroup Workspaces and Close Group. Ungroup keeps the workspaces
and their running terminals; Close Group uses the existing group-close prompt.

Show Workspace Actions still targets the active workspace, including a group
anchor, so its individual metadata remains accessible.
View → Workspace Icon offers Terminal, Code, Build, Cloud, Notes and None.
The same choices are available in the palette. Icons persist in saved layouts
and follow a workspace when it moves into a new window.
File → Workspaces → Edit Workspace Icon accepts a single symbol or emoji. Edit Workspace Color
accepts an exact six-digit hex color, with or without `#`. Both are also in the
palette; an empty value clears the setting. Invalid input stays open for correction.

Both editors also show preset icon buttons or color swatches. Click a preset to
fill the draft, then press Enter to save or Escape to cancel. Preset edits support
the text field's undo/redo; choosing None stages an empty value.

## Unread indicators

A bell from an unfocused terminal marks its tab and workspace with an amber dot. Focusing that
terminal in the active window clears the marker. System notifications and sounds
are not implemented yet.

Cmd+Shift+U jumps to an unread terminal (Ctrl+Alt+U on Linux/Windows), also under
View → Status and Notifications → Next Unread Terminal. It searches the current workspace first, then wraps
through this window's other workspaces in sidebar order. If a workspace is zoomed,
the target pane becomes the zoomed pane. With no unread terminals it does nothing.

## Mark all read

Mark All Read in This Window is available in the View menu, command palette,
and notification panel. It acknowledges bell, notification and task-status
alerts across all workspaces and hidden tabs in the current window while keeping
messages visible. Other windows retain their unread state. A custom shortcut can
bind `mark_all_read`. Clear all dismisses messages instead.
