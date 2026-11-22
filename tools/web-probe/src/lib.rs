//! Compile the actual desktop text input against a candidate web backend.
//! This is a compatibility probe, not a replacement demo or second UI.
use gpui::{prelude::*, *};
use std::ops::Range;
const FONT_FAMILY: &str = "Inter";
#[path = "../../../crates/rmux-desktop/src/control_hint.rs"]
mod control_hint;
#[path = "../../../crates/rmux-desktop/src/menu_navigation.rs"]
mod menu_navigation;
#[path = "../../../crates/rmux-desktop/src/scrollbar_view.rs"]
mod scrollbar_view;
mod pane_view { pub(crate) use crate::control_hint::ControlHint; }

#[path = "../../../crates/rmux-desktop/src/composition.rs"]
mod composition;
#[path = "../../../crates/rmux-desktop/src/preedit_view.rs"]
mod preedit_view;
#[path = "../../../crates/rmux-desktop/src/edit_history.rs"]
mod edit_history;
#[path = "../../../crates/rmux-desktop/src/text_field.rs"]
mod text_field;
mod text_layout;
mod fonts;
mod browser_find;
mod browser_clipboard;
#[path = "../../../crates/rmux-desktop/src/find_presentation.rs"]
mod find_presentation;
#[path = "../../../crates/rmux-desktop/src/pane_actions.rs"]
mod pane_actions;
#[path = "../../../crates/rmux-desktop/src/workspace_actions.rs"]
mod workspace_actions;
#[path = "../../../crates/rmux-desktop/src/tab_actions.rs"]
mod tab_actions;
#[path = "../../../crates/rmux-desktop/src/terminal_actions.rs"]
mod terminal_actions;
#[path = "../../../crates/rmux-desktop/src/palette_view.rs"]
mod palette_view;
use rmux_core::{palette_match, commands::CommandId, layout::{PaneTree, Axis}};
#[path = "../../../crates/rmux-desktop/src/split_presentation.rs"]
mod split_presentation;
const COMMANDS: &[CommandId] = &[
    CommandId::NewWorkspace, CommandId::NewTab, CommandId::RenameTab, CommandId::CloseOtherTabs, CommandId::CloseTabsToRight, CommandId::ToggleSidebar,
    CommandId::Find, CommandId::FindOlder, CommandId::FindNewer,
    CommandId::MoveWorkspaceUp, CommandId::MoveWorkspaceDown,
    CommandId::ShowTabMenu, CommandId::NextTab, CommandId::PreviousTab, CommandId::MoveTabLeft, CommandId::MoveTabRight,
    CommandId::ClosePane, CommandId::NewWorkspaceGroup, CommandId::GroupCurrentWorkspace, CommandId::GroupSelectedWorkspaces, CommandId::ToggleGroupPin, CommandId::RemoveWorkspaceFromGroup, CommandId::SetGroupAnchor, CommandId::CloseWorkspace,
    CommandId::NextWorkspace, CommandId::PreviousWorkspace, CommandId::LastWorkspace, CommandId::UngroupWorkspaces,
    CommandId::Split, CommandId::SplitBelow, CommandId::SplitLeft, CommandId::SplitAbove, CommandId::TogglePaneZoom,
    CommandId::BreakTabRight, CommandId::BreakTabBelow, CommandId::BreakTabLeft, CommandId::BreakTabAbove,
    CommandId::JoinPaneNext, CommandId::JoinPaneLeft, CommandId::JoinPaneRight, CommandId::JoinPaneUp, CommandId::JoinPaneDown,
    CommandId::SwapPaneNext, CommandId::SwapPaneLeft, CommandId::SwapPaneRight,
    CommandId::SwapPaneUp, CommandId::SwapPaneDown,
    CommandId::EqualizePanes, CommandId::ResizePaneLeft, CommandId::ResizePaneRight,
    CommandId::ResizePaneUp, CommandId::ResizePaneDown,
    CommandId::FocusLeft, CommandId::FocusRight, CommandId::FocusUp, CommandId::FocusDown,
    CommandId::IncreaseFontSize, CommandId::DecreaseFontSize, CommandId::ResetFontSize, CommandId::ShowWorkspaceMenu, CommandId::EditWorkspaceIcon, CommandId::EditGroupIcon, CommandId::EditWorkspaceColor, CommandId::EditGroupColor, CommandId::EditWorkspaceDescription, CommandId::CloseWorkspaceGroup, CommandId::ToggleWorkspacePin, CommandId::RenameWorkspaceGroup, CommandId::RenameWorkspace, CommandId::ShowTerminalMenu, CommandId::CopySelectionWithLineBreaks, CommandId::EditorSelectAll, CommandId::ClearScrollback,
    CommandId::ScrollToTop, CommandId::ScrollToBottom,
];
struct WorkspaceCommand(CommandId);
impl EventEmitter<WorkspaceCommand> for InputProbe {}
enum PaletteRequest { Open, SelectWorkspace(EntityId) }
impl EventEmitter<PaletteRequest> for InputProbe {}


actions!(rmux, [IncreaseFontSize, DecreaseFontSize, ResetFontSize, ShowWorkspaceMenu, EditWorkspaceIcon, EditGroupIcon, EditWorkspaceColor, EditGroupColor, EditWorkspaceDescription, CloseWorkspaceGroup, ToggleWorkspacePin, RenameWorkspaceGroup, RenameWorkspace, ShowTerminalMenu, CopySelectionWithLineBreaks, ClearScrollback, ShowTabMenu, Find, FindOlder, FindNewer, CloseWorkspace, UngroupWorkspaces, EditorUndo, EditorRedo, Paste, CopySelection, EditorCut, EditorSelectAll, CommandPalette, NextPane, PreviousPane, NewTab, RenameTab, CloseOtherTabs, CloseTabsToRight, NextTab, PreviousTab, MoveTabLeft, MoveTabRight, ClosePane, NewWorkspaceGroup, GroupCurrentWorkspace, GroupSelectedWorkspaces, ToggleGroupPin, RemoveWorkspaceFromGroup, SetGroupAnchor, MoveWorkspaceUp, MoveWorkspaceDown, NewWorkspace, ToggleSidebar, NextWorkspace, PreviousWorkspace, LastWorkspace, FocusLeft, FocusRight, FocusUp, FocusDown, Split, SplitBelow, TogglePaneZoom, SplitLeft, SplitAbove, EqualizePanes, ResizePaneLeft, ResizePaneRight, ResizePaneUp, ResizePaneDown, SwapPaneNext, SwapPaneLeft, SwapPaneRight, SwapPaneUp, SwapPaneDown, JoinPaneNext, JoinPaneLeft, JoinPaneRight, JoinPaneUp, JoinPaneDown, BreakTabRight, BreakTabBelow, BreakTabLeft, BreakTabAbove]);
#[derive(Clone, Debug, Default, PartialEq, Action)]
#[action(namespace = rmux, no_json)]
struct SelectWorkspaceNumber { number: usize }
#[derive(Clone, Debug, Default, PartialEq, Action)]
#[action(namespace = rmux, no_json)]
struct MoveWorkspaceToGroup { group: u64 }
#[derive(Clone, Debug, Default, PartialEq, Action)]
#[action(namespace = rmux, no_json)]
struct SelectTabNumber { number: usize }
#[derive(Clone, Debug, Default, PartialEq, Action)]
#[action(namespace = rmux, no_json)]
struct SelectPaneNumber { number: usize }
mod clipboard {
    pub fn write(item: gpui::ClipboardItem, cx: &mut gpui::App) {
        cx.write_to_clipboard(item);
    }
}
struct InputProbe {
    name: String,
    empty_focus: FocusHandle,
    input: Entity<text_field::TextField>,
    search: palette_match::RankedSearch,
    workspace_results: Vec<(EntityId, String)>,
    group_results: Vec<(u64, String)>,
    selected: usize,
    scroll: ListState,
    feedback: Option<String>,
    pane_state: rmux_core::pane_commands::State<usize, usize>,
    focused: usize,
    rendered_zoom: Option<usize>,
    next_pane: usize,
    split_drag: Option<split_interaction::SplitDrag>,
    palette_open: bool,
    palette_origin: Option<FocusHandle>,
    result_hints: Vec<Option<String>>,
    samples: std::collections::HashMap<usize, Entity<sample_pane::SamplePane>>,
}
impl InputProbe {
    fn accepts_workspace_tab(&self, cx: &App) -> bool {
        self.samples.is_empty() || self.samples.get(&self.focused).is_some_and(|pane| pane.read(cx).insertion_index(None).is_some())
    }
    fn owns_dragged_tab(&self, item: &drag_item::DragItem, cx: &App) -> bool {
        self.samples.values().any(|pane| pane.entity_id() == item.container && pane.read(cx).contains_transferable_tab(item.item))
    }
    fn take_workspace_tab(&mut self, item: &drag_item::DragItem, cx: &mut Context<Self>) -> Option<sample_pane::TabTransfer> {
        let (&id, pane) = self.samples.iter().find(|(_, pane)| pane.entity_id() == item.container)?;
        let pane = pane.clone();
        let transfer = pane.update(cx, |pane, cx| pane.take_tab(item.item, cx))?;
        if pane.read(cx).tab_count() == 0 {
            let next = self.pane_state.layout.as_ref().and_then(|tree|
                rmux_core::pane_commands::focus_after_close(tree, &id, Some(&self.focused)));
            self.pane_state.remove(&id);
            self.samples.remove(&id);
            if let Some(next) = next { self.focused = next; self.pane_state.focus_changed(next); }
        }
        self.split_drag = None;
        cx.notify();
        Some(transfer)
    }
    fn receive_workspace_tab(&mut self, transfer: sample_pane::TabTransfer, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(pane) = self.samples.get(&self.focused) {
            pane.update(cx, |pane, cx| pane.append_tabs(transfer, window, cx));
        } else {
            let id = self.next_pane;
            self.next_pane += 1;
            let pane = cx.new(|_| sample_pane::SamplePane::from_transfer(transfer));
            self.register_pane(id, pane, window, cx);
            self.pane_state.layout = Some(PaneTree::Pane(id));
            self.focused = id;
        }
        self.close_palette(window, cx);
    }
    fn new(name: String, window: &mut Window, cx: &mut Context<Self>) -> Self {
            let input = cx.new(|cx| text_field::TextField::new("", cx));
            input.update(cx, |input, cx| {
                input.placeholder = Some("Type a command or workspace name…");
                input.focus.focus(window, cx);
            });
            cx.subscribe_in(&input, window, |this: &mut InputProbe, _, event: &text_field::EditEvent, window, cx| {
                match event {
                    text_field::EditEvent::Changed(query) => { this.search.update(query); this.scroll.reset(this.search.matches.len()); this.selected = 0; this.feedback = None; },
                    text_field::EditEvent::Submitted { .. } => this.choose(window, cx),
                    text_field::EditEvent::Cancelled => this.close_palette(window, cx),
                }
                cx.notify();
            }).detach();
            let samples = std::collections::HashMap::new();
            let mut host = InputProbe { name, empty_focus: cx.focus_handle(), palette_origin: None, result_hints: Vec::new(), split_drag: None, scroll: ListState::new(COMMANDS.len(), ListAlignment::Top, px(96.)), workspace_results: Vec::new(), group_results: Vec::new(), samples, input, search: palette_match::RankedSearch::new(COMMANDS.iter().map(|title| title.title().to_lowercase()).collect()), selected: 0, feedback: None, pane_state: rmux_core::pane_commands::State { layout: Some(PaneTree::Pane(0)), ..Default::default() }, focused: 0, rendered_zoom: None, next_pane: 1, palette_open: false };
            host.add_pane(0, window, cx);
            host

    }

    fn add_pane(&mut self, id: usize, window: &mut Window, cx: &mut Context<Self>) {
        let font_size = self.samples.get(&self.focused).map_or(14., |pane| pane.read(cx).active_font_size(cx));
        let pane = cx.new(|cx| sample_pane::SamplePane::new(font_size, window,cx));
        self.register_pane(id, pane, window, cx);
    }
    fn register_pane(&mut self, id: usize, pane: Entity<sample_pane::SamplePane>, window: &mut Window, cx: &mut Context<Self>) {
        let scope = cx.entity_id();
        pane.update(cx, |pane, _| pane.scope = Some(scope));
        cx.subscribe_in(&pane, window, move |this, destination, event: &sample_pane::TabDrop, window, cx| {
            let Some((&source_id, source)) = this.samples.iter().find(|(_, pane)| pane.entity_id() == event.source) else { return; };
            if source_id == id || this.samples.get(&id).map(Entity::entity_id) != Some(destination.entity_id()) { return; }
            let source = source.clone();
            let Some(index) = destination.read(cx).insertion_index(event.target) else { return; };
            // Validate both owners before mutation, then transfer without yielding.
            let Some(transfer) = source.update(cx, |pane, cx| pane.take_tab(event.terminal, cx)) else { return; };
            destination.update(cx, |pane, cx| pane.insert_tab(transfer, index, window, cx));
            if source.read(cx).tab_count() == 0 {
                this.pane_state.remove(&source_id);
                this.samples.remove(&source_id);
            }
            this.split_drag = None;
            this.focused = id;
            this.pane_state.focus_changed(id);
            cx.notify();
        }).detach();
        cx.subscribe_in(&pane, window, move |this, _, _: &sample_pane::PaneEmpty, window, cx| {
            this.split_drag = None;
            let next = this.pane_state.layout.as_ref().and_then(|tree|
                rmux_core::pane_commands::focus_after_close(tree, &id, Some(&this.focused)));
            this.pane_state.remove(&id);
            this.samples.remove(&id);
            if this.pane_state.layout.is_none() { this.empty_focus.focus(window,cx); }
            if let Some(next) = next.filter(|next| *next != this.focused) {
                this.focused = next;
                this.samples[&next].update(cx, |pane, cx| pane.focus(window, cx));
            }
            cx.notify();
        }).detach();
        self.samples.insert(id, pane);
    }
    fn start_terminal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let id = self.next_pane;
        self.next_pane += 1;
        self.add_pane(id, window, cx);
        self.pane_state.layout = Some(PaneTree::Pane(id));
        self.focused = id;
        self.close_palette(window, cx);
    }

    fn break_tab(&mut self, axis: Axis, before: bool, was_dragging: bool, window: &mut Window, cx: &mut Context<Self>) {
        if was_dragging {
            self.feedback = Some("Finish resizing before moving tabs.".into());
            cx.notify(); return;
        }
        let Some(source) = self.samples.get(&self.focused).cloned() else { return; };
        if source.read(cx).tab_count() < 2 { return; }
        if self.samples.len() >= 16 {
            self.feedback = Some("Demo limit: 16 panes".into());
            cx.notify(); return;
        }
        let id = self.next_pane;
        let previous_zoom = self.pane_state.zoomed;
        if self.pane_state.insert( Some(&self.focused), id, axis, before).is_none() {
            self.feedback = Some("Cannot add another split at this nesting depth.".into());
            cx.notify(); return;
        }
        // The destination is accepted before releasing the source's ownership.
        // No terminal is spawned and no event-loop yield occurs during transfer.
        let Some(transfer) = source.update(cx, |pane, cx| pane.take_active_for_split(cx)) else {
            self.pane_state.remove(&id);
            self.pane_state.zoomed = previous_zoom;
            return;
        };
        let pane = cx.new(|_| sample_pane::SamplePane::from_transfer(transfer));
        self.register_pane(id, pane, window, cx);
        self.focused = id;
        self.next_pane += 1;
        self.pane_state.zoomed = None;
        cx.notify();
    }

    fn join_pane(&mut self, direction: Option<rmux_core::layout::Direction>, was_dragging: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(tree) = &self.pane_state.layout else { return; };
        let target = match rmux_core::pane_commands::join_target(tree, &self.focused, direction,
            rmux_core::pane_commands::JoinState { zoomed: self.pane_state.zoomed.is_some(), resizing: was_dragging, splitting: false }) {
            Ok(Some(target)) => target,
            Ok(None) => return,
            Err(message) => { self.feedback = Some(message.into()); cx.notify(); return; }
        };
        let source = self.samples[&self.focused].clone();
        let destination = self.samples[&target].clone();
        if source.read(cx).tab_count() + destination.read(cx).tab_count() > 16 {
            self.feedback = Some("Joining would exceed the demo limit of 16 tabs per pane.".into());
            cx.notify(); return;
        }
        // No event-loop yield between validation and ownership transfer. Keep
        // terminal entities/labels alive and focus the source's active tab once.
        let tabs = source.update(cx, |pane, cx| pane.take_all_tabs(cx));
        destination.update(cx, |pane, cx| pane.append_tabs(tabs, window, cx));
        self.pane_state.remove(&self.focused);
        self.samples.remove(&self.focused);
        self.focused = target;
        self.pane_state.focus_changed(target);
        cx.notify();
    }

    fn cycle_pane(&mut self, direction: isize, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(next) = self.pane_state.layout.as_ref().and_then(|tree|
            rmux_core::pane_commands::cycle(tree, Some(&self.focused), direction)) {
            self.focused = next;
            self.pane_state.focus_changed(next);
            self.samples[&next].update(cx, |sample, cx| sample.focus(window, cx));
            cx.notify();
        }
    }

    fn focus_direction(&mut self, direction: rmux_core::layout::Direction, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(next) = self.pane_state.layout.as_ref().and_then(|tree| tree.neighbor(&self.focused, direction)) {
            self.focused = next;
            self.pane_state.focus_changed(next);
            self.samples[&next].update(cx, |pane, cx| pane.focus(window, cx));
            cx.notify();
        }
    }
    fn close_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.palette_open = false;
        self.focus_terminal(window, cx);
        cx.notify();
    }
    fn focus_terminal(&self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(pane) = self.samples.get(&self.focused) { pane.update(cx, |sample, cx| sample.focus(window, cx)); }
        else { self.empty_focus.focus(window,cx); }
    }
    fn set_workspace_results(&mut self, workspaces: Vec<(EntityId, String)>, groups: Vec<(u64, String)>, window: &Window, cx: &mut Context<Self>) {
        self.workspace_results = workspaces;
        self.group_results = groups;
        // Like the native palette, resolve bindings once from the originating
        // focus path, before later frames replace it with the palette input.
        self.result_hints = (0..COMMANDS.len() + self.workspace_results.len())
            .map(|index| self.result_hint(index, window, cx)).collect();
        self.search = palette_match::RankedSearch::new(COMMANDS.iter().map(|command| command.title().to_lowercase())
            .chain(self.workspace_results.iter().map(|(_, title)| title.to_lowercase()))
            .chain(self.group_results.iter().map(|(_, title)| title.to_lowercase())).collect());
        self.selected = 0;
        self.scroll.reset(self.search.matches.len());
        cx.notify();
    }
    fn result_hint(&self, index: usize, window: &Window, _cx: &App) -> Option<String> {
        let action: Box<dyn Action> = if let Some(command) = COMMANDS.get(index) {
            if let Some(action) = tab_actions::action(*command).or_else(|| terminal_actions::action(*command)).or_else(|| pane_actions::action(*command)).or_else(|| workspace_actions::action(*command)).or_else(|| find_presentation::action(*command)) { action } else { match command {
                CommandId::ToggleSidebar => Box::new(ToggleSidebar),
                CommandId::NewWorkspace => Box::new(NewWorkspace),
                CommandId::NewWorkspaceGroup => Box::new(NewWorkspaceGroup),
                CommandId::LastWorkspace => Box::new(LastWorkspace),
                CommandId::NextWorkspace => Box::new(NextWorkspace),
                CommandId::PreviousWorkspace => Box::new(PreviousWorkspace),
                _ => return None,
            } }
        } else {
            let position = index.checked_sub(COMMANDS.len())?;
            if position >= self.workspace_results.len() { return None; }
            let number = if position < 8 { position + 1 }
                else if position + 1 == self.workspace_results.len() { 9 }
                else { return None; };
            Box::new(SelectWorkspaceNumber { number })
        };
        let focus = self.palette_origin.as_ref()?;
        window.highest_precedence_binding_for_action_in(action.as_ref(), focus).map(|binding| {
            let mac = terminal_engine::is_mac();
            binding.keystrokes().iter().map(|key|
                shortcut_label::format_key_parts(key.key(), *key.modifiers(), mac)).collect::<Vec<_>>().join(" ")
        })
    }
    fn result_title(&self, index: usize) -> &str {
        if let Some(command) = COMMANDS.get(index) { command.title() }
        else if index < COMMANDS.len() + self.workspace_results.len() { &self.workspace_results[index - COMMANDS.len()].1 }
        else { &self.group_results[index - COMMANDS.len() - self.workspace_results.len()].1 }
    }
    fn open_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.palette_open { self.close_palette(window, cx); return; }
        self.palette_origin = window.focused(cx);
        self.palette_open = true;
        cx.emit(PaletteRequest::Open);
        self.input.update(cx, |input, cx| { input.replace_draft("", cx); input.focus.focus(window, cx); });
        cx.notify();
    }

    fn choose(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(&index) = self.search.matches.get(self.selected) else { return; };
        if index >= COMMANDS.len() + self.workspace_results.len() {
            let Some((group, _)) = self.group_results.get(index - COMMANDS.len() - self.workspace_results.len()) else { return; };
            let action = MoveWorkspaceToGroup { group: *group };
            self.close_palette(window, cx);
            window.dispatch_action(Box::new(action), cx);
            return;
        }
        if index >= COMMANDS.len() {
            let Some((id, _)) = self.workspace_results.get(index - COMMANDS.len()) else { return; };
            let id = *id;
            self.close_palette(window, cx);
            cx.emit(PaletteRequest::SelectWorkspace(id));
            return;
        }
        let command = COMMANDS[index];
        if let Some(action) = pane_actions::action(command).or_else(|| workspace_actions::action(command)).or_else(|| find_presentation::action(command)) {
            self.close_palette(window, cx);
            window.dispatch_action(action, cx);
        } else { self.execute(command, window, cx); }
    }
    fn execute(&mut self, command: CommandId, window: &mut Window, cx: &mut Context<Self>) {
        let was_dragging = self.split_drag.is_some();
        self.split_drag = None;
        self.feedback = None;
        if let Some(action) = tab_actions::action(command).or_else(|| terminal_actions::action(command)) {
            // Restore terminal focus before dispatching, just like the native
            // palette. Actions then use the same handlers as keyboard input.
            self.close_palette(window, cx);
            if command == CommandId::NewTab && self.pane_state.layout.is_none() {
                self.start_terminal(window, cx);
            } else {
                window.dispatch_action(action, cx);
            }
            return;
        }
        match command {
            CommandId::ShowTerminalMenu => { self.close_palette(window, cx); window.dispatch_action(Box::new(ShowTerminalMenu), cx); return; }
            command @ (CommandId::BreakTabRight | CommandId::BreakTabBelow | CommandId::BreakTabLeft | CommandId::BreakTabAbove) => {
                let axis = if matches!(command, CommandId::BreakTabRight | CommandId::BreakTabLeft) { Axis::Columns } else { Axis::Rows };
                let before = matches!(command, CommandId::BreakTabLeft | CommandId::BreakTabAbove);
                self.break_tab(axis, before, was_dragging, window, cx);
            }
            command @ (CommandId::JoinPaneNext | CommandId::JoinPaneLeft | CommandId::JoinPaneRight | CommandId::JoinPaneUp | CommandId::JoinPaneDown) => {
                use rmux_core::layout::Direction;
                let direction = match command {
                    CommandId::JoinPaneLeft => Some(Direction::Left),
                    CommandId::JoinPaneRight => Some(Direction::Right),
                    CommandId::JoinPaneUp => Some(Direction::Up),
                    CommandId::JoinPaneDown => Some(Direction::Down),
                    _ => None,
                };
                self.join_pane(direction, was_dragging, window, cx);
            }
            CommandId::ScrollToTop | CommandId::ScrollToBottom => {
                let scroll = if command == CommandId::ScrollToTop { rmux_core::scroll::Scroll::Top } else { rmux_core::scroll::Scroll::Bottom };
                if let Some(pane) = self.samples.get(&self.focused) { pane.update(cx, |pane,cx| pane.scroll(scroll,cx)); }
            }
            CommandId::Split | CommandId::SplitBelow | CommandId::SplitLeft | CommandId::SplitAbove => {
                if self.samples.len() >= 16 { self.feedback = Some("Demo limit: 16 panes".into()); cx.notify(); return; }
                let axis = if matches!(command, CommandId::Split | CommandId::SplitLeft) { Axis::Columns } else { Axis::Rows };
                let before = matches!(command, CommandId::SplitLeft | CommandId::SplitAbove);
                if self.pane_state.layout.is_none() { self.start_terminal(window, cx); return; }
                if self.pane_state.insert( Some(&self.focused), self.next_pane, axis, before).is_some() {
                    self.add_pane(self.next_pane, window, cx);
                    self.focused = self.next_pane; self.next_pane += 1; self.pane_state.zoomed = None;
                }
            }
            command @ (CommandId::FocusLeft | CommandId::FocusRight | CommandId::FocusUp | CommandId::FocusDown) => {
                use rmux_core::layout::Direction;
                let direction = match command {
                    CommandId::FocusLeft => Direction::Left, CommandId::FocusRight => Direction::Right,
                    CommandId::FocusUp => Direction::Up, _ => Direction::Down,
                };
                self.focus_direction(direction, window, cx);
            }
            command @ (CommandId::SwapPaneNext | CommandId::SwapPaneLeft | CommandId::SwapPaneRight | CommandId::SwapPaneUp | CommandId::SwapPaneDown) => {
                use rmux_core::layout::Direction;
                let direction = match command {
                    CommandId::SwapPaneLeft => Some(Direction::Left),
                    CommandId::SwapPaneRight => Some(Direction::Right),
                    CommandId::SwapPaneUp => Some(Direction::Up),
                    CommandId::SwapPaneDown => Some(Direction::Down),
                    _ => None,
                };
                self.pane_state.swap_direction(&self.focused, direction);
            }
            CommandId::EqualizePanes => {
                self.pane_state.equalize();
            }
            command @ (CommandId::ResizePaneLeft | CommandId::ResizePaneRight | CommandId::ResizePaneUp | CommandId::ResizePaneDown) => {
                use rmux_core::layout::Direction;
                let direction = match command {
                    CommandId::ResizePaneLeft => Direction::Left,
                    CommandId::ResizePaneRight => Direction::Right,
                    CommandId::ResizePaneUp => Direction::Up,
                    _ => Direction::Down,
                };
                self.pane_state.resize(&self.focused, direction);
            }
            CommandId::TogglePaneZoom => self.pane_state.toggle_zoom(
                self.samples.contains_key(&self.focused).then_some(self.focused)),
            command @ (CommandId::ToggleSidebar | CommandId::CloseWorkspace | CommandId::NewWorkspace | CommandId::NewWorkspaceGroup
                | CommandId::MoveWorkspaceUp | CommandId::MoveWorkspaceDown | CommandId::ToggleGroupPin | CommandId::RemoveWorkspaceFromGroup | CommandId::SetGroupAnchor | CommandId::GroupCurrentWorkspace | CommandId::GroupSelectedWorkspaces | CommandId::LastWorkspace | CommandId::NextWorkspace | CommandId::PreviousWorkspace | CommandId::UngroupWorkspaces) => {
                self.close_palette(window, cx);
                cx.emit(WorkspaceCommand(command));
                return;
            }
            _ => return,
        }
        self.close_palette(window, cx);
    }
    fn panes(&self, tree: &PaneTree<usize>, path: Vec<usize>, cx: &mut Context<Self>) -> AnyElement {
        split_interaction::render_panes(tree, path, false, cx, &|id, cx| {
                let id = *id;
                split_presentation::leaf(("pane", id))
                    .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, window, cx| { this.focused = id; this.pane_state.focus_changed(id); this.samples[&id].update(cx, |sample, cx| sample.focus(window, cx)); }))
                    .child(self.samples[&id].clone())
                    .into_any_element()
        })
    }

}
impl pane_actions::PaneCommandHost for InputProbe {
    fn select_pane_command(&mut self, number: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(target) = number.checked_sub(1).and_then(|index| self.pane_state.layout.as_ref()?.pane_at(index)).copied() else { return; };
        self.focused = target;
        self.pane_state.focus_changed(target);
        self.samples[&target].update(cx, |pane, cx| pane.focus(window, cx));
        cx.notify();
    }
    fn run_pane_command(&mut self, command: CommandId, window: &mut Window, cx: &mut Context<Self>) {
        self.execute(command, window, cx);
    }
    fn cycle_pane_command(&mut self, direction: isize, window: &mut Window, cx: &mut Context<Self>) {
        self.cycle_pane(direction, window, cx);
    }
}
impl Render for InputProbe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Reclaim only on visibility transitions, not every output repaint.
        // Focus navigation can change the zoom target without toggling zoom.
        if self.rendered_zoom != self.pane_state.zoomed {
            if let Some(visible) = self.pane_state.zoomed {
                for (id, pane) in &self.samples {
                    if *id != visible { pane.update(cx, |pane, cx| pane.release_render_caches(cx)); }
                }
            }
            self.rendered_zoom = self.pane_state.zoomed;
        }
        let height = f32::from(window.viewport_size().height);
        let panes = if let Some(tree) = &self.pane_state.layout {
            if let Some(zoomed) = self.pane_state.zoomed { self.panes(&PaneTree::Pane(zoomed), vec![], cx) } else { self.panes(tree, vec![], cx) }
        } else {
            div().size_full().flex().items_center().justify_center().track_focus(&self.empty_focus).key_context("Terminal")
                .child(div().id("restart-terminal").px_4().py_2().rounded_md().bg(rgb(0x303030)).cursor_pointer()
                    .child("New Terminal").on_click(cx.listener(|this,_,window,cx| this.start_terminal(window,cx))))
                .into_any_element()
        };
        div().font_family(FONT_FAMILY).size_full().bg(rgb(0x151517)).text_color(rgb(0xe2e3e7)).relative().flex().flex_col()
            .on_action(cx.listener(|this, _: &NewTab, window, cx| {
                if this.pane_state.layout.is_none() { this.start_terminal(window,cx); }
            }))
            .map(|root| pane_actions::bind(root, cx))
            .on_action(cx.listener(|this, _: &CommandPalette, window, cx| this.open_palette(window, cx)))
            .child(workspace_presentation::toolbar()
                .child(workspace_presentation::sidebar_toggle())
                .child(workspace_presentation::title("workspace-title", self.name.clone()))
                .child(div().text_xs().text_color(rgb(0x9b9ba3)).child("Sample sessions"))
                .child(div().id("open-commands").px_2().py_1().rounded_sm().text_size(px(13.)).cursor_pointer().hover(|style| style.bg(rgb(0x353535))).child("Commands")
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.open_palette(window, cx);
                    }))))
            .when(self.pane_state.zoomed.is_some(), |root| {
                let shortcut = window.focused(cx).and_then(|focus|
                    window.highest_precedence_binding_for_action_in(&TogglePaneZoom, &focus))
                    .map(|binding| binding.keystrokes().iter().map(|key|
                        shortcut_label::format_key_parts(key.key(), *key.modifiers(), terminal_engine::is_mac()))
                        .collect::<Vec<_>>().join(" "));
                root.child(split_presentation::restore_layout(self.samples.len().saturating_sub(1), shortcut, cx))
            })
            .child(div().flex_1().min_h_0().relative().child(split_interaction::drag_capture(cx)).child(panes))
            .when_some(self.feedback.clone().filter(|_| !self.palette_open), |root, message|
                root.child(div().flex_shrink_0().px_3().py_2().text_sm().text_color(rgb(0xf0b85b)).child(message)))
            .when(self.palette_open, |root| root.child(div().absolute().inset_0().bg(rgba(0x00000066))
                .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| {
                    cx.stop_propagation();
                    this.close_palette(window, cx);
                }))
                .flex().items_start().justify_center().pt(px(palette_view::top_inset(height)))
                .child(palette_view::panel(height)
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                    if this.input.read(cx).is_composing() { return; }
                    let key = event.keystroke.key.as_str();
                    let next = if matches!(key, "pageup" | "pagedown") {
                        palette_view::paged_result(this.selected, this.search.matches.len(), key == "pageup",
                            f32::from(this.scroll.viewport_bounds().size.height), |_| 36.)
                    } else { palette_view::navigated_result(this.selected, this.search.matches.len(), key) };
                    if let Some(next) = next {
                        this.selected = next;
                        list_navigation::reveal(&this.scroll, next);
                        this.feedback = None; cx.notify(); cx.stop_propagation();
                    }
                }))
                .child(palette_view::header("Search commands and workspaces", palette_view::close_button()
                    .on_click(cx.listener(|this, event: &ClickEvent, window, cx| {
                        if event.standard_click() { this.close_palette(window, cx); }
                        cx.stop_propagation();
                    }))))
                .child(palette_view::search(self.input.clone()))
                .child({
                    let entity = cx.entity();
                    list(self.scroll.clone(), move |row, _window, cx| {
                        entity.update(cx, |this, cx| {
                            let index = this.search.matches[row];
                            palette_view::result(index, 36., row == this.selected)
                                .child(div().flex_1().min_w_0().truncate().child({
                                    let title = this.result_title(index).to_owned();
                                    let matches = palette_match::highlights(&title, this.input.read(cx).value());
                                    StyledText::new(title).with_highlights(matches.into_iter().map(|range| (range,
                                        HighlightStyle { color: Some(rgb_to_hsla(rgb(0xffffff))), background_color: Some(rgb_to_hsla(rgb(0x41638c))), ..Default::default() })))
                                }))
                                .when_some(this.result_hints.get(index).cloned().flatten(), |row, hint| row.child(
                                    div().flex_shrink_0().text_xs().text_color(rgb(0xa0a0a0)).child(hint)))
                                .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
                                    if event.standard_click() { this.selected = row; this.choose(window, cx); }
                                })).into_any_element()
                        })
                    }).with_sizing_behavior(ListSizingBehavior::Auto).w_full().min_h_0()
                        .h(px((self.search.matches.len() as f32 * 36.).min(320.)))
                })
                .when(self.search.matches.is_empty(), |panel| panel.child(div().p_3().text_sm().child("No matching commands")))
                .child(palette_view::footer().child(self.feedback.clone().unwrap_or_else(|| "↑↓ Navigate     ↵ Run     Esc Close".into()))))))
    }
}
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    #[cfg(target_family = "wasm")]
    gpui_platform::web_init();
    gpui_platform::application().with_assets(icons::Assets).run(|cx| {
        cx.set_global(fonts::Fonts::default());
        cx.text_system().add_fonts(vec![
            std::borrow::Cow::Borrowed(include_bytes!("../assets/Inter-Regular.ttf").as_slice()),
            std::borrow::Cow::Borrowed(include_bytes!("../assets/FiraMono-Medium.ttf").as_slice()),
        ]).expect("bundled UI and terminal fonts");
        let mac = terminal_engine::is_mac();
        cx.bind_keys(workspace_shortcuts::bindings(mac));
        cx.bind_keys(find_presentation::bindings(mac));
        cx.bind_keys(pane_shortcuts::tab_numbers(mac));
        cx.bind_keys(pane_shortcuts::pane_numbers(mac));
        cx.bind_keys(pane_shortcuts::directions(mac));
        cx.bind_keys(pane_shortcuts::splits(mac));
        cx.bind_keys(pane_shortcuts::tab_reordering());
        cx.bind_keys([
            KeyBinding::new(if mac { "cmd-=" } else { "ctrl-=" }, IncreaseFontSize, Some("Terminal")),
            KeyBinding::new(if mac { "cmd--" } else { "ctrl--" }, DecreaseFontSize, Some("Terminal")),
            KeyBinding::new(if mac { "cmd-0" } else { "ctrl-0" }, ResetFontSize, Some("Terminal")),
            KeyBinding::new(if mac { "cmd-c" } else { "ctrl-shift-c" }, CopySelection, Some("Terminal")),
            KeyBinding::new("ctrl-tab", NextPane, Some("Terminal")),
            KeyBinding::new("ctrl-shift-tab", PreviousPane, Some("Terminal")),
        ]);
        if mac {
            cx.bind_keys(pane_shortcuts::brackets());
            cx.bind_keys(pane_shortcuts::tabs());
            cx.bind_keys([
                KeyBinding::new("cmd-shift-{", PreviousTab, Some("Terminal")),
                KeyBinding::new("cmd-shift-}", NextTab, Some("Terminal")),
            ]);
        } else {
            cx.bind_keys([
                KeyBinding::new("ctrl-pageup", PreviousTab, Some("Terminal")),
                KeyBinding::new("ctrl-pagedown", NextTab, Some("Terminal")),
                KeyBinding::new("ctrl-y", EditorRedo, Some("TextInput")),
            ]);
        }
        cx.bind_keys([
            KeyBinding::new(if mac { "cmd-p" } else { "ctrl-shift-p" }, CommandPalette, None),
            KeyBinding::new(if mac { "cmd-a" } else { "ctrl-shift-a" }, EditorSelectAll, Some("Terminal")),
            KeyBinding::new(if mac { "cmd-shift-r" } else { "ctrl-shift-r" }, RenameWorkspace, None),
            KeyBinding::new("shift-f10", ShowTabMenu, Some("Terminal")),
            KeyBinding::new(if mac { "cmd-t" } else { "ctrl-shift-t" }, NewTab, Some("Terminal")),
            KeyBinding::new(if mac { "cmd-w" } else { "ctrl-shift-w" }, ClosePane, Some("Terminal")),
            KeyBinding::new(if mac { "cmd-a" } else { "ctrl-a" }, EditorSelectAll, Some("TextInput")),
            KeyBinding::new(if mac { "cmd-z" } else { "ctrl-z" }, EditorUndo, Some("TextInput")),
            KeyBinding::new(if mac { "cmd-shift-z" } else { "ctrl-shift-z" }, EditorRedo, Some("TextInput")),
            KeyBinding::new(if mac { "cmd-v" } else { "ctrl-v" }, Paste, Some("TextInput")),
            KeyBinding::new(if mac { "cmd-c" } else { "ctrl-c" }, CopySelection, Some("TextInput")),
            KeyBinding::new(if mac { "cmd-x" } else { "ctrl-x" }, EditorCut, Some("TextInput")),
        ]);
        cx.open_window(WindowOptions::default(), |window, cx| cx.new(|cx| browser_host::BrowserHost::new(window, cx))).expect("open probe window");
    });
}

#[path = "../../../crates/rmux-desktop/src/row_cache.rs"]
mod row_cache;
#[path = "../../../crates/rmux-desktop/src/cell_metrics.rs"]
mod cell_metrics;
mod row_host;
mod terminal_sample;

mod terminal_engine;
#[path = "../../../crates/rmux-desktop/src/cursor_view.rs"]
mod cursor_view;

#[path = "../../../crates/rmux-desktop/src/pane_shortcuts.rs"]
mod pane_shortcuts;

#[path = "../../../crates/rmux-desktop/src/tab_presentation.rs"]
mod tab_presentation;
mod sample_pane;

mod key_adapter;

#[path = "../../../crates/rmux-desktop/src/sidebar_presentation.rs"]
mod sidebar_presentation;
#[path = "../../../crates/rmux-desktop/src/metadata_presets.rs"]
mod metadata_presets;
#[path = "../../../crates/rmux-desktop/src/workspace_menu_view.rs"]
mod workspace_menu_view;
#[path = "../../../crates/rmux-desktop/src/drag_item.rs"]
mod drag_item;
#[path = "../../../crates/rmux-desktop/src/sidebar_interaction.rs"]
mod sidebar_interaction;
#[path = "../../../crates/rmux-desktop/src/workspace_presentation.rs"]
mod workspace_presentation;
mod browser_host;

#[path = "../../../crates/rmux-desktop/src/workspace_shortcuts.rs"]
mod workspace_shortcuts;

#[path = "../../../crates/rmux-desktop/src/list_navigation.rs"]
mod list_navigation;

#[path = "../../../crates/rmux-desktop/src/split_interaction.rs"]
mod split_interaction;
impl split_interaction::SplitHost for InputProbe {
    type Pane = usize;
    fn split_state(&mut self) -> (&mut Option<PaneTree<usize>>, &mut Option<split_interaction::SplitDrag>) {
        (&mut self.pane_state.layout, &mut self.split_drag)
    }
}

#[path = "../../../crates/rmux-desktop/src/shortcut_label.rs"]
mod shortcut_label;

#[path = "../../../crates/rmux-desktop/src/pane_presentation.rs"]
mod pane_presentation;
#[path = "../../../crates/rmux-desktop/src/icons.rs"]
mod icons;
