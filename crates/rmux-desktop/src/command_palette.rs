use super::*;

// Resolve the inherited row style at render time so headings retain their
// size/weight and only inline-code runs switch to the terminal's monospace font.
#[derive(IntoElement)]
struct PreviewStyledText {
    id: SharedString,
    links: Arc<[SharedString]>,
    text: String,
    styles: Vec<(std::ops::Range<usize>, markdown_preview::InlineStyle, bool)>,
}
impl PreviewStyledText {
    fn runs(&self, base: &TextStyle) -> Vec<TextRun> {
        let mut runs = Vec::with_capacity(self.styles.len() * 2 + 1);
        let mut offset = 0;
        for (range, style, matched) in &self.styles {
            if offset < range.start { runs.push(base.to_run(range.start - offset)); }
            let mut styled = base.clone().highlight(HighlightStyle {
                font_weight: style.bold.then_some(FontWeight::BOLD),
                font_style: style.italic.then_some(FontStyle::Italic),
                color: if *matched { Some(rgb(0xffffff).into()) } else { style.link.map(|_| rgb(0x8ebfff).into()) },
                underline: style.link.map(|_| UnderlineStyle { thickness: px(1.), color: None, wavy: false }),
                background_color: if *matched { Some(rgb(0x41638c).into()) } else { style.code.then(|| rgb(0x363b46).into()) },
                strikethrough: style.strike.then_some(StrikethroughStyle { thickness: px(1.), color: None }),
                ..Default::default()
            });
            if style.code { styled.font_family = FONT_FAMILY.into(); }
            runs.push(styled.to_run(range.len()));
            offset = range.end;
        }
        if offset < self.text.len() { runs.push(base.to_run(self.text.len() - offset)); }
        runs
    }
}
impl RenderOnce for PreviewStyledText {
    fn render(self, window: &mut Window, _: &mut App) -> impl IntoElement {
        let runs = self.runs(&window.text_style());
        let targets: Vec<_> = self.styles.iter().filter_map(|(range, style, _)|
            style.link.and_then(|index| self.links.get(index)).map(|url| (range.clone(), url.clone()))).collect();
        let styled = StyledText::new(self.text).with_runs(runs);
        if targets.is_empty() { return styled.into_any_element(); }
        let hints = targets.clone();
        InteractiveText::new(self.id, styled)
            .on_click(targets.iter().map(|(range, _)| range.clone()).collect(), move |index, _, cx| {
                if let Some((_, url)) = targets.get(index) && openable_uri(url) {
                    cx.stop_propagation();
                    cx.open_url(url);
                }
            })
            .tooltip(move |index, _, cx| hints.iter().find(|(range, _)| range.contains(&index))
                .map(|(_, url)| cx.new(|_| pane_view::ControlHint(url.clone().into())).into()))
            .into_any_element()
    }
}
#[cfg(test)]
mod preview_typography_tests {
    use super::{PreviewStyledText, markdown_preview, TextStyle, FontWeight, SharedString, FONT_FAMILY, rgb};
    #[test]
    fn inline_code_font_preserves_heading_and_search_attributes() {
        let text = "before 界 after";
        let start = text.find('界').unwrap();
        let element = PreviewStyledText {
            id: "test".into(), links: Default::default(),
            text: text.into(),
            styles: vec![(start..start + '界'.len_utf8(), markdown_preview::InlineStyle { code: true, strike: true, ..Default::default() }, true)],
        };
        let mut base = TextStyle::default();
        base.font_family = "UI font".into();
        base.font_weight = FontWeight::BOLD;
        let runs = element.runs(&base);
        assert_eq!(runs.len(), 3);
        assert_eq!(runs.iter().map(|run| run.len).sum::<usize>(), text.len());
        assert_eq!(runs[0].font.family, base.font_family);
        assert_eq!(runs[2].font.family, base.font_family);
        assert_eq!(runs[1].font.family, SharedString::from(FONT_FAMILY));
        assert_eq!(runs[1].font.weight, FontWeight::BOLD);
        assert!(runs[1].strikethrough.is_some());
        assert_eq!(runs[1].background_color, Some(rgb(0x41638c).into()));
    }
}

pub(super) use crate::palette_view::top_inset;

fn result_row_height(has_subtitle: bool) -> f32 {
    if has_subtitle { 56. } else { 36. }
}
fn literal_highlights(text: &str, query: &str) -> Vec<std::ops::Range<usize>> {
    if query.is_empty() { return Vec::new(); }
    // ASCII case folding preserves the original UTF-8 byte boundaries.
    text.to_ascii_lowercase().match_indices(&query.to_ascii_lowercase())
        .map(|(start, matched)| start..start + matched.len()).collect()
}
// Stop measuring once the visible panel is full, including for large previews.
fn result_area_height(rows: impl IntoIterator<Item = f32>) -> f32 {
    let mut height = 0.;
    for row_height in rows {
        height += row_height;
        if height >= 320. { return 320.; }
    }
    height
}

pub(super) use crate::palette_view::navigated_result;

pub(super) use crate::palette_view::paged_result;

#[cfg(test)]
mod navigation_tests {
    use super::{navigated_result, paged_result, result_area_height};
    #[test]
    fn folder_selection_identity_survives_order_changes() {
        use super::*;
        let make = |path: &str, directory: bool| Command { title: path.into(), subtitle: None, shortcut: None,
            target: PaletteEvent::Run(if directory { Box::new(BrowseDirectory { path: path.into() }) } else { Box::new(PreviewLocalFile { path: path.into() }) }) };
        let old = [make("/tmp/a", false), make("/tmp/b", false)];
        let selected = folder_target(&old[1]).unwrap();
        let changed = [make("/tmp/b", true), make("/tmp/new", false), make("/tmp/b", false)];
        assert_eq!(changed.iter().position(|entry| folder_target(entry) == Some(selected)), Some(2));
        assert!([make("/tmp/c", false)].iter().all(|entry| folder_target(entry) != Some(selected)));
    }
    #[test]
    fn literal_excerpt_highlights_preserve_unicode_boundaries() {
        let text = "…界 NEEDLE 世界 needle 世界 é É";
        let ranges = super::literal_highlights(text, "needle 世界");
        assert_eq!(ranges.iter().map(|range| &text[range.clone()]).collect::<Vec<_>>(), ["NEEDLE 世界", "needle 世界"]);
        assert_eq!(super::literal_highlights(text, "É").len(), 1);
        assert!(super::literal_highlights(text, "").is_empty());
        assert!(super::literal_highlights(text, "needle absent").is_empty());
    }
    #[test]
    fn preview_search_finds_text_beyond_display_cap_and_keeps_copy_source() {
        let text = format!("{}\tNeedle 世界{}", "İ界".repeat(800), "z".repeat(1400));
        let title = crate::file_browser::preview_line_title(&text, "");
        assert!(!title.contains("Needle"));
        let command = super::Command { title, subtitle: Some("Line 1".into()), shortcut: None,
            target: super::PaletteEvent::Run(Box::new(crate::CopyPreviewLine { text: text.clone(), markdown: None, spans: Vec::new(), links: Default::default() })) };
        let mut search = crate::palette_match::RankedSearch::new(vec![command.search_text()]);
        search.update("NEEDLE 世界");
        assert_eq!(search.matches, [0]);
        let excerpt = crate::file_browser::preview_line_title(command.preview_line().unwrap(), "NEEDLE 世界");
        assert!(excerpt.contains("Needle 世界"));
        assert!(excerpt.starts_with('…') && excerpt.ends_with('…'));
        assert!(excerpt.chars().count() <= 1026);
        assert!(!excerpt.contains('\t'));
        assert_eq!(command.preview_line(), Some(text.as_str()));
        assert_eq!(crate::file_browser::preview_line_title("", ""), "(empty line)");
        assert_eq!(crate::file_browser::preview_line_title("short", "missing"), "short");
    }
    #[test]
    fn paging_respects_visible_height_and_mixed_rows() {
        assert_eq!(paged_result(0, 20, false, 320., |_| 36.), Some(7));
        assert_eq!(paged_result(0, 20, false, 320., |_| 56.), Some(4));
        assert_eq!(paged_result(8, 20, true, 320., |_| 56.), Some(4));
        let heights = [36., 56., 36., 56.];
        assert_eq!(paged_result(0, 4, false, 128., |i| heights[i]), Some(2));
        assert_eq!(paged_result(2, 4, true, 128., |i| heights[i]), Some(0));
        assert_eq!(paged_result(1, 4, false, 20., |i| heights[i]), Some(2));
        assert_eq!(paged_result(3, 4, false, 320., |i| heights[i]), Some(3));
        assert_eq!(paged_result(99, 4, true, 20., |i| heights[i]), Some(2));
        assert_eq!(paged_result(0, 0, false, 320., |_| panic!("empty")), None);
    }
    #[test]
    fn palette_result_height_is_stable_and_bounded() {
        assert_eq!(result_area_height([]), 0.);
        assert_eq!(result_area_height([36.]), 36.);
        assert_eq!(result_area_height([36., 56.]), 92.);
        assert_eq!(result_area_height(std::iter::repeat(36.)), 320.);
        assert_eq!(result_area_height(std::iter::repeat(56.)), 320.);
        assert_eq!(result_area_height([84., 116., 36.]), 236.);
        let mut measured = 0;
        let rows = (0..100_000).map(|_| { measured += 1; 36. });
        assert_eq!(result_area_height(rows), 320.);
        assert_eq!(measured, 9, "redraw work must stop at the viewport boundary");
    }
    #[test]
    fn arrows_wrap_and_page_keys_clamp() {
        assert_eq!(navigated_result(0, 0, "down"), None);
        assert_eq!(navigated_result(0, 20, "up"), Some(19));
        assert_eq!(navigated_result(19, 20, "down"), Some(0));
        assert_eq!(navigated_result(0, 20, "pagedown"), Some(8));
        assert_eq!(navigated_result(17, 20, "pagedown"), Some(19));
        assert_eq!(navigated_result(3, 20, "pageup"), Some(0));
        assert_eq!(navigated_result(99, 2, "pageup"), Some(0));
        assert_eq!(navigated_result(0, 1, "pagedown"), Some(0));
        assert_eq!(navigated_result(0, 20, "home"), None);
    }
}

struct Command {
    title: String,
    subtitle: Option<String>,
    target: PaletteEvent,
    shortcut: Option<String>,
}
impl Command {
    fn markdown_kind(&self) -> Option<markdown_preview::Kind> {
        match &self.target { PaletteEvent::Run(action) => action.as_any().downcast_ref::<CopyPreviewLine>().and_then(|line| line.markdown), _ => None }
    }
    fn preview_line(&self) -> Option<&str> {
        match &self.target {
            PaletteEvent::Run(action) => action.as_any().downcast_ref::<CopyPreviewLine>().map(|line| line.text.as_str()),
            _ => None,
        }
    }
    fn shortcut_text(&self) -> String { self.shortcut.as_deref().map(shortcuts::search_terms).unwrap_or_default() }
    fn search_text(&self) -> String {
        format!("{} {}", self.preview_line().filter(|text| !text.is_empty()).unwrap_or(&self.title), self.subtitle.as_deref().unwrap_or_default()).to_lowercase()
    }
}
pub enum PaletteEvent {
    Run(Box<dyn Action>),
    SelectWorkspace(EntityId),
    SelectWindowWorkspace(AnyWindowHandle, EntityId),
    SelectTerminal(EntityId, EntityId),
    SearchTerminal { window: Option<AnyWindowHandle>, workspace: EntityId, terminal: EntityId, query: String, preview: bool },
    SelectWindowTerminal(AnyWindowHandle, EntityId, EntityId),
    MoveTabToWorkspace(EntityId),
    MoveTabToWindowWorkspace(AnyWindowHandle, EntityId),
    MoveWorkspaceToWindow(EntityId, AnyWindowHandle),
    MergeWorkspaces(EntityId, EntityId, AnyWindowHandle),
    Close,
}
#[derive(Clone, Copy, PartialEq)]
pub enum BufferMode { Paste, Delete, Rename, Copy }

#[derive(Default)]
pub struct PaletteContext {
    pub workspace: bool,
    pub tab: bool,
    pub split: bool,
    pub paste: bool,
    pub file_loading: bool,
    pub restore: bool,
    pub active_group: Option<u64>,
    pub selected_workspaces: usize,
}

pub struct FolderState {
    path: std::path::PathBuf,
    query: String,
    target: Option<(std::path::PathBuf, u8)>,
}
impl FolderState {
    pub fn matches(&self, path: &std::path::Path) -> bool { self.path == path }
}
fn folder_target(command: &Command) -> Option<(&std::path::Path, u8)> {
    let PaletteEvent::Run(action) = &command.target else { return None; };
    if let Some(action) = action.as_any().downcast_ref::<BrowseDirectory>() { Some((action.path.as_path(), 0)) }
    else if let Some(action) = action.as_any().downcast_ref::<PreviewLocalFile>() { Some((action.path.as_path(), 1)) }
    else { action.as_any().downcast_ref::<CopyLocalPath>().map(|action| (action.path.as_path(), 2)) }
}

pub struct PreviewState {
    path: std::path::PathBuf,
    hex: bool,
    markdown: bool,
    query: String,
    wrapped: bool,
    command: Option<usize>,
}
impl PreviewState {
    pub fn matches(&self, path: &std::path::Path, hex: bool, markdown: bool) -> bool { self.path == path && self.hex == hex && self.markdown == markdown }
}

struct CodeViewport {
    query: Option<String>,
    scroll: ScrollHandle,
    lines: Vec<(SharedString, Vec<std::ops::Range<usize>>)>,
}
impl CodeViewport {
    fn new(source: &str) -> Self {
        Self { query: None, scroll: ScrollHandle::new(), lines: markdown_preview::code_lines(source)
            .into_iter().map(|line| (line.into(), Vec::new())).collect() }
    }
    fn search(&mut self, source: &str, query: &str) {
        if self.query.as_deref() == Some(query) { return; }
        self.query = Some(query.to_owned());
        for (line, ranges) in &mut self.lines {
            *ranges = file_browser::preview_highlights(line, query);
        }
        self.scroll.scroll_to_top_of_item(markdown_preview::code_match_line(source, query));
    }
}

pub struct CommandPalette {
    location: Option<SharedString>,
    feedback: Option<&'static str>,
    preview_link: Option<(usize, usize)>,
    code_scroll: std::collections::HashMap<usize, CodeViewport>,
    buffer_rename: Option<(String, u64)>,
    directory_input: bool,
    terminal_search_input: bool,
    ssh_input: bool,
    shortcuts_only: bool,
    hex_preview: bool,
    markdown_preview: bool,
    wrap_preview: bool,
    folder_navigation: Option<(bool, bool, std::path::PathBuf)>,
    preview_navigation: Option<(std::path::PathBuf, bool)>,
    input: Entity<text_field::TextField>,
    heading: &'static str,
    empty_message: &'static str,
    commands: Vec<Command>,
    search: palette_match::RankedSearch,
    selected: usize,
    scroll: ListState,
}
impl EventEmitter<PaletteEvent> for CommandPalette {}
impl CommandPalette {
    pub fn new(
        workspaces: Vec<(EntityId, String)>,
        terminals: Vec<(Option<AnyWindowHandle>, EntityId, EntityId, String, Option<String>)>,
        windows: Vec<(AnyWindowHandle, String)>,
        external_workspaces: Vec<(AnyWindowHandle, EntityId, String)>,
        groups: Vec<(u64, String)>,
        active_workspace: EntityId,
        pending: PaletteContext,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let origin_focus = window.focused(cx);
        let mut commands = Vec::new();
        macro_rules! command {
            ($action:ident) => {
                if crate::workspace_actions::palette_available(rmux_core::commands::CommandId::$action,
                    pending.active_group.is_some(), pending.selected_workspaces) { commands.push(Command {
                    title: rmux_core::commands::CommandId::$action.title().into(),
                    target: PaletteEvent::Run(crate::pane_actions::action(rmux_core::commands::CommandId::$action)
                        .or_else(|| crate::workspace_actions::action(rmux_core::commands::CommandId::$action))
                        .or_else(|| crate::tab_actions::action(rmux_core::commands::CommandId::$action))
                        .or_else(|| crate::find_presentation::action(rmux_core::commands::CommandId::$action))
                        .or_else(|| crate::terminal_actions::action(rmux_core::commands::CommandId::$action))
                        .unwrap_or_else(|| Box::new($action))),
                    shortcut: None,
                subtitle: None,
                }); }
            };
        }
        command!(NewWorkspace);
        command!(NewTab);
        command!(CopySelection);
        command!(Paste);
        command!(NextPane);
        command!(PreviousPane);
        command!(MoveTabLeft);
        command!(MoveTabRight);
        command!(Split);
        command!(SplitBelow);
        command!(TogglePaneZoom);
        command!(Find);
        command!(ShowSshConnection);
        command!(ToggleSettings);
        command!(ShowKeyboardShortcuts);
        command!(OpenUserGuide);
        command!(ReportIssue);
        command!(OpenWebsite);
        if pending.workspace { command!(CancelNewWorkspace); }
        command!(RefreshTerminalMemory);
        command!(CycleTerminalMemorySort);
        command!(NewWorkspaceGroup);
        command!(GroupCurrentWorkspace);
        command!(GroupSelectedWorkspaces);
        command!(UseCurrentDirectoryForGroup);
        command!(ResetGroupDirectory);
        command!(RenameWorkspaceGroup);
        command!(EditGroupColor);
        command!(EditGroupIcon);
        command!(UngroupWorkspaces);
        command!(CloseWorkspaceGroup);
        command!(ToggleGroupPin);
        command!(RemoveWorkspaceFromGroup);
        command!(SetGroupAnchor);
        for (group, name) in groups {
            if Some(group) == pending.active_group { continue; }
            commands.push(Command {
                title: format!("Move Workspace to Group: {name} (#{group})"),
                target: PaletteEvent::Run(Box::new(MoveWorkspaceToGroup { group })),
                shortcut: None,
                subtitle: None,
            });
        }
        command!(NewLocalTab);
        if pending.tab { command!(CancelNewTab); }
        if pending.paste { command!(CancelQueuedPaste); }
        if pending.split { command!(CancelSplit); }
        command!(NewWindow);
        command!(SplitLeft);
        command!(SplitAbove);
        command!(EqualizePanes);
        command!(ResizePaneLeft);
        command!(ResizePaneRight);
        command!(ResizePaneUp);
        command!(ResizePaneDown);
        command!(SwapPaneNext);
        command!(SwapPaneLeft);
        command!(SwapPaneRight);
        command!(SwapPaneUp);
        command!(SwapPaneDown);
        command!(BreakTabRight);
        command!(BreakTabBelow);
        command!(BreakTabLeft);
        command!(BreakTabAbove);
        command!(JoinPaneNext);
        command!(JoinPaneLeft);
        command!(JoinPaneRight);
        command!(JoinPaneUp);
        command!(JoinPaneDown);
        command!(MoveTabNextWorkspace);
        command!(DetachTab);
        command!(DetachWorkspace);
        command!(ShowWorkspaceMergePicker);
        command!(ShowWorkspaceWindowPicker);
        command!(MoveWorkspaceToNextWindow);
        #[cfg(all(target_os = "macos", feature = "browser"))]
        command!(NewBrowserWindow);
        command!(ToggleSidebar);
        command!(RenameWorkspace);
        command!(EditWorkspaceIcon);
        command!(EditWorkspaceColor);
        command!(EditWorkspaceDescription);
        command!(NextWorkspace);
        command!(LastWorkspace);
        command!(LastTerminal);
        command!(FocusBack);
        command!(FocusForward);
        command!(RenameTab);
        command!(NextTab);
        command!(ShowTabPicker);
        command!(ShowTabMenu);
        command!(ShowWorkspaceMenu);
        command!(PreviousTab);
        command!(ToggleWorkspacePin);
        command!(PreviousWorkspace);
        command!(NextUnreadTerminal);
        command!(MarkAllRead);
        command!(ToggleNotifications);
        command!(DismissAllNotifications);
        command!(RefreshGitContext);
        command!(ToggleProgramStatus);
        command!(ShowListeningPorts);
        command!(CloseListeningPorts);
        command!(FindOlder);
        command!(FindNewer);
        command!(EditorSelectAll);
        command!(ShowTerminalMenu);
        command!(CopySelectionWithLineBreaks);
        command!(CopyWorkingDirectory);
        command!(RevealWorkingDirectory);
        command!(ClearScrollback);
        command!(PreviousPrompt);
        command!(NextPrompt);
        command!(LastPrompt);
        command!(MoveToLineStart);
        command!(MoveToLineEnd);
        command!(DeleteToLineStart);
        command!(ScrollToTop);
        command!(ScrollToBottom);
        command!(IncreaseFontSize);
        command!(DecreaseFontSize);
        command!(ResetFontSize);
        command!(EnterCopyMode);
        command!(ShowBufferPicker);
        command!(ShowDeleteBufferPicker);
        command!(ShowRenameBufferPicker);
        command!(ShowCopyBufferPicker);
        command!(BrowseTerminalFiles);
        command!(OpenTextPreview);
        if pending.file_loading { command!(CancelFileLoading); }
        command!(ShowTerminalSearch);
        command!(ShowSshHosts);
        command!(OpenFolderBrowser);
        command!(ShowFontPicker);
        command!(ShowDefaultFontPicker);
        command!(UseConfiguredFont);
        command!(SaveLayout);
        command!(SaveSession);
        command!(OpenSession);
        command!(ShowLayoutPresetPicker);
        command!(ShowCustomCommandPicker);
        if pending.restore { command!(CancelRestore); }
        command!(SaveAppearanceSettings);
        command!(ReloadSettings);
        command!(RevertAppearanceSettings);
        if cx.global::<preferences::LastSettingsSave>().0.is_some() { command!(UndoSettingsSave); }
        command!(OpenSettingsFile);
        command!(OpenLayout);
        command!(FocusLeft);
        command!(FocusRight);
        command!(FocusUp);
        command!(FocusDown);
        command!(MoveWorkspaceUp);
        command!(MoveWorkspaceDown);
        command!(ClosePane);
        command!(CloseOtherTabs);
        command!(CloseTabsToRight);
        command!(CloseFinishedTabs);
        command!(CloseWorkspace);
        command!(CloseWindow);
        command!(ImportTerminalTheme);
        for theme in preferences::TerminalTheme::ALL {
            commands.push(Command {
                title: format!("Terminal Theme: {}", theme.label()),
                target: PaletteEvent::Run(Box::new(SetTerminalTheme { theme })),
                shortcut: None,
                subtitle: Some("Applies to this session; explicit color overrides stay active".into()),
            });
        }
        for (name, icon) in WORKSPACE_ICONS {
            commands.push(Command {
                title: format!("Workspace Icon: {name}"),
                target: PaletteEvent::Run(Box::new(SetWorkspaceIcon {
                    icon: icon.map(str::to_owned),
                })),
                shortcut: None,
                subtitle: None,
            });
        }
        for (name, color) in WORKSPACE_COLORS {
            commands.push(Command {
                title: format!("Workspace Color: {name}"),
                target: PaletteEvent::Run(Box::new(SetWorkspaceColor { color: *color })),
                shortcut: None,
                subtitle: None,
            });
        }
        for number in 1..=9 {
            commands.push(Command {
                title: format!("Select Tab {number}"),
                target: PaletteEvent::Run(Box::new(SelectTabNumber { number })),
                shortcut: None,
                subtitle: None,
            });
            commands.push(Command {
                title: format!("Focus Pane {number}"),
                target: PaletteEvent::Run(Box::new(SelectPaneNumber { number })),
                shortcut: None,
                subtitle: None,
            });
        }
        commands.extend(cx.global::<Preferences>().commands.iter().map(custom_command));
        commands.extend(cx.global::<Preferences>().layout_presets.iter().map(preset_command));
        if let Some(focus) = origin_focus {
            for command in &mut commands {
                let PaletteEvent::Run(action) = &command.target else {
                    continue;
                };
                command.shortcut = shortcuts::hint(action.as_ref(), &focus, window);
            }
        }
        for (index, (id, name)) in workspaces.into_iter().enumerate() {
            commands.push(Command {
                title: format!("Switch to Workspace {}: {name}", index + 1),
                target: PaletteEvent::SelectWorkspace(id),
                shortcut: None,
                subtitle: None,
            });
            if id != active_workspace {
                commands.push(Command {
                    title: format!("Move Tab to Workspace {}: {name}", index + 1),
                    target: PaletteEvent::MoveTabToWorkspace(id),
                    shortcut: None,
                subtitle: None,
                });
            }
        }
        for (destination, workspace, title) in external_workspaces {
            commands.push(Command {
                title: format!("Switch to {title}"),
                target: PaletteEvent::SelectWindowWorkspace(destination, workspace),
                shortcut: None,
                subtitle: None,
            });
            commands.push(Command {
                title: format!("Move Tab to {title}"),
                target: PaletteEvent::MoveTabToWindowWorkspace(destination, workspace),
                shortcut: None,
                subtitle: None,
            });
        }
        for (destination, title) in windows {
            commands.push(Command {
                title: format!("Move Workspace to {title}"),
                target: PaletteEvent::MoveWorkspaceToWindow(active_workspace, destination),
                shortcut: None,
                subtitle: None,
            });
        }
        for (destination, workspace, terminal, title, directory) in terminals {
            commands.push(Command {
                title: format!("Switch to Terminal: {title}"),
                target: match destination { Some(window) => PaletteEvent::SelectWindowTerminal(window, workspace, terminal), None => PaletteEvent::SelectTerminal(workspace, terminal) },
                shortcut: None,
                subtitle: directory,
            });
        }
        let palette = Self::with_commands(commands, "Search commands and workspaces", "No matching commands or workspaces", window, cx);
        palette.input.update(cx, |input, _| input.placeholder = Some("Type a command or workspace name…"));
        palette
    }
    pub fn show_shortcuts(&mut self, focus: Option<&FocusHandle>, window: &Window, cx: &mut Context<Self>) {
        if let Some(focus) = focus {
            for command in &mut self.commands {
                if let PaletteEvent::Run(action) = &command.target {
                    command.shortcut = crate::shortcut_label::binding_aliases(action.as_ref(), focus, window, cfg!(target_os = "macos"));
                }
            }
        }
        self.shortcuts_only = true;
        self.commands.retain(|command| command.shortcut.is_some());
        self.commands.sort_by_cached_key(|command| command.title.to_lowercase());
        self.search = palette_match::RankedSearch::with_shortcuts(self.commands.iter().map(Command::search_text).collect(), self.commands.iter().map(Command::shortcut_text).collect());
        self.selected = 0;
        self.scroll.reset(self.commands.len());
        self.heading = "Keyboard shortcuts · Current context";
        self.empty_message = "No matching bound commands";
        self.input.update(cx, |input, cx| {
            input.placeholder = Some("Search commands or keys, e.g. cmd shift…");
            cx.notify();
        });
        cx.notify();
    }
    pub fn buffers(entries: Vec<(String, usize, String)>, revision: u64, surface_id: String, mode: BufferMode, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let commands = entries.into_iter().map(|(name, bytes, preview)| Command {
            title: name.clone(), subtitle: Some(format!("{bytes} bytes · {preview}")), shortcut: None,
            target: PaletteEvent::Run(if mode == BufferMode::Rename { Box::new(RenameNamedBuffer { name, revision }) }
                else if mode == BufferMode::Delete { Box::new(DeleteNamedBuffer { name, revision }) }
                else if mode == BufferMode::Copy { Box::new(CopyNamedBuffer { name, revision }) }
                else { Box::new(PasteNamedBuffer { name, revision, surface_id: surface_id.clone() }) }),
        }).collect();
        Self::with_commands(commands, match mode { BufferMode::Paste => "Paste named buffer", BufferMode::Delete => "Delete named buffer", BufferMode::Rename => "Rename named buffer", BufferMode::Copy => "Copy buffer to clipboard" }, "No matching buffers · Save a selection in Copy Mode first", window, cx)
    }
    pub fn files(listing: file_browser::Listing, back: bool, forward: bool, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let location: SharedString = if listing.unavailable > 0 {
            format!("{} · {} unavailable entries · Refresh to retry", listing.path.display(), listing.unavailable).into()
        } else { listing.path.display().to_string().into() };
        let navigation = (back, forward, listing.path.clone());
        let mut commands = Vec::new();
        if back { commands.push(Command { title: "Back · Previous folder".into(), subtitle: None, shortcut: None,
            target: PaletteEvent::Run(Box::new(FileBrowserBack)) }); }
        if forward { commands.push(Command { title: "Forward · Next folder".into(), subtitle: None, shortcut: None,
            target: PaletteEvent::Run(Box::new(FileBrowserForward)) }); }

        commands.push(Command { title: "Open Folder…".into(), subtitle: Some("Choose a local folder".into()), shortcut: None,
            target: PaletteEvent::Run(Box::new(OpenFolderBrowser)) });
        commands.push(Command { title: "Go to Directory…".into(), subtitle: Some(listing.path.display().to_string()), shortcut: None,
            target: PaletteEvent::Run(Box::new(EnterDirectory { path: listing.path.clone() })) });
        commands.push(Command { title: "New Workspace Here".into(), subtitle: Some(listing.path.display().to_string()), shortcut: None,
            target: PaletteEvent::Run(Box::new(NewWorkspaceInDirectory { path: listing.path.clone() })) });
        if let Some(parent) = listing.path.parent() {
            commands.push(Command { title: ".. · Parent directory".into(), subtitle: Some(parent.display().to_string()), shortcut: None,
                target: PaletteEvent::Run(Box::new(BrowseDirectory { path: parent.to_owned() })) });
        }
        commands.push(reveal_local_path(&listing.path));
        commands.push(Command { title: "Copy directory path".into(), subtitle: Some(listing.path.display().to_string()), shortcut: None,
            target: PaletteEvent::Run(Box::new(CopyLocalPath { path: listing.path.clone() })) });
        commands.push(Command { title: "Refresh this directory".into(), subtitle: Some(listing.path.display().to_string()), shortcut: None,
            target: PaletteEvent::Run(Box::new(BrowseDirectory { path: listing.path })) });
        commands.extend(listing.entries.into_iter().map(|entry| Command {
            title: entry.path.file_name().unwrap_or_default().to_string_lossy().chars()
                .map(|ch| if ch.is_control() { ' ' } else { ch }).collect(),
            subtitle: Some(if entry.directory { "Directory · Enter to browse" } else if entry.symlink { "Symbolic link · Enter to copy path" } else { "File · Enter for text · Shift+Enter for hex" }.into()),
            shortcut: None,
            target: PaletteEvent::Run(if entry.directory { Box::new(BrowseDirectory { path: entry.path }) }
                else if entry.symlink { Box::new(CopyLocalPath { path: entry.path }) }
                else { Box::new(PreviewLocalFile { path: entry.path }) }),
        }));
        let mut palette = Self::with_commands(commands, if listing.truncated { "Local files · Limited to 2,048 entries; use a narrower directory" } else { "Local files · Browse directories or preview text" },
            "No matching entries", window, cx);
        palette.location = Some(location);
        palette.folder_navigation = Some(navigation);
        palette
    }
    pub fn file_error(path: std::path::PathBuf, error: String, directory: bool, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let location: SharedString = path.display().to_string().into();
        let mut commands = Vec::new();
        commands.push(Command { title: "Open Folder…".into(), subtitle: Some("Choose a different local folder".into()), shortcut: None,
            target: PaletteEvent::Run(Box::new(OpenFolderBrowser)) });
        commands.push(Command { title: "Go to Directory…".into(), subtitle: None, shortcut: None,
            target: PaletteEvent::Run(Box::new(EnterDirectory { path: path.parent().unwrap_or(&path).to_owned() })) });
        if let Some(parent) = path.parent() {
            commands.push(Command { title: "Back to directory".into(), subtitle: Some(parent.display().to_string()), shortcut: None,
                target: PaletteEvent::Run(Box::new(BrowseDirectory { path: parent.to_owned() })) });
        }
        commands.push(Command { title: "Copy path".into(), subtitle: Some(path.display().to_string()), shortcut: None,
            target: PaletteEvent::Run(Box::new(CopyLocalPath { path: path.clone() })) });
        commands.push(reveal_local_path(&path));
        if !directory { commands.push(Command { title: "Preview as Hex".into(), subtitle: Some("Read-only bytes · First 8 KiB".into()), shortcut: None,
            target: PaletteEvent::Run(Box::new(PreviewHexFile { path: path.clone() })) }); }
        commands.push(Command { title: "Retry".into(), subtitle: Some(error), shortcut: None,
            target: PaletteEvent::Run(if directory { Box::new(BrowseDirectory { path }) } else { Box::new(PreviewLocalFile { path }) }) });
        let mut palette = Self::with_commands(commands, if directory { "Could not read directory" } else { "Could not preview file" }, "Clear the search to see recovery actions", window, cx);
        palette.location = Some(location);
        palette
    }
    pub fn file_preview(preview: file_browser::Preview, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let location: SharedString = preview.path.display().to_string().into();
        let navigation = (preview.path.clone(), preview.hex);
        let empty = preview.lines.is_empty();
        let mut commands = Vec::new();
        if let Some(parent) = preview.path.parent() {
            commands.push(Command { title: "Back to directory".into(), subtitle: Some(parent.display().to_string()), shortcut: None,
                target: PaletteEvent::Run(Box::new(BrowseDirectory { path: parent.to_owned() })) });
        }
        commands.push(Command { title: "Copy file path".into(), subtitle: Some(preview.path.display().to_string()), shortcut: None,
            target: PaletteEvent::Run(Box::new(CopyLocalPath { path: preview.path.clone() })) });
        commands.push(reveal_local_path(&preview.path));
        commands.push(Command { title: "Read Markdown".into(), subtitle: Some("Bounded reading view · No active content".into()), shortcut: None,
            target: PaletteEvent::Run(Box::new(PreviewMarkdownFile { path: preview.path.clone() })) });
        commands.push(Command { title: "Open in External Editor".into(), subtitle: Some("Uses file_editor from Settings".into()), shortcut: None,
            target: PaletteEvent::Run(Box::new(EditLocalFile { path: preview.path.clone(), ..Default::default() })) });
        commands.push(Command { title: if preview.hex { "Copy Hex Dump" } else { "Copy Preview Text" }.into(),
            subtitle: Some(if preview.truncated { "Copies captured text only · Preview is incomplete" } else if preview.hex { "Copies the formatted hex dump, not raw bytes" } else { "Preserves original line endings" }.into()), shortcut: None,
            target: PaletteEvent::Run(Box::new(CopyPreviewText { text: preview.contents })) });
        commands.push(Command { title: if preview.hex { "Preview as Text" } else { "Preview as Hex" }.into(), subtitle: None, shortcut: None,
            target: PaletteEvent::Run(if preview.hex { Box::new(PreviewLocalFile { path: preview.path.clone() }) } else { Box::new(PreviewHexFile { path: preview.path.clone() }) }) });
        commands.push(Command { title: "Refresh preview".into(), subtitle: empty.then(|| "Empty file · No text to preview".into()), shortcut: None,
            target: PaletteEvent::Run(if preview.hex { Box::new(PreviewHexFile { path: preview.path }) } else { Box::new(PreviewLocalFile { path: preview.path }) }) });
        commands.extend(preview.lines.into_iter().enumerate().map(|(index, text)| {
            let title = file_browser::preview_line_title(&text, "");
            let copy_hint = if preview.partial_line == Some(index) { "Partial line · Enter copies only the captured text" } else { "Enter copies this preview line" };
            Command { title, subtitle: Some(format!("Line {} · {copy_hint}", index + 1)), shortcut: None,
                target: PaletteEvent::Run(Box::new(CopyPreviewLine { text, markdown: None, spans: Vec::new(), links: Default::default() })) }
        }));
        let mut palette = Self::with_commands(commands, if preview.hex && preview.truncated { "Hex preview · First 8 KiB / 1,024 rows" } else if preview.hex { "Hex preview · Search bytes or ASCII · Read only" } else if preview.truncated { "Text preview · Limited to 64 KiB / 512 lines" } else if empty { "Text preview · Empty file" } else { "Text preview · Search lines · Read only" }, "No matching preview lines", window, cx);
        palette.hex_preview = preview.hex;
        palette.preview_navigation = Some(navigation);
        palette.location = Some(location);
        palette
    }
    pub fn markdown_file(mut preview: file_browser::Preview, blocks: Vec<markdown_preview::Block>, limited: bool, window: &mut Window, cx: &mut Context<Self>) -> Self {
        preview.lines = blocks.iter().map(|block| block.text.clone()).collect();
        preview.partial_line = None;
        preview.truncated |= limited;
        let truncated = preview.truncated;
        let mut palette = Self::file_preview(preview, window, cx);
        let mut blocks = blocks.into_iter();
        let mut block_number = 0;
        for command in &mut palette.commands {
            if let PaletteEvent::Run(action) = &command.target {
                if let Some(line) = action.as_any().downcast_ref::<CopyPreviewLine>() {
                    block_number += 1;
                    let block = blocks.next().expect("one reading block per line");
                    command.target = PaletteEvent::Run(Box::new(CopyPreviewLine { text: line.text.clone(), markdown: Some(block.kind), spans: block.spans, links: block.links.into_iter().map(SharedString::from).collect() }));
                    let label = match command.markdown_kind() {
                        Some(markdown_preview::Kind::Heading(level)) => format!("Heading H{level}"),
                        Some(markdown_preview::Kind::Code) => "Code · Scroll to read".into(),
                        Some(markdown_preview::Kind::Table { header, columns, .. }) => format!("Table {} · {columns} columns{}", if header { "header" } else { "row" }, if columns > 8 { " · first 8 shown; copy for all" } else { "" }),
                        Some(markdown_preview::Kind::List { checked: Some(true), .. }) => "Completed task".into(),
                        Some(markdown_preview::Kind::List { checked: Some(false), .. }) => "Pending task".into(),
                        _ => "Block".into(),
                    };
                    command.subtitle = Some(format!("{label} {block_number} · Enter copies reading text"));
                } else if let Some(action) = action.as_any().downcast_ref::<PreviewMarkdownFile>() {
                    command.title = "View Markdown Source".into();
                    command.subtitle = Some("Read-only text preview".into());
                    command.target = PaletteEvent::Run(Box::new(PreviewLocalFile { path: action.path.clone() }));
                } else if let Some(action) = action.as_any().downcast_ref::<PreviewLocalFile>() {
                    command.target = PaletteEvent::Run(Box::new(PreviewMarkdownFile { path: action.path.clone() }));
                } else if action.as_any().is::<CopyPreviewText>() {
                    command.title = "Copy Markdown Source".into();
                }
            }
        }
        palette.search = palette_match::RankedSearch::with_shortcuts(palette.commands.iter().map(Command::search_text).collect(), palette.commands.iter().map(Command::shortcut_text).collect());
        palette.markdown_preview = true;
        palette.wrap_preview = true;
        palette.heading = if truncated { "Markdown reading view · Partial document · Bounded source/blocks" } else { "Markdown reading view · Images and links are inert" };
        palette
    }
    pub fn terminal_search(query: &str, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut palette = Self::with_commands(Vec::new(), "Search terminal output · All windows", "Enter submits a literal search · Retained history · Up to 256 terminals", window, cx);
        palette.terminal_search_input = true;
        palette.input.update(cx, |input, cx| {
            input.replace_draft(query, cx);
            input.select_all_text(cx);
        });
        palette
    }
    pub fn ssh_connection(destination: &str, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut palette = Self::with_commands(Vec::new(), "Connect via SSH", ssh::INPUT_HINT, window, cx);
        palette.ssh_input = true;
        palette.input.update(cx, |input, cx| { input.replace_draft(destination, cx); input.select_all_text(cx); });
        palette
    }
    pub fn ssh_hosts(result: anyhow::Result<ssh::Aliases>, session_targets: Vec<ssh::HostTarget>, truncated: bool, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut commands = vec![Command { title: "Enter SSH destination…".into(), subtitle: None, shortcut: None,
            target: PaletteEvent::Run(Box::new(ShowSshConnection)) }, Command { title: "Refresh SSH hosts".into(), subtitle: None, shortcut: None,
            target: PaletteEvent::Run(Box::new(ShowSshHosts)) }];
        let (aliases, mut summary) = match result {
            Ok(aliases) => {
                let summary = format!("SSH config/includes{} · Targets from this window", if aliases.limited { " · Discovery incomplete" } else { "" });
                (aliases.names, summary)
            }
            Err(error) => (Vec::new(), format!("Config unavailable: {error} · Targets from this window")),
        };
        if truncated { summary.push_str(" · First 256 terminals"); }
        commands.extend(ssh::host_targets(session_targets, &aliases).into_iter().map(|target| Command {
            title: target.destination.clone(),
            subtitle: Some(match target.source {
                ssh::TargetSource::Session => "Session target · Enter opens a new SSH workspace",
                ssh::TargetSource::Saved => "Saved target · Not connected · Enter opens a new SSH workspace",
                ssh::TargetSource::Config => "Config alias · Enter opens a new SSH workspace",
            }.into()), shortcut: None,
            target: PaletteEvent::Run(Box::new(ConnectSsh { destination: target.destination })),
        }));
        let mut palette = Self::with_commands(commands, "SSH hosts · Choose a destination", "No matching hosts · Clear search to enter a destination", window, cx);
        palette.location = Some(summary.into());
        if palette.commands.len() > 2 { palette.selected = 2; }
        palette
    }
    pub fn memory_sort(current: memory_view::Sort, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let commands = memory_view::Sort::ALL.into_iter().map(|sort| Command {
            title: sort.label().into(), subtitle: (sort == current).then(|| "Current sort order".into()), shortcut: None,
            target: PaletteEvent::Run(Box::new(SetTerminalMemorySort { sort })),
        }).collect();
        let mut palette = Self::with_commands(commands, "Sort terminal memory · Highest first", "No matching sort modes", window, cx);
        palette.selected = memory_view::Sort::ALL.iter().position(|sort| *sort == current).unwrap_or(0);
        palette
    }
    pub fn terminal_search_results(report: terminal_search::Report, query: String, extra: usize, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let summary = format!("{} matching terminals · {} scanned · {} unavailable · {} incomplete · {extra} beyond terminal limit", report.hits.len(), report.scanned, report.unavailable, report.limited);
        let mut commands = vec![Command { title: "Search again".into(), subtitle: Some(query.clone()), shortcut: None,
            target: PaletteEvent::Run(Box::new(SearchTerminalOutput { query: query.clone() })) },
            Command { title: "New search…".into(), subtitle: None, shortcut: None, target: PaletteEvent::Run(Box::new(ShowTerminalSearch)) }];
        commands.extend(report.hits.into_iter().map(|hit| Command { title: hit.name, subtitle: Some(hit.snippet), shortcut: None,
            target: PaletteEvent::SearchTerminal { window: hit.window, workspace: hit.workspace, terminal: hit.terminal, query: query.clone(), preview: hit.preview } }));
        let mut palette = Self::with_commands(commands, "Terminal output matches · Enter opens Find", "No matching results · Clear the filter to search again", window, cx);
        palette.location = Some(summary.into());
        if palette.commands.len() > 2 { palette.selected = 2; }
        palette
    }
    pub fn custom_commands(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut commands: Vec<_> = cx.global::<Preferences>().commands.iter().map(custom_command).collect();
        let empty = if commands.is_empty() { "No commands · Add commands in Settings, then Reload Settings" } else { "No matching configured commands" };
        if let Some(focus) = window.focused(cx) {
            for command in &mut commands {
                if let PaletteEvent::Run(action) = &command.target { command.shortcut = shortcuts::hint(action.as_ref(), &focus, window); }
            }
        }
        Self::with_commands(commands, "Run configured command · Opens a new workspace", empty, window, cx)
    }
    pub fn layout_presets(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut commands: Vec<_> = cx.global::<Preferences>().layout_presets.iter().map(preset_command).collect();
        let empty = if commands.is_empty() { "No presets · Add layout_presets in Settings, then Reload Settings" } else { "No matching presets" };
        if let Some(focus) = window.focused(cx) {
            for command in &mut commands {
                if let PaletteEvent::Run(action) = &command.target { command.shortcut = shortcuts::hint(action.as_ref(), &focus, window); }
            }
        }
        Self::with_commands(commands, "Open layout preset · Fresh shells in new windows", empty, window, cx)
    }
    pub fn workspace_merge(entries: Vec<(AnyWindowHandle, EntityId, String)>, source: EntityId, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let empty = if entries.is_empty() { "No other workspaces available" } else { "No matching workspaces" };
        let commands = entries.into_iter().map(|(target, destination, title)| Command {
            title, subtitle: Some("Keep destination metadata · Add source splits on the right".into()), shortcut: None,
            target: PaletteEvent::MergeWorkspaces(source, destination, target),
        }).collect();
        Self::with_commands(commands, "Merge workspace into…", empty, window, cx)
    }
    pub fn workspace_windows(entries: Vec<(AnyWindowHandle, String, String)>, source: EntityId, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let empty = if entries.is_empty() { "No other windows · Open a new window first" } else { "No matching windows" };
        let commands = entries.into_iter().map(|(destination, title, subtitle)| Command {
            title, subtitle: Some(subtitle), shortcut: None,
            target: PaletteEvent::MoveWorkspaceToWindow(source, destination),
        }).collect();
        Self::with_commands(commands, "Move workspace to window", empty, window, cx)
    }
    pub fn fonts(mut families: Vec<String>, default: bool, window: &mut Window, cx: &mut Context<Self>) -> Self {
        families.retain(|name| !name.trim().is_empty() && name.len() <= 128 && !name.chars().any(char::is_control) && !name.starts_with('.'));
        families.sort_unstable();
        families.dedup();
        let limited = families.len() > 2048;
        families.truncate(2048);
        let commands = families.into_iter().map(|family| Command {
            title: family.clone(), subtitle: None, shortcut: None,
            target: PaletteEvent::Run(if default { Box::new(SetDefaultFont { family }) } else { Box::new(SetTerminalFont { family }) }),
        }).collect();
        Self::with_commands(commands, match (default, limited) {
            (true, true) => "Choose default font · First 2,048 families",
            (true, false) => "Choose default font · Save in Settings to keep",
            (false, true) => "Choose terminal font · First 2,048 families",
            (false, false) => "Choose font for this terminal",
        }, "No matching installed fonts", window, cx)
    }
    fn with_commands(commands: Vec<Command>, heading: &'static str, empty_message: &'static str, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| text_field::TextField::new("", cx));
        input.read(cx).focus.focus(window);
        cx.subscribe(
            &input,
            |this, _, event: &text_field::EditEvent, cx| match event {
                text_field::EditEvent::Changed(query) => {
                    this.feedback = None;
                    if this.buffer_rename.is_some() || this.directory_input || this.terminal_search_input || this.ssh_input {
                        this.empty_message = if this.ssh_input { ssh::INPUT_HINT } else if this.terminal_search_input { "Enter submits a literal search · Retained history · Up to 256 terminals" } else if this.directory_input { "Enter an absolute directory path; no shell expansion" } else { "Enter a unique buffer name (up to 128 bytes)" };
                        cx.notify();
                        return;
                    }
                    if this.shortcuts_only {
                        let filter = crate::shortcut_label::ShortcutQuery::new(query);
                        this.search.update_filtered(query, |index| this.commands[index].shortcut.as_deref().is_some_and(|hint| filter.matches(hint)));
                    } else { this.search.update(query); }
                    this.selected = 0;
                    this.scroll.reset(this.search.matches.len());
                    cx.notify();
                }
                text_field::EditEvent::Submitted { reverse, .. } => {
                    if *reverse && this.selected_ssh_destination().is_some() { this.edit_selected_ssh(cx); }
                    else if *reverse && this.selected_preview_file().is_some() { this.preview_selected_hex(cx); }
                    else { this.run(cx); }
                },
                text_field::EditEvent::Cancelled => cx.emit(PaletteEvent::Close),
            },
        )
        .detach();
        Self {
            input, heading, empty_message, buffer_rename: None, feedback: None, preview_link: None, code_scroll: Default::default(), directory_input: false, terminal_search_input: false, ssh_input: false, shortcuts_only: false, hex_preview: false, markdown_preview: false, wrap_preview: false, folder_navigation: None, preview_navigation: None, location: None,
            search: palette_match::RankedSearch::with_shortcuts(commands.iter().map(Command::search_text).collect(), commands.iter().map(Command::shortcut_text).collect()),
            scroll: ListState::new(commands.len(), ListAlignment::Top, px(96.)),
            commands,
            selected: 0,
        }
    }
    pub fn folder_state(&self, cx: &App) -> Option<FolderState> {
        let (_, _, path) = self.folder_navigation.as_ref()?;
        let target = self.search.matches.get(self.selected).and_then(|index| folder_target(&self.commands[*index])).map(|(path, kind)| (path.to_owned(), kind));
        Some(FolderState { path: path.clone(), query: self.input.read(cx).value().to_owned(), target })
    }
    pub fn restore_folder(&mut self, state: FolderState, cx: &mut Context<Self>) {
        if !self.folder_navigation.as_ref().is_some_and(|(_, _, path)| state.matches(path)) { return; }
        self.input.update(cx, |input, cx| input.replace_draft(&state.query, cx));
        let entity = cx.entity();
        cx.defer(move |cx| entity.update(cx, |this, cx| {
            this.search.update(&state.query);
            this.selected = state.target.and_then(|target| this.search.matches.iter().position(|index| folder_target(&this.commands[*index]) == Some((target.0.as_path(), target.1)))).unwrap_or(0);
            this.scroll.reset(this.search.matches.len());
            crate::list_navigation::reveal(&this.scroll, this.selected);
            cx.notify();
        }));
    }
    pub fn preview_state(&self, cx: &App) -> Option<PreviewState> {
        let (path, hex) = self.preview_navigation.as_ref()?;
        Some(PreviewState { path: path.clone(), hex: *hex, markdown: self.markdown_preview, query: self.input.read(cx).value().to_owned(), wrapped: self.wrap_preview, command: self.search.matches.get(self.selected).copied() })
    }
    pub fn restore_preview(&mut self, state: PreviewState, cx: &mut Context<Self>) {
        if !self.preview_navigation.as_ref().is_some_and(|(path, hex)| state.matches(path, *hex, self.markdown_preview)) { return; }
        self.wrap_preview = state.wrapped;
        self.input.update(cx, |input, cx| input.replace_draft(&state.query, cx));
        // The input's Changed event updates ranking and resets selection first.
        let entity = cx.entity();
        cx.defer(move |cx| entity.update(cx, |this, cx| {
            this.search.update(&state.query);
            this.selected = state.command.and_then(|command| this.search.matches.iter().position(|index| *index == command)).unwrap_or(0);
            this.scroll.reset(this.search.matches.len());
            crate::list_navigation::reveal(&this.scroll, this.selected);
            cx.notify();
        }));
    }
    fn command_height(&self, command: usize) -> f32 {
        if self.wrap_preview && matches!(self.commands[command].markdown_kind(), Some(markdown_preview::Kind::Heading(_))) { 84. }
        else if self.wrap_preview && self.commands[command].preview_line().is_some() { 116. }
        else { result_row_height(self.commands[command].subtitle.is_some()) }
    }
    fn edit_selected_file(&mut self, cx: &mut Context<Self>) {
        if self.input.read(cx).is_composing() { return; }
        let path = self.preview_navigation.as_ref().map(|(path, _)| path.clone())
            .or_else(|| self.selected_preview_file().map(std::path::Path::to_owned));
        if let Some(path) = path { cx.emit(PaletteEvent::Run(Box::new(EditLocalFile { path, ..Default::default() }))); }
    }
    fn navigate_folder(&mut self, action: &dyn Action, cx: &mut Context<Self>) {
        if self.input.read(cx).is_composing() { return; }
        let Some((back, forward, path)) = self.folder_navigation.as_ref() else { return; };
        let action: Box<dyn Action> = if action.as_any().is::<FileBrowserBack>() {
            if !back { return; } Box::new(FileBrowserBack)
        } else if action.as_any().is::<FileBrowserForward>() {
            if !forward { return; } Box::new(FileBrowserForward)
        } else if action.as_any().is::<FileBrowserParent>() {
            let Some(parent) = path.parent() else { return; };
            Box::new(BrowseDirectory { path: parent.to_owned() })
        } else if action.as_any().is::<FileBrowserLocation>() {
            Box::new(EnterDirectory { path: path.clone() })
        } else { Box::new(BrowseDirectory { path: path.clone() }) };
        cx.emit(PaletteEvent::Run(action));
    }
    fn toggle_markdown_preview(&mut self, cx: &mut Context<Self>) {
        if self.input.read(cx).is_composing() { return; }
        let Some((path, _)) = self.preview_navigation.as_ref() else { return; };
        let action: Box<dyn Action> = if self.markdown_preview { Box::new(PreviewLocalFile { path: path.clone() }) }
            else { Box::new(PreviewMarkdownFile { path: path.clone() }) };
        cx.emit(PaletteEvent::Run(action));
    }
    fn navigate_preview(&mut self, directory: bool, switch_mode: bool, cx: &mut Context<Self>) {
        if self.input.read(cx).is_composing() { return; }
        let Some((path, hex)) = self.preview_navigation.as_ref() else { return; };
        let action: Box<dyn Action> = if directory {
            let Some(parent) = path.parent() else { return; };
            Box::new(BrowseDirectory { path: parent.to_owned() })
        } else if self.markdown_preview && !switch_mode {
            Box::new(PreviewMarkdownFile { path: path.clone() })
        } else if *hex != switch_mode {
            Box::new(PreviewHexFile { path: path.clone() })
        } else {
            Box::new(PreviewLocalFile { path: path.clone() })
        };
        cx.emit(PaletteEvent::Run(action));
    }
    fn selected_preview_links(&self) -> Option<(usize, &[SharedString])> {
        if !self.markdown_preview { return None; }
        let command = *self.search.matches.get(self.selected)?;
        let PaletteEvent::Run(action) = &self.commands[command].target else { return None; };
        let line = action.as_any().downcast_ref::<CopyPreviewLine>()?;
        (!line.links.is_empty()).then_some((command, line.links.as_ref()))
    }
    fn preview_link_target(&self) -> Option<(usize, usize, SharedString)> {
        let (command, links) = self.selected_preview_links()?;
        let index = markdown_preview::link_index(self.preview_link, command, links.len(), 0)?;
        Some((index, links.len(), links[index].clone()))
    }
    fn cycle_preview_link(&mut self, delta: i32, cx: &mut Context<Self>) {
        if self.input.read(cx).is_composing() { return; }
        let Some((command, links)) = self.selected_preview_links() else { return; };
        self.preview_link = markdown_preview::link_index(self.preview_link, command, links.len(), delta).map(|index| (command, index));
        cx.notify();
    }
    fn open_preview_link(&self, cx: &mut Context<Self>) {
        if self.input.read(cx).is_composing() { return; }
        if let Some((_, _, url)) = self.preview_link_target() && openable_uri(&url) { cx.open_url(&url); }
    }
    fn toggle_preview_wrap(&mut self, cx: &mut Context<Self>) {
        if self.preview_navigation.is_none() || self.hex_preview || self.input.read(cx).is_composing() { return; }
        self.wrap_preview = !self.wrap_preview;
        self.scroll.reset(self.search.matches.len());
        crate::list_navigation::reveal(&self.scroll, self.selected);
        cx.notify();
    }
    fn render_result(&mut self, row: usize, cx: &mut Context<Self>) -> AnyElement {
        let Some(&command) = self.search.matches.get(row) else { return div().into_any_element(); };
        let selected = row == self.selected;
        let wrapped = self.wrap_preview && self.commands[command].preview_line().is_some();
        let markdown = self.commands[command].markdown_kind();
        let heading = match markdown { Some(markdown_preview::Kind::Heading(level)) => Some(level), _ => None };
        let title = self.commands[command].preview_line()
            .map(|text| file_browser::preview_line_title(text, self.input.read(cx).value()))
            .unwrap_or_else(|| self.commands[command].title.clone());
        let excerpt_len = title.len();
        let title = match markdown {
            Some(markdown_preview::Kind::List { number, checked, marker: true, .. }) => {
                let bullet = number.map(|number| format!("{number}. ")).unwrap_or_else(|| if checked.is_none() { "• ".into() } else { String::new() });
                let checkbox = match checked { Some(true) => "☑ ", Some(false) => "☐ ", None => "" };
                format!("{bullet}{checkbox}{title}")
            },
            _ => title,
        };
        let inline = match &self.commands[command].target {
            PaletteEvent::Run(action) => action.as_any().downcast_ref::<CopyPreviewLine>().map(|line|
                markdown_preview::excerpt_spans(&line.text, self.input.read(cx).value(), &line.spans, title.len() - excerpt_len)).unwrap_or_default(),
            _ => Vec::new(),
        };
        let table = if let Some(markdown_preview::Kind::Table { header, alignment, .. }) = markdown {
            match &self.commands[command].target {
                PaletteEvent::Run(action) => action.as_any().downcast_ref::<CopyPreviewLine>().map(|line| {
                    let cells = markdown_preview::table_cells(&line.text, &line.spans, self.input.read(cx).value());
                    div().flex().w_full().min_w_0().border_b_1().border_color(rgb(0x637894))
                        .when(header, |row| row.font_weight(FontWeight::BOLD))
                        .children(cells.into_iter().enumerate().map(|(column, (text, spans))| {
                            let search = file_browser::preview_highlights(&text, self.input.read(cx).value());
                            div().flex_1().min_w_0().px_2().border_r_1().border_color(rgb(0x637894))
                                .text_align(match alignment[column] {
                                    markdown_preview::Alignment::Center => TextAlign::Center,
                                    markdown_preview::Alignment::Right => TextAlign::Right,
                                    _ => TextAlign::Left,
                                })
                                .when(wrapped, |cell| cell.whitespace_normal().line_clamp(4).text_ellipsis())
                                .when(!wrapped, |cell| cell.truncate())
                                .child(PreviewStyledText { id: format!("preview-cell-{command}-{column}").into(), links: line.links.clone(), styles: markdown_preview::merge_highlights(&spans, &search), text })
                        })).into_any_element()
                }),
                _ => None,
            }
        } else { None };
        let title_hint: SharedString = match &self.commands[command].subtitle {
            Some(subtitle) => format!("{title}\n{subtitle}").into(),
            None => title.clone().into(),
        };
        let title_ranges = if self.commands[command].preview_line().is_some() {
            file_browser::preview_highlights(&title, self.input.read(cx).value())
        } else {
            rmux_core::palette_match::fuzzy_highlights(&title, self.input.read(cx).value())
        };
        let code_view = if wrapped && markdown == Some(markdown_preview::Kind::Code) {
            let source = self.commands[command].preview_line().unwrap_or("");
            let query = self.input.read(cx).value();
            let state = self.code_scroll.entry(command).or_insert_with(|| CodeViewport::new(source));
            state.search(source, query);
            Some(div().id(("code-scroll", command)).h(px(80.)).w_full()
                .overflow_scroll().track_scroll(&state.scroll).flex().flex_col().items_start()
                .children(state.lines.iter().map(|(line, ranges)| {
                    div().h(px(20.)).flex_shrink_0().whitespace_nowrap().child(
                        StyledText::new(line.clone()).with_highlights(ranges.iter().cloned().map(|range| (
                            range, HighlightStyle { color: Some(rgb(0xffffff).into()), background_color: Some(rgb(0x41638c).into()), ..Default::default() }
                        ))))
                })).into_any_element())
        } else { None };
        let links = match &self.commands[command].target {
            PaletteEvent::Run(action) => action.as_any().downcast_ref::<CopyPreviewLine>().map(|line| line.links.clone()).unwrap_or_default(),
            _ => Default::default(),
        };
        let title = PreviewStyledText { id: format!("preview-text-{command}").into(), links, text: title, styles: markdown_preview::merge_highlights(&inline, &title_ranges) };
        let subtitle = self.commands[command].subtitle.as_ref().map(|text| {
            let ranges = match &self.commands[command].target {
                PaletteEvent::SearchTerminal { query, preview: true, .. } => literal_highlights(text, query),
                _ if self.commands[command].preview_line().is_some() => file_browser::preview_highlights(text, self.input.read(cx).value()),
                _ => rmux_core::palette_match::fuzzy_highlights(text, self.input.read(cx).value()),
            };
            StyledText::new(text.clone()).with_highlights(ranges.into_iter().map(|range| (
                range, HighlightStyle { color: Some(rgb(0xffffff).into()), background_color: Some(rgb(0x41638c).into()), ..Default::default() }
            )))
        });
        palette_view::result(command, self.command_height(command), selected)
            .when(self.hex_preview && self.commands[command].preview_line().is_some(), |row| row.font_family(FONT_FAMILY))
            .when_some(heading, |row, level| row.font_weight(FontWeight::BOLD).text_size(px(match level { 1 => 20., 2 => 18., 3 => 16., _ => 14. })).line_height(px(24.)))
            .when(markdown == Some(markdown_preview::Kind::Code), |row| row.font_family(FONT_FAMILY))
            .when_some(match markdown { Some(markdown_preview::Kind::List { depth, .. }) => Some(depth), _ => None }, |row, depth| row.pl(px(12. + depth.saturating_sub(1).min(8) as f32 * 12.)))
            .when(matches!(markdown, Some(markdown_preview::Kind::Quote | markdown_preview::Kind::List { .. })), |row| row.border_l_2().border_color(rgb(0x637894)))
            .on_click(cx.listener(move |this, event: &ClickEvent, _, cx| {
                if !event.standard_click() { return; }
                let Some(row) = this.search.matches.iter().position(|index| *index == command) else { return; };
                this.selected = row;
                this.run(cx);
            }))
            .child(
                div()
                    .id(("command-title", command))
                    .flex_1()
                    .min_w_0()
                    .when(!wrapped, |title| title.truncate())
                    .tooltip(move |_, cx| cx.new(|_| pane_view::ControlHint(title_hint.clone())).into())
                    .child(div().flex().flex_col().gap_1()
                        .child(if let Some(table) = table { table } else if let Some(code) = code_view {
                            code
                        } else { div().when(wrapped, |title| title.whitespace_normal().line_clamp(if heading.is_some() { 2 } else { 4 }).text_ellipsis()).when(!wrapped, |title| title.truncate()).child(title).into_any_element() })
                        .when_some(subtitle, |column, subtitle| column.child(div().truncate().text_xs().line_height(px(16.)).text_color(rgb(0xa0a0a0)).child(subtitle)))),
            )
            .when_some(self.commands[command].shortcut.clone(), |row, shortcut| {
                row.child(
                    palette_view::shortcut(shortcut.clone()).id(("command-shortcut", command))
                        .tooltip({ let hint: SharedString = shortcut.clone().into(); move |_, cx| cx.new(|_| pane_view::ControlHint(hint.clone())).into() }),
                )
            }).into_any_element()
    }
    fn selected_search_excerpt(&self) -> Option<&str> {
        self.search.matches.get(self.selected)
            .map(|index| &self.commands[*index])
            .filter(|command| matches!(command.target, PaletteEvent::SearchTerminal { preview: true, .. }))
            .and_then(|command| command.subtitle.as_deref())
    }
    fn copy_search_excerpt(&mut self, cx: &mut Context<Self>) {
        if self.input.read(cx).is_composing() { return; }
        let Some(excerpt) = self.selected_search_excerpt() else { return; };
        clipboard::write(ClipboardItem::new_string(excerpt.to_owned()), cx);
        self.feedback = Some("Copied displayed excerpt");
        cx.notify();
    }
    fn file_navigation_button(&self, id: &'static str, label: &'static str, action: Option<Box<dyn Action>>, cx: &mut Context<Self>) -> impl IntoElement {
        let enabled = action.is_some();
        div().id(id).px_2().py_1().rounded_sm().text_xs()
            .text_color(rgb(if enabled { 0xdcdfe6 } else { 0x777777 }))
            .bg(rgb(0x303030))
            .when(enabled, |button| button.cursor_pointer().hover(|style| style.bg(rgb(0x454545))))
            .on_click(cx.listener(move |this, event: &ClickEvent, _, cx| {
                if !event.standard_click() || this.input.read(cx).is_composing() { return; }
                if let Some(action) = &action { cx.emit(PaletteEvent::Run(action.boxed_clone())); }
            }))
            .child(label)
    }

    fn selected_preview_file(&self) -> Option<&std::path::Path> {
        if self.ssh_input || self.directory_input || self.terminal_search_input || self.hex_preview || self.buffer_rename.is_some() { return None; }
        let command = self.commands.get(*self.search.matches.get(self.selected)?)?;
        match &command.target {
            PaletteEvent::Run(action) => action.as_any().downcast_ref::<PreviewLocalFile>().map(|action| action.path.as_path()),
            _ => None,
        }
    }
    fn preview_selected_hex(&mut self, cx: &mut Context<Self>) {
        if self.input.read(cx).is_composing() { return; }
        let Some(path) = self.selected_preview_file().map(std::path::Path::to_owned) else { return; };
        cx.emit(PaletteEvent::Run(Box::new(PreviewHexFile { path })));
    }
    fn selected_ssh_destination(&self) -> Option<&str> {
        if self.ssh_input { return None; }
        let command = self.commands.get(*self.search.matches.get(self.selected)?)?;
        match &command.target {
            PaletteEvent::Run(action) => action.as_any().downcast_ref::<ConnectSsh>().map(|action| action.destination.as_str()),
            _ => None,
        }
    }
    fn edit_selected_ssh(&mut self, cx: &mut Context<Self>) {
        if self.input.read(cx).is_composing() { return; }
        let Some(destination) = self.selected_ssh_destination().map(str::to_owned) else { return; };
        self.ssh_input = true;
        self.heading = "Connect via SSH";
        self.empty_message = ssh::INPUT_HINT;
        self.location = None;
        self.feedback = None;
        self.commands.clear();
        self.search.matches.clear();
        self.selected = 0;
        self.scroll.reset(0);
        self.input.update(cx, |input, cx| { input.replace_draft(&destination, cx); input.select_all_text(cx); });
        cx.notify();
    }
    fn run(&mut self, cx: &mut Context<Self>) {
        if self.input.read(cx).is_composing() { return; }
        if self.ssh_input {
            let destination = self.input.read(cx).value();
            match ssh::command(destination) {
                Ok(_) => cx.emit(PaletteEvent::Run(Box::new(ConnectSsh { destination: destination.to_owned() }))),
                Err(error) => { self.empty_message = error; cx.notify(); }
            }
            return;
        }
        if self.terminal_search_input {
            let query = self.input.read(cx).value();
            match terminal_search::validate(query) {
                Ok(()) => cx.emit(PaletteEvent::Run(Box::new(SearchTerminalOutput { query: query.to_owned() }))),
                Err(error) => { self.empty_message = error; cx.notify(); }
            }
            return;
        }
        if self.directory_input {
            match file_browser::entered_directory(self.input.read(cx).value()) {
                Ok(path) => cx.emit(PaletteEvent::Run(Box::new(BrowseDirectory { path }))),
                Err(error) => { self.empty_message = error; cx.notify(); }
            }
            return;
        }
        let directory = self.search.matches.get(self.selected).and_then(|index| match &self.commands[*index].target {
            PaletteEvent::Run(action) => action.as_any().downcast_ref::<EnterDirectory>().map(|action| action.path.clone()),
            _ => None,
        });
        if let Some(path) = directory {
            let Some(path) = path.to_str() else { self.feedback = Some("This path cannot be represented as UTF-8 text"); cx.notify(); return; };
            if path.len() > 4096 { self.feedback = Some("Path exceeds the editor's 4096-byte limit"); cx.notify(); return; }
            self.directory_input = true;
            self.heading = "Go to directory · Enter to open · Escape to cancel";
            self.empty_message = "Enter an absolute directory path; no shell expansion";
            self.commands.clear();
            self.search = palette_match::RankedSearch::new(Vec::new());
            self.scroll.reset(0);
            self.input.update(cx, |input, cx| { input.replace_draft(path, cx); input.select_all_text(cx); });
            cx.notify();
            return;
        }
        if let Some(PaletteEvent::Run(action)) = self.search.matches.get(self.selected)
            .map(|index| &self.commands[*index].target) {
            if let Some(line) = action.as_any().downcast_ref::<CopyPreviewLine>() {
                clipboard::write(ClipboardItem::new_string(line.text.clone()), cx);
                self.feedback = Some("Copied preview line");
                cx.notify();
                return;
            }
            if let Some(preview) = action.as_any().downcast_ref::<CopyPreviewText>() {
                clipboard::write(ClipboardItem::new_string(preview.text.clone()), cx);
                self.feedback = Some("Copied preview text");
                cx.notify();
                return;
            }
        }
        if let Some((name, revision)) = &self.buffer_rename {
            let next = self.input.read(cx).value().to_owned();
            match cx.global_mut::<copy_buffers::Store>().rename_if_unchanged(name, next, *revision) {
                Ok(_) => cx.emit(PaletteEvent::Close),
                Err(error) => { self.empty_message = error; cx.notify(); }
            }
            return;
        }
        let rename = self.search.matches.get(self.selected)
            .and_then(|index| match &self.commands[*index].target {
                PaletteEvent::Run(action) => action.as_any().downcast_ref::<RenameNamedBuffer>().cloned(),
                _ => None,
            });
        if let Some(rename) = rename {
            self.heading = "Rename buffer · Enter to save · Escape to cancel";
            self.empty_message = "Enter a unique buffer name (up to 128 bytes)";
            self.buffer_rename = Some((rename.name.clone(), rename.revision));
            self.commands.clear();
            self.search = palette_match::RankedSearch::new(Vec::new());
            self.scroll.reset(0);
            self.input.update(cx, |input, cx| {
                input.replace_draft(&rename.name, cx);
                input.select_all_text(cx);
            });
            cx.notify();
            return;
        }
        if let Some(index) = self.search.matches.get(self.selected) {
            cx.emit(match &self.commands[*index].target {
                PaletteEvent::Run(action) => PaletteEvent::Run(action.boxed_clone()),
                PaletteEvent::SelectWorkspace(id) => PaletteEvent::SelectWorkspace(*id),
                PaletteEvent::SelectWindowWorkspace(window, workspace) => PaletteEvent::SelectWindowWorkspace(*window, *workspace),
                PaletteEvent::SelectTerminal(workspace, terminal) => {
                    PaletteEvent::SelectTerminal(*workspace, *terminal)
                }
                PaletteEvent::SelectWindowTerminal(window, workspace, terminal) => PaletteEvent::SelectWindowTerminal(*window, *workspace, *terminal),
                PaletteEvent::SearchTerminal { window, workspace, terminal, query, preview } => PaletteEvent::SearchTerminal { window: *window, workspace: *workspace, terminal: *terminal, query: query.clone(), preview: *preview },
                PaletteEvent::MoveTabToWorkspace(id) => PaletteEvent::MoveTabToWorkspace(*id),
                PaletteEvent::MoveTabToWindowWorkspace(window, workspace) => PaletteEvent::MoveTabToWindowWorkspace(*window, *workspace),
                PaletteEvent::MergeWorkspaces(source, destination, target) => PaletteEvent::MergeWorkspaces(*source, *destination, *target),
                PaletteEvent::MoveWorkspaceToWindow(source, destination) => {
                    PaletteEvent::MoveWorkspaceToWindow(*source, *destination)
                }
                PaletteEvent::Close => PaletteEvent::Close,
            });
        }
    }
}
impl Render for CommandPalette {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let height = f32::from(window.viewport_size().height);
        let compact = height < 300.;
        let has_search_excerpt = self.selected_search_excerpt().is_some();
        let has_ssh_destination = self.selected_ssh_destination().is_some();
        let has_preview_file = self.selected_preview_file().is_some();
        let preview_shortcuts = if self.preview_navigation.is_some() {
            [(&RefreshFilePreview as &dyn Action, "refresh"), (&PreviewDirectory as &dyn Action, "directory"), (&SwitchPreviewMode as &dyn Action, "text/hex"), (&ToggleMarkdownPreview as &dyn Action, "Markdown/source"), (&EditSelectedFile as &dyn Action, "edit")]
                .into_iter().filter_map(|(action, label)| shortcuts::hint(action, &self.input.read(cx).focus, window).map(|key| format!("{key} {label}")))
                .collect::<Vec<_>>().join(" · ")
        } else if self.folder_navigation.is_some() {
            [(&FileBrowserBack as &dyn Action, "back"), (&FileBrowserForward as &dyn Action, "forward"), (&FileBrowserParent as &dyn Action, "parent"), (&RefreshFileBrowser as &dyn Action, "refresh"), (&FileBrowserLocation as &dyn Action, "location"), (&EditSelectedFile as &dyn Action, "edit")]
                .into_iter().filter_map(|(action, label)| shortcuts::hint(action, &self.input.read(cx).focus, window).map(|key| format!("{key} {label}")))
                .collect::<Vec<_>>().join(" · ")
        } else { String::new() };
        let link_hints = if self.preview_navigation.is_some() { [(&NextPreviewLink as &dyn Action, "next"), (&PreviousPreviewLink as &dyn Action, "previous"), (&OpenPreviewLink as &dyn Action, "open")]
            .into_iter().filter_map(|(action, label)| shortcuts::hint(action, &self.input.read(cx).focus, window).map(|key| format!(" · {key} {label}"))).collect::<String>() } else { String::new() };
        let wrap_hint = (self.preview_navigation.is_some() && !self.hex_preview)
            .then(|| shortcuts::hint(&TogglePreviewWrap, &self.input.read(cx).focus, window)).flatten();
        let copy_hint = has_search_excerpt
            .then(|| shortcuts::hint(&CopySearchExcerpt, &self.input.read(cx).focus, window)).flatten();
        palette_view::panel(height)
            .key_context(if self.preview_navigation.is_some() { if self.hex_preview { "CommandPalette FilePreview" } else if self.markdown_preview { "CommandPalette FilePreview TextPreview MarkdownPreview" } else { "CommandPalette FilePreview TextPreview" } } else if self.folder_navigation.is_some() { "CommandPalette FileBrowser" } else { "CommandPalette" })
            .on_action(cx.listener(|this, _: &OpenPreviewLink, _, cx| this.open_preview_link(cx)))
            .on_action(cx.listener(|this, _: &NextPreviewLink, _, cx| this.cycle_preview_link(1, cx)))
            .on_action(cx.listener(|this, _: &PreviousPreviewLink, _, cx| this.cycle_preview_link(-1, cx)))
            .on_action(cx.listener(|this, _: &TogglePreviewWrap, _, cx| this.toggle_preview_wrap(cx)))
            .on_action(cx.listener(|this, action: &FileBrowserBack, _, cx| this.navigate_folder(action, cx)))
            .on_action(cx.listener(|this, action: &FileBrowserForward, _, cx| this.navigate_folder(action, cx)))
            .on_action(cx.listener(|this, action: &FileBrowserParent, _, cx| this.navigate_folder(action, cx)))
            .on_action(cx.listener(|this, action: &FileBrowserLocation, _, cx| this.navigate_folder(action, cx)))
            .on_action(cx.listener(|this, action: &RefreshFileBrowser, _, cx| this.navigate_folder(action, cx)))
            .on_action(cx.listener(|this, _: &RefreshFilePreview, _, cx| this.navigate_preview(false, false, cx)))
            .on_action(cx.listener(|this, _: &PreviewDirectory, _, cx| this.navigate_preview(true, false, cx)))
            .on_action(cx.listener(|this, _: &SwitchPreviewMode, _, cx| this.navigate_preview(false, true, cx)))
            .on_action(cx.listener(|this, _: &ToggleMarkdownPreview, _, cx| this.toggle_markdown_preview(cx)))
            .on_action(cx.listener(|this, _: &EditSelectedFile, _, cx| this.edit_selected_file(cx)))
            .on_action(cx.listener(|this, _: &CopySearchExcerpt, _, cx| this.copy_search_excerpt(cx)))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_click(|_, _, cx| cx.stop_propagation())
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                if this.input.read(cx).is_composing() {
                    return;
                }
                let Some(key) = palette_view::navigation_key(&event.keystroke) else { return; };
                // An empty search still owns its plain navigation keys.
                cx.stop_propagation();
                let selected = if matches!(key, "pageup" | "pagedown") {
                    paged_result(this.selected, this.search.matches.len(), key == "pageup",
                        f32::from(this.scroll.viewport_bounds().size.height),
                        |row| this.command_height(this.search.matches[row]))
                } else {
                    navigated_result(this.selected, this.search.matches.len(), key)
                };
                if let Some(selected) = selected {
                    this.feedback = None;
                    this.selected = selected;
                    crate::list_navigation::reveal(&this.scroll, this.selected);
                    cx.notify();
                    cx.stop_propagation();
                }
            }))
            .when(!compact, |palette| palette.child(
                palette_view::header(self.heading, palette_view::close_button()
                    .on_click(cx.listener(|_, event: &ClickEvent, _, cx| {
                        if event.standard_click() { cx.emit(PaletteEvent::Close); }
                        cx.stop_propagation();
                    }))),
            ))
            .when_some(self.location.clone(), |palette, location| palette.child(
                div().id("file-browser-location").flex_shrink_0().min_w_0().px_3().pt_2()
                    .text_xs().text_color(rgb(0xb8b8b8)).truncate()
                    .tooltip({ let location = location.clone(); move |_, cx| cx.new(|_| pane_view::ControlHint(location.clone())).into() })
                    .child(location)
            ))
            .when_some(self.folder_navigation.as_ref(), |palette, (back, forward, path)| palette.child(
                div().flex_shrink_0().px_3().pt_2().flex().flex_wrap().gap_2()
                    .child(self.file_navigation_button("folder-back", "← Back", back.then(|| Box::new(FileBrowserBack) as Box<dyn Action>), cx))
                    .child(self.file_navigation_button("folder-forward", "Forward →", forward.then(|| Box::new(FileBrowserForward) as Box<dyn Action>), cx))
                    .child(self.file_navigation_button("folder-parent", "↑ Parent", path.parent().map(|parent| Box::new(BrowseDirectory { path: parent.to_owned() }) as Box<dyn Action>), cx))
                    .child(self.file_navigation_button("folder-refresh", "Refresh", Some(Box::new(BrowseDirectory { path: path.clone() })), cx))
            ))
            .when_some(self.preview_navigation.as_ref(), |palette, (path, hex)| palette.child(
                div().flex_shrink_0().px_3().pt_2().flex().flex_wrap().gap_2()
                    .child(self.file_navigation_button("preview-directory", "← Directory", path.parent().map(|parent|
                        Box::new(BrowseDirectory { path: parent.to_owned() }) as Box<dyn Action>), cx))
                    .child(self.file_navigation_button("preview-mode", if *hex { "View as Text" } else { "View as Hex" },
                        Some(if *hex { Box::new(PreviewLocalFile { path: path.clone() }) } else { Box::new(PreviewHexFile { path: path.clone() }) }), cx))
                    .child(self.file_navigation_button("preview-markdown", if self.markdown_preview { "View Source" } else { "Read Markdown" },
                        Some(if self.markdown_preview { Box::new(PreviewLocalFile { path: path.clone() }) } else { Box::new(PreviewMarkdownFile { path: path.clone() }) }), cx))
                    .child(self.file_navigation_button("preview-editor", "External Editor", Some(Box::new(EditLocalFile { path: path.clone(), ..Default::default() })), cx))
                    .child(self.file_navigation_button("preview-refresh", "Refresh",
                        Some(if self.markdown_preview { Box::new(PreviewMarkdownFile { path: path.clone() }) } else if *hex { Box::new(PreviewHexFile { path: path.clone() }) } else { Box::new(PreviewLocalFile { path: path.clone() }) }), cx))
            ))
            .when(!preview_shortcuts.is_empty(), |palette| palette.child(
                div().flex_shrink_0().px_3().pt_2().text_xs().text_color(rgb(0xa0a0a0)).child(preview_shortcuts)
            ))
            .when(self.preview_navigation.is_some() && !self.hex_preview, |palette| palette.child(
                div().px_3().pt_2().flex_shrink_0().flex().items_center().gap_2()
                    .child(div().id("preview-wrap").px_2().py_1().rounded_sm().text_xs().cursor_pointer()
                        .bg(rgb(if self.wrap_preview { crate::chrome_style::SELECTED } else { 0x303030 }))
                        .hover(|style| style.bg(rgb(0x454545)))
                        .on_click(cx.listener(|this, event: &ClickEvent, _, cx| {
                            if event.standard_click() { this.toggle_preview_wrap(cx); }
                        }))
                        .tooltip({ let hint: SharedString = format!("Toggle wrapped excerpts{} · Copy retains the full captured line", wrap_hint.as_ref().map(|hint| format!(" · {hint}")).unwrap_or_default()).into();
                            move |_, cx| cx.new(|_| pane_view::ControlHint(hint.clone())).into() })
                        .child(if self.wrap_preview { "Single-line excerpts" } else { "Wrap excerpts" }))
                    .when(self.wrap_preview, |bar| bar.child(div().text_xs().text_color(rgb(0xa0a0a0)).child("Up to 4 visual lines")))
            ))
            .when_some(self.preview_link_target(), |palette, (index, count, url)| palette.child(
                div().px_3().py_2().flex_shrink_0().flex().flex_col().gap_1()
                    .child(div().text_xs().text_color(rgb(0xa0a0a0)).child(format!("Link {}/{count}{link_hints}", index + 1)))
                    .child(div().id("preview-link-target").truncate().text_sm().text_color(rgb(0x8ebfff)).cursor_pointer()
                        .tooltip({ let url = url.clone(); move |_, cx| cx.new(|_| pane_view::ControlHint(url.clone().into())).into() })
                        .on_click(cx.listener(|this, event: &ClickEvent, _, cx| { if event.standard_click() { cx.stop_propagation(); this.open_preview_link(cx); } }))
                        .child(url))))
            .child(palette_view::search(self.input.clone(), cx))
            .child(if self.search.matches.is_empty() {
                palette_view::empty_state(self.empty_message).into_any_element()
            } else {
                let entity = cx.entity();
                list(self.scroll.clone(), move |row, _, cx| {
                    entity.update(cx, |palette, cx| palette.render_result(row, cx))
                })
                .with_sizing_behavior(ListSizingBehavior::Auto)
                .w_full().min_h_0()
                .h(px(result_area_height(self.search.matches.iter().map(|index| self.command_height(*index)))))
                .into_any_element()
            })
            .when(self.buffer_rename.is_some() || self.directory_input || self.terminal_search_input || self.ssh_input, |palette| palette.child(
                div().flex_shrink_0().px_3().py_2().flex().flex_wrap().items_center().gap_2()
                    .child(div().flex_1().min_w_0().text_xs().text_color(rgb(0xa0a0a0))
                        .child(if self.ssh_input { "Enter connects · Escape cancels" } else if self.terminal_search_input { "Enter searches · Escape cancels" } else if self.directory_input { "Enter opens · Escape cancels" } else { "Enter saves · Escape cancels" }))
                    .child(div().id("buffer-rename-cancel").px_3().py_1().rounded_sm()
                        .text_sm().cursor_pointer().bg(rgb(0x353535)).hover(|style| style.bg(rgb(0x454545)))
                        .on_click(cx.listener(|_, event: &ClickEvent, _, cx| {
                            if event.standard_click() { cx.emit(PaletteEvent::Close); }
                        })).child("Cancel"))
                    .child(div().id("buffer-rename-save").px_3().py_1().rounded_sm()
                        .text_sm().cursor_pointer().bg(rgb(0x007aff)).hover(|style| style.bg(rgb(0x1685ff)))
                        .on_click(cx.listener(|this, event: &ClickEvent, _, cx| {
                            if event.standard_click() { this.run(cx); }
                        })).child(if self.ssh_input { "Connect" } else if self.terminal_search_input { "Search" } else if self.directory_input { "Open" } else { "Save" }))
            ))
            .when((!compact || self.feedback.is_some() || has_search_excerpt || has_ssh_destination || has_preview_file) && self.buffer_rename.is_none() && !self.directory_input && !self.terminal_search_input && !self.ssh_input, |palette| palette.child(
                palette_view::footer()
                    .child(div().flex_1().min_w_0().child(if let Some(feedback) = self.feedback {
                        div().child(feedback).into_any_element()
                    } else { palette_view::navigation_hints().into_any_element() }))
                    .when(has_preview_file, |footer| footer.child(
                        div().id("preview-selected-hex").px_2().py_1().rounded_sm()
                            .cursor_pointer().text_color(rgb(0xffffff)).bg(rgb(0x353535))
                            .hover(|style| style.bg(rgb(0x454545)))
                            .on_click(cx.listener(|this, event: &ClickEvent, _, cx| {
                                if !event.standard_click() { return; }
                                this.preview_selected_hex(cx);
                                cx.stop_propagation();
                            })).child("Preview as Hex · Shift+Enter")
                    ))
                    .when(has_preview_file, |footer| footer.child(
                        div().id("edit-selected-file").px_2().py_1().rounded_sm().cursor_pointer().text_color(rgb(0xffffff)).bg(rgb(0x353535))
                            .hover(|style| style.bg(rgb(0x454545)))
                            .on_click(cx.listener(|this, event: &ClickEvent, _, cx| {
                                if event.standard_click() { this.edit_selected_file(cx); }
                            })).child("External Editor")
                    ))
                    .when(has_ssh_destination, |footer| footer.child(
                        div().id("edit-ssh-destination").px_2().py_1().rounded_sm()
                            .cursor_pointer().text_color(rgb(0xffffff)).bg(rgb(0x353535))
                            .hover(|style| style.bg(rgb(0x454545)))
                            .on_click(cx.listener(|this, event: &ClickEvent, _, cx| {
                                if !event.standard_click() { return; }
                                this.edit_selected_ssh(cx);
                                cx.stop_propagation();
                            })).child("Edit destination · Shift+Enter")
                    ))
                    .when(has_search_excerpt, |footer| footer.child(
                        div().id("copy-search-excerpt").px_2().py_1().rounded_sm()
                            .cursor_pointer().text_color(rgb(0xffffff)).bg(rgb(0x353535))
                            .hover(|style| style.bg(rgb(0x454545)))
                            .tooltip(|_, cx| cx.new(|_| pane_view::ControlHint("Copy the displayed excerpt, including clipping ellipses".into())).into())
                            .on_click(cx.listener(move |this, event: &ClickEvent, _, cx| {
                                if !event.standard_click() { return; }
                                this.copy_search_excerpt(cx);
                                cx.stop_propagation();
                            }))
                            .child(copy_hint.map_or_else(|| "Copy excerpt".to_owned(), |hint| format!("Copy excerpt · {hint}")))
                    )),
            ))
    }
}

fn reveal_local_path(path: &std::path::Path) -> Command {
    let title = if cfg!(target_os = "macos") { "Reveal in Finder" }
        else if cfg!(target_os = "windows") { "Reveal in File Explorer" }
        else { "Reveal in File Manager" };
    Command { title: title.into(), subtitle: Some(path.display().to_string()), shortcut: None,
        target: PaletteEvent::Run(Box::new(RevealLocalPath { path: path.to_owned() })) }
}

fn custom_command(command: &preferences::CustomCommand) -> Command {
    // Quote ambiguous arguments for display only; launching always uses literal argv.
    // Keep long definitions bounded in tooltips and omit environment values.
    let argv = command.argv.iter().map(|arg| {
        if arg.is_empty() || arg.chars().any(|ch| ch.is_whitespace() || ch.is_control() || ch == '"' || ch == '\\') {
            serde_json::to_string(arg).expect("string argument serializes")
        } else { arg.clone() }
    }).collect::<Vec<_>>().join(" ");
    let directory = command.cwd.as_ref().map(|cwd| cwd.display().to_string())
        .unwrap_or_else(|| "Current terminal directory".into());
    let mode = if command.mode == preferences::CommandMode::Background { "Background · No terminal" } else { "New terminal workspace" };
    let detail = format!("{mode} · {directory} · {argv}");
    let mut chars = detail.chars().map(|ch| if ch.is_control() { ' ' } else { ch });
    let mut subtitle: String = chars.by_ref().take(768).collect();
    if chars.next().is_some() { subtitle.push('…'); }
    Command {
        title: format!("Run {}", command.name),
        subtitle: Some(subtitle),
        target: PaletteEvent::Run(Box::new(RunCustomCommand { name: command.name.clone() })),
        shortcut: None,
    }
}

fn preset_command(preset: &preferences::LayoutPreset) -> Command {
    let directory = preset.cwd.as_ref().map(|path| format!(" · Project {}", path.display())).unwrap_or_default();
    Command {
        title: format!("Open Preset: {}", preset.name),
        subtitle: Some(format!("{}{} · {}", if preset.session { "Session" } else { "Layout" }, directory, preset.path.display())),
        target: PaletteEvent::Run(Box::new(OpenLayoutPreset { name: preset.name.clone() })),
        shortcut: None,
    }
}
