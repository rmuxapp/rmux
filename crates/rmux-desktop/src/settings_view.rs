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
    "selection_opacity", "import_theme", "file_editor",
];
pub const FOOTER_KEYS: &[&str] = &["save_settings", "open_settings", "reload_settings", "keyboard_shortcuts"];
pub const FOOTER_COUNT: usize = FOOTER_KEYS.len();
pub fn control_index(key: &str) -> Option<usize> {
    KEYS.iter().chain(FOOTER_KEYS).position(|candidate| *candidate == key)
}
pub fn footer_action(index: usize) -> Option<Box<dyn Action>> {
    Some(match index {
        0 => Box::new(SaveAppearanceSettings),
        1 => Box::new(OpenSettingsFile),
        2 => Box::new(ReloadSettings),
        3 => Box::new(ShowKeyboardShortcuts),
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
    let action = matches!(control, "file_editor" | "import_theme" | "font_family")
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
        .bg(rgb(0x353535)).hover(|s| s.bg(rgb(0x454545))).text_size(px(12.))
        .on_click(move |event, window, cx| { if event.standard_click() { window.dispatch_action(Box::new(AdjustSetting { key: key.into(), delta }), cx); } })
        .tooltip(move |_, cx| cx.new(|_| pane_view::ControlHint(hint.clone())).into())
        .child(div().truncate().child(label))
}
fn row(label: &'static str, key: &'static str, value: String, stepper: bool, selected: bool, readonly: bool) -> impl IntoElement {
    div().id(SharedString::from(format!("setting-row-{key}"))).min_w_0().px_1().py_3()
        .on_mouse_down(MouseButton::Left, move |_, window, cx| window.dispatch_action(Box::new(SelectSetting { key: key.into() }), cx))
        .when(selected, |s| s.bg(rgb(chrome_style::SELECTED))).border_b_1().border_color(rgb(0x383838))
        .flex().flex_wrap().items_center().gap_2()
        .child(div().flex_1().w(px(120.)).min_w_0().max_w_full().text_size(px(13.)).child(label))
        .child(div().flex().flex_wrap().items_center().gap_2().min_w_0().max_w_full()
            .when(stepper, |controls| controls
                .child(button(format!("{key}-less"), "−".into(), format!("Decrease {label}"), key, -1)))
            .child(if readonly {
                div().id(SharedString::from(format!("{key}-readonly")))
                    .px_2().py_1().text_size(px(12.)).text_color(rgb(0xa0a0a0))
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
            preview.child(div().id("imported-theme-path").min_w_0().truncate().text_xs().text_color(rgb(0xa0a0a0))
                .tooltip(move |_, cx| cx.new(|_| pane_view::ControlHint(hint.clone())).into())
                .child(format!("Imported: {}", path.file_name().unwrap_or_default().to_string_lossy())))
        })
        .child(div().text_xs().text_color(rgb(0xa0a0a0)).child(if overridden {
            "Preview includes your configured color overrides."
        } else { "Preview of configured colors. Apps may override them." }))
}

pub fn panel(state: State, window: &Window, cx: &mut Context<window_view::WorkspaceWindow>) -> AnyElement {
    let settings = cx.global::<Preferences>();
    let busy = cx.global::<preferences::ReloadState>().0;
    let unsaved = cx.global::<preferences::AppearanceBaseline>().has_changes(settings);
    let save_label = if busy { "Working with settings…".to_owned() } else {
        shortcuts::hint(&SaveAppearanceSettings, &state.focus, window)
            .map(|key| format!("Save these settings · {key}"))
            .unwrap_or_else(|| "Save these settings".into())
    };
    div().track_focus(&state.focus).key_context("Settings")
        .on_mouse_down(MouseButton::Left, { let focus = state.focus.clone(); move |_, window, _| { focus.focus(window); } })
        .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| this.settings_key(event, window, cx)))
        .w(px(380.)).max_w(relative(0.50)).h_full().flex_shrink_0().flex().flex_col()
        .bg(rgb(0x242424)).border_l_1().border_color(rgb(0x383838))
        .child(div().h(px(44.)).px_3().flex().items_center().gap_2().flex_shrink_0()
            .child(div().flex_1().text_size(px(13.)).font_weight(FontWeight::SEMIBOLD).child("Settings"))
            .child(pane_view::pane_control("close-settings", "Close Settings")
                .on_click(|event, window, cx| { if event.standard_click() { window.dispatch_action(Box::new(ToggleSettings), cx); } }).child(icons::icon("close"))))
        .child(div().id("settings-scroll").flex_1().min_h_0().overflow_y_scroll().track_scroll(&state.scroll).px_3()
            .child(div().pt_3().text_xs().text_color(rgb(0xa0a0a0)).child("Changes apply immediately to this session. Save to keep them for next time."))
            .child(row("Default text size", "font", format!("{} pt", settings.font_size), true, state.selected == 0, false))
            .child(row("Terminal padding", "padding", format!("{} px", settings.terminal_padding), true, state.selected == 1, false))
            .child(div().id("setting-row-theme").px_1().py_3().border_b_1().border_color(rgb(0x383838))
                .on_mouse_down(MouseButton::Left, |_, window, cx| window.dispatch_action(Box::new(SelectSetting { key: "theme".into() }), cx))
                .when(state.selected == 2, |row| row.bg(rgb(chrome_style::SELECTED)))
                .child(div().mb_2().text_size(px(13.)).child("Terminal theme"))
                .child(div().flex().flex_wrap().gap_2().children(
                    preferences::TerminalTheme::ALL.into_iter().enumerate().map(|(index, theme)| {
                        let selected = settings.theme_file.is_none() && settings.terminal_theme == theme;
                        div().id(("theme-choice", index)).px_2().py_1().rounded_sm().cursor_pointer()
                            .text_size(px(12.)).border_1()
                            .border_color(rgb(if selected { chrome_style::ACCENT } else { 0x454545 }))
                            .bg(rgb(if selected { crate::chrome_style::SELECTED } else { 0x292929 }))
                            .hover(|style| style.bg(rgb(chrome_style::SELECTED_HOVER)))
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
                .child(row("New terminal history", "scrollback", format!("{} MiB", settings.scrollback_mib), true, state.selected == 9, false))
                .child(div().pt_2().text_xs().text_color(rgb(0xa0a0a0)).child("History budget per new terminal. 0 disables scrollback. Existing terminals keep their current history; total app memory also includes other resources.")))
            .child(div().child(row("Desktop notifications", "native_notifications", if settings.native_notifications { "On" } else { "Off" }.into(), false, state.selected == 10, false))
                .child(div().pt_2().text_xs().text_color(rgb(0xa0a0a0)).child("Optional OS banners for unfocused terminals. macOS asks for permission on first delivery. In-app history stays available when off.")))
            .child(row("Show desktop banners", "notification_focus", settings.notification_focus.label().into(), false, state.selected == 11, false))
            .child(row("Default font family", "font_family", settings.font_family.clone(), false, state.selected == 12, false))
            .child(row("Terminal background opacity", "opacity", format!("{}%", settings.background_opacity), true, state.selected == 13, false))
            .child(div().child(row("Blur behind terminal", "blur", if settings.background_blur { "On" } else { "Off" }.into(), false, state.selected == 14, false))
                .child(div().pt_2().text_xs().text_color(rgb(0xa0a0a0)).child("Blur requires opacity below 100% and OS/compositor support. Text, colored cells, tabs and sidebar remain opaque.")))
            .child(row("New workspace position", "workspace_placement", match settings.new_workspace_placement {
                workspace_groups::Placement::AfterCurrent => "After current",
                workspace_groups::Placement::Top => "First",
                workspace_groups::Placement::End => "Last",
            }.into(), false, state.selected == 15, false))
            .when(cfg!(target_os = "macos"), |panel| panel.child(div()
                .child(row("Option key behavior", "option_as_alt", if settings.macos_option_as_alt { "Terminal Alt" } else { "Native text" }.into(), false, state.selected == 16, false))
                .child(div().pt_2().text_xs().text_color(rgb(0xa0a0a0)).child("Terminal Alt sends Option shortcuts to your shell. Native text allows Option characters and dead keys. Control+Option and navigation keys still use terminal encoding."))))
            .child(div().child(row("Selection opacity", "selection_opacity", settings.selection_opacity.map_or_else(|| "Theme default".into(), |value| format!("{value}%")), true, state.selected == KEYS.len() - 3, false))
                .child(button("selection-opacity-reset".into(), "Use theme default".into(), "Reset selection opacity to the theme default".into(), "selection_opacity", 0)))
            .child(row("Import terminal theme", "import_theme", if busy { "Working…" } else { "Choose file…" }.into(), false, state.selected == KEYS.len() - 2, false))
            .child(div()
                .child(row("External editor", "file_editor", "Configure…".into(), false, state.selected == KEYS.len() - 1, false))
                .child(div().pt_2().text_xs().text_color(rgb(0xa0a0a0)).child("Opens Settings JSON. Set file_editor.program and args, then Reload Settings."))
                .child(div().id("settings-editor-program").pt_1().text_xs().min_w_0().truncate().text_color(rgb(0xb8b8b8))
                    .tooltip({ let program: SharedString = settings.file_editor.as_ref().map(|editor| editor.program.display().to_string()).unwrap_or_else(|| "No external editor configured".into()).into();
                        move |_, cx| cx.new(|_| pane_view::ControlHint(program.clone())).into() })
                    .child(settings.file_editor.as_ref().map(|editor| editor.program.display().to_string()).unwrap_or_else(|| "Not configured".into()))))
            .child(div().py_3().text_xs().text_color(rgb(0xa0a0a0)).child("Font changes apply to terminals using the default size. Per-terminal zoom stays unchanged. Custom colors in your config override the theme.")))
        .child(div().flex_shrink_0().px_3().py_2().border_t_1().border_color(rgb(0x383838)).flex().flex_col().gap_2()
            .child(div().text_xs().text_color(rgb(if unsaved { 0xe3bc78 } else { 0xa0a0a0 }))
                .child(if unsaved { "Unsaved session changes" } else { "No unsaved session changes" }))
            .child(div().text_xs().text_color(rgb(0xa0a0a0)).child("↑↓ / Tab select · ←→ change · Enter activate · Esc close"))
            .child(div().id("settings-save").border_1().border_color(if state.selected == KEYS.len() { rgb(0xffffff) } else { rgb(chrome_style::SELECTED_BORDER) }).px_3().py_2().rounded_sm().text_sm()
                .bg(rgb(if busy { 0x353535 } else { chrome_style::SELECTED }))
                .when(!busy, |button| button.cursor_pointer().hover(|s| s.bg(rgb(chrome_style::SELECTED_HOVER))))
                .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                    if !busy { window.dispatch_action(Box::new(SelectSetting { key: FOOTER_KEYS[0].into() }), cx); }
                })
                .on_click(move |event, window, cx| { if !busy && event.standard_click() { window.dispatch_action(Box::new(SaveAppearanceSettings), cx); } }).child(save_label))
            .child(div().flex().flex_wrap().gap_2().children([
                (1, "settings-json", "Open settings file", "Edit all settings in your editor"),
                (2, "settings-reload", "Reload from file", "Replace this session's settings with the saved file; invalid files leave settings unchanged"),
                (3, "settings-shortcuts", "Keyboard Shortcuts…", "Search command shortcuts available in the current context"),
            ].into_iter().map(|(index, id, label, hint)| {
                div().id(id).px_2().py_1().rounded_sm().text_xs().border_1()
                    .border_color(rgb(if state.selected == KEYS.len() + index { chrome_style::ACCENT } else { 0x454545 }))
                    .text_color(rgb(if busy { 0x777777 } else { chrome_style::ACCENT }))
                    .when(!busy, |link| link.cursor_pointer().hover(|style| style.bg(rgb(chrome_style::SELECTED_HOVER))))
                    .tooltip(move |_, cx| cx.new(|_| pane_view::ControlHint(hint.into())).into())
                    .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                        if !busy { window.dispatch_action(Box::new(SelectSetting { key: FOOTER_KEYS[index].into() }), cx); }
                    })
                    .on_click(move |event, window, cx| {
                        if !busy && event.standard_click() {
                            if let Some(action) = footer_action(index) { window.dispatch_action(action, cx); }
                        }
                    }).child(label)
            }))))
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::{adjust, next_control, Preferences, close_policy, workspace_groups, KEYS, FOOTER_COUNT};
    #[test]
    fn arrows_adjust_values_but_never_launch_actions() {
        use super::{control_input, ControlInput};
        for control in ["file_editor", "import_theme", "font_family"].into_iter().chain(super::FOOTER_KEYS.iter().copied()) {
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
