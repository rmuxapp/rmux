//! Prepare fresh shells off the UI thread before publishing a restored window.
use super::*;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Default)]
pub struct Job(pub Arc<AtomicBool>);
impl Job {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        self.cancel();
    }
}

pub fn cancelled(token: &AtomicBool) -> bool {
    token.load(Ordering::Acquire)
}
fn check_cancelled(token: &AtomicBool) -> anyhow::Result<()> {
    anyhow::ensure!(!cancelled(token), "layout restore cancelled");
    Ok(())
}

pub struct Prepared {
    pub snapshot: snapshot::WindowSnapshot,
    sessions: VecDeque<Session>,
}

// Validate every directory before starting the first shell. Spawn still checks
// again because directories can disappear between preflight and process start.
fn preflight(node: &snapshot::Node, token: &AtomicBool) -> anyhow::Result<()> {
    use anyhow::Context;
    check_cancelled(token)?;
    match node {
        snapshot::Node::Pane { tabs, .. } => {
            for tab in tabs {
                check_cancelled(token)?;
                if let Some(path) = &tab.directory {
                    let metadata = std::fs::metadata(path)
                        .with_context(|| format!("working directory {}", path.display()))?;
                    anyhow::ensure!(
                        metadata.is_dir(),
                        "working directory {} is not a directory",
                        path.display()
                    );
                }
            }
        }
        snapshot::Node::Split { children, .. } => {
            for child in children {
                preflight(child, token)?;
            }
        }
    }
    Ok(())
}

impl Prepared {
    pub fn read(
        path: &std::path::Path,
        token: &AtomicBool,
        scrollback_bytes: usize,
    ) -> anyhow::Result<Self> {
        check_cancelled(token)?;
        let snapshot = snapshot::WindowSnapshot::load(path)?;
        Self::from_snapshot(snapshot, token, scrollback_bytes)
    }
    fn from_snapshot(snapshot: snapshot::WindowSnapshot, token: &AtomicBool, scrollback_bytes: usize) -> anyhow::Result<Self> {
        Self::prepare_with(snapshot, token, &mut |tab| shell_history::shell(80, 24, tab.directory.as_deref(), scrollback_bytes, tab.shell_history_id.as_deref()))
    }
    fn prepare_with(snapshot: snapshot::WindowSnapshot, token: &AtomicBool, spawn: &mut impl FnMut(&snapshot::TabSnapshot) -> anyhow::Result<Session>) -> anyhow::Result<Self> {
        for workspace in &snapshot.workspaces {
            preflight(&workspace.layout, token)?;
        }
        let mut sessions = VecDeque::new();
        fn prepare(
            node: &snapshot::Node,
            sessions: &mut VecDeque<Session>,
            token: &AtomicBool,
            spawn: &mut impl FnMut(&snapshot::TabSnapshot) -> anyhow::Result<Session>,
        ) -> anyhow::Result<()> {
            match node {
                snapshot::Node::Pane { tabs, .. } => {
                    for tab in tabs {
                        check_cancelled(token)?;
                        sessions.push_back(spawn(tab)?);
                        if sessions.back().unwrap().shell_history_id().is_none() {
                            sessions.back_mut().unwrap().set_shell_history_id(tab.shell_history_id.clone())?;
                        }
                    }
                }
                snapshot::Node::Split { children, .. } => {
                    for child in children {
                        prepare(child, sessions, token, spawn)?;
                    }
                }
            }
            Ok(())
        }
        let result = (|| {
            for workspace in &snapshot.workspaces {
                prepare(&workspace.layout, &mut sessions, token, spawn)?;
            }
            check_cancelled(token)
        })();
        if let Err(error) = result {
            // Startup callers may exit immediately on failure. Finish rollback
            // here instead of abandoning asynchronous reapers at process exit.
            let shutdowns: Vec<_> = sessions.iter().map(Session::shutdown_handle).collect();
            drop(sessions);
            for shutdown in shutdowns {
                futures_lite::future::block_on(shutdown.wait());
            }
            return Err(error);
        }
        Ok(Self { snapshot, sessions })
    }

    pub fn into_parts(self) -> (snapshot::WindowSnapshot, VecDeque<Session>) {
        (self.snapshot, self.sessions)
    }
}

pub struct PreparedSession {
    pub active_window: Option<usize>,
    pub windows: Vec<Prepared>,
}
fn default_directory(node: &mut snapshot::Node, directory: &std::path::Path) {
    match node {
        snapshot::Node::Pane { tabs, .. } => {
            for tab in tabs { if tab.directory.is_none() { tab.directory = Some(directory.to_owned()); } }
        }
        snapshot::Node::Split { children, .. } => {
            for child in children { default_directory(child, directory); }
        }
    }
}

impl PreparedSession {
    /// GUI import with bounded, caller-coalesced progress notifications.
    pub fn read_import(path: &std::path::Path, session: bool, directory: Option<&std::path::Path>, token: &AtomicBool, scrollback_bytes: usize,
        mut progress: impl FnMut(usize, usize)) -> anyhow::Result<Self> {
        check_cancelled(token)?;
        let mut snapshot = if session { snapshot::SessionSnapshot::load(path)? } else {
            snapshot::SessionSnapshot { version: 1, active_window: Some(0), windows: vec![snapshot::WindowSnapshot::load(path)?] }
        };
        anyhow::ensure!(!snapshot.windows.is_empty(), "session contains no windows");
        if let Some(directory) = directory {
            anyhow::ensure!(directory.is_absolute(), "preset working directory must be absolute");
            for window in &mut snapshot.windows {
                for workspace in &mut window.workspaces { default_directory(&mut workspace.layout, directory); }
            }
        }
        fn count(node: &snapshot::Node) -> usize {
            match node {
                snapshot::Node::Pane { tabs, .. } => tabs.len(),
                snapshot::Node::Split { children, .. } => children.iter().map(count).sum(),
            }
        }
        let total = snapshot.windows.iter().flat_map(|window| &window.workspaces).map(|workspace| count(&workspace.layout)).sum();
        let mut completed = 0;
        progress(completed, total);
        Self::prepare_with(snapshot, token, &mut |tab| {
            let session = shell_history::shell(80, 24, tab.directory.as_deref(), scrollback_bytes, tab.shell_history_id.as_deref())?;
            completed += 1;
            progress(completed, total);
            Ok(session)
        })
    }
    pub fn read(path: &std::path::Path, token: &AtomicBool, scrollback_bytes: usize) -> anyhow::Result<Self> {
        check_cancelled(token)?;
        let snapshot = snapshot::SessionSnapshot::load(path)?;
        Self::prepare_with(snapshot, token, &mut |tab| shell_history::shell(80, 24, tab.directory.as_deref(), scrollback_bytes, tab.shell_history_id.as_deref()))
    }
    fn prepare_with(snapshot: snapshot::SessionSnapshot, token: &AtomicBool, spawn: &mut impl FnMut(&snapshot::TabSnapshot) -> anyhow::Result<Session>) -> anyhow::Result<Self> {
        // Preflight every window before starting any shell.
        for window in &snapshot.windows {
            for workspace in &window.workspaces { preflight(&workspace.layout, token)?; }
        }
        let mut windows: Vec<Prepared> = Vec::new();
        for window in snapshot.windows {
            match Prepared::prepare_with(window, token, spawn) {
                Ok(prepared) => windows.push(prepared),
                Err(error) => {
                    let handles: Vec<_> = windows.iter().flat_map(|window| window.sessions.iter().map(Session::shutdown_handle)).collect();
                    drop(windows);
                    for handle in handles { futures_lite::future::block_on(handle.wait()); }
                    return Err(error);
                }
            }
        }
        Ok(Self { active_window: snapshot.active_window, windows })
    }
}

impl Workspace {
    pub fn restore_node(
        &mut self,
        node: snapshot::Node,
        sessions: &mut VecDeque<Session>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> PaneTree<Entity<PaneView>> {
        match node {
            snapshot::Node::Pane { active_tab, tabs } => {
                let mut terminals = Vec::with_capacity(tabs.len());
                let mut labels = Vec::with_capacity(tabs.len());
                for tab in tabs {
                    let session = sessions
                        .pop_front()
                        .expect("validated layout has a prepared shell per tab");
                    terminals.push(cx.new(|cx| {
                        let mut view = TerminalView::new(session, window, cx);
                        view.custom_title = tab.custom_title;
                        view.saved_ssh_destination = tab.ssh_destination;
                        if let Some(family) = tab.font_family {
                            view.font_family = family.into();
                        }
                        view.cells = CellMetrics::new(tab.font_size);
                        view
                    }));
                    labels.push(tab.label);
                }
                let workspace = cx.entity_id();
                let pane =
                    cx.new(|cx| PaneView::restored(terminals, labels, active_tab, workspace, cx));
                self.observe_pane(&pane, window, cx);
                self.panes.push(pane.clone());
                PaneTree::Pane(pane)
            }
            snapshot::Node::Split {
                axis,
                weights,
                children,
            } => PaneTree::Split {
                axis,
                weights,
                children: children
                    .into_iter()
                    .map(|node| self.restore_node(node, sessions, window, cx))
                    .collect(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Job, Prepared, cancelled};

    #[test]
    fn preset_default_directory_fills_nested_tabs_without_replacing_saved_paths() -> anyhow::Result<()> {
        let root = tempfile::tempdir()?;
        let project = root.path().join("project 世界");
        let explicit = root.path().join("other project");
        let mut node: super::snapshot::Node = serde_json::from_value(serde_json::json!({
            "kind":"split","axis":"Columns","weights":[1,1],"children":[
                {"kind":"pane","active_tab":0,"tabs":[{"label":"default","directory":null}]},
                {"kind":"pane","active_tab":0,"tabs":[{"label":"explicit","directory":explicit},{"label":"default2","directory":null}]}
            ]
        }))?;
        super::default_directory(&mut node, &project);
        let actual = serde_json::to_value(&node)?;
        assert_eq!(actual["children"][0]["tabs"][0]["directory"], serde_json::to_value(&project)?);
        assert_eq!(actual["children"][1]["tabs"][0]["directory"], serde_json::to_value(&explicit)?);
        assert_eq!(actual["children"][1]["tabs"][1]["directory"], serde_json::to_value(&project)?);
        super::default_directory(&mut node, &root.path().join("replacement"));
        assert_eq!(serde_json::to_value(&node)?, actual);
        Ok(())
    }

    #[test]
    fn import_progress_counts_terminals_and_allows_cancellation_before_spawn() -> anyhow::Result<()> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("import.json");
        let window = serde_json::json!({"version":1,"sidebar":true,"active_workspace":0,
            "workspaces":[{"name":"progress","active_pane":0,"layout":{"kind":"pane","active_tab":0,
                "tabs":[{"label":"first","directory":null},{"label":"second","directory":null}]}}]});
        for session in [false, true] {
            let document = if session { serde_json::json!({"version":1,"active_window":0,"windows":[window.clone(),window.clone()]}) } else { window.clone() };
            std::fs::write(&path, serde_json::to_vec(&document)?)?;
            let token = std::sync::atomic::AtomicBool::new(false);
            let mut progress = Vec::new();
            let result = super::PreparedSession::read_import(&path, session, None, &token, 4096, |completed, total| {
                progress.push((completed, total));
                token.store(true, std::sync::atomic::Ordering::Release);
            });
            assert!(result.err().expect("cancel before spawning").to_string().contains("cancelled"));
            assert_eq!(progress, vec![(0, if session { 4 } else { 2 })]);
        }
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn partial_session_failure_and_cancellation_reap_every_prepared_shell() -> anyhow::Result<()> {
        use super::{PreparedSession, Session, snapshot};
        use std::sync::atomic::{AtomicBool, Ordering};
        let window = serde_json::json!({"version":1,"sidebar":true,"active_workspace":0,
            "workspaces":[{"name":"rollback","active_pane":0,"layout":{"kind":"pane","active_tab":0,
                "tabs":[{"label":"first","directory":null},{"label":"second","directory":null}]}}]});
        let bytes = serde_json::to_vec(&serde_json::json!({"version":1,"active_window":0,"windows":[window.clone(),window]}))?;
        // Fail within the first window, within the second window, and cancel
        // after the first successful spawn. No environment overrides or mocks
        // replace the real PTY/session cleanup path.
        for (fail_at, cancel_after_first) in [(2, false), (4, false), (0, true)] {
            let token = AtomicBool::new(false);
            let mut calls = 0;
            let mut shutdowns = Vec::new();
            let result = PreparedSession::prepare_with(snapshot::SessionSnapshot::decode(&bytes)?, &token, &mut |_| {
                calls += 1;
                anyhow::ensure!(calls != fail_at, "injected later spawn failure");
                let session = Session::command_in(&["/bin/sh".into(), "-c".into(), "IFS= read -r line".into()], 20, 3, None)?;
                let handle = session.shutdown_handle();
                assert!(!handle.is_finished());
                shutdowns.push(handle);
                if cancel_after_first { token.store(true, Ordering::Release); }
                Ok(session)
            });
            let error = result.err().expect("preparation must fail");
            assert!(error.to_string().contains(if cancel_after_first { "cancelled" } else { "injected" }));
            assert_eq!(shutdowns.len(), if cancel_after_first { 1 } else { fail_at - 1 });
            assert!(shutdowns.iter().all(|handle| handle.is_finished()), "rollback returned before reaping prepared shells");
        }
        Ok(())
    }

    #[test]
    fn preflight_checks_nested_directories_and_cancellation() {
        use super::{preflight, snapshot};
        use crate::layout::Axis;
        use std::sync::atomic::AtomicBool;
        let directory = tempfile::tempdir().unwrap();
        let missing = directory.path().join("missing");
        let pane = |path| snapshot::Node::Pane {
            active_tab: 0,
            tabs: vec![snapshot::TabSnapshot {
                shell_history_id: None,
                ssh_destination: None,
                custom_title: None,
                font_family: None,
                label: String::new(),
                directory: path,
                font_size: 14.,
            }],
        };
        let node = snapshot::Node::Split {
            axis: Axis::Columns,
            weights: vec![1., 1.],
            children: vec![
                pane(Some(directory.path().into())),
                pane(Some(missing.clone())),
            ],
        };
        let token = AtomicBool::new(false);
        assert!(
            preflight(&node, &token)
                .unwrap_err()
                .to_string()
                .contains("missing")
        );
        std::fs::write(&missing, "regular file").unwrap();
        assert!(
            preflight(&node, &token)
                .unwrap_err()
                .to_string()
                .contains("not a directory")
        );
        std::fs::remove_file(&missing).unwrap();
        std::fs::create_dir(&missing).unwrap();
        preflight(&node, &token).unwrap();
        preflight(&pane(None), &token).unwrap();
        assert_eq!(
            preflight(&node, &AtomicBool::new(true))
                .unwrap_err()
                .to_string(),
            "layout restore cancelled"
        );
    }

    #[test]
    fn closing_owner_cancels_worker_before_file_access() {
        let job = Job::default();
        let token = job.0.clone();
        assert!(!cancelled(&token));
        drop(job);
        assert!(cancelled(&token));
        let directory = tempfile::tempdir().unwrap();
        let error = Prepared::read(&directory.path().join("missing.json"), &token, 0)
            .err()
            .expect("cancelled preparation must fail");
        assert_eq!(error.to_string(), "layout restore cancelled");
    }
}

/// Publish prepared windows as one UI-thread operation. Roll back only windows
/// created by this request if native window creation fails partway through.
pub(super) fn open_windows(session: PreparedSession, cx: &mut App) -> anyhow::Result<()> {
    let active = session.active_window.unwrap_or(0);
    let mut opened: Vec<WindowHandle<window_view::WorkspaceWindow>> = Vec::new();
    for (index, prepared) in session.windows.into_iter().enumerate() {
        let result = cx.open_window(
            WindowOptions {
                titlebar: workspace_titlebar(),
                focus: false,
                window_bounds: Some(prepared.snapshot.geometry.map(|geometry| geometry.restore(cx))
                    .unwrap_or_else(|| WindowBounds::centered(size(px(1000.), px(680.)), cx))),
                ..Default::default()
            },
            move |window, cx| cx.new(|cx| window_view::WorkspaceWindow::restored(prepared, window, cx)),
        );
        match result {
            Ok(handle) => opened.push(handle),
            Err(error) => {
                for handle in opened { let _ = handle.update(cx, |_, window, _| window.remove_window()); }
                return Err(error.into());
            }
        }
        debug_assert_eq!(opened.len(), index + 1);
    }
    if let Some(handle) = opened.get(active) {
        handle.update(cx, |_, window, _| window.activate_window())?;
        cx.activate(true);
    }
    Ok(())
}
