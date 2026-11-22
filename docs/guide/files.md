# Files and Markdown previews

[← User guide](../user-guide.md)

## On this page

- [Browse a terminal directory](#browse-a-terminal-directory)
- [Read Markdown](#read-markdown)
- [Open an external editor](#open-an-external-editor)
- [Preview limits and background loading](#preview-limits-and-background-loading)

## Browse a terminal directory

**Browse Terminal Directory…** in the menu/palette (custom action `browse_files`)
opens an on-demand local directory picker from the current terminal's cached
working directory. Search names, press Enter on a directory to browse it, or
choose Parent/Refresh. A persistent navigation bar keeps Back, Forward, Parent and
Refresh available while filtering names; unavailable destinations are disabled.
Back/Forward entries navigate successfully visited folders
in this window; custom actions `file_back` and `file_forward` provide keyboard
bindings. History retains at most 32 paths and is not persisted. Refresh preserves
Forward, while opening a different folder after Back replaces the forward branch.
Failed or canceled reads do not change history. Enter on a regular file opens a
read-only text preview; a symbolic link copies its path. Hidden entries are included.

Select a regular file and press **Shift+Enter**, or use the **Preview as Hex**
footer button, to inspect bytes directly without first loading a text preview.
Plain Enter continues to preview text. Text and hex previews keep **Directory**,
**View as Text/Hex**, and **Refresh** above the search field, including when no
lines match. Refresh retains the current preview mode.
**Preview as Hex** is also available in
text previews and file-preview errors, including
binary/invalid-encoding errors. It displays the first 8 KiB as searchable eight-byte
rows with hexadecimal offsets and an ASCII column; non-printable bytes appear as
dots. Copy Hex Dump copies the formatted text, not raw file bytes. Refresh preserves
hex mode, and Preview as Text switches back. Hex previews are read-only, capped at
1,024 rows, and share the regular-file/symlink checks, worker limit and cancellation
used by text previews.

**File → Open Folder…**, the command palette, or the `open_folder` custom action
opens the native folder chooser. This works without a terminal working directory
and loads the selected folder asynchronously. Open Folder is also available in
the directory browser and its error view. Canceling leaves the workspace unchanged.
You can also use **File → Open Text Preview…** or the palette (custom action
`open_text_preview`) to choose a single file with the system file picker.
Canceling the picker makes no change; selected files use the same bounded,
read-only preview loader.
The current path stays visible above the search field, including while filtering
or viewing an error. Hover a truncated path to see it in full. Empty text files
are labeled explicitly.

Choose **Reveal in Finder** (macOS), **Reveal in File Explorer** (Windows), or
**Reveal in File Manager** (Linux) from a directory, preview or error view to
hand the path to the system file manager. This does not open the file in its
associated application. Linux may open the containing folder without selecting
the file. Directory listings also offer **Copy directory path**. Missing or
inaccessible paths may not be revealable by the system file manager.
Choose **Go to Directory…** to enter an absolute path directly. Enter or Open
loads it; Escape or Cancel dismisses the editor. Paths are literal (no `~`,
environment-variable or shell expansion), with a 4,096-byte input limit.

Choose **New Workspace Here** to start a fresh terminal in the displayed
directory. It follows normal workspace placement and group membership, while
the browsed path takes precedence over the group's default directory.
**Cancel File / SSH Host Loading** in File or the command palette stops a pending
directory, text/hex preview, or SSH-host discovery request. It is also available
as the `cancel_file_loading` custom action; use `context: "window"` for a shortcut
that remains available outside the terminal. Canceling discards the pending UI
result and signals cooperative worker cancellation without clearing unrelated
notices. An OS read already in progress cannot be interrupted; its worker slot
remains occupied until it returns. Native file-picker cancellation stays in the
picker itself.


Refreshing the same directory retains its filter and selected file/directory by
path and entry type, even if entries reorder. If the entry disappears or no
longer matches, the first result is selected. Navigating to another directory
starts a fresh filter.

## Read Markdown

**Read Markdown** in a file preview opens a bounded reading view. Headings are
bold with a size hierarchy and searchable H1–H6 labels; wrapped headings use
up to two visual lines. Code blocks use the terminal font, and lists/quotes receive
a visual marker.
Ordered lists retain their starting number and nested lists are indented (visual
indentation stops increasing after eight levels). Continuation paragraphs do not
repeat the item marker. Task-list items (`[ ]`, `[x]`, `[X]`) show read-only
checkboxes and searchable Pending task / Completed task labels. Clicking copies
the block text; it does not toggle completion or write the file.

Block copying returns its text without added list/checkbox markers;
source copying preserves the original Markdown. In wrapped mode, code blocks
have a four-line-high scrollable viewport containing all captured source lines.
Scroll within a block to read more; long lines can scroll horizontally.
Indentation is preserved and tabs expand at four-column stops. Changing the search
brings the first matching source line into view. Copying retains the original block.
Search and highlight operate on reading text; Enter copies a block, while
**Copy Markdown Source** keeps the captured original Markdown. **View Source**
returns to the ordinary text preview. Ctrl+Alt+V toggles reading/source mode and
can be remapped as `toggle_markdown_preview` under `context: "file_preview"`.
Read Markdown / View Markdown Source also appear in the searchable preview
commands; IME composition suspends switching. Refresh/F5 keeps Markdown mode and restores
its query, wrap setting and selected block number when still present.


This initial viewer uses [pulldown-cmark](https://docs.rs/pulldown-cmark/0.13.6/pulldown_cmark/)
on the background file worker. It reads the existing 64 KiB / 512-source-line
prefix and retains at most 512 reading blocks; partial documents are labeled
when source content or an additional reading block is omitted. Exactly 512 blocks
with no omitted content remain complete.
Displayed excerpts remain bounded and default to four visual lines. HTTP(S) links
are underlined and clickable, with their destination shown on hover.
Other links display their label, images display alt text, and raw HTML displays as literal text: no
assets load and no HTML or scripts execute. Bold and italic emphasis, including nesting, render within reading blocks and
remain visible under search highlights. Inline code uses a monospace font and shaded background, and `~~strikethrough~~` draws a strike through its text.

Search colors take precedence over code shading while retaining emphasis and strikes.
Tables display structured header/body rows with separators and inline cell styling.
Each row shows up to eight equal-width columns; wider tables explicitly report
that only the first eight are shown. Row copying retains all tab-separated cells,
and Copy Markdown Source retains the original document. Empty cells remain blank.
Header and body cells honor left (`:---`), center (`:---:`), and right (`---:`)
alignment markers; unspecified columns align left. Table horizontal scrolling,
same-document anchors, images and document appearance controls
remain unimplemented. This is a reading preview, not a complete Markdown renderer.

Relative links resolve from the current document's folder. Links to Markdown
files open in reading view; other local files open in text preview. Directory
links ending in `/` open the file browser. Local `file://` links work the same
way. Missing files show the preview loader's error; they do not launch another
application. Links containing local anchors or query strings remain plain text.

Web links open in the default browser when clicked. For keyboard use, select a
reading block, use Tab/Shift+Tab to cycle its destinations, then Alt+Enter
(Option+Return on macOS) to open the displayed URL. The link bar shows the current
destination and active shortcut hints. Enter alone still copies the block text.
Search changes and row movement keep link selection tied to the block identity;
a different block starts at its first link. IME composition suspends link actions. Link destinations are limited to 8 KiB each, 64 distinct
links per block and 64 KiB across the document. Additional links remain plain text.

## Open an external editor

**External Editor** opens the selected regular file from the directory browser
or text/hex preview. Ctrl+Alt+E invokes the same action; remap `edit_selected_file`
under `context: "files"` for the browser or `context: "file_preview"` for text/hex
previews. Directories and symlink rows do not launch an editor, and IME composition
suspends the action. **Settings → External editor → Configure…** opens Settings
JSON and shows the currently configured executable (hover for the full path).
Set the editor explicitly, then Reload Settings:

```json
"file_editor": {
  "program": "/absolute/path/to/editor",
  "args": ["--", "{file}"]
}
```

Use arguments supported by your editor (`--` is an example argument, not added
by rmux). The argument list must contain exactly one `{file}` placeholder; rmux
replaces it with the absolute file path inside that argument. Optional `{line}`
and `{column}` placeholders use a terminal link’s location, or 1 when absent. It does not interpolate
shell expressions, expand environment variables, or choose a file association.
Paths with spaces, quotes or shell punctuation remain data. Directories and
symbolic links are rejected. The editor starts in the file's parent directory,
so relative editor resources resolve there rather than rmux's launch directory.
Missing-file and launch errors identify the affected file or editor executable.
The configured program controls editing and saving;
rmux does not yet provide an embedded editor, save-conflict detection, or automatic
preview refresh after external edits. Launching the editor keeps the current browser
or preview open, including its filter, selection, wrapping and scroll position.

Return to rmux and use Refresh to reload the preview after saving.
Launch errors and unsuccessful editor/launcher exits appear in the originating
window notice if that window is still open. A successful launcher exit does not
prove that a separate GUI application loaded the file. Up to eight launched editor processes
may remain active; they are reaped asynchronously without blocking a UI or worker
thread. Editors are independent applications and are not closed with the preview.

## Open paths from terminal output

Command-click on macOS (Ctrl-click elsewhere), or use **Open Link** in the
terminal context menu, to open `file://` links and path-shaped output such as
`src/main.rs:42:7`, `./tests/check.rs:12`, or an absolute file path. The first
number is the line and the second is the column. Modifier-hover shows the resolved
file path and location, with an editor configuration hint when needed. The context
menu captures the resolved destination when opened, so later shell directory
changes cannot redirect that action. Relative paths resolve from
the terminal's last reported working directory, falling back to an explicitly
set launch directory; enable shell integration so directory changes stay current.
If neither is known, rmux explains that it needs a working directory instead of
guessing. Remote targets and unsupported paths have separate explanations.

For editors that accept a combined location, configure their supported syntax.
For example, a VS Code CLI configuration can use:

```json
"file_editor": {
  "program": "/absolute/path/to/code",
  "args": ["--goto", "{file}:{line}:{column}"]
}
```

This does not run a shell. File names containing placeholder-looking text are
kept literal. Plain output detection handles fully visible wrapped lines and
Unicode cells. Matching single or double quotes preserve spaces and punctuation
in a path, such as `"/tmp/my project/main.rs"`; unquoted paths stop at whitespace
and enclosing punctuation. Line/column suffixes must be inside the quotes.
Encoded `file://` links also support names containing spaces. Bare names are recognized when they
have an extension and a numeric location, such as `main.rs:12`; otherwise use a
path containing a directory separator. Home-directory shorthand (`~/`) is not
expanded. Missing files report an error when opened; hovering never reads disk.
Paths in old output use the current reported directory, not a historical one.

Recognized remote sessions and remote file URL hosts do not open local files.
A manually started SSH session that rmux cannot identify is not covered by that
remote-context detection. The WASM demo cannot launch a local editor.

## Preview limits and background loading

Directory reads run in the background and retain at most 2,048 entries per
snapshot, sorted directories-first within that subset. A limit notice identifies
partial listings. Entries that disappear or cannot be read are omitted with an explicit
unavailable count in the path summary; Refresh retries. These entries still
consume the scan limit. Switching focus during loading prevents the result from taking
focus. Text previews read up to 64 KiB of source text and retain up to 512 lines, with explicit truncation
labels. A final line cut by the byte limit is labeled partial; copying it includes
only the captured text. Search numbered lines, press Enter to copy a preview line without closing
the preview (a confirmation appears in its footer), or choose
Back, Refresh, or Copy File Path. Search covers the full retained line. Display
excerpts stay capped at 1,024 characters and move near the earliest contiguous
query-word match; fuzzy-only matches retain the opening excerpt. Contiguous
query-word matches are highlighted in text and hex excerpts, using Unicode
lowercasing while preserving original displayed characters. Fuzzy-only matches
are not highlighted. Copying includes
the whole retained line. **Wrap excerpts** (Ctrl+Alt+W) shows up to four visual
lines per text row; toggle again for single-line excerpts. This changes only the
current preview display, preserves search/selection and leaves copying unchanged.

Refreshing the same file in the same preview mode restores its query, wrapping
and selected line number when that line still matches. If it no longer matches,
the first result is selected. Opening another file or switching text/hex modes
starts a fresh view.
Hex rows retain their fixed layout. Preview never executes files
or interprets escape sequences. UTF-8 (with or without a BOM) and BOM-marked
UTF-16 little/big endian are supported. BOMs are omitted from display and copy;
line separators are preserved. UTF-16 decoding can expand the captured text to
at most 96 KiB of UTF-8 before the line limit. Malformed text, NUL-containing
binary files and other encodings are rejected. Editing,
remote files, rich document rendering and a persistent explorer panel remain
unimplemented.


**Copy Preview Text** copies all captured text, regardless of the search filter,
preserving original CRLF/LF separators and any trailing newline. The preview
stays open. If either preview cap was reached, the action explicitly says it
copies captured text only; it does not reread the rest of the file.

If a directory or file cannot be read, the browser shows Back, Copy Path and
Retry actions. The Retry row includes the error; hover it for the full message.
Escape closes the browser and returns to its prior focus.
While a directory or preview is loading, its notice's close control cancels
remaining work. Closing the window also cancels it. Cancellation is checked
between directory entries and preview read chunks; an OS call already in
progress may finish first.
At most four directory/preview reads may run across all windows. A canceled read
keeps its slot until the worker exits; if all slots are occupied, rmux asks you
to retry rather than queuing more work.
