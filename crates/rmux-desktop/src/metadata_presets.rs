//! Workspace metadata presets shared by desktop and browser hosts.
use gpui::{prelude::*, *};
use crate::{text_field::TextField, text_layout, control_hint::ControlHint};
use rmux_core::workspace_lifecycle::parse_workspace_color;

pub(crate) const WORKSPACE_ICONS: &[(&str, Option<&str>)] = &[
    ("None", None),
    ("Terminal", Some("⌘")),
    ("Code", Some("⌨")),
    ("Build", Some("⚙")),
    ("Cloud", Some("☁")),
    ("Notes", Some("✏")),
];
pub(crate) const WORKSPACE_COLORS: &[(&str, Option<u32>)] = &[
    ("None", None),
    ("Blue", Some(0x7aa2f7)),
    ("Green", Some(0x9ece6a)),
    ("Amber", Some(0xe0af68)),
    ("Red", Some(0xf7768e)),
    ("Purple", Some(0xbb9af7)),
    ("Cyan", Some(0x7dcfff)),
];

pub(crate) fn render(input: &Entity<TextField>, icons: bool, cx: &App) -> AnyElement {
    let choices: Vec<(&str, String, Option<u32>)> = if icons {
        WORKSPACE_ICONS.iter().map(|(name, icon)| (*name, icon.unwrap_or_default().to_owned(), None)).collect()
    } else {
        WORKSPACE_COLORS.iter().map(|(name, color)| (*name, color.map(|color| format!("#{color:06X}")).unwrap_or_default(), *color)).collect()
    };
    let draft = input.read(cx).value().trim();
    div().flex().flex_wrap().gap_1().py_1()
        .children(choices.into_iter().enumerate().map(|(index, (name, value, color))| {
            let selected = if icons { draft == value }
                else { parse_workspace_color(draft).is_ok_and(|current| current == color) };
            let input = input.clone();
            let label = if icons && !value.is_empty() { value.clone() }
                else if value.is_empty() { "×".into() } else { String::new() };
            div().id(("metadata-preset", index)).w(px(28.)).h(px(28.))
                .flex().items_center().justify_center().rounded_md().border_1()
                .border_color(rgb(if selected { 0xa7c7ff } else { 0x465063 }))
                .bg(rgb(0x252d3b)).cursor_pointer()
                .hover(|button| button.bg(rgb(0x3a465c)))
                .tooltip(move |_, cx| cx.new(|_| ControlHint(name.into())).into())
                .child(div().w(px(16.)).h(px(16.)).rounded_sm().text_center().line_height(px(16.))
                    .when_some(color, |swatch, color| swatch.bg(rgb(color))).child(label))
                .on_click(move |event, window, cx| {
                    if !event.standard_click() { return; }
                    cx.stop_propagation();
                    input.update(cx, |input, cx| input.replace_draft(&value, cx));
                    let focus = input.read(cx).focus.clone();
                    text_layout::focus(&focus, window, cx);
                })
        })).into_any_element()
}
