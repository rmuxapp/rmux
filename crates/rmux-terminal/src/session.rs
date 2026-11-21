//! PTY ownership and bounded, event-driven redraw notification.
use crate::{Frame, Terminal};
use anyhow::{Context, Result};
use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system};
use std::{
    io::{Read, Write},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc,
    },
    thread,
};

pub struct Session {
    initial_directory: Option<std::path::PathBuf>,
    remote_destination: Option<String>,
    child_pid: Option<u32>,
    direct_command: bool,
    _shell_integration: Option<tempfile::TempDir>,
    terminal: Arc<Mutex<Terminal>>,
    master: Box<dyn MasterPty + Send>,
    writer: mpsc::SyncSender<InputMessage>,
    input_errors: async_channel::Receiver<String>,
    paste_pending: Arc<AtomicBool>,
    paste_generation: Arc<AtomicU64>,
    pending_scroll: Mutex<Vec<crate::Scroll>>,
    report_all_keys: Arc<AtomicBool>,
    mouse_tracking: Arc<AtomicBool>,
    shutdown: ShutdownHandle,
    pub updates: async_channel::Receiver<()>,
    size: (u16, u16),
    exit: Arc<Mutex<Option<String>>>,
    output_bytes: Arc<AtomicU64>,
}

/// A one-shot background read; creating it does not lock or retain the session.
pub type HistoryExport = Box<dyn FnOnce() -> Result<Option<(String, bool)>> + Send>;
pub type HistoryProbe = Box<dyn FnOnce(&str, &std::sync::atomic::AtomicBool, std::time::Instant) -> Result<Option<bool>> + Send>;
pub type MemorySample = Box<dyn FnOnce() -> Result<Option<crate::MemoryUsage>> + Send>;

struct PastePermit {
    pending: Arc<AtomicBool>,
    generation: Arc<AtomicU64>,
    ticket: u64,
}
impl PastePermit {
    fn acquire(pending: &Arc<AtomicBool>, generation: &Arc<AtomicU64>) -> Result<Self> {
        let ticket = generation.load(Ordering::Acquire);
        anyhow::ensure!(
            pending.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire).is_ok(),
            "a paste is already pending for this terminal"
        );
        Ok(Self { pending: pending.clone(), generation: generation.clone(), ticket })
    }
    fn canceled(&self) -> bool {
        self.generation.load(Ordering::Acquire) != self.ticket
    }
}
impl Drop for PastePermit {
    fn drop(&mut self) {
        self.pending.store(false, Ordering::Release);
    }
}

enum InputPayload {
    Bytes(Vec<u8>),
    KeyPress { terminal: std::sync::Weak<Mutex<Terminal>>, key: String, mods: crate::keyboard::Modifiers, repeat: bool, plain_text: Option<String> },
    KeyRelease { terminal: std::sync::Weak<Mutex<Terminal>>, key: String, mods: crate::keyboard::Modifiers },
    Focus { terminal: std::sync::Weak<Mutex<Terminal>>, focused: bool, releases: Vec<String> },
    ClearScrollback { terminal: std::sync::Weak<Mutex<Terminal>> },
    Paste { text: String, bracketed: bool },
    NativePaste { terminal: std::sync::Weak<Mutex<Terminal>>, text: String },
    Mouse { terminal: std::sync::Weak<Mutex<Terminal>>, event: crate::mouse::Event },
}
impl InputPayload {
    fn encode_pending(self, paste: Option<&PastePermit>) -> Result<Option<Vec<u8>>> {
        if paste.is_some_and(PastePermit::canceled) { return Ok(None); }
        let payload = if let Self::NativePaste { terminal, text } = self {
            let Some(terminal) = terminal.upgrade() else { return Ok(None); };
            let engine = loop {
                if paste.is_some_and(PastePermit::canceled) { return Ok(None); }
                match terminal.try_lock() {
                    Ok(engine) => break engine,
                    Err(std::sync::TryLockError::Poisoned(error)) => break error.into_inner(),
                    Err(std::sync::TryLockError::WouldBlock) => thread::sleep(std::time::Duration::from_millis(1)),
                }
            };
            let bracketed = engine.bracketed_paste()?;
            drop(engine);
            Self::Paste { text, bracketed }
        } else { self };
        let bytes = payload.encode()?;
        // Cancellation ends at this boundary: write_all must preserve a whole
        // accepted payload, including bracketed-paste closing delimiters.
        if paste.is_some_and(PastePermit::canceled) { return Ok(None); }
        Ok(Some(bytes))
    }
    fn encode(self) -> Result<Vec<u8>> {
        match self {
            Self::Bytes(bytes) => Ok(bytes),
            Self::Mouse { terminal, event } => {
                let Some(terminal) = terminal.upgrade() else { return Ok(Vec::new()); };
                terminal.lock().unwrap_or_else(|error| error.into_inner()).encode_mouse(event).map(Option::unwrap_or_default)
            }
            Self::ClearScrollback { terminal } => {
                if let Some(terminal) = terminal.upgrade() {
                    terminal.lock().unwrap_or_else(|error| error.into_inner()).clear_scrollback()?;
                }
                Ok(Vec::new())
            }
            Self::KeyPress { terminal, key, mods, repeat, plain_text } => {
                let Some(terminal) = terminal.upgrade() else { return Ok(Vec::new()); };
                let mut terminal = terminal.lock().unwrap_or_else(|error| error.into_inner());
                // If report-all was disabled after admission, deliver committed
                // text normally instead of swallowing an already-handled key.
                Ok(terminal.encode_key_action_with_text(&key, mods,
                    if repeat { crate::keyboard::Action::Repeat } else { crate::keyboard::Action::Press },
                    plain_text.as_deref())?.unwrap_or_else(|| plain_text.unwrap_or_default().into_bytes()))
            }
            Self::KeyRelease { terminal, key, mods } => {
                let Some(terminal) = terminal.upgrade() else { return Ok(Vec::new()); };
                let mut terminal = terminal.lock().unwrap_or_else(|error| error.into_inner());
                Ok(terminal.encode_key_action(&key, mods, crate::keyboard::Action::Release)?.unwrap_or_default())
            }
            Self::Focus { terminal, focused, releases } => {
                let Some(terminal) = terminal.upgrade() else { return Ok(Vec::new()); };
                let mut terminal = terminal.lock().unwrap_or_else(|error| error.into_inner());
                let mut bytes = Vec::new();
                for key in releases {
                    if let Some(release) = terminal.encode_key_action(&key, crate::keyboard::Modifiers::default(), crate::keyboard::Action::Release)? {
                        bytes.extend(release);
                    }
                }
                if focused { terminal.acknowledge_program_status(); }
                if let Some(focus) = terminal.encode_focus(focused)? { bytes.extend(focus); }
                Ok(bytes)
            }
            Self::Paste { text, bracketed } => crate::keyboard::encode_paste(&text, bracketed),
            Self::NativePaste { .. } => unreachable!("native paste is resolved by encode_pending"),
        }
    }
}
struct InputMessage {
    payload: InputPayload,
    // Held through write_all, and released on encoding, enqueue or write failure.
    _paste: Option<PastePermit>,
}
impl From<Vec<u8>> for InputMessage {
    fn from(bytes: Vec<u8>) -> Self {
        Self { payload: InputPayload::Bytes(bytes), _paste: None }
    }
}

fn write_messages(writes: mpsc::Receiver<InputMessage>, mut writer: impl Write, mut report: impl FnMut(String), mut finished: impl FnMut()) {
    while let Ok(message) = writes.recv() {
        let notify = message._paste.is_some() || matches!(&message.payload, InputPayload::Focus { .. } | InputPayload::ClearScrollback { .. });
        let keep_writing = write_message(message, &mut writer, &mut report);
        // The permit must be released before waking a quiet terminal's UI.
        if notify { finished(); }
        if !keep_writing {
            drop(writes); // Release any paste queued behind the failed write.
            finished();
            break;
        }
    }
}

fn write_message(message: InputMessage, writer: &mut impl Write, report: &mut impl FnMut(String)) -> bool {
    let bytes = match message.payload.encode_pending(message._paste.as_ref()) {
        Ok(Some(bytes)) => bytes,
        Ok(None) => return true,
        Err(error) => { report(format!("Terminal input encoding failed: {error}")); return true; }
    };
    if let Err(error) = writer.write_all(&bytes) {
        report(format!("Terminal input failed: {error}"));
        return false;
    }
    true
}

/// Coherent metadata read under one engine lock. Unchanged titles allocate nothing;
/// a title retrieval error does not discard an independently delivered bell.
pub struct MetadataUpdate {
    pub render_delay: Option<std::time::Duration>,
    pub program_status_records: Option<Vec<crate::program_status::Record>>,
    pub program_status: Option<(u64, Option<crate::program_status::Summary>)>,
    pub progress: Option<crate::Progress>,
    pub bell_revision: u64,
    pub notifications: Vec<Arc<crate::DesktopNotification>>,
    pub title: Result<Option<(u64, String)>>,
    pub directory: Result<Option<(u64, Option<std::path::PathBuf>)>>,
    pub remote_directory: Result<Option<(u64, Option<String>)>>,
}

/// Does not retain the PTY or terminal; safe to retain while waiting for cleanup.
#[derive(Clone)]
pub struct ShutdownHandle {
    exit: Arc<Mutex<Option<String>>>,
    closing: Arc<AtomicBool>,
    paste_generation: Arc<AtomicU64>,
    completed: async_channel::Receiver<()>,
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    process_wakeup: crate::process_wakeup::ProcessWakeup,
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    killer: Arc<Mutex<Box<dyn portable_pty::ChildKiller + Send + Sync>>>,
}
impl ShutdownHandle {
    pub fn request(&self) -> Result<()> {
        // Cancel pending paste before touching the process, including when the
        // engine is busy or the UI retains this session after closing its pane.
        self.closing.store(true, Ordering::Release);
        self.paste_generation.fetch_add(1, Ordering::AcqRel);
        if self
            .exit
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_some()
        {
            return Ok(());
        }
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        self.process_wakeup.close()?;
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        self.killer
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .kill()?;
        Ok(())
    }
    pub fn is_finished(&self) -> bool {
        self.completed.is_closed()
    }
    pub async fn wait(&self) {
        let _ = self.completed.recv().await;
    }
}
// If creating the reaper thread fails, its closure is dropped. Keep ownership
// armed until that thread starts so a partially constructed session cannot
// abandon a live child. This cleanup runs only on startup failure.
struct UnstartedChild(Option<Box<dyn portable_pty::Child + Send + Sync>>);
impl UnstartedChild {
    fn take(&mut self) -> Box<dyn portable_pty::Child + Send + Sync> {
        self.0.take().expect("owned child")
    }
}
impl Drop for UnstartedChild {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
/// Local PTY input flags; raw mode alone is not evidence of password entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LocalInputMode { pub echo: bool, pub canonical: bool }

impl Session {
    /// Event-driven callers can sample local tty flags without locking the parser.
    /// ConPTY and remote tty modes are not inferred from output text.
    pub fn local_input_mode(&self) -> Option<LocalInputMode> {
        if self.exit_status().is_some() { return None; }
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        {
            let flags = self.master.get_termios()?.local_flags.bits();
            Some(LocalInputMode { echo: flags & libc::ECHO != 0, canonical: flags & libc::ICANON != 0 })
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        { None }
    }

    pub fn compressor(&self) -> crate::compression::Compressor {
        crate::compression::Compressor::new(Arc::downgrade(&self.terminal))
    }
    /// Create on demand; a search owns no PTY and keeps its terminal data alive
    /// only until the search worker is canceled/dropped.
    pub fn search(&self) -> Result<crate::search::Search> {
        crate::search::Search::new(self.terminal.clone())
    }
    pub fn search_worker(&self) -> Result<crate::search_worker::SearchWorker> {
        crate::search_worker::SearchWorker::new(self.terminal.clone())
    }
    pub fn spawn(command: CommandBuilder, columns: u16, rows: u16) -> Result<Self> {
        Self::spawn_with_scrollback(command, columns, rows, rmux_core::scroll::DEFAULT_SCROLLBACK_BYTES)
    }
    pub fn spawn_with_scrollback(
        command: CommandBuilder,
        columns: u16,
        rows: u16,
        scrollback_bytes: usize,
    ) -> Result<Self> {
        // portable-pty falls back to HOME for a missing Unix cwd. Reject an
        // invalid explicit target before allocating an engine or starting work.
        if let Some(directory) = command.get_cwd() {
            let path = std::path::Path::new(directory);
            let metadata = std::fs::metadata(path)
                .with_context(|| format!("working directory {}", path.display()))?;
            anyhow::ensure!(
                metadata.is_dir(),
                "working directory {} is not a directory",
                path.display()
            );
        }
        let initial_directory = command.get_cwd().map(std::path::absolute).transpose()?;
        let mut engine = Terminal::new(columns, rows)?;
        engine.set_scrollback_limit(scrollback_bytes)?;
        let report_all_keys = Arc::new(AtomicBool::new(engine.reports_all_keys()?));
        let reader_report_all_keys = report_all_keys.clone();
        let mouse_tracking = Arc::new(AtomicBool::new(engine.mouse_tracking()?));
        let reader_mouse_tracking = mouse_tracking.clone();
        let terminal = Arc::new(Mutex::new(engine));
        let pair = native_pty_system().openpty(PtySize {
            rows,
            cols: columns,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        let mut reader = pair.master.try_clone_reader()?;
        let pty_writer = pair.master.take_writer()?;
        let child = pair.slave.spawn_command(command).context("start process")?;
        let child_pid = child.process_id();
        let mut unstarted = UnstartedChild(Some(child));
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        let killer = unstarted.0.as_ref().unwrap().clone_killer();
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        let process_wakeup = crate::process_wakeup::ProcessWakeup::new(
            unstarted
                .0
                .as_ref()
                .unwrap()
                .process_id()
                .context("missing child PID")?,
        )?;
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        let reaper_wakeup = process_wakeup.clone();
        drop(pair.slave);
        let (notify, updates) = async_channel::bounded(1);
        let exit = Arc::new(Mutex::new(None));
        let child_exit = exit.clone();
        let exit_terminal = terminal.clone();
        let exit_notify = notify.clone();
        let read_terminal = terminal.clone();
        let output_bytes = Arc::new(AtomicU64::new(0));
        let read_bytes = output_bytes.clone();
        let (writer, writes) = mpsc::sync_channel::<InputMessage>(32);
        let (input_error, input_errors) = async_channel::bounded(1);
        let input_notify = notify.clone();
        let (completion, completed) = async_channel::bounded(1);
        let reply_writer = writer.clone();
        let paste_generation = Arc::new(AtomicU64::new(0));
        let session = Self {
            initial_directory,
            remote_destination: None,
            child_pid,
            direct_command: false,
            _shell_integration: None,
            terminal,
            master: pair.master,
            writer,
            input_errors,
            paste_pending: Arc::new(AtomicBool::new(false)),
            paste_generation: paste_generation.clone(),
            pending_scroll: Mutex::new(Vec::new()),
            shutdown: ShutdownHandle {
                exit: exit.clone(),
                closing: Arc::new(AtomicBool::new(false)),
                paste_generation,
                completed,
                #[cfg(not(any(target_os = "macos", target_os = "linux")))]
                killer: Arc::new(Mutex::new(killer)),
                #[cfg(any(target_os = "macos", target_os = "linux"))]
                process_wakeup,
            },
            updates,
            size: (columns, rows),
            exit,
            output_bytes,
            report_all_keys,
            mouse_tracking,
        };
        let reaper = thread::Builder::new()
            .name("rmux-pty-reap".into())
            .spawn(move || {
                let _completion = completion;
                let mut child = unstarted.take();
                #[cfg(any(target_os = "macos", target_os = "linux"))]
                if reaper_wakeup.wait().unwrap_or(true) {
                    // This owner alone may reap or terminate the child, avoiding
                    // delayed signals targeting a PID that has been reused.
                    if !matches!(child.try_wait(), Ok(Some(_))) {
                        let _ = child.kill();
                    }
                }
                let message = match child.wait() {
                    Ok(status) => match status.signal() {
                        Some(signal) => format!("Process exited: {signal}"),
                        None => format!("Process exited with status {}", status.exit_code()),
                    },
                    Err(error) => format!("Unable to wait for process: {error}"),
                };
                exit_terminal
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .finish_program_status();
                *child_exit.lock().unwrap_or_else(|e| e.into_inner()) = Some(message);
                let _ = exit_notify.try_send(());
            });
        if let Err(error) = reaper {
            // The dropped closure's guard already terminated/reaped the child.
            // Avoid signaling that child's former PID again from Session::drop.
            *session.exit.lock().unwrap_or_else(|e| e.into_inner()) =
                Some("Shell startup failed".into());
            return Err(error.into());
        }
        let mode_error = input_error.clone();
        thread::Builder::new()
            .name("rmux-pty-write".into())
            .spawn(move || {
                write_messages(writes, pty_writer, |error| {
                    let _ = input_error.try_send(error);
                    let _ = input_notify.try_send(());
                }, || { let _ = input_notify.try_send(()); });
            })?;
        thread::Builder::new()
            .name("rmux-pty-read".into())
            .spawn(move || {
                let mut bytes = [0; 16 * 1024];
                loop {
                    match reader.read(&mut bytes) {
                        Ok(0) => break,
                        Ok(count) => {
                            let replies = {
                                let mut terminal =
                                    read_terminal.lock().unwrap_or_else(|e| e.into_inner());
                                terminal.write(&bytes[..count]);
                                match terminal.reports_all_keys() {
                                    Ok(enabled) => reader_report_all_keys.store(enabled, Ordering::Release),
                                    Err(error) => { let _ = mode_error.try_send(format!("Keyboard mode query failed: {error}")); }
                                }
                                match terminal.mouse_tracking() {
                                    Ok(enabled) => reader_mouse_tracking.store(enabled, Ordering::Release),
                                    Err(error) => { let _ = mode_error.try_send(format!("Mouse mode query failed: {error}")); }
                                }
                                terminal.take_replies()
                            };
                            read_bytes.fetch_add(count as u64, Ordering::Relaxed);
                            if !replies.is_empty() {
                                if reply_writer.send(replies.into()).is_err() {
                                    break;
                                }
                            }
                            // Coalesce notifications, never terminal bytes. No idle timer.
                            let _ = notify.try_send(());
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                        Err(_) => break,
                    }
                }
                read_terminal
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .finish_program_status();
                let _ = notify.try_send(());
            })?;
        Ok(session)
    }
    pub fn shell(columns: u16, rows: u16) -> Result<Self> {
        Self::shell_in(columns, rows, None)
    }
    pub fn shell_in(columns: u16, rows: u16, directory: Option<&std::path::Path>) -> Result<Self> {
        Self::shell_in_with_scrollback(columns, rows, directory, rmux_core::scroll::DEFAULT_SCROLLBACK_BYTES)
    }
    pub fn shell_in_with_scrollback(
        columns: u16,
        rows: u16,
        directory: Option<&std::path::Path>,
        scrollback_bytes: usize,
    ) -> Result<Self> {
        let shell = select_shell(
            std::env::var_os("RMUX_SHELL"),
            std::env::var_os(if cfg!(windows) { "COMSPEC" } else { "SHELL" }),
        )?;
        let mut command = CommandBuilder::new(&shell);
        if let Some(directory) = directory {
            command.cwd(directory);
        }
        let login = std::env::var_os("RMUX_SHELL_LOGIN").as_deref() != Some(std::ffi::OsStr::new("0"));
        command.env("TERM", "xterm-256color");
        command.env("COLORTERM", "truecolor");
        command.env("TERM_PROGRAM", "rmux");
        let powershell = is_powershell(&shell);
        let integration = if powershell
            && std::env::var_os("RMUX_SHELL_INTEGRATION").as_deref()
                != Some(std::ffi::OsStr::new("0"))
        {
            Some(crate::shell_integration::prepare_powershell(
                &mut command,
                login,
            )?)
        } else {
            #[cfg(unix)]
            {
                if std::path::Path::new(&shell).file_name() == Some(std::ffi::OsStr::new("zsh"))
                    && std::env::var_os("RMUX_SHELL_INTEGRATION").as_deref()
                        != Some(std::ffi::OsStr::new("0"))
                {
                    Some(crate::shell_integration::prepare_zsh(
                        &mut command,
                        std::env::var_os("ZDOTDIR").as_deref(),
                    )?)
                } else if !login
                    && std::path::Path::new(&shell).file_name()
                        == Some(std::ffi::OsStr::new("bash"))
                    && std::env::var_os("RMUX_SHELL_INTEGRATION").as_deref()
                        != Some(std::ffi::OsStr::new("0"))
                {
                    Some(crate::shell_integration::prepare_bash(&mut command)?)
                } else if std::path::Path::new(&shell).file_name()
                    == Some(std::ffi::OsStr::new("fish"))
                    && std::env::var_os("RMUX_SHELL_INTEGRATION").as_deref()
                        != Some(std::ffi::OsStr::new("0"))
                {
                    Some(crate::shell_integration::prepare_fish(&mut command)?)
                } else {
                    None
                }
            }
            #[cfg(not(unix))]
            {
                None
            }
        };
        if !cfg!(windows) && !(powershell && integration.is_some()) {
            command.args(unix_shell_flags(&shell, login));
        }
        let mut session = Self::spawn_with_scrollback(command, columns, rows, scrollback_bytes)?;
        session._shell_integration = integration;
        Ok(session)
    }
    /// Execute an argv vector directly; no shell quoting or command evaluation.
    pub fn command_in(
        argv: &[std::ffi::OsString],
        columns: u16,
        rows: u16,
        directory: Option<&std::path::Path>,
    ) -> Result<Self> {
        Self::command_in_with_scrollback(argv, columns, rows, directory, rmux_core::scroll::DEFAULT_SCROLLBACK_BYTES)
    }
    pub fn command_in_with_scrollback(
        argv: &[std::ffi::OsString],
        columns: u16,
        rows: u16,
        directory: Option<&std::path::Path>,
        scrollback_bytes: usize,
    ) -> Result<Self> {
        Self::command_in_with_environment(argv, columns, rows, directory, scrollback_bytes, &[])
    }
    pub fn command_in_with_environment(
        argv: &[std::ffi::OsString], columns: u16, rows: u16,
        directory: Option<&std::path::Path>, scrollback_bytes: usize,
        environment: &[(std::ffi::OsString, std::ffi::OsString)],
    ) -> Result<Self> {
        let program = argv
            .first()
            .filter(|program| !program.is_empty())
            .ok_or_else(|| anyhow::anyhow!("command requires a nonempty program name"))?;
        let mut command = CommandBuilder::new(program);
        command.args(&argv[1..]);
        if let Some(directory) = directory {
            command.cwd(directory);
        }
        command.env("TERM", "xterm-256color");
        command.env("COLORTERM", "truecolor");
        command.env("TERM_PROGRAM", "rmux");
        for (name, value) in environment { command.env(name, value); }
        let mut session = Self::spawn_with_scrollback(command, columns, rows, scrollback_bytes)?;
        session.direct_command = true;
        Ok(session)
    }
    pub fn current_directory(&self) -> Result<Option<std::path::PathBuf>> {
        if self.remote_destination.is_some() { return Ok(None); }
        self.terminal
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .current_directory()
    }
    /// Explicit spawn directory, without consulting the terminal parser.
    pub fn initial_directory(&self) -> Option<&std::path::Path> {
        self.initial_directory.as_deref()
    }
    /// Mark an explicitly remote process before handing it to a UI view.
    /// Its OSC paths must never drive local filesystem access or shell cwd.
    pub fn set_remote_destination(&mut self, destination: &str) -> Result<()> {
        anyhow::ensure!(!destination.is_empty() && destination.len() <= 255 && !destination.chars().any(char::is_control), "Invalid remote destination label");
        self.remote_destination = Some(destination.to_owned());
        self.initial_directory = None;
        Ok(())
    }
    /// Launch identity only; does not imply successful remote authentication.
    pub fn remote_destination(&self) -> Option<&str> { self.remote_destination.as_deref() }
    /// A predictable idle workload, used only by the opt-in performance harness.
    pub fn benchmark_shell(columns: u16, rows: u16) -> Result<Self> {
        #[cfg(unix)]
        let mut command = CommandBuilder::new("/bin/sh");
        #[cfg(windows)]
        let mut command = CommandBuilder::new("cmd.exe");
        #[cfg(unix)]
        {
            command.arg("-i");
            command.env("ENV", "/dev/null");
            command.env("PS1", "$ ");
        }
        #[cfg(windows)]
        command.arg("/d");
        command.env("TERM", "xterm-256color");
        command.env("COLORTERM", "truecolor");
        Self::spawn(command, columns, rows)
    }
    pub fn input(&self, bytes: &[u8]) -> Result<()> {
        self.write_input(bytes, true)
    }
    /// Targeted automation input preserves the viewport and never locks the engine.
    pub fn input_without_scroll(&self, bytes: &[u8]) -> Result<()> {
        self.write_input(bytes, false)
    }
    pub fn shutdown_handle(&self) -> ShutdownHandle {
        self.shutdown.clone()
    }
    fn write_input(&self, bytes: &[u8], follow_output: bool) -> Result<()> {
        anyhow::ensure!(self.exit_status().is_none(), "process has exited");
        // Backpressure is explicit rather than blocking keyboard dispatch. Each
        // accepted message is delivered in order by the sole writer thread.
        anyhow::ensure!(
            bytes.len() <= 64 * 1024,
            "input exceeds the prototype's 64 KiB limit"
        );
        self.writer
            .try_send(bytes.to_vec().into())
            .map_err(|_| anyhow::anyhow!("terminal input queue is full or closed"))?;
        if follow_output && !bytes.is_empty() {
            self.follow_output()?;
        }
        Ok(())
    }
    /// Decide local handling from the last parsed mode without a parser wait.
    /// Application reports use the existing bounded, ordered input worker.
    pub fn queue_mouse(&self, event: crate::mouse::Event) -> Result<bool> {
        if !self.mouse_tracking.load(Ordering::Acquire) { return Ok(false); }
        self.writer.try_send(InputMessage {
            payload: InputPayload::Mouse { terminal: Arc::downgrade(&self.terminal), event },
            _paste: None,
        }).map_err(|_| anyhow::anyhow!("terminal input queue is full or closed"))?;
        Ok(true)
    }
    pub fn mouse(&self, event: crate::mouse::Event) -> Result<bool> {
        let bytes = self
            .terminal
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .encode_mouse(event)?;
        if let Some(bytes) = bytes {
            if !bytes.is_empty() {
                self.write_input(&bytes, false)?;
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }
    /// Input supersedes queued navigation and follows output without parser waits.
    fn follow_output(&self) -> Result<()> {
        self.scroll(crate::Scroll::Bottom)
    }
    fn apply_pending_scroll(&self, terminal: &mut Terminal) -> Result<()> {
        let mut pending = self.pending_scroll.lock().unwrap_or_else(|error| error.into_inner());
        // Only viewport operations hold this small queue lock, never parsing or
        // PTY I/O. Retain its bounded allocation across gestures.
        for scroll in pending.drain(..) { terminal.scroll(scroll)?; }
        Ok(())
    }
    /// Apply immediately when idle, otherwise before the next coherent frame.
    /// Opposite directions remain ordered because viewport edges clamp each step.
    pub fn scroll(&self, scroll: crate::Scroll) -> Result<()> {
        {
            let mut pending = self.pending_scroll.lock().unwrap_or_else(|error| error.into_inner());
            if matches!(scroll, crate::Scroll::Top | crate::Scroll::Bottom | crate::Scroll::Row(_) | crate::Scroll::LastPrompt) {
                pending.clear();
            }
            let merged = match (pending.last_mut(), scroll) {
                (Some(crate::Scroll::Lines(previous)), crate::Scroll::Lines(lines)) if previous.signum() == lines.signum() => {
                    *previous = previous.saturating_add(lines);
                    true
                }
                _ => false,
            };
            if !merged {
                anyhow::ensure!(pending.len() < 64, "terminal scroll queue is full; retry navigation");
                pending.push(scroll);
            }
        }
        let mut terminal = match self.terminal.try_lock() {
            Ok(terminal) => terminal,
            Err(std::sync::TryLockError::Poisoned(error)) => error.into_inner(),
            Err(std::sync::TryLockError::WouldBlock) => return Ok(()),
        };
        self.apply_pending_scroll(&mut terminal)
    }
    /// Queue focus reporting/acknowledgement without waiting for the parser.
    /// The writer wakes metadata observers after processing even if focus
    /// reporting is disabled and therefore produces no PTY bytes.
    pub fn queue_focus(&self, focused: bool) -> Result<()> {
        self.queue_focus_with_releases(focused, Vec::new())
    }
    /// Reserve one queue slot for all held-key releases followed by focus change.
    pub fn queue_focus_with_releases(&self, focused: bool, releases: Vec<String>) -> Result<()> {
        anyhow::ensure!(releases.len() <= 256 && releases.iter().all(|key| key.len() <= 128), "too many held keys or oversized key name");
        self.writer.try_send(InputMessage {
            payload: InputPayload::Focus { terminal: Arc::downgrade(&self.terminal), focused, releases },
            _paste: None,
        }).map_err(|_| anyhow::anyhow!("terminal input queue is full or closed"))
    }
    pub fn focus(&self, focused: bool) -> Result<()> {
        let bytes = {
            let mut terminal = self.terminal.lock().unwrap_or_else(|e| e.into_inner());
            if focused {
                terminal.acknowledge_program_status();
            }
            terminal.encode_focus(focused)?
        };
        if let Some(bytes) = bytes {
            self.write_input(&bytes, false)?;
        }
        Ok(())
    }
    pub fn key(&self, key: &str, mods: crate::keyboard::Modifiers, repeat: bool) -> Result<bool> {
        self.key_with_text(key, mods, repeat, None)
    }
    pub fn key_with_text(
        &self,
        key: &str,
        mods: crate::keyboard::Modifiers,
        repeat: bool,
        text: Option<&str>,
    ) -> Result<bool> {
        if crate::keyboard::key_identity(key).is_none() { return Ok(false); }
        let native_text_key = (key.chars().count() == 1 || key == "space") && !mods.control && !mods.alt;
        // The PTY reader publishes this decision after every parsed output
        // batch. Ordinary native text/IME must not wait behind a busy parser.
        if native_text_key && !self.report_all_keys.load(Ordering::Acquire) { return Ok(false); }
        anyhow::ensure!(self.exit_status().is_none(), "process has exited");
        anyhow::ensure!(key.len() <= 128 && text.is_none_or(|text| text.len() <= 4096), "oversized key name or text");
        let plain_text = native_text_key.then(|| text.unwrap_or(rmux_core::keyboard::key_text(key)).to_owned());
        self.writer.try_send(InputMessage {
            payload: InputPayload::KeyPress { terminal: Arc::downgrade(&self.terminal), key: key.into(), mods, repeat, plain_text },
            _paste: None,
        }).map_err(|_| anyhow::anyhow!("terminal input queue is full or closed"))?;
        self.follow_output()?;
        Ok(true)
    }
    pub fn paste(&self, text: &str) -> Result<()> {
        anyhow::ensure!(text.len() <= 1024 * 1024, "paste exceeds 1 MiB input limit");
        self.paste_input(std::borrow::Cow::Borrowed(text))
    }
    pub fn take_input_error(&self) -> Option<String> {
        self.input_errors.try_recv().ok()
    }
    pub fn paste_pending(&self) -> bool {
        self.paste_pending.load(Ordering::Acquire)
    }
    pub fn cancel_queued_paste(&self) {
        if self.paste_pending.load(Ordering::Acquire) {
            self.paste_generation.fetch_add(1, Ordering::AcqRel);
        }
    }
    /// Reserve queue order before encoding on the sole PTY writer thread.
    pub fn paste_owned(&self, text: String) -> Result<()> {
        anyhow::ensure!(text.len() <= 1024 * 1024, "paste exceeds 1 MiB input limit");
        self.paste_input(std::borrow::Cow::Owned(text))
    }
    fn paste_input(&self, text: std::borrow::Cow<'_, str>) -> Result<()> {
        anyhow::ensure!(self.exit_status().is_none(), "process has exited");
        let permit = PastePermit::acquire(&self.paste_pending, &self.paste_generation)?;
        anyhow::ensure!(!self.shutdown.closing.load(Ordering::Acquire), "terminal is closing");
        self.writer.try_send(InputMessage {
            payload: InputPayload::NativePaste { terminal: Arc::downgrade(&self.terminal), text: text.into_owned() },
            _paste: Some(permit),
        }).map_err(|_| anyhow::anyhow!("terminal input queue is full or closed"))?;
        self.follow_output()
    }
    /// Bounded automation paste. Never waits for parser access or changes viewport.
    /// False means the parser is busy and no input was queued.
    pub fn try_paste_without_scroll(&self, text: &str) -> Result<bool> {
        anyhow::ensure!(text.len() <= 1024 * 1024, "automation paste exceeds 1 MiB");
        self.queue_paste(std::borrow::Cow::Borrowed(text))
    }
    fn queue_paste(&self, text: std::borrow::Cow<'_, str>) -> Result<bool> {
        anyhow::ensure!(self.exit_status().is_none(), "process has exited");
        let permit = PastePermit::acquire(&self.paste_pending, &self.paste_generation)?;
        anyhow::ensure!(!self.shutdown.closing.load(Ordering::Acquire), "terminal is closing");
        let bracketed = {
            let terminal = match self.terminal.try_lock() {
                Ok(terminal) => terminal,
                Err(std::sync::TryLockError::Poisoned(error)) => error.into_inner(),
                Err(std::sync::TryLockError::WouldBlock) => return Ok(false),
            };
            terminal.bracketed_paste()?
        };
        // Borrowed input is copied only after admission; owned clipboard/image
        // paths move their existing allocation. No parser guard spans the copy.
        self.writer.try_send(InputMessage { payload: InputPayload::Paste { text: text.into_owned(), bracketed }, _paste: Some(permit) })
            .map_err(|_| anyhow::anyhow!("terminal input queue is full or closed"))?;
        Ok(true)
    }

    /// Atomically enqueue a key press/release pair without parser waits or scrolling.
    pub fn try_key_tap_without_scroll(&self, key: &str, mods: crate::keyboard::Modifiers) -> Result<Option<usize>> {
        anyhow::ensure!(self.exit_status().is_none(), "process has exited");
        let bytes = {
            let mut terminal = match self.terminal.try_lock() {
                Ok(terminal) => terminal,
                Err(std::sync::TryLockError::Poisoned(error)) => error.into_inner(),
                Err(std::sync::TryLockError::WouldBlock) => return Ok(None),
            };
            terminal.encode_key_tap(key, mods)?
        };
        let count = bytes.len();
        self.input_without_scroll(&bytes)?;
        Ok(Some(count))
    }

    /// Release a delivered key without parser waits or viewport scrolling.
    pub fn queue_key_release(&self, key: &str, mods: crate::keyboard::Modifiers) -> Result<()> {
        anyhow::ensure!(key.len() <= 128, "oversized key name");
        self.writer.try_send(InputMessage {
            payload: InputPayload::KeyRelease { terminal: Arc::downgrade(&self.terminal), key: key.into(), mods },
            _paste: None,
        }).map_err(|_| anyhow::anyhow!("terminal input queue is full or closed"))
    }
    pub fn key_release(&self, key: &str, mods: crate::keyboard::Modifiers) -> Result<()> {
        let bytes = self
            .terminal
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .encode_key_action(key, mods, crate::keyboard::Action::Release)?;
        if let Some(bytes) = bytes.filter(|bytes| !bytes.is_empty()) {
            // Releasing a key must not pull a user out of scrollback.
            self.write_input(&bytes, false)?;
        }
        Ok(())
    }
    pub fn frame(&self) -> Result<Frame> {
        let mut terminal = self.terminal.lock().unwrap_or_else(|e| e.into_inner());
        self.apply_pending_scroll(&mut terminal)?;
        terminal.frame()
    }
    pub fn try_frame(&self) -> Result<Option<Frame>> {
        let mut terminal = match self.terminal.try_lock() {
            Ok(terminal) => terminal,
            Err(std::sync::TryLockError::Poisoned(error)) => error.into_inner(),
            Err(std::sync::TryLockError::WouldBlock) => return Ok(None),
        };
        self.apply_pending_scroll(&mut terminal)?;
        terminal.frame().map(Some)
    }
    pub fn history_export(&self, max_bytes: usize) -> HistoryExport {
        self.text_export(max_bytes, false, true, None)
    }
    pub fn history_probe(&self) -> HistoryProbe {
        let terminal = Arc::downgrade(&self.terminal);
        Box::new(move |query, canceled, deadline| {
            let terminal = terminal.upgrade().context("terminal closed before history search")?;
            crate::search::Search::probe(terminal, query, canceled, deadline)
        })
    }
    pub fn history_tail_export(&self, lines: usize, max_bytes: usize) -> HistoryExport {
        self.text_export(max_bytes, false, true, Some(lines))
    }
    pub fn selection_export(&self, max_bytes: usize, unwrap: bool) -> HistoryExport {
        self.text_export(max_bytes, true, unwrap, None)
    }
    /// Bounded clipboard formatting for a background executor. Construction never
    /// acquires the parser lock or keeps a closed terminal alive.
    pub fn clipboard_export(&self, unwrap: bool, current: impl Fn() -> bool + Send + 'static) -> Box<dyn FnOnce() -> Result<Option<String>> + Send> {
        let terminal = Arc::downgrade(&self.terminal);
        Box::new(move || {
            if !current() { return Ok(None); }
            let terminal = terminal.upgrade().context("terminal closed before copying")?;
            let engine = loop {
                match terminal.try_lock() {
                    Ok(engine) => break engine,
                    Err(std::sync::TryLockError::Poisoned(error)) => break error.into_inner(),
                    Err(std::sync::TryLockError::WouldBlock) => {
                        if !current() { return Ok(None); }
                        // Only pending copies poll; there is no idle timer.
                        thread::sleep(std::time::Duration::from_millis(1));
                    }
                }
            };
            // Another clipboard write may have superseded us while waiting.
            if !current() { return Ok(None); }
            engine.selected_text_with_wraps(16 * 1024 * 1024, unwrap)
        })
    }
    /// Deferred directory lookup without retaining the terminal while queued.
    pub fn directory_lookup(&self, current: impl Fn() -> bool + Send + 'static) -> Box<dyn FnOnce() -> Result<Option<std::path::PathBuf>> + Send> {
        if self.remote_destination.is_some() { return Box::new(|| Ok(None)); }
        let terminal = Arc::downgrade(&self.terminal);
        Box::new(move || {
            if !current() { return Ok(None); }
            let terminal = terminal.upgrade().context("terminal closed before directory lookup")?;
            let engine = loop {
                match terminal.try_lock() {
                    Ok(engine) => break engine,
                    Err(std::sync::TryLockError::Poisoned(error)) => break error.into_inner(),
                    Err(std::sync::TryLockError::WouldBlock) => {
                        if !current() { return Ok(None); }
                        thread::sleep(std::time::Duration::from_millis(1));
                    }
                }
            };
            if !current() { return Ok(None); }
            engine.current_directory()
        })
    }
    fn text_export(&self, max_bytes: usize, selected: bool, unwrap: bool, lines: Option<usize>) -> HistoryExport {
        let terminal = Arc::downgrade(&self.terminal);
        Box::new(move || {
            let terminal = terminal.upgrade().context("terminal closed before history export")?;
            let engine = match terminal.try_lock() {
                Ok(engine) => engine,
                Err(std::sync::TryLockError::Poisoned(error)) => error.into_inner(),
                Err(std::sync::TryLockError::WouldBlock) => return Ok(None),
            };
            if selected { engine.selection_text_prefix_with_wraps(max_bytes, unwrap).map(|text| Some(text.unwrap_or_default())) }
            else if let Some(lines) = lines { engine.history_tail_limited(lines, max_bytes).map(Some) }
            else { engine.history_text_limited(max_bytes).map(Some) }
        })
    }
    pub fn selection_event(&self, event: crate::selection::Event) -> Result<bool> {
        self.terminal
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .selection_event(event)
    }
    pub fn select_all(&self) -> Result<()> {
        self.terminal
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .select_all()
    }
    pub fn clear_scrollback(&self) -> Result<()> {
        self.terminal
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clear_scrollback()
    }
    /// Admit a history-clear command without acquiring the parser on the caller.
    /// The existing writer serializes it with input and wakes the UI on completion.
    pub fn queue_clear_scrollback(&self) -> Result<()> {
        self.writer.try_send(InputMessage {
            payload: InputPayload::ClearScrollback { terminal: Arc::downgrade(&self.terminal) },
            _paste: None,
        }).map_err(|_| anyhow::anyhow!("terminal input queue is full or closed"))
    }
    pub fn memory_usage(&self) -> Result<crate::MemoryUsage> {
        self.terminal
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .memory_usage()
    }
    /// Create a one-shot background sample without retaining the engine.
    pub fn memory_sample(&self) -> MemorySample {
        let terminal = Arc::downgrade(&self.terminal);
        Box::new(move || {
            let terminal = terminal.upgrade().context("terminal closed before memory sample")?;
            let engine = match terminal.try_lock() {
                Ok(engine) => engine,
                Err(std::sync::TryLockError::Poisoned(error)) => error.into_inner(),
                Err(std::sync::TryLockError::WouldBlock) => return Ok(None),
            };
            engine.memory_usage().map(Some)
        })
    }
    /// Opportunistic diagnostics must not wait behind parsing or engine work.
    pub fn try_memory_usage(&self) -> Result<Option<crate::MemoryUsage>> {
        let terminal = match self.terminal.try_lock() {
            Ok(terminal) => terminal,
            Err(std::sync::TryLockError::Poisoned(error)) => error.into_inner(),
            Err(std::sync::TryLockError::WouldBlock) => return Ok(None),
        };
        terminal.memory_usage().map(Some)
    }
    pub fn selected_text(&self) -> Result<Option<String>> {
        self.terminal
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .selected_text()
    }
    /// A small single-line selection can seed Find without copying a large selection.
    pub fn selected_search_query(&self) -> Option<String> {
        // Find can fall back to its remembered query while output is being parsed.
        self.terminal
            .try_lock()
            .ok()?
            // Streaming stops at the limit; a size-probing formatter would
            // still walk the entire selection while holding the parser lock.
            .selection_text_prefix(4096)
            .ok()
            .flatten()
            .filter(|(_, truncated)| !truncated)
            .map(|(text, _)| text)
            .filter(|text| !text.is_empty() && !text.chars().any(char::is_control))
    }
    pub fn hyperlink_at(
        &self,
        column: u16,
        row: u32,
        displayed_revision: u64,
    ) -> Result<Option<String>> {
        let mut terminal = match self.terminal.try_lock() {
            Ok(terminal) => terminal,
            Err(std::sync::TryLockError::Poisoned(error)) => error.into_inner(),
            Err(std::sync::TryLockError::WouldBlock) => anyhow::bail!("Terminal is busy; click the link again to retry."),
        };
        if terminal.revision != displayed_revision {
            return Ok(None);
        }
        terminal.link_at(column, row)
    }
    pub fn program_status_records(&self) -> Vec<crate::program_status::Record> {
        let terminal = self.terminal.lock().unwrap_or_else(|e| e.into_inner());
        Self::snapshot_program_status(&terminal)
    }
    pub fn try_program_status_records(&self) -> Option<Vec<crate::program_status::Record>> {
        let terminal = match self.terminal.try_lock() {
            Ok(terminal) => terminal,
            Err(std::sync::TryLockError::Poisoned(error)) => error.into_inner(),
            Err(std::sync::TryLockError::WouldBlock) => return None,
        };
        Some(Self::snapshot_program_status(&terminal))
    }
    fn snapshot_program_status(terminal: &Terminal) -> Vec<crate::program_status::Record> {
        let records = terminal.program_status();
        let mut snapshot: Vec<_> = records
            .iter()
            .map(|record| {
                let mut record = record.clone();
                record.app = records.effective_app(&record.id).unwrap_or("").to_owned();
                record
            })
            .collect();
        snapshot.sort_by(|left, right| left.id.cmp(&right.id));
        snapshot
    }
    pub fn set_default_colors(&self, colors: [Option<crate::Color>; 3]) -> Result<()> {
        self.terminal.lock().unwrap_or_else(|e| e.into_inner()).set_default_colors(colors)
    }
    pub fn set_palette(&self, colors: Option<[crate::Color; 256]>) -> Result<()> {
        self.terminal.lock().unwrap_or_else(|e| e.into_inner()).set_palette(colors)
    }
    pub fn set_ansi_palette(&self, colors: Option<[crate::Color; 16]>) -> Result<()> {
        self.terminal.lock().unwrap_or_else(|e| e.into_inner()).set_ansi_palette(colors)
    }
    pub fn set_scrollback_limit(&self, bytes: usize) -> Result<()> {
        self.terminal
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .set_scrollback_limit(bytes)
    }
    /// Boolean query for close warnings without copying or sorting status text.
    pub fn reports_active_work(&self) -> bool {
        if self
            .exit
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_some()
        {
            return false;
        }
        self.terminal
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .program_status()
            .has_active_work()
    }
    /// On-demand close-warning check. Unix job control identifies a foreground
    /// group distinct from the owning shell; directly launched programs are work
    /// for their entire lifetime. No process-table scans or polling are needed.
    /// Contended state conservatively requires confirmation without blocking.
    pub fn has_active_work(&self) -> bool {
        // Close actions run on the UI thread. Contention means unknown, so keep
        // confirmation enabled instead of waiting or treating a busy session as idle.
        let exited = match self.exit.try_lock() {
            Ok(exit) => exit.is_some(),
            Err(std::sync::TryLockError::Poisoned(error)) => error.into_inner().is_some(),
            Err(std::sync::TryLockError::WouldBlock) => return true,
        };
        if exited { return false; }
        if self.direct_command {
            return true;
        }
        #[cfg(unix)]
        if let (Some(child), Some(group)) = (self.child_pid, self.master.process_group_leader()) {
            if group > 0 && group as u32 != child {
                return true;
            }
        }
        match self.terminal.try_lock() {
            Ok(terminal) => terminal.program_status().has_active_work(),
            Err(std::sync::TryLockError::Poisoned(error)) => error.into_inner().program_status().has_active_work(),
            Err(std::sync::TryLockError::WouldBlock) => true,
        }
    }
    pub fn program_status_since(
        &self,
        revision: u64,
    ) -> Option<(u64, Option<crate::program_status::Summary>)> {
        self.terminal
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .program_status()
            .summary_since(revision)
    }
    pub fn try_metadata_since(&self, title_revision: u64, directory_revision: u64, notification_revision: u64,
        program_status_revision: u64, include_records: bool) -> Option<MetadataUpdate> {
        let terminal = match self.terminal.try_lock() {
            Ok(terminal) => terminal,
            Err(std::sync::TryLockError::Poisoned(error)) => error.into_inner(),
            Err(std::sync::TryLockError::WouldBlock) => return None,
        };
        let program_status = terminal.program_status().summary_since(program_status_revision);
        let program_status_records = if include_records && program_status.is_some() {
            let records = terminal.program_status();
            let mut snapshot: Vec<_> = records.iter().map(|record| {
                let mut record = record.clone();
                record.app = records.effective_app(&record.id).unwrap_or("").to_owned();
                record
            }).collect();
            snapshot.sort_by(|left, right| left.id.cmp(&right.id));
            Some(snapshot)
        } else { None };
        Some(MetadataUpdate {
            program_status,
            program_status_records,
            render_delay: terminal.render_delay(),
            progress: terminal.progress(),
            bell_revision: terminal.bell_revision(),
            notifications: terminal.notifications_since(notification_revision),
            title: terminal.title_since(title_revision),
            directory: if self.remote_destination.is_none() { terminal.directory_since(directory_revision) } else { Ok(None) },
            remote_directory: if self.remote_destination.is_some() { terminal.remote_directory_since(directory_revision) } else { Ok(None) },
        })
    }
    pub fn try_render_delay(&self) -> Option<Option<std::time::Duration>> {
        let terminal = match self.terminal.try_lock() {
            Ok(terminal) => terminal,
            Err(std::sync::TryLockError::Poisoned(error)) => error.into_inner(),
            Err(std::sync::TryLockError::WouldBlock) => return None,
        };
        Some(terminal.render_delay())
    }
    pub fn render_delay(&self) -> Option<std::time::Duration> {
        self.terminal
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .render_delay()
    }
    pub fn exit_status(&self) -> Option<String> {
        self.exit.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
    /// Original PTY child identity, retained after exit; never a liveness check.
    pub fn process_id(&self) -> Option<u32> {
        self.child_pid
    }
    /// Current Unix foreground process-group leader, excluding the root shell.
    /// A point-in-time identity, not an aggregate or guarantee that it stays alive.
    /// Windows has no equivalent PTY job-control query and returns None.
    pub fn foreground_process_id(&self) -> Option<u32> {
        #[cfg(unix)]
        if !self.process_exited() {
            let group = self.master.process_group_leader()?;
            if group > 0 && Some(group as u32) != self.child_pid { return Some(group as u32); }
        }
        None
    }
    /// Reaper completion observed without acquiring the terminal or exit lock.
    pub fn process_exited(&self) -> bool {
        self.shutdown.is_finished()
    }
    /// PTY bytes parsed by the engine; excludes queued/unread kernel output.
    pub fn output_bytes(&self) -> u64 {
        self.output_bytes.load(Ordering::Relaxed)
    }
    /// Attempt a resize without waiting behind output parsing. None means no
    /// PTY or engine change occurred; callers should retry their latest size.
    /// Some(false) is unchanged; Some(true) applied both PTY and engine sizes.
    pub fn try_resize(&mut self, columns: u16, rows: u16) -> Result<Option<bool>> {
        if self.size == (columns, rows) { return Ok(Some(false)); }
        let mut terminal = match self.terminal.try_lock() {
            Ok(terminal) => terminal,
            Err(std::sync::TryLockError::Poisoned(error)) => error.into_inner(),
            Err(std::sync::TryLockError::WouldBlock) => return Ok(None),
        };
        self.master.resize(PtySize { rows, cols: columns, pixel_width: 0, pixel_height: 0 })?;
        terminal.resize(columns, rows)?;
        self.size = (columns, rows);
        Ok(Some(true))
    }

    pub fn resize(&mut self, columns: u16, rows: u16) -> Result<()> {
        if self.size == (columns, rows) {
            return Ok(());
        }
        self.master.resize(PtySize {
            rows,
            cols: columns,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        self.terminal
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .resize(columns, rows)?;
        self.size = (columns, rows);
        Ok(())
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.shutdown.request();
    }
}

fn is_powershell(shell: &std::ffi::OsStr) -> bool {
    std::path::Path::new(shell)
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            ["pwsh", "pwsh.exe", "powershell", "powershell.exe"]
                .iter()
                .any(|candidate| name.eq_ignore_ascii_case(candidate))
        })
}

/// PowerShell ignores -Login unless it is the first argument. Its startup path
/// uses these fallback flags when automatic integration is disabled.
fn unix_shell_flags(shell: &std::ffi::OsStr, login: bool) -> &'static [&'static str] {
    let powershell = std::path::Path::new(shell).file_name().and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("pwsh") || name.eq_ignore_ascii_case("pwsh.exe"));
    match (powershell, login) {
        (true, true) => &["-Login", "-Interactive"],
        (true, false) => &["-Interactive"],
        (false, true) => &["-i", "-l"],
        (false, false) => &["-i"],
    }
}

fn select_shell(
    explicit: Option<std::ffi::OsString>,
    inherited: Option<std::ffi::OsString>,
) -> Result<std::ffi::OsString> {
    if let Some(shell) = explicit {
        anyhow::ensure!(!shell.is_empty(), "RMUX_SHELL must name a shell executable");
        return Ok(shell);
    }
    Ok(inherited
        .filter(|shell| !shell.is_empty())
        .unwrap_or_else(|| {
            if cfg!(windows) {
                "cmd.exe".into()
            } else {
                "/bin/sh".into()
            }
        }))
}

#[cfg(test)]
mod tests {
    #[test]
    fn powershell_login_flag_precedes_interactive_without_changing_other_shells() {
        use std::ffi::OsStr;
        for shell in ["pwsh", "/opt/Power Shell/pwsh", "/tmp/PWSH", "pwsh.exe"] {
            assert_eq!(super::unix_shell_flags(OsStr::new(shell), true), ["-Login", "-Interactive"]);
            assert_eq!(super::unix_shell_flags(OsStr::new(shell), false), ["-Interactive"]);
        }
        for shell in ["/bin/bash", "/bin/zsh", "fish", "/tmp/pwsh-custom"] {
            assert_eq!(super::unix_shell_flags(OsStr::new(shell), true), ["-i", "-l"]);
            assert_eq!(super::unix_shell_flags(OsStr::new(shell), false), ["-i"]);
        }
    }
    #[test]
    #[cfg(unix)]
    fn explicit_remote_sessions_do_not_export_local_directory_metadata() -> anyhow::Result<()> {
        let directory = tempfile::tempdir()?;
        let mut session = super::Session::command_in(&["/bin/sh".into(), "-c".into(), "exit 0".into()], 80, 24, Some(directory.path()))?;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        while session.exit_status().is_none() {
            anyhow::ensure!(std::time::Instant::now() < deadline, "fixture process did not exit");
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(session.initial_directory().is_some());
        session.set_remote_destination("user@example.invalid")?;
        assert_eq!(session.remote_destination(), Some("user@example.invalid"));
        assert!(session.initial_directory().is_none());
        for report in [b"\x1b]7;file://localhost/tmp\x07".as_slice(), b"\x1b]7;file:///tmp\x07".as_slice()] {
            session.terminal.lock().unwrap().write(report);
            assert!(session.current_directory()?.is_none());
            assert!(session.directory_lookup(|| true)()?.is_none());
            let metadata = session.try_metadata_since(0, 0, 0, 0, false).unwrap();
            assert!(metadata.directory?.is_none());
            let (revision, label) = metadata.remote_directory?.unwrap();
            assert_eq!(label.as_deref(), Some("/tmp"));
            assert!(session.try_metadata_since(0, revision, 0, 0, false).unwrap().remote_directory?.is_none());
        }
        Ok(())
    }
    #[test]
    fn shell_override_preserves_literal_paths_and_rejects_empty_override() -> anyhow::Result<()> {
        use super::select_shell;
        use std::ffi::OsString;
        let custom = OsString::from("/path with spaces/my-shell");
        assert_eq!(
            select_shell(Some(custom.clone()), Some("fallback".into()))?,
            custom
        );
        assert_eq!(
            select_shell(None, Some("inherited-shell".into()))?,
            OsString::from("inherited-shell")
        );
        assert!(select_shell(Some(OsString::new()), Some("fallback".into())).is_err());
        assert_eq!(
            select_shell(None, Some(OsString::new()))?,
            select_shell(None, None)?
        );
        Ok(())
    }
    use super::*;
    #[cfg(unix)]
    #[test]
    fn opportunistic_snapshot_and_targeted_input_do_not_wait_for_parser_lock() -> Result<()> {
        let mut command = CommandBuilder::new("/bin/sh");
        command.args(["-c", "printf READY; IFS= read -r line; printf '\\nRECEIVED:%s\\n' \"$line\""]);
        let session = Session::spawn(command, 40, 4)?;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        while !session.frame()?.text().contains("READY") {
            anyhow::ensure!(std::time::Instant::now() < deadline, "receiver did not start");
            thread::sleep(std::time::Duration::from_millis(5));
        }
        let before = session.frame()?;
        assert!(!session.has_active_work(), "idle fixture must not need a close warning");
        let engine = session.terminal.clone();
        let (ready, started) = mpsc::sync_channel(1);
        let (release, finish) = mpsc::sync_channel(1);
        let holder = thread::spawn(move || {
            let mut engine = engine.lock().unwrap();
            engine.write(b"-pending\x1b]2;metadata-pending\x07");
            ready.send(()).unwrap();
            // Bound failure time if a regression accidentally waits on this lock.
            let _ = finish.recv_timeout(std::time::Duration::from_secs(1));
        });
        started.recv_timeout(std::time::Duration::from_secs(2))?;
        let started_at = std::time::Instant::now();
        let snapshot = session.try_frame()?;
        assert!(session.has_active_work(), "busy parser must require confirmation without waiting");
        assert!(session.hyperlink_at(0, 0, before.revision).unwrap_err().to_string().contains("click the link again"));
        assert!(session.try_metadata_since(0, 0, 0, 0, true).is_none(), "busy parser must not block metadata");
        assert!(session.try_render_delay().is_none(), "busy parser must not block render delay");
        assert!(session.try_program_status_records().is_none(), "busy parser must not block status details");
        assert!(session.history_tail_export(2, 4096)()?.is_none(), "busy parser must return no tail export");
        assert!(session.history_export(4096)()?.is_none(), "busy parser must return no history export");
        assert!(session.selection_export(4096, true)()?.is_none(), "busy parser must return no selection export");
        assert!(session.memory_sample()()?.is_none(), "busy parser must return no memory sample");
        session.paste("must-be-canceled-before-delivery")?;
        session.cancel_queued_paste();
        assert!(!session.key_with_text("a", Default::default(), false, Some("a"))?, "ordinary text should remain native while parser is busy");
        session.report_all_keys.store(true, Ordering::Release);
        assert!(session.key_with_text("a", Default::default(), false, Some("A"))?, "report-all text should queue while parser is busy");
        session.report_all_keys.store(false, Ordering::Release);
        // Ctrl+U clears the canonical fixture input before the payload below.
        assert!(session.key("u", crate::keyboard::Modifiers { control: true, ..Default::default() }, false)?);
        session.queue_key_release("up", Default::default())?;
        session.queue_focus_with_releases(false, vec!["a".into(), "enter".into()])?;
        session.queue_focus(true)?;
        session.queue_clear_scrollback()?;
        session.input_without_scroll("state-世界\n".as_bytes())?;
        let elapsed = started_at.elapsed();
        let _ = release.send(());
        holder.join().unwrap();
        assert!(snapshot.is_none(), "busy parser must return no snapshot");
        assert!(elapsed < std::time::Duration::from_millis(500), "opportunistic work waited behind parser: {elapsed:?}");
        assert!(!before.text().contains("pending"), "previous snapshot mutated");
        while !session.frame()?.text().contains("RECEIVED:state-世界") || session.exit_status().is_none() {
            anyhow::ensure!(std::time::Instant::now() < deadline, "queued input/output did not complete");
            thread::sleep(std::time::Duration::from_millis(5));
        }
        let after = session.try_frame()?.expect("idle snapshot");
        let memory = thread::spawn(session.memory_sample()).join().unwrap()?.expect("idle memory sample");
        assert!(memory.primary_resident_bytes > 0);
        assert!(after.text().contains("pending"), "busy snapshot consumed pending changes");
        let metadata = session.try_metadata_since(0, 0, 0, 0, true).expect("idle metadata");
        assert_eq!(metadata.title?.unwrap().1, "metadata-pending");
        session.select_all()?;
        let expected = session.selected_text()?.unwrap();
        let owners = Arc::strong_count(&session.terminal);
        let export = session.selection_export(4096, true);
        assert_eq!(Arc::strong_count(&session.terminal), owners, "queued export retained the engine");
        let selected = thread::spawn(export).join().unwrap()?.expect("idle selection export");
        assert_eq!(selected, (expected.clone(), false));
        assert_eq!(session.selected_text()?, Some(expected));
        let expected = session.selected_text()?;
        let clipboard = session.clipboard_export(true, || true);
        assert_eq!(Arc::strong_count(&session.terminal), owners, "queued clipboard export retained the engine");
        assert_eq!(thread::spawn(clipboard).join().unwrap()?, expected);
        assert!(session.clipboard_export(true, || false)()?.is_none());
        let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let seen = calls.clone();
        let superseded = session.clipboard_export(true, move || seen.fetch_add(1, Ordering::Relaxed) == 0);
        assert!(thread::spawn(superseded).join().unwrap()?.is_none());
        assert_eq!(calls.load(Ordering::Relaxed), 2);
        assert_eq!(session.selected_text()?, expected, "cancellation changed selection");
        let engine = session.terminal.lock().unwrap();
        let canceled = Arc::new(AtomicBool::new(false));
        let canceled_copy = canceled.clone();
        let (started, starting) = mpsc::channel();
        let waiting = session.clipboard_export(true, move || {
            let _ = started.send(());
            !canceled_copy.load(Ordering::Relaxed)
        });
        let (finished, completion) = mpsc::channel();
        let worker = thread::spawn(move || { let _ = finished.send(waiting()); });
        starting.recv_timeout(std::time::Duration::from_secs(2))?;
        canceled.store(true, Ordering::Relaxed);
        // The parser remains locked: completion must not depend on releasing it.
        let result = completion.recv_timeout(std::time::Duration::from_secs(2));
        drop(engine);
        worker.join().unwrap();
        assert!(result??.is_none(), "obsolete copy waited for parser access");

        let directory = session.directory_lookup(|| true);
        assert_eq!(Arc::strong_count(&session.terminal), owners, "queued directory lookup retained the engine");
        assert_eq!(thread::spawn(directory).join().unwrap()?, session.current_directory()?);
        assert!(session.directory_lookup(|| false)()?.is_none());
        let engine = session.terminal.lock().unwrap_or_else(|error| error.into_inner());
        let canceled = Arc::new(AtomicBool::new(false));
        let token = canceled.clone();
        let (started, starting) = mpsc::channel();
        let lookup = session.directory_lookup(move || {
            let _ = started.send(());
            !token.load(Ordering::Relaxed)
        });
        let (finished, completion) = mpsc::channel();
        let worker = thread::spawn(move || { let _ = finished.send(lookup()); });
        starting.recv_timeout(std::time::Duration::from_secs(2))?;
        canceled.store(true, Ordering::Relaxed);
        let result = completion.recv_timeout(std::time::Duration::from_secs(2));
        drop(engine);
        worker.join().unwrap();
        assert!(result??.is_none(), "obsolete directory lookup waited for parser access");
        Ok(())
    }
    #[test]
    fn queued_protocol_events_arrive_in_order_through_pty() -> Result<()> {
        let expected = b"\x1b[I\x1b[1;2:1A\x1b[1;2:3A\x1b[O";
        let command = format!("stty raw -echo; printf '\\033[>3u\\033[?1004hready\\r\\n'; dd bs=1 count={} 2>/dev/null | od -An -tx1", expected.len());
        let mut builder = CommandBuilder::new("/bin/sh");
        builder.args(["-c", &command]);
        let session = Session::spawn(builder, 80, 8)?;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !session.frame()?.text().contains("ready") {
            anyhow::ensure!(std::time::Instant::now() < deadline, "protocol fixture not ready");
            thread::sleep(std::time::Duration::from_millis(5));
        }
        let mods = crate::keyboard::Modifiers { shift: true, ..Default::default() };
        session.queue_focus(true)?;
        assert!(session.key("up", mods, false)?);
        session.queue_key_release("up", mods)?;
        session.queue_focus(false)?;
        let expected_hex = expected.iter().map(|byte| format!("{byte:02x}")).collect::<Vec<_>>();
        loop {
            let frame = session.frame()?.text();
            let received: Vec<_> = frame.split_whitespace().filter(|token| token.len() == 2 && token.bytes().all(|byte| byte.is_ascii_hexdigit())).collect();
            if received == expected_hex && session.exit_status().is_some() { break; }
            anyhow::ensure!(std::time::Instant::now() < deadline, "protocol bytes missing or reordered: {frame:?}");
            thread::sleep(std::time::Duration::from_millis(5));
        }
        Ok(())
    }
    #[test]
    fn queued_mouse_uses_published_mode_without_waiting_for_parser() -> Result<()> {
        let expected = b"\x1b[<64;3;2M";
        let command = format!("stty raw -echo; printf '\\033[?1000h\\033[?1006hready\\r\\n'; dd bs=1 count={} 2>/dev/null | od -An -tx1; printf '\\033[?1000l'", expected.len());
        let mut builder = CommandBuilder::new("/bin/sh");
        builder.args(["-c", &command]);
        let session = Session::spawn(builder, 80, 8)?;
        let event = crate::mouse::Event {
            action: crate::mouse::Action::Press, button: Some(crate::mouse::Button::WheelUp),
            x: 20., y: 20., width: 800, height: 160, cell_width: 10., cell_height: 20.,
            pressed: false, modifiers: Default::default(),
        };
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !session.mouse_tracking.load(Ordering::Acquire) {
            anyhow::ensure!(std::time::Instant::now() < deadline, "mouse mode was not published");
            thread::sleep(std::time::Duration::from_millis(5));
        }
        let terminal = session.terminal.clone();
        let (started, starting) = mpsc::channel();
        let (release, waiting) = mpsc::channel();
        let holder = thread::spawn(move || {
            let _parser = terminal.lock().unwrap();
            started.send(()).unwrap();
            let _ = waiting.recv_timeout(std::time::Duration::from_secs(2));
        });
        starting.recv()?;
        let start = std::time::Instant::now();
        assert!(session.queue_mouse(event)?);
        let elapsed = start.elapsed();
        release.send(())?;
        holder.join().unwrap();
        assert!(elapsed < std::time::Duration::from_millis(500), "mouse admission waited for parser");
        let expected_hex = expected.iter().map(|byte| format!("{byte:02x}")).collect::<Vec<_>>();
        loop {
            let frame = session.frame()?.text();
            let received: Vec<_> = frame.split_whitespace().filter(|token| token.len() == 2 && token.bytes().all(|byte| byte.is_ascii_hexdigit())).collect();
            if received == expected_hex && !session.mouse_tracking.load(Ordering::Acquire) && session.exit_status().is_some() { break; }
            anyhow::ensure!(std::time::Instant::now() < deadline, "queued mouse bytes or reset missing: {frame:?}");
            thread::sleep(std::time::Duration::from_millis(5));
        }
        let _parser = session.terminal.lock().unwrap();
        assert!(!session.queue_mouse(event)?, "disabled reporting must leave scrolling local");
        Ok(())
    }

    #[test]
    fn queued_plain_text_preserves_native_text_across_mode_changes() -> Result<()> {
        let terminal = Arc::new(Mutex::new(Terminal::new(20, 4)?));
        let mods = crate::keyboard::Modifiers { shift: true, ..Default::default() };
        terminal.lock().unwrap().write(b"\x1b[>24u");
        let payload = || InputPayload::KeyPress { terminal: Arc::downgrade(&terminal), key: "a".into(), mods, repeat: false, plain_text: Some("A".into()) };
        assert_eq!(payload().encode()?, b"\x1b[97;2;65u");
        let pending = payload();
        terminal.lock().unwrap().write(b"\x1b[<u");
        assert_eq!(pending.encode()?, b"A", "mode change must not drop committed text");
        Ok(())
    }
    #[test]
    fn queued_plain_text_uses_mode_published_by_pty_reader() -> Result<()> {
        let expected = b"\x1b[97;2;65u";
        let command = format!("stty raw -echo; printf '\\033[>24uready\\r\\n'; dd bs=1 count={} 2>/dev/null | od -An -tx1; printf '\\033[<u'", expected.len());
        let mut builder = CommandBuilder::new("/bin/sh");
        builder.args(["-c", &command]);
        let session = Session::spawn(builder, 80, 8)?;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !session.report_all_keys.load(Ordering::Acquire) {
            anyhow::ensure!(std::time::Instant::now() < deadline, "keyboard mode was not published");
            thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(session.key_with_text("a", crate::keyboard::Modifiers { shift: true, ..Default::default() }, false, Some("A"))?);
        let expected_hex = expected.iter().map(|byte| format!("{byte:02x}")).collect::<Vec<_>>();
        loop {
            let frame = session.frame()?.text();
            let received: Vec<_> = frame.split_whitespace().filter(|token| token.len() == 2 && token.bytes().all(|byte| byte.is_ascii_hexdigit())).collect();
            if received == expected_hex && !session.report_all_keys.load(Ordering::Acquire) && session.exit_status().is_some() { break; }
            anyhow::ensure!(std::time::Instant::now() < deadline, "queued text or mode reset missing: {frame:?}");
            thread::sleep(std::time::Duration::from_millis(5));
        }
        Ok(())
    }
    #[test]
    fn queued_navigation_press_repeat_and_release_preserve_protocol_order() -> Result<()> {
        let terminal = Arc::new(Mutex::new(Terminal::new(20, 4)?));
        terminal.lock().unwrap().write(b"\x1b[>3u");
        let mods = crate::keyboard::Modifiers { shift: true, ..Default::default() };
        let (sender, receiver) = mpsc::sync_channel(3);
        for repeat in [false, true] {
            sender.send(InputMessage { payload: InputPayload::KeyPress {
                terminal: Arc::downgrade(&terminal), key: "up".into(), mods, repeat, plain_text: None,
            }, _paste: None })?;
        }
        sender.send(InputMessage { payload: InputPayload::KeyRelease {
            terminal: Arc::downgrade(&terminal), key: "up".into(), mods,
        }, _paste: None })?;
        drop(sender);
        let mut output = Vec::new();
        write_messages(receiver, &mut output, |error| panic!("{error}"), || {});
        assert_eq!(output, b"\x1b[1;2:1A\x1b[1;2:2A\x1b[1;2:3A");
        Ok(())
    }
    #[test]
    fn queued_key_release_preserves_modifiers_and_writer_order() -> Result<()> {
        let terminal = Arc::new(Mutex::new(Terminal::new(20, 4)?));
        terminal.lock().unwrap().write(b"\x1b[>3u");
        let (sender, receiver) = mpsc::sync_channel(3);
        sender.send(b"before".to_vec().into())?;
        sender.send(InputMessage { payload: InputPayload::KeyRelease {
            terminal: Arc::downgrade(&terminal), key: "up".into(),
            mods: crate::keyboard::Modifiers { shift: true, ..Default::default() },
        }, _paste: None })?;
        sender.send(b"after".to_vec().into())?;
        drop(sender);
        let mut output = Vec::new();
        write_messages(receiver, &mut output, |error| panic!("{error}"), || {});
        assert_eq!(output, b"before\x1b[1;2:3Aafter");
        let pending = InputPayload::KeyRelease { terminal: Arc::downgrade(&terminal), key: "up".into(), mods: Default::default() };
        drop(terminal);
        assert!(pending.encode()?.is_empty());
        Ok(())
    }
    #[test]
    fn queued_held_key_releases_precede_focus_changes() -> Result<()> {
        let terminal = Arc::new(Mutex::new(Terminal::new(20, 4)?));
        terminal.lock().unwrap().write(b"\x1b[>3u\x1b[?1004h");
        let expected = {
            let mut engine = terminal.lock().unwrap();
            let mut bytes = engine.encode_key_action("a", crate::keyboard::Modifiers::default(), crate::keyboard::Action::Release)?.unwrap();
            bytes.extend(engine.encode_key_action("enter", crate::keyboard::Modifiers::default(), crate::keyboard::Action::Release)?.unwrap());
            assert!(!bytes.is_empty());
            bytes
        };
        for focused in [false, true] {
            let mut expected = expected.clone();
            expected.extend(if focused { b"\x1b[I" } else { b"\x1b[O" });
            let payload = InputPayload::Focus { terminal: Arc::downgrade(&terminal), focused, releases: vec!["a".into(), "enter".into()] };
            assert_eq!(payload.encode()?, expected);
        }
        Ok(())
    }
    #[test]
    fn queued_scrollback_clear_preserves_screen_and_wakes_ui() -> Result<()> {
        let terminal = Arc::new(Mutex::new(Terminal::new(20, 3)?));
        let before = {
            let mut engine = terminal.lock().unwrap();
            for _ in 0..12 { engine.write(b"history\r\n"); }
            engine.write(b"screen");
            let frame = engine.frame()?;
            assert!(frame.scrollback_lines > 0);
            frame.text()
        };
        let (sender, receiver) = mpsc::sync_channel(3);
        sender.send(b"before".to_vec().into())?;
        sender.send(InputMessage { payload: InputPayload::ClearScrollback { terminal: Arc::downgrade(&terminal) }, _paste: None })?;
        sender.send(b"after".to_vec().into())?;
        drop(sender);
        let mut output = Vec::new();
        let mut wakes = 0;
        write_messages(receiver, &mut output, |error| panic!("{error}"), || wakes += 1);
        assert_eq!(output, b"beforeafter");
        assert_eq!(wakes, 1);
        let frame = terminal.lock().unwrap().frame()?;
        assert_eq!(frame.scrollback_lines, 0);
        assert_eq!(frame.text(), before);
        let pending = InputPayload::ClearScrollback { terminal: Arc::downgrade(&terminal) };
        drop(terminal);
        assert!(pending.encode()?.is_empty());
        Ok(())
    }
    #[test]
    fn queued_focus_preserves_order_and_wakes_without_reporting() -> Result<()> {
        let terminal = Arc::new(Mutex::new(Terminal::new(20, 4)?));
        for reporting in [false, true] {
            if reporting { terminal.lock().unwrap().write(b"\x1b[?1004h"); }
            let (sender, receiver) = mpsc::sync_channel(4);
            for focused in [false, true] {
                sender.send(InputMessage { payload: InputPayload::Focus {
                    terminal: Arc::downgrade(&terminal), focused, releases: Vec::new(),
                }, _paste: None })?;
            }
            sender.send(b"after-focus".to_vec().into())?;
            drop(sender);
            let mut output = Vec::new();
            let mut wakes = 0;
            write_messages(receiver, &mut output, |error| panic!("{error}"), || wakes += 1);
            assert_eq!(wakes, 2, "focus acknowledgement wakes quiet metadata observers");
            assert_eq!(output, if reporting { b"\x1b[O\x1b[Iafter-focus".as_slice() } else { b"after-focus".as_slice() });
        }
        let pending = InputPayload::Focus { terminal: Arc::downgrade(&terminal), focused: true, releases: Vec::new() };
        drop(terminal);
        assert!(pending.encode()?.is_empty(), "queued focus must not retain a closed terminal");
        Ok(())
    }
    #[test]
    fn native_paste_resolves_mode_on_worker_and_cancels_while_parser_busy() -> Result<()> {
        let terminal = Arc::new(Mutex::new(Terminal::new(20, 3)?));
        let pending = Arc::new(AtomicBool::new(false));
        let generation = Arc::new(AtomicU64::new(0));
        for bracketed in [true, false] {
            let payload = InputPayload::NativePaste { terminal: Arc::downgrade(&terminal), text: "hello 世界\n".into() };
            terminal.lock().unwrap().write(if bracketed { b"\x1b[?2004h" } else { b"\x1b[?2004l" });
            let permit = PastePermit::acquire(&pending, &generation)?;
            assert_eq!(payload.encode_pending(Some(&permit))?, Some(crate::keyboard::encode_paste("hello 世界\n", bracketed)?));
        }
        let engine = terminal.lock().unwrap();
        let message = InputMessage {
            payload: InputPayload::NativePaste { terminal: Arc::downgrade(&terminal), text: "discard".into() },
            _paste: Some(PastePermit::acquire(&pending, &generation)?),
        };
        let (started, starting) = mpsc::channel();
        let (finished, completion) = mpsc::channel();
        let worker = thread::spawn(move || {
            started.send(()).unwrap();
            let mut output = Vec::new();
            let mut errors = Vec::new();
            assert!(write_message(message, &mut output, &mut |error| errors.push(error)));
            finished.send((output, errors)).unwrap();
        });
        starting.recv_timeout(std::time::Duration::from_secs(2))?;
        generation.fetch_add(1, Ordering::AcqRel);
        let result = completion.recv_timeout(std::time::Duration::from_secs(2));
        drop(engine);
        worker.join().unwrap();
        let (output, errors) = result?;
        assert!(output.is_empty() && errors.is_empty());
        assert!(!pending.load(Ordering::Acquire));
        let payload = InputPayload::NativePaste { terminal: Arc::downgrade(&terminal), text: "closed".into() };
        drop(terminal);
        assert!(payload.encode_pending(None)?.is_none());
        Ok(())
    }

    #[test]
    fn canceled_paste_behind_blocked_writer_preserves_following_input() -> Result<()> {
        struct BlockedWriter {
            entered: Option<mpsc::Sender<()>>,
            release: mpsc::Receiver<()>,
            output: Arc<Mutex<Vec<u8>>>,
        }
        impl Write for BlockedWriter {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                if let Some(entered) = self.entered.take() {
                    entered.send(()).unwrap();
                    self.release.recv_timeout(std::time::Duration::from_secs(2))
                        .map_err(|error| std::io::Error::new(std::io::ErrorKind::TimedOut, error))?;
                }
                self.output.lock().unwrap().extend_from_slice(bytes);
                Ok(bytes.len())
            }
            fn flush(&mut self) -> std::io::Result<()> { Ok(()) }
        }
        let (sender, receiver) = mpsc::sync_channel(4);
        let (entered, starting) = mpsc::channel();
        let (release, waiting) = mpsc::channel();
        let output = Arc::new(Mutex::new(Vec::new()));
        let writer = BlockedWriter { entered: Some(entered), release: waiting, output: output.clone() };
        let pending = Arc::new(AtomicBool::new(false));
        let observed = pending.clone();
        let worker = thread::spawn(move || {
            let mut errors = Vec::new();
            let mut completions = 0;
            write_messages(receiver, writer, |error| errors.push(error), || {
                assert!(!observed.load(Ordering::Acquire));
                completions += 1;
            });
            assert_eq!(completions, 1);
            errors
        });
        sender.send(b"BEFORE".to_vec().into()).ok().unwrap();
        starting.recv_timeout(std::time::Duration::from_secs(2))?;
        let generation = Arc::new(AtomicU64::new(0));
        sender.send(InputMessage {
            payload: InputPayload::Paste { text: "discard me".into(), bracketed: true },
            _paste: Some(PastePermit::acquire(&pending, &generation)?),
        }).ok().unwrap();
        sender.send(b"AFTER".to_vec().into()).ok().unwrap();
        generation.fetch_add(1, Ordering::AcqRel);
        drop(sender);
        release.send(())?;
        assert!(worker.join().unwrap().is_empty());
        assert_eq!(*output.lock().unwrap(), b"BEFOREAFTER");
        assert!(!pending.load(Ordering::Acquire));
        Ok(())
    }

    #[test]
    fn canceled_paste_skips_encoding_and_releases_its_slot() -> Result<()> {
        let pending = Arc::new(AtomicBool::new(false));
        let generation = Arc::new(AtomicU64::new(0));
        let permit = PastePermit::acquire(&pending, &generation)?;
        generation.fetch_add(1, Ordering::AcqRel);
        // An oversized payload would fail encoding if cancellation were ignored.
        let payload = InputPayload::Paste { text: "x".repeat(1024 * 1024 + 1), bracketed: true };
        assert!(payload.encode_pending(Some(&permit))?.is_none());
        assert!(pending.load(Ordering::Acquire));
        drop(permit);
        let permit = PastePermit::acquire(&pending, &generation)?;
        let payload = InputPayload::Paste { text: "next".into(), bracketed: true };
        assert_eq!(payload.encode_pending(Some(&permit))?, Some(b"\x1b[200~next\x1b[201~".to_vec()));
        assert_eq!(InputPayload::Bytes(b"AFTER".to_vec()).encode_pending(None)?, Some(b"AFTER".to_vec()));
        drop(permit);
        assert!(!pending.load(Ordering::Acquire));
        Ok(())
    }
    #[test]
    fn paste_permit_survives_queueing_and_recovers_from_rejection() -> Result<()> {
        let generation = Arc::new(AtomicU64::new(0));
        let pending = Arc::new(AtomicBool::new(false));
        let (sender, receiver) = mpsc::sync_channel(1);
        sender.try_send(InputMessage {
            payload: InputPayload::Bytes(vec![1]),
            _paste: Some(PastePermit::acquire(&pending, &generation)?),
        }).ok().unwrap();
        assert!(PastePermit::acquire(&pending, &generation).is_err());
        let message = receiver.recv()?;
        assert!(PastePermit::acquire(&pending, &generation).is_err());
        drop(message);
        sender.try_send(vec![2].into()).ok().unwrap();
        let rejected = sender.try_send(InputMessage {
            payload: InputPayload::Bytes(vec![3]),
            _paste: Some(PastePermit::acquire(&pending, &generation)?),
        });
        assert!(rejected.is_err());
        drop(rejected);
        drop(PastePermit::acquire(&pending, &generation)?);
        Ok(())
    }
    #[cfg(unix)]
    #[test]
    fn large_bracketed_paste_arrives_before_following_input() -> Result<()> {
        let root = tempfile::tempdir()?;
        let destination = root.path().join("received");
        let text = "界\nabc".repeat(20_000);
        let expected = format!("\x1b[200~{text}\x1b[201~AFTER");
        let mut command = CommandBuilder::new("/bin/sh");
        command.args(["-c", "stty raw -echo; printf '\\033[?2004hREADY'; dd bs=1 count=\"$2\" of=\"$1\" 2>/dev/null", "rmux-test"]);
        command.arg(&destination);
        command.arg(expected.len().to_string());
        let session = Session::spawn(command, 40, 4)?;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !session.frame()?.text().contains("READY") {
            anyhow::ensure!(std::time::Instant::now() < deadline, "paste receiver not ready");
            thread::sleep(std::time::Duration::from_millis(5));
        }
        session.paste(&text)?;
        session.input(b"AFTER")?;
        while session.exit_status().is_none() {
            anyhow::ensure!(std::time::Instant::now() < deadline, "paste delivery timed out");
            thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(std::fs::read(destination)?, expected.as_bytes());
        Ok(())
    }
    #[cfg(unix)]
    #[test]
    fn large_paste_preserves_bytes_with_concurrent_output() -> Result<()> {
        let root = tempfile::tempdir()?;
        let destination = root.path().join("received");
        let text = "界\nabc".repeat(131_072);
        let expected = format!("\x1b[200~{text}\x1b[201~AFTER");
        let mut command = CommandBuilder::new("/bin/sh");
        command.args(["-c", r#"
stty raw -echo
printf '\033[?2004hREADY'
dd bs=1 count=1 of=/dev/null 2>/dev/null
(
    i=0
    while [ "$i" -lt 4096 ]; do
        printf 'pressure-%s abcdefghijklmnopqrstuvwxyz0123456789abcdefghijklmnopqrstuvwxyz\r\n' "$i"
        i=$((i + 1))
    done
) &
pressure=$!
trap 'kill "$pressure" 2>/dev/null; wait "$pressure" 2>/dev/null' EXIT HUP TERM
dd bs=1 count="$2" of="$1" 2>/dev/null
wait "$pressure"
trap - EXIT HUP TERM
"#, "rmux-pressure-test"]);
        command.arg(&destination);
        command.arg(expected.len().to_string());
        let session = Session::spawn(command, 80, 8)?;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !session.frame()?.text().contains("READY") {
            anyhow::ensure!(std::time::Instant::now() < deadline, "pressure receiver not ready");
            thread::sleep(std::time::Duration::from_millis(5));
        }
        let before = session.output_bytes();
        session.input(b"G")?;
        // Native paste admission must succeed without retrying parser contention.
        session.paste(&text)?;
        session.input(b"AFTER")?;
        while session.exit_status().is_none() || session.output_bytes() - before < 250_000 {
            anyhow::ensure!(std::time::Instant::now() < deadline, "concurrent paste/output timed out");
            thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(std::fs::read(destination)?, expected.as_bytes());
        assert!(!session.paste_pending());
        assert!(session.take_input_error().is_none());
        Ok(())
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn local_input_modes_distinguish_hidden_line_input_from_raw_tuis() -> Result<()> {
        let mut command = CommandBuilder::new("/bin/sh");
        command.args(["-c", "stty icanon -echo; printf HIDDEN; IFS= read -r first; stty -icanon -echo; printf RAW; dd bs=1 count=1 >/dev/null 2>&1; stty icanon echo; printf VISIBLE; IFS= read -r last"]);
        let session = Session::spawn(command, 40, 3)?;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let wait = |marker: &str| -> Result<()> {
            while !session.frame()?.text().contains(marker) {
                anyhow::ensure!(std::time::Instant::now() < deadline, "input mode marker timed out: {marker}");
                thread::sleep(std::time::Duration::from_millis(5));
            }
            Ok(())
        };
        wait("HIDDEN")?;
        assert_eq!(session.local_input_mode(), Some(LocalInputMode { echo: false, canonical: true }));
        session.input_without_scroll(b"continue\n")?;
        wait("RAW")?;
        assert_eq!(session.local_input_mode(), Some(LocalInputMode { echo: false, canonical: false }));
        session.input_without_scroll(b"x")?;
        wait("VISIBLE")?;
        assert_eq!(session.local_input_mode(), Some(LocalInputMode { echo: true, canonical: true }));
        session.input_without_scroll(b"done\n")?;
        Ok(())
    }
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn automation_paste_is_nonblocking_bracketed_and_preserves_viewport() -> Result<()> {
        let root = tempfile::tempdir()?;
        let destination = root.path().join("received");
        let text = "世界\nsecond line".repeat(4096);
        let expected = format!("\x1b[200~{text}\x1b[201~AFTER");
        let mut command = CommandBuilder::new("/bin/sh");
        command.args(["-c", "stty raw -echo; printf '1\\n2\\n3\\n4\\n5\\n\\033[?2004hREADY'; dd bs=1 count=\"$2\" of=\"$1\" 2>/dev/null", "rmux-test"]);
        command.arg(&destination);
        command.arg(expected.len().to_string());
        let session = Session::spawn(command, 40, 3)?;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !session.frame()?.text().contains("READY") {
            anyhow::ensure!(std::time::Instant::now() < deadline, "paste receiver not ready");
            thread::sleep(std::time::Duration::from_millis(5));
        }
        session.scroll(crate::Scroll::Top)?;
        let offset = session.frame()?.scrollback_offset;
        assert!(offset > 0);
        {
            let _parser = session.terminal.lock().unwrap();
            assert!(!session.try_paste_without_scroll("not sent")?);
            assert!(!session.paste_pending.load(Ordering::Acquire));
        }
        assert!(session.try_paste_without_scroll(&"x".repeat(1024 * 1024 + 1)).is_err());
        assert!(!session.paste_pending());
        assert!(session.try_paste_without_scroll(&text)?);
        session.input_without_scroll(b"AFTER")?;
        assert_eq!(session.frame()?.scrollback_offset, offset);
        while session.exit_status().is_none() {
            anyhow::ensure!(std::time::Instant::now() < deadline, "paste delivery timed out");
            thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(std::fs::read(destination)?, expected.as_bytes());
        Ok(())
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn input_defers_follow_while_parser_busy_and_navigation_supersedes_it() -> Result<()> {
        let mut command = CommandBuilder::new("/bin/sh");
        command.args(["-c", "stty -echo; printf '1\\n2\\n3\\n4\\n5\\nREADY'; read -r line"]);
        let session = Session::spawn(command, 40, 3)?;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !session.frame()?.text().contains("READY") {
            anyhow::ensure!(std::time::Instant::now() < deadline, "shell not ready");
            thread::sleep(std::time::Duration::from_millis(5));
        }
        for (explicit_navigation, later_input) in [(false, false), (true, false), (true, true)] {
            session.scroll(crate::Scroll::Top)?;
            assert!(session.frame()?.scrollback_offset > 0);
            let terminal = session.terminal.clone();
            let (ready, waiting) = mpsc::channel();
            let (release, released) = mpsc::channel();
            let parser = thread::spawn(move || {
                let _guard = terminal.lock().unwrap();
                ready.send(()).unwrap();
                // Bounded even if input regresses to a blocking parser lock.
                let _ = released.recv_timeout(std::time::Duration::from_secs(2));
            });
            waiting.recv()?;
            let start = std::time::Instant::now();
            if !explicit_navigation {
                for index in 0..64 {
                    session.scroll(crate::Scroll::Lines(if index % 2 == 0 { -1 } else { 1 }))?;
                }
                assert!(session.scroll(crate::Scroll::Lines(-1)).is_err());
                assert_eq!(session.pending_scroll.lock().unwrap().len(), 64);
            }
            session.input(b"x")?;
            if explicit_navigation {
                session.scroll(crate::Scroll::Top)?;
                for _ in 0..100 { session.scroll(crate::Scroll::Lines(-1))?; }
                session.scroll(crate::Scroll::Lines(1))?;
                assert_eq!(session.pending_scroll.lock().unwrap().as_slice(),
                    &[crate::Scroll::Top, crate::Scroll::Lines(-100), crate::Scroll::Lines(1)]);
            }
            if later_input { session.input(b"y")?; }
            let elapsed = start.elapsed();
            let pending = !session.pending_scroll.lock().unwrap().is_empty();
            let _ = release.send(());
            parser.join().unwrap();
            assert!(elapsed < std::time::Duration::from_millis(500), "input waited for parser: {elapsed:?}");
            assert!(pending);
            let frame = session.try_frame()?.expect("parser released");
            assert_eq!(frame.scrollback_offset, if explicit_navigation && !later_input { frame.scrollback_lines - 1 } else { 0 });
            assert!(session.pending_scroll.lock().unwrap().is_empty());
        }
        Ok(())
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn explicit_close_reaps_child_while_session_is_retained() -> Result<()> {
        let session = Session::spawn(shell_command("printf ready; while :; do read line || exit; done"), 40, 4)?;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        while !session.frame()?.text().contains("ready") {
            anyhow::ensure!(std::time::Instant::now() < deadline, "readiness timeout");
            thread::sleep(std::time::Duration::from_millis(10));
        }
        let shutdown = session.shutdown_handle();
        let engine = session.terminal.lock().unwrap();
        session.paste("must not reach the closing shell\n")?;
        shutdown.request()?;
        while session.paste_pending() {
            anyhow::ensure!(std::time::Instant::now() < deadline, "closing paste waited for parser release");
            thread::sleep(std::time::Duration::from_millis(1));
        }
        assert!(session.paste("new paste").is_err());
        assert!(session.try_paste_without_scroll("automation paste").is_err());
        assert!(!session.paste_pending(), "rejected paste retained its permit");
        assert!(session.take_input_error().is_none());
        drop(engine);
        shutdown.request()?; // Repeated close and later Drop remain safe.
        while !shutdown.is_finished() {
            anyhow::ensure!(std::time::Instant::now() < deadline, "retained session kept child alive");
            thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(session.process_exited());
        assert!(session.frame()?.text().contains("ready"));
        Ok(())
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn closing_session_terminates_a_shell_that_ignores_hangup() -> Result<()> {
        let session = Session::spawn(
            shell_command("trap '' HUP; printf 'ready'; while :; do read line || exit; done"),
            40,
            4,
        )?;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        while !session.frame()?.text().contains("ready") {
            anyhow::ensure!(
                std::time::Instant::now() < deadline,
                "shell readiness timeout"
            );
            thread::sleep(std::time::Duration::from_millis(10));
        }
        let exit = session.exit.clone();
        let terminal = Arc::downgrade(&session.terminal);
        let shutdown = session.shutdown_handle();
        let started = std::time::Instant::now();
        drop(session);
        // Termination's grace period must run on the reaper, not this caller.
        assert!(started.elapsed() < std::time::Duration::from_millis(100));
        while exit.lock().unwrap().is_none()
            || terminal.upgrade().is_some()
            || !shutdown.is_finished()
        {
            anyhow::ensure!(
                std::time::Instant::now() < deadline,
                "stubborn shell retained after close"
            );
            thread::sleep(std::time::Duration::from_millis(10));
        }
        Ok(())
    }
    #[test]
    fn dropping_sessions_releases_shell_and_terminal() -> Result<()> {
        for _ in 0..8 {
            let session = Session::benchmark_shell(40, 4)?;
            let terminal = Arc::downgrade(&session.terminal);
            let exit = session.exit.clone();
            drop(session);
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
            while terminal.upgrade().is_some() || exit.lock().unwrap().is_none() {
                anyhow::ensure!(
                    std::time::Instant::now() < deadline,
                    "session retained its terminal or child after drop"
                );
                thread::sleep(std::time::Duration::from_millis(10));
            }
        }
        Ok(())
    }
    fn shell_command(script: &str) -> CommandBuilder {
        #[cfg(unix)]
        let mut command = CommandBuilder::new("/bin/sh");
        #[cfg(windows)]
        let mut command = CommandBuilder::new("cmd.exe");
        #[cfg(unix)]
        command.args(["-c", script]);
        #[cfg(windows)]
        command.args(["/d", "/c", script]);
        command
    }
    #[test]
    fn reports_exit_and_rejects_further_input() -> Result<()> {
        let command = shell_command("exit 7");
        let session = Session::spawn(command, 40, 4)?;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        loop {
            if let Some(status) = session.exit_status() {
                assert_eq!(status, "Process exited with status 7");
                assert!(session.input(b"ignored").is_err());
                return Ok(());
            }
            anyhow::ensure!(
                std::time::Instant::now() < deadline,
                "shell exit deadline exceeded"
            );
            thread::sleep(std::time::Duration::from_millis(10));
        }
    }
    #[cfg(unix)]
    #[test]
    fn close_detection_tracks_foreground_jobs_and_direct_commands() -> Result<()> {
        let session = Session::benchmark_shell(80, 8)?;
        let wait = |expected| -> Result<()> {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
            while session.has_active_work() != expected {
                anyhow::ensure!(
                    std::time::Instant::now() < deadline,
                    "foreground state did not become {expected}"
                );
                thread::sleep(std::time::Duration::from_millis(10));
            }
            Ok(())
        };
        wait(false)?;
        assert_eq!(session.foreground_process_id(), None);
        session.input(b"sleep 30\n")?;
        wait(true)?;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        while session.foreground_process_id().is_none() {
            anyhow::ensure!(std::time::Instant::now() < deadline, "foreground job identity unavailable");
            thread::sleep(std::time::Duration::from_millis(10));
        }
        assert_ne!(session.foreground_process_id(), session.process_id());
        session.input(b"\x03")?;
        wait(false)?;
        assert_eq!(session.foreground_process_id(), None);
        let argv = ["/bin/sleep", "30"].map(std::ffi::OsString::from);
        let direct = Session::command_in(&argv, 80, 8, None)?;
        assert!(direct.has_active_work());
        let shutdown = direct.shutdown_handle();
        drop(direct);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        while !shutdown.is_finished() {
            anyhow::ensure!(
                std::time::Instant::now() < deadline,
                "direct process cleanup timed out"
            );
            thread::sleep(std::time::Duration::from_millis(10));
        }
        Ok(())
    }
    #[cfg(unix)]
    #[test]
    fn direct_command_preserves_arguments_directory_and_terminal_environment() -> Result<()> {
        let root = tempfile::tempdir()?;
        let root = root.path().canonicalize()?;
        let argv = [
            "/bin/sh",
            "-c",
            "printf '%s\\n' \"$1\" \"$2\" \"$PWD\" \"$TERM:$COLORTERM:$TERM_PROGRAM\"",
            "fixture",
            "a b 世界",
            "$(touch nope); *",
        ]
        .map(std::ffi::OsString::from);
        let session = Session::command_in(&argv, 160, 8, Some(&root))?;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        loop {
            let text = session.frame()?.text();
            if text.contains("xterm-256color:truecolor:rmux") {
                assert!(text.contains("a b 世界"));
                assert!(text.contains("$(touch nope); *"));
                assert!(text.contains(root.to_str().unwrap()));
                assert!(!root.join("nope").exists());
                break;
            }
            anyhow::ensure!(
                std::time::Instant::now() < deadline,
                "direct command output timed out: {text}"
            );
            thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(Session::command_in(&[], 80, 24, None).is_err());
        Ok(())
    }
    #[cfg(unix)]
    #[test]
    fn command_environment_is_literal_and_scoped_to_child() -> Result<()> {
        let before = std::env::var_os("RMUX_TEST_COMMAND_VALUE");
        let argv = ["/bin/sh", "-c", "printf '[%s][%s]done' \"$RMUX_TEST_COMMAND_VALUE\" \"$RMUX_TEST_COMMAND_EMPTY\""].map(std::ffi::OsString::from);
        let environment = [("RMUX_TEST_COMMAND_VALUE".into(), "世界 $(literal)".into()), ("RMUX_TEST_COMMAND_EMPTY".into(), "".into())];
        let session = Session::command_in_with_environment(&argv, 100, 4, None, 0, &environment)?;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        loop {
            let text = session.frame()?.text();
            if text.contains("done") { assert!(text.contains("[世界 $(literal)][]done")); break; }
            anyhow::ensure!(std::time::Instant::now() < deadline, "environment fixture timed out");
            thread::sleep(std::time::Duration::from_millis(10));
        }
        assert_eq!(std::env::var_os("RMUX_TEST_COMMAND_VALUE"), before);
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn configured_retention_applies_to_first_process_output() -> Result<()> {
        let argv = ["/bin/sh", "-c", "i=0; while [ $i -lt 100 ]; do printf 'startup output\\n'; i=$((i+1)); done; printf 'finished-marker'"]
            .map(std::ffi::OsString::from);
        for limit in [0, 1024 * 1024] {
            let session = Session::command_in_with_scrollback(&argv, 40, 3, None, limit)?;
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
            loop {
                let frame = session.frame()?;
                if frame.text().contains("finished-marker") {
                    assert_eq!(frame.scrollback_lines > 0, limit > 0);
                    break;
                }
                anyhow::ensure!(
                    std::time::Instant::now() < deadline,
                    "startup output timed out"
                );
                thread::sleep(std::time::Duration::from_millis(10));
            }
        }
        Ok(())
    }
    #[test]
    fn invalid_explicit_directory_is_rejected_before_process_start() -> Result<()> {
        let root = tempfile::tempdir()?;
        let file = root.path().join("file");
        std::fs::write(&file, b"not a directory")?;
        for path in [root.path().join("missing"), file] {
            let mut command = CommandBuilder::new("rmux-fixture-must-not-launch");
            command.cwd(&path);
            let error = match Session::spawn(command, 80, 24) {
                Ok(_) => panic!("invalid directory was accepted"),
                Err(error) => error,
            };
            assert!(error.to_string().contains("working directory"), "{error:#}");
        }
        Ok(())
    }
    #[cfg(unix)]
    #[test]
    fn opportunistic_resize_leaves_busy_pty_unchanged_and_applies_latest_size() -> Result<()> {
        let mut command = CommandBuilder::new("/bin/sh");
        command.args(["-c", "IFS= read -r line"]);
        let mut session = Session::spawn(command, 40, 4)?;
        let engine = session.terminal.clone();
        {
            let _guard = engine.lock().unwrap();
            assert_eq!(session.try_resize(40, 4)?, Some(false));
            assert_eq!(session.try_resize(117, 31)?, None);
            assert_eq!(session.master.get_size()?.cols, 40);
            assert_eq!(session.master.get_size()?.rows, 4);
        }
        assert_eq!(session.try_resize(120, 33)?, Some(true));
        assert_eq!(session.master.get_size()?.cols, 120);
        assert_eq!(session.master.get_size()?.rows, 33);
        let frame = session.frame()?;
        assert_eq!(frame.columns, 120);
        assert_eq!(frame.rows.len(), 33);
        assert_eq!(session.try_resize(120, 33)?, Some(false));
        session.input_without_scroll(b"done\n")?;
        Ok(())
    }

    #[test]
    fn interactive_shell_accepts_input_after_resize() -> Result<()> {
        let mut session = Session::benchmark_shell(80, 24)?;
        let _ = session.frame()?;
        session.resize(117, 31)?;
        // The expected marker is absent from the command itself, so PTY echo
        // alone cannot satisfy this check.
        #[cfg(unix)]
        session.input(b"printf '%s%s\\n' rmux- interactive-ok\r")?;
        #[cfg(windows)]
        session.input(b"echo rmux-interactive-^ok\r")?;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        loop {
            if session.frame()?.text().contains("rmux-interactive-ok") {
                return Ok(());
            }
            anyhow::ensure!(
                std::time::Instant::now() < deadline,
                "interactive output deadline exceeded"
            );
            thread::sleep(std::time::Duration::from_millis(10));
        }
    }
    #[test]
    fn real_shell_output_reaches_ghostty() -> Result<()> {
        let command = shell_command(if cfg!(windows) {
            "echo rmux-pty-ok"
        } else {
            r"printf '\033[32mrmux-pty-ok\033[0m\n'"
        });
        let session = Session::spawn(command, 40, 4)?;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        loop {
            if session.frame()?.text().contains("rmux-pty-ok") {
                return Ok(());
            }
            anyhow::ensure!(
                std::time::Instant::now() < deadline,
                "PTY output deadline exceeded"
            );
            thread::sleep(std::time::Duration::from_millis(10));
        }
    }
}
