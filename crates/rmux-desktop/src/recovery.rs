//! Opt-in, event-driven checkpoints. No periodic idle polling or per-frame hook.
use super::*;
use std::{path::PathBuf, sync::Mutex, time::Duration};

pub(super) struct Lease {
    pub path: PathBuf,
    _lock: std::fs::File,
}
impl Lease {
    pub fn acquire(path: PathBuf) -> anyhow::Result<Arc<Self>> {
        if let Some(parent) = path.parent().filter(|parent| !parent.as_os_str().is_empty()) {
            let mut directories = std::fs::DirBuilder::new();
            directories.recursive(true);
            #[cfg(unix)] {
                use std::os::unix::fs::DirBuilderExt;
                directories.mode(0o700);
            }
            directories.create(parent)?;
        }
        let mut lock_path = path.as_os_str().to_os_string();
        lock_path.push(".lock");
        let mut options = std::fs::OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)] {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let lock = options.open(PathBuf::from(lock_path))?;
        lock.try_lock().map_err(|error| anyhow::anyhow!("recovery file is in use or cannot be locked: {error}"))?;
        // A separate file survives atomic replacement of the checkpoint. Keep
        // the lock file in place: unlinking it could allow two inode owners.
        Ok(Arc::new(Self { path, _lock: lock }))
    }
}

#[derive(Default)]
struct Writer {
    generation: u64,
    previous: Option<Vec<u8>>,
}
impl Writer {
    fn save(&mut self, path: &std::path::Path, generation: u64, snapshot: snapshot::SessionSnapshot) -> anyhow::Result<()> {
        if generation < self.generation { return Ok(()); }
        self.generation = generation;
        let bytes = snapshot.encode()?;
        if self.previous.as_ref() == Some(&bytes) { return Ok(()); }
        snapshot::atomic_save(path, &bytes)?;
        self.previous = Some(bytes);
        Ok(())
    }
}
struct Recovery {
    lease: Option<Arc<Lease>>,
    generation: u64,
    writer: Arc<Mutex<Writer>>,
    wake: async_channel::Sender<()>,
}
impl Global for Recovery {}

pub(super) fn changed(cx: &App) {
    let state = cx.global::<Recovery>();
    if state.lease.is_some() && !cx.global::<Shutdowns>().quitting { let _ = state.wake.try_send(()); }
}

pub(super) fn checkpoint(cx: &mut App) -> Option<Task<anyhow::Result<()>>> {
    let lease = cx.global::<Recovery>().lease.clone()?;
    let snapshot = match super::capture_session(cx) {
        Ok(snapshot) => snapshot,
        // An editing/restore interaction may temporarily prevent coherent capture.
        // Its next notification retries; don't run an idle retry timer.
        Err(error) => { eprintln!("rmux: session checkpoint skipped: {error:#}"); return None; }
    };
    let state = cx.global_mut::<Recovery>();
    state.generation += 1;
    let generation = state.generation;
    let writer = state.writer.clone();
    Some(cx.background_executor().spawn(async move {
        writer.lock().unwrap_or_else(|error| error.into_inner()).save(&lease.path, generation, snapshot)
    }))
}

fn observe<T: 'static>(cx: &App) {
    cx.observe_new::<T>(|_, _, cx| {
        let entity = cx.entity();
        // App's observer retains only the entity ID, not the live terminal.
        App::observe(cx, &entity, |_, cx| changed(cx)).detach();
        changed(cx);
    }).detach();
}

pub(super) fn install(lease: Option<Arc<Lease>>, cx: &mut App) {
    let enabled = lease.is_some();
    let (wake, receiver) = async_channel::bounded(1);
    cx.set_global(Recovery { lease, generation: 0, writer: Arc::new(Mutex::new(Writer::default())), wake });
    if !enabled { return; }
    observe::<window_view::WorkspaceWindow>(cx);
    cx.observe_new::<window_view::WorkspaceWindow>(|_, window, cx| {
        if let Some(window) = window {
            cx.observe_window_bounds(window, |_, _, cx| changed(cx)).detach();
        }
    }).detach();
    observe::<Workspace>(cx);
    observe::<PaneView>(cx);
    cx.spawn(async move |cx| {
        let mut reported_error = None;
        while receiver.recv().await.is_ok() {
            cx.background_executor().timer(Duration::from_millis(750)).await;
            while receiver.try_recv().is_ok() {}
            let job = cx.update(|cx| {
                if cx.global::<Shutdowns>().quitting { None } else { checkpoint(cx) }
            }).ok().flatten();
            if let Some(job) = job {
                match job.await {
                    Ok(()) => reported_error = None,
                    Err(error) => {
                        let error = format!("{error:#}");
                        // Showing a notice notifies WorkspaceWindow, which also
                        // schedules a checkpoint. Suppress identical failures
                        // so that notification cannot create an idle retry loop.
                        if reported_error.as_ref() == Some(&error) { continue; }
                        eprintln!("rmux: save checkpoint: {error}");
                        let notice = format!("Could not save session recovery: {error}. Your latest changes may not survive a restart. Check the session_file location or use Save Session to save elsewhere.");
                        reported_error = Some(error);
                        let _ = cx.update(|cx| {
                            let target = cx.active_window().and_then(|handle| handle.downcast::<window_view::WorkspaceWindow>())
                                .or_else(|| cx.windows().into_iter().find_map(|handle| handle.downcast::<window_view::WorkspaceWindow>()));
                            if let Some(target) = target {
                                let _ = target.update(cx, |view, _, cx| view.show_startup_notice(notice, cx));
                            }
                        });
                    }
                }
            }
        }
    }).detach();
}

#[cfg(test)]
mod tests {
    use super::{Writer, Lease};
    use crate::snapshot;
    #[test]
    fn first_launch_creates_recovery_directory_without_replacing_existing_files() -> anyhow::Result<()> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("new/nested/session.json");
        let lease = Lease::acquire(path.clone())?;
        assert!(!path.exists(), "acquiring recovery must not create a fake checkpoint");
        let mut writer = Writer::default();
        writer.save(&lease.path, 1, snapshot::SessionSnapshot { version: 1, active_window: None, windows: vec![] })?;
        assert!(path.is_file());
        assert!(Lease::acquire(path).is_err());
        let obstruction = directory.path().join("file");
        std::fs::write(&obstruction, b"keep")?;
        assert!(Lease::acquire(obstruction.join("session.json")).is_err());
        assert_eq!(std::fs::read(obstruction)?, b"keep");
        Ok(())
    }
    #[test]
    fn lease_excludes_competitors_and_releases_after_last_owner() -> anyhow::Result<()> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("session.json");
        std::fs::write(&path, b"untouched")?;
        let owner = Lease::acquire(path.clone())?;
        let background = owner.clone();
        assert!(Lease::acquire(path.clone()).is_err());
        drop(owner);
        assert!(Lease::acquire(path.clone()).is_err());
        drop(background);
        let _next = Lease::acquire(path.clone())?;
        assert_eq!(std::fs::read(path)?, b"untouched");
        Ok(())
    }
    #[test]
    fn older_background_job_cannot_replace_newer_checkpoint() -> anyhow::Result<()> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("session.json");
        let mut writer = Writer::default();
        let newer = snapshot::SessionSnapshot { version: 1, active_window: None, windows: vec![] };
        writer.save(&path, 2, newer)?;
        let bytes = std::fs::read(&path)?;
        // A stale job is rejected before serialization, even if its data would
        // now fail validation. The successfully saved newer file stays intact.
        let older = snapshot::SessionSnapshot { version: 99, active_window: None, windows: vec![] };
        writer.save(&path, 1, older)?;
        assert_eq!(std::fs::read(path)?, bytes);
        assert_eq!(writer.generation, 2);
        Ok(())
    }

    #[test]
    fn failed_checkpoint_does_not_suppress_a_later_retry() -> anyhow::Result<()> {
        let directory = tempfile::tempdir()?;
        let obstruction = directory.path().join("blocked");
        std::fs::write(&obstruction, b"keep")?;
        let path = obstruction.join("session.json");
        let snapshot = || snapshot::SessionSnapshot { version: 1, active_window: None, windows: vec![] };
        let mut writer = Writer::default();
        assert!(writer.save(&path, 1, snapshot()).is_err());
        assert!(writer.previous.is_none());
        assert_eq!(std::fs::read(&obstruction)?, b"keep");
        std::fs::remove_file(&obstruction)?;
        std::fs::create_dir(&obstruction)?;
        writer.save(&path, 2, snapshot())?;
        assert_eq!(std::fs::read(path)?, snapshot().encode()?);
        Ok(())
    }
}
