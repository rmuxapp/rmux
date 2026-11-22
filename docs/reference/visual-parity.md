# cmux visual and interaction target

Progress screenshots must use the real main app, per the user's preference.
Do not substitute a synthetic preview for evidence of current UI progress.

The optional `mise run preview-ui` fixture is retained for future regression work. It uses
temporary configuration and synthetic shell output, not the user's sessions.
Close the preview window to finish. Additional scenes after building:

```
mise exec -- python3 scripts/preview-workspace.py --scene overflow
mise exec -- python3 scripts/preview-workspace.py --scene compact
```

`--check` validates the generated layout without starting the GUI. The main scene
requests 1280×800 logical pixels with a 280px sidebar; compact requests 800×540.
Screen bounds may clamp these values. Capture the actual size and display scale
alongside screenshots. The scene labels all terminal output as synthetic and
does not imply real build, Git checkout, agent or network-service activity.

Reference inspected November 20, 2026: the official landing screenshot at
https://cmux.com/_next/static/immutable/media/landing-image.1y26igml71ekg.png
(3840 × 2224 source image), linked from https://cmux.com/.

Observed visual hierarchy: compact top controls and pane tabs; neutral dark
chrome; thin pane dividers; bright blue selected workspace; semibold workspace
titles; subdued description/directory metadata; dense sidebar rows with limited
rounding. The terminal theme in this screenshot is user-configurable, so its
olive terminal background is not a universal cmux chrome requirement.

First rmux pass: neutral charcoal chrome across sidebar, tabs, palette and
diagnostics; blue workspace selection with lighter secondary text; semibold
13px workspace names; 12px tab labels in 30px strips; reduced tab accent thickness;
thin visible sidebar separator retaining its resize hit area; header add button.
These are implementation targets, not pixel measurements or verified parity.

Remaining visual acceptance must compare rendered rmux with the reference at
matched logical size, font scale and content: single terminal, nested splits,
tab overflow, selected/hover/unread states, groups, palette and notification
panel. Check alignment, clipping, hit areas, keyboard focus, drag previews and
small-window behavior. Replace inconsistent text glyph controls with consistent
icons, align the titlebar/toolbar hierarchy, and implement missing metadata
before claiming the same information density. Browser surfaces remain a separate
functional gap; do not fake them for a screenshot.

The follow-up replaces text glyphs for add, close, disclosure, copy, refresh and
split controls with an embedded monochrome SVG set (16-unit viewbox, 1.25-unit
stroke, 14px display). Pane toolbar controls retain 28px hit areas. The workspace
header now exposes sidebar visibility and notifications alongside the workspace
name. These assets are built into the executable and require no icon font or
runtime file lookup. Rendered alignment, contrast and recognizability remain
acceptance work.

Native screenshot access was previously blocked by the locked Mac. Access resumed
after the v0.1.2 release: the installed main app was captured at 4096×2560 physical
pixels, with one workspace/tab, neutral pane chrome, blue sidebar selection and
directory/shortcut metadata. The local capture is retained at
`target/branding/native-v0.1.2-baseline.png`; it contains the user's actual session
context and is not committed. The Metal HUD was enabled and obscured the upper
right controls. This verifies the basic rendered hierarchy, not matched-size cmux
parity, compact layouts, interaction states, or the later unreleased toolbar/menu
changes. Do not treat historical access failures as a current blocker without
rechecking. Compilation alone does not establish polish.

Branch and path now have separate truncation budgets and a full-context tooltip.
Pin status uses a compact icon on the title row rather than consuming another
metadata line. Branch/pin icons use the same stroke language as toolbar controls.
Split dividers draw 1px lines with their existing 5px interaction areas. Workspace
creation lives in the compact header rather than a second footer button, leaving
more vertical room for the sidebar list. Terminal content retains configurable
8px default padding.
On macOS, native traffic lights now share the 36px app header. An 80px reserved
area follows sidebar visibility, and title labels handle native dragging and
double-click zoom. Validate traffic-light clearance, window move, fullscreen and
sidebar toggling before accepting this layout; other OS titlebars stay native.

Sidebar context follow-up: blue/white unread badges replace tiny amber dots, and
an unread notification preview occupies at most two lines beneath workspace
metadata. Group headers can show a hidden member's unread message. The preview
uses the newest unread message across panes and group members, ordered by the
application-wide arrival sequence. Existing shell paths and descriptions remain
secondary. Git branch context is now implemented via background HEAD discovery; PR and port rows remain missing. Native rendering of the branch/path combination is unverified.

November 21, 2026 follow-up: the combined native debug bundle at `1835fbf`
built and launched in an isolated app/config/socket. The Mac was locked, so no
visual or click/focus acceptance is claimed and no screenshot was captured.
The review process was stopped; the installed app was untouched. Pending review:
named shell-line actions in the palette, relative Markdown/text/directory link
activation by click and keyboard, and return focus after leaving a preview.
Focused local-link tests exercise actual preview/directory loaders, including
encoded spaces, uppercase Markdown extensions, file URLs and missing-file errors.

Browser review at `75b359d`: one combined debug WASM build passed. At 1280×720,
Option+Left followed by insertion changed `echo alpha beta` to
`echo alpha new beta`, and Enter produced `alpha new beta`. Cmd+Left/Right
moved to the draft edges; Backspace deleted at the new position. Escape closed
the palette and returned keyboard input to the terminal. Switching between
rmux/website retained separate session output. Two nested splits left independent
drafts editable; a long draft containing CJK and combining text scrolled within
the narrow pane while its caret remained visible. Local screenshots:
`target/demo-palette-current.png` and `target/demo-narrow-pane-editing.png`.
These are browser sample-session screenshots, not native acceptance evidence.
The temporary review tab/server were closed; the public demo was not deployed.

Native project-open integration at `82ce70e` (November 22, 2026, macOS): a
combined debug bundle ran with a separate bundle ID, config, socket and temporary
projects. `/usr/bin/open -a <review bundle> <folder>` delivered real folder-open
events. An existing `api` workspace launched in `client-a/api` was adopted;
opening `client-b/api` created a distinct workspace. After renaming the first
workspace and changing its shell directory, opening `client-a/api` selected its
original workspace and surface. The PTY PID was unchanged and a shell variable
set before reopening retained its value. Three workspaces remained (the initial
empty workspace and two projects), with no duplicate for the reopened project.
The process exited normally through `system.quit`; temporary fixtures were
removed. Local result: `target/native-project-check.json`. This verifies native
event routing, identity and process continuity, not cross-Space activation or
visual focus. The installed application was not used or replaced.


### Palette ordering and workspace context (2026-11-22, after v0.1.8)

The browser palette now orders supported commands as the native palette does,
starting with New Workspace and New Terminal Tab. Workspace results include
current-workspace, group, and description context when present; that context is
searchable. Native and browser palettes share 36/56 px result sizing, and browser
page navigation accounts for the taller contextual rows.

One combined local WASM build and desktop workspace check passed. Browser review
at 1280×720 confirmed initial ordering, current-workspace subtitle spacing, and
Down/Enter selecting the other workspace with terminal focus. Group/description
search was not separately exercised. Screenshots: `target/palette-order-review.png`
and `target/palette-workspaces-review.png`. These are browser screenshots, not
native acceptance evidence. Changes remain local for the next polish batch.


### Shared shell-line actions (2026-11-22, after v0.1.8)

Native and browser terminal hosts now share action dispatch for Move to Start of
Shell Line, Move to End of Shell Line, and Delete to Start of Shell Line. The
browser exposes these in the palette and Keyboard Shortcuts, using the same
macOS Cmd+Left/Right/Backspace bindings. Native input still passes through its
existing focus, composition, copy-mode, modal, and shutdown guards; browser input
requires terminal focus and no composition or find panel.

Desktop compilation and one combined WASM build passed. At 1280×720, browser
review invoked line-start from the palette, inserted `echo ` ahead of a draft,
used Cmd+Right to append ` done`, and observed `alpha beta done` as sample output.
Up recalled the command; Cmd+Backspace cleared it before `echo cleared` produced
`cleared`. Screenshot: `target/palette-shell-actions.png`. Native runtime review
of these shared handlers remains pending. No installed app/session was changed.


### Native workspace search context (2026-11-22, after v0.1.8)

Native palette workspace and tab-transfer destinations now include group and
workspace description alongside project identity, including destinations in
other windows. The selected workspace is explicitly marked Current workspace.
Search already indexes subtitles, so group and description become useful search
terms without per-frame work. Numeric shortcut hints resolve against the opening
focus and actual keymap; the last-workspace action is used as a fallback when
applicable, including lists longer than nine entries. They are also discoverable
in Keyboard Shortcuts.

`cargo check -p rmux-desktop` passed. Native review availability was checked and
reported that the Mac remains locked; no native build was performed solely for
an unavailable visual review. Appearance, remapped/disabled shortcut presentation,
and cross-window search interaction remain pending. Existing user sessions were
not touched.
