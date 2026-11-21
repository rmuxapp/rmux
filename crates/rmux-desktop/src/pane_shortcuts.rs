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
