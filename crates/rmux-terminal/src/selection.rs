//! Tracked Ghostty gestures; borrowed grid references never escape an engine call.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    Press,
    Drag,
    Release,
    Autoscroll,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Event {
    pub action: Action,
    pub x: f32,
    pub y: f32,
    pub cell_width: f32,
    pub cell_height: f32,
    pub rectangle: bool,
    pub clicks: usize,
}

#[cfg(test)]
mod tests {
    #[test]
    fn copy_can_preserve_soft_wraps_without_changing_selection() -> anyhow::Result<()> {
        let mut terminal = super::Terminal::new(5, 4)?;
        terminal.write(b"abcdefghij\r\nnext");
        terminal.select_all()?;
        let before = terminal.frame()?;
        assert_eq!(terminal.selected_text_with_wraps(100, true)?.as_deref(), Some("abcdefghij\nnext"));
        assert_eq!(terminal.selected_text_with_wraps(100, false)?.as_deref(), Some("abcde\nfghij\nnext"));
        assert!(terminal.selected_text_with_wraps(15, false).is_err());
        assert_eq!(terminal.selected_text_with_wraps(16, false)?.unwrap().len(), 16);
        assert_eq!(terminal.selection_text_prefix_with_wraps(6, false)?, Some(("abcde\n".into(), true)));
        assert_eq!(terminal.selection_text_prefix_with_wraps(6, true)?, Some(("abcdef".into(), true)));
        let after = terminal.frame()?;
        assert_eq!(after.selections, before.selections);
        assert_eq!(after.scrollback_offset, before.scrollback_offset);
        Ok(())
    }
    use super::*;
    fn click(terminal: &mut Terminal, column: f32, row: f32, clicks: usize) -> Result<()> {
        terminal
            .selection_event(Event {
                action: Action::Press,
                x: column * 10.,
                y: row * 20.,
                cell_width: 10.,
                cell_height: 20.,
                rectangle: false,
                clicks,
            })
            .map(|_| ())
    }
    #[test]
    fn select_all_covers_history_unicode_and_only_the_active_screen() -> Result<()> {
        let mut terminal = Terminal::new(20, 3)?;
        terminal.write("first 世界\r\nsecond\r\nthird\r\nfourth".as_bytes());
        terminal.select_all()?;
        let copied = terminal.selected_text()?.unwrap();
        assert!(
            copied.starts_with("first 世界\nsecond\nthird\nfourth"),
            "{copied:?}"
        );
        terminal.write(b"\x1b[?1049hALT");
        terminal.select_all()?;
        let alternate = terminal.selected_text()?.unwrap();
        assert!(alternate.contains("ALT"));
        assert!(!alternate.contains("first"));
        terminal.write(b"\x1b[?1049l");
        terminal.select_all()?;
        assert!(terminal.selected_text()?.unwrap().contains("first 世界"));
        terminal.clear_selection()?;
        assert!(terminal.selected_text()?.is_none());
        Ok(())
    }
    #[test]
    fn word_line_copy_and_clear_use_engine_selection() -> Result<()> {
        let mut terminal = Terminal::new(20, 4)?;
        terminal.write("hello 世界\r\nsecond line".as_bytes());
        click(&mut terminal, 1., 0., 2)?;
        assert_eq!(terminal.selected_text()?.as_deref(), Some("hello"));
        let frame = terminal.frame()?;
        assert_eq!(frame.selections[0], Some((0, 4)));
        terminal.write(b"\r\nmore output");
        assert_eq!(terminal.selected_text()?.as_deref(), Some("hello"));
        click(&mut terminal, 7., 0., 2)?;
        assert_eq!(terminal.selected_text()?.as_deref(), Some("世界"));
        assert!(terminal.selected_text_limited(5).is_err());
        assert_eq!(terminal.selected_text_limited(6)?.as_deref(), Some("世界"));
        click(&mut terminal, 1., 1., 3)?;
        assert_eq!(terminal.selected_text()?.as_deref(), Some("second line"));
        terminal.clear_selection()?;
        assert!(terminal.selected_text()?.is_none());
        assert!(terminal.frame()?.selections.iter().all(Option::is_none));
        assert_eq!(frame.selections[0], Some((0, 4)));
        Ok(())
    }
    #[test]
    fn drag_and_rectangle_at_fractional_cell_width() -> Result<()> {
        let mut terminal = Terminal::new(20, 4)?;
        terminal.write(b"abcdef\r\nuvwxyz");
        let mut event = Event {
            action: Action::Press,
            x: 8.5,
            y: 0.,
            cell_width: 8.5,
            cell_height: 20.,
            rectangle: false,
            clicks: 1,
        };
        terminal.selection_event(event)?;
        event.action = Action::Drag;
        event.x = 4. * 8.5;
        terminal.selection_event(event)?;
        assert_eq!(terminal.selected_text()?.as_deref(), Some("bcd"));
        event.rectangle = true;
        event.y = 20.;
        terminal.selection_event(event)?;
        assert_eq!(terminal.selected_text()?.as_deref(), Some("bcd\nvwx"));
        event.action = Action::Release;
        terminal.selection_event(event)?;
        assert_eq!(terminal.selected_text()?.as_deref(), Some("bcd\nvwx"));
        Ok(())
    }
    #[test]
    fn edge_ticks_extend_selection_and_stop_at_history_boundary() -> Result<()> {
        let mut terminal = Terminal::new(20, 3)?;
        for line in 0..8 {
            terminal.write(format!("line-{line}\r\n").as_bytes());
        }
        let mut event = Event {
            action: Action::Press,
            x: 20.,
            y: 20.,
            cell_width: 10.,
            cell_height: 20.,
            rectangle: false,
            clicks: 1,
        };
        terminal.selection_event(event)?;
        event.action = Action::Drag;
        event.y = -20.;
        event.x = 0.;
        assert!(terminal.selection_event(event)?);
        event.action = Action::Autoscroll;
        let mut ticks = 0;
        while terminal.selection_event(event)? {
            ticks += 1;
            assert!(ticks < 20);
        }
        assert!(terminal.frame()?.scrollback_offset > 0);
        assert!(terminal.selected_text()?.unwrap().contains("line-0"));
        event.action = Action::Drag;
        event.y = 80.;
        assert!(terminal.selection_event(event)?);
        event.action = Action::Autoscroll;
        ticks = 0;
        while terminal.selection_event(event)? {
            ticks += 1;
            assert!(ticks < 20);
        }
        assert_eq!(terminal.frame()?.scrollback_offset, 0);
        event.action = Action::Drag;
        event.y = 20.;
        assert!(!terminal.selection_event(event)?);
        event.action = Action::Release;
        assert!(!terminal.selection_event(event)?);
        Ok(())
    }
}

pub(crate) struct Gesture {
    handle: GhosttySelectionGesture,
    events: [GhosttySelectionGestureEvent; 4],
}
impl Gesture {
    pub fn new() -> Result<Self> {
        let mut this = Self {
            handle: ptr::null_mut(),
            events: [ptr::null_mut(); 4],
        };
        // SAFETY: owned handles, including partially initialized handles on error.
        unsafe {
            check(ghostty_selection_gesture_new(ptr::null(), &mut this.handle))?;
            for (index, kind) in [
                GHOSTTY_SELECTION_GESTURE_EVENT_TYPE_PRESS,
                GHOSTTY_SELECTION_GESTURE_EVENT_TYPE_DRAG,
                GHOSTTY_SELECTION_GESTURE_EVENT_TYPE_RELEASE,
                GHOSTTY_SELECTION_GESTURE_EVENT_TYPE_AUTOSCROLL_TICK,
            ]
            .into_iter()
            .enumerate()
            {
                check(ghostty_selection_gesture_event_new(
                    ptr::null(),
                    &mut this.events[index],
                    kind,
                ))?;
            }
        }
        Ok(this)
    }
    pub fn reset(&mut self, terminal: GhosttyTerminal) {
        // SAFETY: gesture is used only with this terminal, while it is alive.
        unsafe {
            ghostty_selection_gesture_reset(self.handle, terminal);
        }
    }
}
impl Drop for Gesture {
    fn drop(&mut self) {
        // SAFETY: Terminal::drop resets tracked refs before freeing its terminal.
        unsafe {
            for event in self.events {
                ghostty_selection_gesture_event_free(event);
            }
            ghostty_selection_gesture_free(self.handle, ptr::null_mut());
        }
    }
}

impl Terminal {
    /// Returns whether an edge-autoscroll tick can advance the viewport.
    pub fn selection_event(&mut self, event: Event) -> Result<bool> {
        self.revision = self.revision.wrapping_add(1);
        ensure!(
            event.cell_width > 0. && event.cell_height > 0.,
            "invalid selection geometry"
        );
        let index = match event.action {
            Action::Press => 0,
            Action::Drag => 1,
            Action::Release => 2,
            Action::Autoscroll => 3,
        };
        let handle = self.selection.events[index];
        // Ghostty geometry uses integer cell widths. Normalize surface units so
        // fractional logical font widths don't shift the selection boundary.
        let scale = 1024. / event.cell_width as f64;
        let point = GhosttyPoint {
            tag: GHOSTTY_POINT_TAG_VIEWPORT,
            value: GhosttyPointValue {
                coordinate: GhosttyPointCoordinate {
                    x: ((event.x.max(0.) / event.cell_width) as u16).min(self.frame.columns - 1),
                    y: ((event.y.max(0.) / event.cell_height) as u32)
                        .min(self.frame.rows.len().saturating_sub(1) as u32),
                },
            },
        };
        // SAFETY: each grid snapshot is obtained and consumed under exclusive
        // access before any terminal mutation. Ghostty tracks the gesture anchor.
        unsafe {
            if matches!(event.action, Action::Autoscroll) {
                check(ghostty_selection_gesture_event_set(
                    handle,
                    GHOSTTY_SELECTION_GESTURE_EVENT_OPT_VIEWPORT,
                    input(&point.value.coordinate),
                ))?;
            } else {
                let mut grid = GhosttyGridRef::default();
                check(ghostty_terminal_grid_ref(self.terminal, point, &mut grid))?;
                check(ghostty_selection_gesture_event_set(
                    handle,
                    GHOSTTY_SELECTION_GESTURE_EVENT_OPT_REF,
                    input(&grid),
                ))?;
            }
            if !matches!(event.action, Action::Release) {
                let position = GhosttySurfacePosition {
                    x: event.x as f64 * scale,
                    y: event.y as f64 * scale,
                };
                check(ghostty_selection_gesture_event_set(
                    handle,
                    GHOSTTY_SELECTION_GESTURE_EVENT_OPT_POSITION,
                    input(&position),
                ))?;
            }
            if matches!(event.action, Action::Press) {
                // GPUI supplies the native click count; use the matching behavior
                // for this untimed press rather than inventing another click timer.
                let behavior = match event.clicks {
                    2 => GHOSTTY_SELECTION_GESTURE_BEHAVIOR_WORD,
                    3.. => GHOSTTY_SELECTION_GESTURE_BEHAVIOR_LINE,
                    _ => GHOSTTY_SELECTION_GESTURE_BEHAVIOR_CELL,
                };
                let behaviors = GhosttySelectionGestureBehaviors {
                    single_click: behavior,
                    double_click: behavior,
                    triple_click: behavior,
                };
                check(ghostty_selection_gesture_event_set(
                    handle,
                    GHOSTTY_SELECTION_GESTURE_EVENT_OPT_BEHAVIORS,
                    input(&behaviors),
                ))?;
            }
            if matches!(event.action, Action::Drag | Action::Autoscroll) {
                let geometry = GhosttySelectionGestureGeometry {
                    columns: self.frame.columns.into(),
                    cell_width: 1024,
                    padding_left: 0,
                    screen_height: (self.frame.rows.len() as f64 * event.cell_height as f64 * scale)
                        .round()
                        .max(1.) as u32,
                };
                check(ghostty_selection_gesture_event_set(
                    handle,
                    GHOSTTY_SELECTION_GESTURE_EVENT_OPT_GEOMETRY,
                    input(&geometry),
                ))?;
                check(ghostty_selection_gesture_event_set(
                    handle,
                    GHOSTTY_SELECTION_GESTURE_EVENT_OPT_RECTANGLE,
                    input(&event.rectangle),
                ))?;
            }
            let mut selected = GhosttySelection::default();
            let result = ghostty_selection_gesture_event(
                self.selection.handle,
                self.terminal,
                handle,
                &mut selected,
            );
            if result == GHOSTTY_SUCCESS {
                check(ghostty_terminal_set(
                    self.terminal,
                    GHOSTTY_TERMINAL_OPT_SELECTION,
                    input(&selected),
                ))?;
            } else if result == GHOSTTY_NO_VALUE {
                if matches!(event.action, Action::Autoscroll) {
                    return Ok(false);
                }
                if matches!(event.action, Action::Press) {
                    self.clear_selection()?;
                }
            } else {
                check(result)?;
            }
            if matches!(event.action, Action::Drag | Action::Autoscroll) {
                let mut direction = GHOSTTY_SELECTION_GESTURE_AUTOSCROLL_NONE;
                check(ghostty_selection_gesture_get(
                    self.selection.handle,
                    self.terminal,
                    GHOSTTY_SELECTION_GESTURE_DATA_AUTOSCROLL,
                    out(&mut direction),
                ))?;
                let mut scrollbar = GhosttyTerminalScrollbar::default();
                check(ghostty_terminal_get(
                    self.terminal,
                    GHOSTTY_TERMINAL_DATA_SCROLLBAR,
                    out(&mut scrollbar),
                ))?;
                return Ok(match direction {
                    GHOSTTY_SELECTION_GESTURE_AUTOSCROLL_UP => scrollbar.offset > 0,
                    GHOSTTY_SELECTION_GESTURE_AUTOSCROLL_DOWN => {
                        scrollbar.offset + scrollbar.len < scrollbar.total
                    }
                    _ => false,
                });
            }
        }
        Ok(false)
    }

    pub fn select_all(&mut self) -> Result<()> {
        self.selection.reset(self.terminal);
        // SAFETY: derive and install the borrowed snapshot under one exclusive
        // engine access; no grid references escape this operation.
        unsafe {
            let mut selection = GhosttySelection::default();
            let result = ghostty_terminal_select_all(self.terminal, &mut selection);
            if result == GHOSTTY_NO_VALUE {
                return self.clear_selection();
            }
            check(result)?;
            check(ghostty_terminal_set(
                self.terminal,
                GHOSTTY_TERMINAL_OPT_SELECTION,
                input(&selection),
            ))
        }
    }

    pub fn clear_selection(&mut self) -> Result<()> {
        unsafe {
            check(ghostty_terminal_set(
                self.terminal,
                GHOSTTY_TERMINAL_OPT_SELECTION,
                ptr::null(),
            ))
        }
    }

    pub fn selected_text(&self) -> Result<Option<String>> {
        self.selected_text_limited(16 * 1024 * 1024)
    }

    pub fn selected_text_limited(&self, max_bytes: usize) -> Result<Option<String>> {
        self.selected_text_with_wraps(max_bytes, true)
    }

    /// When `unwrap` is false, retain line breaks at terminal soft-wrap boundaries.
    pub fn selected_text_with_wraps(&self, max_bytes: usize, unwrap: bool) -> Result<Option<String>> {
        let Some((text, truncated)) = self.export_text_with_wraps(max_bytes, true, unwrap)? else {
            return Ok(None);
        };
        ensure!(!truncated, "selection exceeds {max_bytes} byte limit");
        Ok(Some(text))
    }
}
