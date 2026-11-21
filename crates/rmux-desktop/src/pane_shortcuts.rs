//! Shared pane bindings; tab navigation remains a separate action family.
use gpui::KeyBinding;
use crate::{NextPane, PreviousPane};
pub(crate) fn brackets() -> Vec<KeyBinding> {
    vec![KeyBinding::new("cmd-[", PreviousPane, Some("Terminal")),
         KeyBinding::new("cmd-]", NextPane, Some("Terminal"))]
}
pub(crate) fn tabs() -> Vec<KeyBinding> {
    use crate::{NextTab, PreviousTab};
    vec![KeyBinding::new("cmd-{", PreviousTab, Some("Terminal")),
         KeyBinding::new("cmd-}", NextTab, Some("Terminal")),
         KeyBinding::new("cmd-shift-[", PreviousTab, Some("Terminal")),
         KeyBinding::new("cmd-shift-]", NextTab, Some("Terminal"))]
}

pub(crate) fn tab_reordering() -> Vec<KeyBinding> {
    use crate::{MoveTabLeft, MoveTabRight};
    vec![KeyBinding::new("ctrl-shift-pageup", MoveTabLeft, Some("Terminal")),
         KeyBinding::new("ctrl-shift-pagedown", MoveTabRight, Some("Terminal"))]
}

pub(crate) fn directions(mac: bool) -> Vec<KeyBinding> {
    use crate::{FocusLeft, FocusRight, FocusUp, FocusDown};
    vec![
        KeyBinding::new(if mac { "cmd-ctrl-left" } else { "alt-shift-left" }, FocusLeft, None),
        KeyBinding::new(if mac { "cmd-ctrl-right" } else { "alt-shift-right" }, FocusRight, None),
        KeyBinding::new(if mac { "cmd-ctrl-up" } else { "alt-shift-up" }, FocusUp, None),
        KeyBinding::new(if mac { "cmd-ctrl-down" } else { "alt-shift-down" }, FocusDown, None),
    ]
}

pub(crate) fn splits(mac: bool) -> Vec<KeyBinding> {
    use crate::{Split, SplitBelow, TogglePaneZoom};
    vec![
        KeyBinding::new(if mac { "cmd-d" } else { "ctrl-shift-d" }, Split, None),
        KeyBinding::new(if mac { "cmd-shift-d" } else { "ctrl-shift-e" }, SplitBelow, None),
        KeyBinding::new(if mac { "cmd-shift-enter" } else { "ctrl-shift-enter" }, TogglePaneZoom, None),
    ]
}

/// Direct tab positions use the same terminal context on each platform.
pub(crate) fn tab_numbers(mac: bool) -> Vec<KeyBinding> {
    let modifier = if mac { "cmd-shift" } else { "ctrl-shift" };
    let mut bindings: Vec<_> = (1..=9).map(|number| KeyBinding::new(
        &format!("{modifier}-{number}"), crate::SelectTabNumber { number }, Some("Terminal"),
    )).collect();
    // Web events may retain the shifted symbol; keep numeric bindings too.
    bindings.extend(['!', '@', '#', '$', '%', '^', '&', '*', '('].into_iter().enumerate().map(|(index, symbol)|
        KeyBinding::new(&format!("{modifier}-{symbol}"), crate::SelectTabNumber { number: index + 1 }, Some("Terminal"))));
    bindings
}
