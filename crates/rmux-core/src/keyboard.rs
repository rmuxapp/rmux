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
