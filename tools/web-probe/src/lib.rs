//! Compile the actual desktop text input against a candidate web backend.
//! This is a compatibility probe, not a replacement demo or second UI.
use gpui::{prelude::*, *};
use std::ops::Range;
const FONT_FAMILY: &str = "Inter";
#[path = "../../../crates/rmux-desktop/src/control_hint.rs"]
mod control_hint;
mod pane_view { pub(crate) use crate::control_hint::ControlHint; }

#[path = "../../../crates/rmux-desktop/src/composition.rs"]
mod composition;
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
const COMMANDS: &[CommandId] = &[CommandId::Split, CommandId::SplitBelow, CommandId::TogglePaneZoom];


actions!(rmux, [EditorUndo, EditorRedo, Paste, CopySelection, EditorCut, EditorSelectAll, CommandPalette]);
mod clipboard {
    pub fn write(item: gpui::ClipboardItem, cx: &mut gpui::App) {
        cx.write_to_clipboard(item);
    }
}
struct InputProbe {
    input: Entity<text_field::TextField>,
    search: palette_match::RankedSearch,
    selected: usize,
    feedback: Option<String>,
    tree: PaneTree<usize>,
    focused: usize,
    next_pane: usize,
    zoomed: bool,
    palette_open: bool,
    samples: std::collections::HashMap<usize, Entity<terminal_sample::TerminalSample>>,
}
impl InputProbe {
    fn close_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.palette_open = false;
        self.samples[&self.focused].update(cx, |sample, cx| sample.focus(window, cx));
        cx.notify();
    }
    fn open_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.palette_open = true;
        self.input.update(cx, |input, cx| { input.replace_draft("", cx); input.focus.focus(window, cx); });
        cx.notify();
    }

    fn choose(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(&index) = self.search.matches.get(self.selected) else { return; };
        match COMMANDS[index] {
            CommandId::Split | CommandId::SplitBelow => {
                if self.next_pane >= 16 { self.feedback = Some("Probe limit: 16 panes".into()); cx.notify(); return; }
                let axis = if COMMANDS[index] == CommandId::Split { Axis::Columns } else { Axis::Rows };
                if self.tree.split_positioned(&self.focused, self.next_pane, axis, false) {
                    self.samples.insert(self.next_pane, cx.new(terminal_sample::TerminalSample::new));
                    self.focused = self.next_pane; self.next_pane += 1; self.zoomed = false;
                }
            }
            CommandId::TogglePaneZoom => self.zoomed = !self.zoomed,
            _ => return,
        }
        self.close_palette(window, cx);
    }
    fn panes(&self, tree: &PaneTree<usize>, path: String, cx: &mut Context<Self>) -> AnyElement {
        match tree {
            PaneTree::Pane(id) => {
                let id = *id;
                split_presentation::leaf(("pane", id)).flex().flex_col().bg(rgb(0x111216))
                    .border_t_2().border_color(rgb(if self.focused == id { 0x9790d7 } else { 0x383838 }))
                    .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, window, cx| { this.focused = id; this.samples[&id].update(cx, |sample, cx| sample.focus(window, cx)); cx.notify(); }))
                    .child(div().p_3().flex_shrink_0().text_sm().child(format!("Pane {} · live Ghostty · sample session", id + 1)))
                    .child(div().flex_1().min_h_0().child(self.samples[&id].clone()))
                    .into_any_element()
            }
            PaneTree::Split { axis, children, weights } => {
                let mut elements = Vec::new();
                for (index, child) in children.iter().enumerate() {
                    if index > 0 { elements.push(split_presentation::divider(SharedString::from(format!("divider-{path}-{index}")), *axis, 5.).into_any_element()); }
                    elements.push(split_presentation::weighted(weights[index]).child(self.panes(child, format!("{path}-{index}"), cx)).into_any_element());
                }
                split_presentation::branch(*axis).children(elements).into_any_element()
            }
        }
    }

}
impl Render for InputProbe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let height = f32::from(window.viewport_size().height);
        let panes = if self.zoomed { self.panes(&PaneTree::Pane(self.focused), "root".into(), cx) } else { self.panes(&self.tree, "root".into(), cx) };
        div().font_family(FONT_FAMILY).size_full().bg(rgb(0x151517)).text_color(rgb(0xe2e3e7)).relative().flex().flex_col()
            .on_action(cx.listener(|this, _: &CommandPalette, window, cx| this.open_palette(window, cx)))
            .child(div().h(px(44.)).flex_shrink_0().px_3().flex().items_center().justify_between()
                .child("Shared rmux layout · development probe")
                .child(div().id("open-commands").px_3().py_1().cursor_pointer().child("Commands")
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.open_palette(window, cx);
                    }))))
            .child(div().flex_1().min_h_0().child(panes))
            .when(self.palette_open, |root| root.child(div().absolute().inset_0().bg(rgba(0x00000066))
                .flex().items_start().justify_center().pt(px(palette_view::top_inset(height)))
                .child(palette_view::panel(height)
                .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                    if this.input.read(cx).is_composing() { return; }
                    if let Some(next) = palette_view::navigated_result(this.selected, this.search.matches.len(), &event.keystroke.key) {
                        this.selected = next; this.feedback = None; cx.notify(); cx.stop_propagation();
                    }
                }))
                .child(palette_view::heading("Shared rmux palette · development probe"))
                .child(palette_view::search(self.input.clone()))
                .children(self.search.matches.iter().enumerate().map(|(row, &index)| {
                    palette_view::result(index, 36., row == self.selected)
                        .child(div().flex_1().min_w_0().truncate().child(COMMANDS[index].title()))
                        .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
                            if event.standard_click() { this.selected = row; this.choose(window, cx); }
                        }))
                }))
                .when(self.search.matches.is_empty(), |panel| panel.child(div().p_3().text_sm().child("No matching commands")))
                .child(palette_view::footer().child(self.feedback.clone().unwrap_or_else(|| "↑↓ Navigate     ↵ Run     Esc Close".into()))))))
    }
}
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    #[cfg(target_family = "wasm")]
    gpui_platform::web_init();
    gpui_platform::application().run(|cx| {
        cx.text_system().add_fonts(vec![
            std::borrow::Cow::Borrowed(include_bytes!("../assets/Inter-Regular.ttf").as_slice()),
            std::borrow::Cow::Borrowed(include_bytes!("../assets/FiraMono-Medium.ttf").as_slice()),
        ]).expect("bundled UI and terminal fonts");
        cx.bind_keys([
            KeyBinding::new("cmd-p", CommandPalette, None),
            KeyBinding::new("cmd-a", EditorSelectAll, Some("TextInput")),
            KeyBinding::new("cmd-z", EditorUndo, Some("TextInput")),
            KeyBinding::new("cmd-shift-z", EditorRedo, Some("TextInput")),
            KeyBinding::new("cmd-v", Paste, Some("TextInput")),
            KeyBinding::new("cmd-c", CopySelection, Some("TextInput")),
            KeyBinding::new("cmd-x", EditorCut, Some("TextInput")),
        ]);
        cx.open_window(WindowOptions::default(), |window, cx| cx.new(|cx| {
            let input = cx.new(|cx| text_field::TextField::new("", cx));
            input.update(cx, |input, cx| {
                input.placeholder = Some("Type a command or workspace name…");
                input.focus.focus(window, cx);
            });
            cx.subscribe_in(&input, window, |this: &mut InputProbe, _, event: &text_field::EditEvent, window, cx| {
                match event {
                    text_field::EditEvent::Changed(query) => { this.search.update(query); this.selected = 0; this.feedback = None; },
                    text_field::EditEvent::Submitted { .. } => this.choose(window, cx),
                    text_field::EditEvent::Cancelled => this.close_palette(window, cx),
                }
                cx.notify();
            }).detach();
            let samples = std::collections::HashMap::from([(0, cx.new(terminal_sample::TerminalSample::new))]);
            InputProbe { samples, input, search: palette_match::RankedSearch::new(COMMANDS.iter().map(|title| title.title().to_lowercase()).collect()), selected: 0, feedback: None, tree: PaneTree::Pane(0), focused: 0, next_pane: 1, zoomed: false, palette_open: true }
        })).expect("open probe window");
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
