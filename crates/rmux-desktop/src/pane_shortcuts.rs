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
