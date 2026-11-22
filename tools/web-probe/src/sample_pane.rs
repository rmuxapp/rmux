//! Browser session host using native tab presentation and navigation definitions.
use gpui::{prelude::*, *};
use crate::{terminal_sample::TerminalSample, tab_presentation};
pub(crate) struct PaneEmpty;
pub(crate) struct TabDrop { pub source: EntityId, pub terminal: EntityId, pub target: Option<EntityId> }
impl EventEmitter<TabDrop> for SamplePane {}
impl EventEmitter<PaneEmpty> for SamplePane {}
include!("sample_tab_menu.rs");
include!("sample_tab_picker.rs");
pub(crate) struct SamplePane {
    pub scope: Option<EntityId>,
    menu: Option<SampleTabMenu>,
    menu_blur: Option<Subscription>,
    picker: Option<SampleTabPicker>,
    picker_blur: Option<Subscription>,
    rename: Option<Entity<crate::text_field::TextField>>,
    terminals: rmux_core::tabs::Tabs<Entity<TerminalSample>>,
    labels: std::collections::HashMap<EntityId, String>,
    next_number: usize,
    tab_scroll: ScrollHandle,
}
pub(crate) struct TabTransfer {
    terminals: Vec<Entity<TerminalSample>>,
    labels: std::collections::HashMap<EntityId, String>,
    active: usize,
    next_number: usize,
}
impl SamplePane {
    fn rename_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(input) = &self.rename { input.read(cx).focus.clone().focus(window, cx); return; }
        let Some(terminal) = self.terminals.active() else { return; };
        let id = terminal.entity_id();
        let name = terminal.read(cx).custom_title.clone().unwrap_or_default();
        let input = cx.new(|cx| crate::text_field::TextField::new(&name, cx));
        cx.subscribe_in(&input, window, move |this, _, event: &crate::text_field::EditEvent, window, cx| {
            match event {
                crate::text_field::EditEvent::Submitted { text, .. } => {
                        if rmux_core::tabs::custom_title_length(text) > rmux_core::tabs::CUSTOM_TITLE_LIMIT { return; }

                    if let Some(terminal) = this.terminals.iter().find(|terminal| terminal.entity_id() == id) {
                        terminal.update(cx, |terminal, cx| { terminal.custom_title = rmux_core::tabs::custom_title(text); cx.notify(); });
                    }
                }
                crate::text_field::EditEvent::Cancelled => {}
                crate::text_field::EditEvent::Changed(_) => { cx.notify(); return; },
            }
            this.rename = None;
            this.tab_scroll.scroll_to_item(this.terminals.active_index());
            this.focus(window, cx);
            cx.notify();
        }).detach();
        input.read(cx).focus.clone().focus(window, cx);
        self.rename = Some(input);
        cx.notify();
    }
    pub fn take_active_for_split(&mut self, cx: &mut Context<Self>) -> Option<TabTransfer> {
        if self.terminals.len() < 2 { return None; }
        self.rename = None;
        self.picker = None;
        self.picker_blur = None;
        let id = self.terminals.active()?.entity_id();
        self.take_tab(id, cx)
    }
    pub fn from_transfer(transfer: TabTransfer) -> Self {
        Self { scope: None, menu: None, menu_blur: None, picker: None, picker_blur: None, rename: None, terminals: rmux_core::tabs::Tabs::new(transfer.terminals, transfer.active),
            labels: transfer.labels, next_number: transfer.next_number, tab_scroll: ScrollHandle::new() }
    }
    pub fn tab_count(&self) -> usize { self.terminals.len() }
    pub fn contains_transferable_tab(&self, id: EntityId) -> bool {
        self.rename.is_none() && self.terminals.iter().any(|terminal| terminal.entity_id() == id)
    }
    pub fn insertion_index(&self, target: Option<EntityId>) -> Option<usize> {
        if self.rename.is_some() || self.terminals.len() >= 16 { return None; }
        match target { Some(id) => self.terminals.iter().position(|terminal| terminal.entity_id() == id), None => Some(self.terminals.len()) }
    }
    pub fn take_tab(&mut self, id: EntityId, cx: &mut Context<Self>) -> Option<TabTransfer> {
        if self.rename.is_some() { return None; }
        let index = self.terminals.iter().position(|terminal| terminal.entity_id() == id)?;
        let terminal = self.terminals.remove(index);
        // The terminal is changing layout/focus owners. Retain its engine and
        // session, but end transient input gestures and rebuild geometry at the
        // destination instead of carrying source-pane selection/cursor bounds.
        terminal.update(cx, |terminal, cx| terminal.release_render_cache(cx));
        let label = self.labels.remove(&id).expect("tab label follows terminal ownership");
        self.menu = None;
        self.menu_blur = None;
        self.tab_scroll.scroll_to_item(self.terminals.active_index());
        cx.notify();
        Some(TabTransfer { terminals: vec![terminal], labels: [(id, label)].into_iter().collect(), active: 0, next_number: self.next_number })
    }
    pub fn insert_tab(&mut self, mut transfer: TabTransfer, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.labels.extend(transfer.labels);
        self.next_number = self.next_number.max(transfer.next_number);
        self.terminals.insert(index, transfer.terminals.remove(0));
        self.select(index, window, cx);
    }

    pub fn take_all_tabs(&mut self, cx: &mut Context<Self>) -> TabTransfer {
        self.rename = None;
        self.picker = None;
        self.picker_blur = None;
        self.menu = None;
        self.menu_blur = None;
        // Joining panes changes the layout owner just like dragging one tab.
        // Include inactive tabs: their cached geometry belongs to this pane too.
        self.release_render_caches(cx);
        let active = self.terminals.active_index();
        let transfer = TabTransfer { terminals: self.terminals.take_all(),
            labels: std::mem::take(&mut self.labels), active, next_number: self.next_number };
        cx.notify();
        transfer
    }
    pub fn append_tabs(&mut self, transfer: TabTransfer, window: &mut Window, cx: &mut Context<Self>) {
        self.labels.extend(transfer.labels);
        self.next_number = self.next_number.max(transfer.next_number);
        if let Some(selected) = self.terminals.append(transfer.terminals, transfer.active) {
            self.select(selected, window, cx);
        }
    }

    pub fn scroll(&self, scroll: rmux_core::scroll::Scroll, cx: &mut App) {
        if let Some(terminal) = self.terminals.active() {
            terminal.update(cx, |terminal,cx| terminal.scroll(scroll,cx));
        }
    }
    pub fn release_render_caches(&mut self, cx: &mut Context<Self>) {
        for terminal in &self.terminals {
            terminal.update(cx, |terminal,cx| terminal.release_render_cache(cx));
        }
    }
    fn close_tabs(&mut self, scope: rmux_core::tabs::CloseScope, window: &mut Window, cx: &mut Context<Self>) {
        let Some((keep, targets)) = self.terminals.close_targets(scope, Entity::entity_id, |_| false) else { return; };
        let current: Vec<_> = self.terminals.iter().map(Entity::entity_id).collect();
        for index in rmux_core::tabs::close_indices(&current, keep, &targets) { self.close(index, window, cx); }
    }
    fn close(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if index >= self.terminals.len() { return; }
        let replace_focus = self.terminals[index].read(cx).is_focused(window) || self.rename.is_some();
        self.rename = None;
        self.picker = None;
        self.picker_blur = None;
        let removed = self.terminals.remove(index);
        self.labels.remove(&removed.entity_id());
        if self.terminals.is_empty() { cx.emit(PaneEmpty); }
        else {
            self.tab_scroll.scroll_to_item(self.terminals.active_index());
            if replace_focus { self.focus(window, cx); }
        }
        cx.notify();
    }
    pub fn active_font_size(&self, cx: &App) -> f32 {
        self.terminals.active().map_or(14., |terminal| terminal.read(cx).font_size)
    }
    pub fn new(font_size: f32, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let terminal = cx.new(|cx| TerminalSample::new(font_size, window,cx));
        let labels = [(terminal.entity_id(), "Terminal 1".into())].into_iter().collect();
        Self { scope: None, menu: None, menu_blur: None, picker: None, picker_blur: None, rename: None, terminals: rmux_core::tabs::Tabs::new(vec![terminal], 0), labels, next_number: 2, tab_scroll: ScrollHandle::new() }
    }
    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        if let Some(terminal) = self.terminals.active() {
            terminal.update(cx, |terminal, cx| terminal.focus(window, cx));
        }
    }
    fn select(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if index >= self.terminals.len() { return; }
        if self.terminals.active_index() != index {
            self.terminals[self.terminals.active_index()].update(cx, |terminal, cx| terminal.release_render_cache(cx));
        }
        self.rename = None;
        self.picker = None;
        self.picker_blur = None;
        self.terminals.select(index);
        self.tab_scroll.scroll_to_item(index);
        self.focus(window, cx);
        cx.notify();
    }
    fn add(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.terminals.len() >= 16 { return; }
        let font_size = self.active_font_size(cx);
        let terminal = cx.new(|cx| TerminalSample::new(font_size, window,cx));
        self.labels.insert(terminal.entity_id(), format!("Terminal {}",self.next_number));
        self.next_number += 1;
        self.terminals.push(terminal);
        self.select(self.terminals.len()-1, window, cx);
    }
    pub fn reorder(&mut self, delta: isize, cx: &mut Context<Self>) {
        if let Some((_, next)) = self.terminals.move_active(delta) {
            self.tab_scroll.scroll_to_item(next);
            cx.notify();
        }
    }
    fn drop_tab(&mut self, item: &crate::drag_item::DragItem, target: Option<EntityId>, window: &mut Window, cx: &mut Context<Self>) {
        if Some(item.scope) != self.scope || Some(item.item) == target || self.rename.is_some() { return; }
        if item.container != cx.entity_id() {
            cx.emit(TabDrop { source: item.container, terminal: item.item, target });
            return;
        }
        let Some(from) = self.terminals.iter().position(|terminal| terminal.entity_id() == item.item) else { return; };
        let target = match target {
            Some(target) => { let Some(index) = self.terminals.iter().position(|terminal| terminal.entity_id() == target) else { return; }; index }
            None => self.terminals.len(),
        };
        let to = rmux_core::tabs::drop_index(from, target);
        let terminal = self.terminals.remove(from);
        self.terminals.insert(to, terminal);
        self.select(to, window, cx);
    }
    fn cycle(&mut self, direction: isize, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(next) = self.terminals.cycle_target(direction) {
            self.select(next, window, cx);
        }
    }
}
impl crate::tab_actions::TabCommandHost for SamplePane {
    fn select_tab_number(&mut self, number: usize, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(index) = number.checked_sub(1) { self.select(index, window, cx); }
    }
    fn run_tab_command(&mut self, command: rmux_core::commands::CommandId, window: &mut Window, cx: &mut Context<Self>) {
        use rmux_core::commands::CommandId;
        match command {
            CommandId::ShowTabMenu => {
                let bounds = self.tab_scroll.bounds();
                self.show_menu(self.terminals.active_index(), point(bounds.left(), bounds.bottom()), window, cx);
            }
            CommandId::NewTab => self.add(window, cx),
            CommandId::RenameTab => self.rename_tab(window, cx),
            CommandId::CloseOtherTabs => self.close_tabs(rmux_core::tabs::CloseScope::Others, window, cx),
            CommandId::CloseTabsToRight => self.close_tabs(rmux_core::tabs::CloseScope::Right, window, cx),
            CommandId::NextTab => self.cycle(1, window, cx),
            CommandId::PreviousTab => self.cycle(-1, window, cx),
            CommandId::MoveTabLeft => self.reorder(-1, cx),
            CommandId::MoveTabRight => self.reorder(1, cx),
            CommandId::ClosePane => self.close(self.terminals.active_index(), window, cx),
            _ => {}
        }
    }
}
impl Render for SamplePane {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focused = self.terminals.active().is_some_and(|terminal| terminal.read(cx).is_focused(window));
        let control_focus = self.terminals.active().map(|terminal| terminal.read(cx).focus_handle().clone());
        let mac = crate::terminal_engine::is_mac();
        let container = cx.entity_id();
        let scope = self.scope.unwrap_or(container);
        let accepts_incoming = self.terminals.len() < 16 && self.rename.is_none();
        crate::pane_presentation::frame(false, false)
            .map(|root| crate::tab_actions::bind(root, cx))
            .on_action(cx.listener(|this, _: &crate::ShowTabPicker, window, cx| this.show_picker(window, cx)))
            .child(tab_presentation::strip()
                .child(div().id("sample-tabs").flex().flex_1().h_full().min_w_0().overflow_x_scroll().track_scroll(&self.tab_scroll)
                    .can_drop(move |item, _, _| item.downcast_ref::<crate::drag_item::DragItem>().is_some_and(|item| item.scope == scope && (item.container == container || accepts_incoming)))
                    .on_drop(cx.listener(|this, item, window, cx| this.drop_tab(item, None, window, cx)))
                    .children(self.terminals.iter().enumerate().map(|(index, terminal)| {
                        let target = terminal.entity_id();
                        let container = cx.entity_id();
                        let title: SharedString = terminal.read(cx).custom_title.clone()
                            .unwrap_or_else(|| self.labels[&target].clone()).into();
                        tab_presentation::tab(("sample-tab",terminal.entity_id()),index==self.terminals.active_index(),focused)
                            .when(self.rename.is_none(), |tab| tab
                                .on_drag(crate::drag_item::DragItem { scope, container, item: target, label: title.clone() }, |item, _, _, cx| cx.new(|_| item.clone()))
                                .can_drop(move |item, _, _| item.downcast_ref::<crate::drag_item::DragItem>().is_some_and(|item| item.scope == scope && (item.container == container || accepts_incoming) && item.item != target))
                                .drag_over(|style, _: &crate::drag_item::DragItem, _, _| style.bg(rgb(0x435a7b)))
                                .on_drop(cx.listener(move |this, item, window, cx| this.drop_tab(item, Some(target), window, cx))))
                            .child(tab_presentation::title(("tab-title", target), title.clone(), title))
                            .child(tab_presentation::close_button(("close-sample-tab", target))
                                .on_click(cx.listener(move |this,event: &ClickEvent,window,cx| {
                                    cx.stop_propagation();
                                    if event.standard_click() {
                                        if let Some(index) = this.terminals.iter().position(|terminal| terminal.entity_id() == target) {
                                            this.close(index,window,cx);
                                        }
                                    }
                                })))
                            .on_mouse_down(MouseButton::Middle, cx.listener(move |this,_,window,cx| {
                                cx.stop_propagation();
                                if let Some(index) = this.terminals.iter().position(|terminal| terminal.entity_id() == target) {
                                    this.close(index,window,cx);
                                }
                            }))
                            .on_mouse_down(MouseButton::Right, cx.listener(move |this,event: &MouseDownEvent,window,cx| {
                                cx.stop_propagation();
                                if let Some(index) = this.terminals.iter().position(|terminal| terminal.entity_id() == target) {
                                    this.show_menu(index, event.position, window, cx);
                                }
                            }))
                            .on_click(cx.listener(move |this,event: &ClickEvent,window,cx| {
                                if !event.standard_click() { return; }
                                let Some(index) = this.terminals.iter().position(|terminal| terminal.entity_id() == target) else { return; };
                                this.select(index,window,cx);
                                if matches!(event, ClickEvent::Mouse(mouse) if mouse.down.click_count == 2) { this.rename_tab(window,cx); }
                            }))})))
                .when(self.terminals.len() > 1, |strip| strip.child(crate::pane_presentation::action_control("tab-picker", "Choose Terminal Tab", Box::new(crate::ShowTabPicker), control_focus.clone(), mac)
                    .child(crate::icons::icon("chevron-down"))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_click(cx.listener(|this, event: &ClickEvent, window, cx| { if event.standard_click() { this.show_picker(window, cx); } }))))
                .child(crate::pane_presentation::action_control("add-tab", "New Terminal Tab", Box::new(crate::NewTab), control_focus.clone(), mac).child(crate::icons::icon("plus"))
                    .on_click(cx.listener(|this,event: &ClickEvent,window,cx| { if event.standard_click() { this.add(window,cx); } })))
                .children([
                    ("split-right", "Split Right", Box::new(crate::Split) as Box<dyn Action>),
                    ("split-down", "Split Below", Box::new(crate::SplitBelow) as Box<dyn Action>),
                    ("expand", "Toggle Pane Zoom", Box::new(crate::TogglePaneZoom) as Box<dyn Action>),
                ].into_iter().map(|(icon,label,action)| {
                    crate::pane_presentation::action_control(icon, label, action.boxed_clone(), control_focus.clone(), mac).child(crate::icons::icon(icon))
                        .on_click(cx.listener(move |this,event: &ClickEvent,window,cx| {
                            if !event.standard_click() { return; }
                            this.focus(window,cx);
                            window.dispatch_action(action.boxed_clone(),cx);
                        }))
                })))
            .child(div().flex_1().min_h_0().children(self.terminals.active().cloned()))
            .when_some(self.picker.clone(), |root, picker| root.child(self.picker_element(picker, cx)))
            .when_some(self.menu.clone(), |root, menu| root.child(self.menu_element(menu, window, cx)))
            .when_some(self.rename.as_ref(), |root, input| root.child(tab_presentation::rename_editor(input.clone(), cx)))
    }
}
