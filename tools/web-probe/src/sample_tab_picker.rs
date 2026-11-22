// Sample-host tab chooser using the native search and palette presentation.
#[derive(Clone)]
struct SampleTabPicker {
    input: Entity<crate::text_field::TextField>,
    selected: Option<EntityId>,
    scroll: ScrollHandle,
    origin: Option<FocusHandle>,
}
impl SamplePane {
    fn tab_name(&self, index: usize, cx: &App) -> String {
        let terminal = &self.terminals[index];
        terminal.read(cx).custom_title.clone().unwrap_or_else(|| self.labels[&terminal.entity_id()].clone())
    }
    fn picker_matches(&self, cx: &App) -> Vec<usize> {
        let query = self.picker.as_ref().map(|picker| picker.input.read(cx).value().to_lowercase()).unwrap_or_default();
        let words: Vec<_> = query.split_whitespace().collect();
        if words.is_empty() { return (0..self.terminals.len()).collect(); }
        let mut matches: Vec<_> = (0..self.terminals.len()).filter_map(|index|
            rmux_core::palette_match::tab_score(index + 1, &self.tab_name(index, cx).to_lowercase(), "", &words)
                .map(|score| (score, index))).collect();
        matches.sort_unstable();
        matches.into_iter().map(|(_, index)| index).collect()
    }
    fn dismiss_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let origin = self.picker.take().and_then(|picker| picker.origin);
        self.picker_blur = None;
        if let Some(origin) = origin { crate::text_layout::focus(&origin, window, cx); }
        else { self.focus(window, cx); }
        cx.notify();
    }
    fn show_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.rename.is_some() || self.terminals.is_empty() { return; }
        if let Some(picker) = &self.picker { let focus = picker.input.read(cx).focus.clone(); crate::text_layout::focus(&focus, window, cx); return; }
        self.menu = None;
        self.menu_blur = None;
        let input = cx.new(|cx| {
            let mut input = crate::text_field::TextField::new("", cx);
            input.placeholder = Some("Search tabs by name or number…");
            input
        });
        let focus = input.read(cx).focus.clone();
        let token = focus.clone();
        self.picker_blur = Some(cx.on_blur(&focus, window, move |this, _, cx| {
            if this.picker.as_ref().is_some_and(|picker| picker.input.read(cx).focus == token) {
                this.picker = None; this.picker_blur = None; cx.notify();
            }
        }));
        cx.subscribe_in(&input, window, |this, _, event: &crate::text_field::EditEvent, window, cx| {
            match event {
                crate::text_field::EditEvent::Changed(_) => {
                    let first = this.picker_matches(cx).first().map(|index| this.terminals[*index].entity_id());
                    if let Some(picker) = &mut this.picker { picker.selected = first; picker.scroll.scroll_to_item(0); }
                }
                crate::text_field::EditEvent::Cancelled => this.dismiss_picker(window, cx),
                crate::text_field::EditEvent::Submitted { .. } => {
                    let selected = this.picker.as_ref().and_then(|picker| picker.selected);
                    if let Some(index) = this.picker_matches(cx).into_iter().find(|index| Some(this.terminals[*index].entity_id()) == selected) {
                        this.select(index, window, cx);
                    }
                }
            }
            cx.notify();
        }).detach();
        let scroll = ScrollHandle::new();
        scroll.scroll_to_item(self.terminals.active_index());
        self.picker = Some(SampleTabPicker { input, origin: window.focused(cx), selected: self.terminals.active().map(Entity::entity_id), scroll });
        crate::text_layout::focus(&focus, window, cx);
        cx.notify();
    }
    fn picker_element(&self, picker: SampleTabPicker, cx: &mut Context<Self>) -> AnyElement {
        let matches = self.picker_matches(cx);
        deferred(div().absolute().top(px(30.)).bottom_0().left_0().right_0().flex().flex_col().min_h_0()
            .child(div().id("sample-tab-picker").occlude().w(px(520.)).max_w_full().min_w_0().min_h_0().overflow_hidden().flex().flex_col()
                .bg(rgb(0x292929)).border_1().border_color(rgb(0x465063)).rounded_md().shadow_md()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
                .on_mouse_down(MouseButton::Middle, |_, _, cx| cx.stop_propagation())
                .on_mouse_move(|_, _, cx| cx.stop_propagation())
                .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
                .on_mouse_down_out(cx.listener(|this, _, window, cx| this.dismiss_picker(window, cx)))
                .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                    let Some(key) = crate::palette_view::navigation_key(&event.keystroke) else { return; };
                    let Some(picker) = &this.picker else { return; };
                    if picker.input.read(cx).is_composing() { return; }
                    let matches = this.picker_matches(cx);
                    let selected = matches.iter().position(|index| Some(this.terminals[*index].entity_id()) == picker.selected).unwrap_or(0);
                    if let Some(next) = crate::palette_view::navigated_result(selected, matches.len(), key) {
                        let id = this.terminals[matches[next]].entity_id();
                        if let Some(picker) = &mut this.picker { picker.selected = Some(id); picker.scroll.scroll_to_item(next); }
                        cx.notify();
                    }
                    cx.stop_propagation();
                }))
                .child(crate::palette_view::header("Choose Terminal Tab", crate::palette_view::close_button()
                    .on_click(cx.listener(|this, event: &ClickEvent, window, cx| { if event.standard_click() { this.dismiss_picker(window, cx); } }))))
                .child(crate::palette_view::search(picker.input, cx))
                .child(div().id("sample-tab-picker-results").min_h_0().max_h(px(320.)).overflow_y_scroll().track_scroll(&picker.scroll)
                    .when(matches.is_empty(), |list| list.child(crate::palette_view::empty_state("No matching tabs")))
                    .children(matches.into_iter().map(|index| {
                        let id = self.terminals[index].entity_id();
                        crate::palette_view::result(index, 40., picker.selected == Some(id))
                            .child(div().text_xs().text_color(rgb(0xa0a0a0)).child((index + 1).to_string()))
                            .child(div().min_w_0().flex_1().truncate().child(self.tab_name(index, cx)))
                            .when(index == self.terminals.active_index(), |row| row.child(crate::palette_view::shortcut("Active")))
                            .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
                                if !event.standard_click() { return; }
                                if let Some(index) = this.terminals.iter().position(|terminal| terminal.entity_id() == id) {
                                    this.select(index, window, cx);
                                }
                            }))
                    })))
                .child(crate::palette_view::footer().child(crate::palette_view::navigation_hints()))
            )).priority(2).into_any_element()
    }
}
