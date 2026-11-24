//! Session-local Claude hooks. No changes to the user's Claude settings.
use serde_json::{Value, json};
use std::{ffi::OsString, io::{Read, Write}, path::Path};

const EVENTS: &[&str] = &["SessionStart", "UserPromptSubmit", "PreToolUse", "PostToolUse", "PermissionRequest", "Notification", "Stop", "StopFailure", "SessionEnd"];

fn sequence(event: &Value) -> Option<String> {
    let name = event["hook_event_name"].as_str()?;
    let (state, kind, notice) = match name {
        "SessionStart" => ("idle", "", None),
        "UserPromptSubmit" | "PreToolUse" | "PostToolUse" => ("working", "", None),
        "PermissionRequest" => ("blocked", ":kind=permission", Some("Approval needed")),
        "Notification" => match event["notification_type"].as_str() {
            Some("permission_prompt") => ("blocked", ":kind=permission", None),
            Some("idle_prompt") => ("idle", "", Some("Waiting for input")),
            _ => return None,
        },
        "Stop" => ("done", "", Some("Response ready")),
        "StopFailure" => ("error", "", Some("Claude encountered an error")),
        "SessionEnd" => ("clear", "", None),
        _ => return None,
    };
    // Use a fixed identity per terminal and fixed labels: untrusted prompts and
    // tool output never become terminal controls or notification text.
    let mut output = format!("\x1b]7501;state={state}:id=rmux-claude:app=Claude{kind}\x07");
    if let Some(notice) = notice { output.push_str(&format!("\x1b]777;notify;Claude;{notice}\x07")); }
    Some(output)
}

fn settings(executable: &Path) -> Result<Value, String> {
    let executable = executable.to_str().ok_or("rmuxctl path must be UTF-8")?;
    // Claude command hooks use a shell. Quote the resolved binary path as data.
    let command = format!("'{}' claude-hook", executable.replace('\\', "/").replace('\'', "'\\''"));
    let hooks: serde_json::Map<String, Value> = EVENTS.iter().map(|event| ((*event).into(), json!([
        {"hooks":[{"type":"command", "command":command, "timeout":5}]}
    ]))).collect();
    Ok(json!({"hooks":hooks}))
}

pub fn hook() -> Result<i32, String> {
    let Some(directory) = std::env::var_os("RMUX_CLAUDE_EVENTS") else { return Ok(0); };
    let mut input = Vec::new();
    std::io::stdin().take(1024 * 1024 + 1).read_to_end(&mut input).map_err(|e| e.to_string())?;
    if input.len() > 1024 * 1024 { return Ok(0); }
    let Ok(event) = serde_json::from_slice::<Value>(&input) else { return Ok(0); };
    let Some(output) = sequence(&event) else { return Ok(0); };
    // The parent owns a private temporary directory. Atomic publication keeps
    // concurrent hooks from exposing partial records. No RPC socket is needed.
    let mut file = tempfile::NamedTempFile::new_in(directory).map_err(|e| e.to_string())?;
    file.write_all(output.as_bytes()).map_err(|e| e.to_string())?;
    let ready = file.path().with_extension("ready");
    file.persist(ready).map_err(|e| e.to_string())?;
    Ok(0)
}

fn drain(directory: &Path) -> Result<(), String> {
    let mut files = std::fs::read_dir(directory).map_err(|e| e.to_string())?
        .filter_map(Result::ok).filter(|entry| entry.path().extension().is_some_and(|ext| ext == "ready"))
        .collect::<Vec<_>>();
    files.sort_by_key(|entry| entry.metadata().and_then(|m| m.modified()).ok());
    let mut stdout = std::io::stdout().lock();
    for entry in files {
        let bytes = std::fs::read(entry.path()).map_err(|e| e.to_string())?;
        stdout.write_all(&bytes).map_err(|e| e.to_string())?;
        std::fs::remove_file(entry.path()).map_err(|e| e.to_string())?;
    }
    stdout.flush().map_err(|e| e.to_string())
}

// Catch (rather than ignore) terminal interrupts in the supervisor. exec resets
// caught handlers in Claude, which still receives the foreground-group signal.
#[cfg(unix)]
struct InterruptGuard(Vec<(i32, libc::sigaction)>);
#[cfg(unix)]
impl InterruptGuard {
    fn new() -> Result<Self, String> {
        extern "C" fn caught(_: i32) {}
        let mut guard = Self(Vec::new());
        for signal in [libc::SIGINT, libc::SIGQUIT] {
            // SAFETY: initialized sigaction structures and a static signal-safe handler.
            unsafe {
                let mut action: libc::sigaction = std::mem::zeroed();
                let mut previous: libc::sigaction = std::mem::zeroed();
                action.sa_sigaction = caught as *const () as usize;
                libc::sigemptyset(&mut action.sa_mask);
                if libc::sigaction(signal, &action, &mut previous) != 0 { return Err(std::io::Error::last_os_error().to_string()); }
                guard.0.push((signal, previous));
            }
        }
        Ok(guard)
    }
}
#[cfg(unix)]
impl Drop for InterruptGuard {
    fn drop(&mut self) {
        for (signal, action) in &self.0 {
            // SAFETY: restore the disposition saved by this guard.
            unsafe { libc::sigaction(*signal, action, std::ptr::null_mut()); }
        }
    }
}

#[cfg(windows)]
struct InterruptGuard;
#[cfg(windows)]
#[link(name = "Kernel32")]
unsafe extern "system" {
    fn SetConsoleCtrlHandler(handler: Option<unsafe extern "system" fn(u32) -> i32>, add: i32) -> i32;
}
#[cfg(windows)]
unsafe extern "system" fn console_interrupt(event: u32) -> i32 { i32::from(event == 0 || event == 1) }
#[cfg(windows)]
impl InterruptGuard {
    fn new() -> Result<Self, String> {
        // SAFETY: static callback; child processes do not inherit handler functions.
        if unsafe { SetConsoleCtrlHandler(Some(console_interrupt), 1) } == 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        Ok(Self)
    }
}
#[cfg(windows)]
impl Drop for InterruptGuard {
    fn drop(&mut self) {
        // SAFETY: unregister the same static callback.
        unsafe { SetConsoleCtrlHandler(Some(console_interrupt), 0); }
    }
}

pub fn launch(args: &[OsString]) -> Result<i32, String> {
    use std::io::IsTerminal;
    let mut command = std::process::Command::new("claude");
    // Machine-readable output and caller-supplied session settings pass through.
    let enabled = std::env::var("TERM_PROGRAM").as_deref() == Ok("rmux")
        && std::env::var("RMUX_CLAUDE_INTEGRATION").as_deref() != Ok("0")
        && std::io::stdout().is_terminal()
        && !args.iter().any(|arg| matches!(arg.to_str(), Some("-p" | "--print" | "--settings" | "--help" | "-h" | "--version" | "-v")) || arg.to_str().is_some_and(|arg| arg.starts_with("--settings=")));
    if !enabled {
        command.args(args);
        #[cfg(unix)]
        { use std::os::unix::process::CommandExt; return Err(command.exec().to_string()); }
        #[cfg(not(unix))]
        { return command.status().map(|s| s.code().unwrap_or(1)).map_err(|e| e.to_string()); }
    }
    let directory = tempfile::Builder::new().prefix("rmux-claude-").tempdir().map_err(|e| e.to_string())?;
    let settings = settings(&std::env::current_exe().map_err(|e| e.to_string())?)?;
    #[cfg(any(unix, windows))]
    let _interrupts = InterruptGuard::new()?;
    let mut child = command.arg("--settings").arg(settings.to_string()).args(args)
        .env("RMUX_CLAUDE_EVENTS", directory.path()).spawn().map_err(|e| e.to_string())?;
    let result = loop {
        if let Err(error) = drain(directory.path()) { break Err(error); }
        match child.try_wait() {
            Ok(Some(status)) => { let _ = drain(directory.path()); break Ok(status.code().unwrap_or(1)); }
            Ok(None) => std::thread::sleep(std::time::Duration::from_millis(100)),
            Err(error) => break Err(error.to_string()),
        }
    };
    if result.is_err() { let _ = child.kill(); let _ = child.wait(); }
    let _ = std::io::stdout().write_all(b"\x1b]7501;state=clear:id=rmux-claude\x07");
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lifecycle_and_untrusted_content() {
        for (event, state) in [("SessionStart", "idle"), ("UserPromptSubmit", "working"), ("PreToolUse", "working"), ("PostToolUse", "working"), ("PermissionRequest", "blocked"), ("Stop", "done"), ("StopFailure", "error"), ("SessionEnd", "clear")] {
            let output = sequence(&json!({"hook_event_name": event, "message":"\u{1b}]52;c;secret\u{7}"})).unwrap();
            assert!(output.contains(&format!("state={state}")));
            assert!(!output.contains("secret"));
        }
        assert!(sequence(&json!({"hook_event_name":"SubagentStop"})).is_none());
        assert!(sequence(&json!({"hook_event_name":"Notification","notification_type":"idle_prompt"})).unwrap().contains("Waiting for input"));
    }
    #[test]
    fn settings_preserve_shell_quoting() {
        let settings = settings(Path::new("/a b/it's/rmuxctl")).unwrap();
        assert_eq!(settings["hooks"]["Stop"][0]["hooks"][0]["command"], "'/a b/it'\\''s/rmuxctl' claude-hook");
        assert_eq!(settings["hooks"].as_object().unwrap().len(), EVENTS.len());
    }
    #[test]
    fn published_events_are_complete_and_removed() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("partial"), "partial").unwrap();
        std::fs::write(directory.path().join("event.ready"), "").unwrap();
        drain(directory.path()).unwrap();
        assert!(directory.path().join("partial").exists());
        assert!(!directory.path().join("event.ready").exists());
    }
}
