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
#[derive(Clone)]
pub(crate) struct Store { directory: PathBuf }
impl Store {
    pub fn new(directory: PathBuf) -> Self { Self { directory } }
    fn path(&self, id: &str) -> Result<PathBuf> {
        ensure!(rmux_core::shell_history::valid_identity(id), "invalid terminal history identity");
        Ok(self.directory.join(format!("{id}.json")))
    }
    pub fn allocate(&self) -> Result<String> {
        use std::io::Write;
        self.prepare_directory()?;
        // tempfile supplies collision-resistant randomness and creates mode 0600.
        // Encode the random ASCII basename as hex for the snapshot identity.
        let mut file = tempfile::Builder::new().prefix("").rand_bytes(16).tempfile_in(&self.directory)?;
        let name = file.path().file_name().unwrap().to_string_lossy();
        let id: String = name.as_bytes().iter().map(|byte| format!("{byte:02x}")).collect();
        let path = self.path(&id)?;
        let bytes = serde_json::to_vec(&Saved { version: 1, commands: Vec::new() })?;
        file.write_all(&bytes)?;
        file.as_file().sync_all()?;
        file.persist_noclobber(path)?;
        Ok(id)
    }
    fn prepare_directory(&self) -> Result<()> {
        let mut builder = std::fs::DirBuilder::new();
        builder.recursive(true);
        #[cfg(unix)] {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(&self.directory)?;
        Ok(())
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
        self.prepare_directory()?;
        let saved = Saved { version: 1, commands: history.entries().map(str::to_owned).collect() };
        let bytes = serde_json::to_vec(&saved)?;
        ensure!(bytes.len() <= FILE_LIMIT, "terminal history exceeds size limit");
        crate::snapshot::atomic_save(&path, &bytes)
    }
}

/// An exclusive writer follows the terminal, including transfers between panes.
/// Keep its sidecar inode in place: atomic JSON replacement must not release it.
pub(crate) struct Lease {
    store: Store,
    id: String,
    _lock: std::fs::File,
    saved_revision: std::sync::atomic::AtomicU64,
}
impl Lease {
    pub fn id(&self) -> &str { &self.id }
    fn acquire(store: Store, id: String) -> Result<Self> {
        store.prepare_directory()?;
        let path = store.path(&id)?.with_extension("lock");
        let mut options = std::fs::OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)] {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let lock = options.open(path)?;
        lock.try_lock().map_err(|error| anyhow::anyhow!("terminal history is already in use or cannot be locked: {error}"))?;
        Ok(Self { store, id, _lock: lock, saved_revision: std::sync::atomic::AtomicU64::new(0) })
    }
    pub fn save(&self, history: &History) -> Result<()> { self.store.save(&self.id, history) }
}
impl Store {
    /// A restore always gets a fresh writer identity. Reading the old atomic
    /// snapshot is safe while its original terminal is still active, and two
    /// restores never overwrite each other's subsequent commands. The source
    /// identity remains untouched for other saved sessions that reference it.
    pub fn fork(&self, source: Option<&str>) -> Result<(std::sync::Arc<Lease>, History)> {
        let history = source.map(|id| self.load(id)).transpose()?.unwrap_or_default();
        let id = self.allocate()?;
        let lease = std::sync::Arc::new(Lease::acquire(self.clone(), id)?);
        lease.save(&history)?;
        Ok((lease, history))
    }
}

struct Pending {
    lease: std::sync::Arc<Lease>,
    revision: u64,
    history: History,
}
/// One latest snapshot per terminal, with a finite queue during disk failures.
/// Enqueue is memory-only; the executor calls flush outside the UI/PTY locks.
/// Errors retain snapshots for an explicit retry. Successful writes release
/// leases only after the atomic replacement finishes.
#[derive(Default)]
pub(crate) struct Writes {
    pending: std::sync::Mutex<std::collections::HashMap<String, Pending>>,
    flush_lock: std::sync::Mutex<()>,
}
impl Writes {
    pub fn enqueue(&self, lease: std::sync::Arc<Lease>, revision: u64, history: &History) -> Result<()> {
        let mut pending = self.pending.lock().unwrap_or_else(|error| error.into_inner());
        if revision <= lease.saved_revision.load(std::sync::atomic::Ordering::Acquire) { return Ok(()); }
        if let Some(previous) = pending.get(lease.id()) {
            if revision <= previous.revision { return Ok(()); }
        } else {
            ensure!(pending.len() < 64, "terminal history save queue is full; retry after flushing");
        }
        pending.insert(lease.id().to_owned(), Pending { lease, revision, history: history.clone() });
        Ok(())
    }
    pub fn flush(&self) -> Result<()> {
        let _serial = self.flush_lock.lock().unwrap_or_else(|error| error.into_inner());
        let ids: Vec<_> = self.pending.lock().unwrap_or_else(|error| error.into_inner()).keys().cloned().collect();
        let mut failures = Vec::new();
        for id in ids {
            let (lease, revision, history) = {
                let pending = self.pending.lock().unwrap_or_else(|error| error.into_inner());
                let Some(entry) = pending.get(&id) else { continue; };
                (entry.lease.clone(), entry.revision, entry.history.clone())
            };
            // No queue lock during filesystem I/O; newer commands can coalesce.
            if let Err(error) = lease.save(&history) {
                failures.push(format!("{id}: {error:#}"));
                continue;
            }
            let mut pending = self.pending.lock().unwrap_or_else(|error| error.into_inner());
            lease.saved_revision.store(revision, std::sync::atomic::Ordering::Release);
            if pending.get(&id).is_some_and(|entry| entry.revision == revision) {
                pending.remove(&id);
            }
        }
        ensure!(failures.is_empty(), "could not save terminal history: {}", failures.join("; "));
        Ok(())
    }
}

/// A single event-driven writer is shared by every history-enabled terminal.
/// Clones own only a bounded wake channel and shared state, not another thread.
#[derive(Clone)]
pub(crate) struct Worker {
    writes: std::sync::Arc<Writes>,
    wake: std::sync::mpsc::SyncSender<()>,
    error: std::sync::Arc<std::sync::Mutex<Option<String>>>,
}
impl Worker {
    pub fn start() -> Result<Self> {
        use std::sync::{Arc, Mutex, mpsc};
        let writes = Arc::new(Writes::default());
        let error = Arc::new(Mutex::new(None));
        let (wake, receiver) = mpsc::sync_channel(1);
        let pending = writes.clone();
        let failure = error.clone();
        std::thread::Builder::new().name("rmux-history-save".into()).spawn(move || {
            loop {
                let mut disconnected = receiver.recv().is_err();
                if !disconnected {
                    // A fixed window coalesces bursts without starving a busy
                    // shell. No timer exists while there is no pending work.
                    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(250);
                    loop {
                        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
                        if remaining.is_zero() { break; }
                        match receiver.recv_timeout(remaining) {
                            Ok(()) => (),
                            Err(mpsc::RecvTimeoutError::Timeout) => break,
                            Err(mpsc::RecvTimeoutError::Disconnected) => { disconnected = true; break; }
                        }
                    }
                }
                let result = pending.flush().err().map(|error| format!("{error:#}"));
                let mut previous = failure.lock().unwrap_or_else(|error| error.into_inner());
                if result != *previous {
                    if let Some(message) = &result { eprintln!("rmux: {message}"); }
                    *previous = result;
                }
                if disconnected { break; }
            }
        })?;
        Ok(Self { writes, wake, error })
    }
    fn queue(&self, lease: std::sync::Arc<Lease>, revision: u64, history: &History) -> Result<()> {
        let result = self.writes.enqueue(lease, revision, history);
        // Wake even on overflow so producers can retry after the queue drains.
        match self.wake.try_send(()) {
            Ok(()) | Err(std::sync::mpsc::TrySendError::Full(())) => (),
            Err(std::sync::mpsc::TrySendError::Disconnected(())) => anyhow::bail!("terminal history worker stopped"),
        }
        result
    }
    /// Explicit save/quit barriers call this on a background executor after
    /// terminal readers have drained. It serializes with the regular worker.
    pub fn flush(&self) -> Result<()> { self.writes.flush() }
    pub fn error(&self) -> Option<String> {
        self.error.lock().unwrap_or_else(|error| error.into_inner()).clone()
    }
    pub fn shell(&self, store: &Store, source: Option<&str>, columns: u16, rows: u16,
        directory: Option<&std::path::Path>, scrollback_bytes: usize) -> Result<rmux_terminal::session::Session> {
        let (lease, history) = store.fork(source)?;
        let id = lease.id().to_owned();
        let sink = std::sync::Arc::new(Recorder { worker: self.clone(), lease });
        let mut session = rmux_terminal::session::Session::shell_in_with_history_sink(
            columns, rows, directory, scrollback_bytes, Some(&history), Some(sink))?;
        session.set_shell_history_id(Some(id))?;
        Ok(session)
    }
}
struct Recorder { worker: Worker, lease: std::sync::Arc<Lease> }
impl rmux_terminal::session::HistorySink for Recorder {
    fn changed(&self, revision: u64, history: &History) -> Result<()> {
        self.worker.queue(self.lease.clone(), revision, history)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const FIRST: &str = "0123456789abcdef0123456789abcdef";
    const SECOND: &str = "fedcba9876543210fedcba9876543210";


    #[test]
    fn shared_worker_flushes_when_last_owner_closes() -> Result<()> {
        let temporary = tempfile::tempdir()?;
        let store = Store::new(temporary.path().join("history"));
        let (lease, _) = store.fork(None)?;
        let id = lease.id().to_owned();
        let worker = Worker::start()?;
        let mut history = History::default();
        history.record("echo retained-on-close");
        worker.queue(lease.clone(), 1, &history)?;
        drop(lease);
        drop(worker);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            if let Ok(_lease) = Lease::acquire(store.clone(), id.clone()) {
                assert_eq!(store.load(&id)?, history);
                break;
            }
            ensure!(std::time::Instant::now() < deadline, "history worker did not finish its final flush");
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        Ok(())
    }
    #[test]
    fn restored_writers_are_independent_and_leased_until_pending_save_finishes() -> Result<()> {
        let temporary = tempfile::tempdir()?;
        let store = Store::new(temporary.path().join("history"));
        let mut original = History::default();
        original.record("echo original");
        store.save(FIRST, &original)?;
        let (first, restored) = store.fork(Some(FIRST))?;
        let (second, _) = store.fork(Some(FIRST))?;
        assert_eq!(restored, original);
        assert_ne!(first.id(), second.id());
        let id = first.id().to_owned();
        assert!(Lease::acquire(store.clone(), id.clone()).is_err());
        let writes = Writes::default();
        let mut changed = restored;
        changed.record("echo first-only");
        writes.enqueue(first.clone(), 1, &changed)?;
        drop(first);
        assert!(Lease::acquire(store.clone(), id.clone()).is_err());
        writes.flush()?;
        let _reopened = Lease::acquire(store.clone(), id.clone())?;
        assert_eq!(store.load(&id)?, changed);
        assert_eq!(store.load(second.id())?, original);
        assert_eq!(store.load(FIRST)?, original);
        Ok(())
    }
    #[test]
    fn saves_coalesce_reject_old_revisions_and_retry_after_io_failure() -> Result<()> {
        let temporary = tempfile::tempdir()?;
        let directory = temporary.path().join("history");
        let store = Store::new(directory.clone());
        let (lease, _) = store.fork(None)?;
        let writes = Writes::default();
        let mut old = History::default();
        old.record("echo old");
        let mut recent = History::default();
        recent.record("echo recent");
        writes.enqueue(lease.clone(), 1, &old)?;
        writes.enqueue(lease.clone(), 3, &recent)?;
        writes.enqueue(lease.clone(), 2, &old)?;
        let backup = temporary.path().join("backup");
        std::fs::rename(&directory, &backup)?;
        std::fs::write(&directory, "block directory creation")?;
        assert!(writes.flush().is_err());
        assert_eq!(writes.pending.lock().unwrap().len(), 1);
        std::fs::remove_file(&directory)?;
        std::fs::rename(backup, directory)?;
        writes.flush()?;
        assert_eq!(store.load(lease.id())?, recent);
        // A delayed producer after flush must not resurrect older commands.
        writes.enqueue(lease.clone(), 2, &old)?;
        assert!(writes.pending.lock().unwrap().is_empty());
        writes.enqueue(lease.clone(), 4, &History::default())?;
        writes.flush()?;
        assert_eq!(store.load(lease.id())?, History::default());
        Ok(())
    }
    #[test]
    fn corrupt_source_is_not_replaced_or_forked() -> Result<()> {
        let temporary = tempfile::tempdir()?;
        let store = Store::new(temporary.path().to_owned());
        std::fs::write(store.path(FIRST)?, "{broken")?;
        assert!(store.fork(Some(FIRST)).is_err());
        assert_eq!(std::fs::read_dir(temporary.path())?.count(), 1);
        assert_eq!(std::fs::read_to_string(store.path(FIRST)?)?, "{broken");
        Ok(())
    }
    #[test]
    fn allocated_histories_are_distinct_and_start_empty() -> Result<()> {
        let temporary = tempfile::tempdir()?;
        let store = Store::new(temporary.path().join("history"));
        let first = store.allocate()?;
        let second = store.allocate()?;
        assert_ne!(first, second);
        assert!(rmux_core::shell_history::valid_identity(&first));
        assert_eq!(store.load(&first)?, History::default());
        assert_eq!(store.load(&second)?, History::default());
        assert_eq!(std::fs::read_dir(&store.directory)?.count(), 2);
        Ok(())
    }
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
        assert!(history.clear());
        store.save(FIRST, &history)?;
        assert_eq!(store.load(FIRST)?, History::default());
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
