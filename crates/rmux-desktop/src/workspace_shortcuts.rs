//! Workspace defaults shared by native and browser hosts.
use gpui::KeyBinding;
use crate::{NewWorkspace, NewWorkspaceGroup, GroupSelectedWorkspaces, NextWorkspace, PreviousWorkspace, ToggleSidebar, SelectWorkspaceNumber, MoveWorkspaceUp, MoveWorkspaceDown};

pub(crate) fn bindings(mac: bool) -> Vec<KeyBinding> {
    let mut bindings = vec![
        KeyBinding::new(if mac { "cmd-shift-g" } else { "ctrl-alt-shift-g" }, GroupSelectedWorkspaces, None),
        KeyBinding::new(if mac { "cmd-alt-shift-up" } else { "ctrl-alt-shift-up" }, MoveWorkspaceUp, None),
        KeyBinding::new(if mac { "cmd-alt-shift-down" } else { "ctrl-alt-shift-down" }, MoveWorkspaceDown, None),
        KeyBinding::new(if mac { "cmd-b" } else { "ctrl-shift-b" }, ToggleSidebar, None),
        KeyBinding::new(if mac { "ctrl-cmd-g" } else { "ctrl-alt-g" }, NewWorkspaceGroup, None),
        KeyBinding::new(if mac { "cmd-n" } else { "ctrl-shift-n" }, NewWorkspace, None),
        KeyBinding::new(if mac { "cmd-alt-down" } else { "ctrl-alt-down" }, NextWorkspace, None),
        KeyBinding::new(if mac { "cmd-alt-up" } else { "ctrl-alt-up" }, PreviousWorkspace, None),
    ];
    let modifier = if mac { "cmd" } else { "ctrl-alt" };
    bindings.extend((1..=9).map(|number| KeyBinding::new(
        &format!("{modifier}-{number}"), SelectWorkspaceNumber { number }, None)));
    bindings
}
