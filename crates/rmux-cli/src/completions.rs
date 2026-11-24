//! Offline shell completion: never opens a socket or inspects terminal sessions.
use std::ffi::OsString;

const COMMANDS: &[(&str, &[&str])] = &[
    ("read-events", &["--follow", "--surface", "--workspace", "--kind", "--after", "--limit", "--wait-ms"]),
    ("set-buffer", &["--name"]),
    ("rename-buffer", &["--name"]),
    ("list-buffers", &[]),
    ("paste-buffer", &["--surface", "--name", "--bracketed"]),
    ("dismiss-notification", &["--surface", "--revision"]),
    ("read-notification", &["--surface", "--revision"]),
    ("notify", &["--surface", "--title", "--body"]),
    ("list-notifications", &["--window", "--workspace"]),
    ("close-workspace", &["--workspace"]),
    ("new-window", &["--activate"]),
    ("open-workspace", &["--name", "--window", "--cwd", "--activate"]),
    ("new-workspace", &["--window", "--name", "--description", "--cwd", "--focus", "--activate"]),
    ("list-windows", &[]),
    ("list-workspaces", &["--window"]),
    ("list-panes", &["--window", "--workspace"]),
    ("list-panels", &["--window", "--workspace"]),
    ("select-workspace", &["--workspace", "--activate"]),
    ("rename-workspace", &["--workspace"]),
    ("focus-panel", &["--panel", "--activate"]),
    ("focus-back", &["--window", "--activate"]),
    ("focus-forward", &["--window", "--activate"]),
    ("send-key", &["--surface"]),
    ("send", &["--surface", "--paste"]),
    ("pipe-pane", &["--json", "--surface", "--command", "--lines", "--selection", "--preserve-wraps"]),
    ("capture-pane", &["--surface", "--scrollback", "--lines"]),
    ("read-screen", &["--surface", "--scrollback", "--selection", "--preserve-wraps"]),
    ("move-panel", &["--panel", "--workspace", "--activate"]),
    ("merge-workspace", &["--workspace", "--target-workspace", "--activate"]),
    ("move-workspace", &["--workspace", "--window", "--activate"]),
    ("equalize-panes", &["--workspace"]),
    ("swap-panes", &["--pane", "--other-pane"]),
    ("resize-pane", &["--pane", "--direction"]),
    ("split-pane", &["--pane", "--direction", "--ssh", "--cwd", "--focus", "--activate"]),
    ("set-pane-zoom", &["--pane", "--zoomed"]),
    ("completions", &[]),
    ("claude", &[]),
];

pub fn script(shell: &str) -> Result<&'static str, String> {
    match shell {
        "bash" => Ok(r#"_rmuxctl_complete() {
    local candidate
    COMPREPLY=()
    while IFS= read -r candidate; do
        COMPREPLY+=("$candidate")
    done < <(command rmuxctl __complete "${COMP_WORDS[@]:1:COMP_CWORD}")
}
complete -F _rmuxctl_complete rmuxctl
"#),
        "zsh" => Ok(r#"#compdef rmuxctl
_rmuxctl() {
    local -a candidates
    candidates=("${(@f)$(command rmuxctl __complete "${(@)words[2,CURRENT]}")}")
    (( ${#candidates} )) && compadd -- "${candidates[@]}"
}
compdef _rmuxctl rmuxctl
"#),
        "fish" => Ok(r#"function __rmuxctl_complete
    set -l words (commandline -opc)
    set -e words[1]
    set -l current (commandline -ct)
    command rmuxctl __complete $words "$current"
end
complete -c rmuxctl -f -a '(__rmuxctl_complete)'
"#),
        _ => Err("completions expects bash, zsh, or fish".into()),
    }
}

/// Arguments include the current (possibly empty) word as their final entry.
pub fn candidates(args: &[OsString]) -> Vec<&'static str> {
    let Some((current, previous)) = args.split_last() else { return Vec::new(); };
    let Some(current) = current.to_str() else { return Vec::new(); };
    let mut previous = previous.iter();
    let mut command = previous.next();
    let explicit_socket = command.is_some_and(|word| word == "--socket");
    if explicit_socket {
        if previous.next().is_none() { return Vec::new(); }
        command = previous.next();
    }
    let options = match command {
        None => return COMMANDS.iter().map(|(name, _)| *name)
            .filter(|name| !explicit_socket || *name != "completions")
            .chain(["help", "--help"])
            .chain(["--socket", "--version"].into_iter().filter(|_| !explicit_socket))
            .filter(|name| name.starts_with(current)).collect(),
        Some(command) if command == "help" => {
            if previous.next().is_some() { return Vec::new(); }
            return COMMANDS.iter().map(|(name, _)| *name).filter(|name| name.starts_with(current)).collect();
        }
        Some(command) if command == "completions" => {
            if previous.next().is_some() { return Vec::new(); }
            return ["bash", "zsh", "fish", "--help"].into_iter().filter(|name| name.starts_with(current)).collect();
        }
        Some(command) => match COMMANDS.iter().find(|(name, _)| command == *name) {
            Some((_, options)) => *options,
            None => return Vec::new(), // Raw RPC parameters are user-owned JSON.
        },
    };
    let initial = previous.len() == 0;
    let mut used = Vec::new();
    let mut used_kinds = Vec::new();
    let mut value_for = None;
    for word in previous {
        let Some(word) = word.to_str() else { return Vec::new(); };
        if let Some(flag) = value_for.take() {
            if flag == "--kind" { used_kinds.push(word); }
            continue;
        }
        if word == "--" { return Vec::new(); }
        if options.contains(&word) {
            used.push(word);
            if !matches!(word, "--activate" | "--paste" | "--scrollback" | "--selection"
                | "--follow" | "--json" | "--preserve-wraps" | "--bracketed") {
                value_for = Some(word);
            }
        }
    }
    if let Some(flag) = value_for {
        return if matches!(flag, "--focus" | "--zoomed") {
            ["true", "false"].into_iter().filter(|value| value.starts_with(current)).collect()
        } else if flag == "--direction" {
            ["left", "right", "up", "down"].into_iter().filter(|value| value.starts_with(current)).collect()
        } else if flag == "--kind" {
            rmux_protocol::EVENT_KINDS.iter().copied()
                .filter(|kind| kind.starts_with(current) && !used_kinds.contains(kind)).collect()
        } else { Vec::new() };
    }
    options.iter().copied().chain(["--help"].into_iter().filter(|_| initial)).filter(|flag| flag.starts_with(current) && (*flag == "--kind" || !used.contains(flag))
        && !(matches!(*flag, "--scrollback" | "--selection") && used.iter().any(|used| matches!(*used, "--scrollback" | "--selection"))))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn complete(args: &[&str]) -> Vec<&'static str> {
        candidates(&args.iter().map(OsString::from).collect::<Vec<_>>())
    }
    #[test]
    fn switches_do_not_consume_values_and_event_kinds_remain_repeatable() {
        for (command, switch, next) in [
            ("read-events", "--follow", "--workspace"),
            ("pipe-pane", "--json", "--command"),
            ("read-screen", "--preserve-wraps", "--selection"),
            ("paste-buffer", "--bracketed", "--name"),
        ] {
            let candidates = complete(&[command, switch, ""]);
            assert!(candidates.contains(&next), "{command} {switch}");
            assert!(!candidates.contains(&switch));
        }
        assert!(complete(&["read-events", "--follow", "--workspace", "42", ""]).contains(&"--kind"));
        assert!(!complete(&["read-events", "--follow", "--workspace", "42", ""]).contains(&"--workspace"));
        assert!(complete(&["read-events", "--kind", "workspace.created", "--k"]).contains(&"--kind"));
        assert_eq!(complete(&["read-events", "--kind", "workspace.c"]), ["workspace.created", "workspace.closed"]);
        assert_eq!(complete(&["read-events", "--kind", "workspace.created", "--kind", "workspace.c"]), ["workspace.closed"]);
        assert!(complete(&["read-events", "--kind", "unknown."]).is_empty());
    }
    #[test]
    fn completion_respects_command_values_and_literal_text() {
        assert!(complete(&["new-"]).contains(&"new-workspace"));
        assert!(complete(&["--socket", "/tmp/rmux socket", "send", "--"]).contains(&"--paste"));
        assert!(complete(&["--socket", ""]).is_empty());
        assert!(!complete(&["--socket", "/tmp/rpc", ""]).contains(&"--socket"));
        assert!(!complete(&["--socket", "/tmp/rpc", ""]).contains(&"completions"));
        assert_eq!(complete(&["completions", "z"]), ["zsh"]);
        assert_eq!(complete(&["help", "send-k"]), ["send-key"]);
        assert!(complete(&["send", "--h"]).contains(&"--help"));
        assert_eq!(complete(&["new-workspace", "--focus", "f"]), ["false"]);
        assert!(complete(&["send", "--surface", ""]).is_empty());
        assert!(complete(&["send", "--", "--"]).is_empty());
        assert!(!complete(&["read-screen", "--selection", ""]).contains(&"--scrollback"));
        assert!(!complete(&["send", "--surface", "id", ""]).contains(&"--surface"));
        assert!(complete(&["workspace.list", ""]).is_empty());
        assert!(complete(&["notify", "--body", "--surface", ""]).contains(&"--surface"));
    }
}
