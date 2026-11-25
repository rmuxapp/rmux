//! Focused value editor shared by Settings color, path, and text rows.
use super::*;

#[derive(Clone, Copy)]
pub struct Field { pub key: &'static str, pub config: &'static str, pub color: bool, pub path: bool, pub restart: bool }
pub fn field(key: &str) -> Option<Field> {
    let (key, config, color, path, restart) = match key {
        "foreground_color" => ("foreground_color", "foreground_color", true, false, false),
        "background_color" => ("background_color", "background_color", true, false, false),
        "cursor_color" => ("cursor_color", "cursor_color", true, false, false),
        "cursor_text" => ("cursor_text", "cursor_text", true, false, false),
        "selection_background" => ("selection_background", "selection_background", true, false, false),
        "selection_foreground" => ("selection_foreground", "selection_foreground", true, false, false),
        "session_file" => ("session_file", "session_file", false, true, true),
        "socket_directory" => ("socket_directory", "socket_directory", false, true, true),
        "shell_history" => ("shell_history", "shell_history_directory", false, true, true),
        "font_family" => ("font_family", "font_family", false, false, false),
        _ => return None,
    };
    Some(Field { key, config, color, path, restart })
}
pub fn value(settings: &Preferences, field: Field) -> String {
    // Rendering a value must not serialize unrelated commands, key bindings,
    // palettes, and layouts on every Settings redraw.
    match field.config {
        "foreground_color" => settings.foreground_color.map(String::from),
        "background_color" => settings.background_color.map(String::from),
        "cursor_color" => settings.cursor_color.map(String::from),
        "cursor_text" => settings.cursor_text.map(String::from),
        "selection_background" => settings.selection_background.map(String::from),
        "selection_foreground" => settings.selection_foreground.map(String::from),
        "font_family" => Some(settings.font_family.clone()),
        "session_file" => settings.session_file.as_ref().map(|path| path.to_string_lossy().into_owned()),
        "socket_directory" => settings.socket_directory.as_ref().map(|path| path.to_string_lossy().into_owned()),
        "shell_history_directory" => settings.shell_history_directory.as_ref().map(|path| path.to_string_lossy().into_owned()),
        _ => None,
    }.unwrap_or_default()
}
pub fn apply(settings: &Preferences, field: Field, text: &str) -> anyhow::Result<Preferences> {
    let text = text.trim();
    anyhow::ensure!(!text.chars().any(char::is_control), "Remove control characters from this value.");
    if field.config == "font_family" {
        anyhow::ensure!(!text.is_empty(), "Enter a font family name.");
        anyhow::ensure!(text.len() <= 128, "Use a font family name no longer than 128 bytes.");
    }
    if field.color && !text.is_empty() {
        let valid = if matches!(field.config, "foreground_color" | "background_color") {
            preferences::HexColor::try_from(text.to_owned()).is_ok()
        } else { preferences::CellColor::try_from(text.to_owned()).is_ok() };
        anyhow::ensure!(valid, "Enter a valid color such as #AABBCC, or leave empty to use the theme.");
    }
    if field.path && !text.is_empty() {
        anyhow::ensure!(std::path::Path::new(text).is_absolute(), "Enter an absolute path, or leave empty to use the default.");
        anyhow::ensure!(std::path::Path::new(text).as_os_str().len() <= 8192, "Use a path no longer than 8192 bytes.");
    }
    let value = if text.is_empty() { serde_json::Value::Null } else { serde_json::Value::String(text.into()) };
    settings.edit_panel_value(field.config, value)
}
#[derive(Clone)]
pub struct Editor { pub field: Field, pub input: Entity<text_field::TextField>, pub error: Option<String> }
fn action(id: &'static str, label: &'static str, action: impl Action) -> impl IntoElement {
    div().id(id).px_3().py_2().rounded_md().cursor_pointer().text_sm().bg(chrome_style::hover())
        .hover(|s| s.bg(chrome_style::hover_strong()))
        .on_click(move |event, window, cx| { if event.standard_click() { window.dispatch_action(action.boxed_clone(), cx); } }).child(label)
}
pub fn panel(editor: Editor, height: f32) -> impl IntoElement {
    let field = editor.field;
    let entry = settings_search::entry(field.key);
    div().absolute().inset_0().occlude().bg(rgba(0x00000066)).flex().items_center().justify_center().p_4()
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .child(div().id("setting-editor-scroll").w(px(480.)).max_w_full().max_h(px((height - 32.).max(1.)))
            .overflow_y_scroll().rounded_lg().shadow_lg().bg(chrome_style::elevated())
            .border_1().border_color(chrome_style::border())
            .child(div().p_4().flex().flex_col().gap_3()
            .child(div().text_lg().font_weight(FontWeight::SEMIBOLD).child(entry.label))
            .child(div().text_sm().text_color(chrome_style::muted()).child(entry.description))
            .child(div().px_3().py_2().rounded_md().border_1().border_color(if editor.error.is_some() { chrome_style::danger() } else { chrome_style::selected_border() }).bg(chrome_style::background()).child(editor.input))
            .when(field.color, |root| root.child(div().flex().flex_wrap().gap_2().children(
                [0xffffff,0x111111,0xe78284,0xef9f76,0xe5c890,0xa6d189,0x81c8be,0x99d1db,0x8caaee,0xca9ee6,0xf4b8e4,0xb5bfe2].into_iter().enumerate().map(|(i, color)| {
                    let value = format!("#{color:06x}");
                    div().id(("setting-color",i)).size(px(26.)).rounded_md().border_1().border_color(chrome_style::border()).bg(rgb(color)).cursor_pointer()
                        .tooltip({ let value: SharedString = value.clone().into(); move |_, cx| cx.new(|_| pane_view::ControlHint(value.clone())).into() })
                        .on_click(move |event, window, cx| { if event.standard_click() { window.dispatch_action(Box::new(SetSettingDraft { value: value.clone() }), cx); } })
                })
            )))
            .when(field.color, |root| root.child(div().text_xs().text_color(chrome_style::muted()).child(
                if matches!(field.config, "foreground_color" | "background_color") { "Choose a swatch or enter #RRGGBB. Leave empty to use the theme." }
                else { "Enter #RRGGBB, cell-foreground, or cell-background. Leave empty to use the theme." }
            )))
            .when(field.path, |root| root.child(div().text_xs().text_color(chrome_style::muted()).child(
                if field.config == "socket_directory" { "Enter a new absolute directory path. Browse selects its parent; rmux creates the automation directory on restart." }
                else { "Enter an absolute path, or leave empty to use the default. You can type a path that does not exist yet." }
            )))
            .when(field.restart, |root| root.child(div().text_xs().text_color(chrome_style::accent()).child("Save and restart rmux to apply this change.")))
            .when_some(editor.error, |root, error| root.child(div().text_sm().text_color(chrome_style::danger()).child(error)))
            .child(div().flex().flex_wrap().items_center().gap_2()
                .when(field.path, |row| row.child(action("browse-setting-path", "Browse…", BrowseSettingPath)))
                .when(field.config != "font_family", |row| row.child(action("default-setting-value", "Use default", SetSettingDraft { value: String::new() })))
                .child(div().flex_1())
                .child(action("cancel-setting-value", "Cancel", CancelSettingEditor))
                .child(action("apply-setting-value", "Apply", ApplySettingEditor)))
            .child(div().text_xs().text_color(chrome_style::muted()).child("Enter to apply · Esc to cancel"))))
}

#[cfg(test)]
mod tests {
    use super::{field, value, apply, Preferences};
    #[test]
    fn colors_validate_and_clear_without_changing_other_values() {
        let mut initial = Preferences::default();
        initial.font_size = 23.;
        let color = field("cursor_color").unwrap();
        assert!(apply(&initial, color, "#xyz").is_err());
        let edited = apply(&initial, color, "#AABBCC").unwrap();
        assert_eq!(value(&edited, color), "#aabbcc");
        assert_eq!(edited.font_size, 23.);
        assert!(apply(&edited, color, "").unwrap().cursor_color.is_none());
        assert!(apply(&initial, color, "cell-background").is_ok());
        assert!(apply(&initial, field("foreground_color").unwrap(), "cell-background").is_err());
    }
    #[test]
    fn paths_and_text_validate_before_applying() {
        let initial = Preferences::default();
        assert!(apply(&initial, field("socket_directory").unwrap(), "relative/path").is_err());
        assert!(apply(&initial, field("font_family").unwrap(), "").is_err());
        assert!(apply(&initial, field("font_family").unwrap(), "bad\nfont").is_err());
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("new-socket");
        let next = apply(&initial, field("socket_directory").unwrap(), path.to_str().unwrap()).unwrap();
        assert_eq!(next.socket_directory.as_ref(), Some(&path));
        assert!(!path.exists());
    }
}
