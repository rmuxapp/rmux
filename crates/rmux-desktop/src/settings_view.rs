use super::*;

#[derive(Clone)]
pub struct State {
    pub editor: Option<settings_editor::Editor>,
    pub focus: FocusHandle,
    pub category: usize,
    pub origin_window: AnyWindowHandle,
    pub selected: usize,
    pub scroll: ScrollHandle,
    pub search: Entity<text_field::TextField>,
    pub query: String,
    pub visible: Vec<usize>,
    pub match_hints: std::collections::BTreeMap<usize, String>,
}

pub const CATEGORIES: &[(&str, &str, &[&str])] = &[
    ("Appearance", "Make rmux feel at home.", &["theme", "import_theme", "font_family", "font", "padding", "opacity", "blur", "selection_opacity", "foreground_color", "background_color", "selection_background", "selection_foreground", "ansi_palette"]),
    ("Terminal", "Text input, cursor behavior, and output history.", &["focused_cursor", "cursor", "cursor_blink", "cursor_opacity", "copy", #[cfg(target_os = "macos")] "option_as_alt", "scrollback", "cursor_color", "cursor_text"]),
    ("Workspaces", "Arrange your workspace and keep navigation comfortable.", &["workspace_placement", "group_placement", "close", "controls_on_hover", "hide_top_bar", "compact_sidebar", "layout_presets", "restore_session"]),
    ("Notifications", "Choose when rmux should get your attention.", &["native_notifications", "notification_focus"]),
    ("Advanced", "External tools and experimental session features.", &["file_editor", "editor_example", "editor_fallback", "shell_history", "commands", "keybindings", "session_file", "socket_directory"]),
];
#[cfg(test)]
pub fn visible_controls(category: usize) -> Vec<usize> {
    CATEGORIES[category].2.iter().filter_map(|key| control_index(key))
        .chain(FOOTER_ORDER.iter().map(|index| KEYS.len() + index)).collect()
}
pub fn next_visible(visible: &[usize], selected: usize, key: &str, shift: bool) -> Option<usize> {
    let controls: Vec<_> = visible.iter().copied().chain(FOOTER_ORDER.iter().map(|index| KEYS.len() + index)).collect();
    let position = controls.iter().position(|index| *index == selected).unwrap_or(0);
    let index = match key {
        "tab" if shift => (position + controls.len() - 1) % controls.len(),
        "up" => (position + controls.len() - 1) % controls.len(),
        "tab" | "down" => (position + 1) % controls.len(),
        "home" | "pageup" => 0,
        "end" | "pagedown" => controls.len() - 1,
        _ => return None,
    };
    Some(controls[index])
}

pub const KEYS: &[&str] = &["font", "padding", "theme", "focused_cursor", "cursor_opacity", "cursor", "copy", "close", "group_placement", "scrollback", "native_notifications", "notification_focus", "font_family", "opacity", "blur", "workspace_placement",
    #[cfg(target_os = "macos")] "option_as_alt",
    "selection_opacity", "import_theme", "file_editor", "editor_example", "editor_fallback", "shell_history", "cursor_blink", "controls_on_hover", "hide_top_bar", "compact_sidebar", "commands", "layout_presets", "keybindings", "restore_session", "session_file", "socket_directory", "foreground_color", "background_color", "cursor_color", "cursor_text", "selection_background", "selection_foreground", "ansi_palette",
];
pub const FOOTER_KEYS: &[&str] = &["save_settings", "open_settings", "reload_settings", "keyboard_shortcuts", "revert_settings", "undo_settings_save"];
pub const FOOTER_ORDER: &[usize] = &[1, 2, 3, 5, 4, 0];
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
        || FOOTER_KEYS.contains(&control) || super::settings_search::entry(control).json_only || settings_editor::field(control).is_some();
    match (action, key) {
        (true, "enter" | "space") => ControlInput::Activate,
        (false, "left") => ControlInput::Adjust(-1),
        (false, "right" | "enter" | "space") => ControlInput::Adjust(1),
        _ => ControlInput::None,
    }
}
pub fn adjust(settings: &mut Preferences, key: &str, delta: i32) {
    match key {
        "restore_session" => settings.restore_session = !settings.restore_session,
        "cursor_blink" => settings.cursor_blink = !settings.cursor_blink,
        "controls_on_hover" => settings.controls_on_hover = !settings.controls_on_hover,
        "hide_top_bar" => settings.hide_top_bar = !settings.hide_top_bar,
        "compact_sidebar" => settings.compact_sidebar = !settings.compact_sidebar,

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
/// Reset only the selected panel-controlled setting, preserving other drafts.
pub fn reset(settings: &mut Preferences, key: &str) -> bool {
    if let Some(field) = settings_editor::field(key).filter(|field| field.config != "font_family") {
        if let Ok(next) = settings_editor::apply(settings, field, "") { *settings = next; return true; }
        return false;
    }
    let defaults = Preferences::default();
    match key {
        "restore_session" => settings.restore_session = defaults.restore_session,
        "font" => settings.font_size = defaults.font_size,
        "font_family" => settings.font_family = defaults.font_family,
        "padding" => settings.terminal_padding = defaults.terminal_padding,
        "opacity" => settings.background_opacity = defaults.background_opacity,
        "blur" => settings.background_blur = defaults.background_blur,
        "selection_opacity" => settings.selection_opacity = defaults.selection_opacity,
        "cursor_opacity" => settings.cursor_opacity = defaults.cursor_opacity,
        "cursor" => settings.unfocused_cursor = defaults.unfocused_cursor,
        "focused_cursor" => settings.focused_cursor = defaults.focused_cursor,
        "cursor_blink" => settings.cursor_blink = defaults.cursor_blink,
        "controls_on_hover" => settings.controls_on_hover = defaults.controls_on_hover,
        "hide_top_bar" => settings.hide_top_bar = defaults.hide_top_bar,
        "compact_sidebar" => settings.compact_sidebar = defaults.compact_sidebar,
        "option_as_alt" => settings.macos_option_as_alt = defaults.macos_option_as_alt,
        "scrollback" => settings.scrollback_mib = defaults.scrollback_mib,
        "copy" => settings.copy_on_select = defaults.copy_on_select,
        "native_notifications" => settings.native_notifications = defaults.native_notifications,
        "notification_focus" => settings.notification_focus = defaults.notification_focus,
        "workspace_placement" => settings.new_workspace_placement = defaults.new_workspace_placement,
        "group_placement" => settings.group_new_workspace_placement = defaults.group_new_workspace_placement,
        "theme" => settings.select_theme(defaults.terminal_theme),
        "close" if !settings.close_policy_overridden() => settings.confirm_close = defaults.confirm_close,
        _ => return false,
    }
    true
}
fn resettable(key: &str) -> bool {
    settings_editor::field(key).is_some() || matches!(key, "restore_session" | "font" | "font_family" | "padding" | "opacity" | "blur" | "selection_opacity" | "cursor_opacity" | "cursor" | "focused_cursor" | "cursor_blink" | "controls_on_hover" | "hide_top_bar" | "compact_sidebar" | "option_as_alt" | "scrollback" | "copy" | "native_notifications" | "notification_focus" | "workspace_placement" | "group_placement" | "theme" | "close")
}
fn reset_button(key: &'static str) -> impl IntoElement {
    div().id(SharedString::from(format!("{key}-reset"))).size(px(24.)).flex_shrink_0()
        .flex().items_center().justify_center().rounded_sm().cursor_pointer()
        .hover(|style| style.bg(chrome_style::hover_strong()))
        .tooltip(|_, cx| cx.new(|_| pane_view::ControlHint("Reset this setting to its default · Backspace".into())).into())
        .on_click(move |event, window, cx| { if event.standard_click() { window.dispatch_action(Box::new(ResetSetting { key: key.into() }), cx); } })
        .child(icons::icon("refresh"))
}

fn button(id: String, label: String, hint: String, key: &'static str, delta: i32) -> impl IntoElement {
    let hint: SharedString = hint.into();
    div().id(SharedString::from(id)).min_w_0().max_w_full().px_2().py_1().rounded_sm().cursor_pointer()
        .bg(crate::chrome_style::hover()).hover(|s| s.bg(crate::chrome_style::hover_strong())).text_size(px(12.))
        .on_click(move |event, window, cx| { if event.standard_click() { window.dispatch_action(Box::new(AdjustSetting { key: key.into(), delta }), cx); } })
        .tooltip(move |_, cx| cx.new(|_| pane_view::ControlHint(hint.clone())).into())
        .child(div().truncate().child(label))
}
fn control_hint(key: &str) -> &'static str {
    match key {
        "editor_fallback" => if cfg!(target_os = "macos") { "Open Settings JSON in TextEdit to repair the editor configuration, then Reload Settings." } else { "Open Settings JSON with the system file association to repair the editor configuration, then Reload Settings." },
        "shell_history" => "Recall commands when restoring a saved session. Supports zsh and non-login Bash. Choose a history directory, save the setting, then restart rmux.",
        "editor_example" => "Merge file_editor into Settings JSON. Adjust the executable path for your installation, then Reload Settings.",
        "file_editor" => "Opens Settings JSON. Set file_editor.program and args, then Reload Settings.",
        "native_notifications" => "Optional OS banners for unfocused terminals. macOS asks for permission on first delivery. In-app history stays available when off.",
        "scrollback" => "Output history budget per new terminal. 0 disables scrollback. Existing terminals keep their current history; total app memory also includes other resources.",
        "option_as_alt" => "Terminal Alt sends Option shortcuts to your shell. Native text allows Option characters and dead keys. Control+Option and navigation keys still use terminal encoding.",
        "blur" => "Blur requires opacity below 100% and OS/compositor support. Text, colored cells, tabs and sidebar remain opaque.",
        _ => settings_search::entry(key).description,
    }
}

fn row(label: &'static str, key: &'static str, value: String, stepper: bool, selected: bool, readonly: bool) -> impl IntoElement {
    row_with_detail(label, key, value, stepper, selected, readonly,
        settings_search::entry(key).description.into(), control_hint(key).into())
}
fn row_with_detail(label: &'static str, key: &'static str, value: String, stepper: bool, selected: bool, readonly: bool, description: String, hint: SharedString) -> impl IntoElement {
    div().id(SharedString::from(format!("setting-row-{key}"))).min_w_0().px_3().py_2()
        .on_mouse_down(MouseButton::Left, move |_, window, cx| window.dispatch_action(Box::new(SelectSetting { key: key.into() }), cx))
        .when(selected, |s| s.bg(crate::chrome_style::selected())).border_b_1().border_color(crate::chrome_style::border())
        .flex().flex_wrap().items_center().gap_2()
        .child(div().id(SharedString::from(format!("{key}-label"))).tooltip(move |_, cx| cx.new(|_| pane_view::ControlHint(hint.clone())).into()).flex_1().w(px(120.)).min_w_0().max_w_full().text_size(px(13.)).child(label)
            .child(div().mt_1().text_size(px(11.)).text_color(chrome_style::muted()).child(description)))
        .child(div().flex().flex_wrap().items_center().gap_2().min_w_0().max_w_full()
            .when(stepper, |controls| controls
                .child(button(format!("{key}-less"), "−".into(), format!("Decrease {label}"), key, -1)))
            .child(if readonly {
                div().id(SharedString::from(format!("{key}-readonly")))
                    .px_2().py_1().text_size(px(12.)).text_color(crate::chrome_style::muted())
                    .tooltip(|_, cx| cx.new(|_| pane_view::ControlHint("Controlled by RMUX_CONFIRM_CLOSE. Remove the override and restart rmux to edit this policy here.".into())).into())
                    .child(value).into_any_element()
            } else if matches!(key, "restore_session" | "copy" | "native_notifications" | "blur" | "cursor_blink" | "controls_on_hover" | "hide_top_bar" | "compact_sidebar") {
                let on = value == "On";
                div().id(SharedString::from(format!("{key}-toggle"))).w(px(32.)).h(px(18.)).rounded_full().relative().cursor_pointer()
                    .bg(if on { chrome_style::accent() } else { chrome_style::hover_strong() })
                    .tooltip(move |_, cx| cx.new(|_| pane_view::ControlHint(format!("{label}: {}", if on { "On" } else { "Off" }).into())).into())
                    .on_click(move |event, window, cx| { if event.standard_click() { window.dispatch_action(Box::new(AdjustSetting { key: key.into(), delta: 1 }), cx); } })
                    .child(div().absolute().top(px(2.)).left(px(if on { 16. } else { 2. })).size(px(14.)).rounded_full().bg(if on { chrome_style::on_accent() } else { chrome_style::foreground() }))
                    .into_any_element()
            } else if stepper {
                div().min_w(px(44.)).text_center().text_size(px(12.)).child(value).into_any_element()
            } else {
                button(format!("{key}-value"), value, if settings_search::entry(key).json_only && settings_editor::field(key).is_none() {
                    format!("Edit {} in Settings JSON, then choose Reload to apply the change.", settings_search::entry(key).config)
                } else { format!("Change {label}") }, key, 1).into_any_element()
            })
            .when(stepper, |controls| controls
                .child(button(format!("{key}-more"), "+".into(), format!("Increase {label}"), key, 1)))
            .when(!readonly && resettable(key), |controls| controls.child(reset_button(key))))
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

pub fn panel(state: State, window: &Window, cx: &mut Context<window_view::WorkspaceWindow>) -> AnyElement {
    let settings = cx.global::<Preferences>();
    let busy = cx.global::<preferences::ReloadState>().0;
    let unsaved = cx.global::<preferences::AppearanceBaseline>().has_changes(settings);
    let save_label = if busy { "Working…".to_owned() } else {
        shortcuts::hint(&SaveAppearanceSettings, &state.focus, window)
            .map(|key| format!("Save · {key}"))
            .unwrap_or_else(|| "Save".into())
    };
    div().w_full().flex_1().min_h_0().flex().flex_col().bg(chrome_style::surface()).track_focus(&state.focus).key_context("Settings")
        .on_mouse_down(MouseButton::Left, { let focus = state.focus.clone(); move |_, window, _| { focus.focus(window); } })
        .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| this.settings_key(event, window, cx)))
        .child(div().flex().flex_1().min_h_0()
            .child(div().w(px(210.)).flex_shrink_0().p_3().bg(chrome_style::elevated())
                .border_r_1().border_color(chrome_style::border()).flex().flex_col().gap_1()
                .child(div().px_3().py_4().text_size(px(16.)).font_weight(FontWeight::SEMIBOLD).child("Settings"))
                .child(div().mb_3().px_2().py_1().rounded_md().border_1().border_color(chrome_style::border()).bg(chrome_style::background()).flex().items_center().gap_2()
                    .child(icons::icon("search"))
                    .child(div().flex_1().min_w_0().child(state.search.clone()))
                    .when(!state.query.is_empty(), |row| row.child(pane_view::pane_control("clear-settings-search", "Clear search")
                        .on_click({ let search = state.search.clone(); move |event, window, cx| { if event.standard_click() {
                            search.update(cx, |field, cx| field.replace_draft("", cx));
                            search.read(cx).focus.focus(window);
                        } } }).child(icons::icon("close")))))
                .children(CATEGORIES.iter().enumerate().map(|(index, (label, _, _))| {
                    div().id(("settings-category", index)).px_3().py_2().rounded_md().cursor_pointer()
                        .flex().items_center().gap_2().text_size(px(13.)).when(state.query.is_empty() && index == state.category, |row| row.bg(chrome_style::selected()).text_color(chrome_style::accent()).font_weight(FontWeight::SEMIBOLD))
                        .hover(|row| row.bg(chrome_style::hover()))
                        .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
                            if event.standard_click() { this.select_settings_category(index, window, cx); }
                        })).child(icons::icon(["palette", "terminal", "sidebar", "bell", "sliders"][index]).size(px(16.)).text_color(if state.query.is_empty() && index == state.category { chrome_style::accent() } else { chrome_style::muted() })).child(*label)
                }))
                .child(div().flex_1())
                .child(div().px_3().py_2().text_xs().text_color(chrome_style::muted()).child("Search across all settings.\nEnter to browse results.")))
        .child(div().id("settings-scroll").flex_1().min_w_0().min_h_0().overflow_y_scroll().track_scroll(&state.scroll).px_6().pb_4()
            .child(div().pt_4().pb_3().child(div().text_size(px(18.)).font_weight(FontWeight::SEMIBOLD).child(if state.query.is_empty() { CATEGORIES[state.category].0.to_owned() } else { "Search results".into() })).child(div().mt_2().min_w_0().truncate().text_size(px(12.)).text_color(chrome_style::muted()).child(if state.query.is_empty() { CATEGORIES[state.category].1.to_owned() } else { format!("{} {} matching “{}”", state.visible.len(), if state.visible.len() == 1 { "setting" } else { "settings" }, state.query) })))

            .when(!state.query.is_empty() && state.visible.is_empty(), |panel| panel.child(div().py_6().text_size(px(13.)).text_color(chrome_style::muted()).child("No matching settings. Try a shorter phrase, like font, opacity, or notifications.")))
            .children(state.visible.iter().map(|index| {
                let item = settings_search::entry(KEYS[*index]);
                div().when(!state.query.is_empty(), |row| row.child(div().px_3().pt_3().text_size(px(11.)).text_color(chrome_style::muted()).child(format!("{} · {}", CATEGORIES[item.category].0, state.match_hints.get(index).map(String::as_str).unwrap_or_default()))))
                    .child(setting_element(item.key, settings, &state, busy))
            }))))

        .child(div().flex_shrink_0().px_4().py_2().border_t_1().border_color(chrome_style::border()).flex().flex_col().gap_2()
            .child(div().flex().items_center().gap_1()
                .children([1, 2, 3].into_iter().map(|index| footer_button(index, &state, busy, unsaved, cx.global::<preferences::LastSettingsSave>().0.is_some(), &save_label)))
                .child(div().flex_1())
                .child(div().id("settings-keyboard-help").text_size(px(11.)).text_color(chrome_style::muted())
                    .tooltip(|_, cx| cx.new(|_| pane_view::ControlHint("Tab / ↑↓ select · ←→ change · Backspace reset · Ctrl+Tab category · Cmd/Ctrl+F search · Esc clear search or close".into())).into())
                    .child("Keyboard help")))
            .child(div().flex().items_center().gap_2()
                .child(div().flex_1().text_size(px(12.)).text_color(if unsaved { chrome_style::warning() } else { chrome_style::muted() })
                    .child(if busy { "Working…" } else if unsaved { "Unsaved changes" } else { "No unsaved changes" }))
                .children([5, 4, 0].into_iter().map(|index| footer_button(index, &state, busy, unsaved, cx.global::<preferences::LastSettingsSave>().0.is_some(), &save_label)))))

        .into_any_element()
}

fn footer_button(index: usize, state: &State, busy: bool, unsaved: bool, can_undo: bool, save_label: &str) -> AnyElement {
    let labels = [save_label, "Open JSON", "Reload", "Shortcuts…", "Revert", "Undo save"];
    let hints = ["Save current settings", "Open settings in your configured editor", "Reload settings from the saved file", "Find keyboard shortcuts", "Revert unsaved changes", "Undo the last save while preserving later file edits"];
    let enabled = footer_enabled(index, busy, unsaved, can_undo);
    let selected = state.selected == KEYS.len() + index;
    let hint = hints[index];
    div().id(("settings-footer", index)).flex_shrink_0().h(px(28.)).px_3().rounded_md().flex().items_center().justify_center().text_size(px(12.))
        .border_1().border_color(if selected { chrome_style::accent() } else { rgba(0x00000000) })
        .bg(if index == 0 && enabled { chrome_style::accent() } else { rgba(0x00000000) })
        .text_color(if !enabled { chrome_style::muted() } else if index == 0 { chrome_style::on_accent() } else { chrome_style::foreground() })
        .when(enabled, |button| button.cursor_pointer().hover(move |style| style.bg(if index == 0 { chrome_style::accent() } else { chrome_style::hover() })))
        .tooltip(move |_, cx| cx.new(|_| pane_view::ControlHint(hint.into())).into())
        .on_mouse_down(MouseButton::Left, move |_, window, cx| { if enabled { window.dispatch_action(Box::new(SelectSetting { key: FOOTER_KEYS[index].into() }), cx); } })
        .on_click(move |event, window, cx| { if enabled && event.standard_click() { if let Some(action) = footer_action(index) { window.dispatch_action(action, cx); } } })
        .child(labels[index].to_owned()).into_any_element()
}

fn setting_element(key: &str, settings: &Preferences, state: &State, busy: bool) -> AnyElement {
    match key {
        "theme" => (div().id("setting-row-theme").px_1().py_3().border_b_1().border_color(crate::chrome_style::border())
                .on_mouse_down(MouseButton::Left, |_, window, cx| window.dispatch_action(Box::new(SelectSetting { key: "theme".into() }), cx))
                .when(Some(state.selected) == control_index("theme"), |row| row.bg(crate::chrome_style::selected()))
                .child(div().mb_2().flex().items_center().justify_between().text_size(px(13.)).child("App & terminal theme").child(reset_button("theme")))
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
                .child(theme_preview(settings))).into_any_element(),
        "restore_session" => row_with_detail("Restore session on launch", "restore_session", if settings.restore_session { "On" } else { "Off" }.into(), false, Some(state.selected) == control_index(key), false, "Reopen your saved workspace. Save and restart to apply.".into(), "Changes startup behavior after saving and restarting rmux.".into()).into_any_element(),
        "import_theme" => (row("Import terminal theme", "import_theme", if busy { "Working…" } else { "Choose file…" }.into(), false, Some(state.selected) == control_index("import_theme"), false)).into_any_element(),
        "font_family" => (row("Default font family", "font_family", settings.font_family.clone(), false, Some(state.selected) == control_index("font_family"), false)).into_any_element(),
        "font" => (row("Default text size", "font", format!("{} pt", settings.font_size), true, Some(state.selected) == control_index("font"), false)).into_any_element(),
        "padding" => (row("Terminal padding", "padding", format!("{} px", settings.terminal_padding), true, Some(state.selected) == control_index("padding"), false)).into_any_element(),
        "opacity" => (row("Terminal background opacity", "opacity", format!("{}%", settings.background_opacity), true, Some(state.selected) == control_index("opacity"), false)).into_any_element(),
        "blur" => (div().child(row("Blur behind terminal", "blur", if settings.background_blur { "On" } else { "Off" }.into(), false, Some(state.selected) == control_index("blur"), false))
                ).into_any_element(),
        "selection_opacity" => row("Selection opacity", "selection_opacity", settings.selection_opacity.map_or_else(|| "Theme default".into(), |value| format!("{value}%")), true, Some(state.selected) == control_index("selection_opacity"), false).into_any_element(),
        "focused_cursor" => (row("Focused cursor", "focused_cursor", settings.focused_cursor.label().into(), false, Some(state.selected) == control_index("focused_cursor"), false)).into_any_element(),
        "cursor" => (row("Unfocused cursor", "cursor", match settings.unfocused_cursor { preferences::UnfocusedCursor::Hollow => "Hollow", preferences::UnfocusedCursor::Hidden => "Hidden" }.into(), false, Some(state.selected) == control_index("cursor"), false)).into_any_element(),
        "cursor_blink" => (row("Blink cursor", "cursor_blink", if settings.cursor_blink { "On" } else { "Off" }.into(), false, Some(state.selected) == control_index("cursor_blink"), false)).into_any_element(),
        "cursor_opacity" => (row("Cursor opacity", "cursor_opacity", format!("{}%", settings.cursor_opacity), true, Some(state.selected) == control_index("cursor_opacity"), false)).into_any_element(),
        "copy" => (row("Copy on select", "copy", if settings.copy_on_select { "On" } else { "Off" }.into(), false, Some(state.selected) == control_index("copy"), false)).into_any_element(),
        "option_as_alt" => (div().when(cfg!(target_os = "macos"), |panel| panel.child(div()
                .child(row("Option key behavior", "option_as_alt", if settings.macos_option_as_alt { "Terminal Alt" } else { "Native text" }.into(), false, Some(state.selected) == control_index("option_as_alt"), false))
                ))).into_any_element(),
        "scrollback" => (div()
                .child(row("New terminal scrollback", "scrollback", format!("{} MiB", settings.scrollback_mib), true, Some(state.selected) == control_index("scrollback"), false))
                ).into_any_element(),
        "workspace_placement" => (row("New workspace position", "workspace_placement", match settings.new_workspace_placement {
                workspace_groups::Placement::AfterCurrent => "After current",
                workspace_groups::Placement::Top => "First",
                workspace_groups::Placement::End => "Last",
            }.into(), false, Some(state.selected) == control_index("workspace_placement"), false)).into_any_element(),
        "group_placement" => (row("New group workspace", "group_placement", match settings.group_new_workspace_placement {
                workspace_groups::Placement::AfterCurrent => "After current",
                workspace_groups::Placement::Top => "First",
                workspace_groups::Placement::End => "Last",
            }.into(), false, Some(state.selected) == control_index("group_placement"), false)).into_any_element(),
        "close" => (row("Confirm before closing", "close", format!("{}{}", match settings.confirm_close {
                close_policy::ClosePolicy::Active => "Active tasks", close_policy::ClosePolicy::Always => "Always", close_policy::ClosePolicy::Never => "Never",
            }, if settings.close_policy_overridden() { " · Environment" } else { "" }), false, Some(state.selected) == control_index("close"), settings.close_policy_overridden())).into_any_element(),
        "controls_on_hover" => (row("Show controls only on hover", "controls_on_hover", if settings.controls_on_hover { "On" } else { "Off" }.into(), false, Some(state.selected) == control_index("controls_on_hover"), false)).into_any_element(),
        "hide_top_bar" => (row("Hide top workspace bar", "hide_top_bar", if settings.hide_top_bar { "On" } else { "Off" }.into(), false, Some(state.selected) == control_index("hide_top_bar"), false)).into_any_element(),
        "compact_sidebar" => (row("Compact sidebar", "compact_sidebar", if settings.compact_sidebar { "On" } else { "Off" }.into(), false, Some(state.selected) == control_index("compact_sidebar"), false)).into_any_element(),
        "native_notifications" => (div().child(row("Desktop notifications", "native_notifications", if settings.native_notifications { "On" } else { "Off" }.into(), false, Some(state.selected) == control_index("native_notifications"), false))
                ).into_any_element(),
        "notification_focus" => (row("Show desktop banners", "notification_focus", settings.notification_focus.label().into(), false, Some(state.selected) == control_index("notification_focus"), false)).into_any_element(),
        "file_editor" => {
            let description = settings.file_editor.as_ref().map(|editor| format!("Open files with {}.", editor.program.file_name().unwrap_or_default().to_string_lossy()))
                .unwrap_or_else(|| "No external editor selected. Choose a program to open files.".into());
            let hint = settings.file_editor.as_ref().map(|editor| format!("{}\n{}", editor.program.display(), control_hint(key)))
                .unwrap_or_else(|| control_hint(key).into());
            row_with_detail("External editor", "file_editor", "Configure…".into(), false, Some(state.selected) == control_index(key), false, description, hint.into()).into_any_element()
        }
        "editor_example" => row("VS Code configuration", "editor_example", "Copy example".into(), false, Some(state.selected) == control_index(key), false).into_any_element(),
        "editor_fallback" => row("Repair editor configuration", "editor_fallback", "Open…".into(), false, Some(state.selected) == control_index(key), false).into_any_element(),
        "shell_history" => {
            let description = format!("{} · Experimental. Recall commands after session restore; restart required.", shell_history::settings_status(&settings.shell_history_directory));
            let hint = settings.shell_history_directory.as_ref().map(|path| format!("{}\n{}", path.display(), control_hint(key)))
                .unwrap_or_else(|| control_hint(key).into());
            row_with_detail("Saved command history", "shell_history", "Configure…".into(), false, Some(state.selected) == control_index(key), false, description, hint.into()).into_any_element()
        }
        _ => {
            let item = settings_search::entry(key);
            row_with_detail(item.label, item.key, settings_editor::field(key).map(|field| { let value = settings_editor::value(settings, field); if value.is_empty() { "Default…".into() } else { value } }).unwrap_or_else(|| "Edit JSON…".into()), false, Some(state.selected) == control_index(key), false,
                if settings_editor::field(key).is_some_and(|field| field.restart) { format!("{} Save and restart to apply.", item.description) } else { item.description.into() },
                item.description.into()).into_any_element()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{adjust, Preferences, close_policy, workspace_groups, KEYS, FOOTER_COUNT};
    #[test]
    fn reset_changes_only_the_requested_setting() {
        let mut settings = Preferences::default();
        settings.font_size = 32.;
        settings.terminal_padding = 17;
        settings.background_opacity = 65;
        let mut expected = serde_json::to_value(&settings).unwrap();
        expected["font_size"] = serde_json::json!(Preferences::default().font_size);
        assert!(super::reset(&mut settings, "font"));
        assert_eq!(serde_json::to_value(&settings).unwrap(), expected);
        assert!(!super::reset(&mut settings, "commands"));
        assert!(!super::reset(&mut settings, "unknown"));
        assert_eq!(serde_json::to_value(&settings).unwrap(), expected);
    }

    #[test]
    fn every_reset_control_handles_its_setting() {
        for key in KEYS.iter().copied().filter(|key| super::resettable(key)) {
            assert!(super::reset(&mut Preferences::default(), key), "{key}");
        }
    }
    #[test]
    fn categories_cover_every_control_once_and_navigation_stays_visible() {
        use super::{CATEGORIES, next_visible, visible_controls};
        let mut categorized: Vec<_> = CATEGORIES.iter().flat_map(|(_, _, keys)| keys.iter().copied()).collect();
        categorized.sort();
        let mut expected = KEYS.to_vec();
        expected.sort();
        assert_eq!(categorized, expected);
        for category in 0..CATEGORIES.len() {
            let controls = visible_controls(category);
            assert_eq!(next_visible(&controls[..controls.len() - FOOTER_COUNT], controls[0], "tab", true), controls.last().copied());
            assert_eq!(next_visible(&controls[..controls.len() - FOOTER_COUNT], *controls.last().unwrap(), "down", false), Some(controls[0]));
            for pair in controls.windows(2) {
                assert_eq!(next_visible(&controls[..controls.len() - FOOTER_COUNT], pair[0], "tab", false), Some(pair[1]));
            }
            assert_eq!(next_visible(&controls[..controls.len() - FOOTER_COUNT], controls[0], "right", false), None);
            assert_eq!(controls.len(), CATEGORIES[category].2.len() + FOOTER_COUNT);
        }
    }
    #[test]
    fn arrows_adjust_values_but_never_launch_actions() {
        use super::{control_input, ControlInput};
        for control in ["file_editor", "editor_example", "editor_fallback", "import_theme", "font_family", "shell_history"].into_iter().chain(super::FOOTER_KEYS.iter().copied()).chain(crate::settings_search::ENTRIES.iter().filter(|entry| entry.json_only).map(|entry| entry.key)) {
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
