use super::*;
use std::sync::atomic::{AtomicBool, Ordering};

fn task_badge(record: &rmux_terminal::program_status::Record) -> (&'static str, u32) {
    use rmux_terminal::program_status::{Kind, State};
    match record.state {
        State::Idle => ("Idle", 0xa0a0a0),
        State::Working => ("Working", 0x86b8f5),
        State::Done => ("Done", 0x8dd6a1),
        State::Error => ("Error", 0xf18a8a),
        State::Blocked => (match record.kind { Kind::Permission => "Approval", Kind::Question => "Question", Kind::Auth => "Login", Kind::None => "Blocked" }, 0xf0b85b),
    }
}

pub(super) fn prepare_terminal_launch(
    canceled: &AtomicBool,
    launch: impl FnOnce() -> Result<Session>,
    colors: [Option<Color>; 3],
    palette: Option<[Color; 256]>,
) -> Result<PreparedTerminal> {
    anyhow::ensure!(!canceled.load(Ordering::Relaxed), "terminal launch canceled");
    let session = launch()?;
    anyhow::ensure!(!canceled.load(Ordering::Relaxed), "terminal launch canceled");
    TerminalView::prepare(session, colors, palette)
}

// Metadata changes can alter result order or row details. Invalidate list
// measurements without jumping back to the first row on every title update.
fn refresh_tab_picker_list(scroll: &ListState, count: usize) {
    let mut offset = scroll.logical_scroll_top();
    scroll.splice(0..scroll.item_count(), count);
    if offset.item_ix >= count {
        offset = ListOffset { item_ix: count.saturating_sub(1), offset_in_item: px(0.) };
    }
    scroll.scroll_to(offset);
}

fn reveal_reordered_tab(scroll: &ListState, previous: &mut usize, row: usize) {
    if row != *previous { crate::list_navigation::reveal(scroll, row); }
    *previous = row;
}

use rmux_core::tabs::{CloseScope as TabCloseScope, close_indices as other_tab_close_indices};

use rmux_core::tabs::drop_index as tab_drop_index;

#[cfg(test)]
mod tab_order_tests {
    use super::{tab_drop_index, other_tab_close_indices, TabCloseScope};
    #[test]
    fn picker_metadata_refresh_preserves_scroll_and_clamps_removed_results() {
        use super::{ListState, ListAlignment, ListOffset, px, refresh_tab_picker_list};
        let scroll = ListState::new(256, ListAlignment::Top, px(96.));
        scroll.scroll_to(ListOffset { item_ix: 200, offset_in_item: px(7.) });
        refresh_tab_picker_list(&scroll, 256);
        assert_eq!(scroll.logical_scroll_top().item_ix, 200);
        assert_eq!(scroll.logical_scroll_top().offset_in_item, px(7.));
        refresh_tab_picker_list(&scroll, 3);
        assert_eq!(scroll.item_count(), 3);
        assert_eq!(scroll.logical_scroll_top().item_ix, 2);
        assert_eq!(scroll.logical_scroll_top().offset_in_item, px(0.));
        refresh_tab_picker_list(&scroll, 0);
        assert_eq!(scroll.logical_scroll_top().item_ix, 0);
    }

    #[test]
    fn picker_reordered_selection_is_revealed_without_resetting_manual_scroll() {
        use super::{ListState, ListAlignment, ListOffset, px, refresh_tab_picker_list, reveal_reordered_tab};
        let scroll = ListState::new(256, ListAlignment::Top, px(96.));
        let mut selected = 200;
        scroll.scroll_to(ListOffset { item_ix: 40, offset_in_item: px(7.) });
        refresh_tab_picker_list(&scroll, 256);
        reveal_reordered_tab(&scroll, &mut selected, 200);
        assert_eq!(scroll.logical_scroll_top().item_ix, 40);
        assert_eq!(scroll.logical_scroll_top().offset_in_item, px(7.));
        refresh_tab_picker_list(&scroll, 256);
        reveal_reordered_tab(&scroll, &mut selected, 250);
        assert_eq!(scroll.logical_scroll_top().item_ix, 250);
        assert_eq!(selected, 250);
    }

    #[test]
    fn right_close_scope_preserves_left_and_new_tabs_after_reordering() {
        let original = [10, 20, 30, 40];
        let targets: Vec<_> = original.iter().enumerate()
            .filter(|(index, _)| TabCloseScope::Right.includes(*index, 1, false))
            .map(|(_, id)| *id).collect();
        let mut current = vec![40, 10, 20, 30, 50];
        for index in other_tab_close_indices(&current, 20, &targets) { current.remove(index); }
        assert_eq!(current, vec![10, 20, 50]);
    }
    #[test]
    fn finished_close_scope_preserves_current_and_running_tabs() {
        let tabs = [(10, true), (20, true), (30, false), (40, true)];
        let targets: Vec<_> = tabs.iter().enumerate()
            .filter(|(index, (_, exited))| TabCloseScope::Finished.includes(*index, 1, *exited))
            .map(|(_, (id, _))| *id).collect();
        assert_eq!(targets, vec![10, 40]);
        let mut current = vec![40, 30, 20, 50, 10];
        for index in other_tab_close_indices(&current, 20, &targets) { current.remove(index); }
        assert_eq!(current, vec![30, 20, 50]);
    }
    #[test]
    fn batch_close_preserves_keeper_and_excludes_new_tabs_after_reordering() {
        assert_eq!(other_tab_close_indices(&[3, 4, 2, 1], 2, &[1, 3]), vec![3, 0]);
        assert!(other_tab_close_indices(&[3, 4, 1], 2, &[1, 3]).is_empty());
        assert_eq!(other_tab_close_indices(&[1, 2, 4], 2, &[1, 2, 3]), vec![0]);
    }

    #[test]
    fn drops_insert_before_target_from_either_direction() {
        for (from, target, expected) in [
            (0, 3, vec![1, 2, 0, 3]),
            (0, 4, vec![1, 2, 3, 0]),
            (3, 1, vec![0, 3, 1, 2]),
            (1, 2, vec![0, 1, 2, 3]),
            (2, 2, vec![0, 1, 2, 3]),
        ] {
            let mut tabs = vec![0, 1, 2, 3];
            let moved = tabs.remove(from);
            let selected = tab_drop_index(from, target);
            tabs.insert(selected, moved);
            assert_eq!(tabs, expected);
            assert_eq!(tabs[selected], from);
        }
    }
}

pub(super) use crate::control_hint::ControlHint;

pub(super) fn pane_control(id: impl Into<ElementId>, hint: &'static str) -> Stateful<Div> {
    control_button(id).tooltip(move |_, cx| cx.new(|_| ControlHint(hint.into())).into())
}

pub(super) use crate::pane_presentation::control_button;

/// Resolve on hover so settings reloads cannot leave an obsolete shortcut visible.
pub(super) fn action_control(
    id: &'static str,
    label: &'static str,
    action: impl Action,
    focus: Option<FocusHandle>,
) -> Stateful<Div> {
    crate::pane_presentation::action_control(id, label, Box::new(action), focus, cfg!(target_os = "macos"))
}

#[derive(Clone)]
struct TabMenu {
    target: EntityId,
    position: Point<Pixels>,
    focus: FocusHandle,
    selected: usize,
    scroll: ScrollHandle,
    typeahead: menu_navigation::Typeahead,
}
const TAB_MENU_LABELS: &[&str] = &["New Tab", "Rename Tab", "Copy Working Directory", "Reveal Working Directory", "Move Tab Left", "Move Tab Right", "Close Tab", "Close Other Tabs…", "Close Tabs to the Right…", "Close Finished Tabs…"];

pub(super) struct PaneView {
    // Metadata repaints preserve this offset; selection and structural edits
    // explicitly reveal the active tab. Busy background tabs must not reset it.
    tab_scroll: ScrollHandle,
    tab_picker: Option<(Entity<text_field::TextField>, Option<EntityId>, ListState)>,
    tab_picker_selected_row: usize,
    tab_picker_blur: Option<Subscription>,
    tab_picker_origin: Option<FocusHandle>,
    tab_menu: Option<TabMenu>,
    tab_menu_blur: Option<Subscription>,
    spawning_tab: Option<Task<()>>,
    tab_launch_canceled: Option<Arc<AtomicBool>>,
    workspace: EntityId,
    titles: std::collections::HashMap<EntityId, Subscription>,
    pub terminals: rmux_core::tabs::Tabs<Entity<TerminalView>>,
    next_number: usize,
    labels: Vec<String>,
    benchmark: bool,
    error: Option<String>,
    rename: Option<Entity<text_field::TextField>>,
    close_prompt: Option<EntityId>,
}
impl Drop for PaneView {
    fn drop(&mut self) {
        if let Some(cancel) = &self.tab_launch_canceled { cancel.store(true, Ordering::Relaxed); }
    }
}

pub(super) struct PaneEmpty;
impl EventEmitter<PaneEmpty> for PaneView {}
pub(super) struct PaneMetadataChanged;
impl EventEmitter<PaneMetadataChanged> for PaneView {}
pub(super) struct MoveTab {
    pub source: EntityId,
    pub terminal: EntityId,
    pub target_tab: Option<EntityId>,
}
impl EventEmitter<MoveTab> for PaneView {}
pub(super) struct TabTransfer {
    pub terminal: Entity<TerminalView>,
    label: String,
}

impl PaneView {
    fn dismiss_tab_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let focused = self.tab_menu.as_ref().is_some_and(|menu| menu.focus.is_focused(window));
        self.tab_menu = None;
        self.tab_menu_blur = None;
        if focused { self.focus(window, cx); }
        cx.notify();
    }

    fn show_tab_menu(&mut self, target: EntityId, position: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        if self.close_prompt.is_some() || self.rename.is_some() { return; }
        let Some(index) = self.terminals.iter().position(|terminal| terminal.entity_id() == target) else { return; };
        self.select(index, window, cx);
        let focus = cx.focus_handle();
        let token = focus.clone();
        self.tab_menu_blur = Some(cx.on_blur(&focus, window, move |this, _, cx| {
            if this.tab_menu.as_ref().is_some_and(|menu| menu.focus == token) {
                this.tab_menu = None;
                this.tab_menu_blur = None;
                cx.notify();
            }
        }));
        focus.focus(window);
        let selected = self.tab_menu_enabled(target, cx).iter().position(|enabled| *enabled).unwrap_or(0);
        self.tab_menu = Some(TabMenu { target, position, focus, selected, scroll: ScrollHandle::new(), typeahead: Default::default() });
        cx.notify();
    }

    fn finished_tab_count(&self, keep: EntityId, cx: &App) -> usize {
        self.terminals.iter().filter(|terminal| terminal.entity_id() != keep && terminal.read(cx).process_exit.is_some()).count()
    }
    fn tab_menu_enabled(&self, target: EntityId, cx: &App) -> [bool; 10] {
        let Some(index) = self.terminals.iter().position(|terminal| terminal.entity_id() == target) else { return [false; 10]; };
        let batch_available = self.spawning_tab.is_none();
        let terminal = self.terminals[index].read(cx);
        let local = terminal.session.remote_destination().is_none();
        [batch_available && !cx.global::<Shutdowns>().quitting, true, local || terminal.remote_directory_label.is_some(), local, index > 0, index + 1 < self.terminals.len(), true, batch_available && self.terminals.len() > 1, batch_available && index + 1 < self.terminals.len(), batch_available && self.finished_tab_count(target, cx) > 0]
    }

    fn show_active_tab_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(target) = self.active_terminal_id() else { return; };
        let bounds = self.tab_scroll.bounds();
        self.show_tab_menu(target, point(bounds.left(), bounds.bottom()), window, cx);
    }

    fn activate_tab_menu(&mut self, item: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(menu) = self.tab_menu.clone() else { return; };
        if !self.tab_menu_enabled(menu.target, cx).get(item).copied().unwrap_or(false) { return; }
        self.dismiss_tab_menu(window, cx);
        let Some(index) = self.terminals.iter().position(|terminal| terminal.entity_id() == menu.target) else { return; };
        self.select(index, window, cx);
        match item {
            0 => self.add(window, cx),
            1 => self.rename_tab(window, cx),
            2 => self.terminals[index].update(cx, |terminal, cx| terminal.lookup_working_directory(true, cx)),
            3 => self.terminals[index].update(cx, |terminal, cx| terminal.lookup_working_directory(false, cx)),
            4 => self.reorder(-1, cx),
            5 => self.reorder(1, cx),
            6 => self.close(index, window, cx),
            7 => self.close_tabs(TabCloseScope::Others, window, cx),
            8 => self.close_tabs(TabCloseScope::Right, window, cx),
            9 => self.close_tabs(TabCloseScope::Finished, window, cx),
            _ => {},
        }
    }

    fn tab_menu_element(&self, menu: TabMenu, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let enabled = self.tab_menu_enabled(menu.target, cx);
        let inherits_ssh = self.terminals.iter().find(|terminal| terminal.entity_id() == menu.target)
            .is_some_and(|terminal| terminal.read(cx).session.remote_destination().is_some());
        let focus = self.terminals.iter().find(|terminal| terminal.entity_id() == menu.target).map(|terminal| terminal.read(cx).focus.clone());
        let actions: [Box<dyn Action>; 10] = [Box::new(NewTab), Box::new(RenameTab), Box::new(CopyWorkingDirectory), Box::new(RevealWorkingDirectory), Box::new(MoveTabLeft), Box::new(MoveTabRight), Box::new(ClosePane), Box::new(CloseOtherTabs), Box::new(CloseTabsToRight), Box::new(CloseFinishedTabs)];
        let hints: Vec<_> = actions.iter().map(|action| focus.as_ref().and_then(|focus| shortcuts::hint(action.as_ref(), focus, window))).collect();
        deferred(anchored().position(menu.position).snap_to_window().child(
            tab_presentation::menu_frame(window).track_focus(&menu.focus).key_context("TabMenu")
                .on_mouse_down_out(cx.listener(|this, _, window, cx| this.dismiss_tab_menu(window, cx)))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_mouse_down(MouseButton::Middle, |_, _, cx| cx.stop_propagation())
                .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
                .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
                .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                    let Some(target) = this.tab_menu.as_ref().map(|menu| menu.target) else { return; };
                    let enabled = this.tab_menu_enabled(target, cx);
                    let menu = this.tab_menu.as_mut().expect("open tab menu");
                    let modifiers = event.keystroke.modifiers;
                    let typed = if !modifiers.control && !modifiers.alt && !modifiers.platform && !modifiers.function {
                        menu.typeahead.select(&event.keystroke.key, TAB_MENU_LABELS, &enabled, menu.selected, std::time::Instant::now())
                    } else { None };
                    match crate::menu_navigation::action_key(&event.keystroke).unwrap_or("") {
                        "up" | "down" | "home" | "end" => {
                            if let Some(selected) = crate::menu_navigation::navigate(&event.keystroke.key, menu.selected, &enabled) {
                                menu.selected = selected;
                            }
                        }
                        "enter" | "space" => { let selected = menu.selected; this.activate_tab_menu(selected, window, cx); },
                        "escape" => this.dismiss_tab_menu(window, cx),
                        _ => { if let Some(selected) = typed { menu.selected = selected; } },
                    }
                    if let Some(menu) = &this.tab_menu { menu.scroll.scroll_to_item(menu.selected); }
                    cx.stop_propagation();
                    cx.notify();
                }))
                .child(div().id("tab-menu-scroll")
                    .max_h((window.viewport_size().height - px(26.)).max(px(1.)))
                    .overflow_y_scroll().track_scroll(&menu.scroll)
                    .children(TAB_MENU_LABELS.iter().enumerate().map(|(index, label)| {
                    let label = if index == 0 && inherits_ssh { "New SSH Tab".to_owned() }
                        else if index == 9 { format!("Close Finished Tabs ({})…", self.finished_tab_count(menu.target, cx)) }
                        else { (*label).to_owned() };
                    tab_presentation::menu_row(index, matches!(index, 1 | 4 | 6))
                        .when(enabled[index], |row| row.cursor_pointer()
                            .when(index == menu.selected, |row| row.bg(rgb(crate::chrome_style::SELECTED)))
                            .on_mouse_move(cx.listener(move |this, _, _, cx| {
                                if let Some(menu) = &mut this.tab_menu {
                                    if menu.selected != index {
                                        menu.selected = index;
                                        cx.notify();
                                    }
                                }
                            })))
                        .when(!enabled[index], |row| row.text_color(rgb(0x777777)))
                        .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
                            if event.standard_click() { this.activate_tab_menu(index, window, cx); }
                            cx.stop_propagation();
                        }))
                        .child(div().flex_1().min_w_0().truncate().child(label))
                        .when_some(hints[index].clone(), |row, hint| row.child(
                            div().id(("tab-menu-shortcut", index)).max_w(px(120.)).flex_shrink_0().truncate().text_xs().text_color(rgb(0xa0a0a0))
                                .tooltip({ let hint = hint.clone(); move |_, cx| cx.new(|_| ControlHint(hint.clone().into())).into() })
                                .child(hint)
                        ))
                })))
        )).with_priority(2).into_any_element()
    }

    fn dismiss_tab_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let origin = self.tab_picker_origin.take();
        self.tab_picker = None;
        self.tab_picker_blur = None;
        if let Some(origin) = origin { origin.focus(window); }
        else { self.focus(window, cx); }
        cx.notify();
    }

    fn show_tab_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.terminals.is_empty() || self.rename.is_some() || self.close_prompt.is_some() { return; }
        if let Some((input, _, _)) = &self.tab_picker { input.read(cx).focus.focus(window); return; }
        let input = cx.new(|cx| {
            let mut input = text_field::TextField::new("", cx);
            input.placeholder = Some("Search tabs by name or directory…");
            input
        });
        let focus = input.read(cx).focus.clone();
        let token = focus.clone();
        self.tab_picker_blur = Some(cx.on_blur(&focus, window, move |this, _, cx| {
            if this.tab_picker.as_ref().is_some_and(|(input, _, _)| input.read(cx).focus == token) {
                this.tab_picker = None;
                this.tab_picker_origin = None;
                this.tab_picker_blur = None;
                cx.notify();
            }
        }));
        self.tab_picker_origin = window.focused(cx);
        let scroll = ListState::new(self.terminals.len(), ListAlignment::Top, px(96.));
        scroll.scroll_to(ListOffset { item_ix: self.terminals.active_index(), offset_in_item: px(0.) });
        focus.focus(window);
        cx.subscribe_in(&input, window, |this, _, event: &text_field::EditEvent, window, cx| {
            match event {
                text_field::EditEvent::Changed(_) => {
                    let matches = this.matching_tabs(cx);
                    let first = matches.first().map(|index| this.terminals[*index].entity_id());
                    if let Some((_, selected, scroll)) = &mut this.tab_picker { *selected = first; scroll.reset(matches.len()); }
                    this.tab_picker_selected_row = 0;
                }
                text_field::EditEvent::Submitted { .. } => {
                    if let Some((_, selected, _)) = &this.tab_picker {
                        let matches = this.matching_tabs(cx);
                        if let Some(index) = matches.iter().find(|index| Some(this.terminals[**index].entity_id()) == *selected)
                            .copied() { this.select(index, window, cx); }
                    }
                }
                text_field::EditEvent::Cancelled => this.dismiss_tab_picker(window, cx)
            }
            cx.notify();
        }).detach();
        self.tab_picker = Some((input, self.active_terminal_id(), scroll));
        self.tab_picker_selected_row = self.terminals.active_index();
        cx.notify();
    }
    fn render_tab_picker_row(&mut self, index: usize, selected: Option<EntityId>, cx: &mut Context<Self>) -> AnyElement {
        let Some(terminal) = self.terminals.get(index) else { return div().into_any_element(); };
        let id = terminal.entity_id();
        let title: SharedString = self.terminal_search_title(index, cx).into();
        let directory: Option<SharedString> = terminal.read(cx).snapshot_directory.as_ref()
            .map(|path| path.to_string_lossy().into_owned().into());
        let hint: SharedString = directory.as_ref().map(|directory| format!("{title}\n{directory}").into()).unwrap_or_else(|| title.clone());
        let query = self.tab_picker.as_ref().map(|(input, _, _)| input.read(cx).value()).unwrap_or("");
        let highlighted = |text: SharedString| {
            let ranges = palette_match::fuzzy_highlights(&text, query);
            StyledText::new(text).with_highlights(ranges.into_iter().map(|range| (
                range, HighlightStyle { color: Some(rgb(crate::chrome_style::ACCENT).into()),
                    font_weight: Some(FontWeight::BOLD), ..Default::default() }
            )))
        };
        div().id(("choose-tab", id)).w_full().min_w_0().flex().items_center().gap_2().px_3().h(px(52.)).text_sm().line_height(px(20.)).cursor_pointer()
            .tooltip(move |_, cx| cx.new(|_| ControlHint(hint.clone())).into())
            .when(Some(id) == selected, |row| row.bg(rgb(crate::chrome_style::SELECTED)))
            .hover(|row| row.bg(rgb(crate::chrome_style::SELECTED_HOVER)))
            .child(div().w(px(16.)).flex_shrink_0().child(if index == self.terminals.active_index() { "✓" } else { "" }))
            .child(div().w(px(24.)).flex_shrink_0().text_xs().text_color(rgb(0xaaaaaa)).child((index + 1).to_string()))
            .child(div().flex_1().min_w_0().flex().flex_col().gap_1()
                .child(div().truncate().child(highlighted(title)))
                .when_some(directory, |column, directory| column.child(div().text_xs().line_height(px(16.)).text_color(rgb(0xaaaaaa)).truncate().child(highlighted(directory)))))
            .when(terminal.read(cx).attention.any(), |row| row.child(div().w(px(6.)).h(px(6.)).rounded_full().bg(rgb(0xf0b85b))))
            .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
                if !event.standard_click() { return; }
                cx.stop_propagation();
                if let Some(index) = this.terminals.iter().position(|terminal| terminal.entity_id() == id) { this.select(index, window, cx); }
            }))
            .into_any_element()
    }

    fn matching_tabs(&self, cx: &App) -> Vec<usize> {
        let query = self.tab_picker.as_ref().map(|(input, _, _)| input.read(cx).value().to_lowercase()).unwrap_or_default();
        let words: Vec<_> = query.split_whitespace().collect();
        // An empty picker follows tab order; avoid allocating/lowercasing every
        // title and directory merely to assign identical scores and sort them.
        if words.is_empty() { return (0..self.terminals.len()).collect(); }
        let mut matches: Vec<_> = (0..self.terminals.len()).filter_map(|index| {
            let title = self.terminal_search_title(index, cx).to_lowercase();
            let directory = self.terminals[index].read(cx).snapshot_directory.as_ref()
                .map(|path| path.to_string_lossy().to_lowercase()).unwrap_or_default();
            palette_match::tab_score(index + 1, &title, &directory, &words).map(|score| (score, index))
        }).collect();
        matches.sort_unstable_by_key(|(score, index)| (*score, *index));
        matches.into_iter().map(|(_, index)| index).collect()
    }

    fn reconcile_tab_picker(&mut self, cx: &App) {
        let Some((_, selected, _)) = &self.tab_picker else { return; };
        let selected = *selected;
        let matches = self.matching_tabs(cx);
        if let Some((_, _, scroll)) = &self.tab_picker {
            refresh_tab_picker_list(scroll, matches.len());
        }
        if let Some(row) = matches.iter().position(|index| Some(self.terminals[*index].entity_id()) == selected) {
            if let Some((_, _, scroll)) = &self.tab_picker { reveal_reordered_tab(scroll, &mut self.tab_picker_selected_row, row); }
            return;
        }
        // A removed/renamed result should leave the user near the same place,
        // rather than jumping to the beginning of a potentially long list.
        let row = self.tab_picker_selected_row.min(matches.len().saturating_sub(1));
        let replacement = matches.get(row).map(|index| self.terminals[*index].entity_id());
        if selected == replacement { return; }
        if let Some((_, selected, scroll)) = &mut self.tab_picker {
            *selected = replacement;
            if !matches.is_empty() { crate::list_navigation::reveal(scroll, row); }
        }
        self.tab_picker_selected_row = row;
    }

    pub(super) fn set_workspace_owner(&mut self, workspace: EntityId, cx: &mut Context<Self>) {
        if self.workspace == workspace { return; }
        self.workspace = workspace;
        for terminal in self.terminals.iter() {
            events::publish("surface.location_invalidated", terminal.entity_id(), None, cx);
        }
        cx.notify();
    }
    pub(super) fn automation_busy(&self, cx: &App) -> bool {
        self.rename.is_some() || self.close_prompt.is_some() || self.tab_picker.is_some() || self.tab_menu.is_some() || self.spawning_tab.is_some()
            || self.terminals.iter().any(|terminal| terminal.read(cx).terminal_menu.is_some() || terminal.read(cx).copy_mode.is_some() || terminal.read(cx).image_paste_pending())
    }
    pub(super) fn automation_surfaces(&self, window_id: &str, workspace_id: &str, pane_id: &str, history: bool, cx: &App) -> Vec<serde_json::Value> {
        self.terminals.iter().enumerate().map(|(index, entity)| {
            let terminal = entity.read(cx);
            serde_json::json!({
                "id":entity.entity_id().to_string(), "window_id":window_id,
                "workspace_id":workspace_id, "pane_id":pane_id,
                "index":index, "type":"terminal", "title":self.tab_title(index, cx),
                "selected":index == self.terminals.active_index(), "attention":terminal.attention.any(),
                "input_focused":terminal.input_focused,
                "bell_revision":terminal.bell_revision.to_string(),
                "directory_label":terminal.directory_label,
                "remote_directory":terminal.remote_directory_label,
                "git_branch":terminal.git_branch,
                "program_status":terminal.program_status.as_ref().map(|summary| {
                    use rmux_terminal::program_status::{State, Kind};
                    let record = &summary.record;
                    serde_json::json!({"id":record.id, "count":summary.count,
                        "state":match record.state { State::Idle => "idle", State::Working => "working", State::Done => "done", State::Blocked => "blocked", State::Error => "error" },
                        "kind":match record.kind { Kind::None => "none", Kind::Permission => "permission", Kind::Question => "question", Kind::Auth => "auth" },
                        "progress":record.progress, "app":record.app, "title":record.title, "message":record.message})
                }),
                "progress":terminal.progress.map(|progress| {
                    use rmux_terminal::ProgressState;
                    serde_json::json!({"state":match progress.state {
                        ProgressState::Running => "running", ProgressState::Error => "error",
                        ProgressState::Indeterminate => "indeterminate", ProgressState::Paused => "paused",
                    }, "percent":progress.percent})
                }),
                "notification":terminal.notifications.latest()
                    .map(|message| serde_json::json!({"revision":message.revision,"title":message.title,"body":message.body})),
                "notifications":terminal.notifications.iter().rev().take(if history { usize::MAX } else { 0 }).map(|record| serde_json::json!({
                    "revision":record.message.revision,"title":record.message.title,"body":record.message.body,"unread":record.unread
                })).collect::<Vec<_>>(),
                "process_id":terminal.session.process_id(),
                "ssh_destination":terminal.session.remote_destination(),
                "saved_ssh_destination":terminal.saved_ssh_destination,
                "process_exited":terminal.session.process_exited(),
            })
        }).collect()
    }
    pub fn request_process_close(&self, cx: &App) {
        if let Some(cancel) = &self.tab_launch_canceled { cancel.store(true, Ordering::Relaxed); }
        for terminal in self.terminals.iter() { terminal.read(cx).request_process_close(); }
    }
    pub fn reset_window_editors(&mut self) {
        if let Some(cancel) = self.tab_launch_canceled.take() { cancel.store(true, Ordering::Relaxed); }
        self.spawning_tab = None;
        self.close_prompt = None;
        self.rename = None;
        self.tab_picker = None;
        self.tab_picker_origin = None;
        self.tab_picker_blur = None;
        self.tab_menu = None;
        self.tab_menu_blur = None;
    }
    pub fn tab_title(&self, index: usize, cx: &App) -> String {
        let view = self.terminals[index].read(cx);
        let title = view.custom_title.clone().unwrap_or_else(|| {
            if view.title.is_empty() {
                self.labels[index].clone()
            } else {
                view.title.clone()
            }
        });
        if view.saved_ssh_destination.is_some() {
            format!("Local · {title}")
        } else if view.session.remote_destination().is_some() && !title.starts_with("SSH · ") {
            format!("SSH · {title}")
        } else { title }
    }
    pub fn terminal_search_title(&self, index: usize, cx: &App) -> String {
        let title = self.tab_title(index, cx);
        let terminal = self.terminals[index].read(cx);
        let title = terminal.saved_ssh_destination.as_ref()
            .map(|destination| format!("{title} · Saved SSH: {destination} · Not connected"))
            .unwrap_or(title);
        let title = match terminal.session.remote_destination() {
            Some(destination) if title != format!("SSH · {destination}") => format!("{title} · SSH · {destination}"),
            _ => title,
        };
        match &terminal.process_exit {
            Some(status) => format!("{title} · Finished · {status}"),
            None => title,
        }
    }
    fn rename_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(input) = &self.rename {
            input.read(cx).focus.focus(window);
            return;
        }
        let Some(terminal) = self.terminals.active() else {
            return;
        };
        let id = terminal.entity_id();
        let name = terminal.read(cx).custom_title.clone().unwrap_or_default();
        let input = cx.new(|cx| text_field::TextField::new(&name, cx));
        cx.subscribe_in(
            &input,
            window,
            move |this, _, event: &text_field::EditEvent, window, cx| {
                match event {
                    text_field::EditEvent::Submitted { text, .. } => {
                        if let Some(terminal) = this
                            .terminals
                            .iter()
                            .find(|terminal| terminal.entity_id() == id)
                        {
                            let title = rmux_core::tabs::custom_title(text);
                            terminal.update(cx, |terminal, cx| {
                                if terminal.custom_title == title { return; }
                                terminal.custom_title = title;
                                events::publish("surface.title_changed", cx.entity_id(), None, cx);
                                cx.emit(TabMetadataChanged);
                                cx.notify();
                            });
                        }
                    }
                    text_field::EditEvent::Cancelled => {}
                    text_field::EditEvent::Changed(_) => return,
                }
                this.rename = None;
                this.focus(window, cx);
                cx.notify();
            },
        )
        .detach();
        input.read(cx).focus.focus(window);
        self.rename = Some(input);
        cx.notify();
    }
    pub fn active_cell_metrics(&self, cx: &App) -> CellMetrics {
        self.terminals.active()
            .map(|terminal| terminal.read(cx).cells)
            .unwrap_or_else(|| CellMetrics::new(cx.global::<Preferences>().font_size))
    }
    pub fn active_font_family(&self, cx: &App) -> SharedString {
        self.terminals.active()
            .map(|terminal| terminal.read(cx).font_family.clone())
            .unwrap_or_else(|| cx.global::<Preferences>().font_family.clone().into())
    }
    pub fn active_terminal_id(&self) -> Option<EntityId> {
        self.terminals.active().map(Entity::entity_id)
    }

    pub fn pending_palette_operations(&self, cx: &App) -> (bool, bool) {
        let paste = self.terminals.active().is_some_and(|terminal| {
            let terminal = terminal.read(cx);
            terminal.image_paste_pending() || terminal.session.paste_pending()
        });
        (self.spawning_tab.is_some(), paste)
    }

    pub fn release_render_caches(&mut self, cx: &mut Context<Self>) {
        for terminal in &self.terminals {
            if terminal.read(cx).has_render_cache() {
                terminal.update(cx, |terminal, _| terminal.release_render_cache());
            }
        }
    }

    pub fn take_all_tabs(&mut self, cx: &mut Context<Self>) -> (Vec<TabTransfer>, usize) {
        let active = self.terminals.active_index();
        self.titles.clear();
        let tabs = self
            .terminals
            .take_all().into_iter()
            .zip(self.labels.drain(..))
            .map(|(terminal, label)| TabTransfer { terminal, label })
            .collect();
        cx.emit(PaneMetadataChanged);
        cx.notify();
        (tabs, active)
    }
    pub fn append_tabs(
        &mut self,
        tabs: Vec<TabTransfer>,
        active: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if tabs.is_empty() {
            return;
        }
        let mut incoming = Vec::with_capacity(tabs.len());
        for tab in tabs {
            events::publish("surface.location_invalidated", tab.terminal.entity_id(), None, cx);
            let title = cx.subscribe(&tab.terminal, |this, _, _: &TabMetadataChanged, cx| {
                this.reconcile_tab_picker(cx);
                cx.emit(PaneMetadataChanged);
                cx.notify();
            });
            self.titles.insert(tab.terminal.entity_id(), title);
            incoming.push(tab.terminal);
            self.labels.push(tab.label);
        }
        let selected = self.terminals.append(incoming, active).expect("nonempty transfer checked above");
        cx.emit(PaneMetadataChanged);
        // Select only once, without acknowledging other unread tabs in transit.
        self.select(selected, window, cx);
    }

    pub fn active_tab_for_split(&self) -> Option<(EntityId, TabTransfer)> {
        if self.terminals.len() < 2 {
            return None;
        }
        self.active_tab_transfer()
    }
    pub fn active_tab_transfer(&self) -> Option<(EntityId, TabTransfer)> {
        self.tab_transfer(self.active_terminal_id()?)
    }
    pub fn tab_transfer(&self, id: EntityId) -> Option<(EntityId, TabTransfer)> {
        let index = self.terminals.iter().position(|terminal| terminal.entity_id() == id)?;
        let terminal = self.terminals[index].clone();
        Some((
            terminal.entity_id(),
            TabTransfer {
                terminal,
                label: self.labels[index].clone(),
            },
        ))
    }
    pub fn from_transfer(tab: TabTransfer, workspace: EntityId, cx: &mut Context<Self>) -> Self {
        events::publish("surface.location_invalidated", tab.terminal.entity_id(), None, cx);
        Self::restored(vec![tab.terminal], vec![tab.label], 0, workspace, cx)
    }

    pub fn context_label(&self, cx: &App) -> Option<(Option<String>, Option<String>)> {
        let terminal = self.terminals.active()?.read(cx);
        if let Some(destination) = terminal.session.remote_destination() {
            let ended = terminal.process_exit.is_some();
            let status = if ended { "SSH ended" } else { "SSH" };
            let directory = match terminal.remote_directory_label.as_deref() {
                Some(path) if ended => format!("{status} · {destination} · Last: {path}"),
                Some(path) => format!("{status} · {destination} · {path}"),
                None => format!("{status} · {destination}"),
            };
            return Some((None, Some(directory)));
        }
        let directory = terminal.directory_label.clone()
            .or_else(|| terminal.snapshot_directory.as_ref().map(|path| path.to_string_lossy().into_owned()));
        (terminal.git_branch.is_some() || directory.is_some()).then(|| (terminal.git_branch.clone(), directory))
    }
    pub fn needs_attention(&self, cx: &App) -> bool {
        self.terminals
            .iter()
            .any(|terminal| terminal.read(cx).attention.any())
    }
    pub fn restored(
        terminals: Vec<Entity<TerminalView>>,
        labels: Vec<String>,
        active: usize,
        workspace: EntityId,
        cx: &mut Context<Self>,
    ) -> Self {
        let titles = terminals
            .iter()
            .map(|terminal| {
                (
                    terminal.entity_id(),
                    cx.subscribe(
                        terminal,
                        |this, _, _: &TabMetadataChanged, cx| {
                            this.reconcile_tab_picker(cx);
                            cx.emit(PaneMetadataChanged);
                            cx.notify();
                        },
                    ),
                )
            })
            .collect();
        let tab_scroll = ScrollHandle::new();
        tab_scroll.scroll_to_item(active);
        Self {
            tab_scroll,
            tab_picker: None,
            tab_picker_origin: None,
            tab_picker_selected_row: 0,
            tab_picker_blur: None,
            tab_menu: None,
            tab_menu_blur: None,
            spawning_tab: None,
            tab_launch_canceled: None,
            workspace,
            titles,
            next_number: terminals.len() + 1,
            terminals: rmux_core::tabs::Tabs::new(terminals, active),
            labels,
            benchmark: false,
            error: None,
            rename: None,
            close_prompt: None,
        }
    }
    pub fn snapshot(&self, cx: &App) -> snapshot::Node {
        snapshot::Node::Pane {
            active_tab: self.terminals.active_index(),
            tabs: self
                .terminals
                .iter()
                .zip(&self.labels)
                .map(|(terminal, label)| snapshot::TabSnapshot {
                    shell_history_id: terminal.read(cx).session.shell_history_id().map(str::to_owned),
                    label: label.clone(),
                    ssh_destination: terminal.read(cx).session.remote_destination().map(str::to_owned)
                        .or_else(|| terminal.read(cx).saved_ssh_destination.clone()),
                    custom_title: terminal.read(cx).custom_title.clone(),
                    font_size: terminal.read(cx).cells.font_size,
                    font_family: Some(terminal.read(cx).font_family.to_string()),
                    directory: terminal.read(cx).snapshot_directory.clone(),
                })
                .collect(),
        }
    }
    pub fn new(
        terminal: Entity<TerminalView>,
        label: Option<String>,
        benchmark: bool,
        workspace: EntityId,
        cx: &mut Context<Self>,
    ) -> Self {
        let title = cx.subscribe(
            &terminal,
            |this, _, _: &TabMetadataChanged, cx| {
                this.reconcile_tab_picker(cx);
                cx.emit(PaneMetadataChanged);
                cx.notify();
            },
        );
        Self {
            tab_scroll: ScrollHandle::new(),
            tab_picker: None,
            tab_picker_origin: None,
            tab_picker_selected_row: 0,
            tab_picker_blur: None,
            tab_menu: None,
            tab_menu_blur: None,
            spawning_tab: None,
            tab_launch_canceled: None,
            workspace,
            titles: [(terminal.entity_id(), title)].into_iter().collect(),
            terminals: rmux_core::tabs::Tabs::new(vec![terminal], 0),
            next_number: 2,
            labels: vec![label.unwrap_or_else(|| "Terminal 1".into())],
            benchmark,
            error: None,
            rename: None,
            close_prompt: None,
        }
    }

    pub fn focus(&self, window: &mut Window, cx: &App) {
        if let Some(terminal) = self.terminals.active() {
            terminal.read(cx).focus.focus(window);
        }
    }

    pub fn is_focused(&self, window: &Window, cx: &App) -> bool {
        self.terminals.active()
            .is_some_and(|terminal| terminal.read(cx).focus.contains_focused(window, cx))
    }

    /// Cached OSC directory, or explicit launch directory before the first report.
    /// New-workspace creation must not wait for the terminal parser mutex.
    pub fn cached_directory(&self, cx: &App) -> Option<std::path::PathBuf> {
        self.terminals.active()?.read(cx).snapshot_directory.clone()
    }
    pub fn live_ssh_destination(&self, cx: &App) -> Option<String> {
        self.terminals.active()?.read(cx).session.remote_destination().map(str::to_owned)
    }
    pub fn ssh_destination(&self, cx: &App) -> Option<String> {
        let terminal = self.terminals.active()?.read(cx);
        terminal.session.remote_destination().map(str::to_owned).or_else(|| terminal.saved_ssh_destination.clone())
    }

    pub fn select(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if index >= self.terminals.len() {
            return;
        }
        self.terminals.select(index);
        self.rename = None;
        self.tab_picker = None;
        self.tab_picker_origin = None;
        self.tab_picker_blur = None;
        self.tab_menu = None;
        self.tab_menu_blur = None;
        self.tab_scroll.scroll_to_item(index);
        self.focus(window, cx);
        cx.notify();
    }

    fn cycle(&mut self, delta: isize, window: &mut Window, cx: &mut Context<Self>) {
        if self.terminals.is_empty() {
            return;
        }
        self.select(
            self.terminals.cycle_target(delta).unwrap(),
            window,
            cx,
        );
    }

    fn cancel_tab_launch(&mut self, cx: &mut Context<Self>) {
        if let Some(cancel) = &self.tab_launch_canceled {
            cancel.store(true, Ordering::Relaxed);
            cx.notify();
        }
    }

    fn add(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.add_terminal(true, window, cx);
    }

    fn add_terminal(&mut self, inherit_ssh: bool, window: &mut Window, cx: &mut Context<Self>) {
        if cx.global::<Shutdowns>().quitting || self.spawning_tab.is_some() { return; }
        let directory = self.cached_directory(cx).or_else(|| cx.global::<InitialDirectory>().0.clone());
        let destination = inherit_ssh.then(|| self.live_ssh_destination(cx)).flatten();
        let label = destination.as_ref().map(|destination| format!("SSH · {destination}"));
        let cells = self.active_cell_metrics(cx);
        let family = self.active_font_family(cx);
        let previous_default_size = cx.global::<Preferences>().font_size;
        let previous_default_family = cx.global::<Preferences>().font_family.clone();
        let scrollback = cx.global::<Preferences>().scrollback_bytes();
        let colors = cx.global::<Preferences>().terminal_colors();
        let palette = cx.global::<Preferences>().effective_palette();
        let origin = window.focused(cx);
        let executor = cx.background_executor().clone();
        self.error = None;
        let canceled = Arc::new(AtomicBool::new(false));
        self.tab_launch_canceled = Some(canceled.clone());
        let launch_canceled = canceled.clone();
        self.spawning_tab = Some(cx.spawn_in(window, async move |entity, cx| {
            let mut result = executor.spawn(async move {
                prepare_terminal_launch(&launch_canceled, || {
                    ssh::launch_sibling(destination.as_deref(), directory.as_deref(), scrollback)
                }, colors, palette)
            }).await;
            loop {
                if canceled.load(Ordering::Relaxed) { break; }
                // Settings can change during startup. Re-prepare off-thread before
                // installing the view, rather than showing stale colors indefinitely.
                let refresh = cx.update(|_, cx| {
                    let prepared = result.as_ref().ok()?;
                    let settings = cx.global::<Preferences>();
                    let current = (settings.terminal_colors(), settings.effective_palette());
                    ((prepared.colors, prepared.palette) != current).then_some(current)
                }).ok().flatten();
                let Some((colors, palette)) = refresh else { break; };
                let session = result.expect("successful preparation checked above").session;
                result = executor.spawn(async move { TerminalView::prepare(session, colors, palette) }).await;
            }
            let _ = cx.update(|window, cx| entity.update(cx, |this, cx| {
                this.spawning_tab = None;
                this.tab_launch_canceled = None;
                cx.notify();
                if canceled.load(Ordering::Relaxed) || cx.global::<Shutdowns>().quitting { return; }
                match result {
                    Ok(prepared) => {
                        let terminal = cx.new(|cx| {
                            let mut terminal = TerminalView::from_prepared(prepared, window, cx);
                            let (size, family) = preferences::updated_font(cells.font_size, &family,
                                previous_default_size, &previous_default_family, cx.global::<Preferences>());
                            terminal.cells = if size == cells.font_size { cells } else { CellMetrics::new(size) };
                            terminal.font_family = family.into();
                            terminal
                        });
                        let title = cx.subscribe(&terminal, |this, _, _: &TabMetadataChanged, cx| {
                            this.reconcile_tab_picker(cx);
                            cx.emit(PaneMetadataChanged);
                            cx.notify();
                        });
                        this.titles.insert(terminal.entity_id(), title);
                        this.terminals.push(terminal);
                        this.labels.push(label.unwrap_or_else(|| format!("Terminal {}", this.next_number)));
                        this.next_number += 1;
                        if origin.as_ref().is_some_and(|focus| focus.is_focused(window)) {
                            this.select(this.terminals.len() - 1, window, cx);
                        }
                        this.reconcile_tab_picker(cx);
                        cx.emit(PaneMetadataChanged);
                    }
                    Err(error) => this.error = Some(error.to_string()),
                }
                cx.notify();
            }));
        }));
        cx.notify();
    }

    fn reorder(&mut self, delta: isize, cx: &mut Context<Self>) {
        let Some((from, next)) = self.terminals.move_active(delta) else { return; };
        self.labels.swap(from, next);
        self.tab_scroll.scroll_to_item(next);
        self.reconcile_tab_picker(cx);
        cx.emit(PaneMetadataChanged);
        cx.notify();
    }

    fn drop_tab(
        &mut self,
        item: &drag_item::DragItem,
        target: Option<EntityId>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if item.container != cx.entity_id() {
            if item.scope == self.workspace {
                cx.emit(MoveTab {
                    source: item.container,
                    terminal: item.item,
                    target_tab: target,
                });
            }
            return;
        }
        let Some(from) = self
            .terminals
            .iter()
            .position(|view| view.entity_id() == item.item)
        else {
            return;
        };
        let to = match target {
            Some(target) => {
                let Some(index) = self.terminals.iter().position(|view| view.entity_id() == target) else { return; };
                index
            }
            None => self.terminals.len(),
        };
        let to = tab_drop_index(from, to);
        if from != to {
            let terminal = self.terminals.remove(from);
            let label = self.labels.remove(from);
            self.terminals.insert(to, terminal);
            self.labels.insert(to, label);
            cx.emit(PaneMetadataChanged);
        }
        self.select(to, window, cx);
    }

    pub fn take_tab(&mut self, id: EntityId, cx: &mut Context<Self>) -> Option<TabTransfer> {
        let index = self
            .terminals
            .iter()
            .position(|view| view.entity_id() == id)?;
        self.titles.remove(&id);
        let terminal = self.terminals.remove(index);
        let label = self.labels.remove(index);
        self.tab_scroll.scroll_to_item(self.terminals.active_index());
        self.reconcile_tab_picker(cx);
        cx.emit(PaneMetadataChanged);
        cx.notify();
        Some(TabTransfer { terminal, label })
    }

    /// The workspace resolves this index before detaching the source tab.
    pub fn insert_tab_at(
        &mut self,
        tab: TabTransfer,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        events::publish("surface.location_invalidated", tab.terminal.entity_id(), None, cx);
        let title = cx.subscribe(&tab.terminal, |this, _, _: &TabMetadataChanged, cx| {
            this.reconcile_tab_picker(cx);
            cx.emit(PaneMetadataChanged);
            cx.notify();
        });
        self.titles.insert(tab.terminal.entity_id(), title);
        self.terminals.insert(index, tab.terminal);
        self.labels.insert(index, tab.label);
        cx.emit(PaneMetadataChanged);
        self.select(index, window, cx);
    }

    fn close_tabs(&mut self, scope: TabCloseScope, window: &mut Window, cx: &mut Context<Self>) {
        if self.automation_busy(cx) || cx.global::<Shutdowns>().quitting { return; }
        let Some((keep, targets)) = self.terminals.close_targets(scope, Entity::entity_id,
            |terminal| terminal.read(cx).process_exit.is_some()) else { return; };
        if targets.is_empty() { return; }
        let confirms = cx.global::<ClosePolicy>().confirms(true, || {
            self.terminals.iter().filter(|terminal| targets.contains(&terminal.entity_id()))
                .any(|terminal| terminal.read(cx).has_active_work())
        });
        if !confirms {
            self.close_tab_targets(keep, &targets, window, cx);
            return;
        }
        let Some(permit) = cx.global::<PromptGate>().acquire() else { return; };
        self.close_prompt = Some(keep);
        let (title, button, description) = match scope {
            TabCloseScope::Others => ("Close other terminal tabs?", "Close Other Tabs", "other terminal tabs in this pane"),
            TabCloseScope::Right => ("Close tabs to the right?", "Close Tabs to the Right", "terminal tabs to the right of the current tab"),
            TabCloseScope::Finished => ("Close finished terminal tabs?", "Close Finished Tabs", "finished terminal tabs in this pane"),
        };
        let consequence = match scope {
            TabCloseScope::Finished => "Their output history will be discarded.",
            _ => "Their processes will stop and output history will be discarded.",
        };
        let detail = format!("Close {} {description}? {consequence} The current terminal will remain open.", targets.len());
        let answer = window.prompt(PromptLevel::Warning, title, Some(&detail), &["Cancel", button], cx);
        cx.spawn_in(window, async move |entity, cx| {
            let confirmed = matches!(answer.await, Ok(1));
            drop(permit);
            let _ = cx.update(|window, cx| entity.update(cx, |this, cx| {
                if this.close_prompt != Some(keep) { return; }
                this.close_prompt = None;
                if confirmed { this.close_tab_targets(keep, &targets, window, cx); }
            }));
        }).detach();
    }

    fn close_tab_targets(&mut self, keep: EntityId, targets: &[EntityId], window: &mut Window, cx: &mut Context<Self>) {
        // Resolve identities at confirmation time; new or moved-in tabs were not
        // part of the prompt. Abort if the terminal we promised to retain left.
        let current: Vec<_> = self.terminals.iter().map(Entity::entity_id).collect();
        let indices = other_tab_close_indices(&current, keep, targets);
        if indices.is_empty() { return; }
        let selected = self.active_terminal_id();
        let replace_focus = indices.iter().any(|index| {
            self.terminals[*index].read(cx).focus.contains_focused(window, cx)
        });
        for index in indices {
            self.terminals[index].read(cx).request_process_close();
            let removed = self.terminals.remove(index);
            self.titles.remove(&removed.entity_id());
            self.labels.remove(index);
        }
        // The keeper guarantees a nonempty pane. Preserve a newer surviving
        // selection, otherwise return to the terminal retained by the command.
        let selected_index = self.terminals.iter().position(|terminal| Some(terminal.entity_id()) == selected)
            .or_else(|| self.terminals.iter().position(|terminal| terminal.entity_id() == keep))
            .expect("batch close preserves its keeper");
        self.terminals.select(selected_index);
        self.tab_scroll.scroll_to_item(self.terminals.active_index());
        self.reconcile_tab_picker(cx);
        if replace_focus { self.focus(window, cx); }
        cx.emit(PaneMetadataChanged);
        cx.notify();
    }

    fn close(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(terminal) = self.terminals.get(index) else {
            return;
        };
        let view = terminal.read(cx);
        if !cx
            .global::<ClosePolicy>()
            .confirms(true, || view.has_active_work())
        {
            self.close_confirmed(index, window, cx);
            return;
        }
        if self.close_prompt.is_some() {
            return;
        }
        let id = terminal.entity_id();
        let focus = view.focus.clone();
        let Some(permit) = cx.global::<PromptGate>().acquire() else {
            return;
        };
        self.close_prompt = Some(id);
        let answer = window.prompt(PromptLevel::Warning, "Close terminal?",
            Some("Closing this terminal terminates its owned process and discards its output history."),
            &["Cancel", "Close Terminal"], cx);
        cx.spawn_in(window, async move |entity, cx| {
            let confirmed = matches!(answer.await, Ok(1));
            drop(permit);
            let _ = cx.update(|window, cx| {
                entity.update(cx, |pane, cx| {
                    if pane.close_prompt != Some(id) {
                        return;
                    }
                    pane.close_prompt = None;
                    if confirmed {
                        if let Some(index) = pane.terminals.iter().position(|terminal| {
                            terminal.entity_id() == id && terminal.read(cx).focus == focus
                        }) {
                            pane.close_confirmed(index, window, cx);
                        }
                    }
                })
            });
        })
        .detach();
    }

    fn close_confirmed(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if index >= self.terminals.len() {
            return;
        }
        if self.tab_menu.as_ref().is_some_and(|menu| menu.target == self.terminals[index].entity_id()) {
            self.dismiss_tab_menu(window, cx);
        }
        // Closing an inactive tab must not steal focus from another terminal,
        // a find field or an editor elsewhere in the window.
        let replace_focus = self.terminals[index]
            .read(cx)
            .focus
            .contains_focused(window, cx);
        self.terminals[index].read(cx).request_process_close();
        let removed = self.terminals.remove(index);
        self.titles.remove(&removed.entity_id());
        self.labels.remove(index);
        self.reconcile_tab_picker(cx);
        cx.emit(PaneMetadataChanged);
        if self.terminals.is_empty() {
            cx.emit(PaneEmpty);
        } else {
            self.tab_scroll.scroll_to_item(self.terminals.active_index());
            if replace_focus {
                self.focus(window, cx);
            }
        }
        cx.notify();
    }
}

impl crate::tab_actions::TabCommandHost for PaneView {
    fn select_tab_number(&mut self, number: usize, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(index) = number.checked_sub(1) { self.select(index, window, cx); }
    }
    fn run_tab_command(&mut self, command: rmux_core::commands::CommandId, window: &mut Window, cx: &mut Context<Self>) {
        use rmux_core::commands::CommandId;
        match command {
            CommandId::ShowTabMenu => self.show_active_tab_menu(window, cx),
            CommandId::NewTab => self.add(window, cx),
            CommandId::RenameTab => self.rename_tab(window, cx),
            CommandId::CloseOtherTabs => self.close_tabs(TabCloseScope::Others, window, cx),
            CommandId::CloseTabsToRight => self.close_tabs(TabCloseScope::Right, window, cx),
            CommandId::NextTab => self.cycle(1, window, cx),
            CommandId::PreviousTab => self.cycle(-1, window, cx),
            CommandId::MoveTabLeft => self.reorder(-1, cx),
            CommandId::MoveTabRight => self.reorder(1, cx),
            CommandId::ClosePane => self.close(self.terminals.active_index(), window, cx),
            _ => {}
        }
    }
}
impl Render for PaneView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let control_focus = self.terminals.active().map(|terminal| terminal.read(cx).focus.clone());
        // Hidden tabs keep their engines, but need not retain shaped glyph rows.
        // Do not notify: releasing a cache cannot change visible output.
        for (index, terminal) in self.terminals.iter().enumerate() {
            if index != self.terminals.active_index() && terminal.read(cx).has_render_cache() {
                terminal.update(cx, |terminal, _| terminal.release_render_cache());
            }
        }
        let focused = window.is_window_active() && self.is_focused(window, cx);
        let attention = self.needs_attention(cx);
        crate::pane_presentation::frame(self.benchmark, attention)
            .on_action(cx.listener(|this, _: &NewLocalTab, window, cx| this.add_terminal(false, window, cx)))
            .on_action(cx.listener(|this, _: &CloseFinishedTabs, window, cx| this.close_tabs(TabCloseScope::Finished, window, cx)))
            .on_action(cx.listener(|this, _: &CancelNewTab, _, cx| this.cancel_tab_launch(cx)))
            .on_action(cx.listener(|this, _: &ShowTabPicker, window, cx| this.show_tab_picker(window, cx)))
            .map(|root| crate::tab_actions::bind(root, cx))
            .when(!self.benchmark, |root| {
                root.child(
                    crate::tab_presentation::strip()
                        .child(
                            div()
                                .id("tabs")
                                .flex()
                                .flex_1()
                                .h_full()
                                .min_w_0()
                                .overflow_x_scroll()
                                .track_scroll(&self.tab_scroll)
                                .children(self.terminals.iter().enumerate().map(
                                    |(index, terminal)| {
                                        let container = cx.entity_id();
                                        let scope = self.workspace;
                                        let target = terminal.entity_id();
                                        let label: SharedString = self.tab_title(index, cx).into();
                                        let hint_pane = cx.entity().downgrade();
                                        let display_title = label.clone();
                                        crate::tab_presentation::tab(("tab", terminal.entity_id()), index == self.terminals.active_index(), focused)
                                            .on_drag(drag_item::DragItem { scope, container, item: target, label }, |item, _, _, cx| cx.new(|_| item.clone()))
                                            .can_drop(move |item, _, _| item.downcast_ref::<drag_item::DragItem>().is_some_and(|item| item.scope == scope && item.item != target))
                                            .drag_over(move |style, item: &drag_item::DragItem, _, _| {
                                                if item.scope == scope && item.item != target {
                                                    style.bg(rgb(0x303b4a)).border_color(rgb(0x4599ef))
                                                } else { style }
                                            })
                                            .on_drop(cx.listener(move |this, item: &drag_item::DragItem, window, cx| this.drop_tab(item, Some(target), window, cx)))
                                            .on_mouse_down(MouseButton::Right, cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                                                this.show_tab_menu(target, event.position, window, cx);
                                                cx.stop_propagation();
                                            }))
                                            .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
                                                if !event.standard_click() { return; }
                                                let Some(index) = this.terminals.iter().position(|terminal| terminal.entity_id() == target) else { return; };
                                                this.select(index, window, cx);
                                                if matches!(event, ClickEvent::Mouse(mouse) if mouse.down.click_count == 2) {
                                                    this.rename_tab(window, cx);
                                                }
                                            }))
                                            .on_mouse_down(MouseButton::Middle, cx.listener(move |this, _, window, cx| {
                                                cx.stop_propagation();
                                                if let Some(index) = this.terminals.iter().position(|terminal| terminal.entity_id() == target) {
                                                    this.close(index, window, cx);
                                                }
                                            }))
                                            .when(terminal.read(cx).attention.any(), |tab| tab.child(
                                                div().w(px(6.)).h(px(6.)).flex_shrink_0().rounded_full().bg(rgb(0xf0b85b))
                                            ))
                                            .when_some(terminal.read(cx).program_status.as_ref().filter(|_| terminal.read(cx).process_exit.is_none()), |tab, status| {
                                                let record = &status.record;
                                                let (label, tint) = task_badge(record);
                                                let mut text = label.to_owned();
                                                if let Some(percent) = record.progress { text.push_str(&format!(" {percent}%")); }
                                                if status.count > 1 { text.push_str(&format!(" +{}", status.count - 1)); }
                                                // A report can contain a 2 KiB message. Read and format it
                                                // only on hover, without retaining the terminal/session.
                                                let source = terminal.downgrade();
                                                tab.child(div().id(("program-status", terminal.entity_id())).text_xs().flex_shrink_0().text_color(rgb(tint))
                                                    .tooltip(move |_, cx| {
                                                        let detail = source.upgrade().and_then(|terminal| {
                                                            let terminal = terminal.read(cx);
                                                            let record = &terminal.program_status.as_ref()?.record;
                                                            let label = if terminal.process_exit.is_some() { "Last reported status" } else { task_badge(record).0 };
                                                            Some(format!("{label} · {} · {}\n{}", record.app, record.title, record.message))
                                                        }).unwrap_or_else(|| "Task status unavailable".to_owned());
                                                        cx.new(|_| ControlHint(detail.into())).into()
                                                    }).child(text))
                                            })
                                            .when_some(terminal.read(cx).progress.filter(|_| terminal.read(cx).process_exit.is_none()), |tab, progress| {
                                                use rmux_terminal::ProgressState;
                                                let (label, tint) = match progress.state {
                                                    ProgressState::Running => ("", 0x86b8f5),
                                                    ProgressState::Error => ("Error", 0xf18a8a),
                                                    ProgressState::Indeterminate => ("Working", 0x86b8f5),
                                                    ProgressState::Paused => ("Paused", 0xf0b85b),
                                                };
                                                let text = match progress.percent {
                                                    Some(percent) if progress.state != ProgressState::Indeterminate => format!("{label} {percent}%").trim().to_owned(),
                                                    _ if label.is_empty() => "Working".to_owned(),
                                                    _ => label.to_owned(),
                                                };
                                                tab.child(div().text_xs().flex_shrink_0().text_color(rgb(tint)).child(text))
                                            })
                                            .when(terminal.read(cx).process_exit.is_some(), |tab| {
                                                tab.child(div().text_xs().flex_shrink_0().text_color(rgb(0xa0a0a0)).child("Finished"))
                                            })
                                            .child({
                                                let fallback = display_title.clone();
                                                crate::tab_presentation::title_frame(("tab-title", target)).child(display_title)
                                                    .tooltip(move |_, cx| {
                                                        let detail = hint_pane.upgrade().and_then(|pane| {
                                                            let pane = pane.read(cx);
                                                            pane.terminals.iter().position(|terminal| terminal.entity_id() == target)
                                                                .map(|index| pane.terminal_search_title(index, cx))
                                                        }).map(SharedString::from).unwrap_or_else(|| fallback.clone());
                                                        cx.new(|_| ControlHint(detail)).into()
                                                    })
                                            })
                                            .child(
                                                crate::tab_presentation::close_button(("close-tab", terminal.entity_id()))
                                                    .on_click(cx.listener(
                                                        move |this, event: &ClickEvent, window, cx| {
                                                            cx.stop_propagation();
                                                            if !event.standard_click() { return; }
                                                            if let Some(index) = this.terminals.iter().position(|terminal| terminal.entity_id() == target) {
                                                                this.close(index, window, cx);
                                                            }
                                                        },
                                                    )),
                                            )
                                    },
                                ))
                                .child({
                                    let scope = self.workspace;
                                    div().id("tab-drop-end").w(px(20.)).h_full().flex_shrink_0()
                                        .border_l_2().border_color(gpui::transparent_black())
                                        .can_drop(move |item, _, _| item.downcast_ref::<drag_item::DragItem>().is_some_and(|item| item.scope == scope))
                                        .drag_over(move |style, item: &drag_item::DragItem, _, _| {
                                            if item.scope == scope { style.bg(rgb(0x303b4a)).border_color(rgb(0x4599ef)) } else { style }
                                        })
                                        .on_drop(cx.listener(|this, item: &drag_item::DragItem, window, cx| this.drop_tab(item, None, window, cx)))
                                }),
                        )
                        .when(self.terminals.len() > 1, |header| header.child(
                            action_control("tab-picker", "Choose Terminal Tab", ShowTabPicker, control_focus.clone())
                                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                                .on_click(cx.listener(|this, event: &ClickEvent, window, cx| {
                                    if !event.standard_click() { return; }
                                    this.show_tab_picker(window, cx);
                                })).child(icons::icon("chevron-down"))
                        ))
                        .when(self.spawning_tab.is_some(), |header| header.child(
                            div().flex().items_center().gap_1().px_2().text_xs().text_color(rgb(0xaaaaaa))
                                .child(if self.tab_launch_canceled.as_ref().is_some_and(|cancel| cancel.load(Ordering::Relaxed)) { "Canceling…" } else { "Starting…" })
                                .child(action_control("cancel-new-tab", "Cancel New Terminal Tab", CancelNewTab, control_focus.clone())
                                    .on_click(cx.listener(|this, event: &ClickEvent, _, cx| {
                                        if event.standard_click() { this.cancel_tab_launch(cx); }
                                    })).child(icons::icon("close")))
                        ))
                        .child(
                            action_control("new-tab", if self.spawning_tab.is_some() { "Starting terminal…" } else if self.terminals.active().is_some_and(|terminal| terminal.read(cx).session.remote_destination().is_some()) { "New SSH Tab" } else { "New Terminal Tab" }, NewTab, control_focus.clone())
                                .on_click(cx.listener(|this, event: &ClickEvent, window, cx| {
                                    if event.standard_click() { this.add(window, cx); }
                                }))
                                .child(icons::icon("plus")),
                        )
                        .child(
                            action_control("split-right", "Split Right", Split, control_focus.clone())
                                .on_click(cx.listener(|this, event: &ClickEvent, window, cx| {
                                    if !event.standard_click() { return; }
                                    this.focus(window, cx);
                                    window.dispatch_action(Box::new(Split), cx);
                                }))
                                .child(icons::icon("split-right")),
                        )
                        .child(
                            action_control("split-down", "Split Down", SplitBelow, control_focus.clone())
                                .on_click(cx.listener(|this, event: &ClickEvent, window, cx| {
                                    if !event.standard_click() { return; }
                                    this.focus(window, cx);
                                    window.dispatch_action(Box::new(SplitBelow), cx);
                                }))
                                .child(icons::icon("split-down")),
                        )
                        .child(
                            action_control("pane-zoom", "Toggle Pane Zoom", TogglePaneZoom, control_focus.clone())
                                .on_click(cx.listener(|this, event: &ClickEvent, window, cx| {
                                    if !event.standard_click() { return; }
                                    this.focus(window, cx);
                                    window.dispatch_action(Box::new(TogglePaneZoom), cx);
                                }))
                                .child(icons::icon("expand")),
                        ),
                )
            })
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .when_some(self.terminals.active(), |body, terminal| {
                        body.child(terminal.clone())
                    }),
            )
            .when_some(self.rename.as_ref(), |root, input| {
                root.child(crate::tab_presentation::rename_editor(input.clone()))
            })
            .when_some(self.error.clone(), |root, error| {
                root.child(div_error(error, cx.listener(|this, event: &ClickEvent, _, cx| {
                    if event.standard_click() { this.error = None; cx.stop_propagation(); cx.notify(); }
                })))
            })
            .when_some(self.tab_menu.clone(), |root, menu| root.child(self.tab_menu_element(menu, window, cx)))
            .when_some(self.tab_picker.clone(), |root, (input, selected, scroll)| {
                let matches = self.matching_tabs(cx);
                let selected = matches.iter().map(|index| self.terminals[*index].entity_id())
                    .find(|id| Some(*id) == selected)
                    .or_else(|| matches.first().map(|index| self.terminals[*index].entity_id()));
                root.child(deferred(div().absolute().top(px(30.)).bottom_0().left_0().right_0()
                    .flex().flex_col().min_h_0()
                    .child(div().id("tab-picker-container").occlude().w(px(520.)).max_w_full().min_w_0().min_h_0().overflow_hidden().flex().flex_col()
                    .bg(rgb(0x292929)).border_1().border_color(rgb(0x465063)).rounded_md().shadow_md()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_mouse_down(MouseButton::Middle, |_, _, cx| cx.stop_propagation())
                    .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
                    .on_mouse_up(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_mouse_up(MouseButton::Middle, |_, _, cx| cx.stop_propagation())
                    .on_mouse_up(MouseButton::Right, |_, _, cx| cx.stop_propagation())
                    .on_mouse_move(|_, _, cx| cx.stop_propagation())
                    .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
                    .on_mouse_down_out(cx.listener(|this, _, window, cx| { this.dismiss_tab_picker(window, cx); }))
                    .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                        let Some(key) = crate::palette_view::navigation_key(&event.keystroke) else { return; };
                        let Some((input, _, _)) = &this.tab_picker else { return; };
                        if input.read(cx).is_composing() { return; }
                        cx.stop_propagation();
                        let matches = this.matching_tabs(cx);
                        let Some((_, selected, scroll)) = &mut this.tab_picker else { return; };
                        let current = matches.iter().position(|index| Some(this.terminals[*index].entity_id()) == *selected).unwrap_or(0);
                        let destination = if matches!(key, "pageup" | "pagedown") {
                            command_palette::paged_result(current, matches.len(), key == "pageup",
                                f32::from(scroll.viewport_bounds().size.height), |_| 52.)
                        } else {
                            command_palette::navigated_result(current, matches.len(), key)
                        };
                        if let Some(next) = destination {
                            *selected = Some(this.terminals[matches[next]].entity_id());
                            this.tab_picker_selected_row = next;
                            crate::list_navigation::reveal(scroll, next);
                            cx.stop_propagation(); cx.notify();
                        }
                    }))
                    .child(crate::palette_view::header(format!("Terminal tabs · {} of {}", matches.len(), self.terminals.len()),
                        crate::palette_view::close_button().on_click(cx.listener(|this, event: &ClickEvent, window, cx| {
                            cx.stop_propagation();
                            if event.standard_click() { this.dismiss_tab_picker(window, cx); }
                        }))))
                    .child(crate::palette_view::search(input, cx))
                    .child(if matches.is_empty() {
                        div().p_3().text_sm().text_color(rgb(0xaaaaaa)).child("No matching tabs").into_any_element()
                    } else {
                        let entity = cx.entity();
                        let height = px((matches.len() as f32 * 52.).min(240.));
                        list(scroll, move |row, _, cx| {
                            let Some(&index) = matches.get(row) else { return div().into_any_element(); };
                            entity.update(cx, |pane, cx| pane.render_tab_picker_row(index, selected, cx))
                        }).with_sizing_behavior(ListSizingBehavior::Auto)
                            .w_full().min_h_0().h(height).into_any_element()
                    })
                    .child(crate::palette_view::footer().child("↑↓ choose · Enter open · Esc close"))
                    )).with_priority(1))
            })
    }
}

#[cfg(test)]
mod launch_tests {
    use super::{prepare_terminal_launch, AtomicBool, Ordering, Session, Result};

    #[test]
    fn canceled_launch_never_invokes_the_launcher() {
        let canceled = AtomicBool::new(true);
        let result = prepare_terminal_launch(&canceled, || panic!("canceled launch spawned a session"), [None; 3], None);
        assert!(result.is_err());
    }

    #[cfg(unix)]
    #[test]
    fn cancellation_after_spawn_reaps_the_unattached_child() -> Result<()> {
        let canceled = AtomicBool::new(false);
        let mut shutdown = None;
        let result = prepare_terminal_launch(&canceled, || {
            let session = Session::command_in(&["/bin/sh".into(), "-c".into(), "while read line; do :; done".into()], 40, 4, None)?;
            shutdown = Some(session.shutdown_handle());
            canceled.store(true, Ordering::Relaxed);
            Ok(session)
        }, [None; 3], None);
        assert!(result.is_err());
        let shutdown = shutdown.expect("fixture launched a child");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        while !shutdown.is_finished() {
            anyhow::ensure!(std::time::Instant::now() < deadline, "canceled launch retained its child");
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        Ok(())
    }
}
