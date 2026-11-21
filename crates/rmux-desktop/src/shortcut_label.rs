//! Shared display labels for resolved key bindings.
use gpui::Modifiers;

pub(crate) fn format_key_parts(key: &str, modifiers: Modifiers, mac: bool) -> String {
    let mut parts = Vec::with_capacity(6);
    if modifiers.function { parts.push("Fn".to_owned()); }
    if modifiers.control { parts.push(if mac { "⌃" } else { "Ctrl" }.to_owned()); }
    if modifiers.alt { parts.push(if mac { "⌥" } else { "Alt" }.to_owned()); }
    if modifiers.shift { parts.push(if mac { "⇧" } else { "Shift" }.to_owned()); }
    if modifiers.platform { parts.push(if mac { "⌘" } else if cfg!(target_os = "windows") { "Win" } else { "Super" }.to_owned()); }
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
