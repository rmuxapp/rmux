# Workspaces, tabs and panes

[← User guide](../user-guide.md)

## On this page

- [Merge workspaces](#merge-workspaces)
- [Move a workspace to another window](#move-a-workspace-to-another-window)
- [Git branch labels](#git-branch-labels)
- [Layout actions and pane joining](#layout-actions-and-pane-joining)
- [Workspace groups](#workspace-groups)
- [Move to the next window](#move-to-the-next-window)
- [Resize the sidebar](#resize-the-sidebar)
- [Drag a tab between workspaces](#drag-a-tab-between-workspaces)
- [Detach, transfer and equalize](#detach-transfer-and-equalize)
- [Workspace colors, descriptions and pinning](#workspace-colors-descriptions-and-pinning)
- [Rename workspaces and manage windows](#rename-workspaces-and-manage-windows)
- [Tabs and tab switching](#tabs-and-tab-switching)
- [Close tabs](#close-tabs)
- [Directional pane focus](#directional-pane-focus)
- [Resize panes from the keyboard](#resize-panes-from-the-keyboard)
- [Close finished tabs](#close-finished-tabs)
- [Context-menu keyboard navigation](#context-menu-keyboard-navigation)
- [Find finished tabs](#find-finished-tabs)
- [Split left and above](#split-left-and-above)
- [Search palette shortcuts](#search-palette-shortcuts)

## Merge workspaces

**Merge Workspace into…** in the menu or command palette searches other
workspaces across this app’s windows, labeled by window. Enter moves the source's existing panes into
the destination, preserving both nested split trees with the source on the right,
then removes the source workspace entry. Destination labels, pin/group membership
and metadata remain; source workspace metadata is discarded. Pane zoom clears so
both trees are visible. Escape cancels. The custom shortcut action is
`merge_workspace`. Busy/stale destinations and excessive split depth reject before
transfer. Cross-window merges rebind terminal input/focus to the destination; an
empty source window closes. Native session/focus/group acceptance remains pending.

## Move a workspace to another window

Choose **Move Workspace to Window…** from the menu or command palette to search
other windows by their active workspace name. The picker shows workspace counts
and whether a destination is busy. Enter moves the originating workspace and
activates the destination; Escape returns to the original focus. Closed or busy
destinations are checked again when selected. The custom shortcut action is
`move_workspace_window_picker`; the existing next-window command remains available.

## Git branch labels

Workspace directory subtitles include the Git branch or detached HEAD when
available. Discovery runs in the background on shell directory reports. A shared
native filesystem watcher refreshes branch badges on HEAD changes, including
external checkouts, without running Git or polling while idle. Filesystems that
do not deliver native events still rely on shell reports or manual refresh.
Use **Refresh Pull Request for Current Terminal** from the command palette to
look up the current local branch with GitHub CLI. `gh` must already be available
on the app's PATH and authenticated; rmux does not install it or prompt for login.
The sidebar shows the returned PR number, state and title on a separate line below Git context;
hover for the full text and destination URL. Click the PR summary to open the
page without switching workspaces. **Open Current Pull Request** provides the
same workflow from the keyboard through the command palette.
While a lookup runs, the palette offers **Cancel Pull Request Lookup** instead
of Refresh. Open appears once a PR is available. Custom shortcut actions are
`refresh_pull_request`, `cancel_pull_request` and `open_pull_request`.

Lookup is manual, limited to one lookup at a time, and has a shared ten-second
budget. It uses the repository selected by `gh` and searches open PRs for the
current branch first. If none are open, it checks all states for a single past PR.
Multiple matches show **Multiple matching PRs** instead of choosing one; select
the intended PR in GitHub. Forks with identical branch names may need repository
configuration in `gh`. No result and lookup failure have separate sidebar messages. Directory/branch changes clear the cached result;
refresh again for server-side changes. Remote SSH lookup and automatic PR polling
are not implemented. Native visual/interaction acceptance remains pending.

**Show Listening Ports** provides an on-demand local terminal process-tree
snapshot. For local terminals, the sidebar retains a compact **Ports snapshot**
line after closing the panel, showing up to four unique port numbers and a count
of any others. Partial scans are labeled. This is the last requested snapshot,
not a live service-health indicator; reopen/refresh Listening Ports to update it.
The line follows the selected terminal and disappears when that terminal exits.
PR and port summaries do not replace the branch/directory line.
The palette's Refresh Git Branch for Current Terminal action refreshes explicitly.
Repeated shell directory reports refresh Git even when the path stays unchanged;
the included zsh integration reports its directory at each prompt. `surface.list`
exposes the cached value as `git_branch` (null when unavailable).

## Layout actions and pane joining

Layout and workspace editing also support these action names:
`equalize_panes`, `swap_pane_next`, `break_tab_right`, `break_tab_down`, `break_tab_left`, `break_tab_up`,
`join_pane_next`, `move_tab_next_workspace`, `detach_tab`, `detach_workspace`,
`move_workspace_next_window`, `move_workspace_window_picker`, `rename_tab`, `rename_workspace`,
`toggle_workspace_pin`, `edit_workspace_description`, `edit_workspace_icon`,
and `edit_workspace_color`. These reuse menu handlers, preserving live terminal
sessions during movement and existing split limits. They have no additional
default bindings; choose keys in your configuration. For example:

```json
{"keybindings":[
  {"key":"ctrl-k ctrl-e","action":"equalize_panes"},
  {"key":"ctrl-k ctrl-j","action":"join_pane_next"},
  {"key":"ctrl-k ctrl-d","action":"detach_tab"}
]}
```

Directional swaps are available as **Swap Pane Left/Right/Up/Down** in the View
menu and command palette, or through `swap_pane_left`, `swap_pane_right`,
`swap_pane_up`, and `swap_pane_down` custom bindings. They choose the neighboring
pane using the same geometry as directional focus, exchange entire panes
(including all tabs), and retain focus on the moved terminal. Split proportions
stay fixed. At a layout edge, or while zoomed, they do nothing. No default keys
are assigned.

**Join Pane Left/Right/Up/Down** moves all tabs from the focused pane into the
neighbor in that direction, then removes the empty split. Existing destination
tabs come first, followed by source tabs in their original order; the source's
active terminal stays selected. Other tabs retain their unread state. The
custom actions are `join_pane_left`, `join_pane_right`, `join_pane_up`, and
`join_pane_down`. These do nothing at a layout edge and have no default keybindings.
All join actions require an unzoomed, idle layout. Pending tab creation, pane
editors, close prompts or terminal overlays in either pane prevent the join;
an explanation appears without moving any tabs.

## Workspace groups

Workspace groups are available from the File menu and command palette: create
an empty group (Ctrl-Cmd-G on macOS, Ctrl-Alt-G elsewhere), group the current
workspace, rename, pin or ungroup. Headers represent the anchor terminal;
their chevron collapses members and `+` adds a workspace. New workspaces within
a group inherit the anchor's cached working directory by default and appear after the
active member. Closing the anchor promotes the next member. Groups are saved
with layouts and sessions. “Use Current Terminal Directory for Group” in the
palette fixes the starting directory for new members; “Reset Group Directory to
Follow Anchor” restores the default. Existing shells keep their directories.

Use the palette's “Move Workspace to Group” commands
to regroup existing workspaces. File-menu/palette actions can remove one member
or make it the anchor without restarting terminals. Multi-selection, group drag/drop, configurable
placement and group automation are still in progress. Group color and icon
editors are available in the File menu and palette; styling is saved independently
of the anchor workspace.
Group children support Move Workspace Up/Down and drag placement after another
group member. Dropping a child onto a standalone workspace removes it from its
group. Header drags and Move Workspace Up/Down reorder whole groups among
top-level rows in the same pinned/unpinned section. Native drag acceptance is
still pending.

Workspace number shortcuts and next/previous navigation follow group order.
Collapsed members keep their shortcut numbers; selecting one expands its group.
Group headers show workspace and unread-workspace counts, including hidden
members. Their notification preview shows the newest unread message across all
members, regardless of group order. Collapsing a group does not acknowledge its
notifications.
Close Workspace Group in the File menu/palette closes its terminals after one
confirmation for a multi-member group. Ungroup Workspaces keeps them running.
Cmd-click (macOS) or Ctrl-click toggles sidebar selections; Shift-click selects
a visible range. Group Selected Workspaces (Ctrl+Cmd+Shift+G on macOS,
Ctrl+Alt+Shift+G on Linux/Windows) groups them in sidebar order with no
new shell. Its shortcut is Cmd-Shift-G on macOS, Ctrl-Alt-Shift-G elsewhere.

Settings → New group workspace chooses After current (default), First or Last
for where Cmd-N and the group header `+` insert new members. Save persists it;
changes apply immediately. The config equivalent is `group_new_workspace_placement`
with `after_current`, `top` or `end`.
The header button uses the anchor as its reference; `top` always inserts directly
below the anchor. Reload Settings applies changes to subsequent creations.
The File menu also includes the group directory capture/reset actions. Custom
shortcut action names include `new_group`, `group_selected`, `rename_group`,
`use_current_directory_for_group` and `reset_group_directory`.

## Move to the next window

Move Workspace to Next Window in the menu or palette transfers the active
workspace into the next existing rmux window in the application's window list,
wrapping around. It retains live terminals, layout and workspace metadata;
pinned workspaces join the destination's pinned group. The destination becomes
active, and an emptied source window closes. If there is no other window, the
action shows a message; Move Workspace to New Window creates a destination.
Transfers into windows restoring a layout or waiting for window-close
confirmation are rejected before ownership changes.
The palette also lists `Move Workspace to Window N — workspace name` for each
other window, labelled with its active workspace. These entries retain the
source workspace identity and destination window handle; a closed destination
reports an error without removing the source workspace.

## Resize the sidebar

Drag the sidebar's right edge to resize it; double-click the edge to restore its
200px default width. Save Layout includes the preferred sidebar width.
Double-click a workspace row to rename it. Hover over a truncated workspace name
to read it in full; the New Workspace button also shows its platform shortcut.
In the command palette, arrow keys select results and Page Up/Page Down jump eight
results at a time. Hover over a truncated command title to read it in full.

## Drag a tab between workspaces

Drag a terminal tab onto another sidebar workspace to move that specific tab into
the destination's remembered pane. Valid rows highlight during the drag; the
destination becomes active after dropping. Cross-window tab drops are not supported.

## Detach, transfer and equalize

View → Move Workspace to New Window (also in the command palette) moves the whole
workspace with its existing splits, tabs, live sessions, labels, color and pin
state. The source window closes if it has no remaining workspaces.

View → Equalize Pane Sizes (also in the palette) gives siblings equal space at
every split, exits pane zoom and preserves the split tree, tabs and running shells.

View → Detach Tab to New Window (also in the command palette) moves the active
tab into its own window, retaining its running shell, output and font size.
An emptied source pane/workspace closes.

The command palette also offers `Move Tab to Workspace …` for each other workspace
in this window. Search by its name to choose a destination directly; numbered
results distinguish duplicate names and resolve targets by stable workspace ID.
The browser demo also provides **Choose Terminal Tab** from the pane header
and command palette. Its sample tabs can be searched by title or tab number;
Enter selects and Escape returns to the terminal. Desktop directory and process
metadata are not simulated by that chooser.

Multi-tab pane headers also have a `⌄` tab picker for crowded strips. Use its list
to see titles and unread markers; Up/Down navigates, Enter selects, and Escape
returns to the terminal without switching tabs. Page Up/Down advances by the
visible menu height using measured row sizes, including directory subtitles.
Open it directly with ⌘⇧T on macOS or Ctrl+Alt+Shift+T elsewhere, or choose
“Choose Terminal Tab” in the command palette. Type to filter titles and working
directories: all whitespace-separated query words must match either field,
ignoring case. Abbreviations work as in the command palette: `bld rmx` can match
“Build server” in `/work/rmux`. Contiguous matches rank ahead of subsequences;
equally strong title matches rank ahead of directory matches. With an empty query,
tabs retain their original order. Each row shows its cached working directory below the title when
available, with the full title and path in its tooltip. This uses the last
shell-reported directory or initial directory, without filesystem polling.

Keyboard selection follows the terminal identity when live title/directory updates
change result order. If that terminal no longer matches, the first result becomes
the stored fallback and is revealed, so an old match cannot silently reclaim
selection later; changing the query starts at the first result.
Moving focus elsewhere dismisses the picker. If a result disappears while the
list is open, selection stays near its previous row. Enter only opens the selected
matching tab; a stale target does not silently open the first result.
The picker displays original tab numbers, a matching count and navigation hints.
A displayed tab number matches an exact query word: `3` prioritizes tab 3, and
`3 server` also requires a title or directory match for “server”. Numbered titles
and paths remain searchable normally. Home/End edit the search field. Page Up/Down moves
the highlighted choice by eight results without activating it; Enter opens the
choice and Escape cancels. Empty results cannot activate a terminal. The search
field uses the shared Unicode composition and editing component.

Command-palette terminal results also show and search cached working directories
across all open rmux windows. Other-window results include a window label;
selecting one focuses its existing terminal and activates that window. Closed,
moved or busy destinations show an error instead of switching to another tab.
These paths and targets are captured when the palette opens; reopen it to refresh
them after a shell changes directories or a terminal moves.
Other-window workspaces also appear as `Switch to Window … / Workspace: …`.
Selecting one restores that workspace's remembered pane focus and activates its
window. Switching does not transfer tabs or restart shells; closed, moved or busy
destinations produce a notice.

Direct terminal navigation reveals its workspace if hidden inside a collapsed
group and follows that terminal's pane when the workspace is zoomed.
Tab transfers also reveal a collapsed destination group. Move Tab to Next
Workspace follows sidebar order, including group members.
Next Unread Terminal and the notification list also follow sidebar workspace
order, including collapsed group members. Unread navigation checks the current
workspace first, wraps once, and focuses only the chosen unread terminal.
The notification panel shows a message count and a Clear all control. The command
palette also offers Dismiss All Notifications. Both dismiss current messages in
this window and clear their unread indicators without switching terminals;
subsequent notification revisions remain visible.

For another existing window, choose `Move Tab to Window … / Workspace: …`.
The destination's remembered pane receives the existing terminal and custom tab
label, becomes selected, and follows zoom if enabled. The destination window is
activated. Closed or busy destinations report an error before removing the source
tab. Native cross-window tab interaction acceptance is still pending.
View → Move Tab to Next Workspace (also in the command palette) appends the active
terminal to the next workspace's remembered pane, wrapping at the end. Its session
stays alive and the destination becomes active. Empty source panes/workspaces
collapse; with only one workspace, the action does nothing.

View → Join Pane into Next (also in the command palette) appends the focused
pane's tabs to the next pane in layout order, preserves their order and active
terminal, then collapses the empty split. The last pane wraps to the first;
a single pane is unchanged. Like directional joins, it requires an unzoomed,
idle layout; finish pending tab creation/editing in both panes and retry.
View → Move Tab to Split Left / Right / Above / Below (also in the command palette) moves the
active tab into a new adjacent pane without starting a new shell. It requires at
least two tabs in the source pane and focuses the moved tab. A single-tab pane
is unchanged.

Tab-to-split moves, tab drags between panes and transfers between workspaces in
the same window require pending splits/resizes and tab creation to finish.
Close editors or overlays in the participating panes before retrying a refused
move. Refusal leaves the tab and its process in place.
View → Swap Pane with Next (also in the command palette) exchanges the focused
pane with the next pane in layout order, wrapping at the end. Tabs and sessions
stay attached to their pane, focus follows the moved pane, and split proportions
stay attached to their positions. Swapping exits zoom to reveal the new layout.

Cmd+Shift+Enter toggles pane zoom (Ctrl+Shift+Enter on Linux/Windows), also available
under View. Click the restore strip or repeat the shortcut to restore the splits.
Ctrl+Tab cycles panes while zoomed. Directional focus shortcuts also switch the
zoomed pane, following the underlying split layout; an outer edge is a no-op.
Creating or closing a pane restores the layout. Directional swap/join operations
still require the split layout to be visible.

Drag split dividers to resize adjacent panes; double-click to give the adjacent
branches equal space. Resizing and closing panes preserve surviving sessions.

## Workspace colors, descriptions and pinning

View → Workspace Color offers six colors and None to reset the row.
The selected color fills the entire workspace or group row, with theme-aware
contrast and a visible selection border.
The same named choices appear in the command palette. Colors persist in saved
layouts and do not change the terminal's ANSI colors or unread indicator.

File → Edit Workspace Description (also in the palette) edits a secondary sidebar
label. Enter saves, Escape cancels, and an empty value clears it. Descriptions
are retained in saved layouts; older layouts default to no description.

View → Pin / Unpin Workspace (also in the palette) keeps pinned workspaces at the
top. Reordering stays within pinned/unpinned groups. Save/Open Layout preserves
pins; older layouts open unpinned.
View → Last Workspace (also in the palette) toggles between the current and most
recently visited workspace. Reordering preserves the target; closing it clears
the remembered destination. This history is per window and is not saved.
Cmd+= / Cmd+- change the focused terminal's font size; Cmd+0 resets it to the
configured font size (14 by default).
Linux/Windows use Ctrl+= / Ctrl+- / Ctrl+0. These controls are also in Edit and the
command palette. Zoom is per terminal, ranges from 8 to 40, and follows a moved
tab. New tabs and splits inherit the source terminal's size; a new workspace starts
at the configured size. Save/Open Layout preserves each terminal's size; older
layouts without a saved size default to 14. Reset Font Size preserves that
terminal's font family; Use Configured Font restores both the configured family
and size. Custom shortcuts can bind these separately as `reset_font_size` and
`use_configured_font`, or use `increase_font_size`/`decrease_font_size` for steps.

Cmd+P opens the command palette (Ctrl+Shift+P on Linux/Windows). Type words
to filter implemented actions and workspace names (abbreviations such as `splt blw`
also match, with contiguous matches ranked first), use Up/Down and Enter to run
an action or switch workspaces, or Escape to return to the previous focus.
Workspace results include their sidebar position to distinguish duplicate names.
Clicking outside dismisses it.
Cmd+1 through Cmd+8 select the first eight workspaces; Cmd+9 selects the last.
Linux/Windows use Ctrl+Alt+1 through Ctrl+Alt+9. Sidebar rows show shortcut hints,
and keyboard selection scrolls the selected workspace into view.
Cmd+B toggles the sidebar. Linux/Windows use Ctrl+Shift+N, Ctrl+Alt+Up/Down and
Ctrl+Shift+B. Switching preserves each workspace's sessions, split layout and
last focused pane. Cmd+Alt+Shift+Up/Down moves the active workspace in the sidebar
(Ctrl+Alt+Shift+Up/Down on Linux/Windows).

## Rename workspaces and manage windows

Cmd+Shift+R renames the active workspace (Ctrl+Shift+R on Linux/Windows), also
available in the File menu. Enter saves and Escape cancels. Names appear in the
sidebar and native window title; Save Layout preserves them for manual restoration.

Cmd+Shift+N opens a separate native window; Cmd+Shift+W closes the current window.
Linux/Windows use Ctrl+Alt+N and Ctrl+Alt+W. Each window owns its workspace list.
Closing a window drops its terminal sessions; quitting the app cleans up sessions
across all windows. Native menus expose the main window, workspace, tab, edit and
split commands where the platform provides an application menu.
On macOS, normal system-originated app quits now route through rmux's confirmation
and three-second cleanup path. The initial AppKit request is canceled, then
reissued after approval and cleanup. This does not preserve a pending macOS
logout/shutdown request; retry that system operation after rmux exits. Native
confirmation interaction still needs manual acceptance. Other platform-originated
quits retain GPUI's best-effort shutdown hook, which cannot veto termination and
allows only 100 ms for cleanup. Force Quit cannot be intercepted.

## Tabs and tab switching

In the next release, **Focus Back** and **Focus Forward** in the menu and
command palette revisit terminal focus across this window's workspaces and
panes. History keeps up to 64 visits and skips closed terminals. Choosing a
different terminal after going back clears the forward path. These actions are
unbound by default; custom shortcuts use `focus_back` and `focus_forward`.
**Last Terminal** remains a separate toggle between recent terminals.

Cmd+T creates a tab in the focused pane; Cmd+Shift+[ / ] switches tabs.
Starting with the next release, Cmd+[ / ] cycles split panes on macOS.
Ctrl+Tab / Ctrl+Shift+Tab also cycles panes. Pane and tab navigation appear
in the application menu, and their actions can be rebound in settings.
Terminal tabs can have their own colors: right-click → **Set Tab Color…**,
choose a preset or enter `#RRGGBB`, and press Enter or Save. None (or an empty
value) resets the color; Escape cancels. Tab colors persist in saved layouts and
remain attached to the terminal when it is moved.

Right-click a tab to select it and open a menu with Rename, Copy/Reveal Working
Directory, Move Tab Left/Right, Set Tab Color,
Close, Close Other Tabs and Close Tabs to the Right. Arrow keys navigate, Enter/Space activates,
and Escape returns to the terminal. Clicking outside or moving focus dismisses
the menu. Shift+F10 or Show Tab Actions in the palette opens the menu without
a mouse; the configurable action is `tab_menu`. Unavailable actions are
dimmed and skipped by keyboard navigation. Moving the pointer over an enabled
action selects it for keyboard activation too. Moving a tab preserves its running
terminal; left/right actions are unavailable at the respective ends of the strip.

Directory actions use the clicked terminal's asynchronous local-directory
lookup. Custom shortcuts can name `copy_working_directory` and
`reveal_working_directory`; unavailable directories report an error on that terminal.
If you repeat or switch these actions during lookup, the latest request replaces
the earlier one. Superseded copies do not overwrite newer clipboard content.
Close Other Tabs in the menu/palette keeps the chosen terminal and closes the
other tabs in that pane, using one confirmation when the close policy requires
it. The `close_other_tabs` shortcut action has no default binding. The prompt
captures its targets: tabs added afterward are spared, and the operation aborts
if the terminal being kept leaves the pane before confirmation.

Close Tabs to the Right uses the same policy and captured-target rules, keeping
the chosen tab and those to its left. It is available in the menu/palette and
as `close_tabs_to_right`, with no default shortcut.
New tabs start in the background. The Starting… indicator has a cancel button;
Cancel New Terminal Tab is also available in the menu/palette and as the
`cancel_new_tab` shortcut action. Cancellation preserves existing tabs and focus.
The pane accepts another launch once the pending job finishes cleanup; an OS
startup operation already in progress cannot be interrupted immediately.
Cmd+Shift+1–9 selects a tab directly within that pane (Ctrl+Shift+1–9 on
Linux/Windows). Missing tab numbers do nothing; 9 selects the ninth tab.

The palette also offers Select Tab 1–9, Next Tab and Previous Tab.
Double-click a tab to rename it, hover its title to read the full label, or
middle-click to close it using the configured close-confirmation policy.

## Close tabs

Cmd+W closes the active tab (Ctrl+Shift+W on Linux/Windows). Closing its last tab
removes the pane. Ctrl+Tab and Ctrl+Shift+Tab cycle pane focus. Closing the last
pane removes its workspace; closing the last workspace closes the window.
File → Close Workspace (also in the command palette) closes the active workspace
and its terminal sessions together, then selects a remaining workspace.

## Directional pane focus

Cmd+Ctrl+Arrow focuses a pane in that direction (Alt+Shift+Arrow on Linux/Windows).
Navigation uses the current pane geometry, prioritizing aligned neighbors, and
does not wrap at the outer edge. Ctrl+Tab still cycles panes in layout order.

## Resize panes from the keyboard

The menu and command palette expose **Resize Pane Left/Right/Up/Down**. Bind
`resize_pane_left`, `resize_pane_right`, `resize_pane_up`, or `resize_pane_down`
to your preferred keys in `keybindings`; no new default bindings are reserved.
Each action moves the nearest divider on that axis by 5% of its neighboring
pair's space. It prefers the requested side; at an outer edge it moves the
opposite shared divider in that direction, shrinking the focused pane. Nested
splits resize locally before considering outer splits. Each neighbor retains at
least 10% of the pair's space, or its existing share if it was already narrower
from a drag or restored layout. Such narrow panes never snap in the opposite
direction; growing them raises the minimum toward 10%. Existing pane identities, tabs and unrelated split
weights remain intact. Restore a zoomed layout before resizing; actions are also
ignored while a split shell is starting. **Equalize Pane Sizes** resets ratios.

## Close finished tabs

**Close Finished Tabs…** in the menu or command palette closes exited terminals
in the current pane while retaining the current tab and all running terminals.
It follows your close-confirmation policy. Tabs created or moved into the pane
while a confirmation is open are excluded. Bind `close_finished_tabs` to a custom
shortcut if desired; it has no default binding.
The tab context menu also offers this command with an eligible-tab count. It
retains the tab you opened the menu on and disables the command when no other
finished tabs are available.

## Context-menu keyboard navigation

Terminal, tab, workspace and group context menus support type-to-select: type a label prefix, or
repeat its first letter to cycle enabled matches. After a one-second pause,
typing starts a new prefix. Press Enter or Space to activate, or Escape to close.

## Find finished tabs

The pane's **Choose Terminal Tab** picker also searches finished state and exit
messages. Its scrollable results use fixed-height rows and render on demand as
you scroll, keeping large tab collections from building every result row at once.

## Split left and above

**Split Left** and **Split Above** are also available in the menu and command
palette. Bind `split_left` or `split_up` for custom shortcuts; they have no default
bindings. All four split directions inherit the target terminal’s directory and
font settings through the same shell-launch path.

## Search palette shortcuts

Palette search includes the shortcut shown beside each action. Try separate words
such as `cmd shift d` on macOS or `ctrl shift k` for a configured binding; macOS
symbols such as `⇧⌘D` also work. Combine key names with an action name to narrow
fuzzy matches. Hints reflect the originating terminal's bindings when you open
the picker, including custom overrides and disabled shortcuts.
