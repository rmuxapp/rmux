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
    let named = key_tokens(label).into_iter().map(|word| match word.as_str() {
        "cmd" => "cmd command".into(), "ctrl" => "ctrl control".into(), "alt" => "alt option".into(),
        "super" => "super win".into(), "enter" => "enter return".into(), "esc" => "esc escape".into(),
        _ => word,
    }).collect::<Vec<String>>().join(" ");
    format!("{label} {words} {aliases} {named}").to_lowercase()
}

/// Modifier words must come from a real binding, not fuzzy command-title
/// letters. A key-only query such as "ctrl tab" must fit one alias.
pub(crate) struct ShortcutQuery { modifiers: Vec<String>, keys: Option<Vec<String>> }
fn key_tokens(text: &str) -> Vec<String> {
    let mut expanded = String::new();
    for ch in text.chars() {
        expanded.push_str(match ch {
            '⌘' => " cmd ", '⌃' => " ctrl ", '⌥' => " alt ", '⇧' => " shift ",
            '⇥' => " tab ", '↩' => " enter ", '⌫' => " backspace ", '⌦' => " delete ",
            '↑' => " up ", '↓' => " down ", '←' => " left ", '→' => " right ",
            _ => { expanded.push(ch); continue; }
        });
    }
    expanded.to_lowercase().replace("pgup", "page up").replace("pgdn", "page down")
        .split(|ch: char| ch.is_whitespace() || ch == '+' || ch == '/')
        .filter(|word| !word.is_empty()).map(|word| match word {
            "command" => "cmd", "control" => "ctrl", "option" => "alt", "win" => "super",
            "return" => "enter", "escape" => "esc", other => other,
        }.to_owned()).collect()
}
impl ShortcutQuery {
    pub fn new(query: &str) -> Self {
        let words = key_tokens(query);
        let is_modifier = |word: &str| matches!(word, "cmd" | "ctrl" | "alt" | "shift" | "super" | "fn");
        let is_key = |word: &str| word.chars().count() == 1 || matches!(word,
            "tab" | "space" | "enter" | "esc" | "backspace" | "delete" | "insert" | "home" | "end" | "page" | "up" | "down" | "left" | "right")
            || word.strip_prefix('f').and_then(|number| number.parse::<u8>().ok()).is_some_and(|number| (1..=24).contains(&number));
        let mut modifiers: Vec<String> = words.iter().filter(|word| is_modifier(word)).cloned().collect();
        modifiers.sort(); modifiers.dedup();
        if query.trim().eq_ignore_ascii_case("command") { modifiers.clear(); }
        let keys = words.iter().filter(|word| !is_modifier(word)).all(|word| is_key(word))
            .then(|| words.into_iter().filter(|word| !is_modifier(word)).collect());
        Self { modifiers, keys }
    }
    pub fn matches(&self, label: &str) -> bool {
        if self.modifiers.is_empty() { return true; }
        label.split(" / ").any(|alias| {
            let words = key_tokens(alias);
            self.modifiers.iter().all(|word| words.contains(word))
                && self.keys.as_ref().is_none_or(|keys| keys.iter().all(|word| words.contains(word)))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{format_key_parts, search_terms, join_aliases, ShortcutQuery};
    use gpui::Modifiers;
    #[test]
    fn key_queries_do_not_match_title_letters_or_combine_separate_aliases() {
        let query = ShortcutQuery::new("ctrl tab");
        assert!(query.matches("⌘] / ⌃⇥"));
        assert!(!query.matches("⌘W"));
        assert!(!query.matches("⌃⇧PgUp"));
        assert!(!query.matches("⌃K / ⌘⇥"));
        assert!(ShortcutQuery::new("ctrl move tab").matches("⌃⇧PgUp"));
        assert!(ShortcutQuery::new("next tab").matches("⌘}"));
        assert!(ShortcutQuery::new("command").matches("⌃K"));
        assert!(ShortcutQuery::new("ctrl shift").matches("Ctrl+Shift+Tab"));
        assert!(ShortcutQuery::new("control tab").matches("Ctrl+Tab"));
        assert!(ShortcutQuery::new("⌘⇥").matches("⌘⇥"));
        assert!(!ShortcutQuery::new("⌘⇥").matches("⌘W"));
    }
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
