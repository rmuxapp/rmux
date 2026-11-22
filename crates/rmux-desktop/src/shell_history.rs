//! Bounded per-terminal history storage. Shell hooks, not PTY input, supply entries.
//! Call from background work; no global shell history file is opened here.
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::{io::Read, path::PathBuf};

use rmux_core::shell_history::{History, ENTRY_LIMIT, TEXT_LIMIT, COMMAND_LIMIT};
// JSON escaping may expand each byte sixfold.
const FILE_LIMIT: usize = TEXT_LIMIT * 6 + ENTRY_LIMIT * 8 + 128;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Saved {
    version: u32,
    commands: Vec<String>,
}

/// The caller owns a private application-state directory. IDs are opaque hex,
/// never caller-selected paths or names derived from a working directory.
pub(crate) struct Store { directory: PathBuf }
impl Store {
    pub fn new(directory: PathBuf) -> Self { Self { directory } }
    fn path(&self, id: &str) -> Result<PathBuf> {
        ensure!(id.len() == 32 && id.bytes().all(|byte| byte.is_ascii_hexdigit()), "invalid terminal history identity");
        Ok(self.directory.join(format!("{}.json", id.to_ascii_lowercase())))
    }
    pub fn load(&self, id: &str) -> Result<History> {
        let path = self.path(id)?;
        let file = match std::fs::File::open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(History::default()),
            Err(error) => return Err(error.into()),
        };
        let mut bytes = Vec::new();
        file.take(FILE_LIMIT as u64 + 1).read_to_end(&mut bytes)?;
        ensure!(bytes.len() <= FILE_LIMIT, "terminal history exceeds size limit");
        let saved: Saved = serde_json::from_slice(&bytes)?;
        ensure!(saved.version == 1, "unsupported terminal history version");
        ensure!(saved.commands.len() <= ENTRY_LIMIT, "too many terminal history entries");
        let mut history = History::default();
        for command in saved.commands {
            ensure!(command.len() <= COMMAND_LIMIT && !command.contains('\0'), "invalid terminal history entry");
            ensure!(history.text_bytes() + command.len() <= TEXT_LIMIT, "terminal history exceeds text limit");
            history.record(&command);
        }
        Ok(history)
    }
    pub fn save(&self, id: &str, history: &History) -> Result<()> {
        let path = self.path(id)?;
        let mut builder = std::fs::DirBuilder::new();
        builder.recursive(true);
        #[cfg(unix)] {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(&self.directory)?;
        let saved = Saved { version: 1, commands: history.entries().map(str::to_owned).collect() };
        let bytes = serde_json::to_vec(&saved)?;
        ensure!(bytes.len() <= FILE_LIMIT, "terminal history exceeds size limit");
        crate::snapshot::atomic_save(&path, &bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const FIRST: &str = "0123456789abcdef0123456789abcdef";
    const SECOND: &str = "fedcba9876543210fedcba9876543210";
    #[test]
    fn restore_keeps_multiline_unicode_and_terminal_histories_separate() -> Result<()> {
        let temporary = tempfile::tempdir()?;
        let store = Store::new(temporary.path().join("history"));
        let mut history = History::default();
        history.record("printf '%s\\n' 'café 世界'\n echo next");
        history.record(" leading space remains data; shell decides whether to record");
        store.save(FIRST, &history)?;
        assert_eq!(store.load(FIRST)?, history);
        assert_eq!(store.load(SECOND)?, History::default());
        #[cfg(unix)] {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(std::fs::metadata(&store.directory)?.permissions().mode() & 0o077, 0);
            assert_eq!(std::fs::metadata(store.path(FIRST)?)?.permissions().mode() & 0o077, 0);
        }
        Ok(())
    }
    #[test]
    fn recording_retains_recent_commands_with_bounded_memory() {
        let mut history = History::default();
        for index in 0..1000 { history.record(&format!("echo {index}")); }
        assert_eq!(history.entries().len(), ENTRY_LIMIT);
        assert_eq!(history.entries().next().unwrap(), "echo 488");
        assert!(!history.record("echo 999"));
        assert!(!history.record(" \n"));
        assert!(!history.record("bad\0command"));
        assert!(!history.record(&"x".repeat(COMMAND_LIMIT + 1)));
        for index in 0..100 { history.record(&format!("{index}{}", "x".repeat(8000))); }
        assert!(history.text_bytes() <= TEXT_LIMIT);
        assert!(history.entries().len() < ENTRY_LIMIT);
        assert!(history.entries().next_back().unwrap().starts_with("99"));
    }
    #[test]
    fn malformed_or_future_history_is_not_silently_overwritten() -> Result<()> {
        let temporary = tempfile::tempdir()?;
        let store = Store::new(temporary.path().to_owned());
        assert!(store.load("../../other").is_err());
        assert!(store.save("../other", &History::default()).is_err());
        let path = store.path(FIRST)?;
        for bytes in [b"{broken".as_slice(), b"{\"version\":2,\"commands\":[]}"] {
            std::fs::write(&path, bytes)?;
            assert!(store.load(FIRST).is_err());
            assert_eq!(std::fs::read(&path)?, bytes);
        }
        std::fs::write(&path, vec![b' '; FILE_LIMIT + 1])?;
        assert!(store.load(FIRST).is_err());
        Ok(())
    }
}
