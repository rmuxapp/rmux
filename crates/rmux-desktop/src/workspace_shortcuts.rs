//! Workspace defaults shared by native and browser hosts.
use gpui::KeyBinding;
use crate::{NewWorkspace, NextWorkspace, PreviousWorkspace, SelectWorkspaceNumber};

pub(crate) fn bindings(mac: bool) -> Vec<KeyBinding> {
    let mut bindings = vec![
        KeyBinding::new(if mac { "cmd-n" } else { "ctrl-shift-n" }, NewWorkspace, None),
        KeyBinding::new(if mac { "cmd-alt-down" } else { "ctrl-alt-down" }, NextWorkspace, None),
        KeyBinding::new(if mac { "cmd-alt-up" } else { "ctrl-alt-up" }, PreviousWorkspace, None),
    ];
    let modifier = if mac { "cmd" } else { "ctrl-alt" };
    bindings.extend((1..=9).map(|number| KeyBinding::new(
        &format!("{modifier}-{number}"), SelectWorkspaceNumber { number }, None)));
    bindings
}
