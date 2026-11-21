//! Explicit supported command subset; reject unsupported flags rather than dropping them.
use serde_json::{Map, Value};
use std::ffi::OsString;
use std::io::Read;

pub const HELP: &str = "Commands (JSON output except pipe-pane; explicit opaque IDs required):
  set-buffer [--name NAME] [TEXT|-] | list-buffers
  rename-buffer [--name NAME] NEW_NAME
  paste-buffer --surface ID [--name NAME] [--bracketed]
  notify --surface ID [--title TITLE] [--body BODY]
  list-notifications [--window ID] [--workspace ID]
  dismiss-notification --surface ID --revision NUMBER
  read-notification --surface ID --revision NUMBER
  close-workspace --workspace ID
  new-window [--activate]
  new-workspace --window ID [--name NAME] [--description TEXT] [--cwd PATH] [--ssh DESTINATION]
                [--focus true|false] [--activate]
  list-windows | list-workspaces | list-panes | list-panels
  select-workspace --workspace ID [--activate]
  rename-workspace --workspace ID NAME
  focus-panel --panel ID [--activate]
  focus-back --window ID [--activate]
  focus-forward --window ID [--activate]
  move-panel --panel ID --workspace ID [--activate]
  merge-workspace --workspace ID --target-workspace ID [--activate]
  move-workspace --workspace ID --window ID [--activate]
  equalize-panes --workspace ID
  swap-panes --pane ID --other-pane ID
  resize-pane --pane ID --direction left|right|up|down
  split-pane --pane ID --direction left|right|up|down [--ssh DESTINATION] [--cwd PATH] [--focus true|false] [--activate]
  set-pane-zoom --pane ID --zoomed true|false
  send-key --surface ID KEY  (Enter, Up, Ctrl-c, Alt-Left, Shift-Tab)
  send --surface ID [--paste] TEXT
  capture-pane --surface ID [--scrollback] [--lines N]
  read-screen --surface ID [--scrollback | --selection [--preserve-wraps]]
  pipe-pane --surface ID [--lines N | --selection [--preserve-wraps]] [--command SHELL_COMMAND | SHELL_COMMAND]
Use -- before positional text beginning with a dash. List commands list the
whole process unless narrowed with --window (workspaces/panes/panels) or
--workspace (panes/panels). No implicit target, ref/index lookup or --force yet.
";

pub fn help(command: &str) -> Result<String, String> {
    let (arguments, description) = match command {
        "pipe-pane" => ("--surface ID [--lines N | --selection [--preserve-wraps]] [--command SHELL_COMMAND | SHELL_COMMAND]", "Capture retained history (up to 1 MiB), optionally its last 1–65536 logical lines with --lines N or selected text with --selection, and feed it once to a local /bin/sh command via stdin. Truncated captures are rejected. Runs in the CLI working directory, forwards command stdout/stderr and returns its exit status. The CLI waits for the command; use normal shell interruption for long-running commands. Does not stream future output."),
        "paste-buffer" => ("--surface ID [--name NAME] [--bracketed]", "Queue a retained buffer to an explicit terminal without changing focus or scroll position. --bracketed uses terminal paste encoding. The buffer is retained; requests are never retried."),
        "rename-buffer" => ("[--name NAME] NEW_NAME", "Rename a retained buffer without copying its contents. Source defaults to default; an existing destination is never overwritten."),
        "set-buffer" => ("[--name NAME] [TEXT|-]", "Set a process-local copy buffer; name defaults to default. Omitted text or - reads UTF-8 stdin exactly, including trailing newlines. Storage and request size are bounded; buffers are not persisted or copied to the OS clipboard."),
        "list-buffers" => ("", "List process-local copy buffer names and byte counts without their contents."),
        "read-notification" => ("--surface ID --revision NUMBER", "Mark one message read without dismissing it or focusing its terminal. Missing or already read messages return marked_read:false."),
        "dismiss-notification" => ("--surface ID --revision NUMBER", "Dismiss one retained message using its surface ID and revision from list-notifications. A missing or already dismissed message returns dismissed:false."),
        "notify" => ("--surface ID [--title TITLE] [--body BODY]", "Add a notification to a live terminal's history."),
        "list-notifications" => ("[--window ID] [--workspace ID]", "List retained notifications, newest first within each terminal."),
        "close-workspace" => ("--workspace ID", "Close a workspace through the app's close policy. No force option is supported."),
        "new-window" => ("[--activate]", "Create a window. Activating it requires --activate."),
        "new-workspace" => ("--window ID [--name NAME] [--description TEXT] [--cwd PATH] [--ssh DESTINATION] [--focus true|false] [--activate]", "Create a workspace in an existing window. Selection and activation are opt-in."),
        "list-windows" => ("", "List windows in the connected app process."),
        "list-workspaces" => ("[--window ID]", "List workspaces, optionally limited to one window."),
        "list-panes" | "list-panels" => ("[--window ID] [--workspace ID]", "List panes or terminal surfaces, optionally limited to a window or workspace."),
        "select-workspace" => ("--workspace ID [--activate]", "Select a workspace; --activate also activates its window."),
        "rename-workspace" => ("--workspace ID NAME", "Set the workspace's manual name. Quote names containing spaces."),
        "focus-back" | "focus-forward" => ("--window ID [--activate]", "Navigate terminal focus history in one window. At the boundary this is a no-op. OS activation requires --activate."),
        "focus-panel" => ("--panel ID [--activate]", "Focus an existing terminal surface; --activate also activates its window."),
        "send-key" => ("--surface ID KEY", "Send one key tap, such as Enter, Up, Ctrl-c, Alt-Left or Shift-Tab."),
        "send" => ("--surface ID [--paste] TEXT", "Send literal text. --paste applies terminal paste encoding. Use -- before text starting with a dash."),
        "capture-pane" => ("--surface ID [--scrollback] [--lines N]", "Capture visible text, or retained history with --scrollback. --lines N implies history and requests the last 1–65536 logical text lines; unused trailing screen rows are omitted and soft wraps are joined. Returns JSON with text and truncation status, bounded to 1 MiB."),
        "read-screen" => ("--surface ID [--scrollback | --selection [--preserve-wraps]]", "Read visible text by default, retained history with --scrollback, or selected text with --selection; add --preserve-wraps to keep soft-wrap line breaks."),
        "move-panel" => ("--panel ID --workspace ID [--activate]", "Move a live terminal to another workspace, preserving its process. Destination selection changes; window activation requires --activate."),
        "merge-workspace" => ("--workspace ID --target-workspace ID [--activate]", "Merge into another workspace in any window of the connected app, retaining live panes and nested splits. Selects the destination and removes the source entry; keeps destination metadata. OS activation requires --activate."),
        "move-workspace" => ("--workspace ID --window ID [--activate]", "Move a workspace and its live terminals to another window. Window activation requires --activate."),
        "equalize-panes" => ("--workspace ID", "Balance every split in a workspace without changing focus. Rejects zoomed or busy layouts."),
        "swap-panes" => ("--pane ID --other-pane ID", "Exchange two panes in the same workspace without changing focus or restarting terminals."),
        "resize-pane" => ("--pane ID --direction left|right|up|down", "Move a shared divider one step without changing focus. Rejects zoomed or busy layouts."),
        "split-pane" => ("--pane ID --direction left|right|up|down [--ssh DESTINATION] [--cwd PATH] [--focus true|false] [--activate]", "Create a terminal beside an explicit pane. Inherits its directory and font; focus and OS activation are opt-in."),
        "set-pane-zoom" => ("--pane ID --zoomed true|false", "Set pane zoom explicitly without toggling. Zooming selects its terminal within the window, without OS window activation."),
        "completions" => ("bash|zsh|fish", "Print an offline shell completion script. No app connection is made."),
        _ => return Err(format!("no local help for {command}; use --help for supported commands or system.capabilities to discover RPC methods")),
    };
    let prefix = if command == "completions" { "rmuxctl" } else { "rmuxctl [--socket PATH]" };
    Ok(format!("Usage: {prefix} {command} {arguments}\n\n{description}\n\nTargets use explicit opaque IDs from list commands. RPC commands print JSON; pipe-pane forwards child output and status.\nExit 0: success; 1: RPC error; 2: usage/transport failure.\n"))
}

pub fn expand_with_input(command: &str, args: Vec<OsString>, mut input: impl std::io::Read) -> Result<(String, Value), String> {
    let (method, mut params) = expand(command, args)?;
    if command == "set-buffer" && params.get("text").is_none_or(|text| text.as_str() == Some("-")) {
        let mut bytes = Vec::new();
        input.by_ref().take(64 * 1024 + 1).read_to_end(&mut bytes).map_err(|error| format!("read buffer stdin: {error}"))?;
        if bytes.len() > 64 * 1024 { return Err("buffer stdin exceeds the 64 KiB request budget".into()); }
        let text = String::from_utf8(bytes).map_err(|_| "buffer stdin must be UTF-8")?;
        params["text"] = Value::String(text);
    }
    Ok((method, params))
}

pub fn expand(command: &str, args: Vec<OsString>) -> Result<(String, Value), String> {
    let (method, target, positional) = match command {
        "pipe-pane" => ("surface.read_text", Some(("--surface", "surface_id")), Some("command")),
        "paste-buffer" => ("buffer.paste", Some(("--surface", "surface_id")), None),
        "set-buffer" => ("buffer.set", None, Some("text")),
        "rename-buffer" => ("buffer.rename", None, Some("new_name")),
        "list-buffers" => ("buffer.list", None, None),
        "move-panel" => ("surface.move_to_workspace", Some(("--panel", "surface_id")), None),
        "merge-workspace" => ("workspace.merge", Some(("--workspace", "workspace_id")), None),
        "move-workspace" => ("workspace.move_to_window", Some(("--workspace", "workspace_id")), None),
        "equalize-panes" => ("workspace.equalize", Some(("--workspace", "workspace_id")), None),
        "swap-panes" => ("pane.swap", Some(("--pane", "pane_id")), None),
        "resize-pane" => ("pane.resize", Some(("--pane", "pane_id")), None),
        "split-pane" => ("pane.split", Some(("--pane", "pane_id")), None),
        "set-pane-zoom" => ("pane.zoom.set", Some(("--pane", "pane_id")), None),
        "list-notifications" => ("notification.list", None, None),
        "dismiss-notification" => ("notification.dismiss", Some(("--surface", "surface_id")), None),
        "read-notification" => ("notification.mark_read", Some(("--surface", "surface_id")), None),
        "notify" => ("notification.send", Some(("--surface", "surface_id")), None),
        "close-workspace" => (
            "workspace.close",
            Some(("--workspace", "workspace_id")),
            None,
        ),
        "new-window" => ("window.create", None, None),
        "new-workspace" => ("workspace.create", Some(("--window", "window_id")), None),
        "list-windows" => ("window.list", None, None),
        "list-workspaces" => ("workspace.list", None, None),
        "list-panes" => ("pane.list", None, None),
        "list-panels" => ("surface.list", None, None),
        "select-workspace" => (
            "workspace.select",
            Some(("--workspace", "workspace_id")),
            None,
        ),
        "rename-workspace" => (
            "workspace.rename",
            Some(("--workspace", "workspace_id")),
            Some("name"),
        ),
        "focus-back" | "focus-forward" => ("window.focus_history", Some(("--window", "window_id")), None),
        "focus-panel" => ("surface.focus", Some(("--panel", "surface_id")), None),
        "send-key" => (
            "surface.send_key",
            Some(("--surface", "surface_id")),
            Some("key"),
        ),
        "send" => (
            "surface.send_text",
            Some(("--surface", "surface_id")),
            Some("text"),
        ),
        "capture-pane" | "read-screen" => ("surface.read_text", Some(("--surface", "surface_id")), None),
        _ => {
            if args.len() > 1 {
                return Err("quote the JSON object as one argument".into());
            }
            let params = match args.first() {
                Some(value) => serde_json::from_str(value.to_str().ok_or("JSON must be UTF-8")?)
                    .map_err(|e| e.to_string())?,
                None => Value::Object(Map::new()),
            };
            if !params.is_object() {
                return Err("parameters must be a JSON object".into());
            }
            return Ok((command.into(), params));
        }
    };
    let mut params = Map::new();
    let mut literal = false;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        let arg = arg
            .into_string()
            .map_err(|_| "command arguments must be UTF-8")?;
        if !literal && arg == "--" {
            literal = true;
            continue;
        }
        if !literal && arg.starts_with('-') && !(command == "set-buffer" && arg == "-") {
            if matches!(command, "capture-pane" | "pipe-pane") && arg == "--lines" {
                let raw = args.next().ok_or("--lines requires a count")?.into_string().map_err(|_| "count must be UTF-8")?;
                let lines: usize = raw.parse().map_err(|_| "--lines requires an integer from 1 to 65536")?;
                if !raw.bytes().all(|byte| byte.is_ascii_digit()) || !(1..=65536).contains(&lines) { return Err("--lines requires an integer from 1 to 65536".into()); }
                if params.insert("lines".into(), Value::from(lines)).is_some() { return Err("duplicate --lines".into()); }
                continue;
            }
            if command == "pipe-pane" && arg == "--command" {
                let text = args.next().ok_or("--command requires shell command text")?.into_string().map_err(|_| "shell command must be UTF-8")?;
                if params.insert("command".into(), Value::String(text)).is_some() { return Err("duplicate pipe command".into()); }
                continue;
            }
            if matches!(command, "set-buffer" | "paste-buffer" | "rename-buffer") && arg == "--name" {
                let name = args.next().ok_or("--name requires text")?.into_string().map_err(|_| "name must be UTF-8")?;
                if params.insert("name".into(), Value::String(name)).is_some() { return Err("duplicate --name".into()); }
                continue;
            }
            if matches!(command, "dismiss-notification" | "read-notification") && arg == "--revision" {
                let raw = args.next().ok_or("--revision requires an unsigned integer")?.into_string().map_err(|_| "revision must be UTF-8")?;
                if raw.is_empty() || !raw.bytes().all(|byte| byte.is_ascii_digit()) { return Err("--revision expects an unsigned integer".into()); }
                let revision: u64 = raw.parse().map_err(|_| "revision exceeds unsigned 64-bit range")?;
                if params.insert("revision".into(), Value::from(revision)).is_some() { return Err("duplicate --revision".into()); }
                continue;
            }
            let destination = match (command, arg.as_str()) {
                ("move-panel", "--workspace") => Some("workspace_id"),
                ("merge-workspace", "--target-workspace") => Some("target_workspace_id"),
                ("move-workspace", "--window") => Some("window_id"),
                _ => None,
            };
            if let Some(key) = destination {
                let value = args.next().ok_or("destination requires an ID")?.into_string().map_err(|_| "ID must be UTF-8")?;
                if value.is_empty() || value.len() > 128 || value.starts_with('-') { return Err("invalid destination ID".into()); }
                if params.insert(key.into(), Value::String(value)).is_some() { return Err("duplicate destination".into()); }
                continue;
            }
            if command == "swap-panes" && arg == "--other-pane" {
                let value = args.next().ok_or("--other-pane requires an ID")?.into_string().map_err(|_| "pane ID must be UTF-8")?;
                if value.is_empty() { return Err("pane ID must not be empty".into()); }
                if params.insert("other_pane_id".into(), Value::String(value)).is_some() { return Err("duplicate --other-pane".into()); }
                continue;
            }
            if matches!(command, "split-pane" | "resize-pane") && arg == "--direction" {
                let value = args.next().ok_or("--direction requires a value")?.into_string().map_err(|_| "direction must be UTF-8")?;
                if !matches!(value.as_str(), "left" | "right" | "up" | "down") { return Err("invalid direction for this command".into()); }
                if params.insert("direction".into(), Value::String(value)).is_some() { return Err("duplicate --direction".into()); }
                continue;
            }
            if command == "set-pane-zoom" && arg == "--zoomed" {
                let value = match args.next().as_deref().and_then(|value| value.to_str()) {
                    Some("true") => true,
                    Some("false") => false,
                    _ => return Err("--zoomed expects true or false".into()),
                };
                if params.insert("zoomed".into(), Value::Bool(value)).is_some() { return Err("duplicate --zoomed".into()); }
                continue;
            }
            if command == "notify" && matches!(arg.as_str(), "--title" | "--body") {
                let value = args
                    .next()
                    .ok_or("notification option requires text")?
                    .into_string()
                    .map_err(|_| "text must be UTF-8")?;
                if params
                    .insert(arg.trim_start_matches("--").into(), Value::String(value))
                    .is_some()
                {
                    return Err("duplicate notification option".into());
                }
                continue;
            }
            if (command == "new-workspace" && matches!(arg.as_str(), "--name" | "--description" | "--cwd" | "--focus" | "--ssh"))
                || (command == "split-pane" && matches!(arg.as_str(), "--cwd" | "--focus" | "--ssh"))
            {
                let value = args
                    .next()
                    .ok_or("option requires a value")?
                    .into_string()
                    .map_err(|_| "value must be UTF-8")?;
                let key = if arg == "--ssh" { "ssh_destination" } else { arg.trim_start_matches("--") };
                let value = if key == "focus" {
                    Value::Bool(match value.as_str() {
                        "true" => true,
                        "false" => false,
                        _ => return Err("--focus expects true or false".into()),
                    })
                } else {
                    Value::String(value)
                };
                if params.insert(key.into(), value).is_some() {
                    return Err("duplicate option".into());
                }
                continue;
            }
            if matches!(
                command,
                "list-workspaces" | "list-panes" | "list-panels" | "list-notifications"
            ) && (arg == "--window" || (arg == "--workspace" && command != "list-workspaces"))
            {
                let key = if arg == "--window" {
                    "window_id"
                } else {
                    "workspace_id"
                };
                let value = args
                    .next()
                    .ok_or("filter requires an ID")?
                    .into_string()
                    .map_err(|_| "ID must be UTF-8")?;
                if value.is_empty() || value.len() > 128 || value.starts_with('-') {
                    return Err("invalid filter ID".into());
                }
                if params.insert(key.into(), Value::String(value)).is_some() {
                    return Err("duplicate filter".into());
                }
                continue;
            }
            if let Some((flag, key)) = target {
                if arg == flag {
                    let value = args
                        .next()
                        .ok_or_else(|| format!("{flag} requires an ID"))?
                        .into_string()
                        .map_err(|_| "ID must be UTF-8")?;
                    if value.is_empty() || value.len() > 128 || value.starts_with('-') {
                        return Err("invalid target ID".into());
                    }
                    if params.insert(key.into(), Value::String(value)).is_some() {
                        return Err(format!("duplicate {flag}"));
                    }
                    continue;
                }
            }
            let (key, value) = match (command, arg.as_str()) {
                ("paste-buffer", "--bracketed") => ("bracketed", Value::Bool(true)),
                ("send", "--paste") => ("paste", Value::Bool(true)),
                (
                    "select-workspace" | "focus-panel" | "focus-back" | "focus-forward" | "new-window" | "new-workspace" | "move-panel" | "move-workspace" | "merge-workspace" | "split-pane",
                    "--activate",
                ) => ("activate", Value::Bool(true)),
                ("read-screen" | "capture-pane", "--scrollback") => ("scope", Value::String("history".into())),
                ("read-screen" | "pipe-pane", "--selection") => ("scope", Value::String("selection".into())),
                ("read-screen" | "pipe-pane", "--preserve-wraps") => ("preserve_wraps", Value::Bool(true)),
                _ => return Err(format!("unsupported option for {command}: {arg}")),
            };
            if params.insert(key.into(), value).is_some() {
                return Err(format!("duplicate or conflicting option: {arg}"));
            }
        } else {
            let key = positional.ok_or("unexpected positional argument")?;
            if params.insert(key.into(), Value::String(arg)).is_some() {
                return Err("quote text as one argument".into());
            }
        }
    }
    if let Some((flag, key)) = target {
        if !params.contains_key(key) {
            return Err(format!("{command} requires {flag} ID"));
        }
    }
    if matches!(command, "focus-back" | "focus-forward") {
        params.insert("direction".into(), Value::String(if command == "focus-back" { "back" } else { "forward" }.into()));
    }
    if command == "capture-pane" {
        params.insert("max_bytes".into(), Value::from(1024 * 1024));
        if params.contains_key("lines") { params.insert("scope".into(), Value::String("history".into())); }
    }
    if matches!(command, "read-screen" | "pipe-pane") && params.contains_key("preserve_wraps")
        && params.get("scope").and_then(Value::as_str) != Some("selection") {
        return Err("--preserve-wraps requires --selection".into());
    }
    let required = match command {
        "dismiss-notification" | "read-notification" => Some(("revision", "--revision")),
        "split-pane" | "resize-pane" => Some(("direction", "--direction")),
        "swap-panes" => Some(("other_pane_id", "--other-pane")),
        "move-panel" => Some(("workspace_id", "--workspace")),
        "merge-workspace" => Some(("target_workspace_id", "--target-workspace")),
        "move-workspace" => Some(("window_id", "--window")),
        "set-pane-zoom" => Some(("zoomed", "--zoomed")),
        _ => None,
    };
    if let Some((key, flag)) = required {
        if !params.contains_key(key) { return Err(format!("{command} requires {flag}")); }
    }
    if let Some(key) = positional {
        if !params.contains_key(key) && command != "set-buffer" {
            return Err(format!("{command} requires {key}"));
        }
    }
    if command == "pipe-pane" {
        if params["command"].as_str().is_none_or(|command| command.trim().is_empty()) { return Err("pipe-pane requires nonempty shell command text".into()); }
        if params.get("scope").and_then(Value::as_str) == Some("selection") && params.contains_key("lines") {
            return Err("--lines cannot be combined with --selection".into());
        }
        params.entry("scope").or_insert_with(|| Value::String("history".into()));
        params.insert("max_bytes".into(), Value::from(1024 * 1024));
    }
    if command == "send-key" {
        let original = params.remove("key").unwrap().as_str().unwrap().to_owned();
        let mut key = original.as_str();
        while let Some((prefix, rest)) = key.split_once('-') {
            let field = match prefix.to_ascii_lowercase().as_str() {
                "ctrl" | "control" | "c" => "control",
                "alt" | "meta" | "m" => "alt",
                "shift" | "s" => "shift",
                _ => break,
            };
            if params.insert(field.into(), Value::Bool(true)).is_some() {
                return Err("duplicate key modifier".into());
            }
            key = rest;
        }
        if key.is_empty() {
            return Err("send-key requires a key after modifiers".into());
        }
        let canonical = if key.chars().count() == 1 {
            key.to_owned()
        } else {
            match key.to_ascii_lowercase().as_str() {
                "return" => "enter".into(),
                "esc" => "escape".into(),
                "bspace" => "backspace".into(),
                other => other.into(),
            }
        };
        params.insert("key".into(), Value::String(canonical));
    }
    Ok((method.into(), Value::Object(params)))
}
#[cfg(test)]
mod tests {
    #[test]
    fn focus_history_commands_require_a_window_and_preserve_activation() {
        for (command, direction) in [("focus-back", "back"), ("focus-forward", "forward")] {
            let (method, params) = super::expand(command, args(&["--window", "4294967297", "--activate"])).unwrap();
            assert_eq!(method, "window.focus_history");
            assert_eq!(params, serde_json::json!({"window_id":"4294967297","direction":direction,"activate":true}));
            assert!(super::expand(command, args(&[])).is_err());
            assert!(super::expand(command, args(&["--window", "4294967297", "--direction", "back"])).is_err());
            let (_, params) = super::expand(command, args(&["--window", "4294967297"])).unwrap();
            assert!(params.get("activate").is_none());
        }
    }
    use super::*;
    #[test]
    fn rename_buffer_requires_one_destination_and_preserves_unicode() {
        assert_eq!(expand("rename-buffer", args(&["--name", "old", "build 世界"])).unwrap(),
            ("buffer.rename".into(), serde_json::json!({"name":"old", "new_name":"build 世界"})));
        assert_eq!(expand("rename-buffer", args(&["new"])).unwrap().1, serde_json::json!({"new_name":"new"}));
        for input in [vec![], vec!["one", "two"], vec!["--name", "a", "--name", "b", "new"], vec!["--overwrite", "new"]] {
            assert!(expand("rename-buffer", args(&input)).is_err());
        }
    }
    fn args(v: &[&str]) -> Vec<OsString> {
        v.iter().map(OsString::from).collect()
    }
    #[test]
    fn swap_requires_two_explicit_panes() {
        assert_eq!(expand("swap-panes", args(&["--pane", "12", "--other-pane", "34"])).unwrap(),
            ("pane.swap".into(), serde_json::json!({"pane_id":"12", "other_pane_id":"34"})));
        for input in [vec![], vec!["--pane", "12"], vec!["--other-pane", "34"], vec!["--pane", "12", "--other-pane", ""],
            vec!["--pane", "12", "--other-pane", "34", "--activate"], vec!["--pane", "12", "--other-pane", "34", "--other-pane", "56"]] {
            assert!(expand("swap-panes", args(&input)).is_err());
        }
    }

    #[test]
    fn equalize_requires_workspace_and_rejects_unrelated_options() {
        assert_eq!(expand("equalize-panes", args(&["--workspace", "12"])).unwrap(),
            ("workspace.equalize".into(), serde_json::json!({"workspace_id":"12"})));
        for input in [vec![], vec!["--pane", "12"], vec!["--workspace", "12", "--direction", "left"], vec!["--workspace", "12", "--activate"]] {
            assert!(expand("equalize-panes", args(&input)).is_err());
        }
    }

    #[test]
    fn resize_pane_requires_explicit_target_and_cardinal_direction() {
        for direction in ["left", "right", "up", "down"] {
            assert_eq!(expand("resize-pane", args(&["--pane", "12", "--direction", direction])).unwrap(),
                ("pane.resize".into(), serde_json::json!({"pane_id":"12", "direction":direction})));
        }
        for input in [vec!["--pane", "12"], vec!["--direction", "left"], vec!["--pane", "12", "--direction", "diagonal"], vec!["--pane", "12", "--direction", "up", "--activate"]] {
            assert!(expand("resize-pane", args(&input)).is_err());
        }
    }

    #[test]
    fn merge_workspace_requires_two_explicit_targets() {
        assert_eq!(expand("merge-workspace", args(&["--workspace", "a", "--target-workspace", "b", "--activate"])).unwrap(),
            ("workspace.merge".into(), serde_json::json!({"workspace_id":"a","target_workspace_id":"b","activate":true})));
        for flags in [vec!["--workspace", "a"], vec!["--target-workspace", "b"], vec!["--workspace", "a", "--target-workspace", ""]] {
            assert!(expand("merge-workspace", args(&flags)).is_err());
        }
    }
    #[test]
    fn capture_pane_routes_bounded_tail_and_rejects_invalid_counts() {
        for flags in [
            vec!["--surface", "7", "--lines", "20"],
            vec!["--lines", "20", "--scrollback", "--surface", "7"],
            vec!["--scrollback", "--surface", "7", "--lines", "20"],
        ] {
            let (method, params) = expand("capture-pane", args(&flags)).unwrap();
            assert_eq!(method, "surface.read_text");
            assert_eq!(params, serde_json::json!({"surface_id":"7","scope":"history","lines":20,"max_bytes":1048576}));
        }
        let (_, params) = expand("capture-pane", args(&["--surface", "7"])).unwrap();
        assert!(params.get("scope").is_none());
        for count in ["0", "-1", "+1", "65537", "1.5", "abc", "999999999999999999999999"] {
            assert!(expand("capture-pane", args(&["--surface", "7", "--lines", count])).is_err());
        }
        assert!(expand("capture-pane", args(&["--lines", "2"])).is_err());
        assert!(expand("capture-pane", args(&["--surface", "7", "--lines", "2", "--lines", "3"])).is_err());
    }

    #[test]
    fn pipe_pane_selection_preserves_scope_and_rejects_history_options() {
        let (_, params) = expand("pipe-pane", args(&["--surface", "7", "--selection", "--preserve-wraps", "cat"])).unwrap();
        assert_eq!(params, serde_json::json!({"surface_id":"7", "scope":"selection", "preserve_wraps":true, "max_bytes":1048576, "command":"cat"}));
        for flags in [vec!["--selection", "--lines", "2"], vec!["--lines", "2", "--selection"], vec!["--selection", "--selection"], vec!["--preserve-wraps"]] {
            let mut input = vec!["--surface", "7"];
            input.extend(flags);
            input.push("cat");
            assert!(expand("pipe-pane", args(&input)).is_err());
        }
    }

    #[test]
    fn pipe_pane_can_limit_history_before_local_execution() {
        let (method, params) = expand("pipe-pane", args(&["--surface", "7", "--lines", "20", "--command", "wc -l"])).unwrap();
        assert_eq!(method, "surface.read_text");
        assert_eq!(params, serde_json::json!({"surface_id":"7", "scope":"history", "lines":20, "max_bytes":1048576, "command":"wc -l"}));
        for count in ["0", "65537", "-1", "+2", "many"] {
            assert!(expand("pipe-pane", args(&["--surface", "7", "--lines", count, "cat"])).is_err());
        }
        assert!(expand("pipe-pane", args(&["--surface", "7", "--lines", "2", "--lines", "3", "cat"])).is_err());
    }

    #[test]
    fn buffer_stdin_preserves_text_and_is_bounded_without_reading_for_literals() {
        for flags in [vec![], args(&["--name", "notes", "-"])] {
            let (_, params) = expand_with_input("set-buffer", flags, "first\n世界\n\n".as_bytes()).unwrap();
            assert_eq!(params["text"], "first\n世界\n\n");
        }
        let mut reader = std::io::Cursor::new(b"unused");
        let (_, params) = expand_with_input("set-buffer", args(&["literal"]), &mut reader).unwrap();
        assert_eq!(params["text"], "literal");
        assert_eq!(reader.position(), 0);
        assert!(expand_with_input("set-buffer", vec![], &[0xff][..]).is_err());
        assert!(expand_with_input("set-buffer", vec![], std::io::repeat(b'x')).is_err());
        assert_eq!(expand_with_input("set-buffer", vec![], std::io::empty()).unwrap().1["text"], "");
    }
    #[test]
    fn paste_buffer_requires_an_explicit_target_and_no_implicit_activation() {
        let (method, params) = expand("paste-buffer", args(&["--surface", "42", "--name", "notes", "--bracketed"])).unwrap();
        assert_eq!(method, "buffer.paste");
        assert_eq!(params, serde_json::json!({"surface_id":"42","name":"notes","bracketed":true}));
        assert!(expand("paste-buffer", vec![]).is_err());
        assert!(expand("paste-buffer", args(&["--surface", "42", "--activate"])).is_err());
        assert!(expand("paste-buffer", args(&["--surface", "42", "--bracketed", "--bracketed"])).is_err());
    }
    #[test]
    fn copy_buffer_commands_preserve_text_and_reject_extra_arguments() {
        let (method, params) = expand("set-buffer", args(&["--name", "build", "--", "-literal 世界\n"])).unwrap();
        assert_eq!(method, "buffer.set");
        assert_eq!(params, serde_json::json!({"name":"build", "text":"-literal 世界\n"}));
        assert_eq!(expand("list-buffers", vec![]).unwrap().0, "buffer.list");
        assert!(expand("list-buffers", args(&["--name", "x"])).is_err());
        assert!(expand("set-buffer", args(&["--name", "x"])).unwrap().1.get("text").is_none());
        assert!(expand("set-buffer", args(&["--name", "x", "--name", "y", "text"])).is_err());
    }
    #[test]
    fn preserve_wraps_requires_selection_and_rejects_duplicates() {
        let (_, params) = expand("read-screen", args(&["--surface", "7", "--preserve-wraps", "--selection"])).unwrap();
        assert_eq!(params["preserve_wraps"], true);
        assert_eq!(params["scope"], "selection");
        for flags in [
            vec!["--surface", "7", "--preserve-wraps"],
            vec!["--surface", "7", "--scrollback", "--preserve-wraps"],
            vec!["--surface", "7", "--selection", "--preserve-wraps", "--preserve-wraps"],
        ] { assert!(expand("read-screen", args(&flags)).is_err()); }
    }
    #[test]
    fn explicit_targets_and_literal_text() {
        let (method, params) =
            expand("send", args(&["--surface", "42", "--", "--世界\n"])).unwrap();
        assert_eq!(method, "surface.send_text");
        assert_eq!(params["text"], "--世界\n");
        assert_eq!(params["surface_id"], "42");
        assert_eq!(
            expand(
                "send",
                args(&["--surface", "42", "--paste", "multiline\ntext"])
            )
            .unwrap()
            .1["paste"],
            true
        );
        assert_eq!(
            expand("read-screen", args(&["--selection", "--surface", "7"]))
                .unwrap()
                .1["scope"],
            "selection"
        );
        assert!(
            expand("select-workspace", args(&["--workspace", "3"]))
                .unwrap()
                .1
                .get("activate")
                .is_none()
        );
    }
    #[test]
    fn symbolic_key_modifiers_and_aliases() {
        let (method, params) =
            expand("send-key", args(&["--surface", "42", "Ctrl-Shift-Tab"])).unwrap();
        assert_eq!(method, "surface.send_key");
        assert_eq!(params["key"], "tab");
        assert_eq!(params["control"], true);
        assert_eq!(params["shift"], true);
        assert_eq!(
            expand("send-key", args(&["--surface", "42", "Return"]))
                .unwrap()
                .1["key"],
            "enter"
        );
        assert_eq!(
            expand("send-key", args(&["--surface", "42", "Ctrl--"]))
                .unwrap()
                .1["key"],
            "-"
        );
        assert!(expand("send-key", args(&["--surface", "42", "Ctrl-Ctrl-c"])).is_err());
    }
    #[test]
    fn dismissal_preserves_full_revision_precision() {
        assert_eq!(expand("read-notification", args(&["--surface", "42", "--revision", "9"])).unwrap(),
            ("notification.mark_read".into(), serde_json::json!({"surface_id":"42","revision":9})));
        assert!(expand("read-notification", args(&["--surface", "42"])).is_err());
        assert_eq!(expand("dismiss-notification", args(&["--surface", "42", "--revision", "18446744073709551615"])).unwrap(),
            ("notification.dismiss".into(), serde_json::json!({"surface_id":"42","revision":u64::MAX})));
        for value in ["-1", "+1", "1.0", "1e3", "", "18446744073709551616"] {
            assert!(expand("dismiss-notification", args(&["--surface", "42", "--revision", value])).is_err());
        }
        assert!(expand("dismiss-notification", args(&["--surface", "42"])).is_err());
        assert!(expand("dismiss-notification", args(&["--surface", "42", "--revision", "1", "--revision", "2"])).is_err());
    }

    #[test]
    fn split_command_keeps_target_direction_and_focus_explicit() {
        assert_eq!(expand("split-pane", args(&["--pane", "12", "--direction", "right", "--ssh", "user@host"])).unwrap(),
            ("pane.split".into(), serde_json::json!({"pane_id":"12", "direction":"right", "ssh_destination":"user@host"})));

        for direction in ["left", "right", "up", "down"] {
            assert_eq!(expand("split-pane", args(&["--pane", "12", "--direction", direction])).unwrap(),
                ("pane.split".into(), serde_json::json!({"pane_id":"12", "direction":direction})));
        }
        assert_eq!(expand("split-pane", args(&["--pane", "12", "--direction", "down", "--cwd", "/work/project", "--focus", "false"])).unwrap(),
            ("pane.split".into(), serde_json::json!({"pane_id":"12","direction":"down","cwd":"/work/project","focus":false})));
        for input in [
            vec!["--pane", "12"],
            vec!["--direction", "right"],
            vec!["--pane", "12", "--direction", "diagonal"],
            vec!["--pane", "12", "--direction", "down", "--direction", "right"],
            vec!["--pane", "12", "--direction", "down", "--name", "unsupported"],
        ] { assert!(expand("split-pane", args(&input)).is_err()); }
    }

    #[test]
    fn layout_commands_require_destinations_and_preserve_boolean_types() {
        assert_eq!(expand("move-panel", args(&["--panel", "42", "--workspace", "7"])).unwrap(),
            ("surface.move_to_workspace".into(), serde_json::json!({"surface_id":"42","workspace_id":"7"})));
        assert_eq!(expand("move-workspace", args(&["--workspace", "7", "--window", "9", "--activate"])).unwrap(),
            ("workspace.move_to_window".into(), serde_json::json!({"workspace_id":"7","window_id":"9","activate":true})));
        assert_eq!(expand("set-pane-zoom", args(&["--pane", "12", "--zoomed", "false"])).unwrap(),
            ("pane.zoom.set".into(), serde_json::json!({"pane_id":"12","zoomed":false})));
        for (command, input) in [
            ("move-panel", vec!["--panel", "42"]),
            ("move-workspace", vec!["--workspace", "7", "--window", ""]),
            ("move-panel", vec!["--panel", "42", "--workspace", "7", "--workspace", "8"]),
            ("set-pane-zoom", vec!["--pane", "12", "--zoomed", "yes"]),
            ("set-pane-zoom", vec!["--pane", "12"]),
            ("set-pane-zoom", vec!["--pane", "12", "--zoomed", "true", "--activate"]),
        ] { assert!(expand(command, args(&input)).is_err()); }
    }

    #[test]
    fn creation_keeps_selection_opt_in() {
        let (method, params) = expand("new-workspace", args(&["--window", "42", "--ssh", "ssh://user@[2001:db8::1]:2222"])).unwrap();
        assert_eq!(method, "workspace.create");
        assert_eq!(params["ssh_destination"], "ssh://user@[2001:db8::1]:2222");
        assert!(params.get("focus").is_none() && params.get("activate").is_none());
        assert!(expand("new-workspace", args(&["--window", "42", "--ssh", "one", "--ssh", "two"])).is_err());
        assert!(expand("new-workspace", args(&["--window", "42", "--ssh"])).is_err());
        let (method, params) = expand(
            "new-workspace",
            args(&["--window", "42", "--name", "世界", "--focus", "false"]),
        )
        .unwrap();
        assert_eq!(method, "workspace.create");
        assert_eq!(params["focus"], false);
        assert_eq!(params["name"], "世界");
        assert!(params.get("activate").is_none());
        assert_eq!(
            expand("new-window", args(&["--activate"])).unwrap().1["activate"],
            true
        );
        assert!(
            expand(
                "new-workspace",
                args(&["--window", "42", "--focus", "maybe"])
            )
            .is_err()
        );
        assert!(expand("new-workspace", args(&["--name", "Missing window"])).is_err());
    }

    #[test]
    fn unsupported_or_ambiguous_options_never_silently_succeed() {
        for (cmd, options) in [
            (
                "read-screen",
                vec!["--surface", "7", "--selection", "--scrollback"],
            ),
            ("send", vec!["--surface", "7", "--force", "hello"]),
            ("select-workspace", vec![]),
            ("list-workspaces", vec!["--workspace", "4"]),
            (
                "rename-workspace",
                vec!["--workspace", "3", "--workspace", "4", "name"],
            ),
        ] {
            assert!(expand(cmd, args(&options)).is_err());
        }
    }
}
