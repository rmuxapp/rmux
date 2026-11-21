//! Workspace action routing shared by native and browser application hosts.
use gpui::{prelude::*, *};
use rmux_core::commands::CommandId;
use crate::{NewWorkspace, NewWorkspaceGroup, CloseWorkspace, UngroupWorkspaces,
    NextWorkspace, PreviousWorkspace, LastWorkspace, ToggleSidebar, SelectWorkspaceNumber};

pub(crate) trait WorkspaceCommandHost: Sized + 'static {
    fn run_workspace_command(&mut self, command: CommandId, window: &mut Window, cx: &mut Context<Self>);
    fn select_workspace_number(&mut self, number: usize, window: &mut Window, cx: &mut Context<Self>);
}

macro_rules! workspace_commands {
    ($($command:ident),* $(,)?) => {
        pub(crate) fn action(command: CommandId) -> Option<Box<dyn Action>> {
            match command { $(CommandId::$command => Some(Box::new($command)),)* _ => None }
        }
        pub(crate) fn bind<V: WorkspaceCommandHost>(root: Div, cx: &mut Context<V>) -> Div {
            root$(.on_action(cx.listener(|this, _: &$command, window, cx|
                this.run_workspace_command(CommandId::$command, window, cx))))*
                .on_action(cx.listener(|this, action: &SelectWorkspaceNumber, window, cx|
                    this.select_workspace_number(action.number, window, cx)))
        }
    };
}
workspace_commands! { NewWorkspace, NewWorkspaceGroup, CloseWorkspace, UngroupWorkspaces,
    NextWorkspace, PreviousWorkspace, LastWorkspace, ToggleSidebar }
