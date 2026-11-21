//! Tab action routing shared by native and sample-session panes.
use gpui::{prelude::*, *};
use rmux_core::commands::CommandId;
use crate::{NewTab, NextTab, PreviousTab, MoveTabLeft, MoveTabRight, ClosePane, SelectTabNumber};

pub(crate) trait TabCommandHost: Sized + 'static {
    fn select_tab_number(&mut self, number: usize, window: &mut Window, cx: &mut Context<Self>);
    fn run_tab_command(&mut self, command: CommandId, window: &mut Window, cx: &mut Context<Self>);
}
macro_rules! tab_commands {
    ($($command:ident),* $(,)?) => {
        pub(crate) fn action(command: CommandId) -> Option<Box<dyn Action>> {
            match command { $(CommandId::$command => Some(Box::new($command)),)* _ => None }
        }
        pub(crate) fn bind<V: TabCommandHost>(root: Div, cx: &mut Context<V>) -> Div {
            root$(.on_action(cx.listener(|this, _: &$command, window, cx|
                this.run_tab_command(CommandId::$command, window, cx))))*
                .on_action(cx.listener(|this, action: &SelectTabNumber, window, cx|
                    this.select_tab_number(action.number, window, cx)))
        }
    };
}
tab_commands! { NewTab, NextTab, PreviousTab, MoveTabLeft, MoveTabRight, ClosePane }
