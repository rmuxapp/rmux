//! Shared event routing for pane commands; hosts supply session/platform work.
use gpui::{prelude::*, *};
use rmux_core::commands::CommandId;
use crate::{Split, SplitBelow, SplitLeft, SplitAbove, TogglePaneZoom, EqualizePanes, ResizePaneLeft, ResizePaneRight, ResizePaneUp, ResizePaneDown, SwapPaneNext, JoinPaneNext, BreakTabRight, BreakTabBelow, BreakTabLeft, BreakTabAbove, JoinPaneLeft, JoinPaneRight, JoinPaneUp, JoinPaneDown, SwapPaneLeft, SwapPaneRight, SwapPaneUp, SwapPaneDown, FocusLeft, FocusRight, FocusUp, FocusDown, NextPane, PreviousPane, SelectPaneNumber};

pub(crate) trait PaneCommandHost: Sized + 'static {
    fn select_pane_command(&mut self, number: usize, window: &mut Window, cx: &mut Context<Self>);
    fn run_pane_command(&mut self, command: CommandId, window: &mut Window, cx: &mut Context<Self>);
    fn cycle_pane_command(&mut self, direction: isize, window: &mut Window, cx: &mut Context<Self>);
}

macro_rules! pane_commands {
    ($($command:ident),* $(,)?) => {
        pub(crate) fn action(command: CommandId) -> Option<Box<dyn Action>> {
            match command { $(CommandId::$command => Some(Box::new($command)),)* _ => None }
        }
        pub(crate) fn bind<V: PaneCommandHost>(root: Div, cx: &mut Context<V>) -> Div {
            root$(.on_action(cx.listener(|this, _: &$command, window, cx|
                this.run_pane_command(CommandId::$command, window, cx))))*
                .on_action(cx.listener(|this, action: &SelectPaneNumber, window, cx| this.select_pane_command(action.number, window, cx)))
                .on_action(cx.listener(|this, _: &NextPane, window, cx| this.cycle_pane_command(1, window, cx)))
                .on_action(cx.listener(|this, _: &PreviousPane, window, cx| this.cycle_pane_command(-1, window, cx)))
        }
    };
}
pane_commands! {Split, SplitBelow, SplitLeft, SplitAbove, TogglePaneZoom, EqualizePanes, ResizePaneLeft, ResizePaneRight, ResizePaneUp, ResizePaneDown, SwapPaneNext, JoinPaneNext, BreakTabRight, BreakTabBelow, BreakTabLeft, BreakTabAbove, JoinPaneLeft, JoinPaneRight, JoinPaneUp, JoinPaneDown, SwapPaneLeft, SwapPaneRight, SwapPaneUp, SwapPaneDown, FocusLeft, FocusRight, FocusUp, FocusDown }
