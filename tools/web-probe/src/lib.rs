//! Compile the actual desktop text input against a candidate web backend.
//! This is a compatibility probe, not a replacement demo or second UI.
use gpui::{prelude::*, *};
use std::ops::Range;
const FONT_FAMILY: &str = "Inter";
#[path = "../../../crates/rmux-desktop/src/control_hint.rs"]
mod control_hint;
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
#[path = "../../../crates/rmux-desktop/src/palette_view.rs"]
mod palette_view;
use rmux_core::{palette_match, commands::CommandId, layout::{PaneTree, Axis}};
#[path = "../../../crates/rmux-desktop/src/split_presentation.rs"]
mod split_presentation;
const COMMANDS: &[CommandId] = &[
    CommandId::NewWorkspace, CommandId::NewTab, CommandId::ToggleSidebar,
    CommandId::NextTab, CommandId::PreviousTab, CommandId::MoveTabLeft, CommandId::MoveTabRight,
    CommandId::ClosePane, CommandId::NewWorkspaceGroup, CommandId::CloseWorkspace,
    CommandId::NextWorkspace, CommandId::PreviousWorkspace, CommandId::LastWorkspace, CommandId::UngroupWorkspaces,
    CommandId::Split, CommandId::SplitBelow, CommandId::TogglePaneZoom,
    CommandId::EqualizePanes, CommandId::ResizePaneLeft, CommandId::ResizePaneRight,
    CommandId::ResizePaneUp, CommandId::ResizePaneDown,
    CommandId::FocusLeft, CommandId::FocusRight, CommandId::FocusUp, CommandId::FocusDown,
    CommandId::ScrollToTop, CommandId::ScrollToBottom,
];
struct WorkspaceCommand(CommandId);
impl EventEmitter<WorkspaceCommand> for InputProbe {}
enum PaletteRequest { Open, SelectWorkspace(EntityId) }
impl EventEmitter<PaletteRequest> for InputProbe {}


actions!(rmux, [EditorUndo, EditorRedo, Paste, CopySelection, EditorCut, EditorSelectAll, CommandPalette, NextPane, PreviousPane, NewTab, NextTab, PreviousTab, MoveTabLeft, MoveTabRight, ClosePane, NewWorkspaceGroup, NewWorkspace, ToggleSidebar, NextWorkspace, PreviousWorkspace, LastWorkspace, FocusLeft, FocusRight, FocusUp, FocusDown, Split, SplitBelow, TogglePaneZoom]);
fn tab_action(command: CommandId) -> Option<Box<dyn Action>> {
    Some(match command {
        CommandId::NewTab => Box::new(NewTab),
        CommandId::NextTab => Box::new(NextTab),
        CommandId::PreviousTab => Box::new(PreviousTab),
        CommandId::MoveTabLeft => Box::new(MoveTabLeft),
        CommandId::MoveTabRight => Box::new(MoveTabRight),
        CommandId::ClosePane => Box::new(ClosePane),
        _ => return None,
    })
}
#[derive(Clone, Debug, Default, PartialEq, Action)]
#[action(namespace = rmux, no_json)]
struct SelectWorkspaceNumber { number: usize }
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
    selected: usize,
    scroll: ListState,
    feedback: Option<String>,
    tree: Option<PaneTree<usize>>,
    focused: usize,
    next_pane: usize,
    zoomed: bool,
    split_drag: Option<split_interaction::SplitDrag>,
    palette_open: bool,
    palette_origin: Option<FocusHandle>,
    samples: std::collections::HashMap<usize, Entity<sample_pane::SamplePane>>,
}
impl InputProbe {
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
            let mut host = InputProbe { name, empty_focus: cx.focus_handle(), palette_origin: None, split_drag: None, scroll: ListState::new(COMMANDS.len(), ListAlignment::Top, px(96.)), workspace_results: Vec::new(), samples, input, search: palette_match::RankedSearch::new(COMMANDS.iter().map(|title| title.title().to_lowercase()).collect()), selected: 0, feedback: None, tree: Some(PaneTree::Pane(0)), focused: 0, next_pane: 1, zoomed: false, palette_open: false };
            host.add_pane(0, window, cx);
            host

    }

    fn add_pane(&mut self, id: usize, window: &mut Window, cx: &mut Context<Self>) {
        let pane = cx.new(|cx| sample_pane::SamplePane::new(window,cx));
        cx.subscribe_in(&pane, window, move |this, _, _: &sample_pane::PaneEmpty, window, cx| {
            this.split_drag = None;
            let old_index = this.tree.as_ref().and_then(|tree| tree.pane_index(&id)).unwrap_or(0);
            this.tree = this.tree.take().and_then(|tree| tree.remove(&id));
            this.samples.remove(&id);
            if this.tree.is_none() { this.empty_focus.focus(window,cx); }
            this.zoomed = false;
            if this.focused == id {
                let mut ordered: Vec<_> = this.samples.keys().copied().collect();
                ordered.sort_by_key(|id| this.tree.as_ref().and_then(|tree| tree.pane_index(id)));
                if let Some(next) = ordered.get(old_index.min(ordered.len().saturating_sub(1))) {
                    this.focused = *next;
                    this.samples[next].update(cx, |pane, cx| pane.focus(window, cx));
                }
            }
            cx.notify();
        }).detach();
        self.samples.insert(id, pane);
    }
    fn start_terminal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let id = self.next_pane;
        self.next_pane += 1;
        self.add_pane(id, window, cx);
        self.tree = Some(PaneTree::Pane(id));
        self.focused = id;
        self.close_palette(window, cx);
    }

    fn cycle_pane(&mut self, direction: isize, window: &mut Window, cx: &mut Context<Self>) {
        let mut ordered: Vec<_> = self.samples.keys().copied().collect();
        ordered.sort_by_key(|id| self.tree.as_ref().and_then(|tree| tree.pane_index(id)));
        let current = ordered.iter().position(|id| *id == self.focused).unwrap_or(0);
        if let Some(next) = rmux_core::layout::cycle_index(current, ordered.len(), direction) {
            self.focused = ordered[next];
            self.samples[&self.focused].update(cx, |sample, cx| sample.focus(window, cx));
            cx.notify();
        }
    }

    fn focus_direction(&mut self, direction: rmux_core::layout::Direction, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(next) = self.tree.as_ref().and_then(|tree| tree.neighbor(&self.focused, direction)) {
            self.focused = next;
            self.samples[&next].update(cx, |pane, cx| pane.focus(window, cx));
            cx.notify();
        }
    }
    fn close_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.palette_open = false;
        if let Some(pane) = self.samples.get(&self.focused) { pane.update(cx, |sample, cx| sample.focus(window, cx)); }
        else { self.empty_focus.focus(window,cx); }
        cx.notify();
    }
    fn set_workspace_results(&mut self, workspaces: Vec<(EntityId, String)>, cx: &mut Context<Self>) {
        self.workspace_results = workspaces;
        self.search = palette_match::RankedSearch::new(COMMANDS.iter().map(|command| command.title().to_lowercase())
            .chain(self.workspace_results.iter().map(|(_, title)| title.to_lowercase())).collect());
        self.selected = 0;
        self.scroll.reset(self.search.matches.len());
        cx.notify();
    }
    fn result_hint(&self, index: usize, window: &Window, _cx: &App) -> Option<String> {
        let action: Box<dyn Action> = if let Some(command) = COMMANDS.get(index) {
            if let Some(action) = tab_action(*command) { action } else { match command {
                CommandId::ToggleSidebar => Box::new(ToggleSidebar),
                CommandId::NewWorkspace => Box::new(NewWorkspace),
                CommandId::NewWorkspaceGroup => Box::new(NewWorkspaceGroup),
                CommandId::LastWorkspace => Box::new(LastWorkspace),
                CommandId::NextWorkspace => Box::new(NextWorkspace),
                CommandId::PreviousWorkspace => Box::new(PreviousWorkspace),
                CommandId::Split => Box::new(Split),
                CommandId::SplitBelow => Box::new(SplitBelow),
                CommandId::TogglePaneZoom => Box::new(TogglePaneZoom),
                CommandId::FocusLeft => Box::new(FocusLeft),
                CommandId::FocusRight => Box::new(FocusRight),
                CommandId::FocusUp => Box::new(FocusUp),
                CommandId::FocusDown => Box::new(FocusDown),
                _ => return None,
            } }
        } else {
            let position = index.checked_sub(COMMANDS.len())?;
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
        else { &self.workspace_results[index - COMMANDS.len()].1 }
    }
    fn open_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.palette_origin = window.focused(cx);
        self.palette_open = true;
        cx.emit(PaletteRequest::Open);
        self.input.update(cx, |input, cx| { input.replace_draft("", cx); input.focus.focus(window, cx); });
        cx.notify();
    }

    fn choose(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(&index) = self.search.matches.get(self.selected) else { return; };
        if index >= COMMANDS.len() {
            let Some((id, _)) = self.workspace_results.get(index - COMMANDS.len()) else { return; };
            let id = *id;
            self.close_palette(window, cx);
            cx.emit(PaletteRequest::SelectWorkspace(id));
            return;
        }
        self.execute(COMMANDS[index], window, cx);
    }
    fn execute(&mut self, command: CommandId, window: &mut Window, cx: &mut Context<Self>) {
        self.split_drag = None;
        if let Some(action) = tab_action(command) {
            // Restore terminal focus before dispatching, just like the native
            // palette. Actions then use the same handlers as keyboard input.
            self.close_palette(window, cx);
            if command == CommandId::NewTab && self.tree.is_none() {
                self.start_terminal(window, cx);
            } else {
                window.dispatch_action(action, cx);
            }
            return;
        }
        match command {
            CommandId::ScrollToTop | CommandId::ScrollToBottom => {
                let scroll = if command == CommandId::ScrollToTop { rmux_core::scroll::Scroll::Top } else { rmux_core::scroll::Scroll::Bottom };
                if let Some(pane) = self.samples.get(&self.focused) { pane.update(cx, |pane,cx| pane.scroll(scroll,cx)); }
            }
            CommandId::Split | CommandId::SplitBelow => {
                if self.samples.len() >= 16 { self.feedback = Some("Probe limit: 16 panes".into()); cx.notify(); return; }
                let axis = if command == CommandId::Split { Axis::Columns } else { Axis::Rows };
                if self.tree.is_none() { self.start_terminal(window, cx); return; }
                if self.tree.as_mut().is_some_and(|tree| tree.split_positioned(&self.focused, self.next_pane, axis, false)) {
                    self.add_pane(self.next_pane, window, cx);
                    self.focused = self.next_pane; self.next_pane += 1; self.zoomed = false;
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
            CommandId::EqualizePanes => { if let Some(tree) = &mut self.tree { tree.equalize(); } }
            command @ (CommandId::ResizePaneLeft | CommandId::ResizePaneRight | CommandId::ResizePaneUp | CommandId::ResizePaneDown) => {
                use rmux_core::layout::Direction;
                let direction = match command {
                    CommandId::ResizePaneLeft => Direction::Left,
                    CommandId::ResizePaneRight => Direction::Right,
                    CommandId::ResizePaneUp => Direction::Up,
                    _ => Direction::Down,
                };
                if let Some(tree) = &mut self.tree { tree.resize_direction(&self.focused, direction); }
            }
            CommandId::TogglePaneZoom => self.zoomed = !self.zoomed,
            command @ (CommandId::ToggleSidebar | CommandId::CloseWorkspace | CommandId::NewWorkspace | CommandId::NewWorkspaceGroup
                | CommandId::LastWorkspace | CommandId::NextWorkspace | CommandId::PreviousWorkspace | CommandId::UngroupWorkspaces) => {
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
                    .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, window, cx| { this.focused = id; this.samples[&id].update(cx, |sample, cx| sample.focus(window, cx)); }))
                    .child(self.samples[&id].clone())
                    .into_any_element()
        })
    }

}
impl Render for InputProbe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let height = f32::from(window.viewport_size().height);
        let panes = if let Some(tree) = &self.tree {
            if self.zoomed { self.panes(&PaneTree::Pane(self.focused), vec![], cx) } else { self.panes(tree, vec![], cx) }
        } else {
            div().size_full().flex().items_center().justify_center().track_focus(&self.empty_focus).key_context("Terminal")
                .child(div().id("restart-terminal").px_4().py_2().rounded_md().bg(rgb(0x303030)).cursor_pointer()
                    .child("New Terminal").on_click(cx.listener(|this,_,window,cx| this.start_terminal(window,cx))))
                .into_any_element()
        };
        div().font_family(FONT_FAMILY).size_full().bg(rgb(0x151517)).text_color(rgb(0xe2e3e7)).relative().flex().flex_col()
            .on_action(cx.listener(|this, _: &NewTab, window, cx| {
                if this.tree.is_none() { this.start_terminal(window,cx); }
            }))
            .on_action(cx.listener(|this, _: &Split, window, cx| this.execute(CommandId::Split, window, cx)))
            .on_action(cx.listener(|this, _: &SplitBelow, window, cx| this.execute(CommandId::SplitBelow, window, cx)))
            .on_action(cx.listener(|this, _: &TogglePaneZoom, window, cx| this.execute(CommandId::TogglePaneZoom, window, cx)))
            .on_action(cx.listener(|this, _: &FocusLeft, window, cx| this.focus_direction(rmux_core::layout::Direction::Left, window, cx)))
            .on_action(cx.listener(|this, _: &FocusRight, window, cx| this.focus_direction(rmux_core::layout::Direction::Right, window, cx)))
            .on_action(cx.listener(|this, _: &FocusUp, window, cx| this.focus_direction(rmux_core::layout::Direction::Up, window, cx)))
            .on_action(cx.listener(|this, _: &FocusDown, window, cx| this.focus_direction(rmux_core::layout::Direction::Down, window, cx)))
            .on_action(cx.listener(|this, _: &NextPane, window, cx| this.cycle_pane(1, window, cx)))
            .on_action(cx.listener(|this, _: &PreviousPane, window, cx| this.cycle_pane(-1, window, cx)))
            .on_action(cx.listener(|this, _: &CommandPalette, window, cx| this.open_palette(window, cx)))
            .child(workspace_presentation::toolbar()
                .child(workspace_presentation::sidebar_toggle())
                .child(workspace_presentation::title("workspace-title", self.name.clone()))
                .child(div().text_xs().text_color(rgb(0x9b9ba3)).child("Sample sessions"))
                .child(div().id("open-commands").px_2().py_1().rounded_sm().text_size(px(13.)).cursor_pointer().hover(|style| style.bg(rgb(0x353535))).child("Commands")
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.open_palette(window, cx);
                    }))))
            .child(div().flex_1().min_h_0().relative().child(split_interaction::drag_capture(cx)).child(panes))
            .when(self.palette_open, |root| root.child(div().absolute().inset_0().bg(rgba(0x00000066))
                .flex().items_start().justify_center().pt(px(palette_view::top_inset(height)))
                .child(palette_view::panel(height)
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
                .child(palette_view::heading("Search commands and workspaces"))
                .child(palette_view::search(self.input.clone()))
                .child({
                    let entity = cx.entity();
                    list(self.scroll.clone(), move |row, window, cx| {
                        entity.update(cx, |this, cx| {
                            let index = this.search.matches[row];
                            palette_view::result(index, 36., row == this.selected)
                                .child(div().flex_1().min_w_0().truncate().child(this.result_title(index).to_owned()))
                                .when_some(this.result_hint(index, window, cx), |row, hint| row.child(
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
        cx.text_system().add_fonts(vec![
            std::borrow::Cow::Borrowed(include_bytes!("../assets/Inter-Regular.ttf").as_slice()),
            std::borrow::Cow::Borrowed(include_bytes!("../assets/FiraMono-Medium.ttf").as_slice()),
        ]).expect("bundled UI and terminal fonts");
        let mac = terminal_engine::is_mac();
        cx.bind_keys(workspace_shortcuts::bindings(mac));
        cx.bind_keys(pane_shortcuts::directions(mac));
        cx.bind_keys(pane_shortcuts::splits(mac));
        cx.bind_keys(pane_shortcuts::tab_reordering());
        cx.bind_keys([
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
        (&mut self.tree, &mut self.split_drag)
    }
}

#[path = "../../../crates/rmux-desktop/src/shortcut_label.rs"]
mod shortcut_label;

#[path = "../../../crates/rmux-desktop/src/pane_presentation.rs"]
mod pane_presentation;
#[path = "../../../crates/rmux-desktop/src/icons.rs"]
mod icons;
