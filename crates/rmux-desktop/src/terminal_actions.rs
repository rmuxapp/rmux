//! Shared terminal action dispatch; clipboard and engine services belong to hosts.
use gpui::{prelude::*, *};
use crate::{IncreaseFontSize, DecreaseFontSize, ResetFontSize, CopySelectionWithLineBreaks, CopySelection, Paste, EditorSelectAll, ClearScrollback, MoveToLineStart, MoveToLineEnd, DeleteToLineStart};
use rmux_core::commands::CommandId;

pub(crate) trait TerminalCommandHost: Sized + 'static {
    fn send_terminal_control(&mut self, text: &str, window: &mut Window, cx: &mut Context<Self>);
    fn resize_terminal_font(&mut self, delta: Option<f32>, cx: &mut Context<Self>);
    fn copy_terminal(&mut self, window: &mut Window, cx: &mut Context<Self>);
    fn copy_terminal_with_line_breaks(&mut self, window: &mut Window, cx: &mut Context<Self>);
    fn select_terminal(&mut self, window: &mut Window, cx: &mut Context<Self>);
    fn clear_terminal_history(&mut self, window: &mut Window, cx: &mut Context<Self>);
}
pub(crate) fn action(command: CommandId) -> Option<Box<dyn Action>> {
    match command {
        CommandId::MoveToLineStart => Some(Box::new(MoveToLineStart)),
        CommandId::MoveToLineEnd => Some(Box::new(MoveToLineEnd)),
        CommandId::DeleteToLineStart => Some(Box::new(DeleteToLineStart)),
        CommandId::CopySelection => Some(Box::new(CopySelection)),
        CommandId::Paste => Some(Box::new(Paste)),
        CommandId::IncreaseFontSize => Some(Box::new(IncreaseFontSize)),
        CommandId::DecreaseFontSize => Some(Box::new(DecreaseFontSize)),
        CommandId::ResetFontSize => Some(Box::new(ResetFontSize)),
        CommandId::CopySelectionWithLineBreaks => Some(Box::new(CopySelectionWithLineBreaks)),
        CommandId::EditorSelectAll => Some(Box::new(EditorSelectAll)),
        CommandId::ClearScrollback => Some(Box::new(ClearScrollback)),
        _ => None,
    }
}
pub(crate) fn bind<V: TerminalCommandHost>(root: Div, cx: &mut Context<V>) -> Div {
    root.on_action(cx.listener(|this, _: &MoveToLineStart, window, cx| this.send_terminal_control("\x01", window, cx)))
        .on_action(cx.listener(|this, _: &MoveToLineEnd, window, cx| this.send_terminal_control("\x05", window, cx)))
        .on_action(cx.listener(|this, _: &DeleteToLineStart, window, cx| this.send_terminal_control("\x15", window, cx)))
        .on_action(cx.listener(|this, _: &IncreaseFontSize, _, cx| this.resize_terminal_font(Some(1.), cx)))
        .on_action(cx.listener(|this, _: &DecreaseFontSize, _, cx| this.resize_terminal_font(Some(-1.), cx)))
        .on_action(cx.listener(|this, _: &ResetFontSize, _, cx| this.resize_terminal_font(None, cx)))
        .on_action(cx.listener(|this, _: &CopySelection, window, cx| this.copy_terminal(window, cx)))
        .on_action(cx.listener(|this, _: &CopySelectionWithLineBreaks, window, cx| this.copy_terminal_with_line_breaks(window, cx)))
        .on_action(cx.listener(|this, _: &EditorSelectAll, window, cx| this.select_terminal(window, cx)))
        .on_action(cx.listener(|this, _: &ClearScrollback, window, cx| this.clear_terminal_history(window, cx)))
}
