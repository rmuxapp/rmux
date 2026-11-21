//! Shared terminal action dispatch; clipboard and engine services belong to hosts.
use gpui::{prelude::*, *};
use crate::{CopySelectionWithLineBreaks, CopySelection, EditorSelectAll, ClearScrollback};
use rmux_core::commands::CommandId;

pub(crate) trait TerminalCommandHost: Sized + 'static {
    fn copy_terminal(&mut self, window: &mut Window, cx: &mut Context<Self>);
    fn copy_terminal_with_line_breaks(&mut self, window: &mut Window, cx: &mut Context<Self>);
    fn select_terminal(&mut self, window: &mut Window, cx: &mut Context<Self>);
    fn clear_terminal_history(&mut self, window: &mut Window, cx: &mut Context<Self>);
}
pub(crate) fn action(command: CommandId) -> Option<Box<dyn Action>> {
    match command {
        CommandId::CopySelectionWithLineBreaks => Some(Box::new(CopySelectionWithLineBreaks)),
        CommandId::EditorSelectAll => Some(Box::new(EditorSelectAll)),
        CommandId::ClearScrollback => Some(Box::new(ClearScrollback)),
        _ => None,
    }
}
pub(crate) fn bind<V: TerminalCommandHost>(root: Div, cx: &mut Context<V>) -> Div {
    root.on_action(cx.listener(|this, _: &CopySelection, window, cx| this.copy_terminal(window, cx)))
        .on_action(cx.listener(|this, _: &CopySelectionWithLineBreaks, window, cx| this.copy_terminal_with_line_breaks(window, cx)))
        .on_action(cx.listener(|this, _: &EditorSelectAll, window, cx| this.select_terminal(window, cx)))
        .on_action(cx.listener(|this, _: &ClearScrollback, window, cx| this.clear_terminal_history(window, cx)))
}
