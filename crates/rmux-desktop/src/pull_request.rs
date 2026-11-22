//! Explicit GitHub CLI lookup, never called by rendering or Git watchers.
use anyhow::{Context, Result, ensure};
use std::{io::{Read, Seek}, path::Path, process::{Child, Command, Stdio}, sync::atomic::{AtomicBool, Ordering}, time::{Duration, Instant}};
static ACTIVE: AtomicBool = AtomicBool::new(false);
struct Permit;
impl Drop for Permit { fn drop(&mut self) { ACTIVE.store(false, Ordering::Release); } }
struct Running(Child);
impl Drop for Running { fn drop(&mut self) { let _ = self.0.kill(); let _ = self.0.wait(); } }
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PullRequest { pub number: u64, pub title: String, pub state: String, pub url: String, pub is_draft: bool }
impl PullRequest {
    pub fn label(&self) -> String {
        let state = match self.state.as_str() {
            "OPEN" if self.is_draft => "Draft",
            "OPEN" => "Open", "CLOSED" => "Closed", "MERGED" => "Merged", _ => "Unknown",
        };
        format!("PR #{} · {state} · {}", self.number, self.title)
    }
}
#[derive(Debug)]
pub struct Ambiguous;
impl std::fmt::Display for Ambiguous {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Multiple PRs match this branch; select the intended pull request in GitHub")
    }
}
impl std::error::Error for Ambiguous {}
fn parse(bytes: &[u8]) -> Result<Option<PullRequest>> {
    ensure!(bytes.len() <= 65536, "Pull request response exceeded 64 KiB");
    let mut rows: Vec<PullRequest> = serde_json::from_slice(bytes).context("Invalid GitHub CLI response")?;
    ensure!(rows.len() <= 2, "Unexpected pull request count");
    if rows.len() == 2 { return Err(Ambiguous.into()); }
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
    let expected_git_dir = &git_dir;
    let result = find(|state, timeout| async move {
        ensure!(crate::git_context::branch_at(expected_git_dir).as_deref() == Some(branch), "Git branch changed; refresh and retry");
        capture(command(directory, branch, state), timeout).await
    }).await?;
    // Resolve .git again too: the directory may now belong to a different worktree.
    let current = crate::git_context::git_directory(directory);
    ensure!(current.as_ref() == Some(&git_dir) && current.as_deref().is_some_and(|path| crate::git_context::branch_at(path).as_deref() == Some(branch)), "Git branch changed during lookup; refresh and retry");
    Ok(result)
}
fn command(directory: &Path, branch: &str, state: &str) -> Command {
    let mut command = Command::new("gh");
    command.args(["pr", "list", "--state", state, "--limit", "2", "--json", "number,title,state,url,isDraft"])
        .arg(format!("--head={branch}")).current_dir(directory)
        .env("GH_PROMPT_DISABLED", "1").env("GH_PAGER", "cat")
        .stdin(Stdio::null()).stderr(Stdio::null());
    #[cfg(target_os = "windows")]
    { use std::os::windows::process::CommandExt; command.creation_flags(0x08000000); }
    command
}
async fn find<F, Fut>(mut query: F) -> Result<Option<PullRequest>>
where F: FnMut(&'static str, Duration) -> Fut,
      Fut: std::future::Future<Output = Result<Vec<u8>>> {
    let started = Instant::now();
    for state in ["open", "all"] {
        let remaining = Duration::from_secs(10).checked_sub(started.elapsed())
            .filter(|remaining| !remaining.is_zero()).context("Pull request lookup timed out")?;
        if let Some(pr) = parse(&query(state, remaining).await?)? {
            ensure!(state != "open" || pr.state == "OPEN", "Unexpected closed PR in open results");
            return Ok(Some(pr));
        }
    }
    Ok(None)
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
    fn prefers_open_and_never_guesses_between_matches() {
        fn row(number: u64, state: &str) -> serde_json::Value {
            serde_json::json!({"number":number,"title":"Work","state":state,
                "url":format!("https://github.com/org/repo/pull/{number}"),"isDraft":false})
        }
        fn run(responses: Vec<Result<Vec<u8>>>) -> (Result<Option<PullRequest>>, Vec<&'static str>) {
            let mut responses = responses.into_iter();
            let mut calls = Vec::new();
            let mut previous = Duration::from_secs(10);
            let result = futures_lite::future::block_on(find(|state, remaining| {
                assert!(remaining <= previous); previous = remaining;
                calls.push(state);
                std::future::ready(responses.next().expect("unexpected query"))
            }));
            (result, calls)
        }
        let bytes = |rows: Vec<serde_json::Value>| Ok(serde_json::to_vec(&rows).unwrap());
        let (result, calls) = run(vec![bytes(vec![row(2, "OPEN")])]);
        assert_eq!(result.unwrap().unwrap().number, 2);
        assert_eq!(calls, ["open"]);
        let (result, calls) = run(vec![bytes(vec![]), bytes(vec![row(1, "MERGED")])]);
        assert_eq!(result.unwrap().unwrap().number, 1);
        assert_eq!(calls, ["open", "all"]);
        for responses in [
            vec![bytes(vec![row(1, "OPEN"), row(2, "OPEN")])],
            vec![bytes(vec![]), bytes(vec![row(1, "MERGED"), row(2, "CLOSED")])],
        ] {
            assert!(run(responses).0.unwrap_err().is::<Ambiguous>());
        }
        let (result, calls) = run(vec![Err(anyhow::anyhow!("offline"))]);
        assert!(result.is_err()); assert_eq!(calls, ["open"]);
        assert!(run(vec![bytes(vec![]), bytes(vec![])]).0.unwrap().is_none());
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
