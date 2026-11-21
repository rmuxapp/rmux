//! Shared pane bindings; tab navigation remains a separate action family.
use gpui::KeyBinding;
use crate::{NextPane, PreviousPane};
pub(crate) fn brackets() -> Vec<KeyBinding> {
    vec![KeyBinding::new("cmd-[", PreviousPane, Some("Terminal")),
         KeyBinding::new("cmd-]", NextPane, Some("Terminal"))]
}
