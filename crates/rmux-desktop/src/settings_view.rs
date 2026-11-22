use super::*;

#[derive(Clone)]
pub struct State {
    pub focus: FocusHandle,
    pub origin: Option<FocusHandle>,
    pub selected: usize,
    pub scroll: ScrollHandle,
}
pub const KEYS: &[&str] = &["font", "padding", "theme", "focused_cursor", "cursor_opacity", "cursor", "copy", "close", "group_placement", "scrollback", "native_notifications", "notification_focus", "font_family", "opacity", "blur", "workspace_placement",
    #[cfg(target_os = "macos")] "option_as_alt",
    "selection_opacity", "import_theme", "file_editor", "editor_example", "editor_fallback", "shell_history",
];
pub const FOOTER_KEYS: &[&str] = &["save_settings", "open_settings", "reload_settings", "keyboard_shortcuts", "revert_settings", "undo_settings_save"];
pub const FOOTER_COUNT: usize = FOOTER_KEYS.len();
pub fn control_index(key: &str) -> Option<usize> {
    KEYS.iter().chain(FOOTER_KEYS).position(|candidate| *candidate == key)
}
pub fn footer_enabled(index: usize, busy: bool, unsaved: bool, can_undo: bool) -> bool {
    index < FOOTER_COUNT && !busy && (index != 4 || unsaved) && (index != 5 || can_undo)
}
pub fn footer_action(index: usize) -> Option<Box<dyn Action>> {
    Some(match index {
        0 => Box::new(SaveAppearanceSettings),
        1 => Box::new(OpenSettingsFile),
        2 => Box::new(ReloadSettings),
        3 => Box::new(ShowKeyboardShortcuts),
        4 => Box::new(RevertAppearanceSettings),
        5 => Box::new(UndoSettingsSave),
        _ => return None,
    })
}

#[derive(Debug, PartialEq, Eq)]
pub enum ControlInput {
    Adjust(i32),
    Activate,
    None,
}

/// Arrow keys change values; they must never open a dialog or an editor.
pub fn control_input(control: &str, key: &str) -> ControlInput {
    let action = matches!(control, "file_editor" | "editor_example" | "editor_fallback" | "import_theme" | "font_family" | "shell_history")
        || FOOTER_KEYS.contains(&control);
    match (action, key) {
        (true, "enter" | "space") => ControlInput::Activate,
        (false, "left") => ControlInput::Adjust(-1),
        (false, "right" | "enter" | "space") => ControlInput::Adjust(1),
        _ => ControlInput::None,
    }
}
// The heading precedes the settings. Save/editor controls live in a fixed footer.
pub fn scroll_index(control: usize) -> usize {
    control + 1
}
pub fn next_control(selected: usize, key: &str, shift: bool) -> Option<usize> {
    if shift && key != "tab" { return None; }
    let count = KEYS.len() + FOOTER_COUNT;
    match key {
        "up" => Some((selected + count - 1) % count),
        "down" => Some((selected + 1) % count),
        "tab" => Some((selected + if shift { count - 1 } else { 1 }) % count),
        "home" => Some(0),
        "end" => Some(count - 1),
        _ => None,
    }
}


pub fn adjust(settings: &mut Preferences, key: &str, delta: i32) {
    match key {
        "opacity" => settings.background_opacity = (settings.background_opacity as i32 + 5 * delta.signum()).clamp(0, 100) as u8,
        "selection_opacity" => {
            settings.selection_opacity = if delta == 0 { None } else {
                let current = settings.selection_opacity.unwrap_or(if settings.selection_background().is_some() { 100 } else { 50 });
                Some((i32::from(current) + 5 * delta.signum()).clamp(0, 100) as u8)
            };
        }
        "cursor_opacity" => settings.cursor_opacity = (settings.cursor_opacity as i32 + 5 * delta.signum()).clamp(0, 100) as u8,
        "blur" => settings.background_blur = !settings.background_blur,
        "option_as_alt" => settings.macos_option_as_alt = !settings.macos_option_as_alt,
        "font" => settings.font_size = (settings.font_size + delta as f32).clamp(8., 40.),
        "padding" => settings.terminal_padding = (settings.terminal_padding as i32 + delta).clamp(0, 32) as u8,
        "scrollback" => settings.scrollback_mib = (settings.scrollback_mib as i32 + delta.signum()).clamp(0, 256) as u16,
        "theme" => settings.select_theme(settings.terminal_theme.cycle(delta)),
        "focused_cursor" => settings.focused_cursor = settings.focused_cursor.cycle(delta < 0),
        "close" if !settings.close_policy_overridden() => {
            use close_policy::ClosePolicy::{Active, Always, Never};
            settings.confirm_close = match (settings.confirm_close, delta < 0) {
                (Active, false) | (Never, true) => Always,
                (Always, false) | (Active, true) => Never,
                _ => Active,
            };
        }
        "group_placement" | "workspace_placement" => {
            use workspace_groups::Placement::{AfterCurrent, Top, End};
            let placement = if key == "group_placement" { &mut settings.group_new_workspace_placement } else { &mut settings.new_workspace_placement };
            *placement = match (*placement, delta < 0) {
                (AfterCurrent, false) | (End, true) => Top,
                (Top, false) | (AfterCurrent, true) => End,
                _ => AfterCurrent,
            };
        }
        "notification_focus" => settings.notification_focus = settings.notification_focus.toggle(),
        "native_notifications" => settings.native_notifications = !settings.native_notifications,
        "copy" => settings.copy_on_select = !settings.copy_on_select,
        "cursor" => settings.unfocused_cursor = match settings.unfocused_cursor { preferences::UnfocusedCursor::Hollow => preferences::UnfocusedCursor::Hidden, preferences::UnfocusedCursor::Hidden => preferences::UnfocusedCursor::Hollow },
        _ => {}
    }
}
fn button(id: String, label: String, hint: String, key: &'static str, delta: i32) -> impl IntoElement {
    let hint: SharedString = hint.into();
    div().id(SharedString::from(id)).min_w_0().max_w_full().px_2().py_1().rounded_sm().cursor_pointer()
        .bg(crate::chrome_style::hover()).hover(|s| s.bg(crate::chrome_style::hover_strong())).text_size(px(12.))
        .on_click(move |event, window, cx| { if event.standard_click() { window.dispatch_action(Box::new(AdjustSetting { key: key.into(), delta }), cx); } })
        .tooltip(move |_, cx| cx.new(|_| pane_view::ControlHint(hint.clone())).into())
        .child(div().truncate().child(label))
}
fn row(label: &'static str, key: &'static str, value: String, stepper: bool, selected: bool, readonly: bool) -> impl IntoElement {
    div().id(SharedString::from(format!("setting-row-{key}"))).min_w_0().px_1().py_3()
        .on_mouse_down(MouseButton::Left, move |_, window, cx| window.dispatch_action(Box::new(SelectSetting { key: key.into() }), cx))
        .when(selected, |s| s.bg(crate::chrome_style::selected())).border_b_1().border_color(crate::chrome_style::border())
        .flex().flex_wrap().items_center().gap_2()
        .child(div().flex_1().w(px(120.)).min_w_0().max_w_full().text_size(px(13.)).child(label))
        .child(div().flex().flex_wrap().items_center().gap_2().min_w_0().max_w_full()
            .when(stepper, |controls| controls
                .child(button(format!("{key}-less"), "−".into(), format!("Decrease {label}"), key, -1)))
            .child(if readonly {
                div().id(SharedString::from(format!("{key}-readonly")))
                    .px_2().py_1().text_size(px(12.)).text_color(crate::chrome_style::muted())
                    .tooltip(|_, cx| cx.new(|_| pane_view::ControlHint("Controlled by RMUX_CONFIRM_CLOSE. Remove the override and restart rmux to edit this policy here.".into())).into())
                    .child(value).into_any_element()
            } else if stepper {
                div().min_w(px(44.)).text_center().text_size(px(12.)).child(value).into_any_element()
            } else {
                button(format!("{key}-value"), value, format!("Change {label}"), key, 1).into_any_element()
            })
            .when(stepper, |controls| controls
                .child(button(format!("{key}-more"), "+".into(), format!("Increase {label}"), key, 1))))
}
// Configuration-only preview: never creates a terminal or reads a running session.
fn theme_preview(settings: &Preferences) -> impl IntoElement {
    use rmux_terminal::{Color, DEFAULT_BACKGROUND, DEFAULT_CURSOR, DEFAULT_FOREGROUND};
    let rgb_color = |Color(r, g, b): Color| rgb((u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b));
    let colors = settings.terminal_colors();
    let overridden = settings.foreground_color.is_some() || settings.background_color.is_some()
        || settings.cursor_color.is_some() || settings.cursor_text.is_some()
        || settings.selection_background.is_some() || settings.selection_foreground.is_some()
        || settings.ansi_palette.is_some();
    div().mt_2().min_w_0().flex().flex_col().gap_2()
        .child(div().p_2().rounded_sm().overflow_hidden()
            .bg(rgb_color(colors[1].unwrap_or(DEFAULT_BACKGROUND)))
            .text_color(rgb_color(colors[0].unwrap_or(DEFAULT_FOREGROUND)))
            .font_family(settings.font_family.clone()).text_size(px(12.))
            .child(div().flex().items_center().line_height(px(16.))
                .child(div().min_w_0().truncate().child("❯ echo hell"))
                .child(div().id("theme-cursor-sample").w(px(8.)).h(px(16.)).flex_shrink_0().text_center()
                    .bg(settings.cursor_paint(settings.cursor_fill(colors[0].unwrap_or(DEFAULT_FOREGROUND), colors[1].unwrap_or(DEFAULT_BACKGROUND), colors[2].unwrap_or(DEFAULT_CURSOR), false)))
                    .text_color(rgb_color(settings.cursor_text(colors[0].unwrap_or(DEFAULT_FOREGROUND), colors[1].unwrap_or(DEFAULT_BACKGROUND))))
                    .tooltip(|_, cx| cx.new(|_| pane_view::ControlHint("Block cursor text".into())).into())
                    .child("o")))
            .child(div().mt_1().flex().min_w_0()
                .child(div().min_w_0().truncate().px_1().line_height(px(16.))
                    .bg(settings.selection_background_fill(colors[0].unwrap_or(DEFAULT_FOREGROUND), colors[1].unwrap_or(DEFAULT_BACKGROUND)))
                    .text_color(rgb_color(settings.selection_foreground().map(|setting| setting.resolve(colors[0].unwrap_or(DEFAULT_FOREGROUND), colors[1].unwrap_or(DEFAULT_BACKGROUND))).unwrap_or(colors[0].unwrap_or(DEFAULT_FOREGROUND))))
                    .child("Selected text")))
            .when_some(settings.terminal_palette(), |preview, palette| preview
                .child(div().mt_2().flex().h(px(8.)).children(palette.into_iter().map(|color|
                    div().flex_1().h_full().bg(rgb_color(color)))))))
        .when_some(settings.theme_file.as_ref(), |preview, path| {
            let hint: SharedString = path.display().to_string().into();
            preview.child(div().id("imported-theme-path").min_w_0().truncate().text_xs().text_color(crate::chrome_style::muted())
                .tooltip(move |_, cx| cx.new(|_| pane_view::ControlHint(hint.clone())).into())
                .child(format!("Imported: {}", path.file_name().unwrap_or_default().to_string_lossy())))
        })
        .child(div().text_xs().text_color(crate::chrome_style::muted()).child(if overridden {
            "Preview includes your configured color overrides."
        } else { "Preview of configured colors. Apps may override them." }))
}

pub fn panel(state: State, overlay: bool, window: &Window, cx: &mut Context<window_view::WorkspaceWindow>) -> AnyElement {
    let settings = cx.global::<Preferences>();
    let busy = cx.global::<preferences::ReloadState>().0;
    let unsaved = cx.global::<preferences::AppearanceBaseline>().has_changes(settings);
    let save_label = if busy { "Working…".to_owned() } else {
        shortcuts::hint(&SaveAppearanceSettings, &state.focus, window)
            .map(|key| format!("Save · {key}"))
            .unwrap_or_else(|| "Save".into())
    };
    chrome_style::inspector(overlay).track_focus(&state.focus).key_context("Settings")
        .on_mouse_down(MouseButton::Left, { let focus = state.focus.clone(); move |_, window, _| { focus.focus(window); } })
        .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| this.settings_key(event, window, cx)))
        .child(div().h(px(44.)).px_3().flex().items_center().gap_2().flex_shrink_0()
            .child(div().flex_1().text_size(px(13.)).font_weight(FontWeight::SEMIBOLD).child("Settings"))
            .child(pane_view::pane_control("close-settings", "Close Settings")
                .on_click(|event, window, cx| { if event.standard_click() { window.dispatch_action(Box::new(ToggleSettings), cx); } }).child(icons::icon("close"))))
        .child(div().id("settings-scroll").flex_1().min_h_0().overflow_y_scroll().track_scroll(&state.scroll).px_3()
            .child(div().pt_3().text_xs().text_color(crate::chrome_style::muted()).child("Changes apply now. Save to keep them. Settings marked below require a restart."))
            .child(row("Default text size", "font", format!("{} pt", settings.font_size), true, state.selected == 0, false))
            .child(row("Terminal padding", "padding", format!("{} px", settings.terminal_padding), true, state.selected == 1, false))
            .child(div().id("setting-row-theme").px_1().py_3().border_b_1().border_color(crate::chrome_style::border())
                .on_mouse_down(MouseButton::Left, |_, window, cx| window.dispatch_action(Box::new(SelectSetting { key: "theme".into() }), cx))
                .when(state.selected == 2, |row| row.bg(crate::chrome_style::selected()))
                .child(div().mb_2().text_size(px(13.)).child("App & terminal theme"))
                .child(div().flex().flex_wrap().gap_2().children(
                    preferences::TerminalTheme::ALL.into_iter().enumerate().map(|(index, theme)| {
                        let selected = settings.theme_file.is_none() && settings.terminal_theme == theme;
                        div().id(("theme-choice", index)).px_2().py_1().rounded_sm().cursor_pointer()
                            .text_size(px(12.)).border_1()
                            .border_color(if selected { crate::chrome_style::accent() } else { crate::chrome_style::hover() })
                            .bg(if selected { crate::chrome_style::selected() } else { crate::chrome_style::elevated() })
                            .hover(|style| style.bg(crate::chrome_style::selected_hover()))
                            .on_click(move |event, window, cx| {
                                if event.standard_click() { window.dispatch_action(Box::new(SetTerminalTheme { theme }), cx); }
                            }).child(theme.label())
                    })
                ))
                .child(theme_preview(settings)))
            .child(row("Focused cursor", "focused_cursor", settings.focused_cursor.label().into(), false, state.selected == 3, false))
            .child(row("Cursor opacity", "cursor_opacity", format!("{}%", settings.cursor_opacity), true, state.selected == 4, false))
            .child(row("Unfocused cursor", "cursor", match settings.unfocused_cursor { preferences::UnfocusedCursor::Hollow => "Hollow", preferences::UnfocusedCursor::Hidden => "Hidden" }.into(), false, state.selected == 5, false))
            .child(row("Copy on select", "copy", if settings.copy_on_select { "On" } else { "Off" }.into(), false, state.selected == 6, false))
            .child(row("Confirm before closing", "close", format!("{}{}", match settings.confirm_close {
                close_policy::ClosePolicy::Active => "Active tasks", close_policy::ClosePolicy::Always => "Always", close_policy::ClosePolicy::Never => "Never",
            }, if settings.close_policy_overridden() { " · Environment" } else { "" }), false, state.selected == 7, settings.close_policy_overridden()))
            .child(row("New group workspace", "group_placement", match settings.group_new_workspace_placement {
                workspace_groups::Placement::AfterCurrent => "After current",
                workspace_groups::Placement::Top => "First",
                workspace_groups::Placement::End => "Last",
            }.into(), false, state.selected == 8, false))
            .child(div()
                .child(row("New terminal scrollback", "scrollback", format!("{} MiB", settings.scrollback_mib), true, state.selected == 9, false))
                .child(div().pt_2().text_xs().text_color(crate::chrome_style::muted()).child("Output history budget per new terminal. 0 disables scrollback. Existing terminals keep their current history; total app memory also includes other resources.")))
            .child(div().child(row("Desktop notifications", "native_notifications", if settings.native_notifications { "On" } else { "Off" }.into(), false, state.selected == 10, false))
                .child(div().pt_2().text_xs().text_color(crate::chrome_style::muted()).child("Optional OS banners for unfocused terminals. macOS asks for permission on first delivery. In-app history stays available when off.")))
            .child(row("Show desktop banners", "notification_focus", settings.notification_focus.label().into(), false, state.selected == 11, false))
            .child(row("Default font family", "font_family", settings.font_family.clone(), false, state.selected == 12, false))
            .child(row("Terminal background opacity", "opacity", format!("{}%", settings.background_opacity), true, state.selected == 13, false))
            .child(div().child(row("Blur behind terminal", "blur", if settings.background_blur { "On" } else { "Off" }.into(), false, state.selected == 14, false))
                .child(div().pt_2().text_xs().text_color(crate::chrome_style::muted()).child("Blur requires opacity below 100% and OS/compositor support. Text, colored cells, tabs and sidebar remain opaque.")))
            .child(row("New workspace position", "workspace_placement", match settings.new_workspace_placement {
                workspace_groups::Placement::AfterCurrent => "After current",
                workspace_groups::Placement::Top => "First",
                workspace_groups::Placement::End => "Last",
            }.into(), false, state.selected == 15, false))
            .when(cfg!(target_os = "macos"), |panel| panel.child(div()
                .child(row("Option key behavior", "option_as_alt", if settings.macos_option_as_alt { "Terminal Alt" } else { "Native text" }.into(), false, state.selected == 16, false))
                .child(div().pt_2().text_xs().text_color(crate::chrome_style::muted()).child("Terminal Alt sends Option shortcuts to your shell. Native text allows Option characters and dead keys. Control+Option and navigation keys still use terminal encoding."))))
            .child(div().child(row("Selection opacity", "selection_opacity", settings.selection_opacity.map_or_else(|| "Theme default".into(), |value| format!("{value}%")), true, Some(state.selected) == control_index("selection_opacity"), false))
                .child(button("selection-opacity-reset".into(), "Use theme default".into(), "Reset selection opacity to the theme default".into(), "selection_opacity", 0)))
            .child(row("Import terminal theme", "import_theme", if busy { "Working…" } else { "Choose file…" }.into(), false, Some(state.selected) == control_index("import_theme"), false))
            .child(div()
                .child(row("External editor", "file_editor", "Configure…".into(), false, Some(state.selected) == control_index("file_editor"), false))
                .child(div().pt_2().text_xs().text_color(crate::chrome_style::muted()).child("Opens Settings JSON. Set file_editor.program and args, then Reload Settings."))
                .child(div().id("settings-editor-program").pt_1().text_xs().min_w_0().truncate().text_color(crate::chrome_style::muted())
                    .tooltip({ let program: SharedString = settings.file_editor.as_ref().map(|editor| editor.program.display().to_string()).unwrap_or_else(|| "No external editor configured".into()).into();
                        move |_, cx| cx.new(|_| pane_view::ControlHint(program.clone())).into() })
                    .child(settings.file_editor.as_ref().map(|editor| editor.program.display().to_string()).unwrap_or_else(|| "Not configured".into()))))
            .child(div()
                .child(row("VS Code configuration", "editor_example", "Copy example".into(), false, Some(state.selected) == control_index("editor_example"), false))
                .child(div().pt_2().text_xs().text_color(crate::chrome_style::muted()).child("Merge file_editor into Settings JSON. Adjust the executable path for your installation, then Reload Settings.")))
            .child(div()
                .child(row("Edit settings without configured editor", "editor_fallback", "Open…".into(), false, Some(state.selected) == control_index("editor_fallback"), false))
                .child(div().pt_2().text_xs().text_color(crate::chrome_style::muted()).child(if cfg!(target_os = "macos") { "Opens TextEdit if your editor configuration needs repair." } else { "Uses the system file association if your editor configuration needs repair." })))
            .child(div()
                .child(row("Saved command history", "shell_history", "Configure…".into(), false,
                    Some(state.selected) == control_index("shell_history"), false))
                .child(div().pt_2().text_xs().text_color(crate::chrome_style::muted())
                    .child(format!("{} · Experimental", shell_history::settings_status(&settings.shell_history_directory))))
                .child(div().pt_2().text_xs().text_color(crate::chrome_style::muted())
                    .child("Recall commands when restoring a saved session. Supports zsh and non-login Bash. Set shell_history_directory in Settings JSON, then restart rmux."))
                .when_some(settings.shell_history_directory.as_ref(), |row, path| {
                    let path: SharedString = path.display().to_string().into();
                    row.child(div().id("settings-history-directory").pt_1().text_xs().min_w_0().truncate().text_color(crate::chrome_style::muted())
                        .tooltip({ let path = path.clone(); move |_, cx| cx.new(|_| pane_view::ControlHint(path.clone())).into() })
                        .child(path))
                }))
            .child(div().py_3().text_xs().text_color(crate::chrome_style::muted()).child("Font changes apply to terminals using the default size. Per-terminal zoom stays unchanged. Custom colors in your config override the theme.")))
        .child(div().flex_shrink_0().px_3().py_2().border_t_1().border_color(crate::chrome_style::border()).flex().flex_col().gap_2()
            .child(div().text_xs().text_color(if unsaved { crate::chrome_style::warning() } else { crate::chrome_style::muted() })
                .child(if unsaved { "Unsaved changes" } else { "No unsaved changes" }))
            .child(div().text_xs().text_color(crate::chrome_style::muted()).child("↑↓ select · ←→ change · Enter apply · Esc close"))
            .children([
                [
                    (0, "settings-save", save_label.as_str(), "Save current appearance settings to the settings file"),
                    (1, "settings-json", "Open JSON", "Open settings file in your configured editor"),
                    (2, "settings-reload", "Reload", "Replace this session's settings with the saved file; invalid files leave settings unchanged"),
                ],
                [
                    (3, "settings-shortcuts", "Shortcuts…", "Search command shortcuts available in the current context"),
                    (4, "settings-revert", "Revert", "Restore the last loaded or saved panel values without changing the settings file"),
                    (5, "settings-undo-save", "Undo save", "Restore saved values only where the file still matches the last save; preserve later edits"),
                ],
            ].into_iter().map(|row| div().flex().gap_2().children(row.into_iter().map(|(index, id, label, hint)| {
                let enabled = footer_enabled(index, busy, unsaved,
                    cx.global::<preferences::LastSettingsSave>().0.is_some());
                div().id(id).flex_1().min_w_0().h(px(30.)).flex().items_center().justify_center().px_2().rounded_sm().text_xs().border_1()
                    .when(index == 0, |button| button.bg(if busy { crate::chrome_style::hover() } else { crate::chrome_style::selected() }))
                    .border_color(if state.selected == KEYS.len() + index { crate::chrome_style::accent() } else { crate::chrome_style::hover() })
                    .text_color(if enabled { crate::chrome_style::accent() } else { crate::chrome_style::muted() })
                    .when(enabled, |link| link.cursor_pointer().hover(|style| style.bg(crate::chrome_style::selected_hover())))
                    .tooltip(move |_, cx| cx.new(|_| pane_view::ControlHint(hint.into())).into())
                    .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                        if enabled { window.dispatch_action(Box::new(SelectSetting { key: FOOTER_KEYS[index].into() }), cx); }
                    })
                    .on_click(move |event, window, cx| {
                        if enabled && event.standard_click() {
                            if let Some(action) = footer_action(index) { window.dispatch_action(action, cx); }
                        }
                    }).child(div().min_w_0().truncate().child(label.to_owned()))
            })))))
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::{adjust, next_control, Preferences, close_policy, workspace_groups, KEYS, FOOTER_COUNT};
    #[test]
    fn arrows_adjust_values_but_never_launch_actions() {
        use super::{control_input, ControlInput};
        for control in ["file_editor", "editor_example", "editor_fallback", "import_theme", "font_family", "shell_history"].into_iter().chain(super::FOOTER_KEYS.iter().copied()) {
            assert_eq!(control_input(control, "left"), ControlInput::None);
            assert_eq!(control_input(control, "right"), ControlInput::None);
            assert_eq!(control_input(control, "enter"), ControlInput::Activate);
            assert_eq!(control_input(control, "space"), ControlInput::Activate);
        }
        assert_eq!(control_input("font", "left"), ControlInput::Adjust(-1));
        assert_eq!(control_input("font", "right"), ControlInput::Adjust(1));
        assert_eq!(control_input("copy", "space"), ControlInput::Adjust(1));
    }
    #[test]
    fn keyboard_navigation_wraps_and_adjustments_stay_in_bounds() {
        assert_eq!(next_control(0, "tab", true), Some(KEYS.len() + FOOTER_COUNT - 1));
        assert_eq!(next_control(KEYS.len() + FOOTER_COUNT - 1, "down", false), Some(0));
        assert_eq!(next_control(3, "end", false), Some(KEYS.len() + FOOTER_COUNT - 1));
        assert_eq!(next_control(3, "a", false), None);
        let original = Preferences::default();
        let mut option = original.clone();
        adjust(&mut option, "option_as_alt", 1);
        assert!(!option.macos_option_as_alt);
        assert!(!option.same_saved_appearance(&original));
        adjust(&mut option, "option_as_alt", -1);
        assert!(option.same_saved_appearance(&original));
        let mut prefs = Preferences::default();
        for _ in 0..100 { adjust(&mut prefs, "font", 1); adjust(&mut prefs, "padding", -1); }
        adjust(&mut prefs, "close", 1);
        assert_eq!(prefs.confirm_close, close_policy::ClosePolicy::Always);
        adjust(&mut prefs, "close", -1);
        assert_eq!(prefs.confirm_close, close_policy::ClosePolicy::Active);
        adjust(&mut prefs, "close", -1);
        assert_eq!(prefs.confirm_close, close_policy::ClosePolicy::Never);
        adjust(&mut prefs, "group_placement", 1);
        assert_eq!(prefs.group_new_workspace_placement, workspace_groups::Placement::Top);
        adjust(&mut prefs, "group_placement", -1);
        assert_eq!(prefs.group_new_workspace_placement, workspace_groups::Placement::AfterCurrent);
        adjust(&mut prefs, "group_placement", -1);
        assert_eq!(prefs.group_new_workspace_placement, workspace_groups::Placement::End);
        assert_eq!(prefs.font_size, 40.);
        assert_eq!(prefs.terminal_padding, 0);
        for _ in 0..300 { adjust(&mut prefs, "scrollback", -1); }
        assert_eq!(prefs.scrollback_mib, 0);
        for _ in 0..300 { adjust(&mut prefs, "scrollback", 1); }
        assert_eq!(prefs.scrollback_mib, 256);
    }
}
