use super::*;
use anyhow::{Result, ensure};

/// macOS reports shifted punctuation as its resulting character (`{` / `}`),
/// without the Shift modifier. Keep explicit Shift bindings for other layouts.
#[cfg(target_os = "macos")]
pub(super) fn bracket_navigation() -> Vec<KeyBinding> {
    let mut bindings = crate::pane_shortcuts::brackets();
    bindings.extend(crate::pane_shortcuts::tabs());
    bindings
}

/// Resolve the current binding at the terminal that will receive the action.
/// This respects custom overrides, disabled bindings and multi-stroke chords.
pub(super) fn hint(action: &dyn Action, focus: &FocusHandle, window: &Window) -> Option<String> {
    crate::shortcut_label::binding_hint(action, focus, window, cfg!(target_os = "macos"))
}

/// Presentation only: parsing and keymap identity continue using GPUI's syntax.
pub(super) fn key_label(key: &Keystroke) -> String { format_keystroke(key, cfg!(target_os = "macos")) }
/// Keep the displayed shortcut searchable both as symbols and familiar key names.
/// This is derived from the resolved hint, so disabled/default overrides cannot
/// advertise a binding that is not actually used by the originating terminal.
pub(super) use crate::shortcut_label::search_terms;

fn format_keystroke(key: &Keystroke, mac: bool) -> String { format_key_parts(&key.key, key.modifiers, mac) }
use crate::shortcut_label::format_key_parts;

#[derive(Clone, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub key: String,
    pub action: Option<String>,
    pub command: Option<String>,
    pub text: Option<String>,
    pub preset: Option<String>,
    #[serde(default)]
    pub context: Scope,
}
#[derive(Clone, Copy, Default, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope { #[default] Terminal, Window, Settings, Find, Preview, FilePreview, Files }
impl Scope { fn context(self) -> Option<&'static str> { match self { Self::Terminal => Some("Terminal"), Self::Window => None, Self::Settings => Some("Settings"), Self::Find => Some("FindInput"), Self::Preview => Some("TextPreview"), Self::FilePreview => Some("FilePreview"), Self::Files => Some("FileBrowser") } } }
struct Defaults(Vec<KeyBinding>);
impl Global for Defaults {}
fn action(name: &str) -> Option<Box<dyn Action>> {
    if let Some((kind, digit)) = name.rsplit_once('_') {
        if digit.len() == 1 && matches!(digit.as_bytes()[0], b'1'..=b'9') {
            let number = usize::from(digit.as_bytes()[0] - b'0');
            match kind {
                "select_tab" => return Some(Box::new(SelectTabNumber { number })),
                "select_pane" => return Some(Box::new(SelectPaneNumber { number })),
                "select_workspace" => return Some(Box::new(SelectWorkspaceNumber { number })),
                _ => {},
            }
        }
    }
    Some(match name {
        "new_workspace" => Box::new(NewWorkspace), "new_window" => Box::new(NewWindow),
        "copy" => Box::new(CopySelection), "paste" => Box::new(Paste),
        "select_all" => Box::new(EditorSelectAll), "clear_scrollback" => Box::new(ClearScrollback),
        "close_tab" => Box::new(ClosePane), "close_window" => Box::new(CloseWindow),
        "quit" => Box::new(Quit),
        "save_layout" => Box::new(SaveLayout), "open_layout" => Box::new(OpenLayout),
        "save_session" => Box::new(SaveSession), "open_session" => Box::new(OpenSession),
        "cancel_restore" => Box::new(CancelRestore),
        "custom_command_picker" => Box::new(ShowCustomCommandPicker),
        "layout_preset_picker" => Box::new(ShowLayoutPresetPicker),
        "open_settings_file" => Box::new(OpenSettingsFile),
        "import_theme" => Box::new(ImportTerminalTheme),
        "save_appearance" => Box::new(SaveAppearanceSettings),
        "terminal_memory" => Box::new(RefreshTerminalMemory),
        "close_terminal_memory" => Box::new(CloseTerminalMemory),
        "copy_terminal_memory" => Box::new(CopyTerminalMemory),
        "cycle_terminal_memory_sort" => Box::new(CycleTerminalMemorySort),
        "refresh_git_context" => Box::new(RefreshGitContext),
        "toggle_program_status" => Box::new(ToggleProgramStatus),
        "dismiss_all_notifications" => Box::new(DismissAllNotifications),
        "cancel_new_workspace" => Box::new(CancelNewWorkspace),
        "cancel_new_tab" => Box::new(CancelNewTab),
        "cancel_queued_paste" => Box::new(CancelQueuedPaste),
        "cancel_split" => Box::new(CancelSplit),
        "close_other_tabs" => Box::new(CloseOtherTabs),
        "close_tabs_to_right" => Box::new(CloseTabsToRight),
        "close_finished_tabs" => Box::new(CloseFinishedTabs),
        "new_tab" => Box::new(NewTab), "new_local_tab" => Box::new(NewLocalTab), "split_right" => Box::new(Split), "split_down" => Box::new(SplitBelow),
        "split_left" => Box::new(SplitLeft), "split_up" => Box::new(SplitAbove),
        "focus_left" => Box::new(FocusLeft), "focus_right" => Box::new(FocusRight),
        "focus_up" => Box::new(FocusUp), "focus_down" => Box::new(FocusDown),
        "next_workspace" => Box::new(NextWorkspace), "previous_workspace" => Box::new(PreviousWorkspace),
        "next_tab" => Box::new(NextTab), "previous_tab" => Box::new(PreviousTab),
        "focus_back" => Box::new(FocusBack), "focus_forward" => Box::new(FocusForward),
        "last_workspace" => Box::new(LastWorkspace), "last_terminal" => Box::new(LastTerminal),
        "next_pane" => Box::new(NextPane), "previous_pane" => Box::new(PreviousPane),
        "move_tab_left" => Box::new(MoveTabLeft), "move_tab_right" => Box::new(MoveTabRight),
        "move_workspace_up" => Box::new(MoveWorkspaceUp), "move_workspace_down" => Box::new(MoveWorkspaceDown),
        "previous_prompt" => Box::new(PreviousPrompt), "next_prompt" => Box::new(NextPrompt),
        "last_prompt" => Box::new(LastPrompt),
        "scroll_to_top" => Box::new(ScrollToTop), "scroll_to_bottom" => Box::new(ScrollToBottom),
        "find_older" => Box::new(FindOlder), "find_newer" => Box::new(FindNewer),
        "increase_font_size" => Box::new(IncreaseFontSize),
        "decrease_font_size" => Box::new(DecreaseFontSize),
        "reset_font_size" => Box::new(ResetFontSize),
        "copy_mode" => Box::new(EnterCopyMode),
        "paste_buffer" => Box::new(ShowBufferPicker),
        "delete_buffer" => Box::new(ShowDeleteBufferPicker),
        "rename_buffer" => Box::new(ShowRenameBufferPicker),
        "copy_buffer" => Box::new(ShowCopyBufferPicker),
        "browse_files" => Box::new(BrowseTerminalFiles),
        "refresh_files" => Box::new(RefreshFileBrowser),
        "file_parent" => Box::new(FileBrowserParent),
        "file_location" => Box::new(FileBrowserLocation),
        "refresh_preview" => Box::new(RefreshFilePreview),
        "preview_directory" => Box::new(PreviewDirectory),
        "edit_selected_file" => Box::new(EditSelectedFile),
        "toggle_markdown_preview" => Box::new(ToggleMarkdownPreview),
        "switch_preview_mode" => Box::new(SwitchPreviewMode),
        "open_preview_link" => Box::new(OpenPreviewLink),
        "next_preview_link" => Box::new(NextPreviewLink),
        "previous_preview_link" => Box::new(PreviousPreviewLink),
        "toggle_preview_wrap" => Box::new(TogglePreviewWrap),
        "cancel_file_loading" => Box::new(CancelFileLoading),
        "open_text_preview" => Box::new(OpenTextPreview),
        "search_terminal_output" => Box::new(ShowTerminalSearch),
        "connect_ssh" => Box::new(ShowSshConnection),
        "ssh_hosts" => Box::new(ShowSshHosts),
        "open_folder" => Box::new(OpenFolderBrowser),
        "file_back" => Box::new(FileBrowserBack), "file_forward" => Box::new(FileBrowserForward),
        "choose_font" => Box::new(ShowFontPicker),
        "choose_default_font" => Box::new(ShowDefaultFontPicker),
        "use_configured_font" => Box::new(UseConfiguredFont),
        "close_listening_ports" => Box::new(CloseListeningPorts),
        "listening_ports" => Box::new(ShowListeningPorts),
        "terminal_menu" => Box::new(ShowTerminalMenu),
        "tab_menu" => Box::new(ShowTabMenu),
        "workspace_menu" => Box::new(ShowWorkspaceMenu),
        "tab_picker" => Box::new(ShowTabPicker), "palette" => Box::new(ShowCommandPalette),
        "keyboard_shortcuts" => Box::new(ShowKeyboardShortcuts),
        "toggle_zoom" => Box::new(TogglePaneZoom), "toggle_sidebar" => Box::new(ToggleSidebar),
        "resize_pane_left" => Box::new(ResizePaneLeft),
        "resize_pane_right" => Box::new(ResizePaneRight),
        "resize_pane_up" => Box::new(ResizePaneUp),
        "resize_pane_down" => Box::new(ResizePaneDown),
        "equalize_panes" => Box::new(EqualizePanes), "swap_pane_next" => Box::new(SwapPaneNext),
        "break_tab_right" => Box::new(BreakTabRight), "break_tab_down" => Box::new(BreakTabBelow),
        "break_tab_left" => Box::new(BreakTabLeft), "break_tab_up" => Box::new(BreakTabAbove),
        "join_pane_next" => Box::new(JoinPaneNext),
        "join_pane_left" => Box::new(JoinPaneLeft),
        "join_pane_right" => Box::new(JoinPaneRight),
        "join_pane_up" => Box::new(JoinPaneUp),
        "join_pane_down" => Box::new(JoinPaneDown),
        "swap_pane_left" => Box::new(SwapPaneLeft),
        "swap_pane_right" => Box::new(SwapPaneRight),
        "swap_pane_up" => Box::new(SwapPaneUp),
        "swap_pane_down" => Box::new(SwapPaneDown),
        "move_tab_next_workspace" => Box::new(MoveTabNextWorkspace),
        "detach_tab" => Box::new(DetachTab), "detach_workspace" => Box::new(DetachWorkspace),
        "merge_workspace" => Box::new(ShowWorkspaceMergePicker),
        "move_workspace_window_picker" => Box::new(ShowWorkspaceWindowPicker),
        "move_workspace_next_window" => Box::new(MoveWorkspaceToNextWindow),
        "rename_tab" => Box::new(RenameTab), "rename_workspace" => Box::new(RenameWorkspace),
        "toggle_workspace_pin" => Box::new(ToggleWorkspacePin),
        "edit_workspace_description" => Box::new(EditWorkspaceDescription),
        "edit_workspace_icon" => Box::new(EditWorkspaceIcon), "edit_workspace_color" => Box::new(EditWorkspaceColor),
        "settings" => Box::new(ToggleSettings), "notifications" => Box::new(ToggleNotifications),
        "mark_all_read" => Box::new(MarkAllRead),
        "copy_with_line_breaks" => Box::new(CopySelectionWithLineBreaks),
        "copy_working_directory" => Box::new(CopyWorkingDirectory),
        "reveal_working_directory" => Box::new(RevealWorkingDirectory),
        "new_group" => Box::new(NewWorkspaceGroup),
        "group_selected" => Box::new(GroupSelectedWorkspaces),
        "rename_group" => Box::new(RenameWorkspaceGroup),
        "group_current_workspace" => Box::new(GroupCurrentWorkspace),
        "edit_group_color" => Box::new(EditGroupColor),
        "edit_group_icon" => Box::new(EditGroupIcon),
        "toggle_group_pin" => Box::new(ToggleGroupPin),
        "ungroup_workspaces" => Box::new(UngroupWorkspaces),
        "close_group" => Box::new(CloseWorkspaceGroup),
        "remove_workspace_from_group" => Box::new(RemoveWorkspaceFromGroup),
        "set_group_anchor" => Box::new(SetGroupAnchor),
        "close_workspace" => Box::new(CloseWorkspace),
        "use_current_directory_for_group" => Box::new(UseCurrentDirectoryForGroup),
        "reset_group_directory" => Box::new(ResetGroupDirectory),
        "next_unread" => Box::new(NextUnreadTerminal), "find" => Box::new(Find),
        "reload_settings" => Box::new(ReloadSettings), "disabled" => Box::new(NoAction {}),
        _ => return None,
    })
}
pub fn compile(settings: &Preferences) -> Result<Vec<KeyBinding>> {
    ensure!(settings.keybindings.len() <= 128, "at most 128 custom keybindings are allowed");
    let mut used = std::collections::HashSet::new();
    let mut sequences: Vec<(Option<&str>, Vec<String>)> = Vec::new();
    settings.keybindings.iter().map(|binding| {
        ensure!(binding.key.len() <= 128, "keybinding exceeds 128 bytes");
        let strokes = binding.key.split_whitespace().map(Keystroke::parse).collect::<std::result::Result<Vec<_>, _>>()?;
        ensure!((1..=2).contains(&strokes.len()), "keybinding requires one or two strokes");
        for (index, stroke) in strokes.iter().enumerate() {
            ensure!(!stroke.key.chars().any(char::is_control) && rmux_terminal::keyboard::key_identity(&stroke.key).is_some(), "unknown shortcut key: {}", stroke.key);
            if let Some(number) = stroke.key.strip_prefix('f').and_then(|text| text.parse::<u8>().ok()) {
                ensure!(stroke.key == format!("f{number}"), "use canonical function-key spelling: f{number}");
            }
            let function = stroke.key.strip_prefix('f').and_then(|n| n.parse::<u8>().ok()).is_some_and(|n| (1..=24).contains(&n));
            ensure!(index > 0 || stroke.modifiers.control || stroke.modifiers.platform || stroke.modifiers.alt || function, "the first custom stroke requires Ctrl/Cmd/Alt or a function key");
        }
        let sequence: Vec<_> = strokes.iter().map(ToString::to_string).collect();
        let canonical = sequence.join(" ");
        ensure!(used.insert((binding.context.context(), canonical.clone())), "duplicate shortcut in the same context: {canonical}");
        for (scope, previous) in &sequences {
            if *scope == binding.context.context() && previous.len() != sequence.len()
                && (previous.starts_with(&sequence) || sequence.starts_with(previous)) {
                anyhow::bail!("shortcut prefix conflict in the same context: {} and {canonical}", previous.join(" "));
            }
        }
        sequences.push((binding.context.context(), sequence));
        let target = match (&binding.action, &binding.command, &binding.text, &binding.preset) {
            (Some(name), None, None, None) => action(name).ok_or_else(|| anyhow::anyhow!("unknown shortcut action: {name}"))?,
            (None, Some(name), None, None) => {
                ensure!(settings.commands.iter().any(|command| command.name == *name), "shortcut command is not configured: {name}");
                Box::new(RunCustomCommand { name: name.clone() }) as Box<dyn Action>
            }
            (None, None, Some(text), None) => {
                ensure!(matches!(binding.context, Scope::Terminal), "text bindings require terminal context");
                ensure!(!text.is_empty() && text.len() <= 16 * 1024, "shortcut text requires 1..16384 UTF-8 bytes");
                Box::new(SendShortcutText { text: text.clone() }) as Box<dyn Action>
            }
            (None, None, None, Some(name)) => {
                ensure!(settings.layout_presets.iter().any(|preset| preset.name == *name), "shortcut preset is not configured: {name}");
                Box::new(OpenLayoutPreset { name: name.clone() }) as Box<dyn Action>
            }
            _ => anyhow::bail!("keybinding requires exactly one action, command, text or preset"),
        };
        let context = binding.context.context().map(|context| KeyBindingContextPredicate::parse(context).unwrap().into());
        Ok(KeyBinding::load(&binding.key, target, context, false, None, &DummyKeyboardMapper)?)
    }).collect()
}
pub fn initialize(cx: &mut App) {
    let defaults = cx.key_bindings().borrow().bindings().cloned().collect();
    cx.set_global(Defaults(defaults));
    install(cx);
}
pub fn install(cx: &mut App) {
    let bindings = compile(cx.global::<Preferences>()).expect("validated shortcuts");
    let defaults = cx.global::<Defaults>().0.clone();
    cx.clear_key_bindings();
    cx.bind_keys(defaults);
    cx.bind_keys(bindings);
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "macos")]
    #[test]
    fn bracket_navigation_distinguishes_panes_from_tabs() {
        use super::*;
        let bindings = bracket_navigation();
        for (stroke, expected) in [
            ("cmd-[", Box::new(PreviousPane) as Box<dyn Action>),
            ("cmd-]", Box::new(NextPane)),
            ("cmd-{", Box::new(PreviousTab)),
            ("cmd-}", Box::new(NextTab)),
            ("cmd-shift-[", Box::new(PreviousTab)),
            ("cmd-shift-]", Box::new(NextTab)),
        ] {
            let event = Keystroke::parse(stroke).unwrap();
            let matched: Vec<_> = bindings.iter().filter(|binding|
                binding.match_keystrokes(&[event.clone()]) == Some(false)).collect();
            assert_eq!(matched.len(), 1, "{stroke}");
            assert_eq!(matched[0].action().type_id(), expected.as_ref().type_id(), "{stroke}");
        }
    }
    #[test]
    fn resolved_shortcuts_are_searchable_by_symbols_and_key_names() {
        use crate::palette_match::RankedSearch;
        let mut search = RankedSearch::new(vec![
            format!("split below {}", super::search_terms("⇧⌘D")),
            format!("focus pane {}", super::search_terms("⌃⌥←")),
            format!("custom command {}", super::search_terms("Ctrl+Shift+K Ctrl+Enter")),
            "split above".into(),
        ]);
        for (query, expected) in [
            ("cmd shift d", 0), ("split command shift", 0), ("⇧⌘D", 0),
            ("control option left", 1), ("alt left", 1),
            ("ctrl shift k enter", 2),
        ] {
            search.update(query);
            assert_eq!(search.matches.first(), Some(&expected), "{query}");
        }
        search.update("split above cmd");
        assert!(search.matches.is_empty(), "unbound action must not inherit shortcut terms");
    }
    use super::compile;
    use crate::{preferences::Preferences, ShowCommandPalette, ToggleSettings};
    use gpui::{KeyBinding, KeyContext, Keymap, Keystroke};
    fn settings(value: serde_json::Value) -> Preferences { serde_json::from_value(value).unwrap() }
    #[test]
    fn preview_link_bindings_are_scoped_and_remappable() {
        let mut map = Keymap::new(vec![KeyBinding::new("alt-enter", crate::OpenPreviewLink, Some("MarkdownPreview"))]);
        let key = [Keystroke::parse("alt-enter").unwrap()];
        let markdown = [KeyContext::parse("CommandPalette FilePreview TextPreview MarkdownPreview").unwrap(), KeyContext::parse("TextInput").unwrap()];
        assert!(map.bindings_for_input(&key, &markdown).0[0].action().as_any().is::<crate::OpenPreviewLink>());
        for context in ["Terminal", "CommandPalette", "CommandPalette TextPreview"] {
            assert!(map.bindings_for_input(&key, &[KeyContext::parse(context).unwrap()]).0.is_empty());
        }
        map.add_bindings(compile(&settings(serde_json::json!({"keybindings":[
            {"key":"alt-enter", "action":"disabled", "context":"preview"},
            {"key":"ctrl-alt-o", "action":"open_preview_link", "context":"preview"}
        ]}))).unwrap());
        assert!(map.bindings_for_input(&key, &markdown).0.is_empty());
        assert!(map.bindings_for_input(&[Keystroke::parse("ctrl-alt-o").unwrap()], &markdown).0[0].action().as_any().is::<crate::OpenPreviewLink>());
    }
    #[test]
    fn markdown_toggle_binding_can_be_remapped_in_previews_only() {
        let mut map = Keymap::new(vec![KeyBinding::new("ctrl-alt-v", crate::ToggleMarkdownPreview, Some("FilePreview"))]);
        map.add_bindings(compile(&settings(serde_json::json!({"keybindings":[
            {"key":"ctrl-alt-v","action":"disabled","context":"file_preview"},
            {"key":"ctrl-alt-d","action":"toggle_markdown_preview","context":"file_preview"}
        ]}))).unwrap());
        let original = [Keystroke::parse("ctrl-alt-v").unwrap()];
        let replacement = [Keystroke::parse("ctrl-alt-d").unwrap()];
        for context in ["CommandPalette FilePreview", "CommandPalette FilePreview TextPreview"] {
            let contexts = [KeyContext::parse(context).unwrap()];
            assert!(map.bindings_for_input(&original, &contexts).0.is_empty());
            assert!(map.bindings_for_input(&replacement, &contexts).0[0].action().as_any().is::<crate::ToggleMarkdownPreview>());
        }
        for context in ["Terminal", "CommandPalette FileBrowser", "Settings"] {
            assert!(map.bindings_for_input(&replacement, &[KeyContext::parse(context).unwrap()]).0.is_empty());
        }
    }
    #[test]
    fn external_edit_shortcut_is_scoped_to_file_views() {
        let mut map = Keymap::new(vec![
            KeyBinding::new("ctrl-alt-e", crate::EditSelectedFile, Some("FileBrowser")),
            KeyBinding::new("ctrl-alt-e", crate::EditSelectedFile, Some("FilePreview")),
        ]);
        let prefs = settings(serde_json::json!({"keybindings":[
            {"key":"ctrl-alt-e","action":"disabled","context":"files"},
            {"key":"ctrl-alt-o","action":"edit_selected_file","context":"files"}
        ]}));
        map.add_bindings(compile(&prefs).unwrap());
        let original = [Keystroke::parse("ctrl-alt-e").unwrap()];
        let replacement = [Keystroke::parse("ctrl-alt-o").unwrap()];
        let browser = [KeyContext::parse("CommandPalette FileBrowser").unwrap()];
        assert!(map.bindings_for_input(&original, &browser).0.is_empty());
        assert!(map.bindings_for_input(&replacement, &browser).0[0].action().as_any().is::<crate::EditSelectedFile>());
        for context in ["CommandPalette FilePreview", "CommandPalette FilePreview TextPreview"] {
            assert!(map.bindings_for_input(&original, &[KeyContext::parse(context).unwrap()]).0[0].action().as_any().is::<crate::EditSelectedFile>());
        }
        for context in ["Terminal", "CommandPalette", "Settings"] {
            assert!(map.bindings_for_input(&original, &[KeyContext::parse(context).unwrap()]).0.is_empty());
        }
    }
    #[test]
    fn folder_navigation_keys_are_scoped_and_remappable() {
        let mut map = Keymap::new(vec![KeyBinding::new("f5", crate::RefreshFileBrowser, Some("FileBrowser"))]);
        let prefs = settings(serde_json::json!({"keybindings":[
            {"key":"f5","action":"disabled","context":"files"},
            {"key":"ctrl-alt-r","action":"refresh_files","context":"files"},
            {"key":"alt-up","action":"file_parent","context":"files"},
            {"key":"ctrl-l","action":"file_location","context":"files"},
            {"key":"alt-left","action":"file_back","context":"files"},
            {"key":"alt-right","action":"file_forward","context":"files"}
        ]}));
        map.add_bindings(compile(&prefs).unwrap());
        let contexts = [KeyContext::parse("CommandPalette FileBrowser").unwrap()];
        assert!(map.bindings_for_input(&[Keystroke::parse("f5").unwrap()], &contexts).0.is_empty());
        assert!(map.bindings_for_input(&[Keystroke::parse("ctrl-alt-r").unwrap()], &contexts).0[0].action().as_any().is::<crate::RefreshFileBrowser>());
        for key in ["alt-up", "ctrl-l", "alt-left", "alt-right"] {
            let keys = [Keystroke::parse(key).unwrap()];
            assert_eq!(map.bindings_for_input(&keys, &contexts).0.len(), 1);
            for context in ["Terminal", "CommandPalette FilePreview", "Settings"] {
                assert!(map.bindings_for_input(&keys, &[KeyContext::parse(context).unwrap()]).0.is_empty());
            }
        }
    }
    #[test]
    fn file_preview_navigation_bindings_cover_text_and_hex_only() {
        let mut map = Keymap::new(vec![KeyBinding::new("f5", crate::RefreshFilePreview, Some("FilePreview"))]);
        let prefs = settings(serde_json::json!({"keybindings":[
            {"key":"f5","action":"disabled","context":"file_preview"},
            {"key":"ctrl-alt-r","action":"refresh_preview","context":"file_preview"},
            {"key":"alt-up","action":"preview_directory","context":"file_preview"},
            {"key":"ctrl-alt-m","action":"switch_preview_mode","context":"file_preview"}
        ]}));
        map.add_bindings(compile(&prefs).unwrap());
        for context in ["CommandPalette FilePreview TextPreview", "CommandPalette FilePreview"] {
            let contexts = [KeyContext::parse(context).unwrap()];
            assert!(map.bindings_for_input(&[Keystroke::parse("f5").unwrap()], &contexts).0.is_empty());
            assert!(map.bindings_for_input(&[Keystroke::parse("ctrl-alt-r").unwrap()], &contexts).0[0].action().as_any().is::<crate::RefreshFilePreview>());
            assert!(map.bindings_for_input(&[Keystroke::parse("alt-up").unwrap()], &contexts).0[0].action().as_any().is::<crate::PreviewDirectory>());
            assert!(map.bindings_for_input(&[Keystroke::parse("ctrl-alt-m").unwrap()], &contexts).0[0].action().as_any().is::<crate::SwitchPreviewMode>());
        }
        for context in ["Terminal", "CommandPalette", "Settings"] {
            assert!(map.bindings_for_input(&[Keystroke::parse("alt-up").unwrap()], &[KeyContext::parse(context).unwrap()]).0.is_empty());
        }
    }
    #[test]
    fn preview_wrap_bindings_are_scoped_and_remappable() {
        let mut map = Keymap::new(vec![KeyBinding::new("ctrl-alt-w", crate::TogglePreviewWrap, Some("TextPreview"))]);
        let prefs = settings(serde_json::json!({"keybindings":[
            {"key":"ctrl-alt-w","action":"disabled","context":"preview"},
            {"key":"ctrl-alt-r","action":"toggle_preview_wrap","context":"preview"}
        ]}));
        map.add_bindings(compile(&prefs).unwrap());
        let preview = [KeyContext::parse("CommandPalette TextPreview").unwrap()];
        assert!(map.bindings_for_input(&[Keystroke::parse("ctrl-alt-w").unwrap()], &preview).0.is_empty());
        let replacement = [Keystroke::parse("ctrl-alt-r").unwrap()];
        assert!(map.bindings_for_input(&replacement, &preview).0[0].action().as_any().is::<crate::TogglePreviewWrap>());
        for context in ["Terminal", "CommandPalette", "Settings"] {
            assert!(map.bindings_for_input(&replacement, &[KeyContext::parse(context).unwrap()]).0.is_empty());
        }
    }
    #[test]
    fn find_bindings_override_only_the_search_field() {
        let key = if cfg!(target_os = "macos") { "cmd-g" } else { "f3" };
        let mut map = Keymap::new(vec![
            KeyBinding::new(key, crate::FindOlder, Some("Terminal")),
            KeyBinding::new(key, crate::FindOlder, Some("FindInput")),
        ]);
        let prefs = settings(serde_json::json!({"keybindings":[
            {"key":key,"action":"disabled","context":"find"},
            {"key":"ctrl-alt-n","action":"find_newer","context":"find"}
        ]}));
        map.add_bindings(compile(&prefs).unwrap());
        let keys = [Keystroke::parse(key).unwrap()];
        let find = [KeyContext::parse("FindInput").unwrap()];
        let terminal = [KeyContext::parse("Terminal").unwrap()];
        assert!(map.bindings_for_input(&keys, &find).0.is_empty());
        assert!(map.bindings_for_input(&keys, &terminal).0[0].action().as_any().is::<crate::FindOlder>());
        let replacement = [Keystroke::parse("ctrl-alt-n").unwrap()];
        assert!(map.bindings_for_input(&replacement, &find).0[0].action().as_any().is::<crate::FindNewer>());
        assert!(map.bindings_for_input(&replacement, &terminal).0.is_empty());
        assert!(compile(&settings(serde_json::json!({"keybindings":[
            {"key":"ctrl-alt-n","text":"shell text","context":"find"}
        ]}))).is_err());
    }
    #[test]
    fn settings_save_binding_is_scoped_and_can_be_disabled() {
        let key = if cfg!(target_os = "macos") { "cmd-s" } else { "ctrl-s" };
        let mut map = Keymap::new(vec![KeyBinding::new(key, crate::SaveAppearanceSettings, Some("Settings"))]);
        let keys = [Keystroke::parse(key).unwrap()];
        let panel = [KeyContext::parse("Settings").unwrap()];
        assert!(map.bindings_for_input(&keys, &panel).0[0].action().as_any().is::<crate::SaveAppearanceSettings>());
        assert!(map.bindings_for_input(&keys, &[KeyContext::parse("Terminal").unwrap()]).0.is_empty());
        let prefs = settings(serde_json::json!({"keybindings":[
            {"key":key,"action":"disabled","context":"settings"},
            {"key":"ctrl-alt-s","action":"save_appearance","context":"settings"}
        ]}));
        map.add_bindings(compile(&prefs).unwrap());
        assert!(map.bindings_for_input(&keys, &panel).0.is_empty());
        let replacement = [Keystroke::parse("ctrl-alt-s").unwrap()];
        assert!(map.bindings_for_input(&replacement, &panel).0[0].action().as_any().is::<crate::SaveAppearanceSettings>());
        assert!(compile(&settings(serde_json::json!({"keybindings":[
            {"key":"ctrl-alt-s","text":"do not send","context":"settings"}
        ]}))).is_err());
    }
    #[test]
    fn shortcut_labels_use_platform_notation_without_changing_key_identity() {
        for (source, mac, expected) in [
            ("cmd-shift-p", true, "⇧⌘P"), ("ctrl-alt-shift-left", true, "⌃⌥⇧←"),
            ("ctrl-shift-p", false, "Ctrl+Shift+P"), ("alt-enter", false, "Alt+Enter"),
            ("cmd-backspace", true, "⌘⌫"), ("f12", false, "F12"), ("space", true, "Space"),
        ] {
            let key = Keystroke::parse(source).unwrap();
            let identity = key.to_string();
            assert_eq!(super::format_keystroke(&key, mac), expected);
            assert_eq!(key.to_string(), identity);
        }
    }

    #[test]
    fn override_precedence_context_and_disable_use_real_keymap() {
        let defaults = vec![KeyBinding::new("ctrl-shift-p", ShowCommandPalette, None)];
        let mut map = Keymap::new(defaults.clone());
        let prefs = settings(serde_json::json!({"keybindings":[{"key":"ctrl-shift-p","action":"settings"}]}));
        map.add_bindings(compile(&prefs).unwrap());
        let keys = [Keystroke::parse("ctrl-shift-p").unwrap()];
        let terminal = [KeyContext::parse("Terminal").unwrap()];
        assert!(map.bindings_for_input(&keys, &terminal).0[0].action().as_any().is::<ToggleSettings>());
        assert!(map.bindings_for_input(&keys, &[KeyContext::parse("Settings").unwrap()]).0[0].action().as_any().is::<ShowCommandPalette>());
        let disabled = settings(serde_json::json!({"keybindings":[{"key":"ctrl-shift-p","action":"disabled"}]}));
        map.clear(); map.add_bindings(defaults.clone()); map.add_bindings(compile(&disabled).unwrap());
        assert!(map.bindings_for_input(&keys, &terminal).0.is_empty());
        map.clear(); map.add_bindings(defaults);
        assert_eq!(map.bindings().len(), 1);
        assert!(map.bindings_for_input(&keys, &terminal).0[0].action().as_any().is::<ShowCommandPalette>());
    }
    #[test]
    fn numbered_navigation_actions_preserve_the_requested_number() {
        for number in 1..=9 {
            for kind in ["select_tab", "select_pane", "select_workspace"] {
                let prefs = settings(serde_json::json!({"keybindings":[{
                    "key":format!("ctrl-k {number}"), "action":format!("{kind}_{number}")
                }]}));
                let bindings = compile(&prefs).unwrap();
                let action = bindings[0].action().as_any();
                let actual = match kind {
                    "select_tab" => action.downcast_ref::<crate::SelectTabNumber>().unwrap().number,
                    "select_pane" => action.downcast_ref::<crate::SelectPaneNumber>().unwrap().number,
                    _ => action.downcast_ref::<crate::SelectWorkspaceNumber>().unwrap().number,
                };
                assert_eq!(actual, number);
            }
        }
        for name in ["select_tab_0", "select_tab_10", "select_pane_01", "select_workspace_-1", "select_tab_x"] {
            assert!(compile(&settings(serde_json::json!({"keybindings":[{"key":"ctrl-k t","action":name}]}))).is_err());
        }
    }

    #[test]
    fn leader_chords_accept_plain_suffixes_without_binding_plain_typing() {
        let prefs = settings(serde_json::json!({"keybindings":[
            {"key":"ctrl-k t","action":"settings"},
            {"key":"ctrl-k p","action":"palette"}
        ]}));
        let mut map = Keymap::default();
        map.add_bindings(compile(&prefs).unwrap());
        let context = [KeyContext::parse("Terminal").unwrap()];
        let prefix = [Keystroke::parse("ctrl-k").unwrap()];
        let (actions, pending) = map.bindings_for_input(&prefix, &context);
        assert!(actions.is_empty());
        assert!(pending);
        let suffix = Keystroke::parse("t").unwrap();
        let (actions, pending) = map.bindings_for_input(&[suffix.clone()], &context);
        assert!(actions.is_empty() && !pending);
        let (actions, pending) = map.bindings_for_input(&[prefix[0].clone(), suffix], &context);
        assert!(!pending);
        assert!(actions[0].action().as_any().is::<ToggleSettings>());
        let (actions, pending) = map.bindings_for_input(&prefix, &[KeyContext::parse("Settings").unwrap()]);
        assert!(actions.is_empty() && !pending);
    }

    #[test]
    fn text_chords_preserve_bytes_and_are_terminal_only() {
        let text = "printf '世界'\r\u{1b}[A";
        let prefs = settings(serde_json::json!({"keybindings":[{"key":"ctrl-k t","text":text}]}));
        let mut map = Keymap::default();
        map.add_bindings(compile(&prefs).unwrap());
        let keys = [Keystroke::parse("ctrl-k").unwrap(), Keystroke::parse("t").unwrap()];
        let terminal = [KeyContext::parse("Terminal").unwrap()];
        let (actions, pending) = map.bindings_for_input(&keys, &terminal);
        assert!(!pending);
        assert_eq!(actions[0].action().as_any().downcast_ref::<crate::SendShortcutText>().unwrap().text.as_bytes(), text.as_bytes());
        assert!(map.bindings_for_input(&keys, &[KeyContext::parse("Settings").unwrap()]).0.is_empty());
        for binding in [
            serde_json::json!({"key":"ctrl-k t","text":""}),
            serde_json::json!({"key":"ctrl-k t","text":"界".repeat(5462)}),
            serde_json::json!({"key":"ctrl-k t","text":"hello","context":"window"}),
            serde_json::json!({"key":"ctrl-k t","text":"hello","action":"settings"}),
            serde_json::json!({"key":"ctrl-k t","text":"hello","command":"Tests"}),
        ] { assert!(compile(&settings(serde_json::json!({"keybindings":[binding]}))).is_err()); }
        assert!(compile(&settings(serde_json::json!({"keybindings":[{"key":"ctrl-k t","text":"x".repeat(16384)}]}))).is_ok());
    }

    #[test]
    fn invalid_and_ambiguous_shortcuts_are_rejected() {
        for value in [
            serde_json::json!([{"key":"x","action":"settings"}]),
            serde_json::json!([{"key":"x t","action":"settings"}]),
            serde_json::json!([{"key":"ctrl-k entter","action":"settings"}]),
            serde_json::json!([{"key":"","action":"settings"}]),
            serde_json::json!([{"key":"ctrl-x","action":"missing"}]),
            serde_json::json!([{"key":"ctrl-entter","action":"settings"}]),
            serde_json::json!([{"key":"f01","action":"settings"}]),
            serde_json::json!([{"key":"ctrl-k","action":"settings"},{"key":"ctrl-k ctrl-t","action":"palette"}]),
            serde_json::json!([{"key":"ctrl-k ctrl-t","action":"settings"},{"key":"ctrl-k","action":"palette"}]),
            serde_json::json!([{"key":"ctrl-X","action":"settings"},{"key":"ctrl-shift-x","action":"palette"}]),
            serde_json::json!([{"key":"ctrl-x","command":"missing"}]),
            serde_json::json!([{"key":"ctrl-x","action":"settings","command":"missing"}]),
            serde_json::json!([{"key":"ctrl-x","action":"settings"},{"key":"ctrl-x","action":"palette"}]),
        ] { assert!(compile(&settings(serde_json::json!({"keybindings":value}))).is_err()); }
        let valid = settings(serde_json::json!({"commands":[{"name":"Tests","argv":["mise","run","test"]}],"keybindings":[{"key":"ctrl-k ctrl-t","command":"Tests"}]}));
        assert_eq!(compile(&valid).unwrap().len(), 1);
        let shared_prefix = settings(serde_json::json!({"keybindings":[{"key":"ctrl-k ctrl-t","action":"settings"},{"key":"ctrl-k ctrl-p","action":"palette"}]}));
        assert_eq!(compile(&shared_prefix).unwrap().len(), 2);
    }
}
