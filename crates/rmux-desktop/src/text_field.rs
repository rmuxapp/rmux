use super::*;

pub(super) enum EditEvent {
    Changed(String),
    Submitted { text: String, reverse: bool },
    Cancelled,
}
pub(super) struct TextField {
    pub focus: FocusHandle,
    text: composition::Composition,
    bounds: Bounds<Pixels>,
    scroll_x: Pixels,
    reversed: bool,
    pointer_anchor: Option<usize>,
    history: edit_history::EditHistory,
    rejected_edit: bool,
    pub placeholder: Option<&'static str>,
}
impl EventEmitter<EditEvent> for TextField {}
impl TextField {
    /// Replace a draft from an adjacent picker through the normal undo path.
    pub fn replace_draft(&mut self, text: &str, cx: &mut Context<Self>) {
        if self.is_composing() { return; }
        let len = self.text.text.encode_utf16().count();
        self.insert(Some(0..len), text, cx);
    }
    pub fn select_all_text(&mut self, cx: &mut Context<Self>) {
        if self.is_composing() {
            return;
        }
        self.pointer_anchor = None;
        self.text.selected = 0..self.text.text.encode_utf16().count();
        self.reversed = false;
        cx.notify();
    }
    pub fn value(&self) -> &str {
        &self.text.text
    }
    pub fn is_composing(&self) -> bool {
        self.text.marked.is_some()
    }
    pub fn new(text: &str, cx: &mut Context<Self>) -> Self {
        let mut document = composition::Composition::default();
        document.insert(None, text);
        document.selected = 0..text.encode_utf16().count();
        Self {
            focus: cx.focus_handle(),
            text: document,
            bounds: Bounds::default(),
            scroll_x: px(0.),
            reversed: false,
            pointer_anchor: None,
            history: Default::default(),
            rejected_edit: false,
            placeholder: None,
        }
    }
    fn changed(&mut self, cx: &mut Context<Self>) {
        cx.emit(EditEvent::Changed(self.text.text.clone()));
        cx.notify();
    }
    fn insert(&mut self, range: Option<Range<usize>>, text: &str, cx: &mut Context<Self>) {
        self.pointer_anchor = None;
        let range = range.unwrap_or_else(|| {
            self.text
                .marked
                .clone()
                .unwrap_or_else(|| self.text.selected.clone())
        });
        let bytes = self.text.byte_range(range.clone());
        // Newline normalization preserves UTF-8 byte length. Reject before
        // allocating another copy of a potentially very large clipboard value.
        if text.len() > 4096usize.saturating_sub(self.text.text.len() - bytes.len()) {
            self.rejected_edit = true;
            cx.notify();
            return;
        }
        self.rejected_edit = false;
        let text = text.replace(['\r', '\n'], " ");
        if self.text.marked.is_none() && self.text.text[bytes] != text {
            self.history.record(&self.text, self.reversed);
        }
        self.text.insert(Some(range), &text);
        self.reversed = false;
        self.changed(cx);
    }
    fn key(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.pointer_anchor = None;
        if self.text.marked.is_some() {
            return;
        }
        let key = &event.keystroke;
        // WASM's compilation target is not the user's keyboard platform.
        #[cfg(target_arch = "wasm32")]
        let mac = crate::terminal_engine::is_mac();
        #[cfg(not(target_arch = "wasm32"))]
        let mac = cfg!(target_os = "macos");
        let word = if mac {
            key.modifiers.alt
        } else {
            key.modifiers.control
        };
        match key.key.as_str() {
            "escape" => cx.emit(EditEvent::Cancelled),
            "enter" => cx.emit(EditEvent::Submitted {
                text: self.text.text.clone(),
                reverse: key.modifiers.shift,
            }),
            "backspace" | "delete" => {
                let range = if mac && key.modifiers.platform {
                    self.text.edge_deletion_range(key.key == "backspace")
                } else if word {
                    self.text.word_deletion_range(key.key == "backspace")
                } else {
                    self.text.deletion_range(key.key == "backspace")
                };
                self.insert(Some(range), "", cx);
            }
            "left" | "right" | "home" | "end" => {
                let backwards = matches!(key.key.as_str(), "left" | "home");
                let edge = matches!(key.key.as_str(), "home" | "end") || key.modifiers.platform;
                if word && !edge {
                    self.text.move_caret_by(
                        backwards,
                        false,
                        true,
                        key.modifiers.shift,
                        &mut self.reversed,
                    );
                } else {
                    self.text
                        .move_caret(backwards, edge, key.modifiers.shift, &mut self.reversed);
                }
                cx.notify();
            }
            _ => return,
        }
        cx.stop_propagation();
    }
    fn restore_history(&mut self, redo: bool, cx: &mut Context<Self>) {
        if self.is_composing() {
            return;
        }
        self.pointer_anchor = None;
        if self
            .history
            .restore(&mut self.text, &mut self.reversed, redo)
        {
            self.rejected_edit = false;
            self.changed(cx);
        }
    }
    fn pointer_offset(&self, x: Pixels, window: &Window) -> usize {
        let line = self.line(window);
        let x = x - self.bounds.origin.x + self.scroll_x;
        let bounds = self.text.grapheme_bounds(line.closest_index_for_x(x));
        let left = line.x_for_index(bounds.start);
        let right = line.x_for_index(bounds.end);
        self.text.utf16_offset(if x - left < right - x {
            bounds.start
        } else {
            bounds.end
        })
    }
    fn line(&self, window: &Window) -> text_layout::Line {
        text_layout::shape(window, self.text.text.clone().into(), font(FONT_FAMILY), 0xffffff)
    }

}

impl Render for TextField {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let prepaint = cx.entity();
        let paint = cx.entity();
        div()
            .h(px(24.))
            .flex_shrink_0()
            .flex()
            .items_center()
            .bg(rgb(0x242a36))
            .track_focus(&self.focus)
            .key_context("TextInput")
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, window, cx| {
                    text_layout::focus(&this.focus, window, cx);
                    cx.stop_propagation();
                }),
            )
            .on_mouse_down(MouseButton::Middle, |_, _, cx| cx.stop_propagation())
            .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, window, cx| {
                if event.pressed_button != Some(MouseButton::Left) || this.is_composing() {
                    this.pointer_anchor = None;
                } else if let Some(anchor) = this.pointer_anchor {
                    let offset = this.pointer_offset(event.position.x, window);
                    this.text.selected = anchor.min(offset)..anchor.max(offset);
                    this.reversed = offset < anchor;
                    cx.notify();
                }
                cx.stop_propagation();
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.pointer_anchor = None;
                    cx.stop_propagation();
                }),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, _, _, _| {
                    this.pointer_anchor = None;
                }),
            )
            .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
            .on_key_down(cx.listener(Self::key))
            .on_action(cx.listener(|this, _: &EditorUndo, _, cx| this.restore_history(false, cx)))
            .on_action(cx.listener(|this, _: &EditorRedo, _, cx| this.restore_history(true, cx)))
            .on_action(cx.listener(|this, _: &Paste, _, cx| {
                if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
                    this.insert(None, &text, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &CopySelection, _, cx| {
                let text = this.text.substring(this.text.selected.clone()).0;
                if !text.is_empty() {
                    clipboard::write(ClipboardItem::new_string(text), cx);
                }
            }))
            .on_action(cx.listener(|this, _: &EditorCut, _, cx| {
                if this.is_composing() {
                    return;
                }
                let range = this.text.selected.clone();
                let text = this.text.substring(range.clone()).0;
                if !text.is_empty() {
                    clipboard::write(ClipboardItem::new_string(text), cx);
                    this.insert(Some(range), "", cx);
                }
            }))
            .on_action(cx.listener(|this, _: &EditorSelectAll, _, cx| {
                this.select_all_text(cx);
            }))
            .child(
                div()
                    .id("text-input")
                    .flex_1()
                    .min_w_0()
                    .h(px(24.))
                    .overflow_hidden()
                    .bg(rgb(0x12141a))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, event: &MouseDownEvent, window, cx| {
                            text_layout::focus(&this.focus, window, cx);
                            if this.is_composing() {
                                cx.stop_propagation();
                                return;
                            }
                            let offset = this.pointer_offset(event.position.x, window);
                            let anchor = if event.modifiers.shift {
                                if this.reversed {
                                    this.text.selected.end
                                } else {
                                    this.text.selected.start
                                }
                            } else {
                                offset
                            };
                            this.text.selected = anchor.min(offset)..anchor.max(offset);
                            this.reversed = offset < anchor;
                            this.pointer_anchor = Some(anchor);
                            if event.click_count >= 2 {
                                this.text.selected = if event.click_count == 2 {
                                    this.text.word_range(offset)
                                } else {
                                    0..this.text.text.encode_utf16().count()
                                };
                                this.reversed = false;
                                this.pointer_anchor = None;
                            }
                            cx.notify();
                            cx.stop_propagation();
                        }),
                    )
                    .child(
                        canvas(
                            move |bounds, window, cx| {
                                prepaint.update(cx, |this, _| {
                                    this.bounds = bounds;
                                    let line = this.line(window);
                                    let selection =
                                        this.text.byte_range(this.text.selected.clone());
                                    let caret = line.x_for_index(if this.reversed {
                                        selection.start
                                    } else {
                                        selection.end
                                    });
                                    this.scroll_x = this
                                        .scroll_x
                                        .min(caret)
                                        .max((caret - bounds.size.width + px(3.)).max(px(0.)));
                                    line
                                })
                            },
                            move |bounds, line, window, cx| {
                                let this = paint.read(cx);
                                let origin = bounds.origin - point(this.scroll_x, px(0.));
                                let selection = this.text.byte_range(this.text.selected.clone());
                                let start = line.x_for_index(selection.start);
                                let end = line.x_for_index(selection.end);
                                let focused = this.focus.is_focused(window);
                                let reversed = this.reversed;
                                let placeholder = this.placeholder.filter(|_| this.text.text.is_empty() && !this.is_composing());
                                window.handle_input(
                                    &this.focus,
                                    ElementInputHandler::new(bounds, paint.clone()),
                                    cx,
                                );
                                window.with_content_mask(Some(text_layout::mask(bounds)), |window| {
                                    if focused && !selection.is_empty() {
                                        window.paint_quad(fill(
                                            Bounds::new(
                                                origin + point(start, px(0.)),
                                                size(end - start, px(24.)),
                                            ),
                                            rgb(0x344b70),
                                        ));
                                    }
                                    let _ = line.paint(origin, px(24.), window, cx);
                                        if let Some(placeholder) = placeholder {
                                            let hint = text_layout::shape(window, placeholder.into(), window.text_style().font(), 0x858585);
                                            let _ = hint.paint(origin + point(px(5.), px(0.)), px(24.), window, cx);
                                        }
                                    if focused {
                                        window.paint_quad(fill(
                                            Bounds::new(
                                                origin
                                                    + point(
                                                        if reversed { start } else { end },
                                                        px(2.),
                                                    ),
                                                size(px(1.), px(20.)),
                                            ),
                                            rgb(0xffffff),
                                        ));
                                    }
                                });
                            },
                        )
                        .size_full(),
                    ),
            )
            .when(self.rejected_edit, |field| field.child(
                div().id("text-too-long").flex_shrink_0().px_2().text_xs()
                    .text_color(rgb(0xf18a8a))
                    .tooltip(|_, cx| cx.new(|_| pane_view::ControlHint(
                        "Text is too long. Shorten it and try again (4 KB limit). Your existing text was kept.".into()
                    )).into())
                    .child("Too long")
            ))
    }
}

impl EntityInputHandler for TextField {
    fn text_for_range(
        &mut self,
        range: Range<usize>,
        actual: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        let (text, range) = self.text.substring(range);
        *actual = Some(range);
        Some(text)
    }
    fn selected_text_range(
        &mut self,
        _: bool,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.text.selected.clone(),
            reversed: self.reversed,
        })
    }
    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        self.text.marked.clone()
    }
    fn unmark_text(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.text.marked = None;
        self.changed(cx);
    }
    fn replace_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.insert(range, text, cx);
    }
    fn replace_and_mark_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        selected: Option<Range<usize>>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range.unwrap_or_else(|| {
            self.text
                .marked
                .clone()
                .unwrap_or_else(|| self.text.selected.clone())
        });
        let retained = self.text.text.len() - self.text.byte_range(range.clone()).len();
        if text.len() <= 4096usize.saturating_sub(retained) {
            self.rejected_edit = false;
            if self.text.marked.is_none() {
                self.history.record(&self.text, self.reversed);
            }
            self.text.update(Some(range), text, selected);
            self.reversed = false;
            cx.notify();
        } else {
            self.rejected_edit = true;
            cx.notify();
        }
    }
    fn bounds_for_range(
        &mut self,
        range: Range<usize>,
        _: Bounds<Pixels>,
        window: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let line = self.line(window);
        let range = self.text.byte_range(range);
        let start = line.x_for_index(range.start);
        let end = line.x_for_index(range.end);
        Some(Bounds::new(
            self.bounds.origin + point(start - self.scroll_x, px(0.)),
            size((end - start).max(px(2.)), px(24.)),
        ))
    }
    fn character_index_for_point(
        &mut self,
        point: Point<Pixels>,
        window: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<usize> {
        Some(
            self.text.utf16_offset(
                self.line(window)
                    .closest_index_for_x(point.x - self.bounds.origin.x + self.scroll_x),
            ),
        )
    }
}
