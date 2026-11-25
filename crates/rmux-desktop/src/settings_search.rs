//! Local sparse-vector retrieval. No model downloads, network requests, or per-frame indexing.
use std::{collections::BTreeMap, sync::OnceLock};
pub struct Entry { pub key: &'static str, pub config: &'static str, pub label: &'static str, pub description: &'static str, pub aliases: &'static str, pub category: usize, pub json_only: bool }
pub static ENTRIES: &[Entry] = &[
    Entry { key: "font", config: "font_size", label: "Default text size", description: "Size of terminal text. Per-terminal zoom is preserved.", aliases: "bigger smaller readable text letters zoom", category: 0, json_only: false },
    Entry { key: "padding", config: "terminal_padding", label: "Terminal padding", description: "Space between terminal text and the pane edges.", aliases: "spacing margin inset border", category: 0, json_only: false },
    Entry { key: "theme", config: "terminal_theme", label: "App & terminal theme", description: "Choose the colors for the app and terminal.", aliases: "dark light appearance colour scheme solarized nord catppuccin", category: 0, json_only: false },
    Entry { key: "focused_cursor", config: "focused_cursor", label: "Focused cursor", description: "Shape of the cursor in the active terminal.", aliases: "caret block beam underline", category: 1, json_only: false },
    Entry { key: "cursor_opacity", config: "cursor_opacity", label: "Cursor opacity", description: "Visibility of the terminal cursor.", aliases: "transparent caret", category: 1, json_only: false },
    Entry { key: "cursor", config: "unfocused_cursor", label: "Unfocused cursor", description: "Show a hollow cursor or hide it in inactive terminals.", aliases: "inactive caret hidden outline", category: 1, json_only: false },
    Entry { key: "copy", config: "copy_on_select", label: "Copy on select", description: "Copy selected text to the clipboard automatically.", aliases: "highlight selection clipboard mouse", category: 1, json_only: false },
    Entry { key: "close", config: "confirm_close", label: "Confirm before closing", description: "Choose when to ask before closing terminals.", aliases: "stop close warnings confirmation prompt active tasks", category: 2, json_only: false },
    Entry { key: "group_placement", config: "group_new_workspace_placement", label: "New group workspace", description: "Where new workspaces appear within their group.", aliases: "position ordering first last after current", category: 2, json_only: false },
    Entry { key: "scrollback", config: "scrollback_mib", label: "New terminal scrollback", description: "Output history budget for new terminals, in MiB.", aliases: "memory ram buffer lines scroll previous output limit", category: 1, json_only: false },
    Entry { key: "native_notifications", config: "native_notifications", label: "Desktop notifications", description: "Allow system banners for terminal notifications.", aliases: "alerts notification popups operating system", category: 3, json_only: false },
    Entry { key: "notification_focus", config: "notification_focus", label: "Show desktop banners", description: "Choose when terminal notifications appear.", aliases: "alerts notification focus background inactive", category: 3, json_only: false },
    Entry { key: "font_family", config: "font_family", label: "Default font family", description: "Typeface for terminals using the default font.", aliases: "monospace typography menlo typeface", category: 0, json_only: false },
    Entry { key: "opacity", config: "background_opacity", label: "Terminal background opacity", description: "Make the terminal background more or less transparent.", aliases: "transparency see through window", category: 0, json_only: false },
    Entry { key: "blur", config: "background_blur", label: "Blur behind terminal", description: "Soften the background behind transparent terminals.", aliases: "frosted glass transparency", category: 0, json_only: false },
    Entry { key: "workspace_placement", config: "new_workspace_placement", label: "New workspace position", description: "Where new workspaces appear in the sidebar.", aliases: "ordering first last after current", category: 2, json_only: false },
    #[cfg(target_os = "macos")]
    Entry { key: "option_as_alt", config: "macos_option_as_alt", label: "Option key behavior", description: "Use Option as terminal Alt or for native text input.", aliases: "keyboard mac accents dead keys special characters", category: 1, json_only: false },
    Entry { key: "selection_opacity", config: "selection_opacity", label: "Selection opacity", description: "Visibility of the highlight behind selected text.", aliases: "highlight transparency", category: 0, json_only: false },
    Entry { key: "import_theme", config: "theme_file", label: "Import terminal theme", description: "Load colors from an external theme file.", aliases: "ghostty import colour scheme", category: 0, json_only: false },
    Entry { key: "file_editor", config: "file_editor", label: "External editor", description: "Configure the program used to open files.", aliases: "program args vscode code sublime textedit", category: 4, json_only: false },
    Entry { key: "editor_example", config: "file_editor", label: "VS Code configuration", description: "Copy an example external editor configuration.", aliases: "vscode visual studio code example", category: 4, json_only: false },
    Entry { key: "editor_fallback", config: "file_editor", label: "Repair editor configuration", description: "Open Settings JSON using the system editor.", aliases: "fallback broken editor textedit", category: 4, json_only: false },
    Entry { key: "shell_history", config: "shell_history_directory", label: "Saved command history", description: "Recall shell commands after restoring a saved session.", aliases: "bash zsh persistence history experimental restart", category: 4, json_only: false },
    Entry { key: "cursor_blink", config: "cursor_blink", label: "Blink cursor", description: "Animate the cursor in the focused terminal.", aliases: "flashing blinking caret", category: 1, json_only: false },
    Entry { key: "controls_on_hover", config: "controls_on_hover", label: "Show controls on hover", description: "Reveal pane controls when the pointer is over them.", aliases: "mouse toolbar buttons clean", category: 2, json_only: false },
    Entry { key: "hide_top_bar", config: "hide_top_bar", label: "Hide top workspace bar", description: "Give terminals more space by hiding the top bar.", aliases: "toolbar header compact", category: 2, json_only: false },
    Entry { key: "compact_sidebar", config: "compact_sidebar", label: "Compact sidebar", description: "Use tighter spacing in the workspace sidebar.", aliases: "dense small navigation", category: 2, json_only: false },
    Entry { key: "commands", config: "commands", label: "Custom commands", description: "Define named commands, arguments, working directories, and environment variables.", aliases: "automation scripts argv cwd env", category: 4, json_only: true },
    Entry { key: "layout_presets", config: "layout_presets", label: "Layout presets", description: "Configure reusable workspace layouts and session presets.", aliases: "template arrangement saved splits", category: 2, json_only: true },
    Entry { key: "keybindings", config: "keybindings", label: "Keyboard shortcuts", description: "Customize key combinations and the actions they trigger.", aliases: "hotkeys bindings shortcuts remap keys", category: 4, json_only: true },
    Entry { key: "restore_session", config: "restore_session", label: "Restore session on launch", description: "Restore your saved workspace when rmux starts.", aliases: "reopen startup remember windows tabs resume", category: 2, json_only: true },
    Entry { key: "session_file", config: "session_file", label: "Session file", description: "Choose where the saved workspace session is stored.", aliases: "persistence restore path backup", category: 4, json_only: true },
    Entry { key: "socket_directory", config: "socket_directory", label: "Automation socket directory", description: "Choose the directory for the local RPC socket.", aliases: "api ipc remote control integration automation", category: 4, json_only: true },
    Entry { key: "foreground_color", config: "foreground_color", label: "Terminal text color", description: "Override the theme’s default terminal text color.", aliases: "foreground colour letters", category: 0, json_only: true },
    Entry { key: "background_color", config: "background_color", label: "Terminal background color", description: "Override the theme’s default terminal background color.", aliases: "background colour", category: 0, json_only: true },
    Entry { key: "cursor_color", config: "cursor_color", label: "Cursor color", description: "Override the terminal cursor color.", aliases: "caret colour", category: 1, json_only: true },
    Entry { key: "cursor_text", config: "cursor_text", label: "Cursor text color", description: "Choose the text color inside a block cursor.", aliases: "caret foreground colour", category: 1, json_only: true },
    Entry { key: "selection_background", config: "selection_background", label: "Selection background color", description: "Choose the background color for selected text.", aliases: "highlight colour", category: 0, json_only: true },
    Entry { key: "selection_foreground", config: "selection_foreground", label: "Selection text color", description: "Choose the text color inside a selection.", aliases: "highlight foreground colour", category: 0, json_only: true },
    Entry { key: "ansi_palette", config: "ansi_palette", label: "ANSI palette", description: "Override the terminal’s sixteen standard colors.", aliases: "ansi colour palette red green blue", category: 0, json_only: true },
];
pub fn entry(key: &str) -> &'static Entry { ENTRIES.iter().find(|entry| entry.key == key).expect("registered setting") }

fn tokens(text: &str) -> Vec<String> {
    text.to_lowercase().split(|c: char| !c.is_alphanumeric()).filter_map(|word| {
        let word = match word {
            "" => return None,
            "the" | "a" | "an" | "to" | "of" | "my" | "how" | "do" | "i" | "and" | "or" | "is" | "it" | "for" | "in" | "on" | "off" | "stop" | "enable" | "disable" | "change" | "make" => return None,
            "colours" | "colors" | "colour" => "color",
            "notifications" | "notification" | "alerts" | "alert" | "banners" | "banner" | "popups" => "notification",
            "transparent" | "transparency" | "translucent" => "opacity",
            "bigger" | "larger" | "smaller" => "size",
            "typeface" | "typography" | "fonts" => "font",
            "caret" => "cursor",
            "warnings" | "warning" | "confirmation" => "confirm",
            "closing" => "close",
            "blinking" | "flashing" => "blink",
            "workspaces" => "workspace",
            "shortcuts" | "hotkeys" => "shortcut",
            "reopen" | "resume" => "restore",
            "startup" => "launch",
            "ram" => "memory",
            _ => word,
        };
        Some(word.to_owned())
    }).take(64).collect()
}
fn one_typo(a: &str, b: &str) -> bool {
    if a.len().abs_diff(b.len()) > 1 || a.len() < 4 || !a.is_ascii() || !b.is_ascii() { return false; }
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let (mut i, mut j, mut edits) = (0, 0, 0);
    while i < a.len() && j < b.len() {
        if a[i] == b[j] { i += 1; j += 1; }
        else {
            edits += 1;
            if edits > 1 { return false; }
            if a.len() == b.len() && i + 1 < a.len() && a[i] == b[j + 1] && a[i + 1] == b[j] { i += 2; j += 2; continue; }
            if a.len() >= b.len() { i += 1; }
            if b.len() >= a.len() { j += 1; }
        }
    }
    edits + (a.len() - i) + (b.len() - j) <= 1
}
struct Index { documents: Vec<BTreeMap<String, f32>>, idf: BTreeMap<String, f32> }
fn index() -> &'static Index {
    static INDEX: OnceLock<Index> = OnceLock::new();
    INDEX.get_or_init(|| {
        let mut documents = Vec::new();
        let mut frequencies = BTreeMap::<String, usize>::new();
        for entry in ENTRIES {
            let mut vector = BTreeMap::<String, f32>::new();
            for (text, weight) in [(entry.label, 3.), (entry.config, 3.), (entry.description, 1.), (entry.aliases, 1.5)] {
                for term in tokens(text) { *vector.entry(term).or_default() += weight; }
            }
            for term in vector.keys() { *frequencies.entry(term.clone()).or_default() += 1; }
            documents.push(vector);
        }
        let idf: BTreeMap<_, _> = frequencies.into_iter().map(|(term, count)| (term, (1. + ENTRIES.len() as f32 / count as f32).ln())).collect();
        for vector in &mut documents {
            for (term, value) in vector.iter_mut() { *value = (1. + *value).ln() * idf[term]; }
            let norm = vector.values().map(|v| v * v).sum::<f32>().sqrt();
            for value in vector.values_mut() { *value /= norm; }
        }
        Index { documents, idf }
    })
}
/// Ranked TF-IDF cosine similarity with concept aliases, prefix and one-edit typo matching.
/// This is a small local lexical/concept index, not neural embedding inference.
pub fn search(query: &str) -> Vec<&'static str> {
    let terms = tokens(query);
    if terms.is_empty() { return Vec::new(); }
    let index = index();
    let expanded: Vec<Vec<(&str, f32)>> = terms.iter().map(|term| {
        index.idf.keys().filter_map(|candidate| {
            let weight = if candidate == term { 1. } else if term.len() >= 2 && candidate.starts_with(term) { 0.75 }
                else if one_typo(term, candidate) || term.strip_suffix('s').is_some_and(|term| one_typo(term, candidate)) { 0.65 } else { return None; };
            Some((candidate.as_str(), weight))
        }).collect()
    }).collect();
    let mut query_vector = BTreeMap::<&str, f32>::new();
    for terms in &expanded { for (term, weight) in terms { *query_vector.entry(term).or_default() += weight * index.idf[*term]; } }
    let norm = query_vector.values().map(|v| v * v).sum::<f32>().sqrt();
    if norm == 0. { return Vec::new(); }
    let mut results: Vec<_> = index.documents.iter().enumerate().filter_map(|(i, document)| {
        let coverage = expanded.iter().filter(|terms| terms.iter().any(|(term, _)| document.contains_key(*term))).count();
        if coverage * 2 < terms.len() || coverage == 0 { return None; }
        let cosine: f32 = query_vector.iter().map(|(term, value)| document.get(*term).copied().unwrap_or(0.) * value / norm).sum();
        let exact = ENTRIES[i].config.eq_ignore_ascii_case(query.trim());
        Some((i, coverage, cosine + coverage as f32 / terms.len() as f32 + if exact { 2. } else { 0. }))
    }).collect();
    // When every query concept can be matched, omit loose one-word matches.
    // Fall back to partial matches only when the query has no complete match.
    if results.iter().any(|(_, coverage, _)| *coverage == terms.len()) {
        results.retain(|(_, coverage, _)| *coverage == terms.len());
    }
    results.sort_by(|(a, _, x), (b, _, y)| y.total_cmp(x).then(a.cmp(b)));
    results.into_iter().map(|(i, _, _)| ENTRIES[i].key).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn finds_intent_typos_and_exact_config_keys() {
        for (query, expected) in [("text size", "font"), ("transparent background", "opacity"), ("stop close warnings", "close"), ("notificaton", "native_notifications"), ("notificaitons", "native_notifications"), ("background opactiy", "opacity"), ("restore session", "restore_session"), ("socket_directory", "socket_directory"), ("font_family", "font_family")] {
            assert!(search(query).iter().take(2).any(|key| *key == expected), "{query}: {:?}", search(query));
        }
        assert!(!search("transparent background").contains(&"cursor_opacity"));
        assert_eq!(search("socket_directory"), vec!["socket_directory"]);
        assert!(search("xyzzynothing").is_empty());
        assert!(search("  ").is_empty());
    }
    #[test]
    fn catalog_covers_all_serialized_preferences_and_controls() {
        let value = serde_json::to_value(crate::Preferences::default()).unwrap();
        for key in value.as_object().unwrap().keys() {
            if key == "macos_option_as_alt" && !cfg!(target_os = "macos") { continue; }
            assert!(ENTRIES.iter().any(|entry| entry.config == key), "missing {key}");
        }
        for key in crate::settings_view::KEYS { assert_eq!(ENTRIES.iter().filter(|entry| entry.key == *key).count(), 1); }
        for entry in ENTRIES { assert!(crate::settings_view::CATEGORIES[entry.category].2.contains(&entry.key)); }
    }
}
