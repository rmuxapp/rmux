//! Symbolic keyboard mapping shared by native and browser input hosts.
#[derive(Clone, Copy, Default)]
pub struct Modifiers {
    pub shift: bool,
    pub control: bool,
    pub alt: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Press,
    Repeat,
    Release,
}


#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    Enter, Backspace, Tab, Escape, ArrowUp, ArrowDown, ArrowLeft, ArrowRight, Home, End, Insert, Delete, PageUp, PageDown, Space, BracketLeft, BracketRight, Backslash, Slash, Minus, Equal, Backquote, Semicolon, Quote, Comma, Period, Digit1, Digit2, Digit3, Digit4, Digit5, Digit6, Digit7, Digit8, Digit9, Digit0,
    Letter(u8), Function(u8), Character(char),
}

/// Tracks only presses a host actually delivered to a terminal. Preserve the
/// first spelling through repeats and shifted aliases for its release report.
#[derive(Default)]
pub struct PressedKeys(std::collections::HashMap<Key, String>);
pub fn key_text(name: &str) -> &str {
    if name == "space" { " " } else if name.chars().count() == 1 { name } else { "" }
}
impl PressedKeys {
    pub fn press(&mut self, name: &str) {
        if let Some(identity) = key(name) {
            self.0.entry(identity).or_insert_with(|| name.to_owned());
        }
    }
    pub fn release(&mut self, name: &str) -> Option<String> {
        self.0.remove(&key(name)?)
    }
    /// Keep release ownership until a bounded input queue accepts the batch.
    /// On rejection, key-up or a later focus transition can retry the same keys.
    pub fn try_release_all<E>(&mut self, enqueue: impl FnOnce(Vec<String>) -> Result<(), E>) -> Result<(), E> {
        enqueue(self.0.values().cloned().collect())?;
        self.0.clear();
        Ok(())
    }
    pub fn drain(&mut self) -> impl Iterator<Item = String> + '_ {
        self.0.drain().map(|(_, name)| name)
    }
}

pub fn key(key: &str) -> Option<Key> {
    Some(match key {
        "enter" => Key::Enter,
        "backspace" => Key::Backspace,
        "tab" => Key::Tab,
        "escape" => Key::Escape,
        "up" => Key::ArrowUp,
        "down" => Key::ArrowDown,
        "left" => Key::ArrowLeft,
        "right" => Key::ArrowRight,
        "home" => Key::Home,
        "end" => Key::End,
        "insert" => Key::Insert,
        "delete" => Key::Delete,
        "pageup" => Key::PageUp,
        "pagedown" => Key::PageDown,
        "space" | " " => Key::Space,
        "[" | "{" => Key::BracketLeft,
        "]" | "}" => Key::BracketRight,
        "\\" | "|" => Key::Backslash,
        "/" | "?" => Key::Slash,
        "-" | "_" => Key::Minus,
        "=" | "+" => Key::Equal,
        "`" | "~" => Key::Backquote,
        ";" | ":" => Key::Semicolon,
        "'" | "\"" => Key::Quote,
        "," | "<" => Key::Comma,
        "." | ">" => Key::Period,
        "!" => Key::Digit1,
        "@" => Key::Digit2,
        "#" => Key::Digit3,
        "$" => Key::Digit4,
        "%" => Key::Digit5,
        "^" => Key::Digit6,
        "&" => Key::Digit7,
        "*" => Key::Digit8,
        "(" => Key::Digit9,
        ")" => Key::Digit0,
        value if value.len() == 1 && value.as_bytes()[0].is_ascii_alphabetic() => Key::Letter(value.as_bytes()[0].to_ascii_lowercase() - b'a'),
        value if value.len() == 1 && value.as_bytes()[0].is_ascii_digit() => match value.as_bytes()[0] - b'0' {
            0 => Key::Digit0,
            1 => Key::Digit1,
            2 => Key::Digit2,
            3 => Key::Digit3,
            4 => Key::Digit4,
            5 => Key::Digit5,
            6 => Key::Digit6,
            7 => Key::Digit7,
            8 => Key::Digit8,
            9 => Key::Digit9,
            _ => unreachable!(),
        },
        value if value.chars().count() == 1 => Key::Character(value.chars().next()?),
        value => {
            let number: u8 = value.strip_prefix('f')?.parse().ok()?;
            if !(1..=24).contains(&number) { return None; }
            Key::Function(number)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::{key, Key};
    #[test]
    fn releases_pair_with_delivered_presses_and_drain_once() {
        let mut held = super::PressedKeys::default();
        assert_eq!(held.release("a"),None);
        held.press("{"); held.press("[");
        assert_eq!(held.release("["),Some("{".into()));
        assert_eq!(held.release("{"),None);
        held.press("é"); held.press("界"); held.press("unknown-key");
        assert_eq!(held.release("é"),Some("é".into()));
        assert_eq!(held.drain().collect::<Vec<_>>(),vec!["界"]);
        assert_eq!(held.release("界"),None);
    }
    #[test]
    fn rejected_release_batch_keeps_keys_until_queue_accepts() {
        let mut held = super::PressedKeys::default();
        held.press("{");
        held.press("["); // A repeat/alias must not add a second release.
        held.press("界");
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        sender.try_send(vec!["occupied".to_owned()]).unwrap();
        assert!(held.try_release_all(|keys| sender.try_send(keys)).is_err());
        assert_eq!(receiver.recv().unwrap(), vec!["occupied"]);
        // A key-up arriving after the failed batch still owns its release.
        assert_eq!(held.release("["), Some("{".into()));
        held.try_release_all(|keys| sender.try_send(keys)).unwrap();
        assert_eq!(receiver.recv().unwrap(), vec!["界"]);
        assert_eq!(held.release("界"), None);
        held.try_release_all(|keys| sender.try_send(keys)).unwrap();
        assert!(receiver.recv().unwrap().is_empty());
        held.press("up");
        drop(receiver);
        assert!(held.try_release_all(|keys| sender.try_send(keys)).is_err());
        assert_eq!(held.release("up"), Some("up".into()));
    }
    #[test]
    fn aliases_preserve_identity_and_unicode() {
        for (base, shifted) in [("[","{"),("]","}"),("/","?"),("-","_"),("2","@"),("a","A"),("space"," ")] {
            assert_eq!(key(base),key(shifted));
        }
        assert_eq!(key("é"),Some(Key::Character('é')));
        assert_ne!(key("é"),key("界"));
        assert_eq!(key("f1"),Some(Key::Function(1)));
        assert_eq!(key("f24"),Some(Key::Function(24)));
        for invalid in ["", "unknown-key", "f0", "f25", "f256"] { assert_eq!(key(invalid),None); }
    }
}
