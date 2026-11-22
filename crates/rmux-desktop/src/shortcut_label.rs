//! Shared display labels for resolved key bindings.
use gpui::Modifiers;

pub(crate) fn binding_hint(action: &dyn gpui::Action, focus: &gpui::FocusHandle, window: &gpui::Window, mac: bool) -> Option<String> {
    window.highest_precedence_binding_for_action_in(action, focus).map(|binding|
        binding.keystrokes().iter().map(|key| format_key_parts(key.key(), *key.modifiers(), mac))
            .collect::<Vec<_>>().join(" "))
}

/// All effective aliases, highest precedence first. Shadowed/disabled bindings
/// are filtered by GPUI; equivalent display spellings appear only once.
pub(crate) fn binding_aliases(action: &dyn gpui::Action, focus: &gpui::FocusHandle, window: &gpui::Window, mac: bool) -> Option<String> {
    let labels = window.bindings_for_action_in(action, focus).into_iter().rev().map(|binding|
        binding.keystrokes().iter().map(|key| format_key_parts(key.key(), *key.modifiers(), mac))
            .collect::<Vec<_>>().join(" "));
    join_aliases(labels)
}
fn join_aliases(labels: impl IntoIterator<Item = String>) -> Option<String> {
    let mut unique = Vec::new();
    for label in labels {
        if !unique.contains(&label) { unique.push(label); }
    }
    (!unique.is_empty()).then(|| unique.join(" / "))
}

pub(crate) fn format_key_parts(key: &str, modifiers: Modifiers, mac: bool) -> String {
    let mut parts = Vec::with_capacity(6);
    if modifiers.function { parts.push("Fn".to_owned()); }
    if modifiers.control { parts.push(if mac { "⌃" } else { "Ctrl" }.to_owned()); }
    if modifiers.alt { parts.push(if mac { "⌥" } else { "Alt" }.to_owned()); }
    if modifiers.shift { parts.push(if mac { "⇧" } else { "Shift" }.to_owned()); }
    if modifiers.platform { parts.push(if mac { "⌘" } else if cfg!(target_os = "windows") { "Win" } else { "Super" }.to_owned()); }
    // GPUI/browser bindings may carry the shifted character as well as Shift.
    // Display the underlying key once, rather than e.g. Shift+Command+}.
    let key = if modifiers.shift {
        match key {
            "{" => "[", "}" => "]",
            "!" => "1", "@" => "2", "#" => "3", "$" => "4", "%" => "5",
            "^" => "6", "&" => "7", "*" => "8", "(" => "9", ")" => "0",
            other => other,
        }
    } else { key };
    parts.push(match key {
        "up" => "↑".into(), "down" => "↓".into(), "left" => "←".into(), "right" => "→".into(),
        "enter" => if mac { "↩" } else { "Enter" }.into(),
        "backspace" => if mac { "⌫" } else { "Backspace" }.into(),
        "delete" => if mac { "⌦" } else { "Delete" }.into(),
        "escape" => "Esc".into(), "tab" => if mac { "⇥" } else { "Tab" }.into(),
        "space" => "Space".into(), "pageup" => "PgUp".into(), "pagedown" => "PgDn".into(),
        "home" => "Home".into(), "end" => "End".into(), "insert" => "Insert".into(),
        other => other.to_uppercase(),
    });
    parts.join(if mac { "" } else { "+" })
}

pub(crate) fn search_terms(label: &str) -> String {
    let mut words = String::new();
    for ch in label.chars() {
        words.push_str(match ch {
            '⌘' => " cmd command ", '⌃' => " ctrl control ",
            '⌥' => " alt option ", '⇧' => " shift ",
            '↑' => " up ", '↓' => " down ", '←' => " left ", '→' => " right ",
            '↩' => " enter return ", '⌫' => " backspace ", '⌦' => " delete ",
            '⇥' => " tab ", '+' => " ",
            _ => { words.push(ch); continue; }
        });
    }
    let aliases = label.to_lowercase()
        .replace("pgup", "page up").replace("pgdn", "page down")
        .replace("esc", "escape");
    format!("{label} {words} {aliases}").to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::{format_key_parts, search_terms, join_aliases};
    use gpui::Modifiers;
    #[test]
    fn aliases_preserve_precedence_and_deduplicate_display_spellings() {
        assert_eq!(join_aliases(["⌘]", "⌘⌥→", "⌘]"].map(str::to_owned)), Some("⌘] / ⌘⌥→".into()));
        assert_eq!(join_aliases([]), None);
        let words = search_terms(&join_aliases(["⌘]", "⌘⌥→"].map(str::to_owned)).unwrap());
        assert!(words.contains("right") && words.contains("option"));
    }
    #[test]
    fn shifted_aliases_display_the_base_key_once() {
        let mac = Modifiers { platform: true, shift: true, ..Default::default() };
        assert_eq!(format_key_parts("}", mac, true), "⇧⌘]");
        assert_eq!(format_key_parts("{", mac, true), "⇧⌘[");
        assert_eq!(format_key_parts("]", mac, true), "⇧⌘]");
        assert_eq!(format_key_parts("!", mac, true), "⇧⌘1");
        let other = Modifiers { control: true, shift: true, ..Default::default() };
        assert_eq!(format_key_parts("@", other, false), "Ctrl+Shift+2");
        let literal = Modifiers { platform: true, ..Default::default() };
        assert_eq!(format_key_parts("}", literal, true), "⌘}");
        assert_eq!(format_key_parts("@", literal, true), "⌘@");
    }
    #[test]
    fn named_navigation_keys_have_searchable_long_names() {
        assert!(search_terms("Ctrl+PgUp").contains("page up"));
        assert!(search_terms("Ctrl+PgDn").contains("page down"));
        assert!(search_terms("Esc").contains("escape"));
        assert!(search_terms("⇧⌘]").contains("command"));
    }
}
