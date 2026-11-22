//! Non-interactive configured commands. No PTY, output buffer or UI-thread wait.
use anyhow::{Context, Result, ensure};
use std::{path::Path, process::{Child, Command, ExitStatus, Stdio}, sync::atomic::{AtomicUsize, Ordering}};
use crate::preferences::CustomCommand;

static ACTIVE: AtomicUsize = AtomicUsize::new(0);
struct Permit;
impl Permit {
    fn acquire() -> Result<Self> {
        ACTIVE.try_update(Ordering::AcqRel, Ordering::Acquire, |count| (count < 8).then_some(count + 1))
            .map_err(|_| anyhow::anyhow!("Eight background commands are still running"))?;
        Ok(Self)
    }
}
impl Drop for Permit { fn drop(&mut self) { ACTIVE.fetch_sub(1, Ordering::AcqRel); } }

pub struct Running { child: Child, _permit: Permit }
pub fn launch(config: &CustomCommand, directory: Option<&Path>) -> Result<Running> {
    let permit = Permit::acquire()?;
    let program = config.argv.first().context("Command has no program")?;
    let mut command = Command::new(program);
    command.args(&config.argv[1..]).envs(&config.env)
        .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    if let Some(directory) = config.cwd.as_deref().or(directory) { command.current_dir(directory); }
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    let child = command.spawn().with_context(|| format!("Could not launch {program}"))?;
    Ok(Running { child, _permit: permit })
}
fn outcome(status: ExitStatus) -> Result<()> {
    ensure!(status.success(), "Command exited unsuccessfully ({status})");
    Ok(())
}
impl Running {
    pub async fn reap(mut self) -> Result<()> {
        loop {
            if let Some(status) = self.child.try_wait().context("Could not collect command exit status")? { return outcome(status); }
            gpui::Timer::after(std::time::Duration::from_millis(250)).await;
        }
    }
    /// Standalone --command invocation has no UI; report its exit status.
    pub fn wait(mut self) -> Result<()> { outcome(self.child.wait().context("Could not wait for command")?) }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    #[test]
    fn literal_arguments_environment_directory_and_exit_status() -> Result<()> {
        let root = tempfile::tempdir()?;
        let output = root.path().join("result");
        let literal = "spaces 'quotes' $(exit 99); 世界";
        let config: CustomCommand = serde_json::from_value(serde_json::json!({
            "name":"fixture", "mode":"background", "cwd":root.path(),
            "env":{"RMUX_FIXTURE_VALUE":"environment value"},
            "argv":["/bin/sh", "-c", "printf '%s\\n%s\\n%s' \"$PWD\" \"$RMUX_FIXTURE_VALUE\" \"$1\" > result", "fixture", literal]
        }))?;
        launch(&config, Some(Path::new("/missing-fallback")))?.wait()?;
        let actual = std::fs::read_to_string(output)?;
        assert!(actual.ends_with(&format!("\nenvironment value\n{literal}")));
        assert_eq!(std::fs::canonicalize(actual.lines().next().unwrap())?, root.path().canonicalize()?);
        let mut failing = config.clone();
        failing.argv = vec!["/bin/sh".into(), "-c".into(), "exit 7".into()];
        assert!(launch(&failing, None)?.wait().unwrap_err().to_string().contains('7'));
        failing.argv = vec![root.path().join("missing-program").display().to_string()];
        assert!(launch(&failing, None).is_err());
        // Failed launches release the slot rather than exhausting the cap.
        for _ in 0..10 { assert!(launch(&failing, None).is_err()); }
        Ok(())
    }
}
