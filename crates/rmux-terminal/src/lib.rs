//! A small ownership boundary around the pinned Ghostty VT engine.
//!
//! A terminal may move between threads but must only be accessed exclusively.
//! Screen snapshots own their text; no borrowed engine pointers leave this crate.
#[allow(
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    dead_code
)]
mod ffi {
    include!(concat!(env!("OUT_DIR"), "/ghostty.rs"));
}

use anyhow::{Result, ensure};
use ffi::*;
use std::time::{Duration, Instant};
use std::{ffi::c_void, ptr, sync::Arc};

const RENDER_HOLD_LIMIT: Duration = Duration::from_millis(250);

/// Parse bounded literal theme colors using the pinned engine's color grammar.
/// This accepts X11 names and RGB specifications without creating a terminal.
pub fn parse_color(value: &str) -> Result<Color> {
    ensure!(!value.is_empty() && value.len() <= 256 && !value.chars().any(|ch| ch.is_control() && ch != '\t'), "color must fit 256 bytes without control characters");
    let mut parsed = GhosttyColorRgb::default();
    // SAFETY: the engine receives a valid UTF-8 byte slice and length, and a
    // writable RGB output. It neither retains the slice nor needs a NUL sentinel.
    let result = unsafe { ghostty_color_parse(value.as_ptr().cast(), value.len(), &mut parsed) };
    ensure!(result == GHOSTTY_SUCCESS, "unsupported or invalid literal color");
    Ok(parsed.into())
}

/// Parse a bounded N=COLOR palette assignment with Ghostty's numeric grammar.
pub fn parse_palette_entry(value: &str) -> Result<(u8, Color)> {
    ensure!(!value.is_empty() && value.len() <= 272 && !value.chars().any(|ch| ch.is_control() && ch != '\t'), "palette entry must fit 272 bytes without control characters");
    let mut index = 0;
    let mut parsed = GhosttyColorRgb::default();
    // SAFETY: valid length-delimited input and correctly sized writable outputs;
    // the parser does not retain input or output pointers.
    let result = unsafe { ghostty_color_parse_palette_entry(value.as_ptr().cast(), value.len(), &mut index, &mut parsed) };
    ensure!(result == GHOSTTY_SUCCESS, "invalid palette index or color");
    Ok((index, parsed.into()))
}

/// Factory indexed palette, obtained without allocating a terminal.
pub fn default_palette() -> [Color; 256] {
    static DEFAULTS: std::sync::OnceLock<[Color; 256]> = std::sync::OnceLock::new();
    *DEFAULTS.get_or_init(|| {
        let mut palette = [GhosttyColorRgb::default(); 256];
        // SAFETY: the API writes exactly 256 RGB values into this stack array.
        unsafe { ghostty_color_palette_default(palette.as_mut_ptr()); }
        palette.map(Into::into)
    })
}

/// Read the pinned engine's factory ANSI palette without allocating a terminal.
pub fn default_ansi_palette() -> [Color; 16] {
    let palette = default_palette();
    std::array::from_fn(|index| palette[index])
}


#[derive(Default)]
struct Callbacks {
    replies: Vec<u8>,
    hold_started: Option<Instant>,
    title_revision: u64,
    bell_revision: u64,
    directory_revision: u64,
    notification: Option<Arc<DesktopNotification>>,
    notification_history: std::collections::VecDeque<Arc<DesktopNotification>>,
    kitty_notifications: kitty_notification::Assembler,
    progress: Option<Progress>,
    program_status: program_status::Records,
    program_finished: bool,
}

impl Callbacks {
    fn store_notification(&mut self, title: String, body: String, visibility: NotificationVisibility) {
        let revision = self.notification.as_ref().map_or(1, |previous| previous.revision.wrapping_add(1));
        let message = Arc::new(DesktopNotification { revision, title, body, visibility });
        self.notification = Some(message.clone());
        if self.notification_history.len() == 32 { self.notification_history.pop_front(); }
        self.notification_history.push_back(message);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProgressState {
    Running,
    Error,
    Indeterminate,
    Paused,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Progress {
    pub state: ProgressState,
    pub percent: Option<u8>,
}

unsafe extern "C" fn progress_report(
    _: GhosttyTerminal,
    userdata: *mut c_void,
    report: *const GhosttyTerminalProgressReport,
) {
    if report.is_null() {
        return;
    }
    // SAFETY: synchronous callback; borrowed report and stable userdata remain
    // valid throughout this exclusive engine access. No allocation or UI work.
    unsafe {
        let report = &*report;
        let state = match report.state {
            GHOSTTY_TERMINAL_PROGRESS_STATE_SET => Some(ProgressState::Running),
            GHOSTTY_TERMINAL_PROGRESS_STATE_ERROR => Some(ProgressState::Error),
            GHOSTTY_TERMINAL_PROGRESS_STATE_INDETERMINATE => Some(ProgressState::Indeterminate),
            GHOSTTY_TERMINAL_PROGRESS_STATE_PAUSE => Some(ProgressState::Paused),
            _ => None,
        };
        let callbacks = &mut *userdata.cast::<Callbacks>();
        // The reader may still drain buffered output after process exit. Keep
        // final errors and explicit clears, but do not revive active progress.
        if callbacks.program_finished && state.is_some_and(|state| state != ProgressState::Error) {
            return;
        }
        callbacks.progress = state.map(|state| Progress {
            state,
            percent: (0..=100)
                .contains(&report.progress)
                .then_some(report.progress as u8),
        });
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NotificationVisibility {
    #[default]
    Always,
    Unfocused,
    Invisible,
}
impl NotificationVisibility {
    pub fn allows(self, focused: bool, visible: bool) -> bool {
        match self {
            Self::Always => true,
            Self::Unfocused => !focused,
            Self::Invisible => !focused && !visible,
        }
    }
}
#[derive(Debug)]
pub struct DesktopNotification {
    pub visibility: NotificationVisibility,
    pub revision: u64,
    pub title: String,
    pub body: String,
}

unsafe extern "C" fn desktop_notification(
    _: GhosttyTerminal,
    userdata: *mut c_void,
    notification: *const GhosttyTerminalDesktopNotification,
) {
    if notification.is_null() {
        return;
    }
    // SAFETY: Ghostty invokes this synchronously with borrowed strings valid
    // throughout the callback and our stable, exclusively accessed userdata.
    unsafe {
        let request = &*notification;
        let copy = |value: &GhosttyString, limit: usize| -> String {
            if value.len == 0 || value.ptr.is_null() {
                return String::new();
            }
            String::from_utf8_lossy(std::slice::from_raw_parts(value.ptr, value.len.min(limit)))
                .chars()
                .filter(|ch| !ch.is_control())
                .take(limit)
                .collect()
        };
        let state = &mut *userdata.cast::<Callbacks>();
        state.store_notification(copy(&request.title, 256), copy(&request.body, 1024), NotificationVisibility::Always);
    }
}

pub mod compression;
mod directory;
mod kitty_notification;
mod hyperlink;
pub mod keyboard;
pub mod mouse;
#[cfg(any(target_os = "macos", target_os = "linux"))]
mod process_wakeup;
pub mod program_status;
pub mod search;
pub mod search_worker;
pub mod selection;
pub mod session;
mod text_export;
mod shell_integration;

pub use rmux_core::scroll::Scroll;

/// Ghostty screen-page storage, excluding Rust snapshots, fonts and GPU assets.
#[derive(Clone, Copy, Debug)]
pub struct MemoryUsage {
    pub primary_resident_bytes: u64,
    pub alternate_resident_bytes: u64,
    pub virtual_bytes: u64,
    pub compressed_bytes: u64,
    pub image_bytes: u64,
    pub compression_supported: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Compression {
    Unsupported,
    Pending,
    Complete,
}

pub use rmux_core::terminal_frame::{
    Cell, Color, CursorShape, Frame, FrameStorage,
    DEFAULT_BACKGROUND, DEFAULT_CURSOR, DEFAULT_FOREGROUND,
};

impl From<GhosttyColorRgb> for Color {
    fn from(c: GhosttyColorRgb) -> Self {
        Self(c.r, c.g, c.b)
    }
}

pub struct Terminal {
    osc_capture: Option<Vec<u8>>,
    osc_discard: bool,
    selection: selection::Gesture,
    terminal: GhosttyTerminal,
    render: GhosttyRenderState,
    iterator: GhosttyRenderStateRowIterator,
    cells: GhosttyRenderStateRowCells,
    callbacks: Box<Callbacks>,
    frame: Frame,
    revision: u64,
    keyboard: keyboard::Encoder,
    mouse: mouse::Encoder,
}
// SAFETY: All handles are exclusively owned and never accessed without &mut self.
// The boxed callback state has a stable address across moves. No GUI/thread-local
// resources are used by libghostty-vt. Callers sharing Terminal must use a mutex.
unsafe impl Send for Terminal {}

fn check(result: GhosttyResult) -> Result<()> {
    ensure!(result == GHOSTTY_SUCCESS, "Ghostty VT error {result}");
    Ok(())
}
fn out<T>(value: &mut T) -> *mut c_void {
    ptr::from_mut(value).cast()
}
fn input<T>(value: &T) -> *const c_void {
    ptr::from_ref(value).cast()
}

unsafe extern "C" fn reply(
    _terminal: GhosttyTerminal,
    userdata: *mut c_void,
    bytes: *const u8,
    len: usize,
) {
    if len == 0 {
        return;
    }
    // SAFETY: registered userdata points to our stable boxed state and the callback
    // is synchronous during exclusive engine access. Ghostty owns bytes for len.
    unsafe {
        (&mut *userdata.cast::<Callbacks>())
            .replies
            .extend_from_slice(std::slice::from_raw_parts(bytes, len));
    }
}

unsafe extern "C" fn render_hold(_: GhosttyTerminal, userdata: *mut c_void, held: bool) {
    // SAFETY: the callback is synchronous and userdata is our stable owned box.
    unsafe {
        (*userdata.cast::<Callbacks>()).hold_started = held.then(Instant::now);
    }
}

unsafe extern "C" fn title_changed(_: GhosttyTerminal, userdata: *mut c_void) {
    // SAFETY: synchronous callback under exclusive access to the stable box.
    unsafe {
        let state = &mut *userdata.cast::<Callbacks>();
        state.title_revision = state.title_revision.wrapping_add(1);
    }
}

unsafe extern "C" fn bell(_: GhosttyTerminal, userdata: *mut c_void) {
    // SAFETY: synchronous callback under exclusive access to our stable box.
    unsafe {
        let state = &mut *userdata.cast::<Callbacks>();
        state.bell_revision = state.bell_revision.wrapping_add(1);
    }
}

unsafe extern "C" fn directory_changed(_: GhosttyTerminal, userdata: *mut c_void) {
    // SAFETY: synchronous callback under exclusive access to our stable box.
    unsafe {
        let state = &mut *userdata.cast::<Callbacks>();
        state.directory_revision = state.directory_revision.wrapping_add(1);
    }
}
unsafe extern "C" fn metadata_reset(terminal: GhosttyTerminal, userdata: *mut c_void) {
    // SAFETY: same registered userdata and synchronous access as the callbacks.
    unsafe {
        title_changed(terminal, userdata);
        directory_changed(terminal, userdata);
        (*userdata.cast::<Callbacks>()).progress = None;
        (*userdata.cast::<Callbacks>()).kitty_notifications = Default::default();
    }
}

impl Terminal {
    pub fn program_status(&self) -> &program_status::Records {
        &self.callbacks.program_status
    }
    pub fn finish_program_status(&mut self) {
        self.callbacks.program_finished = true;
        self.callbacks.program_status.finish_active();
        if self.callbacks.progress.is_some_and(|progress| progress.state != ProgressState::Error) {
            self.callbacks.progress = None;
        }
    }
    pub fn acknowledge_program_status(&mut self) {
        self.callbacks.program_status.acknowledge();
    }
    pub fn progress(&self) -> Option<Progress> {
        self.callbacks.progress
    }
    pub fn notification_since(&self, revision: u64) -> Option<Arc<DesktopNotification>> {
        self.callbacks
            .notification
            .as_ref()
            .filter(|notification| notification.revision != revision)
            .cloned()
    }
    /// Ordered retained notifications after the supplied revision. If a consumer
    /// fell behind the bounded ring, deliver all messages still retained.
    pub fn notifications_since(&self, revision: u64) -> Vec<Arc<DesktopNotification>> {
        if self.callbacks.notification.as_ref().is_none_or(|message| message.revision == revision) {
            return Vec::new();
        }
        let history = &self.callbacks.notification_history;
        let start = history.iter().position(|message| message.revision == revision).map_or(0, |index| index + 1);
        history.iter().skip(start).cloned().collect()
    }
    pub fn directory_since(
        &self,
        revision: u64,
    ) -> Result<Option<(u64, Option<std::path::PathBuf>)>> {
        if revision == self.callbacks.directory_revision {
            return Ok(None);
        }
        Ok(Some((
            self.callbacks.directory_revision,
            self.current_directory()?,
        )))
    }
    pub fn bell_revision(&self) -> u64 {
        self.callbacks.bell_revision
    }
    pub fn current_directory(&self) -> Result<Option<std::path::PathBuf>> {
        self.with_directory_uri(|uri| directory::local_path(uri, directory::local_hostname().as_deref()))
    }
    pub fn remote_directory_since(&self, revision: u64) -> Result<Option<(u64, Option<String>)>> {
        if revision == self.callbacks.directory_revision { return Ok(None); }
        Ok(Some((self.callbacks.directory_revision, self.with_directory_uri(directory::remote_label)?)))
    }
    fn with_directory_uri<T>(&self, convert: impl FnOnce(&str) -> Option<T>) -> Result<Option<T>> {
        let mut value = GhosttyString {
            ptr: ptr::null(),
            len: 0,
        };
        // SAFETY: the borrowed string is consumed under exclusive engine access.
        unsafe {
            check(ghostty_terminal_get(
                self.terminal,
                GHOSTTY_TERMINAL_DATA_PWD,
                out(&mut value),
            ))?;
            if value.len == 0 || value.len > 8192 {
                return Ok(None);
            }
            let Ok(uri) = std::str::from_utf8(std::slice::from_raw_parts(value.ptr, value.len))
            else {
                return Ok(None);
            };
            Ok(convert(uri))
        }
    }
    /// Return a bounded, owned UI title only when Ghostty reports a change.
    pub fn title_since(&self, revision: u64) -> Result<Option<(u64, String)>> {
        if revision == self.callbacks.title_revision {
            return Ok(None);
        }
        let mut value = GhosttyString {
            ptr: ptr::null(),
            len: 0,
        };
        // SAFETY: exclusive terminal ownership keeps the borrowed bytes valid
        // until copied; an empty GhosttyString may have a null pointer.
        unsafe {
            check(ghostty_terminal_get(
                self.terminal,
                GHOSTTY_TERMINAL_DATA_TITLE,
                out(&mut value),
            ))?;
            let title = if value.len == 0 {
                String::new()
            } else {
                String::from_utf8_lossy(std::slice::from_raw_parts(value.ptr, value.len.min(1024)))
                    .chars()
                    .filter(|c| !c.is_control())
                    .take(128)
                    .collect()
            };
            Ok(Some((self.callbacks.title_revision, title)))
        }
    }
    pub fn new(columns: u16, rows: u16) -> Result<Self> {
        ensure!(
            columns > 0 && rows > 0,
            "terminal dimensions must be nonzero"
        );
        let mut this = Self {
            osc_capture: None,
            osc_discard: false,
            selection: selection::Gesture::new()?,
            mouse: mouse::Encoder::new()?,
            keyboard: keyboard::Encoder::new()?,
            terminal: ptr::null_mut(),
            render: ptr::null_mut(),
            iterator: ptr::null_mut(),
            cells: ptr::null_mut(),
            callbacks: Box::default(),
            revision: 1,
            frame: Frame {
                revision: 0,
                selections: Arc::new(vec![None; rows as usize]),
                columns,
                rows: Arc::new(vec![Arc::default(); rows as usize]),
                background: DEFAULT_BACKGROUND,
                cursor: None,
                cursor_shape: CursorShape::Block,
                cursor_color: DEFAULT_CURSOR,
                cursor_color_explicit: false,
                scrollback_lines: 0,
                scrollback_offset: 0,
            },
        };
        // SAFETY: constructors initialize owned handles. Drop handles partial init.
        unsafe {
            check(ghostty_terminal_new(
                ptr::null(),
                &mut this.terminal,
                columns,
                rows,
            ))?;
            check(ghostty_render_state_new(ptr::null(), &mut this.render))?;
            check(ghostty_render_state_row_iterator_new(
                ptr::null(),
                &mut this.iterator,
            ))?;
            check(ghostty_render_state_row_cells_new(
                ptr::null(),
                &mut this.cells,
            ))?;
            check(ghostty_terminal_set(
                this.terminal,
                GHOSTTY_TERMINAL_OPT_USERDATA,
                out(&mut *this.callbacks),
            ))?;
            check(ghostty_terminal_set(
                this.terminal,
                GHOSTTY_TERMINAL_OPT_WRITE_PTY,
                reply as *const c_void,
            ))?;
            check(ghostty_terminal_set(
                this.terminal,
                GHOSTTY_TERMINAL_OPT_RENDER_HOLD,
                render_hold as *const c_void,
            ))?;
            check(ghostty_terminal_set(
                this.terminal,
                GHOSTTY_TERMINAL_OPT_BELL,
                bell as *const c_void,
            ))?;
            check(ghostty_terminal_set(
                this.terminal,
                GHOSTTY_TERMINAL_OPT_DESKTOP_NOTIFICATION,
                desktop_notification as *const c_void,
            ))?;
            check(ghostty_terminal_set(
                this.terminal,
                GHOSTTY_TERMINAL_OPT_PROGRESS_REPORT,
                progress_report as *const c_void,
            ))?;
            check(ghostty_terminal_set(
                this.terminal,
                GHOSTTY_TERMINAL_OPT_PROGRAM_STATUS,
                program_status::report as *const c_void,
            ))?;
            check(ghostty_terminal_set(
                this.terminal,
                GHOSTTY_TERMINAL_OPT_SEMANTIC_PROMPT,
                program_status::prompt as *const c_void,
            ))?;
            check(ghostty_terminal_set(
                this.terminal,
                GHOSTTY_TERMINAL_OPT_TITLE_CHANGED,
                title_changed as *const c_void,
            ))?;
            check(ghostty_terminal_set(
                this.terminal,
                GHOSTTY_TERMINAL_OPT_PWD_CHANGED,
                directory_changed as *const c_void,
            ))?;
            check(ghostty_terminal_set(
                this.terminal,
                GHOSTTY_TERMINAL_OPT_RESET,
                metadata_reset as *const c_void,
            ))?;
            let limit = rmux_core::scroll::DEFAULT_SCROLLBACK_BYTES;
            check(ghostty_terminal_set(
                this.terminal,
                GHOSTTY_TERMINAL_OPT_SCROLLBACK_MAX_BYTES,
                input(&limit),
            ))?;
            let fg = GhosttyColorRgb {
                r: DEFAULT_FOREGROUND.0,
                g: DEFAULT_FOREGROUND.1,
                b: DEFAULT_FOREGROUND.2,
            };
            let bg = GhosttyColorRgb {
                r: DEFAULT_BACKGROUND.0,
                g: DEFAULT_BACKGROUND.1,
                b: DEFAULT_BACKGROUND.2,
            };
            check(ghostty_terminal_set(
                this.terminal,
                GHOSTTY_TERMINAL_OPT_COLOR_FOREGROUND,
                input(&fg),
            ))?;
            check(ghostty_terminal_set(
                this.terminal,
                GHOSTTY_TERMINAL_OPT_COLOR_BACKGROUND,
                input(&bg),
            ))?;
        }
        Ok(this)
    }

    pub fn write(&mut self, bytes: &[u8]) {
        self.revision = self.revision.wrapping_add(1);
        self.mouse.refresh = true;
        // A supported notification starts with literal OSC + "99;". A chunk
        // without '9' cannot contain it. Keep the boundary path if an OSC is
        // already captured, or if an introducer could be split at this suffix.
        // Other OSCs can remain entirely on Ghostty's normal bulk path.
        let partial_introducer = bytes.last().is_some_and(|byte| matches!(byte, 0x1b | b']' | 0x9d));
        if self.osc_capture.is_none() && !partial_introducer && memchr::memchr(b'9', bytes).is_none() {
            // SAFETY: exclusively owned terminal and valid borrowed input.
            unsafe { ghostty_terminal_vt_write(self.terminal, bytes.as_ptr(), bytes.len()); }
        } else {
            self.write_with_osc_boundaries(bytes);
        }
    }

    fn write_with_osc_boundaries(&mut self, bytes: &[u8]) {
        let mut remaining = bytes;
        while !remaining.is_empty() {
            let mut stop = GhosttyTerminalWriteStop { size: std::mem::size_of::<GhosttyTerminalWriteStop>(), ..Default::default() };
            // SAFETY: owned terminal and borrowed input remain valid. Ghostty
            // handles UTF-8, nested control strings, partial input and cancellation.
            let result = unsafe { ghostty_terminal_vt_write_until(self.terminal, remaining.as_ptr(), remaining.len(),
                (GHOSTTY_TERMINAL_BOUNDARY_OSC_START | GHOSTTY_TERMINAL_BOUNDARY_OSC_END) as u32, &mut stop) };
            if !matches!(result, GHOSTTY_SUCCESS | GHOSTTY_NO_VALUE) || stop.consumed == 0 || stop.consumed > remaining.len() { break; }
            let end = stop.reached & GHOSTTY_TERMINAL_BOUNDARY_OSC_END as u32 != 0;
            if let Some(capture) = &mut self.osc_capture {
                if !self.osc_discard {
                    let data = &remaining[..stop.consumed - usize::from(end)];
                    if capture.len() + data.len() > 8192 || !capture.iter().chain(data.iter()).zip(b"99;").all(|(a, b)| a == b) {
                        self.osc_discard = true;
                        capture.clear();
                    } else {
                        capture.extend_from_slice(data);
                    }
                }
            }
            if end {
                if !self.osc_discard && matches!(stop.end, GHOSTTY_TERMINAL_STRING_END_ST_ESC | GHOSTTY_TERMINAL_STRING_END_BEL) {
                    if let Some(response) = self.osc_capture.as_deref().and_then(kitty_notification::query_response) {
                        let terminator: &[u8] = if stop.end == GHOSTTY_TERMINAL_STRING_END_BEL { b"\x07" } else { b"\x1b\\" };
                        // Bound our query replies between PTY drains. Never append a
                        // partial response; subsequent terminal replies remain ordered.
                        if self.callbacks.replies.len() + response.len() + terminator.len() <= 64 * 1024 {
                            self.callbacks.replies.extend_from_slice(response.as_bytes());
                            self.callbacks.replies.extend_from_slice(terminator);
                        }
                    }
                    if let Some((title, body, visibility)) = self.osc_capture.as_deref().and_then(|bytes| self.callbacks.kitty_notifications.receive(bytes)) {
                        self.callbacks.store_notification(title, body, visibility);
                    }
                }
                self.osc_capture = None;
            }
            if stop.reached & GHOSTTY_TERMINAL_BOUNDARY_OSC_START as u32 != 0 {
                self.osc_capture = Some(Vec::new());
                self.osc_discard = false;
            }
            remaining = &remaining[stop.consumed..];
        }
    }

    pub fn take_replies(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.callbacks.replies)
    }
    pub fn scroll(&mut self, scroll: Scroll) -> Result<()> {
        if matches!(scroll, Scroll::LastPrompt) {
            // Begin near the end, then advance through any prompt inside the
            // active viewport. Going backward alone can skip a visible prompt.
            self.scroll(Scroll::Bottom)?;
            self.scroll(Scroll::Prompts(-1))?;
            return self.scroll(Scroll::Prompts(isize::MAX));
        }
        self.revision = self.revision.wrapping_add(1);
        let (tag, delta) = match scroll {
            Scroll::Row(_) => (GHOSTTY_TERMINAL_SCROLL_VIEWPORT_ROW, 0),
            Scroll::Lines(delta) => (GHOSTTY_TERMINAL_SCROLL_VIEWPORT_DELTA, delta),
            Scroll::Prompts(delta) => (GHOSTTY_TERMINAL_SCROLL_VIEWPORT_DELTA_PROMPT, delta),
            Scroll::Top => (GHOSTTY_TERMINAL_SCROLL_VIEWPORT_TOP, 0),
            Scroll::Bottom => (GHOSTTY_TERMINAL_SCROLL_VIEWPORT_BOTTOM, 0),
            Scroll::LastPrompt => unreachable!(),
        };
        // SAFETY: owned terminal and correctly tagged viewport union.
        unsafe {
            check(ghostty_terminal_scroll_viewport(
                self.terminal,
                GhosttyTerminalScrollViewport {
                    tag,
                    value: if let Scroll::Row(row) = scroll {
                        GhosttyTerminalScrollViewportValue { row: usize::try_from(row).unwrap_or(usize::MAX) }
                    } else if matches!(scroll, Scroll::Prompts(_)) {
                        GhosttyTerminalScrollViewportValue {
                            delta_prompt: delta,
                        }
                    } else {
                        GhosttyTerminalScrollViewportValue { delta }
                    },
                },
            ))
        }
    }
    /// One bounded storage-only step. Call again on Pending while idle; normal
    /// terminal access transparently restores any compressed pages it needs.
    pub fn compress_scrollback_step(&mut self) -> Result<Compression> {
        let mut result = GHOSTTY_TERMINAL_COMPRESSION_RESULT_UNSUPPORTED;
        unsafe {
            check(ghostty_terminal_compress(
                self.terminal,
                GHOSTTY_TERMINAL_COMPRESSION_MODE_INCREMENTAL,
                &mut result,
            ))?;
        }
        match result {
            GHOSTTY_TERMINAL_COMPRESSION_RESULT_UNSUPPORTED => Ok(Compression::Unsupported),
            GHOSTTY_TERMINAL_COMPRESSION_RESULT_PENDING => Ok(Compression::Pending),
            GHOSTTY_TERMINAL_COMPRESSION_RESULT_COMPLETE => Ok(Compression::Complete),
            _ => anyhow::bail!("unknown compression result {result}"),
        }
    }
    pub fn compression_activity(&self) -> Result<u64> {
        let mut token = 0;
        unsafe {
            check(ghostty_terminal_compression_activity(
                self.terminal,
                &mut token,
            ))?;
        }
        Ok(token)
    }
    pub fn memory_usage(&self) -> Result<MemoryUsage> {
        let mut usage = GhosttyTerminalMemoryUsage {
            size: std::mem::size_of::<GhosttyTerminalMemoryUsage>(),
            ..Default::default()
        };
        // SAFETY: sized output is valid for this serialized terminal access.
        unsafe {
            check(ghostty_terminal_get(
                self.terminal,
                GHOSTTY_TERMINAL_DATA_MEMORY_USAGE,
                out(&mut usage),
            ))?;
        }
        Ok(MemoryUsage {
            primary_resident_bytes: usage.primary_resident_bytes,
            alternate_resident_bytes: usage.alternate_resident_bytes,
            virtual_bytes: usage
                .primary_virtual_bytes
                .saturating_add(usage.alternate_virtual_bytes),
            compressed_bytes: usage
                .primary_compressed_bytes
                .saturating_add(usage.alternate_compressed_bytes),
            image_bytes: usage
                .primary_image_bytes
                .saturating_add(usage.alternate_image_bytes),
            compression_supported: usage.compression_supported,
        })
    }
    pub fn clear_scrollback(&mut self) -> Result<()> {
        self.revision = self.revision.wrapping_add(1);
        self.selection.reset(self.terminal);
        self.clear_selection()?;
        // Change retention through the API, never by injecting bytes into a
        // potentially incomplete application escape sequence.
        unsafe {
            let mut limit: usize = 0;
            check(ghostty_terminal_get(
                self.terminal,
                GHOSTTY_TERMINAL_DATA_SCROLLBACK_MAX_BYTES,
                out(&mut limit),
            ))?;
            let zero: usize = 0;
            check(ghostty_terminal_set(
                self.terminal,
                GHOSTTY_TERMINAL_OPT_SCROLLBACK_MAX_BYTES,
                input(&zero),
            ))?;
            check(ghostty_terminal_set(
                self.terminal,
                GHOSTTY_TERMINAL_OPT_SCROLLBACK_MAX_BYTES,
                input(&limit),
            ))?;
        }
        Ok(())
    }
    /// Update configured defaults without replacing application OSC overrides.
    /// Missing foreground/background restore rmux defaults; cursor uses fallback.
    pub fn set_default_colors(&mut self, colors: [Option<Color>; 3]) -> Result<()> {
        self.revision = self.revision.wrapping_add(1);
        let colors = [
            colors[0].or(Some(DEFAULT_FOREGROUND)),
            colors[1].or(Some(DEFAULT_BACKGROUND)),
            colors[2],
        ];
        for (option, color) in [
            GHOSTTY_TERMINAL_OPT_COLOR_FOREGROUND,
            GHOSTTY_TERMINAL_OPT_COLOR_BACKGROUND,
            GHOSTTY_TERMINAL_OPT_COLOR_CURSOR,
        ].into_iter().zip(colors) {
            let value = color.map(|Color(r, g, b)| GhosttyColorRgb { r, g, b });
            // SAFETY: exclusive handle; option expects RGB or a null reset.
            unsafe {
                check(ghostty_terminal_set(
                    self.terminal,
                    option,
                    value.as_ref().map_or(ptr::null(), input),
                ))?;
            }
        }
        Ok(())
    }
    pub fn set_palette(&mut self, colors: Option<[Color; 256]>) -> Result<()> {
        self.revision = self.revision.wrapping_add(1);
        let palette = colors.map(|colors| colors.map(|Color(r,g,b)| GhosttyColorRgb { r,g,b }));
        // SAFETY: the tagged option reads exactly 256 RGB entries synchronously;
        // null restores factory defaults. Engine-owned OSC overrides are retained.
        unsafe { check(ghostty_terminal_set(self.terminal, GHOSTTY_TERMINAL_OPT_COLOR_PALETTE, palette.as_ref().map_or(ptr::null(), input))) }
    }
    pub fn set_ansi_palette(&mut self, colors: Option<[Color; 16]>) -> Result<()> {
        self.revision = self.revision.wrapping_add(1);
        // SAFETY: both tagged APIs require exactly 256 RGB values. Read defaults
        // rather than effective colors so OSC overrides never become defaults.
        unsafe {
            if let Some(colors) = colors {
                let mut palette = [GhosttyColorRgb::default(); 256];
                check(ghostty_terminal_get(self.terminal, GHOSTTY_TERMINAL_DATA_COLOR_PALETTE_DEFAULT, out(&mut palette)))?;
                for (slot, Color(r, g, b)) in palette.iter_mut().zip(colors) {
                    *slot = GhosttyColorRgb { r, g, b };
                }
                check(ghostty_terminal_set(self.terminal, GHOSTTY_TERMINAL_OPT_COLOR_PALETTE, input(&palette)))
            } else {
                check(ghostty_terminal_set(self.terminal, GHOSTTY_TERMINAL_OPT_COLOR_PALETTE, ptr::null()))
            }
        }
    }
    pub fn set_scrollback_limit(&mut self, bytes: usize) -> Result<()> {
        self.revision = self.revision.wrapping_add(1);
        self.selection.reset(self.terminal);
        self.clear_selection()?;
        // SAFETY: owned terminal, usize payload required by this option.
        unsafe {
            check(ghostty_terminal_set(
                self.terminal,
                GHOSTTY_TERMINAL_OPT_SCROLLBACK_MAX_BYTES,
                input(&bytes),
            ))
        }
    }
    pub fn resize(&mut self, columns: u16, rows: u16) -> Result<()> {
        self.revision = self.revision.wrapping_add(1);
        ensure!(
            columns > 0 && rows > 0,
            "terminal dimensions must be nonzero"
        );
        unsafe {
            check(ghostty_terminal_resize(self.terminal, columns, rows, 0, 0))?;
        }
        self.frame.columns = columns;
        Arc::make_mut(&mut self.frame.rows).resize_with(rows as usize, Arc::default);
        Arc::make_mut(&mut self.frame.selections).resize(rows as usize, None);
        Ok(())
    }

    /// Updates changed rows only; unchanged rows share their existing allocations.
    pub fn frame(&mut self) -> Result<Frame> {
        if self.render_delay().is_some() {
            return Ok(self.frame.clone());
        }
        if self.callbacks.hold_started.take().is_some() {
            // A crashed producer must not freeze the screen indefinitely.
            let mode = GhosttyTerminalModeConfig {
                mode: 2026,
                value: false,
            };
            unsafe {
                check(ghostty_terminal_set(
                    self.terminal,
                    GHOSTTY_TERMINAL_OPT_MODE,
                    input(&mode),
                ))?;
            }
        }
        // SAFETY: all queried types match Ghostty's tagged output API. Iterators
        // are consumed before the next update and all cell text is copied out.
        unsafe {
            check(ghostty_render_state_update(self.render, self.terminal))?;
            let mut scrollbar = GhosttyTerminalScrollbar::default();
            check(ghostty_terminal_get(
                self.terminal,
                GHOSTTY_TERMINAL_DATA_SCROLLBAR,
                out(&mut scrollbar),
            ))?;
            self.frame.scrollback_lines = scrollbar.total.saturating_sub(scrollbar.len);
            self.frame.scrollback_offset =
                self.frame.scrollback_lines.saturating_sub(scrollbar.offset);
            let mut colors = GhosttyRenderStateColors {
                size: size_of::<GhosttyRenderStateColors>(),
                ..Default::default()
            };
            check(ghostty_render_state_get(
                self.render,
                GHOSTTY_RENDER_STATE_DATA_COLORS,
                out(&mut colors),
            ))?;
            self.frame.background = colors.background.into();
            self.frame.cursor_color_explicit = colors.cursor_has_value;
            self.frame.cursor_color = if colors.cursor_has_value {
                colors.cursor.into()
            } else {
                DEFAULT_CURSOR
            };
            let mut cursor = GhosttyRenderStateCursor {
                size: size_of::<GhosttyRenderStateCursor>(),
                ..Default::default()
            };
            check(ghostty_render_state_get(
                self.render,
                GHOSTTY_RENDER_STATE_DATA_CURSOR,
                out(&mut cursor),
            ))?;
            self.frame.cursor = (cursor.visible && cursor.viewport_has_value).then_some((
                cursor
                    .viewport_x
                    .saturating_sub(u16::from(cursor.wide_tail)),
                cursor.viewport_y,
            ));
            self.frame.cursor_shape = match cursor.visual_style {
                GHOSTTY_CURSOR_STYLE_BAR => CursorShape::Bar,
                GHOSTTY_CURSOR_STYLE_UNDERLINE => CursorShape::Underline,
                GHOSTTY_CURSOR_STYLE_BLOCK_HOLLOW => CursorShape::Hollow,
                _ => CursorShape::Block,
            };
            check(ghostty_render_state_get(
                self.render,
                GHOSTTY_RENDER_STATE_DATA_ROW_ITERATOR,
                out(&mut self.iterator),
            ))?;
            let mut y = 0;
            let mut inline_glyph = [0u8; 64];
            let mut overflow_glyph = Vec::new();
            while ghostty_render_state_row_iterator_next_dirty(self.iterator, &mut y) {
                let mut selection = GhosttyRenderStateRowSelection {
                    size: size_of::<GhosttyRenderStateRowSelection>(),
                    ..Default::default()
                };
                let result = ghostty_render_state_row_get(
                    self.iterator,
                    GHOSTTY_RENDER_STATE_ROW_DATA_SELECTION,
                    out(&mut selection),
                );
                if result != GHOSTTY_NO_VALUE {
                    check(result)?;
                }
                let next_selection =
                    (result == GHOSTTY_SUCCESS).then_some((selection.start_x, selection.end_x));
                if self.frame.selections[y as usize] != next_selection {
                    Arc::make_mut(&mut self.frame.selections)[y as usize] = next_selection;
                }
                check(ghostty_render_state_row_get(
                    self.iterator,
                    GHOSTTY_RENDER_STATE_ROW_DATA_CELLS,
                    out(&mut self.cells),
                ))?;
                let previous = &self.frame.rows[y as usize];
                let mut row: Option<Vec<Cell>> = None;
                // Raw-cell equality includes packed background colors. Keep
                // this cache row-local because style IDs are page-local.
                let mut last_style = None;
                let mut x = 0;
                while ghostty_render_state_row_cells_next(self.cells) {
                    let mut raw = GhosttyCell::default();
                    check(ghostty_render_state_row_cells_get(
                        self.cells,
                        GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_RAW,
                        out(&mut raw),
                    ))?;
                    let mut wide = GHOSTTY_CELL_WIDE_NARROW;
                    check(ghostty_cell_get(
                        raw,
                        GHOSTTY_CELL_DATA_WIDE,
                        out(&mut wide),
                    ))?;
                    let width = match wide {
                        GHOSTTY_CELL_WIDE_WIDE => 2,
                        GHOSTTY_CELL_WIDE_SPACER_TAIL | GHOSTTY_CELL_WIDE_SPACER_HEAD => 0,
                        _ => 1,
                    };
                    let scratch = if overflow_glyph.is_empty() { &mut inline_glyph[..] } else { &mut overflow_glyph[..] };
                    let mut buf = GhosttyBuffer {
                        ptr: scratch.as_mut_ptr(),
                        cap: scratch.len(),
                        len: 0,
                    };
                    let result = ghostty_render_state_row_cells_get(
                        self.cells,
                        GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_GRAPHEMES_UTF8,
                        out(&mut buf),
                    );
                    if result == GHOSTTY_OUT_OF_SPACE {
                        overflow_glyph.resize(buf.len, 0);
                        buf.ptr = overflow_glyph.as_mut_ptr();
                        buf.cap = overflow_glyph.len();
                        check(ghostty_render_state_row_cells_get(
                            self.cells,
                            GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_GRAPHEMES_UTF8,
                            out(&mut buf),
                        ))?;
                    } else {
                        check(result)?;
                    }
                    let (style, fg, bg, underline_color) = match last_style {
                        Some((previous, style, fg, bg, underline)) if previous == raw => (style, fg, bg, underline),
                        _ => {
                            let mut style = GhosttyStyle::default();
                            check(ghostty_render_state_row_cells_get(
                                self.cells,
                                GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_STYLE,
                                out(&mut style),
                            ))?;
                            let mut fg = resolve_color(style.fg_color, &colors, colors.foreground);
                            let mut bg = resolve_color(style.bg_color, &colors, colors.background);
                            if style.flags & 16 != 0 {
                                std::mem::swap(&mut fg, &mut bg);
                            }
                            let mut underline_color = resolve_color(style.underline_color, &colors, fg);
                            if style.flags & 4 != 0 {
                                fg = dim_color(fg, bg);
                                underline_color = dim_color(underline_color, bg);
                            }
                            if style.flags & 32 != 0 {
                                fg = bg;
                                underline_color = bg;
                            }
                            last_style = Some((raw, style, fg, bg, underline_color));
                            (style, fg, bg, underline_color)
                        }
                    };
                    let glyph_bytes = if overflow_glyph.is_empty() { &inline_glyph[..] } else { &overflow_glyph[..] };
                    let text = String::from_utf8_lossy(&glyph_bytes[..buf.len]);
                    let previous_cell = previous.get(x as usize);
                    let text = match previous_cell {
                        Some(cell) if cell.text.is_heap_allocated() && cell.text.as_str() == text.as_ref() => cell.text.clone(),
                        _ => smol_str::SmolStr::new(text),
                    };
                    let cell = Cell {
                        text,
                        column: x,
                        width,
                        foreground: fg.into(),
                        background: bg.into(),
                        bold: style.flags & 1 != 0,
                        italic: style.flags & 2 != 0,
                        underline: style.flags & 0x700 != 0,
                        underline_color: underline_color.into(),
                        underline_curly: (style.flags >> 8) & 7
                            == GHOSTTY_SGR_UNDERLINE_CURLY as u16,
                        strikethrough: style.flags & 64 != 0,
                        overline: style.flags & 128 != 0,
                        hidden: style.flags & 32 != 0,
                    };
                    if row.is_none() && previous_cell != Some(&cell) {
                        let mut changed = Vec::with_capacity(self.frame.columns as usize);
                        changed.extend_from_slice(&previous[..(x as usize).min(previous.len())]);
                        row = Some(changed);
                    }
                    if let Some(row) = &mut row { row.push(cell); }
                    x += 1;
                }
                // A shortened row is a change even when every remaining cell matches.
                if row.is_none() && (x as usize) < previous.len() {
                    row = Some(previous[..x as usize].to_vec());
                }
                if let Some(row) = row {
                    Arc::make_mut(&mut self.frame.rows)[y as usize] = Arc::new(row);
                }
            }
            // The owned snapshot has consumed every dirty row. Clearing only
            // the global flag leaves stale per-row flags for the next update.
            check(ghostty_render_state_clean(self.render))?;
        }
        self.frame.revision = self.revision;
        Ok(self.frame.clone())
    }

    pub fn render_delay(&self) -> Option<Duration> {
        self.callbacks.hold_started.and_then(|started| {
            RENDER_HOLD_LIMIT
                .checked_sub(started.elapsed())
                .filter(|delay| !delay.is_zero())
        })
    }
}

fn resolve_color(
    color: GhosttyStyleColor,
    colors: &GhosttyRenderStateColors,
    fallback: GhosttyColorRgb,
) -> GhosttyColorRgb {
    // SAFETY: the tag selects the initialized union member per the C API.
    unsafe {
        match color.tag as GhosttyStyleColorTag {
            GHOSTTY_STYLE_COLOR_RGB => color.value.rgb,
            GHOSTTY_STYLE_COLOR_PALETTE => colors.palette[color.value.palette as usize],
            _ => fallback,
        }
    }
}
fn dim_color(foreground: GhosttyColorRgb, background: GhosttyColorRgb) -> GhosttyColorRgb {
    let blend = |a: u8, b: u8| ((u16::from(a) + u16::from(b)) / 2) as u8;
    GhosttyColorRgb {
        r: blend(foreground.r, background.r),
        g: blend(foreground.g, background.g),
        b: blend(foreground.b, background.b),
    }
}
impl Drop for Terminal {
    fn drop(&mut self) {
        self.selection.reset(self.terminal);
        // Free functions accept null, including when construction failed halfway.
        unsafe {
            ghostty_render_state_row_cells_free(self.cells);
            ghostty_render_state_row_iterator_free(self.iterator);
            ghostty_render_state_free(self.render);
            ghostty_terminal_free(self.terminal);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    #[test]
    fn shell_directory_report_tracks_changes_and_reset() -> Result<()> {
        let mut terminal = Terminal::new(20, 3)?;
        assert!(terminal.current_directory()?.is_none());
        terminal.write(b"\x1b]7;file://localhost/tmp/hello%20world\x07");
        assert_eq!(
            terminal.current_directory()?,
            Some(std::path::PathBuf::from("/tmp/hello world"))
        );
        terminal.write(b"\x1b]7;file://another-remote-host/tmp\x07");
        assert!(terminal.current_directory()?.is_none());
        terminal.write(b"\x1b]7;file:///tmp\x07\x1bc");
        assert!(terminal.current_directory()?.is_none());
        Ok(())
    }
    #[test]
    fn synchronized_output_holds_then_publishes_with_bounded_recovery() -> Result<()> {
        let mut terminal = Terminal::new(20, 3)?;
        terminal.write(b"before");
        let before = terminal.frame()?;
        terminal.write(b"\x1b[?2026h\r\x1b[2Kafter");
        assert!(terminal.render_delay().is_some());
        let held = terminal.frame()?;
        assert!(Arc::ptr_eq(&before.rows[0], &held.rows[0]));
        terminal.write(b"\x1b[?2026l");
        assert!(terminal.render_delay().is_none());
        assert!(terminal.frame()?.text().starts_with("after"));

        terminal.write(b"\x1b[?2026h\r\x1b[2Ktimeout");
        let started = terminal.callbacks.hold_started;
        terminal.write(b"\x1b[?2026h");
        assert_eq!(terminal.callbacks.hold_started, started);
        // Advance the stored deadline instead of sleeping in a unit test.
        terminal.callbacks.hold_started = Some(Instant::now() - RENDER_HOLD_LIMIT);
        assert!(terminal.frame()?.text().starts_with("timeout"));
        assert!(terminal.callbacks.hold_started.is_none());
        // Timeout clears engine mode too, permitting a fresh hold callback.
        terminal.write(b"\x1b[?2026h");
        assert!(terminal.render_delay().is_some());
        terminal.resize(21, 3)?;
        assert!(terminal.render_delay().is_none());
        terminal.write(b"\x1b[?2026h\x1bc");
        assert!(terminal.render_delay().is_none());
        Ok(())
    }
    #[test]
    fn snapshot_repeated_cells_keep_packed_backgrounds_and_row_styles() -> Result<()> {
        let mut terminal = Terminal::new(12, 3)?;
        terminal.write(b"\x1b[48;2;10;20;30m    \x1b[48;2;40;50;60m    \x1b[0m\r\n\x1b[1;38;2;70;80;90mxxxx\x1b[0mxxxx");
        let frame = terminal.frame()?;
        for cell in &frame.rows[0][..4] { assert_eq!(cell.background, Color(10, 20, 30)); }
        for cell in &frame.rows[0][4..8] { assert_eq!(cell.background, Color(40, 50, 60)); }
        for cell in &frame.rows[1][..4] { assert!(cell.bold); assert_eq!(cell.foreground, Color(70, 80, 90)); }
        for cell in &frame.rows[1][4..8] { assert!(!cell.bold); }
        terminal.write(b"\x1b[1;1H\x1b[48;2;90;80;70m    ");
        let changed = terminal.frame()?;
        assert_eq!(changed.rows[0][0].background, Color(90, 80, 70));
        assert_eq!(frame.rows[0][0].background, Color(10, 20, 30));
        Ok(())
    }
    #[test]
    fn exposes_text_decorations_and_cursor_styles() -> Result<()> {
        let mut terminal = Terminal::new(20, 3)?;
        terminal.write(b"\x1b[2mA\x1b[0;9;53mB\x1b[0;4:3;58:2::1:2:3mC\x1b[0;8mD\x1b[0m");
        let frame = terminal.frame()?;
        assert_eq!(frame.rows[0][0].foreground, Color(119, 121, 128));
        assert!(frame.rows[0][1].strikethrough && frame.rows[0][1].overline);
        assert!(frame.rows[0][2].underline && frame.rows[0][2].underline_curly);
        assert_eq!(frame.rows[0][2].underline_color, Color(1, 2, 3));
        assert!(frame.rows[0][3].hidden);
        for (sequence, shape) in [
            (b"\x1b[2 q".as_slice(), CursorShape::Block),
            (b"\x1b[4 q".as_slice(), CursorShape::Underline),
            (b"\x1b[6 q".as_slice(), CursorShape::Bar),
        ] {
            terminal.write(sequence);
            assert_eq!(terminal.frame()?.cursor_shape, shape);
        }
        terminal.write(b"\x1b]12;#123456\x07");
        assert_eq!(terminal.frame()?.cursor_color, Color(0x12, 0x34, 0x56));
        terminal.write(b"\x1b[?25l");
        assert_eq!(terminal.frame()?.cursor, None);
        Ok(())
    }
    #[test]
    fn prompt_navigation_uses_semantic_markers_and_ignores_unmarked_output() -> Result<()> {
        let mut terminal = Terminal::new(30, 3)?;
        for command in 0..4 {
            terminal.write(format!("\x1b]133;A\x07prompt-{command}> \x1b]133;B\x07run\r\n\x1b]133;C\x07output-{command}\r\nmore\r\n\x1b]133;D;0\x07").as_bytes());
        }
        terminal.scroll(Scroll::Top)?;
        assert!(terminal.frame()?.text().starts_with("prompt-0>"));
        terminal.scroll(Scroll::Prompts(1))?;
        assert!(terminal.frame()?.text().starts_with("prompt-1>"));
        terminal.scroll(Scroll::Prompts(1))?;
        assert!(terminal.frame()?.text().starts_with("prompt-2>"));
        terminal.scroll(Scroll::Prompts(-1))?;
        assert!(terminal.frame()?.text().starts_with("prompt-1>"));
        terminal.scroll(Scroll::Prompts(-100))?;
        assert!(terminal.frame()?.text().starts_with("prompt-0>"));
        terminal.write(b"long output\r\nline\r\nline\r\nline\r\n");
        terminal.scroll(Scroll::LastPrompt)?;
        assert!(terminal.frame()?.text().starts_with("prompt-3>"));
        terminal.scroll(Scroll::Top)?;
        terminal.scroll(Scroll::LastPrompt)?;
        assert!(terminal.frame()?.text().starts_with("prompt-3>"));
        terminal.write(b"\x1b]133;A\x07latest> \x1b]133;B\x07");
        terminal.scroll(Scroll::LastPrompt)?;
        assert_eq!(terminal.frame()?.scrollback_offset, 0);
        assert!(terminal.frame()?.text().contains("latest>"));
        let mut plain = Terminal::new(20, 3)?;
        plain.write(b"one\r\ntwo\r\nthree\r\nfour\r\nfive");
        let before = plain.frame()?.scrollback_offset;
        plain.scroll(Scroll::Prompts(-1))?;
        assert_eq!(plain.frame()?.scrollback_offset, before);
        plain.scroll(Scroll::Top)?;
        plain.scroll(Scroll::LastPrompt)?;
        assert_eq!(plain.frame()?.scrollback_offset, 0);
        Ok(())
    }
    #[test]
    fn incremental_compression_preserves_history_and_future_output() -> Result<()> {
        let mut terminal = Terminal::new(80, 24)?;
        for line in 0..10000 {
            terminal.write(format!("line-{line} repeated text 世界\r\n").as_bytes());
        }
        terminal.select_all()?;
        let before = terminal.selected_text()?;
        terminal.clear_selection()?;
        let mut finished = false;
        for _ in 0..10000 {
            if terminal.compress_scrollback_step()? != Compression::Pending {
                finished = true;
                break;
            }
        }
        assert!(finished);
        terminal.select_all()?;
        assert_eq!(terminal.selected_text()?, before);
        terminal.write(b"after-compression");
        terminal.scroll(Scroll::Bottom)?;
        assert!(terminal.frame()?.text().contains("after-compression"));
        Ok(())
    }
    #[test]
    fn memory_usage_reports_primary_and_lazy_alternate_storage() -> Result<()> {
        let mut terminal = Terminal::new(80, 24)?;
        terminal.write(b"primary");
        let primary = terminal.memory_usage()?;
        assert!(primary.primary_resident_bytes > 0);
        assert_eq!(primary.alternate_resident_bytes, 0);
        assert!(primary.virtual_bytes >= primary.primary_resident_bytes);
        assert_eq!(primary.image_bytes, 0);
        terminal.write(b"\x1b[?1049halternate");
        let alternate = terminal.memory_usage()?;
        assert!(alternate.alternate_resident_bytes > 0);
        assert!(
            alternate.virtual_bytes
                >= alternate.primary_resident_bytes + alternate.alternate_resident_bytes
        );
        assert!(terminal.frame()?.text().contains("alternate"));
        terminal.write(b"\x1b[?1049l");
        assert!(terminal.frame()?.text().contains("primary"));
        Ok(())
    }
    #[test]
    fn clear_scrollback_preserves_screen_parser_and_future_history() -> Result<()> {
        let mut terminal = Terminal::new(20, 3)?;
        for line in 0..10 {
            terminal.write(format!("line-{line}\r\n").as_bytes());
        }
        let before = terminal.frame()?;
        assert!(before.scrollback_lines > 0);
        let text = before.text();
        terminal.select_all()?;
        terminal.write(b"\x1b[3");
        terminal.clear_scrollback()?;
        let cleared = terminal.frame()?;
        assert_eq!(cleared.scrollback_lines, 0);
        assert_eq!(cleared.text(), text);
        assert!(terminal.selected_text()?.is_none());
        terminal.write(b"1mred\r\nnew\r\nmore\r\n");
        assert!(terminal.frame()?.scrollback_lines > 0);
        terminal.scroll(Scroll::Top)?;
        let history = terminal.frame()?.text();
        assert!(history.contains("red"));
        assert!(
            !history.contains("1mred"),
            "clear must preserve the VT parser state"
        );
        Ok(())
    }
    #[test]
    fn scrollback_limit_can_disable_and_resume_history() -> Result<()> {
        let mut terminal = Terminal::new(20, 3)?;
        for _ in 0..20 {
            terminal.write(b"history\r\n");
        }
        assert!(terminal.frame()?.scrollback_lines > 0);
        let screen = terminal.frame()?.text();
        terminal.set_scrollback_limit(0)?;
        assert_eq!(terminal.frame()?.text(), screen);
        assert_eq!(terminal.frame()?.scrollback_lines, 0);
        for _ in 0..20 {
            terminal.write(b"new output\r\n");
        }
        assert_eq!(terminal.frame()?.scrollback_lines, 0);
        terminal.set_scrollback_limit(1024 * 1024)?;
        for _ in 0..20 {
            terminal.write(b"retained\r\n");
        }
        assert!(terminal.frame()?.scrollback_lines > 0);
        Ok(())
    }
    #[test]
    fn scrollback_survives_output_and_alternate_screen() -> Result<()> {
        let mut terminal = Terminal::new(20, 3)?;
        for row in 0..10 {
            terminal.write(format!("line-{row}\r\n").as_bytes());
        }
        let live = terminal.frame()?;
        assert!(live.text().contains("line-9"));
        assert_eq!(live.scrollback_offset, 0);
        terminal.scroll(Scroll::Top)?;
        let history = terminal.frame()?;
        assert!(history.text().starts_with("line-0"));
        assert!(history.scrollback_offset > 0);
        terminal.write(b"line-10\r\n");
        assert!(terminal.frame()?.text().starts_with("line-0"));
        terminal.scroll(Scroll::Lines(2))?;
        assert!(terminal.frame()?.text().starts_with("line-2"));
        terminal.write(b"\x1b[?1049hfull-screen");
        terminal.scroll(Scroll::Top)?;
        let alternate = terminal.frame()?;
        assert!(alternate.text().contains("full-screen"));
        assert_eq!(alternate.scrollback_lines, 0);
        terminal.write(b"\x1b[?1049l");
        terminal.scroll(Scroll::Bottom)?;
        let restored = terminal.frame()?;
        assert!(restored.text().contains("line-10"));
        assert_eq!(restored.scrollback_offset, 0);
        Ok(())
    }
    #[test]
    fn directory_revisions_are_lazy_and_reset() -> Result<()> {
        let mut terminal = Terminal::new(40, 4)?;
        assert!(terminal.directory_since(0)?.is_none());
        terminal.write(if cfg!(windows) {
            b"\x1b]7;file:///C:/rmux\x07"
        } else {
            b"\x1b]7;file:///tmp/rmux\x07"
        });
        let (revision, path) = terminal.directory_since(0)?.unwrap();
        assert!(path.is_some());
        assert!(terminal.directory_since(revision)?.is_none());
        terminal.write(if cfg!(windows) { b"\x1b]7;file:///C:/rmux\x07" } else { b"\x1b]7;file:///tmp/rmux\x07" });
        let (revision, repeated_path) = terminal.directory_since(revision)?.expect("repeated prompt directory report must remain observable");
        assert_eq!(path, repeated_path);
        terminal.write(b"ordinary output");
        assert!(terminal.directory_since(revision)?.is_none());
        terminal.write(b"\x1bc");
        assert_eq!(terminal.directory_since(revision)?.unwrap().1, None);
        Ok(())
    }
    #[test]
    fn program_status_after_exit_keeps_results_without_resurrecting_work() -> Result<()> {
        let mut terminal = Terminal::new(20, 4)?;
        terminal.write(b"\x1b]7501;state=working:id=build\x07");
        terminal.finish_program_status();
        terminal.write(b"\x1b]7501;state=working:id=late\x07\x1b]7501;state=blocked:id=wait\x07\x1b]7501;state=done:id=build\x07");
        let records: Vec<_> = terminal.program_status().iter().collect();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].id, "build");
        assert_eq!(records[0].state, program_status::State::Done);
        terminal.finish_program_status();
        assert_eq!(terminal.program_status().iter().count(), 1);
        terminal.acknowledge_program_status();
        assert_eq!(terminal.program_status().iter().count(), 0);
        Ok(())
    }
    #[test]
    fn program_status_protocol_decodes_clears_and_tracks_prompt_lifecycle() -> Result<()> {
        let mut terminal = Terminal::new(20, 4)?;
        terminal.write(b"\x1b]7501;state=blocked:kind=permission:app=terraform:msg=QXBwbHk/\x07");
        let report = terminal.program_status().iter().next().unwrap();
        assert_eq!(report.state, program_status::State::Blocked);
        assert_eq!(report.kind, program_status::Kind::Permission);
        assert_eq!(report.message, "Apply?");
        terminal.write(b"\x1b]7501;state=done:id=build/test\x1b\\");
        assert_eq!(
            terminal.program_status().effective_app("build/test"),
            Some("terraform")
        );
        terminal.write(b"\x1b]133;A\x07");
        assert_eq!(terminal.program_status().iter().count(), 1);
        terminal.acknowledge_program_status();
        assert_eq!(terminal.program_status().iter().count(), 0);
        terminal.write(
            b"\x1b]7501;state=working:id=build\x07\x1b]7501;state=working:id=build/test\x07",
        );
        terminal.write(b"\x1b]7501;state=clear:id=build\x07");
        assert_eq!(terminal.program_status().iter().count(), 0);
        terminal.write(b"\x1b]7501;state=working\x07\x1bc");
        assert_eq!(terminal.program_status().iter().count(), 0);
        terminal.write(b"\x1b]7501;?\x07");
        assert!(!terminal.take_replies().is_empty());
        Ok(())
    }
    #[test]
    fn progress_after_exit_clears_activity_and_preserves_errors() -> Result<()> {
        for code in [1, 3, 4] {
            let mut terminal = Terminal::new(20, 4)?;
            terminal.write(format!("\x1b]9;4;{code};42\x07").as_bytes());
            assert!(terminal.progress().is_some());
            terminal.finish_program_status();
            assert_eq!(terminal.progress(), None);
            // Buffered output can arrive after the reaper reports exit.
            terminal.write(format!("\x1b]9;4;{code};80\x07").as_bytes());
            assert_eq!(terminal.progress(), None);
            terminal.write(b"\x1b]9;4;2;42\x07");
            let error = Some(Progress { state: ProgressState::Error, percent: Some(42) });
            assert_eq!(terminal.progress(), error);
            terminal.finish_program_status();
            assert_eq!(terminal.progress(), error);
            terminal.write(b"\x1b]9;4;1;90\x07");
            assert_eq!(terminal.progress(), error);
            terminal.write(b"\x1b]9;4;0\x07");
            assert_eq!(terminal.progress(), None);
        }
        Ok(())
    }
    #[test]
    fn progress_reports_update_remove_and_reset_without_notifications() -> Result<()> {
        let mut terminal = Terminal::new(20, 4)?;
        for (code, state) in [
            (1, ProgressState::Running),
            (2, ProgressState::Error),
            (4, ProgressState::Paused),
        ] {
            terminal.write(format!("\x1b]9;4;{code};42\x07").as_bytes());
            assert_eq!(
                terminal.progress(),
                Some(Progress {
                    state,
                    percent: Some(42)
                })
            );
        }
        terminal.write(b"\x1b]9;4;3\x1b\\");
        assert_eq!(
            terminal.progress().unwrap().state,
            ProgressState::Indeterminate
        );
        terminal.write(b"\x1b]9;4;0\x07");
        assert_eq!(terminal.progress(), None);
        terminal.write(b"\x1b]9;4;1;25\x07\x1bc");
        assert_eq!(terminal.progress(), None);
        assert!(terminal.notification_since(0).is_none());
        assert_eq!(terminal.bell_revision(), 0);
        Ok(())
    }
    #[test]
    #[ignore = "manual release-mode comparison; prints measurements, not a timing gate"]
    fn notification_stream_boundary_throughput() -> Result<()> {
        use std::hint::black_box;
        let workloads = [
            ("plain", "build src/main.rs compiling output\r\n".repeat(128)),
            ("ansi_unicode", "\x1b[32mbuild\x1b[0m compiling 界 e\u{301}\r\n".repeat(128)),
            ("shell_metadata", "\x1b]7;file://localhost/tmp\x1b\\\x1b]2;build\x07output\r\n".repeat(64)),
        ];
        for panes in [1, 8] {
            for (workload, chunk) in &workloads {
                for sample in 0..3 {
                    // Alternate order to reduce systematic warmup/thermal bias.
                    for boundaries in if sample % 2 == 0 { [false, true] } else { [true, false] } {
                        let mut terminals = (0..panes).map(|_| Terminal::new(160, 48)).collect::<Result<Vec<_>>>()?;
                        for terminal in &mut terminals { for _ in 0..32 { terminal.write(chunk.as_bytes()); } }
                        let started = Instant::now();
                        for _ in 0..1024 {
                            for terminal in &mut terminals {
                                if boundaries { terminal.write(black_box(chunk.as_bytes())); }
                                else {
                                    terminal.revision = terminal.revision.wrapping_add(1);
                                    terminal.mouse.refresh = true;
                                    // SAFETY: comparison uses the previous write path on
                                    // its own exclusively owned terminal and live bytes.
                                    unsafe { ghostty_terminal_vt_write(terminal.terminal, chunk.as_ptr(), chunk.len()); }
                                }
                            }
                        }
                        let elapsed = started.elapsed().as_secs_f64();
                        let mib = (chunk.len() * 1024 * panes) as f64 / (1024. * 1024.);
                        let resident: u64 = terminals.iter_mut().map(|terminal| terminal.memory_usage().map(|m| m.primary_resident_bytes + m.alternate_resident_bytes)).collect::<Result<Vec<_>>>()?.into_iter().sum();
                        println!("{{\"workload\":\"{workload}\",\"panes\":{panes},\"sample\":{sample},\"boundaries\":{boundaries},\"mib_per_second\":{:.3},\"resident_bytes\":{resident}}}", mib / elapsed);
                        for terminal in &mut terminals { black_box(terminal.frame()?); }
                    }
                }
            }
        }
        Ok(())
    }

    #[test]
    fn osc_fast_path_matches_boundary_path_at_every_chunk_split() -> Result<()> {
        let streams: &[&[u8]] = &[
            b"plain\x1b]7;file://localhost/tmp\x1b\\\x1b]99;i=x:d=0;Build\x1b\\\x1b]99;i=x:p=body;Done\x07tail",
            b"\x1b]2;known99title\x07\x1b]99;;\xe4\xb8\x96\xe7\x95\x8c\x1b\\",
            b"\x1bPignored]99;;fake\x1b\\\x1b]99;;actual\x07",
            b"\x1b]99;i=x:d=0;keep\x07\x1b]99;;cancel\x18\x1b]99;i=x:p=body;body\x07",
            b"\x1b]\0\x07\x1b]\x18\x1b]\x9d99;;C1\x07",
        ];
        for stream in streams {
            for split in 0..=stream.len() {
                let mut optimized = Terminal::new(80, 4)?;
                let mut reference = Terminal::new(80, 4)?;
                for chunk in [&stream[..split], &stream[split..]] {
                    optimized.write(chunk);
                    reference.write_with_osc_boundaries(chunk);
                }
                let message = |terminal: &Terminal| terminal.notification_since(0).map(|n| (n.revision, n.title.clone(), n.body.clone(), n.visibility));
                assert_eq!(message(&optimized), message(&reference), "split {split} in {stream:?}");
                assert_eq!(optimized.frame()?.text(), reference.frame()?.text());
            }
        }
        Ok(())
    }

    #[test]
    fn notification_bursts_retain_order_and_bound_consumer_backlog() -> Result<()> {
        let mut terminal = Terminal::new(40, 4)?;
        terminal.write(b"\x1b]9;first\x07\x1b]99;;second\x07\x1b]777;notify;third;body\x07");
        let messages = terminal.notifications_since(0);
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0].body, "first");
        assert_eq!(messages[1].title, "second");
        assert_eq!(messages[2].title, "third");
        assert!(terminal.notifications_since(messages[2].revision).is_empty());
        assert_eq!(terminal.notifications_since(messages[0].revision).len(), 2);
        terminal.write(b"\x1b]9;overflow\x07".repeat(40).as_slice());
        let retained = terminal.notifications_since(messages[2].revision);
        assert_eq!(retained.len(), 32);
        assert_eq!(retained.last().unwrap().revision, 43);
        Ok(())
    }

    #[test]
    fn notification_capability_queries_are_ordered_bounded_and_not_notifications() -> Result<()> {
        let mut terminal = Terminal::new(40, 4)?;
        for byte in b"\x1b]99;i=probe-1:p=?;\x1b\\\x1b[c" { terminal.write(&[*byte]); }
        let replies = String::from_utf8(terminal.take_replies())?;
        let expected = "\x1b]99;i=probe-1:p=?;p=title,body,?:o=always,unfocused,invisible\x1b\\";
        assert!(replies.starts_with(expected), "{replies:?}");
        assert!(replies[expected.len()..].starts_with("\x1b[?"), "device attributes must follow query reply");
        assert!(terminal.notification_since(0).is_none());
        terminal.write(expected.as_bytes());
        assert!(terminal.take_replies().is_empty(), "response must not cause a loop");
        terminal.write(b"\x1b]99;i=bad[id:p=?;\x07");
        assert!(terminal.take_replies().is_empty());
        terminal.write(b"\x1b]99;p=?;\x07");
        assert!(terminal.take_replies().ends_with(b"\x07"));
        terminal.write(b"\x1b]99;p=?;\x07".repeat(2048).as_slice());
        assert!(terminal.take_replies().len() <= 64 * 1024);
        Ok(())
    }

    #[test]
    fn kitty_notifications_use_ghostty_framing_and_ignore_cancelled_sequences() -> Result<()> {
        let mut terminal = Terminal::new(40, 4)?;
        for byte in b"\x1b]99;i=job:d=0;Build\x1b\\\x1b]99;i=job:p=body;e=ignored" {
            terminal.write(&[*byte]);
        }
        // Cancel the incomplete body, then complete the original title.
        terminal.write(b"\x18\x1b]99;i=job:p=body;Passed\x07");
        let first = terminal.notification_since(0).expect("OSC 99 text");
        assert_eq!((&*first.title, &*first.body), ("Build", "Passed"));
        terminal.write(b"\x1b]99;p=icon;ignored\x07");
        assert!(terminal.notification_since(first.revision).is_none());
        terminal.write(b"\x1b]9;Legacy\x07");
        let second = terminal.notification_since(first.revision).expect("legacy notification still works");
        terminal.write(b"\x1b]99;o=invisible;Current\x1b\\");
        let third = terminal.notification_since(second.revision).expect("shared notification revision");
        assert_eq!(third.title, "Current");
        assert_eq!(third.visibility, NotificationVisibility::Invisible);
        assert_eq!(terminal.frame()?.text().trim(), "");
        Ok(())
    }

    #[test]
    fn osc_notifications_are_bounded_revisioned_and_distinct_from_bells() -> Result<()> {
        let mut terminal = Terminal::new(20, 4)?;
        terminal.write(b"\x1b]9;build complete\x07");
        let first = terminal.notification_since(0).expect("OSC 9 notification");
        assert_eq!(first.body, "build complete");
        assert!(first.title.is_empty());
        assert_eq!(terminal.bell_revision(), 0);
        assert!(terminal.notification_since(first.revision).is_none());
        terminal.write(b"\x1b]777;notify;Build;tests passed\x1b\\");
        let second = terminal
            .notification_since(first.revision)
            .expect("OSC 777 notification");
        assert_eq!((&*second.title, &*second.body), ("Build", "tests passed"));
        terminal.write(b"\x1b]9;4;1;50\x07");
        assert!(terminal.notification_since(second.revision).is_none());
        let mut large = b"\x1b]9;".to_vec();
        large.extend(std::iter::repeat_n(b'x', 1500));
        large.push(7);
        terminal.write(&large);
        let latest = terminal.notification_since(second.revision).unwrap();
        assert_eq!(latest.body.len(), 1024);
        assert_eq!(first.body, "build complete"); // Previous owned message stays valid.
        Ok(())
    }
    #[test]
    fn bell_revision_ignores_osc_terminators() -> Result<()> {
        let mut terminal = Terminal::new(40, 4)?;
        assert_eq!(terminal.bell_revision(), 0);
        terminal.write(b"\x1b]2;title\x07");
        assert_eq!(terminal.bell_revision(), 0);
        terminal.write(b"hello\x07\x07");
        assert_eq!(terminal.bell_revision(), 2);
        terminal.write(b"\x1bc");
        terminal.write(b"\x07");
        assert_eq!(terminal.bell_revision(), 3);
        Ok(())
    }
    #[test]
    fn title_changes_are_coalesced_bounded_and_reset() -> Result<()> {
        let mut terminal = Terminal::new(40, 4)?;
        assert!(terminal.title_since(0)?.is_none());
        terminal.write(b"\x1b]2;project");
        assert!(terminal.title_since(0)?.is_none());
        terminal.write(b" shell\x07");
        let (revision, title) = terminal.title_since(0)?.unwrap();
        assert_eq!(title, "project shell");
        terminal.write(b"ordinary output");
        assert!(terminal.title_since(revision)?.is_none());
        terminal.write(format!("\x1b]2;{}\x07", "界".repeat(400)).as_bytes());
        let (revision, title) = terminal.title_since(revision)?.unwrap();
        assert_eq!(title, "界".repeat(128));
        terminal.write(b"\x1bc");
        assert_eq!(terminal.title_since(revision)?.unwrap().1, "");
        Ok(())
    }
    #[test]
    fn parses_styles_unicode_cursor_and_terminal_reply() -> Result<()> {
        let mut terminal = Terminal::new(40, 4)?;
        terminal.write("\x1b[31mred\x1b[0m 界e\u{301}\r\nnext\x1b[6n".as_bytes());
        let frame = terminal.frame()?;
        assert!(
            frame.text().starts_with("red 界e\u{301}\nnext"),
            "{}",
            frame.text()
        );
        assert_ne!(frame.rows[0][0].foreground, frame.rows[1][0].foreground);
        assert_eq!(frame.rows[0][4].width, 2);
        assert_eq!(frame.cursor, Some((4, 1)));
        assert_eq!(terminal.take_replies(), b"\x1b[2;5R");
        let second = terminal.frame()?;
        assert!(Arc::ptr_eq(&frame.rows[0], &second.rows[0]));
        assert!(Arc::ptr_eq(&frame.rows, &second.rows));
        assert!(Arc::ptr_eq(&frame.selections, &second.selections));
        terminal.write(b" changed");
        let changed = terminal.frame()?;
        assert!(!Arc::ptr_eq(&second.rows, &changed.rows));
        // Ghostty can mark unchanged rows dirty; preserve their identities so
        // the renderer does not reshape identical cells.
        assert!(Arc::ptr_eq(&second.rows[0], &changed.rows[0]));
        assert!(!second.text().contains("changed"));
        assert!(changed.text().contains("changed"));
        terminal.resize(20, 6)?;
        assert_eq!(terminal.frame()?.rows.len(), 6);
        assert_eq!(second.rows.len(), 4);
        Ok(())
    }

    #[test]
    fn bounded_viewport_text_preserves_utf8_spacing_and_truncation() -> Result<()> {
        let mut terminal = Terminal::new(12, 3)?;
        terminal.write(" A界e\u{301}  \r\nnext  ".as_bytes());
        let frame = terminal.frame()?;
        let expected = " A界e\u{301}\nnext\n";
        assert_eq!(frame.text(), expected);
        for limit in 0..=expected.len() + 1 {
            let (text, truncated) = frame.text_with_limit(limit);
            assert!(text.len() <= limit);
            assert!(expected.starts_with(&text));
            let mut boundary = expected.len().min(limit);
            while !expected.is_char_boundary(boundary) { boundary -= 1; }
            assert_eq!(text, expected[..boundary]);
            assert_eq!(truncated, limit < expected.len());
        }
        Ok(())
    }
    #[test]
    fn snapshot_identical_rewrites_preserve_rows_and_late_edits_keep_prefix() -> Result<()> {
        let mut terminal = Terminal::new(20, 3)?;
        terminal.write("abc界e\u{301}tail".as_bytes());
        let original = terminal.frame()?;
        terminal.write("\rabc界e\u{301}tail".as_bytes());
        let identical = terminal.frame()?;
        assert!(Arc::ptr_eq(&original.rows, &identical.rows));
        terminal.write(b"!");
        let changed = terminal.frame()?;
        assert!(changed.text().starts_with("abc界e\u{301}tail!"));
        assert!(original.text().starts_with("abc界e\u{301}tail\n"));
        assert!(!Arc::ptr_eq(&original.rows[0], &changed.rows[0]));
        assert!(Arc::ptr_eq(&original.rows[1], &changed.rows[1]));
        terminal.resize(8, 2)?;
        let resized = terminal.frame()?;
        assert_eq!(resized.columns, 8);
        assert!(resized.rows.iter().all(|row| row.len() == 8));
        Ok(())
    }

    #[test]
    fn snapshot_handles_inline_and_overflow_graphemes_across_frames() -> Result<()> {
        let mut terminal = Terminal::new(20, 3)?;
        let long = format!("a{}", "\u{301}".repeat(40));
        terminal.write(format!("x{long}z").as_bytes());
        let frame = terminal.frame()?;
        assert_eq!(frame.rows[0][0].text.as_str(), "x");
        assert_eq!(frame.rows[0][1].text.as_str(), long);
        assert_eq!(frame.rows[0][2].text.as_str(), "z");
        assert!(frame.rows[0][1].text.is_heap_allocated());
        terminal.write(format!("\r\x1b[31mx{long}z").as_bytes());
        let styled = terminal.frame()?;
        assert!(!Arc::ptr_eq(&frame.rows[0], &styled.rows[0]));
        assert_ne!(frame.rows[0][1].foreground, styled.rows[0][1].foreground);
        assert_eq!(frame.rows[0][1].text.as_str().as_ptr(), styled.rows[0][1].text.as_str().as_ptr());
        let different = format!("b{}", "\u{302}".repeat(40));
        terminal.write(format!("\rx{different}z").as_bytes());
        let changed = terminal.frame()?;
        assert_eq!(changed.rows[0][1].text.as_str(), different);
        assert_ne!(styled.rows[0][1].text.as_str().as_ptr(), changed.rows[0][1].text.as_str().as_ptr());
        terminal.write(b"\rshort\x1b[K");
        assert!(terminal.frame()?.text().starts_with("short"));
        assert_eq!(frame.rows[0][1].text.as_str(), long);
        Ok(())
    }

    #[test]
    fn completed_snapshot_consumes_row_dirtiness() -> Result<()> {
        let mut terminal = Terminal::new(40, 4)?;
        terminal.write(b"first\r\nsecond");
        let previous = terminal.frame()?;
        terminal.write(b"!");
        // Inspect the next update before snapshot extraction: only the edited
        // row should remain dirty, not all rows from the initial full frame.
        unsafe {
            check(ghostty_render_state_update(terminal.render, terminal.terminal))?;
            check(ghostty_render_state_get(
                terminal.render,
                GHOSTTY_RENDER_STATE_DATA_ROW_ITERATOR,
                out(&mut terminal.iterator),
            ))?;
            let mut rows = Vec::new();
            let mut y = 0;
            while ghostty_render_state_row_iterator_next_dirty(terminal.iterator, &mut y) {
                rows.push(y);
            }
            assert_eq!(rows, vec![1]);
        }
        let next = terminal.frame()?;
        assert!(next.text().starts_with("first\nsecond!"));
        assert!(previous.text().starts_with("first\nsecond"));
        assert!(Arc::ptr_eq(&previous.rows[0], &next.rows[0]));
        Ok(())
    }
    #[test]
    fn default_colors_reload_without_replacing_application_overrides() -> Result<()> {
        let mut terminal = Terminal::new(20, 3)?;
        terminal.write(b"text");
        let original = terminal.frame()?;
        terminal.set_default_colors([Some(Color(1, 2, 3)), Some(Color(4, 5, 6)), Some(Color(7, 8, 9))])?;
        let changed = terminal.frame()?;
        assert_eq!(changed.background, Color(4, 5, 6));
        assert_eq!(changed.rows[0][0].foreground, Color(1, 2, 3));
        assert_eq!(changed.cursor_color, Color(7, 8, 9));
        assert_eq!(original.background, Color(18, 20, 26));
        terminal.write(b"\x1b]11;#112233\x07");
        terminal.set_default_colors([None, Some(Color(10, 11, 12)), None])?;
        assert_eq!(terminal.frame()?.background, Color(17, 34, 51));
        terminal.write(b"\x1b]111\x07");
        assert_eq!(terminal.frame()?.background, Color(10, 11, 12));
        terminal.set_default_colors([None; 3])?;
        let reset = terminal.frame()?;
        assert_eq!(reset.background, original.background);
        assert_eq!(reset.rows[0][0].foreground, original.rows[0][0].foreground);
        assert_eq!(reset.cursor_color, original.cursor_color);
        Ok(())
    }
    #[test]
    fn theme_color_parser_handles_names_rgb_and_rejects_controls() -> Result<()> {
        for (input, expected) in [("#abc", Color(170,187,204)), ("aabbcc", Color(170,187,204)), (" RED ", Color(255,0,0)), ("LightGoldenrodYellow", Color(250,250,210)), ("rgb:ffff/0000/ffff", Color(255,0,255)), ("rgbi:1/0/1", Color(255,0,255))] {
            assert_eq!(parse_color(input)?, expected, "{input}");
        }
        for input in ["", "##123456", "not-a-color", "red\0", "red\n", "cell-background"] {
            assert!(parse_color(input).is_err(), "{input:?}");
        }
        assert!(parse_color(&"x".repeat(257)).is_err());
        Ok(())
    }

    #[test]
    fn palette_entry_parser_accepts_radices_and_bounds() -> Result<()> {
        for index in ["15", "0xf", "0o17", "0b1111"] {
            assert_eq!(parse_palette_entry(&format!(" {index} = red "))?, (15, Color(255,0,0)));
        }
        assert_eq!(parse_palette_entry("255=#010203")?, (255, Color(1,2,3)));
        for value in ["256=red", "-1=red", "0x100=red", "0b2=red", "1=invalid", "1=red\0"] {
            assert!(parse_palette_entry(value).is_err(), "{value:?}");
        }
        assert!(parse_palette_entry(&"x".repeat(273)).is_err());
        Ok(())
    }

    #[test]
    fn cursor_color_source_tracks_application_override_and_reset() -> Result<()> {
        let mut terminal = Terminal::new(4, 2)?;
        assert!(!terminal.frame()?.cursor_color_explicit);
        terminal.write(b"\x1b]12;#112233\x07");
        let frame = terminal.frame()?;
        assert!(frame.cursor_color_explicit);
        assert_eq!(frame.cursor_color, Color(17,34,51));
        terminal.write(b"\x1b]112\x07");
        assert!(!terminal.frame()?.cursor_color_explicit);
        terminal.set_default_colors([None, None, Some(Color(4,5,6))])?;
        assert!(terminal.frame()?.cursor_color_explicit);
        terminal.set_default_colors([None; 3])?;
        assert!(!terminal.frame()?.cursor_color_explicit);
        Ok(())
    }

    #[test]
    fn extended_palette_reload_preserves_osc_and_truecolor() -> Result<()> {
        let mut terminal = Terminal::new(20, 3)?;
        terminal.write(b"\x1b[38;5;200mE\x1b[38;2;9;8;7mT");
        let mut colors = default_palette();
        colors[200] = Color(1,2,3);
        terminal.set_palette(Some(colors))?;
        assert_eq!(terminal.frame()?.rows[0][0].foreground, colors[200]);
        assert_eq!(terminal.frame()?.rows[0][1].foreground, Color(9,8,7));
        terminal.write(b"\x1b]4;200;#112233\x07");
        colors[200] = Color(4,5,6);
        terminal.set_palette(Some(colors))?;
        assert_eq!(terminal.frame()?.rows[0][0].foreground, Color(17,34,51));
        terminal.write(b"\x1b]104;200\x07");
        assert_eq!(terminal.frame()?.rows[0][0].foreground, colors[200]);
        terminal.set_palette(None)?;
        assert_eq!(terminal.frame()?.rows[0][0].foreground, default_palette()[200]);
        Ok(())
    }

    #[test]
    fn ansi_palette_updates_existing_cells_and_preserves_extended_colors() -> Result<()> {
        let mut terminal = Terminal::new(20, 3)?;
        terminal.write(b"\x1b[31mR\x1b[91mB\x1b[38;5;200mE\x1b[38;2;9;8;7mT");
        let original = terminal.frame()?;
        let defaults = default_ansi_palette();
        assert_eq!(original.rows[0][0].foreground, defaults[1]);
        assert_eq!(original.rows[0][1].foreground, defaults[9]);
        let mut colors = [Color(20, 30, 40); 16];
        colors[1] = Color(100, 110, 120);
        terminal.set_ansi_palette(Some(colors))?;
        let changed = terminal.frame()?;
        assert_eq!(changed.rows[0][0].foreground, colors[1]);
        assert_eq!(changed.rows[0][1].foreground, colors[9]);
        assert_eq!(changed.rows[0][2].foreground, original.rows[0][2].foreground);
        assert_eq!(changed.rows[0][3].foreground, Color(9, 8, 7));
        terminal.write(b"\x1b]4;1;#112233\x07");
        colors[1] = Color(200, 210, 220);
        terminal.set_ansi_palette(Some(colors))?;
        assert_eq!(terminal.frame()?.rows[0][0].foreground, Color(17, 34, 51));
        terminal.write(b"\x1b]104;1\x07");
        assert_eq!(terminal.frame()?.rows[0][0].foreground, colors[1]);
        terminal.set_ansi_palette(None)?;
        assert_eq!(terminal.frame()?.rows[0][0].foreground, original.rows[0][0].foreground);
        Ok(())
    }
    #[test]
    fn cell_text_keeps_short_graphemes_inline_and_long_graphemes_intact() -> Result<()> {
        let mut terminal = Terminal::new(40, 3)?;
        terminal.write("A界e\u{301}".as_bytes());
        let first = terminal.frame()?;
        assert!(first.rows[0].iter().all(|cell| !cell.text.is_heap_allocated()));
        assert!(first.text().starts_with("A界e\u{301}"));
        let long = format!("e{}", "\u{301}".repeat(20));
        terminal.write(format!("\r\n{long}").as_bytes());
        let next = terminal.frame()?;
        assert_eq!(next.rows[1][0].text.as_str(), long);
        assert!(next.rows[1][0].text.is_heap_allocated());
        let storage = next.storage();
        assert_eq!(storage.heap_cells, 1);
        assert_eq!(storage.inline_cells + storage.heap_cells, 40 * 3);
        assert_eq!(storage.heap_text_payload_bytes, long.len());
        assert!(storage.cell_capacity_bytes >= 40 * 3 * size_of::<Cell>());
        assert!(!first.text().contains(&long));
        Ok(())
    }
    #[test]
    fn identical_rewrites_reuse_rows_but_style_changes_replace_them() -> Result<()> {
        let mut terminal = Terminal::new(20, 3)?;
        terminal.write(b"hello");
        let first = terminal.frame()?;
        terminal.write(b"\rhello");
        let identical = terminal.frame()?;
        assert!(Arc::ptr_eq(&first.rows, &identical.rows));
        terminal.select_all()?;
        let selected = terminal.frame()?;
        assert!(Arc::ptr_eq(&identical.rows, &selected.rows));
        assert!(!Arc::ptr_eq(&identical.selections, &selected.selections));
        terminal.write(b"\r\x1b[4;31mhello\x1b[0m");
        let styled = terminal.frame()?;
        assert!(!Arc::ptr_eq(&selected.rows[0], &styled.rows[0]));
        assert!(styled.rows[0][0].underline);
        assert!(!first.rows[0][0].underline);
        Ok(())
    }
}

pub mod unicode;
