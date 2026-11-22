//! One bounded text snapshot sent to an explicitly supplied local shell command.
// Command output is inherited, so the CLI does not accumulate stdout/stderr.
pub fn run(command: &str, text: &str) -> Result<i32, String> {
    if text.len() > 1024 * 1024 { return Err("pipe-pane snapshot exceeds 1 MiB".into()); }
    if command.trim().is_empty() { return Err("pipe-pane requires a shell command".into()); }
    run_platform(command, text)
}
#[cfg(unix)]
fn run_platform(command: &str, text: &str) -> Result<i32, String> {
    use std::{io::{Write, ErrorKind}, process::{Command, Stdio}, os::unix::process::ExitStatusExt};
    let mut child = Command::new("/bin/sh").args(["-c", command])
        .stdin(Stdio::piped()).stdout(Stdio::inherit()).stderr(Stdio::inherit())
        .spawn().map_err(|error| format!("start pipe-pane command: {error}"))?;
    let written = child.stdin.take().expect("piped stdin").write_all(text.as_bytes());
    // Early-consuming commands (e.g. head) may legitimately close stdin.
    if let Err(error) = written {
        if error.kind() != ErrorKind::BrokenPipe {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("write pipe-pane input: {error}"));
        }
    }
    match child.wait() {
        Ok(status) => Ok(status.code().unwrap_or_else(|| 128 + status.signal().unwrap_or(1))),
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            Err(format!("wait for pipe-pane command: {error}"))
        }
    }
}
#[cfg(not(unix))]
fn run_platform(_: &str, _: &str) -> Result<i32, String> {
    Err("pipe-pane local shell execution is not available on this platform".into())
}
#[cfg(all(test, unix))]
mod tests {
    #[test]
    fn json_pipe_drains_both_streams_and_bounds_retained_output() {
        let (code, output) = super::run_json("cat; printf 'problem' >&2; exit 7", "literal $(exit 9) 世界\n").unwrap();
        assert_eq!(code, 7);
        assert_eq!(output["exit_code"], 7);
        assert_eq!(output["stdout"], "literal $(exit 9) 世界\n");
        assert_eq!(output["stderr"], "problem");
        assert_eq!(output["stdout_truncated"], false);
        let (code, output) = super::run_json("dd if=/dev/zero bs=1048576 count=2 2>/dev/null; dd if=/dev/zero bs=1048576 count=2 >&2; cat >/dev/null", &"x".repeat(1024*1024)).unwrap();
        assert_eq!(code, 0);
        assert_eq!(output["stdout"].as_str().unwrap().len(), 1024*1024);
        assert_eq!(output["stderr"].as_str().unwrap().len(), 1024*1024);
        assert_eq!(output["stdout_truncated"], true);
        assert_eq!(output["stderr_truncated"], true);
    }
    #[test]
    fn pipe_snapshot_stays_stdin_and_preserves_child_exit_status() {
        assert_eq!(super::run("IFS= read -r line; test \"$line\" = 'literal $(exit 7)'", "literal $(exit 7)\n").unwrap(), 0);
        assert_eq!(super::run("cat >/dev/null; exit 3", "世界\nnext\n").unwrap(), 3);
        assert_eq!(super::run("exit 0", &"x".repeat(1024 * 1024)).unwrap(), 0);
        assert!(super::run("exit 0", &"x".repeat(1024 * 1024 + 1)).is_err());
    }
}

/// JSON captures retain at most 1 MiB per stream while continuing to drain both.
#[cfg(unix)]
pub fn run_json(command: &str, text: &str) -> Result<(i32, serde_json::Value), String> {
    use std::{io::{Read, Write, ErrorKind}, process::{Command, Stdio}, os::unix::process::ExitStatusExt};
    const LIMIT: usize = 1024 * 1024;
    if text.len() > LIMIT { return Err("pipe-pane snapshot exceeds 1 MiB".into()); }
    if command.trim().is_empty() { return Err("pipe-pane requires a shell command".into()); }
    fn drain(mut stream: impl Read) -> std::io::Result<(Vec<u8>, bool)> {
        let mut retained = Vec::new();
        let mut truncated = false;
        let mut chunk = [0; 8192];
        loop {
            let size = match stream.read(&mut chunk) {
                Err(error) if error.kind() == ErrorKind::Interrupted => continue,
                result => result?,
            };
            if size == 0 { return Ok((retained, truncated)); }
            let keep = size.min(LIMIT - retained.len());
            retained.extend_from_slice(&chunk[..keep]);
            truncated |= keep < size;
        }
    }
    let mut child = Command::new("/bin/sh").args(["-c", command])
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())
        .spawn().map_err(|error| format!("start pipe-pane command: {error}"))?;
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    std::thread::scope(|scope| {
        let out = scope.spawn(move || drain(stdout));
        let err = scope.spawn(move || drain(stderr));
        if let Err(error) = child.stdin.take().unwrap().write_all(text.as_bytes()) {
            if error.kind() != ErrorKind::BrokenPipe {
                let _ = child.kill(); let _ = child.wait();
                return Err(format!("write pipe-pane input: {error}"));
            }
        }
        let status = child.wait().map_err(|error| {
            let _ = child.kill(); let _ = child.wait();
            format!("wait for pipe-pane command: {error}")
        })?;
        let (stdout, stdout_truncated) = out.join().map_err(|_| "stdout reader failed")?.map_err(|error| format!("read pipe-pane stdout: {error}"))?;
        let (stderr, stderr_truncated) = err.join().map_err(|_| "stderr reader failed")?.map_err(|error| format!("read pipe-pane stderr: {error}"))?;
        let code = status.code().unwrap_or_else(|| 128 + status.signal().unwrap_or(1));
        Ok((code, serde_json::json!({"exit_code":code,
            "stdout":String::from_utf8_lossy(&stdout), "stderr":String::from_utf8_lossy(&stderr),
            "stdout_truncated":stdout_truncated, "stderr_truncated":stderr_truncated})))
    })
}
#[cfg(not(unix))]
pub fn run_json(_: &str, _: &str) -> Result<(i32, serde_json::Value), String> {
    Err("pipe-pane local shell execution is not available on this platform".into())
}
