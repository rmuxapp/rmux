//! Mouse reporting uses Ghostty's negotiated tracking and wire format.
use super::*;

#[derive(Clone, Copy)]
pub enum Action {
    Press,
    Release,
    Motion,
}
#[derive(Clone, Copy)]
pub enum Button {
    Left,
    Middle,
    Right,
    WheelUp,
    WheelDown,
}
#[derive(Clone, Copy)]
pub struct Event {
    pub action: Action,
    pub button: Option<Button>,
    pub x: f32,
    pub y: f32,
    pub width: u32,
    pub height: u32,
    pub cell_width: f32,
    pub cell_height: f32,
    pub pressed: bool,
    pub modifiers: keyboard::Modifiers,
}

pub(crate) struct Encoder {
    encoder: GhosttyMouseEncoder,
    event: GhosttyMouseEvent,
    pub refresh: bool,
    geometry: Option<(u32, u32, u32, u32)>,
}
impl Encoder {
    pub fn new() -> Result<Self> {
        let mut this = Self {
            encoder: ptr::null_mut(),
            event: ptr::null_mut(),
            refresh: true,
            geometry: None,
        };
        unsafe {
            check(ghostty_mouse_encoder_new(ptr::null(), &mut this.encoder))?;
            check(ghostty_mouse_event_new(ptr::null(), &mut this.event))?;
            check(ghostty_mouse_encoder_set(
                this.encoder,
                GHOSTTY_MOUSE_ENCODER_OPT_TRACK_LAST_CELL,
                input(&true),
            ))?;
        }
        Ok(this)
    }
}
impl Drop for Encoder {
    fn drop(&mut self) {
        unsafe {
            ghostty_mouse_event_free(self.event);
            ghostty_mouse_encoder_free(self.encoder);
        }
    }
}
impl Terminal {
    pub fn mouse_tracking(&self) -> Result<bool> {
        let mut tracking = false;
        unsafe {
            check(ghostty_terminal_get(self.terminal, GHOSTTY_TERMINAL_DATA_MOUSE_TRACKING, out(&mut tracking)))?;
        }
        Ok(tracking)
    }
    /// None means local scrolling/selection should handle this event. An empty
    /// Some means reporting is active but the selected mode suppresses it.
    pub fn encode_mouse(&mut self, event: Event) -> Result<Option<Vec<u8>>> {
        ensure!(
            event.x.is_finite() && event.y.is_finite(),
            "non-finite mouse position"
        );
        ensure!(
            event.cell_width.is_finite()
                && event.cell_height.is_finite()
                && event.cell_width > 0.
                && event.cell_height > 0.,
            "invalid mouse cell geometry"
        );
        // SAFETY: all handles are owned by this terminal and all pointers refer
        // to initialized, correctly typed values for the duration of each call.
        unsafe {
            if !self.mouse_tracking()? {
                return Ok(None);
            }
            let encoder = self.mouse.encoder;
            let handle = self.mouse.event;
            if self.mouse.refresh {
                check(ghostty_mouse_encoder_set_from_terminal(
                    encoder,
                    self.terminal,
                ))?;
                self.mouse.refresh = false;
            }
            let mut pixels = GhosttyTerminalModeConfig {
                mode: 1016,
                value: false,
            };
            check(ghostty_terminal_get(
                self.terminal,
                GHOSTTY_TERMINAL_DATA_MODE,
                out(&mut pixels),
            ))?;
            // Cell protocols use normalized cell coordinates, preserving exact
            // hit-testing even at fractional cell widths/DPI. SGR pixel reports
            // retain actual surface pixel coordinates.
            let (x, y, geometry_key) = if pixels.value {
                (
                    event.x,
                    event.y,
                    (
                        event.width,
                        event.height,
                        event.cell_width.ceil() as u32,
                        event.cell_height.ceil() as u32,
                    ),
                )
            } else {
                (
                    (event.x / event.cell_width).clamp(0., self.frame.columns.saturating_sub(1) as f32),
                    (event.y / event.cell_height).clamp(0., self.frame.rows.len().saturating_sub(1) as f32),
                    (
                        u32::from(self.frame.columns),
                        self.frame.rows.len() as u32,
                        1,
                        1,
                    ),
                )
            };
            let geometry = GhosttyMouseEncoderSize {
                size: size_of::<GhosttyMouseEncoderSize>(),
                screen_width: geometry_key.0,
                screen_height: geometry_key.1,
                cell_width: geometry_key.2,
                cell_height: geometry_key.3,
                ..Default::default()
            };
            if self.mouse.geometry != Some(geometry_key) {
                check(ghostty_mouse_encoder_set(
                    encoder,
                    GHOSTTY_MOUSE_ENCODER_OPT_SIZE,
                    input(&geometry),
                ))?;
                self.mouse.geometry = Some(geometry_key);
            }
            check(ghostty_mouse_encoder_set(
                encoder,
                GHOSTTY_MOUSE_ENCODER_OPT_ANY_BUTTON_PRESSED,
                input(&event.pressed),
            ))?;
            let action = match event.action {
                Action::Press => GHOSTTY_MOUSE_ACTION_PRESS,
                Action::Release => GHOSTTY_MOUSE_ACTION_RELEASE,
                Action::Motion => GHOSTTY_MOUSE_ACTION_MOTION,
            };
            check(ghostty_mouse_event_set_action(handle, action))?;
            if let Some(button) = event.button {
                check(ghostty_mouse_event_set_button(
                    handle,
                    match button {
                        Button::Left => GHOSTTY_MOUSE_BUTTON_LEFT,
                        Button::Middle => GHOSTTY_MOUSE_BUTTON_MIDDLE,
                        Button::Right => GHOSTTY_MOUSE_BUTTON_RIGHT,
                        Button::WheelUp => GHOSTTY_MOUSE_BUTTON_FOUR,
                        Button::WheelDown => GHOSTTY_MOUSE_BUTTON_FIVE,
                    },
                ))?;
            } else {
                check(ghostty_mouse_event_clear_button(handle))?;
            }
            let mods = (if event.modifiers.shift {
                GHOSTTY_MODS_SHIFT
            } else {
                0
            }) | (if event.modifiers.control {
                GHOSTTY_MODS_CTRL
            } else {
                0
            }) | (if event.modifiers.alt {
                GHOSTTY_MODS_ALT
            } else {
                0
            });
            check(ghostty_mouse_event_set_mods(handle, mods as GhosttyMods))?;
            check(ghostty_mouse_event_set_position(
                handle,
                GhosttyMousePosition { x, y },
            ))?;
            // Mouse reports have bounded numeric coordinates and no text payload.
            let mut bytes = [0; 128];
            let mut written = 0;
            check(ghostty_mouse_encoder_encode(
                encoder,
                handle,
                bytes.as_mut_ptr(),
                bytes.len(),
                &mut written,
            ))?;
            Ok(Some(bytes[..written].to_vec()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn trailing_partial_cells_do_not_expand_reported_grid() -> Result<()> {
        let mut terminal = Terminal::new(10, 3)?;
        terminal.write(b"\x1b[?1000h\x1b[?1006h");
        let event = Event {
            action: Action::Press, button: Some(Button::Left),
            x: 88., y: 64., width: 89, height: 65,
            cell_width: 8.5, cell_height: 20., pressed: true,
            modifiers: keyboard::Modifiers::default(),
        };
        assert_eq!(terminal.encode_mouse(event)?, Some(b"\x1b[<0;10;3M".to_vec()));
        terminal.resize(8, 2)?;
        assert_eq!(terminal.encode_mouse(event)?, Some(b"\x1b[<0;8;2M".to_vec()));
        Ok(())
    }
    #[test]
    fn negotiated_mouse_reports_and_motion_filtering() -> Result<()> {
        let mut terminal = Terminal::new(100, 20)?;
        let mut event = Event {
            action: Action::Press,
            button: Some(Button::Left),
            x: 20.,
            y: 35.,
            width: 800,
            height: 400,
            cell_width: 8.,
            cell_height: 20.,
            pressed: true,
            modifiers: keyboard::Modifiers::default(),
        };
        assert!(terminal.encode_mouse(event)?.is_none());
        terminal.write(b"\x1b[?1000h\x1b[?1006h");
        assert_eq!(
            terminal.encode_mouse(event)?,
            Some(b"\x1b[<0;3;2M".to_vec())
        );
        event.action = Action::Release;
        event.pressed = false;
        assert_eq!(
            terminal.encode_mouse(event)?,
            Some(b"\x1b[<0;3;2m".to_vec())
        );
        event.action = Action::Motion;
        event.button = None;
        assert_eq!(terminal.encode_mouse(event)?, Some(vec![]));
        terminal.write(b"\x1b[?1003h");
        assert!(!terminal.encode_mouse(event)?.unwrap().is_empty());
        assert_eq!(terminal.encode_mouse(event)?, Some(vec![]));
        event.action = Action::Press;
        event.button = Some(Button::WheelUp);
        assert_eq!(
            terminal.encode_mouse(event)?,
            Some(b"\x1b[<64;3;2M".to_vec())
        );
        event.x = 80.;
        event.cell_width = 8.5;
        assert_eq!(
            terminal.encode_mouse(event)?,
            Some(b"\x1b[<64;10;2M".to_vec())
        );
        terminal.write(b"\x1b[?1016h");
        assert_eq!(
            terminal.encode_mouse(event)?,
            Some(b"\x1b[<64;80;35M".to_vec())
        );
        terminal.write(b"\x1b[?1000l\x1b[?1003l");
        assert!(terminal.encode_mouse(event)?.is_none());
        Ok(())
    }
}
