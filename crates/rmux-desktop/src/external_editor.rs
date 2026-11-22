//! Explicit external-editor argv; file paths are data, never shell source.
use anyhow::{Context, Result, ensure};
use std::{path::{Path, PathBuf}, process::{Child, Command, Stdio}, sync::atomic::{AtomicUsize, Ordering}};
#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Editor {
    pub program: PathBuf,
    pub args: Vec<String>,
}
impl Editor {
    pub fn validate(&self) -> Result<()> {
        ensure!(self.program.is_absolute() && self.program.as_os_str().len() <= 8192 && !self.program.as_os_str().as_encoded_bytes().contains(&0), "file_editor.program must be an absolute path up to 8192 bytes without NULs");
        ensure!(self.args.len() <= 64 && self.args.iter().all(|arg| arg.len() <= 8192 && !arg.contains('\0')), "file_editor.args must contain at most 64 arguments, each up to 8192 bytes without NULs");
        ensure!(self.args.iter().filter(|arg| arg.as_str() == "{file}").count() == 1, "file_editor.args must contain exactly one standalone {{file}} argument");
        Ok(())
    }
    fn command(&self, path: &Path) -> Result<Command> {
        self.validate()?;
        ensure!(path.is_absolute() && path.as_os_str().len() <= 8192
            && !path.as_os_str().as_encoded_bytes().contains(&0), "Editor file path must be absolute and up to 8192 bytes without NULs");
        let mut command = Command::new(&self.program);
        command.current_dir(path.parent().context("Editor file path has no parent directory")?);
        for arg in &self.args {
            if arg == "{file}" { command.arg(path); } else { command.arg(arg); }
        }
        command.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
        #[cfg(windows)] {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000); // CREATE_NO_WINDOW for CLI editor launchers.
        }
        Ok(command)
    }
    pub fn launch(&self, path: &Path) -> Result<RunningEditor> {
        let permit = Permit::acquire()?;
        let mut command = self.command(path)?;
        let metadata = std::fs::symlink_metadata(path)
            .with_context(|| format!("Could not access editor file {}", path.display()))?;
        ensure!(metadata.is_file(), "External editing requires a regular file, not a directory or symbolic link: {}", path.display());
        let child = command.spawn()
            .with_context(|| format!("Could not launch editor {} for {}", self.program.display(), path.display()))?;
        Ok(RunningEditor { child, _permit: permit })
    }
}
static ACTIVE: AtomicUsize = AtomicUsize::new(0);
struct Permit;
impl Permit {
    fn acquire() -> Result<Self> {
        ACTIVE.try_update(Ordering::AcqRel, Ordering::Acquire, |n| (n < 8).then_some(n + 1))
            .map_err(|_| anyhow::anyhow!("Eight editor launches are still active"))?;
        Ok(Self)
    }
}
impl Drop for Permit { fn drop(&mut self) { ACTIVE.fetch_sub(1, Ordering::AcqRel); } }
pub struct RunningEditor { child: Child, _permit: Permit }
impl RunningEditor {
    pub async fn reap(mut self) -> Result<()> {
        loop {
            if let Some(status) = self.child.try_wait().context("Could not collect editor exit status")? {
                ensure!(status.success(), "Editor exited unsuccessfully ({status})");
                return Ok(());
            }
            gpui::Timer::after(std::time::Duration::from_secs(1)).await;
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    #[test]
    fn launched_editor_exit_status_and_literal_path_are_observed() -> Result<()> {
        let root = tempfile::tempdir()?;
        let path = root.path().join("spaces 'quotes' $(exit 99).sh");
        std::fs::write(&path, "This file is data, not an executable script")?;
        for code in [0, 7] {
            let editor = Editor { program: "/bin/sh".into(), args: vec![
                "-c".into(), format!("[ \"$1\" = \"$2\" ] || exit 99; [ \"$(pwd -P)\" = \"$3\" ] || exit 98; exit {code}"), "fixture".into(), "{file}".into(), path.to_str().unwrap().into(), root.path().canonicalize()?.to_str().unwrap().into()
            ] };
            let mut running = editor.launch(&path)?;
            // Waiting here is test-only; production reaping uses a timer.
            running.child.wait()?;
            let outcome = futures_lite::future::block_on(running.reap());
            if code == 0 { outcome?; } else { assert!(outcome.unwrap_err().to_string().contains('7')); }
        }
        let editor = Editor { program: "/bin/sh".into(), args: vec!["{file}".into()] };
        assert!(editor.launch(root.path()).is_err());
        let link = root.path().join("link");
        std::os::unix::fs::symlink(&path, &link)?;
        assert!(editor.launch(&link).is_err());
        let missing = root.path().join("missing-file");
        let error = editor.launch(&missing).err().unwrap();
        assert!(error.to_string().contains("Could not access editor file"));
        assert!(error.to_string().contains("missing-file"));
        let missing_editor = Editor { program: root.path().join("missing-editor"), args: vec!["{file}".into()] };
        let error = missing_editor.launch(&path).err().unwrap();
        assert!(error.to_string().contains("Could not launch editor"));
        assert!(error.to_string().contains("missing-editor"));
        Ok(())
    }
    #[test]
    fn editor_paths_remain_single_literal_arguments() -> Result<()> {
        let editor = Editor { program: std::env::current_exe()?, args: vec!["--".into(), "{file}".into()] };
        let path = std::env::temp_dir().join("spaces $(touch nope); 'quoted'.sh");
        let command = editor.command(&path)?;
        assert_eq!(command.get_current_dir(), path.parent());
        let args: Vec<_> = command.get_args().collect();
        assert_eq!(args, [std::ffi::OsStr::new("--"), path.as_os_str()]);
        for args in [vec![], vec!["{file}", "{file}"], vec!["prefix{file}"], vec!["{file}", "bad\0arg"]] {
            assert!(Editor { program: editor.program.clone(), args: args.into_iter().map(String::from).collect() }.validate().is_err());
        }
        assert!(editor.command(Path::new("relative.txt")).is_err());
        Ok(())
    }
}
