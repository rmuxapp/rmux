//! Terminal-state-aware input encoding. Native IME commits remain UTF-8 text.
use super::*;

pub use rmux_core::keyboard::{Modifiers, Action};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KeyIdentity {
    Mapped(u32),
    Character(char),
}

/// Pair aliases exposed by the UI while retaining distinct Unicode keys.
/// This is a symbolic identity, not a hardware scan code.
pub fn key_identity(key: &str) -> Option<KeyIdentity> {
    let code = key_code(key)?;
    if code == GHOSTTY_KEY_UNIDENTIFIED {
        key.chars().next().map(KeyIdentity::Character)
    } else {
        Some(KeyIdentity::Mapped(code as u32))
    }
}

pub(crate) struct Encoder {
    encoder: GhosttyKeyEncoder,
    event: GhosttyKeyEvent,
}
impl Encoder {
    pub fn new() -> Result<Self> {
        let mut this = Self {
            encoder: ptr::null_mut(),
            event: ptr::null_mut(),
        };
        // SAFETY: owned handles; Drop handles partial initialization.
        unsafe {
            check(ghostty_key_encoder_new(ptr::null(), &mut this.encoder))?;
            check(ghostty_key_event_new(ptr::null(), &mut this.event))?;
        }
        Ok(this)
    }
}
impl Drop for Encoder {
    fn drop(&mut self) {
        unsafe {
            ghostty_key_event_free(self.event);
            ghostty_key_encoder_free(self.encoder);
        }
    }
}

fn key_code(key: &str) -> Option<GhosttyKey> {
    use rmux_core::keyboard::{key as shared_key, Key};
    Some(match shared_key(key)? {
        Key::Enter => GHOSTTY_KEY_ENTER,
        Key::Backspace => GHOSTTY_KEY_BACKSPACE,
        Key::Tab => GHOSTTY_KEY_TAB,
        Key::Escape => GHOSTTY_KEY_ESCAPE,
        Key::ArrowUp => GHOSTTY_KEY_ARROW_UP,
        Key::ArrowDown => GHOSTTY_KEY_ARROW_DOWN,
        Key::ArrowLeft => GHOSTTY_KEY_ARROW_LEFT,
        Key::ArrowRight => GHOSTTY_KEY_ARROW_RIGHT,
        Key::Home => GHOSTTY_KEY_HOME,
        Key::End => GHOSTTY_KEY_END,
        Key::Insert => GHOSTTY_KEY_INSERT,
        Key::Delete => GHOSTTY_KEY_DELETE,
        Key::PageUp => GHOSTTY_KEY_PAGE_UP,
        Key::PageDown => GHOSTTY_KEY_PAGE_DOWN,
        Key::Space => GHOSTTY_KEY_SPACE,
        Key::BracketLeft => GHOSTTY_KEY_BRACKET_LEFT,
        Key::BracketRight => GHOSTTY_KEY_BRACKET_RIGHT,
        Key::Backslash => GHOSTTY_KEY_BACKSLASH,
        Key::Slash => GHOSTTY_KEY_SLASH,
        Key::Minus => GHOSTTY_KEY_MINUS,
        Key::Equal => GHOSTTY_KEY_EQUAL,
        Key::Backquote => GHOSTTY_KEY_BACKQUOTE,
        Key::Semicolon => GHOSTTY_KEY_SEMICOLON,
        Key::Quote => GHOSTTY_KEY_QUOTE,
        Key::Comma => GHOSTTY_KEY_COMMA,
        Key::Period => GHOSTTY_KEY_PERIOD,
        Key::Digit1 => GHOSTTY_KEY_DIGIT_1,
        Key::Digit2 => GHOSTTY_KEY_DIGIT_2,
        Key::Digit3 => GHOSTTY_KEY_DIGIT_3,
        Key::Digit4 => GHOSTTY_KEY_DIGIT_4,
        Key::Digit5 => GHOSTTY_KEY_DIGIT_5,
        Key::Digit6 => GHOSTTY_KEY_DIGIT_6,
        Key::Digit7 => GHOSTTY_KEY_DIGIT_7,
        Key::Digit8 => GHOSTTY_KEY_DIGIT_8,
        Key::Digit9 => GHOSTTY_KEY_DIGIT_9,
        Key::Digit0 => GHOSTTY_KEY_DIGIT_0,
        Key::Letter(index) => GHOSTTY_KEY_A + GhosttyKey::from(index),
        Key::Function(number) => GHOSTTY_KEY_F1 + GhosttyKey::from(number) - 1,
        Key::Character(_) => GHOSTTY_KEY_UNIDENTIFIED,
    })
}

impl Terminal {
    /// Focus reports are opt-in (DEC mode 1004) and never ordinary input.
    pub fn encode_focus(&mut self, focused: bool) -> Result<Option<[u8; 3]>> {
        unsafe {
            let mut mode = GhosttyTerminalModeConfig {
                mode: 1004,
                value: false,
            };
            // SAFETY: exclusive terminal access and correctly typed output storage.
            check(ghostty_terminal_get(
                self.terminal,
                GHOSTTY_TERMINAL_DATA_MODE,
                out(&mut mode),
            ))?;
            if !mode.value {
                return Ok(None);
            }
            let mut bytes = [0; 3];
            let mut written = 0;
            check(ghostty_focus_encode(
                if focused {
                    GHOSTTY_FOCUS_GAINED
                } else {
                    GHOSTTY_FOCUS_LOST
                },
                bytes.as_mut_ptr(),
                bytes.len(),
                &mut written,
            ))?;
            ensure!(written == bytes.len(), "unexpected focus report length");
            Ok(Some(bytes))
        }
    }

    /// None means the native text/IME input path should handle the event.
    pub fn encode_key(
        &mut self,
        key: &str,
        mods: Modifiers,
        repeat: bool,
    ) -> Result<Option<Vec<u8>>> {
        self.encode_key_action(
            key,
            mods,
            if repeat {
                Action::Repeat
            } else {
                Action::Press
            },
        )
    }

    /// One symbolic key tap, including release when the application requests it.
    pub fn encode_key_tap(&mut self, key: &str, mods: Modifiers) -> Result<Vec<u8>> {
        ensure!(key_identity(key).is_some() && !key.chars().any(char::is_control), "unknown key");
        let text = if key == "space" { " ".to_owned() } else if mods.shift && key.len() == 1 && key.is_ascii() { key.to_ascii_uppercase() } else { key.to_owned() };
        let Some(mut bytes) = self.encode_key_action_with_text(key, mods, Action::Press, Some(&text))? else {
            ensure!(!mods.control && !mods.alt && (key.chars().count() == 1 || key == "space"), "key has no encoding");
            return Ok(text.into_bytes());
        };
        if let Some(release) = self.encode_key_action_with_text(key, mods, Action::Release, Some(&text))? { bytes.extend(release); }
        Ok(bytes)
    }

    pub fn encode_key_action(
        &mut self,
        key: &str,
        mods: Modifiers,
        action: Action,
    ) -> Result<Option<Vec<u8>>> {
        self.encode_key_action_with_text(key, mods, action, None)
    }

    /// Read negotiated report-all mode while the parser is exclusively owned.
    pub(crate) fn reports_all_keys(&self) -> Result<bool> {
        Ok(self.keyboard_flags()? as u32 & GHOSTTY_KITTY_KEY_REPORT_ALL != 0)
    }
    fn keyboard_flags(&self) -> Result<GhosttyKittyKeyFlags> {
        let mut flags: GhosttyKittyKeyFlags = 0;
        // SAFETY: the documented query writes a uint8 flag value.
        unsafe { check(ghostty_terminal_get(self.terminal,
            GHOSTTY_TERMINAL_DATA_KITTY_KEYBOARD_FLAGS, out(&mut flags)))?; }
        Ok(flags)
    }

    pub fn encode_key_action_with_text(
        &mut self,
        key: &str,
        mods: Modifiers,
        action: Action,
        native_text: Option<&str>,
    ) -> Result<Option<Vec<u8>>> {
        // macOS terminal conventions: readline/zle understand Meta-b/f without
        // shell-specific bindings for modified CSI arrows. Leave negotiated
        // keyboard protocols untouched so full-screen apps receive real keys.
        #[cfg(target_os = "macos")]
        if mods.alt && !mods.control && !mods.shift && matches!(key, "left" | "right")
            && self.keyboard_flags()? == 0 {
            return Ok(Some(if action == Action::Release { Vec::new() }
                else if key == "left" { b"\x1bb".to_vec() } else { b"\x1bf".to_vec() }));
        }
        let Some(code) = key_code(key) else {
            return Ok(None);
        };
        let text_key = key.chars().count() == 1 || key == "space";
        if text_key && !mods.control && !mods.alt && action != Action::Release {
            if !self.reports_all_keys()? {
                return Ok(None);
            }
        }
        let text = rmux_core::keyboard::key_text(key);
        let unshifted = text
            .chars()
            .next()
            .map_or(0, |c| c.to_ascii_lowercase() as u32);
        let text = if text_key && !mods.control && !mods.alt {
            native_text.unwrap_or(text)
        } else {
            text
        };
        let modifiers = (if mods.shift { GHOSTTY_MODS_SHIFT } else { 0 })
            | (if mods.control { GHOSTTY_MODS_CTRL } else { 0 })
            | (if mods.alt { GHOSTTY_MODS_ALT } else { 0 });
        // SAFETY: handles are exclusively owned; text remains valid through encode.
        unsafe {
            let encoder = self.keyboard.encoder;
            let event = self.keyboard.event;
            check(ghostty_key_encoder_set_from_terminal(
                encoder,
                self.terminal,
            ))?;
            let alt = GHOSTTY_OPTION_AS_ALT_TRUE;
            check(ghostty_key_encoder_set(
                encoder,
                GHOSTTY_KEY_ENCODER_OPT_MACOS_OPTION_AS_ALT,
                input(&alt),
            ))?;
            check(ghostty_key_event_set_action(
                event,
                match action {
                    Action::Press => GHOSTTY_KEY_ACTION_PRESS,
                    Action::Repeat => GHOSTTY_KEY_ACTION_REPEAT,
                    Action::Release => GHOSTTY_KEY_ACTION_RELEASE,
                },
            ))?;
            check(ghostty_key_event_set_key(event, code))?;
            check(ghostty_key_event_set_mods(event, modifiers as GhosttyMods))?;
            check(ghostty_key_event_set_utf8(
                event,
                GhosttyString {
                    ptr: text.as_ptr(),
                    len: text.len(),
                },
            ))?;
            check(ghostty_key_event_set_unshifted_codepoint(event, unshifted))?;
            let mut buffer = [0; 128];
            let mut written = 0;
            let result = ghostty_key_encoder_encode(
                encoder,
                event,
                buffer.as_mut_ptr(),
                buffer.len(),
                &mut written,
            );
            if result == GHOSTTY_OUT_OF_SPACE {
                let mut bytes = vec![0; written];
                check(ghostty_key_encoder_encode(
                    encoder,
                    event,
                    bytes.as_mut_ptr(),
                    bytes.len(),
                    &mut written,
                ))?;
                bytes.truncate(written);
                Ok(Some(bytes))
            } else {
                check(result)?;
                Ok(Some(buffer[..written].to_vec()))
            }
        }
    }

    pub fn encode_paste(&mut self, text: &str) -> Result<Vec<u8>> {
        encode_paste(text, self.bracketed_paste()?)
    }

    pub(crate) fn bracketed_paste(&self) -> Result<bool> {
        let mut mode = GhosttyTerminalModeConfig { mode: 2004, value: false };
        // SAFETY: mode 2004 is DEC private (ANSI bit unset), and the output lives
        // through the call. Session callers serialize this read with parsing.
        unsafe {
            check(ghostty_terminal_get(self.terminal, GHOSTTY_TERMINAL_DATA_MODE, out(&mut mode)))?;
        }
        Ok(mode.value)
    }
}

/// Encode against a captured mode without accessing terminal state. Payload
/// allocation and normalization need not retain the terminal/parser mutex.
pub(crate) fn encode_paste(text: &str, bracketed: bool) -> Result<Vec<u8>> {
    ensure!(text.len() <= 1024 * 1024, "paste exceeds 1 MiB input limit");
    let prefix: &[u8] = if bracketed { b"\x1b[200~" } else { b"" };
    let suffix: &[u8] = if bracketed { b"\x1b[201~" } else { b"" };
    let length = prefix.len() + text.len() + suffix.len();
    let mut bytes = Vec::with_capacity(length);
    bytes.extend_from_slice(prefix);
    bytes.extend_from_slice(text.as_bytes());
    let mut written = 0;
    // SAFETY: Ghostty's documented size-query mode sanitizes the mutable input
    // even on OUT_OF_SPACE. Only the initialized payload is passed, with no
    // output buffer; input/output never alias. Delimiters are added separately.
    let result = unsafe {
        ghostty_paste_encode(
            bytes.as_mut_ptr().add(prefix.len()).cast(), text.len(), bracketed,
            ptr::null_mut(), 0, &mut written,
        )
    };
    if result != GHOSTTY_OUT_OF_SPACE { check(result)?; }
    ensure!(written == length, "unexpected encoded paste length");
    bytes.extend_from_slice(suffix);
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(target_os = "macos")]
    #[test]
    fn option_arrows_move_words_without_overriding_negotiated_keys() -> Result<()> {
        let mut terminal = Terminal::new(20, 3)?;
        let mods = Modifiers { alt: true, ..Default::default() };
        for (key, expected) in [("left", b"\x1bb"), ("right", b"\x1bf")] {
            assert_eq!(terminal.encode_key_action(key, mods, Action::Press)?, Some(expected.to_vec()));
            assert_eq!(terminal.encode_key_action(key, mods, Action::Repeat)?, Some(expected.to_vec()));
            assert_eq!(terminal.encode_key_action(key, mods, Action::Release)?, Some(Vec::new()));
        }
        terminal.write(b"\x1b[>11u");
        let encoded = terminal.encode_key_action("left", mods, Action::Press)?.unwrap();
        assert_ne!(encoded, b"\x1bb");
        assert!(encoded.starts_with(b"\x1b["));
        assert!(!terminal.encode_key_action("left", mods, Action::Release)?.unwrap().is_empty());
        terminal.write(b"\x1b[<u");
        assert_eq!(terminal.encode_key_action("left", mods, Action::Press)?, Some(b"\x1bb".to_vec()));
        Ok(())
    }
    #[test]
    fn single_buffer_paste_matches_ghostty_output() -> Result<()> {
        let controls: String = (0u8..=127).map(char::from).collect();
        for text in ["", "hello", "界é🙂\r\n", "\x1b[201~\0\x03", controls.as_str()] {
            for bracketed in [false, true] {
                let mut input = text.as_bytes().to_vec();
                let mut expected = vec![0; input.len() + 12];
                let mut written = 0;
                unsafe {
                    check(ghostty_paste_encode(input.as_mut_ptr().cast(), input.len(), bracketed,
                        expected.as_mut_ptr(), expected.len(), &mut written))?;
                }
                expected.truncate(written);
                assert_eq!(encode_paste(text, bracketed)?, expected);
            }
        }
        Ok(())
    }
    #[test]
    fn paste_limit_preserves_complete_bracketed_payload() -> Result<()> {
        let mut terminal = Terminal::new(20, 3)?;
        terminal.write(b"\x1b[?2004h");
        let text = "x".repeat(1024 * 1024);
        let encoded = terminal.encode_paste(&text)?;
        assert_eq!(&encoded[..6], b"\x1b[200~");
        assert_eq!(&encoded[6..encoded.len() - 6], text.as_bytes());
        assert_eq!(&encoded[encoded.len() - 6..], b"\x1b[201~");
        assert!(terminal.encode_paste(&(text + "x")).is_err());
        Ok(())
    }
    #[test]
    fn key_identity_pairs_shift_aliases_without_merging_unicode() {
        for (pressed, released) in [
            ("?", "/"),
            ("_", "-"),
            ("@", "2"),
            ("A", "a"),
            (" ", "space"),
        ] {
            assert_eq!(key_identity(pressed), key_identity(released));
        }
        assert_ne!(key_identity("é"), key_identity("界"));
        assert_ne!(key_identity("a"), key_identity("b"));
        assert_eq!(key_identity("unknown-key"), None);
    }
    #[test]
    fn report_all_keys_encodes_plain_press_repeat_and_release() -> Result<()> {
        let mut terminal = Terminal::new(20, 3)?;
        let mods = Modifiers::default();
        assert_eq!(terminal.encode_key("a", mods, false)?, None);
        assert_eq!(terminal.encode_key("é", mods, false)?, None);
        terminal.write(b"\x1b[>11u");
        assert_eq!(
            terminal.encode_key("é", mods, false)?,
            Some(b"\x1b[233u".to_vec())
        );
        for (action, expected) in [
            (Action::Press, "\x1b[97u"),
            (Action::Repeat, "\x1b[97;1:2u"),
            (Action::Release, "\x1b[97;1:3u"),
        ] {
            assert_eq!(
                terminal.encode_key_action("a", mods, action)?,
                Some(expected.as_bytes().to_vec())
            );
        }
        terminal.write(b"\x1b[<u");
        assert_eq!(terminal.encode_key("a", mods, false)?, None);
        terminal.write(b"\x1b[>24u");
        assert_eq!(
            terminal.encode_key_action_with_text(
                "a",
                Modifiers {
                    shift: true,
                    ..mods
                },
                Action::Press,
                Some("A")
            )?,
            Some(b"\x1b[97;2;65u".to_vec())
        );
        Ok(())
    }
    #[test]
    fn focus_reports_require_application_opt_in() -> Result<()> {
        let mut terminal = Terminal::new(40, 4)?;
        assert_eq!(terminal.encode_focus(true)?, None);
        terminal.write(b"\x1b[?1004h");
        assert_eq!(terminal.encode_focus(true)?, Some(*b"\x1b[I"));
        assert_eq!(terminal.encode_focus(false)?, Some(*b"\x1b[O"));
        terminal.write(b"\x1b[?1004l");
        assert_eq!(terminal.encode_focus(false)?, None);
        Ok(())
    }
    #[test]
    fn key_release_and_repeat_follow_negotiated_protocol() -> Result<()> {
        let mut terminal = Terminal::new(40, 4)?;
        let mods = Modifiers {
            shift: true,
            ..Modifiers::default()
        };
        assert_eq!(
            terminal.encode_key_action("up", mods, Action::Release)?,
            Some(vec![])
        );
        terminal.write(b"\x1b[>3u");
        assert_eq!(
            terminal.encode_key_action("c", Modifiers::default(), Action::Release)?,
            Some(b"\x1b[99;1:3u".to_vec())
        );
        assert_eq!(
            terminal.encode_key_action("up", mods, Action::Press)?,
            Some(b"\x1b[1;2:1A".to_vec())
        );
        assert_eq!(
            terminal.encode_key_action("up", mods, Action::Repeat)?,
            Some(b"\x1b[1;2:2A".to_vec())
        );
        assert_eq!(
            terminal.encode_key_action("up", mods, Action::Release)?,
            Some(b"\x1b[1;2:3A".to_vec())
        );
        terminal.write(b"\x1b[<u");
        assert_eq!(
            terminal.encode_key_action("up", mods, Action::Release)?,
            Some(vec![])
        );
        Ok(())
    }
    #[test]
    fn symbolic_taps_follow_modes_and_pair_releases() -> Result<()> {
        let mut terminal = Terminal::new(40, 4)?;
        let plain = Modifiers::default();
        assert_eq!(terminal.encode_key_tap("enter", plain)?, b"\r");
        assert_eq!(terminal.encode_key_tap("a", Modifiers { shift: true, ..plain })?, b"A");
        assert_eq!(terminal.encode_key_tap("界", plain)?, "界".as_bytes());
        assert_eq!(terminal.encode_key_tap("c", Modifiers { control: true, ..plain })?, vec![3]);
        terminal.write(b"\x1b[?1h");
        assert_eq!(terminal.encode_key_tap("up", plain)?, b"\x1bOA");
        terminal.write(b"\x1b[>11u");
        assert_eq!(terminal.encode_key_tap("a", plain)?, b"\x1b[97u\x1b[97;1:3u");
        assert!(terminal.encode_key_tap("unknown", plain).is_err());
        Ok(())
    }

    #[test]
    fn encodes_terminal_modes_modifiers_and_paste() -> Result<()> {
        let mut terminal = Terminal::new(40, 4)?;
        let plain = Modifiers::default();
        assert_eq!(terminal.encode_key("@", plain, false)?, None);
        assert_eq!(
            terminal.encode_key(
                "@",
                Modifiers {
                    control: true,
                    shift: true,
                    ..plain
                },
                false
            )?,
            Some(b"\x1b[64;6u".to_vec())
        );
        for (key, byte) in [("_", 31), ("^", 30), ("?", 127)] {
            assert_eq!(
                terminal.encode_key(
                    key,
                    Modifiers {
                        control: true,
                        shift: true,
                        ..plain
                    },
                    false
                )?,
                Some(vec![byte]),
                "{key}"
            );
            assert_eq!(terminal.encode_key(key, plain, false)?, None);
        }
        assert_eq!(
            terminal.encode_key("up", plain, false)?,
            Some(b"\x1b[A".to_vec())
        );
        terminal.write(b"\x1b[?1h");
        assert_eq!(
            terminal.encode_key("up", plain, false)?,
            Some(b"\x1bOA".to_vec())
        );
        assert_eq!(
            terminal.encode_key(
                "c",
                Modifiers {
                    control: true,
                    ..plain
                },
                false
            )?,
            Some(vec![3])
        );
        assert_eq!(
            terminal.encode_key(
                "tab",
                Modifiers {
                    shift: true,
                    ..plain
                },
                false
            )?,
            Some(b"\x1b[Z".to_vec())
        );
        assert_eq!(terminal.encode_key("a", plain, false)?, None);
        assert_eq!(terminal.encode_paste("a\nb")?, b"a\rb");
        terminal.write(b"\x1b[?2004h");
        assert_eq!(terminal.encode_paste("a\nb")?, b"\x1b[200~a\nb\x1b[201~");
        assert_eq!(
            terminal.encode_paste("\x1b[201~")?,
            b"\x1b[200~ [201~\x1b[201~"
        );
        Ok(())
    }
}
