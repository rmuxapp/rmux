//! Explicit GitHub CLI lookup, never called by rendering or Git watchers.
use anyhow::{Context, Result, ensure};
use std::{io::{Read, Seek}, path::Path, process::{Child, Command, Stdio}, sync::atomic::{AtomicBool, Ordering}, time::{Duration, Instant}};
static ACTIVE: AtomicBool = AtomicBool::new(false);
struct Permit;
impl Drop for Permit { fn drop(&mut self) { ACTIVE.store(false, Ordering::Release); } }
struct Running(Child);
impl Drop for Running { fn drop(&mut self) { let _ = self.0.kill(); let _ = self.0.wait(); } }
#[derive(Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PullRequest { pub number: u64, pub title: String, pub state: String, pub url: String, pub is_draft: bool }
impl PullRequest {
    pub fn label(&self) -> String {
        format!("PR #{} · {} · {}", self.number, if self.is_draft && self.state == "OPEN" { "Draft" } else { self.state.as_str() }, self.title)
    }
}
fn parse(bytes: &[u8]) -> Result<Option<PullRequest>> {
    ensure!(bytes.len() <= 65536, "Pull request response exceeded 64 KiB");
    let mut rows: Vec<PullRequest> = serde_json::from_slice(bytes).context("Invalid GitHub CLI response")?;
    ensure!(rows.len() <= 1, "Unexpected pull request count");
    let Some(mut row) = rows.pop() else { return Ok(None); };
    ensure!(row.number > 0 && matches!(row.state.as_str(), "OPEN" | "CLOSED" | "MERGED"), "Invalid pull request state");
    let url = url::Url::parse(&row.url)?;
    ensure!(url.scheme() == "https" && url.host_str().is_some() && url.username().is_empty() && url.password().is_none(), "Invalid pull request URL");
    row.title = row.title.chars().filter(|c| !c.is_control()).take(512).collect();
    Ok(Some(row))
}
pub async fn lookup(directory: &Path, branch: &str) -> Result<Option<PullRequest>> {
    ensure!(ACTIVE.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire).is_ok(), "Another pull request lookup is running");
    let _permit = Permit;
    let git_dir = crate::git_context::git_directory(directory).context("Local Git repository is no longer available")?;
    ensure!(crate::git_context::branch_at(&git_dir).as_deref() == Some(branch), "Git branch changed; refresh and retry");
    let mut command = Command::new("gh");
    command.args(["pr", "list", "--state", "all", "--limit", "1", "--json", "number,title,state,url,isDraft"])
        .arg(format!("--head={branch}")).current_dir(directory)
        .env("GH_PROMPT_DISABLED", "1").env("GH_PAGER", "cat")
        .stdin(Stdio::null()).stderr(Stdio::null());
    #[cfg(target_os = "windows")]
    { use std::os::windows::process::CommandExt; command.creation_flags(0x08000000); }
    let bytes = capture(command, Duration::from_secs(10)).await?;
    ensure!(crate::git_context::branch_at(&git_dir).as_deref() == Some(branch), "Git branch changed during lookup; refresh and retry");
    parse(&bytes)
}
async fn capture(mut command: Command, timeout: Duration) -> Result<Vec<u8>> {
    let mut output = tempfile::tempfile()?;
    command.stdout(output.try_clone()?);
    let mut running = Running(command.spawn().context("Could not launch gh; install it with mise and sign in with gh auth login")?);
    let started = Instant::now();
    loop {
        ensure!(output.metadata()?.len() <= 65536, "Pull request response exceeded 64 KiB");
        if let Some(status) = running.0.try_wait()? {
            ensure!(status.success(), "GitHub lookup failed; check gh authentication, repository access and network connectivity");
            break;
        }
        ensure!(started.elapsed() < timeout, "Pull request lookup timed out");
        gpui::Timer::after(Duration::from_millis(50)).await;
    }
    output.rewind()?;
    let mut bytes = Vec::new();
    output.take(65537).read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= 65536, "Pull request response exceeded 64 KiB");
    Ok(bytes)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    #[test]
    fn command_capture_handles_success_failure_and_timeout() {
        let run = |script: &str, timeout| {
            let mut command = Command::new("/bin/sh");
            command.args(["-c", script]).stdin(Stdio::null()).stderr(Stdio::null());
            futures_lite::future::block_on(capture(command, timeout))
        };
        assert_eq!(run("printf '[]'", Duration::from_secs(1)).unwrap(), b"[]");
        assert!(run("exit 7", Duration::from_secs(1)).is_err());
        let start = Instant::now();
        assert!(run("exec sleep 5", Duration::from_millis(50)).unwrap_err().to_string().contains("timed out"));
        assert!(start.elapsed() < Duration::from_secs(2));
    }
    #[test]
    fn validates_pr_metadata_and_safe_browser_targets() {
        assert!(parse(b"[]").unwrap().is_none());
        let row = serde_json::json!({"number":42,"title":"Build\n世界","state":"OPEN","url":"https://github.com/org/repo/pull/42","isDraft":true});
        let result = parse(&serde_json::to_vec(&vec![row.clone()]).unwrap()).unwrap().unwrap();
        assert_eq!(result.label(), "PR #42 · Draft · Build世界");
        for url in ["javascript:alert(1)","http://example.com/pr/42","https://user:secret@example.com/pr/42"] {
            let mut bad = row.clone(); bad["url"] = url.into();
            assert!(parse(&serde_json::to_vec(&vec![bad]).unwrap()).is_err());
        }
        assert!(parse(&serde_json::to_vec(&vec![row.clone(),row]).unwrap()).is_err());
        assert!(parse(&vec![b' ';65537]).is_err());
    }
}
