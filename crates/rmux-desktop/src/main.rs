use gpui::{prelude::*, *};
use rmux_terminal::{Color, CursorShape, Frame, Scroll, session::Session};
use std::ops::Range;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
mod app_menu;
mod updater;
mod attention;
mod cell_metrics;
mod icons;
#[cfg(unix)]
mod file_drop;
mod file_browser;
mod markdown_preview;
mod external_editor;
mod terminal_search;
#[cfg(unix)]
mod image_paste;
mod git_context;
mod git_watch;
mod clipboard;
#[cfg(target_os = "macos")]
mod file_clipboard;
mod notifications;
mod native_notifications;
mod automation;
#[cfg(unix)]
mod socket_transport;
mod close_policy;
use cell_metrics::CellMetrics;
use close_policy::{ClosePolicy, PromptGate};
mod command_palette;
mod palette_view;
mod composition;
mod control_hint;
mod diagnostics;
mod memory_view;
mod settings_view;
mod shortcuts;
use rmux_core::focus_trail;
mod process_metrics;
mod ports;
mod process_tree;
mod ports_view;
mod copy_mode;
mod copy_buffers;
mod drag_item;
mod edit_history;
mod find_bar;
use rmux_core::layout;
use rmux_core::palette_match;
mod pane_view;
mod preferences;
mod theme_file;
use preferences::Preferences;
mod restore;
mod recovery;
mod row_cache;
mod list_navigation;
mod menu_navigation;
mod snapshot;
mod startup;
mod ssh;
mod status_view;
#[cfg(all(target_os = "macos", feature = "browser"))]
mod browser_window;
mod text_field;
mod text_layout;
mod terminal_menu;
use pane_view::{PaneEmpty, PaneView};
mod split_view;
mod split_presentation;
mod window_view;
mod window_geometry;
mod workspace_groups;
use layout::{Axis as SplitAxis, PaneTree};

#[derive(Clone, Debug, Default, PartialEq, Action)]
#[action(namespace = rmux, no_json)]
struct SetTerminalFont { family: String }

#[derive(Clone, Debug, Default, PartialEq, Action)]
#[action(namespace = rmux, no_json)]
struct ConnectSsh { destination: String }

#[derive(Clone, Debug, Default, PartialEq, Action)]
#[action(namespace = rmux, no_json)]
struct SetTerminalMemorySort { sort: memory_view::Sort }

#[derive(Clone, Debug, Default, PartialEq, Action)]
#[action(namespace = rmux, no_json)]
struct SetDefaultFont { family: String }

#[derive(Clone, Debug, Default, PartialEq, Action)]
#[action(namespace = rmux, no_json)]
struct PasteNamedBuffer { name: String, revision: u64, surface_id: String }

#[derive(Clone, Debug, Default, PartialEq, Action)]
#[action(namespace = rmux, no_json)]
struct DeleteNamedBuffer { name: String, revision: u64 }

#[derive(Clone, Debug, Default, PartialEq, Action)]
#[action(namespace = rmux, no_json)]
struct RenameNamedBuffer { name: String, revision: u64 }

#[derive(Clone, Debug, Default, PartialEq, Action)]
#[action(namespace = rmux, no_json)]
struct CopyNamedBuffer { name: String, revision: u64 }

#[derive(Clone, Debug, Default, PartialEq, Action)]
#[action(namespace = rmux, no_json)]
struct BrowseDirectory { path: std::path::PathBuf }

#[derive(Clone, Debug, Default, PartialEq, Action)]
#[action(namespace = rmux, no_json)]
struct EnterDirectory { path: std::path::PathBuf }

#[derive(Clone, Debug, Default, PartialEq, Action)]
#[action(namespace = rmux, no_json)]
struct NewWorkspaceInDirectory { path: std::path::PathBuf }

#[derive(Clone, Debug, Default, PartialEq, Action)]
#[action(namespace = rmux, no_json)]
struct CopyLocalPath { path: std::path::PathBuf }

#[derive(Clone, Debug, Default, PartialEq, Action)]
#[action(namespace = rmux, no_json)]
struct RevealLocalPath { path: std::path::PathBuf }

#[derive(Clone, Debug, Default, PartialEq, Action)]
#[action(namespace = rmux, no_json)]
struct PreviewLocalFile { path: std::path::PathBuf }

#[derive(Clone, Debug, Default, PartialEq, Action)]
#[action(namespace = rmux, no_json)]
struct EditLocalFile { path: std::path::PathBuf }

#[derive(Clone, Debug, Default, PartialEq, Action)]
#[action(namespace = rmux, no_json)]
struct PreviewHexFile { path: std::path::PathBuf }

#[derive(Clone, Debug, Default, PartialEq, Action)]
#[action(namespace = rmux, no_json)]
struct PreviewMarkdownFile { path: std::path::PathBuf }


#[derive(Clone, Debug, Default, PartialEq, Action)]
#[action(namespace = rmux, no_json)]
struct CopyPreviewLine { text: String, markdown: Option<markdown_preview::Kind>, spans: Vec<markdown_preview::Span> }

#[derive(Clone, Debug, Default, PartialEq, Action)]
#[action(namespace = rmux, no_json)]
struct CopyPreviewText { text: String }

#[derive(Clone, Debug, Default, PartialEq, Action)]
#[action(namespace = rmux, no_json)]
struct SearchTerminalOutput { query: String }

#[derive(Clone, Debug, Default, PartialEq, Action)]
#[action(namespace = rmux, no_json)]
struct CheckForUpdates;

#[derive(Clone, Debug, Default, PartialEq, Action)]
#[action(namespace = rmux, no_json)]
struct AdjustSetting { key: String, delta: i32 }
#[derive(Clone, Debug, Default, PartialEq, Action)]
#[action(namespace = rmux, no_json)]
struct SelectSetting { key: String }

#[derive(Clone, Debug, Default, PartialEq, Action)]
#[action(namespace = rmux, no_json)]
struct SetTerminalTheme { theme: preferences::TerminalTheme }

#[derive(Clone, Debug, Default, PartialEq, Action)]
#[action(namespace = rmux, no_json)]
struct RunCustomCommand { name: String }

#[derive(Clone, Debug, Default, PartialEq, Action)]
#[action(namespace = rmux, no_json)]
struct OpenLayoutPreset { name: String }


#[derive(Clone, Debug, Default, PartialEq, Action)]
#[action(namespace = rmux, no_json)]
struct SendShortcutText { text: String }


#[derive(Default)]
struct Shutdowns {
    handles: Vec<rmux_terminal::session::ShutdownHandle>,
    quitting: bool,
    quit_prompt: bool,
}

fn register_shutdown(handle: rmux_terminal::session::ShutdownHandle, cx: &mut App) {
    cx.global_mut::<Shutdowns>().handles.push(handle.clone());
    // Completion closes the reaper channel. Waiting retains no terminal/PTY and
    // schedules no timer; prune even when no further tabs are ever created.
    cx.spawn(async move |cx| {
        handle.wait().await;
        let _ = cx.update(|cx| {
            let handles = &mut cx.global_mut::<Shutdowns>().handles;
            handles.retain(|handle| !handle.is_finished());
            if handles.is_empty() {
                *handles = Vec::new();
            }
        });
    }).detach();
}

#[derive(Clone, Debug, Default, PartialEq, Action)]
#[action(namespace = rmux, no_json)]
struct MoveWorkspaceToGroup {
    group: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Action)]
#[action(namespace = rmux, no_json)]
struct SelectWorkspaceNumber {
    number: usize,
}
#[derive(Clone, Debug, Default, PartialEq, Action)]
#[action(namespace = rmux, no_json)]
struct SelectPaneNumber {
    number: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Action)]
#[action(namespace = rmux, no_json)]
struct SelectTabNumber {
    number: usize,
}
#[derive(Clone, Debug, Default, PartialEq, Action)]
#[action(namespace = rmux, no_json)]
struct SetWorkspaceColor {
    color: Option<u32>,
}
#[derive(Clone, Debug, Default, PartialEq, Action)]
#[action(namespace = rmux, no_json)]
struct SetWorkspaceIcon {
    icon: Option<String>,
}
const WORKSPACE_ICONS: &[(&str, Option<&str>)] = &[
    ("None", None),
    ("Terminal", Some("⌘")),
    ("Code", Some("⌨")),
    ("Build", Some("⚙")),
    ("Cloud", Some("☁")),
    ("Notes", Some("✎")),
];
const WORKSPACE_COLORS: &[(&str, Option<u32>)] = &[
    ("None", None),
    ("Blue", Some(0x7aa2f7)),
    ("Green", Some(0x9ece6a)),
    ("Amber", Some(0xe0af68)),
    ("Red", Some(0xf7768e)),
    ("Purple", Some(0xbb9af7)),
    ("Cyan", Some(0x7dcfff)),
];
impl Global for Shutdowns {}
fn checkpoint_before_quit(cx: &mut App) -> Option<Task<anyhow::Result<()>>> {
    if cx.global::<Shutdowns>().quitting { return None; }
    recovery::checkpoint(cx)
}
fn capture_session(cx: &mut App) -> anyhow::Result<snapshot::SessionSnapshot> {
    let target = cx.active_window().and_then(|handle| handle.downcast::<window_view::WorkspaceWindow>())
        .or_else(|| cx.windows().into_iter().find_map(|handle| handle.downcast::<window_view::WorkspaceWindow>()));
    match target {
        Some(handle) => handle.update(cx, |view, window, cx| view.session_snapshot(window, cx)).and_then(|result| result),
        None => Ok(snapshot::SessionSnapshot { version: 1, active_window: None, windows: Vec::new() }),
    }
}
struct InitialDirectory(Option<std::path::PathBuf>);
impl Global for InitialDirectory {}
struct InitialCommand(Option<(Session, Option<String>)>);
impl Global for InitialCommand {}

fn request_quit(cx: &mut App) {
    if cx.global::<Shutdowns>().quitting || cx.global::<Shutdowns>().quit_prompt {
        return;
    }
    let windows: Vec<_> = cx
        .windows()
        .into_iter()
        .filter_map(|window| window.downcast::<window_view::WorkspaceWindow>())
        .collect();
    let active_work = windows.iter().any(|window| {
        window
            .read(cx)
            .is_ok_and(|root| root.should_confirm_close(cx))
    });
    if !active_work {
        quit_after_cleanup(cx);
        return;
    }
    let target = cx
        .active_window()
        .and_then(|window| window.downcast::<window_view::WorkspaceWindow>())
        .or_else(|| windows.first().copied());
    let Some(target) = target else { return };
    let Some(permit) = cx.global::<PromptGate>().acquire() else {
        return;
    };
    let answer = target.update(cx, |_, window, cx| window.prompt(PromptLevel::Warning,
        "Quit rmux?",
        Some("Quitting terminates the owned terminal processes in every rmux window and discards their output history."),
        &["Cancel", "Quit rmux"], cx));
    let Ok(answer) = answer else { return };
    cx.global_mut::<Shutdowns>().quit_prompt = true;
    cx.spawn(async move |cx| {
        let confirmed = matches!(answer.await, Ok(1));
        drop(permit);
        let _ = cx.update(|cx| {
            cx.global_mut::<Shutdowns>().quit_prompt = false;
            if confirmed {
                quit_after_cleanup(cx);
            }
        });
    })
    .detach();
}

fn begin_shutdown(cx: &mut App) -> Option<Vec<rmux_terminal::session::ShutdownHandle>> {
    let registry = cx.global_mut::<Shutdowns>();
    if registry.quitting {
        return None;
    }
    registry.quitting = true;
    let handles = std::mem::take(&mut registry.handles);
    for handle in &handles {
        if let Err(error) = handle.request() {
            eprintln!("shell shutdown: {error}");
        }
    }
    Some(handles)
}

fn quit_after_cleanup(cx: &mut App) {
    let checkpoint = checkpoint_before_quit(cx);
    let Some(handles) = begin_shutdown(cx) else { return; };
    let timeout = cx
        .background_executor()
        .timer(std::time::Duration::from_secs(3));
    cx.spawn(async move |cx| {
        let finished = futures_lite::future::race(
            async move {
                if let Some(checkpoint) = checkpoint {
                    if let Err(error) = checkpoint.await { eprintln!("rmux: save session: {error:#}"); }
                }
                for handle in handles {
                    handle.wait().await;
                }
                true
            },
            async move {
                timeout.await;
                false
            },
        )
        .await;
        if !finished {
            eprintln!("session checkpoint or shell shutdown exceeded three-second deadline");
        }
        let _ = cx.update(|cx| cx.quit());
    })
    .detach();
}

const FONT_FAMILY: &str = if cfg!(target_os = "macos") {
    "Menlo"
} else if cfg!(target_os = "windows") {
    "Consolas"
} else {
    "DejaVu Sans Mono"
};

actions!(
    rmux,
    [
        Quit,
        IncreaseFontSize,
        DecreaseFontSize,
        ResetFontSize,
        UseConfiguredFont,
        ShowFontPicker,
        ShowDefaultFontPicker,
        ShowCommandPalette,
        NewWindow,
        CloseWindow,
        CloseWorkspace,
        Split,
        SplitBelow,
        SplitLeft,
        SplitAbove,
        TogglePaneZoom,
        EqualizePanes,
        ResizePaneLeft,
        ResizePaneRight,
        ResizePaneUp,
        ResizePaneDown,
        SwapPaneNext,
        SwapPaneLeft,
        SwapPaneRight,
        SwapPaneUp,
        SwapPaneDown,
        BreakTabRight,
        BreakTabBelow,
        BreakTabLeft,
        BreakTabAbove,
        JoinPaneNext,
        JoinPaneLeft,
        JoinPaneRight,
        JoinPaneUp,
        JoinPaneDown,
        MoveTabNextWorkspace,
        DetachTab,
        DetachWorkspace,
        MoveWorkspaceToNextWindow,
        ShowWorkspaceWindowPicker,
        ShowWorkspaceMergePicker,
        NewWorkspace,
        CancelNewWorkspace,
        RefreshTerminalMemory,
        CloseTerminalMemory,
        CopyTerminalMemory,
        CycleTerminalMemorySort,
        ShowTerminalMemorySort,
        NewWorkspaceGroup,
        UseCurrentDirectoryForGroup,
        ResetGroupDirectory,
        GroupCurrentWorkspace,
        GroupSelectedWorkspaces,
        RenameWorkspaceGroup,
        EditGroupColor,
        EditGroupIcon,
        UngroupWorkspaces,
        CloseWorkspaceGroup,
        ToggleGroupPin,
        RemoveWorkspaceFromGroup,
        SetGroupAnchor,
        RenameWorkspace,
        EditWorkspaceDescription,
        EditWorkspaceIcon,
        EditWorkspaceColor,
        SaveLayout,
        SaveSession,
        OpenSession,
        ShowLayoutPresetPicker,
        ShowCustomCommandPicker,
        CancelRestore,
        OpenLayout,
        ReloadSettings,
        ToggleSettings,
        SaveAppearanceSettings,
        OpenSettingsFile,
        ImportTerminalTheme,
        NextWorkspace,
        LastWorkspace,
        LastTerminal,
        FocusBack,
        FocusForward,
        RenameTab,
        ToggleWorkspacePin,
        NextUnreadTerminal,
        MarkAllRead,
        ToggleNotifications,
        DismissAllNotifications,
        RefreshGitContext,
        ToggleProgramStatus,
        ShowListeningPorts,
        CloseListeningPorts,
        PreviousWorkspace,
        MoveWorkspaceUp,
        MoveWorkspaceDown,
        ToggleSidebar,
        NewTab,
        NewLocalTab,
        CancelNewTab,
        CancelSplit,
        NextTab,
        ShowTabPicker,
        ShowTabMenu,
        ShowWorkspaceMenu,
        PreviousTab,
        MoveTabLeft,
        MoveTabRight,
        Paste,
        CancelQueuedPaste,
        CopySelection,
        EnterCopyMode,
        ShowBufferPicker,
        ShowDeleteBufferPicker,
        ShowRenameBufferPicker,
        ShowCopyBufferPicker,
        BrowseTerminalFiles,
        OpenTextPreview,
        CancelFileLoading,
        ShowTerminalSearch,
        ShowSshConnection,
        ShowSshHosts,
        CopySearchExcerpt,
        TogglePreviewWrap,
        RefreshFilePreview,
        RefreshFileBrowser,
        FileBrowserParent,
        FileBrowserLocation,
        PreviewDirectory,
        SwitchPreviewMode,
        ToggleMarkdownPreview,
        EditSelectedFile,
        OpenFolderBrowser,
        NewBrowserWindow,
        FileBrowserBack,
        FileBrowserForward,
        ShowTerminalMenu,
        CopySelectionWithLineBreaks,
        CopyWorkingDirectory,
        RevealWorkingDirectory,
        ClearScrollback,
        PreviousPrompt,
        NextPrompt,
        LastPrompt,
        ScrollToTop,
        ScrollToBottom,
        Find,
        FindOlder,
        FindNewer,
        EditorSelectAll,
        EditorCut,
        EditorUndo,
        EditorRedo,
        ClosePane,
        CloseOtherTabs,
        CloseTabsToRight,
        CloseFinishedTabs,
        NextPane,
        FocusLeft,
        FocusRight,
        FocusUp,
        FocusDown,
        PreviousPane,
        DumpRenderMetrics
    ]
);

struct TerminalView {
    font_family: SharedString,
    default_font_family: SharedString,
    default_font_size: f32,
    measured_font_size: Option<f32>,
    cells: CellMetrics,
    find: Option<Entity<find_bar::FindBar>>,
    last_find_query: String,
    find_history: find_bar::QueryHistory,
    selecting: bool,
    selection_pointer: Option<(Point<Pixels>, bool)>,
    selection_scroll: Option<Task<()>>,
    clipboard_task: Option<Task<()>>,
    #[cfg(unix)]
    image_paste_task: Option<Task<()>>,
    #[cfg(unix)]
    pasted_images: Vec<image_paste::Prepared>,
    directory_task: Option<Task<()>>,
    directory_generation: Arc<AtomicU64>,
    rendered_paste_pending: bool,
    terminal_menu: Option<terminal_menu::Menu>,
    terminal_menu_blur: Option<Subscription>,
    title: String,
    custom_title: Option<String>,
    saved_ssh_destination: Option<String>,
    title_revision: u64,
    directory_revision: u64,
    directory_label: Option<String>,
    remote_directory_label: Option<String>,
    snapshot_directory: Option<std::path::PathBuf>,
    git_branch: Option<String>,
    git_loading: bool,
    git_watch_path: Option<std::path::PathBuf>,
    git_watch_task: Option<Task<()>>,
    git_refresh_pending: bool,
    bell_revision: u64,
    notifications: notifications::History,
    engine_notification_revision: u64,
    progress: Option<rmux_terminal::Progress>,
    program_status: Option<rmux_terminal::program_status::Summary>,
    program_status_revision: u64,
    status_details: Option<Vec<rmux_terminal::program_status::Record>>,
    status_load: Option<Task<()>>,
    status_ui: Option<status_view::State>,
    ports: ports_view::State,
    copy_mode: Option<Entity<copy_mode::CopyMode>>,
    hidden_line_input: bool,
    link_hover: Option<(u64, String)>,
    link_hover_cell: Option<(u64, u16, u32)>,
    attention: attention::Attention,
    session: Session,
    applied_colors: Option<[Option<Color>; 3]>,
    applied_palette: Option<[Color; 256]>,
    focus: FocusHandle,
    frame: Frame,
    composition: composition::Composition,
    last_bounds: Bounds<Pixels>,
    error: Option<String>,
    process_exit: Option<String>,
    _updates: Task<()>,
    render_hold_timer: Option<Task<()>>,
    compression_task: Option<Task<()>>,
    compression_activity: std::time::Instant,
    row_cache: row_cache::RowCache,
    render_snapshot_released: bool,
    cursor_cache: row_cache::CursorCache,
    scroll_remainder: f32,
    mouse_button: Option<MouseButton>,
    metrics: Option<diagnostics::Metrics>,
    pressed_keys: std::collections::HashMap<rmux_terminal::keyboard::KeyIdentity, String>,
    input_focused: bool,
    _focus_subscriptions: Vec<Subscription>,
}
struct PreparedTerminal {
    session: Session,
    frame: Frame,
    colors: [Option<Color>; 3],
    palette: Option<[Color; 256]>,
    applied_colors: Option<[Option<Color>; 3]>,
    applied_palette: Option<[Color; 256]>,
    color_error: Option<String>,
}
struct TabMetadataChanged;
impl EventEmitter<TabMetadataChanged> for TerminalView {}
impl TerminalView {
    fn refresh_input_mode(&mut self, cx: &mut Context<Self>) {
        let hidden = self.input_focused && self.session.local_input_mode()
            .is_some_and(|mode| mode.canonical && !mode.echo);
        if self.hidden_line_input != hidden { self.hidden_line_input = hidden; cx.notify(); }
    }

    fn refresh_git_context(&mut self, cx: &mut Context<Self>) {
        if self.git_loading { self.git_refresh_pending = true; return; }
        let directory = self.snapshot_directory.clone();
        self.git_loading = true;
        let watched = self.git_watch_path.clone();
        let executor = cx.background_executor().clone();
        cx.spawn(async move |entity, cx| {
            let path = directory.clone();
            let (branch, watch_path, subscription) = executor.spawn(async move {
                let git_dir = path.as_deref().and_then(git_context::git_directory);
                let subscription = if git_dir != watched { git_dir.as_deref().and_then(git_watch::subscribe) } else { None };
                // Subscribe before reading, so a checkout during discovery cannot be missed.
                let branch = git_dir.as_deref().and_then(git_context::branch_at);
                (branch, git_dir, subscription)
            }).await;
            let _ = entity.update(cx, |this, cx| {
                this.git_loading = false;
                if this.snapshot_directory == directory && this.git_watch_path != watch_path {
                    this.git_watch_task = None;
                    this.git_watch_path = subscription.as_ref().and(watch_path);
                    if let Some(subscription) = subscription {
                        this.git_watch_task = Some(cx.spawn(async move |entity, cx| {
                            while subscription.changed().await.is_ok() {
                                if entity.update(cx, |this, cx| this.refresh_git_context(cx)).is_err() { break; }
                            }
                        }));
                    }
                }
                if this.snapshot_directory == directory && this.git_branch != branch {
                    this.git_branch = branch;
                    cx.emit(TabMetadataChanged); cx.notify();
                }
                if std::mem::take(&mut this.git_refresh_pending) { this.refresh_git_context(cx); }
            });
        }).detach();
    }
    fn has_active_work(&self) -> bool {
        self.session.has_active_work()
    }
    fn store_notification(&mut self, title: String, body: String, cx: &mut Context<Self>) -> u64 {
        let revision = self.notifications.push(title, body, !self.input_focused);
        if !self.input_focused {
            self.attention.notify();
            native_notifications::enqueue(revision, cx);
        }
        revision
    }
    fn dismiss_notification(&mut self, revision: u64, cx: &mut Context<Self>) {
        if self.notifications.dismiss(revision) {
            if !self.notifications.unread() { self.attention.dismiss_notification(); }
            cx.emit(TabMetadataChanged);
            cx.notify();
        }
    }
    fn mark_notification_read(&mut self, revision: u64, cx: &mut Context<Self>) {
        if self.notifications.mark_one_read(revision) {
            if !self.notifications.unread() { self.attention.dismiss_notification(); }
            cx.emit(TabMetadataChanged);
            cx.notify();
        }
    }
    fn set_font_size(&mut self, value: f32, cx: &mut Context<Self>) {
        self.set_font(value, self.font_family.clone(), cx);
    }
    fn set_font(&mut self, value: f32, family: SharedString, cx: &mut Context<Self>) {
        let next = CellMetrics::new(value);
        if next.font_size != self.cells.font_size || family != self.font_family {
            self.cells = next;
            self.font_family = family;
            self.measured_font_size = None;
            self.scroll_remainder = 0.;
            self.selection_scroll = None;
            self.selection_pointer = None;
            self.selecting = false;
            // Persistent font metadata must reach pane/workspace observers.
            // Observing every terminal repaint would checkpoint output frames.
            cx.emit(TabMetadataChanged);
            cx.notify();
        }
    }
    fn window_focus(
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> (FocusHandle, Vec<Subscription>) {
        let focus = cx.focus_handle();
        let blur = cx.on_blur(&focus, window, |view, _, cx| {
            view.set_input_focus(false, cx);
        });
        let focus_in = cx.on_focus(&focus, window, |view, window, cx| {
            view.set_input_focus(window.is_window_active(), cx);
        });
        let activation = cx.observe_window_activation(window, |view, window, cx| {
            view.set_input_focus(
                window.is_window_active() && view.focus.is_focused(window),
                cx,
            );
            cx.emit(TabMetadataChanged);
        });
        let pane_focus_in = cx.on_focus_in(&focus, window, |_, _, cx| cx.emit(TabMetadataChanged));
        let pane_focus_out =
            cx.on_focus_out(&focus, window, |_, _, _, cx| cx.emit(TabMetadataChanged));
        let preferences = cx.observe_global::<Preferences>(|view, cx| {
            let settings = cx.global::<Preferences>();
            if settings.font_size != view.default_font_size || settings.font_family.as_str() != view.default_font_family.as_ref() {
                let (size, family) = preferences::updated_font(view.cells.font_size, &view.font_family,
                    view.default_font_size, &view.default_font_family, settings);
                view.default_font_size = settings.font_size;
                view.default_font_family = settings.font_family.clone().into();
                view.set_font(size, family.into(), cx);
            }
            let colors = cx.global::<Preferences>().terminal_colors();
            if view.applied_colors != Some(colors) {
                match view.session.set_default_colors(colors) {
                    Ok(()) => view.applied_colors = Some(colors),
                    Err(error) => view.error = Some(error.to_string()),
                }
            }
            let palette = cx.global::<Preferences>().effective_palette();
            if view.applied_palette != palette {
                match view.session.set_palette(palette) {
                    Ok(()) => view.applied_palette = palette,
                    Err(error) => view.error = Some(error.to_string()),
                }
            }
            cx.notify();
        });
        (
            focus,
            vec![blur, focus_in, activation, pane_focus_in, pane_focus_out, preferences],
        )
    }
    fn rebind_window(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.clear_link_hover(cx);
        self.set_input_focus(false, cx);
        self.terminal_menu = None;
        self.terminal_menu_blur = None;
        self.ports = Default::default();
        self.copy_mode = None;
        // Pointer coordinates and wheel accumulation belong to the old window,
        // even when its terminal was already unfocused.
        self.mouse_button = None;
        self.selecting = false;
        self.selection_scroll = None;
        self.selection_pointer = None;
        self.scroll_remainder = 0.;
        self._focus_subscriptions.clear();
        let (focus, subscriptions) = Self::window_focus(window, cx);
        self.focus = focus;
        self._focus_subscriptions = subscriptions;
        self.dismiss_find(cx);
        self.composition = composition::Composition::default();
        self.last_bounds = Bounds::default();
        self.row_cache = row_cache::RowCache::default();
        self.cursor_cache = row_cache::CursorCache::default();
        cx.notify();
    }
    fn prepare(session: Session, colors: [Option<Color>; 3], palette: Option<[Color; 256]>) -> Result<PreparedTerminal> {
        let color_error = session.set_default_colors(colors).err().map(|error| error.to_string());
        let applied_colors = color_error.is_none().then_some(colors);
        let palette_error = session.set_palette(palette).err();
        let applied_palette = if palette_error.is_none() { palette } else { None };
        let color_error = color_error.or_else(|| palette_error.map(|error| error.to_string()));
        let frame = session.frame()?;
        Ok(PreparedTerminal { session, frame, colors, palette, applied_colors, applied_palette, color_error })
    }
    fn new(session: Session, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let settings = cx.global::<Preferences>();
        let prepared = Self::prepare(session, settings.terminal_colors(), settings.effective_palette())
            .expect("initial terminal frame");
        Self::from_prepared(prepared, window, cx)
    }
    fn from_prepared(prepared: PreparedTerminal, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let PreparedTerminal { session, frame, applied_colors, applied_palette, color_error, .. } = prepared;
        let snapshot_directory = session.initial_directory().map(std::path::Path::to_path_buf);
        register_shutdown(session.shutdown_handle(), cx);
        let (focus, subscriptions) = Self::window_focus(window, cx);
        let updates = session.updates.clone();
        let task = cx.spawn(async move |entity, cx| {
            let mut retry = false;
            loop {
                if retry { cx.background_executor().timer(std::time::Duration::from_millis(8)).await; }
                else if updates.recv().await.is_err() { break; }
                match entity
                    .update(cx, |view, cx| {
                        if !retry && let Some(metrics) = &mut view.metrics { metrics.notifications += 1; }
                        if view.process_exit.is_none() {
                            if let Some(status) = view.session.exit_status() {
                                view.process_exit = Some(status);
                                cx.emit(TabMetadataChanged);
                                cx.notify();
                            }
                        }
                        if !retry { view.refresh_input_mode(cx); }
                        if view.rendered_paste_pending != view.session.paste_pending() { cx.notify(); }
                        if let Some(error) = view.session.take_input_error() {
                            view.error = Some(error);
                            cx.notify();
                        }
                        let Some(metadata) = view.session.try_metadata_since(
                            view.title_revision,
                            view.directory_revision,
                            view.engine_notification_revision,
                            view.program_status_revision,
                            view.status_details.is_some(),
                        ) else {
                            if let Some(metrics) = &mut view.metrics { metrics.metadata_busy += 1; }
                            return false;
                        };
                        let mut metadata_changed = false;
                        if let Some((revision, status)) = metadata.program_status {
                            use rmux_terminal::program_status::State;
                            if !view.input_focused
                                && status.as_ref().is_some_and(|status| {
                                    matches!(
                                        status.record.state,
                                        State::Blocked | State::Error | State::Done
                                    )
                                })
                            {
                                view.attention.status();
                            }
                            view.program_status = status;
                            view.program_status_revision = revision;
                            if let Some(records) = metadata.program_status_records { view.status_details = Some(records); view.status_load = None; }
                            metadata_changed = true;
                        }
                        if view.progress != metadata.progress {
                            view.progress = metadata.progress;
                            metadata_changed = true;
                        }
                        for notification in metadata.notifications {
                            view.engine_notification_revision = notification.revision;
                            let visible = notification.visibility == rmux_terminal::NotificationVisibility::Invisible
                                && window_view::terminal_is_visible(cx.entity_id(), cx);
                            if notification.visibility.allows(view.input_focused, visible) {
                                view.store_notification(notification.title.clone(), notification.body.clone(), cx);
                                metadata_changed = true;
                            }
                        }
                        if metadata.bell_revision != view.bell_revision {
                            view.bell_revision = metadata.bell_revision;
                            if !view.input_focused {
                                view.attention.bell();
                                metadata_changed = true;
                            }
                        }
                        if let Ok(Some((revision, title))) = metadata.title {
                            view.title_revision = revision;
                            if title != view.title {
                                view.title = title;
                                metadata_changed = true;
                            }
                        }
                        if let Ok(Some((revision, directory))) = metadata.directory {
                            view.directory_revision = revision;
                            // Revision zero means no OSC directory report yet.
                            // Preserve the explicit launch directory until then;
                            // later remote/cleared reports must clear that fallback.
                            if (revision != 0 || directory.is_some()) && view.snapshot_directory != directory {
                                view.snapshot_directory = directory.clone();
                                view.git_branch = None;
                                metadata_changed = true;
                            }
                            let label = directory.map(|path| {
                                path.to_string_lossy()
                                    .chars()
                                    .filter(|c| !c.is_control())
                                    .collect::<String>()
                            });
                            if view.directory_label != label {
                                view.directory_label = label;
                                metadata_changed = true;
                            }
                            view.refresh_git_context(cx);
                        }
                        if let Ok(Some((revision, directory))) = metadata.remote_directory {
                            view.directory_revision = revision;
                            if view.remote_directory_label != directory {
                                view.remote_directory_label = directory;
                                metadata_changed = true;
                            }
                        }
                        if metadata_changed {
                            cx.emit(TabMetadataChanged);
                        }
                        view.request_output_render_with_delay(metadata.render_delay, cx);
                        if let Some(find) = &view.find {
                            find.read(cx).worker.refresh();
                        }
                        true
                    }) {
                    Ok(handled) => retry = !handled,
                    Err(_) => break,
                }
            }
        });
        Self {
            cells: CellMetrics::new(cx.global::<Preferences>().font_size),
            font_family: cx.global::<Preferences>().font_family.clone().into(),
            default_font_family: cx.global::<Preferences>().font_family.clone().into(),
            default_font_size: cx.global::<Preferences>().font_size,
            measured_font_size: None,
            find: None,
            last_find_query: String::new(),
            find_history: Default::default(),
            selecting: false,
            selection_pointer: None,
            selection_scroll: None,
            clipboard_task: None,
            #[cfg(unix)]
            image_paste_task: None,
            #[cfg(unix)]
            pasted_images: Vec::new(),
            directory_task: None,
            directory_generation: Default::default(),
            rendered_paste_pending: false,
            terminal_menu: None,
            terminal_menu_blur: None,
            title: String::new(),
            custom_title: None,
            saved_ssh_destination: None,
            title_revision: u64::MAX,
            directory_revision: u64::MAX,
            directory_label: None,
            remote_directory_label: None,
            snapshot_directory,
            git_branch: None,
            git_loading: false,
            git_watch_path: None,
            git_watch_task: None,
            git_refresh_pending: false,
            bell_revision: 0,
            notifications: Default::default(),
            engine_notification_revision: 0,
            progress: None,
            program_status: None,
            program_status_revision: 0,
            status_details: None,
            status_load: None,
            status_ui: None,
            ports: Default::default(),
            copy_mode: None,
            hidden_line_input: false,
            link_hover: None,
            link_hover_cell: None,
            attention: attention::Attention::default(),
            input_focused: false,
            _focus_subscriptions: subscriptions,
            pressed_keys: Default::default(),
            metrics: diagnostics::Metrics::enabled(),
            mouse_button: None,
            scroll_remainder: 0.,
            row_cache: row_cache::RowCache::default(),
            render_snapshot_released: false,
            cursor_cache: row_cache::CursorCache::default(),
            session,
            applied_colors,
            applied_palette,
            focus,
            frame,
            composition: composition::Composition::default(),
            last_bounds: Bounds::default(),
            error: color_error,
            process_exit: None,
            _updates: task,
            render_hold_timer: None,
            compression_task: None,
            compression_activity: std::time::Instant::now(),
        }
    }
    fn schedule_compression(&mut self, cx: &mut Context<Self>) {
        self.compression_activity = std::time::Instant::now();
        if self.input_focused || self.find.is_some() || self.compression_task.is_some() {
            return;
        }
        let compressor = self.session.compressor();
        let cancellation = compressor.cancel_on_drop();
        let executor = cx.background_executor().clone();
        self.compression_task = Some(cx.spawn(async move |entity, cx| {
            let _cancellation = cancellation;
            use rmux_terminal::compression::Step;
            let idle = std::time::Duration::from_secs(2);
            let mut delay = idle;
            let mut token = None;
            loop {
                executor.timer(delay).await;
                let ready = entity.update(cx, |view, _| {
                    if view.input_focused || view.find.is_some() {
                        None
                    } else {
                        Some(idle.saturating_sub(view.compression_activity.elapsed()))
                    }
                });
                let Ok(Some(remaining)) = ready else {
                    break;
                };
                if !remaining.is_zero() {
                    delay = remaining;
                    continue;
                }
                let worker = compressor.clone();
                match executor.spawn(async move { worker.step(token) }).await {
                    Ok(Step::Activity(next)) => {
                        token = Some(next);
                        delay = idle;
                    }
                    Ok(Step::Busy) => delay = std::time::Duration::from_millis(50),
                    Ok(Step::Pending) => delay = std::time::Duration::from_millis(16),
                    Ok(Step::Gone | Step::Complete) | Err(_) => break,
                }
            }
            let _ = entity.update(cx, |view, cx| {
                view.compression_task = None;
                // Output can arrive between the last background step and this
                // callback. Preserve that wakeup without polling after completion.
                if view.compression_activity.elapsed() < idle {
                    view.schedule_compression(cx);
                }
            });
        }));
    }
    fn has_render_cache(&self) -> bool {
        !self.render_snapshot_released || !self.row_cache.is_empty()
    }
    fn release_render_cache(&mut self) {
        if let Some(metrics) = &mut self.metrics { metrics.snapshot_recovery.hide(); }
        if !self.render_snapshot_released {
            if let Some(metrics) = &mut self.metrics { metrics.snapshot_releases.record(&self.frame); }
            row_cache::release_frame_cells(&mut self.frame);
            self.render_snapshot_released = true;
        }
        self.cursor_cache = row_cache::CursorCache::default();
        let released = self.row_cache.release();
        if released > 0
            && let Some(metrics) = &mut self.metrics
        {
            metrics.cache_releases += 1;
            metrics.rows_released += released as u64;
        }
    }
    fn request_output_render(&mut self, cx: &mut Context<Self>) {
        let delay = self.session.try_render_delay().unwrap_or(Some(std::time::Duration::from_millis(8)));
        self.request_output_render_with_delay(delay, cx);
    }
    fn request_output_render_with_delay(&mut self, delay: Option<std::time::Duration>, cx: &mut Context<Self>) {
        self.schedule_compression(cx);
        if let Some(delay) = delay {
            if self.render_hold_timer.is_none() {
                let timer = cx.background_executor().timer(delay);
                self.render_hold_timer = Some(cx.spawn(async move |entity, cx| {
                    timer.await;
                    let _ = entity.update(cx, |view, cx| {
                        view.render_hold_timer = None;
                        view.request_output_render(cx);
                    });
                }));
            }
        } else {
            self.render_hold_timer = None;
            cx.notify();
        }
    }
    fn acknowledge_attention(&mut self, cx: &mut Context<Self>) {
        if self.attention.any() {
            self.attention.acknowledge();
            self.notifications.mark_read();
            cx.emit(TabMetadataChanged);
            cx.notify();
        }
    }
    fn set_input_focus(&mut self, focused: bool, cx: &mut Context<Self>) {
        if focused {
            self.compression_task = None;
        }
        if focused { self.acknowledge_attention(cx); }
        if self.input_focused == focused {
            return;
        }
        self.input_focused = focused;
        self.refresh_input_mode(cx);
        if !focused {
            self.schedule_compression(cx);
        }
        cx.emit(TabMetadataChanged);
        if !focused {
            // Finish the old pane's presses before reporting focus loss.
            for (_, key) in self.pressed_keys.drain() {
                if let Err(error) = self
                    .session
                    .key_release(&key, rmux_terminal::keyboard::Modifiers::default())
                {
                    self.error = Some(error.to_string());
                }
            }
            self.mouse_button = None;
            self.selecting = false;
            self.selection_scroll = None;
            self.selection_pointer = None;
        }
        if let Err(error) = self.session.focus(focused) {
            self.error = Some(error.to_string());
        }
        if focused
            && let Some((revision, status)) = self
                .session
                .program_status_since(self.program_status_revision)
        {
            self.program_status_revision = revision;
            self.program_status = status;
            if self.status_details.is_some() {
                self.status_details = Some(self.session.program_status_records());
            }
            cx.emit(TabMetadataChanged);
        }
        cx.notify();
    }
    fn toggle_status_details(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.status_details.is_some() {
            self.close_status_details(window, cx);
            return;
        }
        let focus = cx.focus_handle();
        focus.focus(window);
        self.status_ui = Some(status_view::State { focus, selected: None, scroll: ScrollHandle::new() });
        if let Some(records) = self.session.try_program_status_records() {
            self.status_details = Some(records);
        } else {
            self.status_details = Some(Vec::new());
            self.status_load = Some(cx.spawn(async move |entity, cx| {
                loop {
                    cx.background_executor().timer(std::time::Duration::from_millis(8)).await;
                    let done = entity.update(cx, |view, cx| {
                        if view.status_details.is_none() { return true; }
                        let Some(records) = view.session.try_program_status_records() else { return false; };
                        view.status_details = Some(records);
                        view.status_load = None;
                        cx.notify();
                        true
                    });
                    if !matches!(done, Ok(false)) { break; }
                }
            }));
        }
        cx.notify();
    }
    fn image_paste_pending(&self) -> bool {
        #[cfg(unix)]
        { self.image_paste_task.is_some() }
        #[cfg(not(unix))]
        { false }
    }
    fn cancel_image_preparation(&mut self, cx: &mut Context<Self>) {
        #[cfg(unix)]
        if self.image_paste_task.take().is_some() { cx.notify(); }
        #[cfg(not(unix))]
        let _ = cx;
    }
    fn cancel_paste(&mut self, cx: &mut Context<Self>) {
        self.cancel_image_preparation(cx);
        self.session.cancel_queued_paste();
        cx.notify();
    }
    fn input(&mut self, bytes: &[u8], cx: &mut Context<Self>) {
        self.cancel_image_preparation(cx);
        self.refresh_input_mode(cx);
        if let Err(error) = self.session.input(bytes) {
            self.error = Some(error.to_string());
            cx.notify();
        } else if self.frame.scrollback_offset > 0 {
            cx.notify();
        }
    }
    fn scroll(&mut self, scroll: Scroll, cx: &mut Context<Self>) {
        if let Err(error) = self.session.scroll(scroll) {
            self.error = Some(error.to_string());
        }
        cx.notify();
    }
    fn mouse_event(
        &mut self,
        action: rmux_terminal::mouse::Action,
        button: Option<rmux_terminal::mouse::Button>,
        position: Point<Pixels>,
        modifiers: Modifiers,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let scale = window.scale_factor();
        let position = position - self.last_bounds.origin;
        let event = rmux_terminal::mouse::Event {
            action,
            button,
            x: (f32::from(position.x) * scale).max(0.),
            y: (f32::from(position.y) * scale).max(0.),
            width: (f32::from(self.last_bounds.size.width) * scale)
                .round()
                .max(1.) as u32,
            height: (f32::from(self.last_bounds.size.height) * scale)
                .round()
                .max(1.) as u32,
            cell_width: self.cells.width * scale,
            cell_height: self.cells.height * scale,
            pressed: self.mouse_button.is_some(),
            modifiers: rmux_terminal::keyboard::Modifiers {
                shift: modifiers.shift,
                control: modifiers.control,
                alt: modifiers.alt,
            },
        };
        match self.session.mouse(event) {
            Ok(reported) => reported,
            Err(error) => {
                self.error = Some(error.to_string());
                cx.notify();
                true
            }
        }
    }
    fn mouse_down(&mut self, event: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.clear_link_hover(cx);
        self.focus.focus(window);
        if !self.last_bounds.contains(&event.position) {
            cx.stop_propagation();
            return;
        }
        let open_link = if cfg!(target_os = "macos") {
            event.modifiers.platform
        } else {
            event.modifiers.control
        };
        if event.button == MouseButton::Left
            && open_link
            && self.last_bounds.contains(&event.position)
        {
            let position = event.position - self.last_bounds.origin;
            let column = (f32::from(position.x) / self.cells.width) as u16;
            let row = (f32::from(position.y) / self.cells.height) as u32;
            match self.session.hyperlink_at(column, row, self.frame.revision) {
                Ok(Some(uri)) => {
                    if openable_uri(&uri) {
                        cx.open_url(&uri);
                    } else {
                        self.error = Some("Only HTTP(S) hyperlinks can be opened here.".into());
                        cx.notify();
                    }
                    self.mouse_button = None;
                    cx.stop_propagation();
                    return;
                }
                Ok(None) => {}
                Err(error) => {
                    self.error = Some(error.to_string());
                    cx.notify();
                    cx.stop_propagation();
                    return;
                }
            }
        }
        self.mouse_button = Some(event.button);
        if !event.modifiers.shift
            && self.mouse_event(
                rmux_terminal::mouse::Action::Press,
                terminal_button(event.button),
                event.position,
                event.modifiers,
                window,
                cx,
            )
        {
            cx.stop_propagation();
        } else {
            self.mouse_button = None;
            if event.button == MouseButton::Right {
                self.show_terminal_menu(event.position, window, cx);
                cx.stop_propagation();
            } else if event.button == MouseButton::Left {
                self.selecting = true;
                self.select(
                    rmux_terminal::selection::Action::Press,
                    event.position,
                    event.modifiers.alt,
                    event.click_count as usize,
                    cx,
                );
                cx.stop_propagation();
            }
        }
    }
    fn mouse_up(&mut self, event: &MouseUpEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.selecting && event.button == MouseButton::Left {
            self.selecting = false;
            self.selection_scroll = None;
            self.selection_pointer = None;
            self.select(
                rmux_terminal::selection::Action::Release,
                event.position,
                event.modifiers.alt,
                1,
                cx,
            );
            if cx.global::<Preferences>().copy_on_select {
                self.copy(&CopySelection, window, cx);
            }
            cx.stop_propagation();
            return;
        }
        if self.mouse_button != Some(event.button) {
            return;
        }
        self.mouse_button = None;
        self.mouse_event(
            rmux_terminal::mouse::Action::Release,
            terminal_button(event.button),
            event.position,
            event.modifiers,
            window,
            cx,
        );
        cx.stop_propagation();
    }
    fn clear_link_hover(&mut self, cx: &mut Context<Self>) {
        self.link_hover_cell = None;
        if self.link_hover.take().is_some() { cx.notify(); }
    }
    fn update_link_hover(&mut self, position: Point<Pixels>, modifiers: Modifiers, cx: &mut Context<Self>) {
        let modifier = if cfg!(target_os = "macos") { modifiers.platform } else { modifiers.control };
        if !modifier || self.selecting || self.mouse_button.is_some() || self.copy_mode.is_some()
            || self.find.is_some() || self.ports.visible || self.terminal_menu.is_some()
            || cx.global::<PromptGate>().is_active() || !self.last_bounds.contains(&position) {
            self.clear_link_hover(cx);
            return;
        }
        let point = position - self.last_bounds.origin;
        let cell = (self.frame.revision, (f32::from(point.x) / self.cells.width) as u16, (f32::from(point.y) / self.cells.height) as u32);
        if self.link_hover_cell == Some(cell) { return; }
        // A busy parser leaves the cell uncached so the next motion can retry.
        let uri = match self.session.hyperlink_at(cell.1, cell.2, cell.0) {
            Ok(uri) => { self.link_hover_cell = Some(cell); uri },
            Err(_) => { self.clear_link_hover(cx); return; },
        };
        let next = uri.map(|uri| (cell.0, uri));
        if next != self.link_hover { self.link_hover = next; cx.notify(); }
    }
    fn mouse_move(&mut self, event: &MouseMoveEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.update_link_hover(event.position, event.modifiers, cx);
        if self.selecting {
            if event.dragging() {
                self.select(
                    rmux_terminal::selection::Action::Drag,
                    event.position,
                    event.modifiers.alt,
                    1,
                    cx,
                );
                cx.stop_propagation();
            } else {
                self.selecting = false;
                self.selection_scroll = None;
                self.selection_pointer = None;
            }
            return;
        }
        if event.modifiers.shift {
            return;
        }
        if self.mouse_button.is_none() && !self.last_bounds.contains(&event.position) {
            return;
        }
        self.mouse_event(
            rmux_terminal::mouse::Action::Motion,
            self.mouse_button.and_then(terminal_button),
            event.position,
            event.modifiers,
            window,
            cx,
        );
    }
    fn wheel(&mut self, event: &ScrollWheelEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.scroll_remainder -=
            f32::from(event.delta.pixel_delta(px(self.cells.height)).y) / self.cells.height;
        let lines = (self.scroll_remainder.trunc() as isize).clamp(-32, 32);
        if lines != 0 {
            self.scroll_remainder -= lines as f32;
            let button = if lines < 0 {
                rmux_terminal::mouse::Button::WheelUp
            } else {
                rmux_terminal::mouse::Button::WheelDown
            };
            if !event.modifiers.shift
                && self.mouse_event(
                    rmux_terminal::mouse::Action::Press,
                    Some(button),
                    event.position,
                    event.modifiers,
                    window,
                    cx,
                )
            {
                for _ in 1..lines.unsigned_abs() {
                    self.mouse_event(
                        rmux_terminal::mouse::Action::Press,
                        Some(button),
                        event.position,
                        event.modifiers,
                        window,
                        cx,
                    );
                }
            } else {
                self.scroll(Scroll::Lines(lines), cx);
            }
        }
        cx.stop_propagation();
    }
    fn key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if !self.focus.is_focused(window) {
            return;
        }
        if !self.composition.text.is_empty() {
            return;
        }
        self.refresh_input_mode(cx);
        let key = &event.keystroke;
        if key.modifiers.platform {
            return;
        }
        // Let AppKit compose Option text/dead keys when requested. These presses
        // never enter pressed_keys, so their releases stay on the native path too.
        if cfg!(target_os = "macos")
            && !cx.global::<Preferences>().macos_option_as_alt
            && key.modifiers.alt
            && !key.modifiers.control
            && (key.key.chars().count() == 1 || key.key == "space")
        {
            return;
        }
        // Native dead keys can have a key identity but no committed character.
        // Keep them on the composition path even when a TUI requests all keys.
        if key.key.chars().count() == 1
            && key.key_char.is_none()
            && !key.modifiers.control
            && !key.modifiers.alt
        {
            return;
        }
        if key.modifiers.shift && !key.modifiers.control && !key.modifiers.alt {
            let page = self.frame.rows.len().saturating_sub(1).max(1) as isize;
            let scroll = match key.key.as_str() {
                "pageup" => Some(Scroll::Lines(-page)),
                "pagedown" => Some(Scroll::Lines(page)),
                "home" => Some(Scroll::Top),
                "end" => Some(Scroll::Bottom),
                _ => None,
            };
            if let Some(scroll) = scroll {
                self.scroll_remainder = 0.;
                self.scroll(scroll, cx);
                cx.stop_propagation();
                return;
            }
        }
        let mods = rmux_terminal::keyboard::Modifiers {
            shift: key.modifiers.shift,
            control: key.modifiers.control,
            alt: key.modifiers.alt,
        };
        self.cancel_image_preparation(cx);
        match self
            .session
            .key_with_text(&key.key, mods, event.is_held, key.key_char.as_deref())
        {
            Ok(true) => {
                if let Some(identity) = rmux_terminal::keyboard::key_identity(&key.key) {
                    self.pressed_keys
                        .entry(identity)
                        .or_insert_with(|| key.key.clone());
                }
                if self.frame.scrollback_offset > 0 {
                    cx.notify();
                }
                cx.stop_propagation();
            }
            Ok(false) => {}
            Err(error) => {
                self.error = Some(error.to_string());
                cx.notify();
                cx.stop_propagation();
            }
        }
    }
    fn key_up(&mut self, event: &KeyUpEvent, _: &mut Window, cx: &mut Context<Self>) {
        let key = &event.keystroke;
        // Local shortcuts and native text/IME presses must not produce orphan
        // release reports. Only pair releases with presses sent to this pane.
        let Some(pressed) = rmux_terminal::keyboard::key_identity(&key.key)
            .and_then(|identity| self.pressed_keys.remove(&identity))
        else {
            return;
        };
        let mods = rmux_terminal::keyboard::Modifiers {
            shift: key.modifiers.shift,
            control: key.modifiers.control,
            alt: key.modifiers.alt,
        };
        if let Err(error) = self.session.key_release(&pressed, mods) {
            self.error = Some(error.to_string());
            cx.notify();
        }
        cx.stop_propagation();
    }
    fn drop_files(&mut self, paths: &ExternalPaths, window: &mut Window, cx: &mut Context<Self>) {
        cx.stop_propagation();
        if !self.focus.is_focused(window) || self.copy_mode.is_some() || self.find.is_some()
            || self.ports.visible || cx.global::<PromptGate>().is_active()
            || cx.global::<Shutdowns>().quitting || cx.global::<Shutdowns>().quit_prompt {
            self.error = Some("Focus the terminal and close its overlays before dropping files".into());
            cx.notify();
            return;
        }
        #[cfg(unix)]
        let result = {
            self.image_paste_task = None;
            file_drop::quoted_paths(paths.paths()).and_then(|text| self.session.paste_owned(text))
        };
        #[cfg(not(unix))]
        let result: anyhow::Result<()> = { let _ = paths; Err(anyhow::anyhow!("File drops need Windows shell quoting support")) };
        self.error = result.err().map(|error| error.to_string());
        self.refresh_input_mode(cx);
        cx.notify();
    }
    #[cfg(unix)]
    fn paste_image(&mut self, image: Image, window: &mut Window, cx: &mut Context<Self>) {
        if self.image_paste_task.is_some() { self.error = Some("An image paste is already being prepared".into()); cx.notify(); return; }
        if !self.focus.is_focused(window) || self.copy_mode.is_some() || self.find.is_some() || self.ports.visible
            || cx.global::<PromptGate>().is_active() || cx.global::<Shutdowns>().quitting || cx.global::<Shutdowns>().quit_prompt { return; }
        let reservation = match image_paste::Reservation::acquire(image.bytes.len()) {
            Ok(reservation) => reservation,
            Err(error) => { self.error = Some(error.to_string()); cx.notify(); return; }
        };
        let focus = self.focus.clone();
        let executor = cx.background_executor().clone();
        self.image_paste_task = Some(cx.spawn_in(window, async move |entity, cx| {
            let prepared = executor.spawn(async move { image_paste::Prepared::write(image, reservation) }).await;
            let _ = cx.update(|window, cx| entity.update(cx, |this, cx| {
                this.image_paste_task = None;
                cx.notify();
                if !focus.is_focused(window) || this.copy_mode.is_some() || this.find.is_some() || this.ports.visible
                    || cx.global::<PromptGate>().is_active() || cx.global::<Shutdowns>().quitting || cx.global::<Shutdowns>().quit_prompt { return; }
                let result = prepared.and_then(|file| {
                    let text = file_drop::quoted_paths(&[file.path().to_owned()])?;
                    this.session.paste_owned(text)?;
                    this.pasted_images.push(file);
                    Ok(())
                });
                this.error = result.err().map(|error: anyhow::Error| error.to_string());
                cx.notify();
            }));
        }));
        cx.notify();
    }
    fn paste(&mut self, _: &Paste, window: &mut Window, cx: &mut Context<Self>) {
        self.refresh_input_mode(cx);
        #[cfg(target_os = "macos")]
        match file_clipboard::read_paths() {
            Ok(Some(paths)) => {
                if !self.focus.is_focused(window) || self.copy_mode.is_some() || self.find.is_some() || self.ports.visible
                    || cx.global::<PromptGate>().is_active() || cx.global::<Shutdowns>().quitting || cx.global::<Shutdowns>().quit_prompt { return; }
                self.cancel_image_preparation(cx);
                self.error = file_drop::quoted_paths(&paths).and_then(|text| self.session.paste_owned(text)).err().map(|error| error.to_string());
                cx.notify();
                return;
            }
            Err(error) => { self.error = Some(error.to_string()); cx.notify(); return; }
            Ok(None) => {},
        }
        let Some(item) = cx.read_from_clipboard() else { return; };
        if let Some(text) = item.text() {
            #[cfg(unix)]
            { self.image_paste_task = None; }
            self.error = self.session.paste_owned(text).err().map(|error| error.to_string());
            cx.notify();
        } else {
            #[cfg(unix)]
            if let Some(ClipboardEntry::Image(image)) = item.into_entries().next() { self.paste_image(image, window, cx); }
            #[cfg(not(unix))]
            { let _ = window; self.error = Some("Image paste needs Windows shell quoting support".into()); cx.notify(); }
        }
    }
    fn select(
        &mut self,
        action: rmux_terminal::selection::Action,
        position: Point<Pixels>,
        rectangle: bool,
        clicks: usize,
        cx: &mut Context<Self>,
    ) {
        self.clipboard_task = None;
        if self.selecting {
            self.selection_pointer = Some((position, rectangle));
        }
        let position = position - self.last_bounds.origin;
        let result = self
            .session
            .selection_event(rmux_terminal::selection::Event {
                action,
                x: f32::from(position.x),
                y: f32::from(position.y),
                cell_width: self.cells.width,
                cell_height: self.cells.height,
                rectangle,
                clicks,
            });
        match result {
            Ok(true) if self.selecting => {
                if self.selection_scroll.is_none() {
                    let timer = cx
                        .background_executor()
                        .timer(std::time::Duration::from_millis(30));
                    self.selection_scroll = Some(cx.spawn(async move |entity, cx| {
                        timer.await;
                        let _ = entity.update(cx, |view, cx| {
                            view.selection_scroll = None;
                            if view.selecting
                                && let Some((position, rectangle)) = view.selection_pointer
                            {
                                view.select(
                                    rmux_terminal::selection::Action::Autoscroll,
                                    position,
                                    rectangle,
                                    1,
                                    cx,
                                );
                            }
                        });
                    }));
                }
            }
            Ok(_) => self.selection_scroll = None,
            Err(error) => {
                self.error = Some(error.to_string());
                self.selection_scroll = None;
            }
        }
        cx.notify();
    }
    fn copy(&mut self, _: &CopySelection, _: &mut Window, cx: &mut Context<Self>) {
        self.copy_selection(true, cx);
    }
    fn copy_selection(&mut self, unwrap: bool, cx: &mut Context<Self>) {
        // Latest user intent wins, including changing copy modes while an older
        // export waits on output. Invalidate it before canceling its UI task.
        let ticket = clipboard::begin();
        self.clipboard_task = None;
        let export = self.session.clipboard_export(unwrap, move || clipboard::current(ticket));
        let executor = cx.background_executor().clone();
        self.clipboard_task = Some(cx.spawn(async move |entity, cx| {
            let result = executor.spawn(async move { export() }).await;
            let _ = entity.update(cx, |this, cx| {
                this.clipboard_task = None;
                if !clipboard::current(ticket) { return; }
                match result {
                    Ok(Some(text)) => clipboard::write(ClipboardItem::new_string(text), cx),
                    Ok(None) => {},
                    Err(error) => { this.error = Some(error.to_string()); cx.notify(); }
                }
            });
        }));
    }

    fn copy_working_directory(
        &mut self,
        _: &CopyWorkingDirectory,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.lookup_working_directory(true, cx);
    }
    fn reveal_working_directory(
        &mut self,
        _: &RevealWorkingDirectory,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.lookup_working_directory(false, cx);
    }
    fn lookup_working_directory(&mut self, copy: bool, cx: &mut Context<Self>) {
        let generation = self.directory_generation.fetch_add(1, Ordering::Relaxed).wrapping_add(1);
        self.directory_task = None;
        if self.session.remote_destination().is_some() {
            if copy {
                if let Some(path) = &self.remote_directory_label {
                    clipboard::write(ClipboardItem::new_string(path.clone()), cx);
                    self.error = None;
                } else {
                    self.error = Some("No remote working directory reported by this terminal.".into());
                }
            } else {
                self.error = Some("Remote directories cannot be revealed in the local file manager.".into());
            }
            cx.notify();
            return;
        }
        let ticket = copy.then(clipboard::begin);
        let current = Arc::downgrade(&self.directory_generation);
        let lookup = self.session.directory_lookup(move || {
            current.upgrade().is_some_and(|current| current.load(Ordering::Relaxed) == generation)
                && ticket.is_none_or(clipboard::current)
        });
        let executor = cx.background_executor().clone();
        self.directory_task = Some(cx.spawn(async move |entity, cx| {
            let result = executor.spawn(async move { lookup() }).await;
            let _ = entity.update(cx, |this, cx| {
                this.directory_task = None;
                if ticket.is_some_and(|ticket| !clipboard::current(ticket)) { return; }
                match result {
                    Ok(Some(path)) if copy => {
                        if let Some(text) = path.to_str() {
                            clipboard::write(ClipboardItem::new_string(text.to_owned()), cx);
                        } else {
                            this.error = Some("Working directory cannot be represented as clipboard text.".into());
                        }
                    }
                    Ok(Some(path)) => cx.reveal_path(&path),
                    Ok(None) => this.error = Some("No local working directory reported by this terminal.".into()),
                    Err(error) => this.error = Some(error.to_string()),
                }
                cx.notify();
            });
        }));
    }
    fn select_all(&mut self, _: &EditorSelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.clipboard_task = None;
        self.selecting = false;
        self.selection_scroll = None;
        self.selection_pointer = None;
        if let Err(error) = self.session.select_all() {
            self.error = Some(error.to_string());
        }
        cx.notify();
    }
    fn clear_scrollback(
        &mut self,
        _: &ClearScrollback,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.clipboard_task = None;
        self.selecting = false;
        self.selection_scroll = None;
        self.selection_pointer = None;
        self.scroll_remainder = 0.;
        if self.dismiss_find(cx) {
            self.focus.focus(window);
        }
        if let Err(error) = self.session.clear_scrollback() {
            self.error = Some(error.to_string());
        }
        cx.notify();
    }
    fn dismiss_find(&mut self, cx: &App) -> bool {
        let Some(find) = self.find.take() else {
            return false;
        };
        self.last_find_query = find.read(cx).query(cx).to_owned();
        self.find_history = find.read(cx).saved_history(cx);
        true
    }
    fn open_find(&mut self, _: &Find, window: &mut Window, cx: &mut Context<Self>) {
        self.open_find_with_query(None, window, cx);
    }
    fn open_find_with_query(
        &mut self,
        initial_query: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.compression_task = None;
        if let Some(find) = &self.find {
            find.update(cx, |find, cx| find.focus_query(window, cx));
            return;
        }
        match self.session.search_worker() {
            Ok(worker) => {
                let initial_query = initial_query.unwrap_or_else(|| {
                    self.session
                        .selected_search_query()
                        .unwrap_or_else(|| self.last_find_query.clone())
                });
                self.install_find(worker, initial_query, window, cx);
            }
            Err(error) => {
                self.error = Some(error.to_string());
                cx.notify();
            }
        }
    }
    fn install_find(&mut self, worker: rmux_terminal::search_worker::SearchWorker, initial_query: String, window: &mut Window, cx: &mut Context<Self>) {
        self.compression_task = None;
        let parent = cx.weak_entity();
        let find = cx.new(|cx| find_bar::FindBar::new(worker, parent, &initial_query, std::mem::take(&mut self.find_history), cx));
        cx.subscribe_in(
            &find,
            window,
            |this, _, _: &find_bar::Closed, window, cx| {
                this.dismiss_find(cx);
                this.focus.focus(window);
                this.schedule_compression(cx);
                cx.notify();
            },
        )
        .detach();
        find.read(cx).focus.focus(window);
        self.find = Some(find);
        cx.notify();
    }
    fn navigate_find(
        &mut self,
        direction: rmux_terminal::search::Direction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.find.is_none() {
            let remembered = (!self.last_find_query.is_empty())
                .then(|| self.last_find_query.clone());
            self.open_find_with_query(remembered, window, cx);
        }
        if let Some(find) = &self.find {
            find.update(cx, |find, cx| find.navigate(direction, cx));
        }
    }
}
impl Focusable for TerminalView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}
impl Render for TerminalView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ended_ssh = self.session.remote_destination().is_some();
        let entity = cx.weak_entity();
        self.rendered_paste_pending = self.session.paste_pending();
        div().id("terminal-interaction")
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .track_focus(&self.focus)
            .key_context("Terminal")
            .on_key_down(cx.listener(Self::key))
            .on_key_up(cx.listener(Self::key_up))
            .on_scroll_wheel(cx.listener(Self::wheel))
            .on_drop(cx.listener(Self::drop_files))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(|this, action: &SendShortcutText, window, cx| {
                cx.stop_propagation();
                if !this.focus.is_focused(window) || this.copy_mode.is_some() || this.find.is_some() || this.ports.visible
                    || cx.global::<PromptGate>().is_active() || cx.global::<Shutdowns>().quitting || cx.global::<Shutdowns>().quit_prompt { return; }
                this.input(action.text.as_bytes(), cx);
            }))
            .on_action(cx.listener(|this, _: &EnterCopyMode, window, cx| {
                if let Some(mode) = &this.copy_mode { mode.read(cx).focus.focus(window); return; }
                this.selecting = false;
                this.selection_scroll = None;
                this.selection_pointer = None;
                let mode = cx.new(|cx| copy_mode::CopyMode::new(&this.session, window, cx));
                cx.subscribe_in(&mode, window, |this, _, _: &copy_mode::Close, window, cx| {
                    this.copy_mode = None;
                    this.focus.focus(window);
                    cx.notify();
                }).detach();
                this.copy_mode = Some(mode);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &ShowListeningPorts, window, cx| this.refresh_ports(window, cx)))
            .on_action(cx.listener(|this, _: &CloseListeningPorts, window, cx| this.close_ports(window, cx)))
            .on_action(cx.listener(|this, _: &ShowTerminalMenu, window, cx| {
                this.show_terminal_menu(this.last_bounds.origin + point(px(8.), px(8.)), window, cx);
            }))
            .on_action(cx.listener(|this, _: &CancelQueuedPaste, _, cx| this.cancel_paste(cx)))
            .on_action(cx.listener(Self::copy))
            .on_action(cx.listener(|this, _: &CopySelectionWithLineBreaks, _, cx| this.copy_selection(false, cx)))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::copy_working_directory))
            .on_action(cx.listener(Self::reveal_working_directory))
            .on_action(cx.listener(|this, _: &RefreshGitContext, _, cx| this.refresh_git_context(cx)))
            .on_action(cx.listener(Self::clear_scrollback))
            .on_action(
                cx.listener(|this, _: &PreviousPrompt, _, cx| this.scroll(Scroll::Prompts(-1), cx)),
            )
            .on_action(
                cx.listener(|this, _: &NextPrompt, _, cx| this.scroll(Scroll::Prompts(1), cx)),
            )
            .on_action(cx.listener(|this, _: &ScrollToTop, _, cx| this.scroll(Scroll::Top, cx)))
            .on_action(
                cx.listener(|this, _: &LastPrompt, _, cx| this.scroll(Scroll::LastPrompt, cx)),
            )
            .on_action(
                cx.listener(|this, _: &ScrollToBottom, _, cx| this.scroll(Scroll::Bottom, cx)),
            )
            .on_action(cx.listener(Self::open_find))
            .on_action(cx.listener(|this, _: &FindOlder, window, cx| {
                this.navigate_find(rmux_terminal::search::Direction::Older, window, cx)
            }))
            .on_action(cx.listener(|this, _: &FindNewer, window, cx| {
                this.navigate_find(rmux_terminal::search::Direction::Newer, window, cx)
            }))
            .on_action(cx.listener(|view, _: &ToggleProgramStatus, window, cx| {
                view.toggle_status_details(window, cx);
            }))
            .on_action(cx.listener(|view, _: &IncreaseFontSize, _, cx| {
                view.set_font_size(view.cells.font_size + 1., cx)
            }))
            .on_action(cx.listener(|view, _: &DecreaseFontSize, _, cx| {
                view.set_font_size(view.cells.font_size - 1., cx)
            }))
            .on_action(cx.listener(|view, _: &ResetFontSize, _, cx| {
                view.set_font_size(cx.global::<Preferences>().font_size, cx)
            }))
            .on_action(cx.listener(|view, action: &SetTerminalFont, _, cx| {
                if !action.family.trim().is_empty() && action.family.len() <= 128 && !action.family.chars().any(char::is_control) {
                    view.set_font(view.cells.font_size, action.family.clone().into(), cx);
                }
            }))
            .on_action(cx.listener(|view, _: &UseConfiguredFont, _, cx| {
                let settings = cx.global::<Preferences>();
                let (size, family) = (settings.font_size, settings.font_family.clone().into());
                view.set_font(size, family, cx);
            }))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::mouse_down))
            .on_mouse_down(MouseButton::Middle, cx.listener(Self::mouse_down))
            .on_mouse_down(MouseButton::Right, cx.listener(Self::mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::mouse_up))
            .on_mouse_up(MouseButton::Middle, cx.listener(Self::mouse_up))
            .on_mouse_up(MouseButton::Right, cx.listener(Self::mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::mouse_up))
            .on_mouse_up_out(MouseButton::Middle, cx.listener(Self::mouse_up))
            .on_mouse_up_out(MouseButton::Right, cx.listener(Self::mouse_up))
            .on_mouse_move(cx.listener(Self::mouse_move))
            .on_modifiers_changed(cx.listener(|this, _, window, cx| this.update_link_hover(window.mouse_position(), window.modifiers(), cx)))
            .on_hover(cx.listener(|this, hovered, window, cx| {
                if *hovered { this.update_link_hover(window.mouse_position(), window.modifiers(), cx); }
                else { this.clear_link_hover(cx); }
            }))
            .when_some(self.link_hover.as_ref().filter(|(revision, _)| *revision == self.frame.revision).map(|(_, uri)| uri.clone()), |root, uri| {
                let allowed = openable_uri(&uri);
                let target: SharedString = uri.into();
                root.when(allowed, |root| root.cursor_pointer())
                    .tooltip(move |_, cx| cx.new(|_| LinkHint { target: target.clone(), allowed }).into())
            })
            .child(
                canvas(
                    |_, _, _| (),
                    move |_, _, window, _| {
                        let moving = entity.clone();
                        window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
                            if phase == DispatchPhase::Capture
                                && moving
                                    .read_with(cx, |view, _| view.selecting)
                                    .unwrap_or(false)
                            {
                                let _ = moving
                                    .update(cx, |view, cx| view.mouse_move(event, window, cx));
                            }
                        });
                        let released = entity.clone();
                        window.on_mouse_event(move |event: &MouseUpEvent, phase, window, cx| {
                            if phase == DispatchPhase::Capture
                                && released
                                    .read_with(cx, |view, _| view.selecting)
                                    .unwrap_or(false)
                            {
                                let _ = released
                                    .update(cx, |view, cx| view.mouse_up(event, window, cx));
                            }
                        });
                    },
                )
                .absolute()
                .size_full(),
            )
            .when_some(self.find.clone(), |root, find| root.child(find))
            .when_some(self.status_details.as_ref().zip(self.status_ui.as_ref()), |root, (records, state)| {
                root.child(status_view::panel(records, self.status_load.is_some(), state, window, cx))
            })
            .when(self.ports.visible, |root| root.child(ports_view::panel(&self.ports, window, cx)))
            .child(TerminalElement { view: cx.entity() })
            .when(self.hidden_line_input, |root| root.child(
                div().id("hidden-line-input").absolute().bottom(px(8.)).left(px(8.)).px_2().py_1()
                    .rounded_sm().bg(rgb(0x292929)).text_xs().text_color(rgb(0xb8c4d4))
                    .tooltip(|_, cx| cx.new(|_| pane_view::ControlHint("The local terminal has disabled echo for line input. Remote and raw-mode password prompts may not be detected.".into())).into())
                    .child("Input hidden")
            ))
            .when(self.rendered_paste_pending || self.image_paste_pending(), |root| root.child(
                div().id("paste-status").absolute().bottom(px(8.)).right(px(8.))
                    .max_w_full().px_2().h(px(28.)).flex().items_center().gap_2()
                    .occlude().rounded_md().border_1().border_color(rgb(0x465063))
                    .text_xs().text_color(rgb(0xc6cdd8)).bg(rgb(0x292929))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_mouse_down(MouseButton::Middle, |_, _, cx| cx.stop_propagation())
                    .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
                    .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
                    .child(div().min_w_0().truncate().child(if self.image_paste_pending() { "Preparing image paste…" } else { "Paste pending…" }))
                    .child(pane_view::pane_control("cancel-paste", "Cancel Queued Paste")
                        .on_click(cx.listener(|this, event: &ClickEvent, _, cx| {
                            if event.standard_click() { this.cancel_paste(cx); }
                            cx.stop_propagation();
                        })).child(icons::icon("close")))
            ))
            .when_some(self.saved_ssh_destination.clone().filter(|_| !ended_ssh), |root, destination| root.child(
                div().id("saved-ssh-notice").flex().flex_wrap().items_center().gap_2()
                    .px_2().py_1().flex_shrink_0().min_w_0().bg(rgb(0x292929))
                    .border_t_1().border_color(rgb(0x414141)).text_xs().text_color(rgb(0xb8b8b8))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_mouse_down(MouseButton::Middle, |_, _, cx| cx.stop_propagation())
                    .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
                    .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
                    .child(div().id("saved-ssh-message").flex_1().min_w_0().truncate()
                        .tooltip({ let destination = destination.clone(); move |_, cx| {
                            cx.new(|_| pane_view::ControlHint(format!("Local shell. Saved SSH destination: {destination}. Not connected.").into())).into()
                        }})
                        .child(format!("Local shell · SSH not connected: {destination}")))
                    .child(div().id("connect-saved-ssh").px_2().py_1().rounded_sm().bg(rgb(0x383838))
                        .cursor_pointer().hover(|style| style.bg(rgb(0x484848)))
                        .tooltip(|_, cx| cx.new(|_| pane_view::ControlHint("Connect to the saved SSH host in a new workspace; keep this local shell".into())).into())
                        .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
                            if event.standard_click() {
                                this.focus.focus(window);
                                window.dispatch_action(Box::new(ConnectSsh { destination: destination.clone() }), cx);
                            }
                            cx.stop_propagation();
                        })).child("Connect SSH"))
            ))
            .when_some(self.process_exit.clone(), |root, status| root.child(
                div().id("process-exit-status").flex().flex_wrap().items_center().gap_2()
                    .px_2().py_1().flex_shrink_0().min_w_0().bg(rgb(0x292929))
                    .border_t_1().border_color(rgb(0x414141)).text_xs().text_color(rgb(0xb8b8b8))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_mouse_down(MouseButton::Middle, |_, _, cx| cx.stop_propagation())
                    .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
                    .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
                    .child(div().id("process-exit-message").flex_1().min_w_0().truncate()
                        .tooltip({ let hint: SharedString = status.clone().into(); move |_, cx| cx.new(|_| pane_view::ControlHint(hint.clone())).into() })
                        .child(status))
                    .child(div().id("new-shell-after-exit").px_2().py_1().rounded_sm().bg(rgb(0x383838))
                        .cursor_pointer().hover(|style| style.bg(rgb(0x484848)))
                        .tooltip(move |_, cx| cx.new(|_| pane_view::ControlHint(if ended_ssh {
                            "Reconnect to the same SSH destination in a new tab, keeping this terminal's output"
                        } else {
                            "Open a fresh shell in a new tab, keeping this terminal's output"
                        }.into())).into())
                        .on_click(cx.listener(|this, event: &ClickEvent, window, cx| {
                            if event.standard_click() {
                                this.focus.focus(window);
                                window.dispatch_action(Box::new(NewTab), cx);
                            }
                            cx.stop_propagation();
                        })).child(if ended_ssh { "Reconnect SSH" } else { "New shell" }))
                    .when(ended_ssh, |row| row.child(
                        div().id("local-shell-after-ssh-exit").px_2().py_1().rounded_sm()
                            .cursor_pointer().hover(|style| style.bg(rgb(0x383838)))
                            .tooltip(|_, cx| cx.new(|_| pane_view::ControlHint("Open a local shell in a new tab without reconnecting SSH".into())).into())
                            .on_click(cx.listener(|this, event: &ClickEvent, window, cx| {
                                if event.standard_click() {
                                    this.focus.focus(window);
                                    window.dispatch_action(Box::new(NewLocalTab), cx);
                                }
                                cx.stop_propagation();
                            })).child("Local shell")
                    ))
                    .child(pane_view::pane_control("close-exited-terminal", "Close finished terminal")
                        .on_click(cx.listener(|this, event: &ClickEvent, window, cx| {
                            if event.standard_click() {
                                this.focus.focus(window);
                                window.dispatch_action(Box::new(ClosePane), cx);
                            }
                            cx.stop_propagation();
                        })).child(icons::icon("close")))
            ))
            .when_some(self.error.clone(), |div, error| div.child(div_error(error,
                cx.listener(|this, event: &ClickEvent, _, cx| {
                    if event.standard_click() { this.error = None; cx.stop_propagation(); cx.notify(); }
                }))))
            .when_some(self.copy_mode.clone(), |root, mode| root.child(mode))
            .when_some(self.terminal_menu.clone(), |root, menu| root.child(self.terminal_menu_element(menu, window, cx)))
    }
}
fn div_error(error: String, dismiss: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> impl IntoElement {
    let copy = error.clone();
    div().flex().items_start().gap_2().px_2().py_1().flex_shrink_0().min_w_0()
        .bg(rgb(0x302626)).border_t_1().border_color(rgb(0x644141))
        .text_sm().text_color(rgb(0xf2a5a5))
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_mouse_down(MouseButton::Middle, |_, _, cx| cx.stop_propagation())
        .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
        .on_mouse_move(|_, _, cx| cx.stop_propagation())
        .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
        .child(div().id("error-message").flex_1().min_w_0().max_h(px(96.)).overflow_y_scroll().py_1().child(error))
        .child(pane_view::pane_control("copy-error", "Copy error message")
            .on_click(move |event, _, cx| {
                if event.standard_click() { clipboard::write(ClipboardItem::new_string(copy.clone()), cx); cx.stop_propagation(); }
            }).child(icons::icon("copy")))
        .child(pane_view::pane_control("dismiss-error", "Dismiss error message")
            .on_click(dismiss).child(icons::icon("close")))
}
fn terminal_button(button: MouseButton) -> Option<rmux_terminal::mouse::Button> {
    Some(match button {
        MouseButton::Left => rmux_terminal::mouse::Button::Left,
        MouseButton::Middle => rmux_terminal::mouse::Button::Middle,
        MouseButton::Right => rmux_terminal::mouse::Button::Right,
        _ => return None,
    })
}

// IME preedit stays local until committed. Native text input handles printable
// characters; the key listener handles only terminal control keys.
impl EntityInputHandler for TerminalView {
    fn text_for_range(
        &mut self,
        range: Range<usize>,
        actual: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        let (text, adjusted) = self.composition.substring(range);
        *actual = Some(adjusted);
        Some(text)
    }
    fn selected_text_range(
        &mut self,
        _: bool,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.composition.selected.clone(),
            reversed: false,
        })
    }
    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        self.composition.marked.clone()
    }
    fn unmark_text(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let text = self.composition.take();
        if !text.is_empty() {
            self.input(text.as_bytes(), cx);
        }
        cx.notify();
    }
    fn replace_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let text = self.composition.commit(range, text);
        self.input(text.as_bytes(), cx);
        cx.notify();
    }
    fn replace_and_mark_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        selection: Option<Range<usize>>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.composition.update(range, text, selection);
        cx.notify();
    }
    fn bounds_for_range(
        &mut self,
        range: Range<usize>,
        _: Bounds<Pixels>,
        window: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let (x, y) = self.frame.cursor.unwrap_or_default();
        let (offset, width) = if self.composition.text.is_empty() {
            (px(0.), px(self.cells.width))
        } else {
            let line = shape_preedit(
                &self.composition.text,
                self.cells,
                &self.font_family,
                window,
            );
            let range = self.composition.byte_range(range);
            let start = line.x_for_index(range.start);
            let end = line.x_for_index(range.end);
            (start.min(end), (end - start).abs().max(px(2.)))
        };
        Some(Bounds::new(
            self.last_bounds.origin
                + point(
                    px(x as f32 * self.cells.width) + offset,
                    px(y as f32 * self.cells.height),
                ),
            size(width, px(self.cells.height)),
        ))
    }
    fn character_index_for_point(
        &mut self,
        position: Point<Pixels>,
        window: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<usize> {
        if self.composition.text.is_empty() {
            return Some(0);
        }
        let (x, y) = self.frame.cursor?;
        let origin = self.last_bounds.origin
            + point(
                px(x as f32 * self.cells.width),
                px(y as f32 * self.cells.height),
            );
        if position.y < origin.y || position.y > origin.y + px(self.cells.height) {
            return None;
        }
        let line = shape_preedit(
            &self.composition.text,
            self.cells,
            &self.font_family,
            window,
        );
        Some(
            self.composition
                .utf16_offset(line.closest_index_for_x(position.x - origin.x)),
        )
    }
}

fn shape_preedit(
    text: &str,
    cells: CellMetrics,
    family: &SharedString,
    window: &Window,
) -> ShapedLine {
    let run = TextRun {
        len: text.len(),
        font: font(family.clone()),
        color: rgb(0xffffff).into(),
        background_color: None,
        underline: Some(UnderlineStyle {
            thickness: px(1.),
            color: None,
            wavy: false,
        }),
        strikethrough: None,
    };
    window
        .text_system()
        .shape_line(text.to_owned().into(), px(cells.font_size), &[run], None)
}

struct TerminalElement {
    view: Entity<TerminalView>,
}
struct PaintedText {
    line: ShapedLine,
    origin: Point<Pixels>,
}
struct TerminalPaint {
    content_bounds: Bounds<Pixels>,
    selections: Arc<Vec<Option<(u16, u16)>>>,
    scrollbar: Option<PaintQuad>,
    rows: Arc<Vec<Arc<row_cache::ShapedRow>>>,
    backgrounds: Vec<PaintQuad>,
    preedit_backgrounds: Vec<PaintQuad>,
    lines: Vec<PaintedText>,
    cursor: Vec<PaintQuad>,
    cursor_text: Option<PaintedText>,
}
impl IntoElement for TerminalElement {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl Element for TerminalElement {
    type RequestLayoutState = ();
    type PrepaintState = TerminalPaint;
    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        let mut style = Style::default();
        style.size = size(relative(1.).into(), relative(1.).into());
        style.flex_grow = 1.;
        style.flex_shrink = 1.;
        (window.request_layout(style, [], cx), ())
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> TerminalPaint {
        let outer_bounds = bounds;
        let bounds = cell_metrics::content_bounds(bounds, cx.global::<Preferences>().terminal_padding);
        let (cells, family) = self.view.update(cx, |view, _| {
            if view.measured_font_size != Some(view.cells.font_size) {
                view.cells =
                    CellMetrics::measured(view.cells.font_size, view.font_family.clone(), window);
                view.measured_font_size = Some(view.cells.font_size);
            }
            (view.cells, view.font_family.clone())
        });
        let started = self
            .view
            .read(cx)
            .metrics
            .as_ref()
            .map(|_| std::time::Instant::now());
        let columns = ((f32::from(bounds.size.width) / cells.width) as u16).max(1);
        let rows = ((f32::from(bounds.size.height) / cells.height) as u16).max(1);
        let (frame, marked, focused) = self.view.update(cx, |view, cx| {
            let ready = match view.session.try_resize(columns, rows) {
                Ok(Some(changed)) => {
                    if changed && let Some(find) = &view.find { find.read(cx).worker.refresh(); }
                    true
                }
                Ok(None) => {
                    if let Some(metrics) = &mut view.metrics { metrics.resize_busy += 1; }
                    window.request_animation_frame();
                    false
                }
                Err(error) => { view.error = Some(error.to_string()); false }
            };
            if ready {
                match view.session.try_frame() {
                    Ok(Some(frame)) => {
                        if frame.revision != view.frame.revision { view.clear_link_hover(cx); }
                        view.frame = frame;
                        view.render_snapshot_released = false;
                    },
                    Ok(None) => {
                        // Keep the last coherent frame instead of waiting behind output
                        // parsing/history work on the UI thread. Retry only while busy.
                        if let Some(metrics) = &mut view.metrics { metrics.snapshot_busy += 1; }
                        window.request_animation_frame();
                    }
                    Err(error) => view.error = Some(error.to_string()),
                }
            }
            if let Some(metrics) = &mut view.metrics {
                metrics.snapshot_recovery.observe(view.render_snapshot_released, std::time::Instant::now());
            }
            view.last_bounds = bounds;
            (
                view.frame.clone(),
                view.composition.text.clone(),
                window.is_window_active() && view.focus.is_focused(window),
            )
        });
        let mut background = color(frame.background);
        background.a = f32::from(cx.global::<Preferences>().background_opacity) / 100.;
        let backgrounds = vec![fill(outer_bounds, background)];
        let mut lines = Vec::new();
        let mut preedit_backgrounds = Vec::new();
        let selection_foreground = cx.global::<Preferences>().selection_foreground();
        let shaped_rows = self.view.update(cx, |view, _| {
            let rows = view.row_cache.update(&frame, cells, &family, window, view.metrics.as_mut());
            view.row_cache.prepare_selection(&frame, selection_foreground, cells, &family, window);
            rows
        });
        let mut cursor_text = None;
        let cursor_shape = cx.global::<Preferences>().cursor_shape(
            focused,
            !marked.is_empty(),
            frame.cursor_shape,
        );
        let cursor = frame
            .cursor
            .zip(cursor_shape)
            .map(|((x, y), shape)| {
                let mut origin =
                    bounds.origin + point(px(x as f32 * cells.width), px(y as f32 * cells.height));
                if focused && !marked.is_empty() {
                    let line = shape_preedit(&marked, cells, &family, window);
                    let composition = &self.view.read(cx).composition;
                    let selection = composition.byte_range(composition.selected.clone());
                    let start = line.x_for_index(selection.start);
                    let offset = line.x_for_index(selection.end);
                    preedit_backgrounds.push(fill(
                        Bounds::new(origin, size(line.width + px(2.), px(cells.height))),
                        color(frame.background),
                    ));
                    if !selection.is_empty() {
                        preedit_backgrounds.push(fill(
                            Bounds::new(
                                origin + point(start.min(offset), px(0.)),
                                size((offset - start).abs(), px(cells.height)),
                            ),
                            rgb(0x344b70),
                        ));
                    }
                    lines.push(PaintedText { line, origin });
                    origin.x += offset;
                }
                let cell = frame
                    .rows
                    .get(y as usize)
                    .and_then(|row| row.get(x as usize));
                let width = cell.map_or(1, |cell| cell.width.max(1)) as f32 * cells.width;
                if shape == CursorShape::Block
                    && let Some(cell) = cell
                    && !cell.text.is_empty()
                    && !cell.hidden
                {
                    let foreground = cx.global::<Preferences>().cursor_text(cell.foreground, cell.background);
                    cursor_text = Some(PaintedText {
                        line: self.view.update(cx, |view, _| view.cursor_cache.shape(cell, foreground, cells, &family, window, view.metrics.as_mut())),
                        origin,
                    });
                }
                let cursor_fill = cell.map(|cell| cx.global::<Preferences>().cursor_fill(cell.foreground, cell.background, frame.cursor_color, frame.cursor_color_explicit)).unwrap_or(frame.cursor_color);
                let paint = |offset, dimensions| {
                    fill(
                        Bounds::new(origin + offset, dimensions),
                        color(cursor_fill),
                    )
                };
                match shape {
                    CursorShape::Bar => {
                        vec![paint(point(px(0.), px(0.)), size(px(2.), px(cells.height)))]
                    }
                    CursorShape::Underline => vec![paint(
                        point(px(0.), px(cells.height - 2.)),
                        size(px(width), px(2.)),
                    )],
                    CursorShape::Block => vec![paint(
                        point(px(0.), px(0.)),
                        size(px(width), px(cells.height)),
                    )],
                    CursorShape::Hollow => vec![
                        paint(point(px(0.), px(0.)), size(px(width), px(1.))),
                        paint(
                            point(px(0.), px(cells.height - 1.)),
                            size(px(width), px(1.)),
                        ),
                        paint(point(px(0.), px(0.)), size(px(1.), px(cells.height))),
                        paint(
                            point(px(width - 1.), px(0.)),
                            size(px(1.), px(cells.height)),
                        ),
                    ],
                }
            })
            .unwrap_or_default();
        let state = TerminalPaint {
            content_bounds: bounds,
            selections: frame.selections.clone(),
            scrollbar: (frame.scrollback_offset > 0 && frame.scrollback_lines > 0).then(|| {
                let height = f32::from(bounds.size.height);
                let total = frame.scrollback_lines as f32 + frame.rows.len() as f32;
                let thumb = (height * frame.rows.len() as f32 / total)
                    .max(8.)
                    .min(height);
                let offset = (frame.scrollback_lines - frame.scrollback_offset) as f32
                    / frame.scrollback_lines as f32;
                fill(
                    Bounds::new(
                        bounds.origin
                            + point(bounds.size.width - px(3.), px((height - thumb) * offset)),
                        size(px(3.), px(thumb)),
                    ),
                    rgb(0x667085),
                )
            }),
            rows: shaped_rows,
            backgrounds,
            preedit_backgrounds,
            lines,
            cursor,
            cursor_text,
        };
        if let Some(started) = started {
            self.view.update(cx, |view, _| {
                if let Some(metrics) = &mut view.metrics {
                    metrics.prepaint.record(started.elapsed());
                }
            });
        }
        state
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        state: &mut TerminalPaint,
        window: &mut Window,
        cx: &mut App,
    ) {
        let cells = self.view.read(cx).cells;
        let started = self
            .view
            .read(cx)
            .metrics
            .as_ref()
            .map(|_| std::time::Instant::now());
        let focus = self.view.read(cx).focus.clone();
        window.handle_input(
            &focus,
            ElementInputHandler::new(state.content_bounds, self.view.clone()),
            cx,
        );
        window.with_content_mask(Some(ContentMask { bounds }), |window| {
            for quad in state.backgrounds.drain(..) {
                window.paint_quad(quad);
            }
        });
        let selection_background = cx.global::<Preferences>().selection_background();
        window.with_content_mask(Some(ContentMask { bounds: state.content_bounds }), |window| {
            for (y, row) in state.rows.iter().enumerate() {
                row_cache::paint_row(
                    row,
                    state.content_bounds.origin + point(px(0.), px(y as f32 * cells.height)),
                    state.selections.get(y).copied().flatten(),
                    selection_background,
                    cells,
                    window,
                    cx,
                );
            }
            for quad in state.preedit_backgrounds.drain(..) {
                window.paint_quad(quad);
            }
            for text in &state.lines {
                let _ = text.line.paint(text.origin, px(cells.height), window, cx);
            }
            for cursor in state.cursor.drain(..) {
                window.paint_quad(cursor);
            }
            if let Some(text) = &state.cursor_text {
                let _ = text.line.paint(text.origin, px(cells.height), window, cx);
            }
            if let Some(scrollbar) = state.scrollbar.take() {
                window.paint_quad(scrollbar);
            }
        });
        if let Some(started) = started {
            self.view.update(cx, |view, _| {
                if let Some(metrics) = &mut view.metrics {
                    metrics.paint.record(started.elapsed());
                }
            });
        }
    }
}
struct LinkHint { target: SharedString, allowed: bool }
impl Render for LinkHint {
    fn render(&mut self, window: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().w(px(520.).min((window.viewport_size().width - px(24.)).max(px(1.))))
            .px_2().py_1().rounded_md().bg(rgb(0x383838)).text_color(rgb(0xe5e5e5))
            .child(div().text_xs().text_color(rgb(0xa0a0a0)).truncate().child(
                if !self.allowed { "Unsupported link target" }
                else if cfg!(target_os = "macos") { "Cmd-click to open link" }
                else { "Ctrl-click to open link" }))
            .child(div().text_sm().truncate().child(self.target.clone()))
    }
}

fn openable_uri(uri: &str) -> bool {
    uri.split_once("://").is_some_and(|(scheme, rest)| {
        (scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https"))
            && !rest.is_empty() && !rest.starts_with('/')
    }) && !uri.chars().any(|c| c.is_control() || c.is_whitespace())
}
fn color(c: Color) -> Hsla {
    rgb((u32::from(c.0) << 16) | (u32::from(c.1) << 8) | u32::from(c.2)).into()
}

enum TerminalSeed {
    Session(Session),
    Ready(PreparedTerminal, CellMetrics, SharedString),
}

struct Workspace {
    spawning_split: Option<Task<()>>,
    split_launch_canceled: Option<Arc<AtomicBool>>,
    pane_subscriptions: std::collections::HashMap<EntityId, Vec<Subscription>>,
    zoomed: Option<EntityId>,
    split_drag: Option<split_view::SplitDrag>,
    panes: Vec<Entity<PaneView>>,
    layout: Option<PaneTree<Entity<PaneView>>>,
    error: Option<String>,
    benchmark: bool,
}
struct WorkspaceEmpty;
impl Drop for Workspace {
    fn drop(&mut self) {
        if let Some(canceled) = &self.split_launch_canceled { canceled.store(true, Ordering::Relaxed); }
    }
}
impl EventEmitter<WorkspaceEmpty> for Workspace {}
struct WorkspaceMetadataChanged;
impl EventEmitter<WorkspaceMetadataChanged> for Workspace {}
impl Workspace {
    fn absorb_workspace(&mut self, source: &Entity<Self>, rebind: bool, window: &mut Window, cx: &mut Context<Self>) -> Result<()> {
        let other = source.read(cx);
        anyhow::ensure!(source.entity_id() != cx.entity_id(), "Cannot merge a workspace into itself");
        anyhow::ensure!(!self.benchmark && !other.benchmark && self.spawning_split.is_none() && other.spawning_split.is_none()
            && self.split_drag.is_none() && other.split_drag.is_none(), "A workspace is busy");
        let destination_tree = self.layout.as_ref().ok_or_else(|| anyhow::anyhow!("Destination has no layout"))?;
        let source_tree = other.layout.as_ref().ok_or_else(|| anyhow::anyhow!("Source has no layout"))?;
        anyhow::ensure!(destination_tree.can_merge(source_tree), "Merged layout would exceed the split nesting limit");
        // Validation is complete. Both entities are owned by this UI thread;
        // no fallible operation or event-loop yield occurs during ownership transfer.
        let (tree, panes) = source.update(cx, |source, cx| {
            source.pane_subscriptions.clear();
            source.zoomed = None;
            source.split_drag = None;
            let tree = source.layout.take().expect("validated source layout");
            let panes = std::mem::take(&mut source.panes);
            cx.notify();
            (tree, panes)
        });
        self.layout = Some(PaneTree::Split {
            axis: SplitAxis::Columns,
            children: vec![self.layout.take().expect("validated destination layout"), tree],
            weights: vec![1., 1.],
        });
        for pane in &panes {
            let owner = cx.entity_id();
            pane.update(cx, |pane, cx| {
                pane.set_workspace_owner(owner, cx);
                if rebind { pane.reset_window_editors(); }
            });
            if rebind {
                for terminal in pane.read(cx).terminals.clone() {
                    terminal.update(cx, |terminal, cx| terminal.rebind_window(window, cx));
                }
            }
            self.observe_pane(pane, window, cx);
        }
        self.panes.extend(panes);
        self.zoomed = None;
        cx.emit(WorkspaceMetadataChanged);
        cx.notify();
        Ok(())
    }
    fn release_render_caches(&mut self, cx: &mut Context<Self>) {
        for pane in &self.panes {
            pane.update(cx, |pane, cx| pane.release_render_caches(cx));
        }
    }
    fn needs_attention(&self, cx: &App) -> bool {
        self.panes
            .iter()
            .any(|pane| pane.read(cx).needs_attention(cx))
    }
    fn observe_pane(
        &mut self,
        view: &Entity<PaneView>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let metadata = cx.subscribe(view, |_, _, _: &pane_view::PaneMetadataChanged, cx| {
            cx.emit(WorkspaceMetadataChanged);
        });
        let movement = cx.subscribe_in(
            view,
            window,
            |workspace, target, event: &pane_view::MoveTab, window, cx| {
                workspace.move_tab(target, event, window, cx);
            },
        );
        let empty = cx.subscribe_in(
            view,
            window,
            |workspace, pane, _: &PaneEmpty, window, cx| {
                workspace.close(pane, window, cx);
            },
        );
        self.pane_subscriptions
            .insert(view.entity_id(), vec![metadata, movement, empty]);
    }
    fn rebind_window(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.split_drag = None;
        if let Some(canceled) = self.split_launch_canceled.take() { canceled.store(true, Ordering::Relaxed); }
        self.spawning_split = None;
        self.pane_subscriptions.clear();
        for pane in self.panes.clone() {
            pane.update(cx, |pane, _| pane.reset_window_editors());
            for terminal in pane.read(cx).terminals.clone() {
                terminal.update(cx, |terminal, cx| terminal.rebind_window(window, cx));
            }
            self.observe_pane(&pane, window, cx);
        }
        cx.notify();
    }
    fn toggle_zoom(&mut self, window: &Window, cx: &mut Context<Self>) {
        self.split_drag = None;
        self.zoomed = if self.zoomed.is_some() {
            None
        } else if self.panes.len() > 1 {
            self.focused_index(window, cx)
                .map(|index| self.panes[index].entity_id())
        } else {
            None
        };
        cx.notify();
    }
    fn resize_pane(&mut self, direction: layout::Direction, window: &Window, cx: &mut Context<Self>) {
        if self.zoomed.is_some() || self.spawning_split.is_some() || cx.global::<Shutdowns>().quitting { return; }
        let Some(index) = self.focused_index(window, cx) else { return; };
        if self.layout.as_mut().is_some_and(|tree| tree.resize_direction(&self.panes[index], direction)) {
            self.split_drag = None;
            cx.notify();
        }
    }
    fn equalize_panes(&mut self, cx: &mut Context<Self>) {
        if let Some(layout) = &mut self.layout {
            layout.equalize();
            self.split_drag = None;
            self.zoomed = None;
            cx.notify();
        }
    }
    fn join_pane_next(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.panes.len() < 2 || cx.global::<Shutdowns>().quitting {
            return;
        }
        let Some(index) = self.focused_index(window, cx) else {
            return;
        };
        self.join_panes(index, (index + 1) % self.panes.len(), window, cx);
    }
    fn join_pane_direction(
        &mut self,
        direction: layout::Direction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if cx.global::<Shutdowns>().quitting { return; }
        let Some(current) = self.focused_index(window, cx) else { return; };
        if let Some(next) = self.directional_neighbor(current, direction) {
            self.join_panes(current, next, window, cx);
        }
    }
    fn join_panes(&mut self, source: usize, target: usize, window: &mut Window, cx: &mut Context<Self>) {
        if cx.global::<Shutdowns>().quitting || source == target { return; }
        let Some(source) = self.panes.get(source).cloned() else { return; };
        let Some(target) = self.panes.get(target).cloned() else { return; };
        if self.zoomed.is_some() || self.spawning_split.is_some() || self.split_drag.is_some() {
            self.error = Some("Finish resizing or splitting, and unzoom before joining panes.".into());
            cx.notify();
            return;
        }
        if source.read(cx).automation_busy(cx) || target.read(cx).automation_busy(cx) {
            self.error = Some("Finish pending tab creation and close pane editors or overlays before joining.".into());
            cx.notify();
            return;
        }
        self.error = None;
        let (tabs, active) = source.update(cx, |pane, cx| pane.take_all_tabs(cx));
        target.update(cx, |pane, cx| pane.append_tabs(tabs, active, window, cx));
        self.remove_pane(&source, cx);
    }
    fn break_tab(&mut self, axis: SplitAxis, before: bool, window: &mut Window, cx: &mut Context<Self>) {
        if cx.global::<Shutdowns>().quitting {
            return;
        }
        let Some(index) = self.focused_index(window, cx) else {
            return;
        };
        let source = self.panes[index].clone();
        if !self.tab_transfer_ready(&source, None, cx) { return; }
        let Some((id, tab)) = source.read(cx).active_tab_for_split() else {
            return;
        };
        let Some(tree) = &mut self.layout else {
            return;
        };
        if !tree.can_split(&source) {
            self.error = Some(format!("Cannot split this pane: maximum nesting is {}", layout::MAX_SPLIT_DEPTH));
            cx.notify();
            return;
        }
        let workspace = cx.entity_id();
        let target = cx.new(|cx| PaneView::from_transfer(tab, workspace, cx));
        // Remove the original tab only after the tree accepts its destination.
        // Both handles refer to the existing terminal; no PTY is created.
        if !tree.split_positioned(&source, target.clone(), axis, before) {
            return;
        }
        self.error = None;
        source.update(cx, |pane, cx| pane.take_tab(id, cx));
        self.panes.insert(index + usize::from(!before), target.clone());
        self.observe_pane(&target, window, cx);
        self.split_drag = None;
        self.zoomed = None;
        target.read(cx).focus(window, cx);
        cx.notify();
    }
    fn swap_pane_next(&mut self, window: &Window, cx: &mut Context<Self>) {
        if self.panes.len() < 2 {
            return;
        }
        let Some(current) = self.focused_index(window, cx) else {
            return;
        };
        let next = (current + 1) % self.panes.len();
        self.swap_panes(current, next, cx);
    }
    fn swap_pane_direction(&mut self, direction: layout::Direction, window: &Window, cx: &mut Context<Self>) {
        if self.zoomed.is_some() { return; }
        let Some(current) = self.focused_index(window, cx) else { return; };
        if let Some(next) = self.directional_neighbor(current, direction) {
            self.swap_panes(current, next, cx);
        }
    }
    fn swap_panes(&mut self, current: usize, next: usize, cx: &mut Context<Self>) -> bool {
        let Some(tree) = &mut self.layout else {
            return false;
        };
        if tree.swap(&self.panes[current], &self.panes[next]) {
            // Keep the pane vector in tree traversal order for save/restore and
            // keyboard cycling. The focused entity and its sessions stay alive.
            self.panes.swap(current, next);
            self.split_drag = None;
            self.zoomed = None;
            cx.notify();
            return true;
        }
        false
    }
    fn snapshot_tree(tree: &PaneTree<Entity<PaneView>>, cx: &App) -> snapshot::Node {
        match tree {
            PaneTree::Pane(pane) => pane.read(cx).snapshot(cx),
            PaneTree::Split {
                axis,
                weights,
                children,
            } => snapshot::Node::Split {
                axis: *axis,
                weights: weights.clone(),
                children: children
                    .iter()
                    .map(|child| Self::snapshot_tree(child, cx))
                    .collect(),
            },
        }
    }
    fn tab_transfer_ready(&mut self, source: &Entity<PaneView>, target: Option<&Entity<PaneView>>, cx: &mut Context<Self>) -> bool {
        if self.spawning_split.is_some() || self.split_drag.is_some() {
            self.error = Some("Finish the pending split or divider resize before moving tabs.".into());
        } else if source.read(cx).automation_busy(cx) || target.is_some_and(|target| target.read(cx).automation_busy(cx)) {
            self.error = Some("Finish pending tab creation and close pane editors or overlays before moving tabs.".into());
        } else {
            return true;
        }
        cx.notify();
        false
    }
    fn move_tab(
        &mut self,
        target: &Entity<PaneView>,
        event: &pane_view::MoveTab,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if cx.global::<Shutdowns>().quitting
            || target.entity_id() == event.source
            || !self.panes.contains(target)
        {
            return;
        }
        let Some(source) = self
            .panes
            .iter()
            .find(|pane| pane.entity_id() == event.source)
            .cloned()
        else {
            return;
        };
        if !self.tab_transfer_ready(&source, Some(target), cx) { return; }
        // Resolve before detaching anything: a stale explicit target is not an
        // append request. There is no await between validation and insertion.
        let destination = target.read(cx);
        let insertion = match event.target_tab {
            Some(id) => {
                let Some(index) = destination.terminals.iter().position(|tab| tab.entity_id() == id) else { return; };
                index
            }
            None => destination.terminals.len(),
        };
        let Some(tab) = source.update(cx, |pane, cx| pane.take_tab(event.terminal, cx)) else {
            return;
        };
        if source.read(cx).terminals.is_empty() {
            self.remove_pane(&source, cx);
        }
        target.update(cx, |pane, cx| {
            pane.insert_tab_at(tab, insertion, window, cx)
        });
        self.error = None;
        cx.notify();
    }
    fn dump_metrics(&mut self, window: &Window, cx: &mut Context<Self>) {
        if cx.global::<diagnostics::ExportState>().0 {
            return;
        }
        let Some(path) = diagnostics::destination() else {
            return;
        };
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let mut output = String::new();
        for entity in self.panes.iter().flat_map(|pane| &pane.read(cx).terminals) {
            let view = entity.read(cx);
            if let Some(metrics) = &view.metrics {
                output.push_str(&metrics.snapshot(
                    timestamp,
                    view.session.output_bytes(),
                    view.focus.is_focused(window),
                    window.is_window_active(),
                    &view.frame,
                    view.session.try_memory_usage().ok().flatten(),
                ));
            }
        }
        cx.global_mut::<diagnostics::ExportState>().0 = true;
        let executor = cx.background_executor().clone();
        cx.spawn(async move |entity, cx| {
            let result = executor
                .spawn(async move {
                    use std::io::Write;
                    std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(path)
                        .and_then(|mut file| file.write_all(output.as_bytes()))
                })
                .await;
            let _ = cx.update(|cx| {
                cx.global_mut::<diagnostics::ExportState>().0 = false;
                if let Err(error) = result {
                    let _ = entity.update(cx, |view, cx| {
                        view.error = Some(format!("Write render metrics: {error}"));
                        cx.notify();
                    });
                }
            });
        })
        .detach();
    }
    fn focused_index(&self, window: &Window, cx: &App) -> Option<usize> {
        self.panes
            .iter()
            .position(|view| view.read(cx).is_focused(window, cx))
    }
    fn select_pane_number(&mut self, number: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(index) = number.checked_sub(1) else {
            return;
        };
        let Some(pane) = self.panes.get(index) else {
            return;
        };
        if self.zoomed.is_some() {
            self.zoomed = Some(pane.entity_id());
        }
        pane.read(cx).focus(window, cx);
        cx.notify();
    }
    fn focus_direction(
        &mut self,
        direction: layout::Direction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(current) = self.focused_index(window, cx) else { return; };
        if let Some(next) = self.directional_neighbor(current, direction) {
            if self.zoomed.is_some() {
                self.zoomed = Some(self.panes[next].entity_id());
            }
            self.panes[next].read(cx).focus(window, cx);
            cx.notify();
        }
    }
    fn directional_neighbor(&self, current: usize, direction: layout::Direction) -> Option<usize> {
        let next = self.layout.as_ref()?.neighbor(self.panes.get(current)?, direction)?;
        self.panes.iter().position(|pane| *pane == next)
    }
    fn cycle(&mut self, direction: isize, window: &mut Window, cx: &mut Context<Self>) {
        if self.panes.is_empty() {
            return;
        }
        let current = self.focused_index(window, cx).unwrap_or(0);
        let next = (current as isize + direction).rem_euclid(self.panes.len() as isize) as usize;
        if self.zoomed.is_some() {
            self.zoomed = Some(self.panes[next].entity_id());
        }
        self.panes[next].read(cx).focus(window, cx);
        cx.notify();
    }
    fn close(&mut self, pane: &Entity<PaneView>, window: &mut Window, cx: &mut Context<Self>) {
        // The emptied pane has already dropped its tab. Any other focused pane
        // is still valid and should keep its focus when this split disappears.
        let preserve_focus = self.panes.iter().any(|candidate| {
            candidate != pane && candidate.read(cx).is_focused(window, cx)
        });
        let Some(index) = self.remove_pane(pane, cx) else {
            return;
        };
        if !preserve_focus && let Some(survivor) = self
            .panes
            .get(index.min(self.panes.len().saturating_sub(1)))
        {
            survivor.read(cx).focus(window, cx);
        }
    }
    /// Remove layout ownership without changing input focus. Transfers must not
    /// acknowledge unrelated unread terminals on the way to their destination.
    fn remove_pane(&mut self, pane: &Entity<PaneView>, cx: &mut Context<Self>) -> Option<usize> {
        let index = self.panes.iter().position(|candidate| candidate == pane)?;
        self.split_drag = None;
        let removed = self.panes.remove(index);
        self.pane_subscriptions.remove(&removed.entity_id());
        self.zoomed = None;
        self.layout = self.layout.take().and_then(|tree| tree.remove(&removed));
        if self.panes.is_empty() {
            cx.emit(WorkspaceEmpty);
        }
        cx.notify();
        Some(index)
    }
    fn cancel_split(&mut self, cx: &mut Context<Self>) {
        if let Some(canceled) = &self.split_launch_canceled {
            canceled.store(true, Ordering::Relaxed);
            // Keep the launch slot occupied until completion to bound retries.
            cx.notify();
        }
    }
    fn add(&mut self, axis: SplitAxis, window: &mut Window, cx: &mut Context<Self>) {
        self.add_positioned(axis, false, window, cx);
    }
    fn add_positioned(&mut self, axis: SplitAxis, before: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.benchmark {
            self.add_prepared(axis, None, true, None, window, cx);
            return;
        }
        if self.spawning_split.is_some() || cx.global::<Shutdowns>().quitting { return; }
        let Some(index) = self.focused_index(window, cx).or_else(|| self.panes.len().checked_sub(1)) else { return; };
        let pane = &self.panes[index];
        if self.layout.as_ref().is_some_and(|tree| !tree.can_split(pane)) {
            self.error = Some(format!("Cannot split this pane: maximum nesting is {}", layout::MAX_SPLIT_DEPTH));
            cx.notify();
            return;
        }
        let destination = pane.read(cx).live_ssh_destination(cx);
        let label = destination.as_ref().map(|destination| format!("SSH · {destination}"));
        let target = pane.entity_id();
        let cells = pane.read(cx).active_cell_metrics(cx);
        let family = pane.read(cx).active_font_family(cx);
        let previous_default_size = cx.global::<Preferences>().font_size;
        let previous_default_family = cx.global::<Preferences>().font_family.clone();
        let directory = pane.read(cx).cached_directory(cx)
            .or_else(|| cx.global::<InitialDirectory>().0.clone());
        let scrollback = cx.global::<Preferences>().scrollback_bytes();
        let colors = cx.global::<Preferences>().terminal_colors();
        let palette = cx.global::<Preferences>().effective_palette();
        let origin = window.focused(cx);
        let executor = cx.background_executor().clone();
        self.error = None;
        let canceled = Arc::new(AtomicBool::new(false));
        self.split_launch_canceled = Some(canceled.clone());
        let launch_canceled = canceled.clone();
        self.spawning_split = Some(cx.spawn_in(window, async move |entity, cx| {
            let mut result = executor.spawn(async move {
                pane_view::prepare_terminal_launch(&launch_canceled, || {
                    ssh::launch_sibling(destination.as_deref(), directory.as_deref(), scrollback)
                }, colors, palette)
            }).await;
            loop {
                if canceled.load(Ordering::Relaxed) { break; }
                let refresh = cx.update(|_, cx| {
                    let prepared = result.as_ref().ok()?;
                    let settings = cx.global::<Preferences>();
                    let current = (settings.terminal_colors(), settings.effective_palette());
                    ((prepared.colors, prepared.palette) != current).then_some(current)
                }).ok().flatten();
                let Some((colors, palette)) = refresh else { break; };
                let session = result.expect("successful preparation checked above").session;
                result = executor.spawn(async move { TerminalView::prepare(session, colors, palette) }).await;
            }
            let _ = cx.update(|window, cx| entity.update(cx, |this, cx| {
                this.spawning_split = None;
                this.split_launch_canceled = None;
                cx.notify();
                if canceled.load(Ordering::Relaxed) || cx.global::<Shutdowns>().quitting || !this.panes.iter().any(|pane| pane.entity_id() == target) { return; }
                match result {
                    Ok(prepared) => {
                        let focus = origin.as_ref().is_some_and(|focus| focus.is_focused(window));
                        let (size, family) = preferences::updated_font(cells.font_size, &family,
                            previous_default_size, &previous_default_family, cx.global::<Preferences>());
                        let cells = if size == cells.font_size { cells } else { CellMetrics::new(size) };
                        this.add_seed_at(Some(target), axis, before, None, focus, Some((TerminalSeed::Ready(prepared, cells, family.into()), label)), window, cx);
                    }
                    Err(error) => this.error = Some(format!("Start split terminal: {error}")),
                }
            }));
        }));
        cx.notify();
    }

    fn add_prepared(&mut self, axis: SplitAxis, directory: Option<std::path::PathBuf>, focus: bool, prepared: Option<(Session, Option<String>)>, window: &mut Window, cx: &mut Context<Self>) {
        self.add_prepared_at(None, axis, false, directory, focus, prepared, window, cx);
    }
    fn add_prepared_at(&mut self, target: Option<EntityId>, axis: SplitAxis, before: bool, directory: Option<std::path::PathBuf>, focus: bool, prepared: Option<(Session, Option<String>)>, window: &mut Window, cx: &mut Context<Self>) -> Option<EntityId> {
        self.add_seed_at(target, axis, before, directory, focus, prepared.map(|(session, label)| (TerminalSeed::Session(session), label)), window, cx)
    }
    fn add_seed_at(&mut self, target: Option<EntityId>, axis: SplitAxis, before: bool, directory: Option<std::path::PathBuf>, focus: bool, prepared: Option<(TerminalSeed, Option<String>)>, window: &mut Window, cx: &mut Context<Self>) -> Option<EntityId> {
        if cx.global::<Shutdowns>().quitting { return None; }
        let anchor = if let Some(target) = target {
            Some(self.panes.iter().position(|pane| pane.entity_id() == target)?)
        } else { self.focused_index(window, cx) };
        let split_index = anchor.unwrap_or(self.panes.len().saturating_sub(1));
        if !self.benchmark && self.layout.as_ref().is_some_and(|tree| {
            self.panes.get(split_index).is_none_or(|pane| !tree.can_split(pane))
        }) {
            self.error = Some(format!("Cannot split this pane: maximum nesting is {}", layout::MAX_SPLIT_DEPTH));
            cx.notify();
            return None;
        }
        let mut created = None;
        let directory = directory.or_else(|| anchor
            .and_then(|index| self.panes[index].read(cx).cached_directory(cx)));
        let mut cells = if self.benchmark {
            CellMetrics::new(14.)
        } else {
            anchor
                .map(|index| self.panes[index].read(cx).active_cell_metrics(cx))
                .unwrap_or_else(|| CellMetrics::new(cx.global::<Preferences>().font_size))
        };
        let (prepared, command_label) = match prepared.or_else(|| cx.global_mut::<InitialCommand>().0.take().map(|(session, label)| (TerminalSeed::Session(session), label))) {
            Some((session, label)) => (Some(session), label),
            None => (None, None),
        };
        let mut family = if self.benchmark {
            FONT_FAMILY.into()
        } else {
            anchor
                .map(|index| self.panes[index].read(cx).active_font_family(cx))
                .unwrap_or_else(|| cx.global::<Preferences>().font_family.clone().into())
        };
        let prepared = match prepared {
            Some(TerminalSeed::Ready(prepared, inherited_cells, inherited_family)) => {
                cells = inherited_cells;
                family = inherited_family;
                Ok(prepared)
            }
            seed => {
                let session = if let Some(TerminalSeed::Session(session)) = seed {
                    Ok(session)
                } else if self.benchmark {
                    Session::benchmark_shell(80, 24)
                } else {
                    Session::shell_in_with_scrollback(
                        80,
                        24,
                        directory
                            .as_deref()
                            .or(cx.global::<InitialDirectory>().0.as_deref()),
                        cx.global::<Preferences>().scrollback_bytes(),
                    )
                };
                let settings = cx.global::<Preferences>();
                session.and_then(|session| TerminalView::prepare(session, settings.terminal_colors(), settings.effective_palette()))
            }
        };
        match prepared {
            Ok(prepared) => {
                let focused = split_index;
                self.error = None;
                self.split_drag = None;
                self.zoomed = None;
                let terminal = cx.new(|cx| {
                    let mut terminal = TerminalView::from_prepared(prepared, window, cx);
                    terminal.cells = cells;
                    terminal.font_family = family;
                    terminal
                });
                let workspace_id = cx.entity_id();
                let view = cx.new(|cx| {
                    PaneView::new(terminal, command_label, self.benchmark, workspace_id, cx)
                });
                self.observe_pane(&view, window, cx);
                if self.benchmark {
                    // Preserve the existing equal-width benchmark fixture.
                    self.panes.push(view.clone());
                    self.layout = Some(PaneTree::Split {
                        axis: SplitAxis::Columns,
                        children: self.panes.iter().cloned().map(PaneTree::Pane).collect(),
                        weights: vec![1.; self.panes.len()],
                    });
                } else if let Some(tree) = &mut self.layout {
                    tree.split_positioned(&self.panes[focused], view.clone(), axis, before);
                    self.panes.insert(focused + usize::from(!before), view.clone());
                } else {
                    self.layout = Some(PaneTree::Pane(view.clone()));
                    self.panes.push(view.clone());
                }
                created = Some(view.entity_id());
                if focus { view.read(cx).focus(window, cx); }
            }
            Err(error) => self.error = Some(error.to_string()),
        }
        cx.notify();
        created
    }
}
impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(zoomed) = self.zoomed {
            for pane in &self.panes {
                if pane.entity_id() != zoomed {
                    pane.update(cx, |pane, cx| pane.release_render_caches(cx));
                }
            }
        }
        div()
            .size_full()
            .relative()
            .child(split_view::drag_capture(cx))
            .text_color(rgb(0xdcdfe6))
            .flex()
            .flex_col()
            .on_action(cx.listener(|this, _: &CancelSplit, _, cx| this.cancel_split(cx)))
            .on_action(
                cx.listener(|view, _: &TogglePaneZoom, window, cx| view.toggle_zoom(window, cx)),
            )
            .on_action(cx.listener(|view, _: &EqualizePanes, _, cx| view.equalize_panes(cx)))
            .on_action(cx.listener(|view, _: &ResizePaneLeft, window, cx| view.resize_pane(layout::Direction::Left, window, cx)))
            .on_action(cx.listener(|view, _: &ResizePaneRight, window, cx| view.resize_pane(layout::Direction::Right, window, cx)))
            .on_action(cx.listener(|view, _: &ResizePaneUp, window, cx| view.resize_pane(layout::Direction::Up, window, cx)))
            .on_action(cx.listener(|view, _: &ResizePaneDown, window, cx| view.resize_pane(layout::Direction::Down, window, cx)))
            .on_action(
                cx.listener(|view, _: &SwapPaneNext, window, cx| view.swap_pane_next(window, cx)),
            )
            .on_action(cx.listener(|view, _: &SwapPaneLeft, window, cx| {
                view.swap_pane_direction(layout::Direction::Left, window, cx)
            }))
            .on_action(cx.listener(|view, _: &SwapPaneRight, window, cx| {
                view.swap_pane_direction(layout::Direction::Right, window, cx)
            }))
            .on_action(cx.listener(|view, _: &SwapPaneUp, window, cx| {
                view.swap_pane_direction(layout::Direction::Up, window, cx)
            }))
            .on_action(cx.listener(|view, _: &SwapPaneDown, window, cx| {
                view.swap_pane_direction(layout::Direction::Down, window, cx)
            }))
            .on_action(
                cx.listener(|view, _: &JoinPaneNext, window, cx| view.join_pane_next(window, cx)),
            )
            .on_action(cx.listener(|view, _: &JoinPaneLeft, window, cx| {
                view.join_pane_direction(layout::Direction::Left, window, cx)
            }))
            .on_action(cx.listener(|view, _: &JoinPaneRight, window, cx| {
                view.join_pane_direction(layout::Direction::Right, window, cx)
            }))
            .on_action(cx.listener(|view, _: &JoinPaneUp, window, cx| {
                view.join_pane_direction(layout::Direction::Up, window, cx)
            }))
            .on_action(cx.listener(|view, _: &JoinPaneDown, window, cx| {
                view.join_pane_direction(layout::Direction::Down, window, cx)
            }))
            .on_action(cx.listener(|view, _: &BreakTabLeft, window, cx| {
                view.break_tab(SplitAxis::Columns, true, window, cx)
            }))
            .on_action(cx.listener(|view, _: &BreakTabAbove, window, cx| {
                view.break_tab(SplitAxis::Rows, true, window, cx)
            }))
            .on_action(cx.listener(|view, _: &BreakTabRight, window, cx| {
                view.break_tab(SplitAxis::Columns, false, window, cx)
            }))
            .on_action(cx.listener(|view, _: &BreakTabBelow, window, cx| {
                view.break_tab(SplitAxis::Rows, false, window, cx)
            }))
            .on_action(cx.listener(|view, _: &SplitLeft, window, cx| view.add_positioned(SplitAxis::Columns, true, window, cx)))
            .on_action(cx.listener(|view, _: &SplitAbove, window, cx| view.add_positioned(SplitAxis::Rows, true, window, cx)))
            .on_action(
                cx.listener(|view, _: &Split, window, cx| view.add(SplitAxis::Columns, window, cx)),
            )
            .on_action(
                cx.listener(|view, _: &SplitBelow, window, cx| {
                    view.add(SplitAxis::Rows, window, cx)
                }),
            )
            .on_action(
                cx.listener(|view, _: &DumpRenderMetrics, window, cx| {
                    view.dump_metrics(window, cx)
                }),
            )
            .on_action(cx.listener(|view, _: &NextPane, window, cx| view.cycle(1, window, cx)))
            .on_action(cx.listener(|view, _: &FocusLeft, window, cx| {
                view.focus_direction(layout::Direction::Left, window, cx)
            }))
            .on_action(cx.listener(|view, _: &FocusRight, window, cx| {
                view.focus_direction(layout::Direction::Right, window, cx)
            }))
            .on_action(cx.listener(|view, _: &FocusUp, window, cx| {
                view.focus_direction(layout::Direction::Up, window, cx)
            }))
            .on_action(cx.listener(|view, action: &SelectPaneNumber, window, cx| {
                view.select_pane_number(action.number, window, cx);
            }))
            .on_action(cx.listener(|view, _: &FocusDown, window, cx| {
                view.focus_direction(layout::Direction::Down, window, cx)
            }))
            .on_action(cx.listener(|view, _: &PreviousPane, window, cx| view.cycle(-1, window, cx)))
            .when(self.zoomed.is_some(), |root| {
                let focus = self.panes.iter().find(|pane| Some(pane.entity_id()) == self.zoomed)
                    .and_then(|pane| {
                        let pane = pane.read(cx);
                        pane.terminals.iter().find(|terminal| Some(terminal.entity_id()) == pane.active_terminal_id())
                            .map(|terminal| terminal.read(cx).focus.clone())
                    });
                let shortcut = focus.as_ref().and_then(|focus| shortcuts::hint(&TogglePaneZoom, focus, window));
                let hidden = self.panes.len().saturating_sub(1);
                let status = format!("Pane zoomed · {hidden} {} hidden", if hidden == 1 { "pane" } else { "panes" });
                let hint: SharedString = match &shortcut {
                    Some(shortcut) => format!("Restore split layout  {shortcut}"),
                    None => "Restore split layout".into(),
                }.into();
                root.child(
                    div()
                        .id("restore-pane-layout")
                        .h(px(28.)).min_w_0()
                        .flex_shrink_0()
                        .px_3()
                        .flex().gap_2()
                        .items_center()
                        .text_xs()
                        .bg(rgb(0x27344a))
                        .cursor_pointer()
                        .hover(|style| style.bg(rgb(0x354660)))
                        .tooltip(move |_, cx| cx.new(|_| pane_view::ControlHint(hint.clone())).into())
                        .on_click(cx.listener(|view, event: &ClickEvent, window, cx| {
                            if event.standard_click() { view.toggle_zoom(window, cx); }
                        }))
                        .child(icons::icon("split-right"))
                        .child(div().flex_1().min_w_0().truncate().child(status))
                        .child(div().flex_shrink_0().child("Restore layout"))
                        .when_some(shortcut, |row, shortcut| row.child(div().max_w(relative(0.3)).truncate().text_color(rgb(0xa0b6d6)).child(shortcut))),
                )
            })
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .when_some(self.layout.as_ref(), |div, tree| {
                        if let Some(pane) = self
                            .panes
                            .iter()
                            .find(|pane| Some(pane.entity_id()) == self.zoomed)
                        {
                            return div.child(
                                gpui::div()
                                    .size_full()
                                    .min_w_0()
                                    .min_h_0()
                                    .overflow_hidden()
                                    .child(pane.clone()),
                            );
                        }
                        div.child(split_view::render_panes(
                            tree,
                            Vec::new(),
                            self.benchmark,
                            cx,
                        ))
                    }),
            )
            .when_some(self.error.clone(), |div, error| div.child(div_error(error,
                cx.listener(|this, event: &ClickEvent, _, cx| {
                    if event.standard_click() { this.error = None; cx.stop_propagation(); cx.notify(); }
                }))))
            .when(self.spawning_split.is_some(), |root| root.child(
                // Out of flow: starting/canceling must not resize every live PTY.
                div().id("split-launch-status").absolute().top(px(36.)).right(px(8.))
                    .w(px(250.)).max_w_full().h(px(32.)).px_2().flex().items_center().gap_2()
                    .occlude().rounded_md().shadow_md().border_1().border_color(rgb(0x465063))
                    .text_xs().text_color(rgb(0xc6cdd8)).bg(rgb(0x292929))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_mouse_down(MouseButton::Middle, |_, _, cx| cx.stop_propagation())
                    .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
                    .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
                    .child(div().flex_1().min_w_0().truncate().child(
                        if self.split_launch_canceled.as_ref().is_some_and(|canceled| canceled.load(Ordering::Relaxed)) { "Canceling split terminal…" } else { "Starting split terminal…" }
                    ))
                    .child(pane_view::pane_control("cancel-split", "Cancel Split Terminal")
                        .on_click(cx.listener(|this, event: &ClickEvent, _, cx| {
                            if event.standard_click() { this.cancel_split(cx); }
                            cx.stop_propagation();
                        })).child(icons::icon("close")))
            ))
    }
}
fn open_window(benchmark_panes: Option<usize>, cx: &mut App) -> anyhow::Result<()> {
    if cx.global::<Shutdowns>().quitting {
        return Ok(());
    }
    open_window_with_activation(benchmark_panes, true, cx)?;
    Ok(())
}

fn open_window_with_activation(benchmark_panes: Option<usize>, activate: bool, cx: &mut App) -> anyhow::Result<WindowHandle<window_view::WorkspaceWindow>> {
    anyhow::ensure!(!cx.global::<Shutdowns>().quitting, "application is quitting");
    let bounds = Bounds::centered(None, size(px(1000.), px(680.)), cx);
    let handle = cx.open_window(
        WindowOptions {
                        titlebar: workspace_titlebar(),
                    app_id: Some("rmux".into()),
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            focus: activate,
            ..Default::default()
        },
        move |window, cx| {
            window.set_window_title("rmux");
            cx.new(|cx| window_view::WorkspaceWindow::new(benchmark_panes, window, cx))
        },
    )?;
    if activate { cx.activate(true); }
    Ok(handle)
}

// Keep native window buttons while sharing their row with app chrome on macOS.
fn workspace_titlebar() -> Option<TitlebarOptions> {
    Some(TitlebarOptions {
        appears_transparent: cfg!(target_os = "macos"),
        traffic_light_position: cfg!(target_os = "macos").then(|| point(px(10.), px(10.))),
        ..Default::default()
    })
}

fn main() {
    let mut initial_directory = None;
    let mut initial_environment = Vec::new();
    let mut initial_command_label = None;
    let mut initial_command = Vec::new();
    let mut initial_layout = None;
    let mut initial_session = None;
    let command = startup::parse(std::env::args_os().skip(1)).unwrap_or_else(|error| {
        eprintln!("rmux: {error}");
        std::process::exit(2);
    });
    let live_rpc_stdio = match &command {
        startup::Command::Launch { live_rpc_stdio, .. } | startup::Command::Layout { live_rpc_stdio, .. } | startup::Command::Session { live_rpc_stdio, .. } | startup::Command::Preset { live_rpc_stdio, .. } | startup::Command::CustomCommand { live_rpc_stdio, .. } => *live_rpc_stdio,
        _ => false,
    };
    let preferences = if matches!(
        &command,
        startup::Command::Help | startup::Command::Version | startup::Command::CheckLayout(_) | startup::Command::CheckSession(_) | startup::Command::RpcStdio
    ) {
        Preferences::default()
    } else {
        Preferences::load().unwrap_or_else(|error| {
            eprintln!("rmux: {error:#}");
            std::process::exit(2);
        })
    };
    // Freeze recovery policy for this launch. Explicit commands/layouts and
    // benchmarks neither restore nor overwrite the normal recovery file.
    let recovery_path = if matches!(&command, startup::Command::Launch { directory: None, argv, .. } if argv.is_empty())
        && std::env::var_os("RMUX_BENCH_PANES").is_none() {
        preferences.session_file.clone()
    } else { None };
    let mut recovery_notice = None;
    let mut recovery_lease = recovery_path.and_then(|path| match recovery::Lease::acquire(path.clone()) {
        Ok(lease) => Some(lease),
        Err(error) => {
            eprintln!("rmux: recovery disabled for {}: {error:#}", path.display());
            recovery_notice = Some(format!("Session recovery is unavailable: {error}. This window will not overwrite the recovery file."));
            None
        }
    });
    match command {
        startup::Command::RpcStdio => {
            if let Err(error) = automation::serve(std::io::stdin().lock(), std::io::stdout().lock()) {
                eprintln!("rmux: RPC stream: {error}");
                std::process::exit(2);
            }
            return;
        }
        startup::Command::CheckConfig => {
            println!("Configuration valid");
            return;
        }
        startup::Command::CheckLayout(path) => {
            match snapshot::WindowSnapshot::load(&path) {
                Ok(_) => {
                    println!("Layout structure valid (directories and executables not checked)")
                }
                Err(error) => {
                    eprintln!("rmux: layout {}: {error:#}", path.display());
                    std::process::exit(2);
                }
            }
            return;
        }
        startup::Command::CheckSession(path) => {
            match snapshot::SessionSnapshot::load(&path) {
                Ok(_) => println!("Session structure valid (directories and executables not checked)"),
                Err(error) => { eprintln!("rmux: session {}: {error:#}", path.display()); std::process::exit(2); }
            }
            return;
        }
        startup::Command::CustomCommand { name, .. } => {
            if std::env::var_os("RMUX_BENCH_PANES").is_some() {
                eprintln!("rmux: --command cannot be combined with RMUX_BENCH_PANES");
                std::process::exit(2);
            }
            let Some(configured) = preferences.commands.iter().find(|command| command.name == name) else {
                eprintln!("rmux: command is not configured: {name}");
                std::process::exit(2);
            };
            initial_command = configured.argv.iter().map(std::ffi::OsString::from).collect();
            initial_environment = configured.env.iter().map(|(key, value)| (key.into(), value.into())).collect();
            initial_directory = configured.cwd.clone().or_else(|| std::env::current_dir().ok());
            initial_command_label = Some(name);
        }
        startup::Command::Preset { name, .. } => {
            if std::env::var_os("RMUX_BENCH_PANES").is_some() {
                eprintln!("rmux: --preset cannot be combined with RMUX_BENCH_PANES");
                std::process::exit(2);
            }
            let Some(preset) = preferences.layout_presets.iter().find(|preset| preset.name == name) else {
                eprintln!("rmux: layout preset is not configured: {name}");
                std::process::exit(2);
            };
            let token = std::sync::atomic::AtomicBool::new(false);
            match restore::PreparedSession::read_import(&preset.path, preset.session, preset.cwd.as_deref(), &token, preferences.scrollback_bytes(), |_, _| {}) {
                Ok(prepared) => initial_session = Some(prepared),
                Err(error) => { eprintln!("rmux: preset {name}: {error:#}"); std::process::exit(2); }
            }
        }
        startup::Command::Session { path, .. } => {
            if std::env::var_os("RMUX_BENCH_PANES").is_some() {
                eprintln!("rmux: --session cannot be combined with RMUX_BENCH_PANES");
                std::process::exit(2);
            }
            let token = std::sync::atomic::AtomicBool::new(false);
            match restore::PreparedSession::read(&path, &token, preferences.scrollback_bytes()) {
                Ok(prepared) => initial_session = Some(prepared),
                Err(error) => { eprintln!("rmux: session {}: {error:#}", path.display()); std::process::exit(2); }
            }
        }
        startup::Command::DumpConfig => {
            println!(
                "{}",
                serde_json::to_string_pretty(&preferences).expect("validated settings serialize")
            );
            return;
        }
        startup::Command::Help => {
            print!("{}", startup::HELP);
            return;
        }
        startup::Command::Version => {
            println!("rmux {}", env!("CARGO_PKG_VERSION"));
            return;
        }
        startup::Command::Launch { directory, argv, .. } => {
            initial_command = argv;
            if !initial_command.is_empty() && std::env::var_os("RMUX_BENCH_PANES").is_some() {
                eprintln!("rmux: a startup command cannot be combined with RMUX_BENCH_PANES");
                std::process::exit(2);
            }
            if let Some(path) = directory {
                if let Err(error) = std::env::set_current_dir(&path) {
                    eprintln!("rmux: working directory {}: {error}", path.display());
                    std::process::exit(2);
                }
                initial_directory =
                    Some(std::env::current_dir().expect("read selected working directory"));
            } else if !initial_command.is_empty() {
                initial_directory = std::env::current_dir().ok();
            }
        }
        startup::Command::Layout { path, .. } => {
            if std::env::var_os("RMUX_BENCH_PANES").is_some() {
                eprintln!("rmux: --layout cannot be combined with RMUX_BENCH_PANES");
                std::process::exit(2);
            }
            let token = std::sync::atomic::AtomicBool::new(false);
            match restore::Prepared::read(&path, &token, preferences.scrollback_bytes()) {
                Ok(prepared) => initial_layout = Some(prepared),
                Err(error) => {
                    eprintln!("rmux: layout {}: {error:#}", path.display());
                    std::process::exit(2);
                }
            }
        }
    }
    if let Some(lease) = &recovery_lease {
        let path = &lease.path;
        if path.exists() {
            let token = std::sync::atomic::AtomicBool::new(false);
            match restore::PreparedSession::read(path, &token, preferences.scrollback_bytes()) {
                Ok(prepared) => initial_session = Some(prepared),
                Err(error) => {
                    eprintln!("rmux: could not recover session {}; opening a fresh window: {error:#}", path.display());
                    recovery_notice = Some(format!("Could not restore the saved session: {error}. The original file is preserved; automatic saving is disabled for this launch. Use Save Session to save this window separately."));
                }
            }
        }
    }
    if recovery_notice.is_some() { recovery_lease = None; }
    // Opt-in only: deterministic shell startup for local performance runs.
    // Normal launches continue to use the user's configured login shell.
    let benchmark_panes = std::env::var("RMUX_BENCH_PANES").ok().map(|value| {
        let panes: usize = value.parse().expect("RMUX_BENCH_PANES must be an integer");
        assert!((1..=32).contains(&panes), "RMUX_BENCH_PANES must be 1..32");
        panes
    });
    let mut prepared_command = if initial_command.is_empty() {
        None
    } else {
        let label = std::path::Path::new(&initial_command[0])
            .file_name()
            .map(|name| {
                name.to_string_lossy()
                    .chars()
                    .filter(|c| !c.is_control())
                    .take(128)
                    .collect::<String>()
            })
            .filter(|name| !name.is_empty());
        match Session::command_in_with_environment(
            &initial_command,
            80,
            24,
            initial_directory.as_deref(),
            preferences.scrollback_bytes(),
            &initial_environment,
        ) {
            Ok(session) => Some((session, initial_command_label.or(label))),
            Err(error) => {
                eprintln!(
                    "rmux: could not start {}: {error:#}",
                    initial_command[0].to_string_lossy()
                );
                std::process::exit(2);
            }
        }
    };
    Application::new().with_assets(icons::Assets).run(move |cx: &mut App| {
        cx.set_global(Shutdowns::default());
        cx.set_global(native_notifications::Dispatcher::default());
        cx.set_global(copy_buffers::Store::default());
        cx.set_global(preferences.confirm_close);
        cx.set_global(preferences.clone());
        recovery::install(recovery_lease.clone(), cx);
        cx.set_global(preferences::ReloadState::default());
        cx.set_global(diagnostics::ExportState::default());
        cx.set_global(PromptGate::default());
        cx.set_global(InitialDirectory(initial_directory.clone()));
        cx.set_global(InitialCommand(prepared_command.take()));
        if diagnostics::destination().is_some() {
            cx.bind_keys([KeyBinding::new("f12", DumpRenderMetrics, None)]);
        }
        if cfg!(target_os = "macos") {
            cx.bind_keys([
                KeyBinding::new("cmd-q", Quit, None),
                KeyBinding::new("cmd-p", ShowCommandPalette, None),
                KeyBinding::new("cmd-shift-f", ShowTerminalSearch, None),
                KeyBinding::new("cmd-ctrl-left", FocusLeft, None),
                KeyBinding::new("cmd-ctrl-right", FocusRight, None),
                KeyBinding::new("cmd-ctrl-up", FocusUp, None),
                KeyBinding::new("cmd-ctrl-down", FocusDown, None),
                KeyBinding::new("cmd-shift-n", NewWindow, None),
                KeyBinding::new("cmd-shift-w", CloseWindow, None),
                KeyBinding::new("cmd-d", Split, None),
                KeyBinding::new("cmd-shift-d", SplitBelow, None),
                KeyBinding::new("cmd-shift-enter", TogglePaneZoom, None),
                KeyBinding::new("cmd-n", NewWorkspace, None),
                KeyBinding::new("ctrl-cmd-g", NewWorkspaceGroup, None),
                KeyBinding::new("cmd-shift-g", GroupSelectedWorkspaces, None),
                KeyBinding::new("cmd-shift-r", RenameWorkspace, None),
                KeyBinding::new("cmd-t", NewTab, Some("Terminal")),
                KeyBinding::new("cmd-shift-t", ShowTabPicker, Some("Terminal")),
                KeyBinding::new("cmd-alt-down", NextWorkspace, None),
                KeyBinding::new("cmd-shift-u", NextUnreadTerminal, None),
                KeyBinding::new("cmd-alt-up", PreviousWorkspace, None),
                KeyBinding::new("cmd-alt-shift-up", MoveWorkspaceUp, None),
                KeyBinding::new("cmd-alt-shift-down", MoveWorkspaceDown, None),
                KeyBinding::new("cmd-b", ToggleSidebar, None),
                KeyBinding::new("cmd-v", Paste, Some("Terminal")),
                KeyBinding::new("cmd-c", CopySelection, Some("Terminal")),
                KeyBinding::new("cmd-a", EditorSelectAll, Some("Terminal")),
                KeyBinding::new("cmd-up", PreviousPrompt, Some("Terminal")),
                KeyBinding::new("cmd-down", NextPrompt, Some("Terminal")),
                KeyBinding::new("cmd-f", Find, Some("Terminal")),
                KeyBinding::new("cmd-f", Find, Some("FindInput")),
                KeyBinding::new("cmd-g", FindOlder, Some("Terminal")),
                KeyBinding::new("cmd-shift-g", FindNewer, Some("Terminal")),
                KeyBinding::new("cmd-g", FindOlder, Some("FindInput")),
                KeyBinding::new("cmd-shift-g", FindNewer, Some("FindInput")),
                KeyBinding::new("cmd-=", IncreaseFontSize, Some("Terminal")),
                KeyBinding::new("cmd--", DecreaseFontSize, Some("Terminal")),
                KeyBinding::new("cmd-0", ResetFontSize, Some("Terminal")),
                KeyBinding::new("cmd-v", Paste, Some("TextInput")),
                KeyBinding::new("cmd-c", CopySelection, Some("TextInput")),
                KeyBinding::new("cmd-c", CopySelection, Some("Notifications")),
                KeyBinding::new("cmd-c", CopySelection, Some("ListeningPorts")),
                KeyBinding::new("cmd-c", CopySelection, Some("CopyMode")),
                KeyBinding::new("cmd-a", EditorSelectAll, Some("CopyMode")),
                KeyBinding::new("cmd-f", Find, Some("CopyMode")),
                KeyBinding::new("cmd-x", EditorCut, Some("TextInput")),
                KeyBinding::new("cmd-z", EditorUndo, Some("TextInput")),
                KeyBinding::new("cmd-shift-z", EditorRedo, Some("TextInput")),
                KeyBinding::new("cmd-a", EditorSelectAll, Some("TextInput")),
                KeyBinding::new("cmd-w", ClosePane, Some("Terminal")),
            ]);
        } else {
            cx.bind_keys([
                KeyBinding::new("ctrl-shift-q", Quit, None),
                KeyBinding::new("ctrl-shift-p", ShowCommandPalette, None),
                KeyBinding::new("ctrl-alt-shift-f", ShowTerminalSearch, None),
                KeyBinding::new("alt-shift-left", FocusLeft, None),
                KeyBinding::new("alt-shift-right", FocusRight, None),
                KeyBinding::new("alt-shift-up", FocusUp, None),
                KeyBinding::new("alt-shift-down", FocusDown, None),
                KeyBinding::new("ctrl-alt-n", NewWindow, None),
                KeyBinding::new("ctrl-alt-w", CloseWindow, None),
                KeyBinding::new("ctrl-shift-d", Split, None),
                KeyBinding::new("ctrl-shift-e", SplitBelow, None),
                KeyBinding::new("ctrl-shift-enter", TogglePaneZoom, None),
                KeyBinding::new("ctrl-shift-n", NewWorkspace, None),
                KeyBinding::new("ctrl-alt-g", NewWorkspaceGroup, None),
                KeyBinding::new("ctrl-alt-shift-g", GroupSelectedWorkspaces, None),
                KeyBinding::new("ctrl-shift-r", RenameWorkspace, None),
                KeyBinding::new("ctrl-shift-t", NewTab, Some("Terminal")),
                KeyBinding::new("ctrl-pagedown", NextTab, Some("Terminal")),
                KeyBinding::new("ctrl-alt-shift-t", ShowTabPicker, Some("Terminal")),
                KeyBinding::new("ctrl-pageup", PreviousTab, Some("Terminal")),
                KeyBinding::new("ctrl-alt-down", NextWorkspace, None),
                KeyBinding::new("ctrl-alt-u", NextUnreadTerminal, None),
                KeyBinding::new("ctrl-alt-up", PreviousWorkspace, None),
                KeyBinding::new("ctrl-alt-shift-up", MoveWorkspaceUp, None),
                KeyBinding::new("ctrl-alt-shift-down", MoveWorkspaceDown, None),
                KeyBinding::new("ctrl-shift-b", ToggleSidebar, None),
                KeyBinding::new("ctrl-shift-v", Paste, Some("Terminal")),
                KeyBinding::new("ctrl-shift-c", CopySelection, Some("Terminal")),
                KeyBinding::new("ctrl-shift-a", EditorSelectAll, Some("Terminal")),
                KeyBinding::new("ctrl-shift-up", PreviousPrompt, Some("Terminal")),
                KeyBinding::new("ctrl-shift-down", NextPrompt, Some("Terminal")),
                KeyBinding::new("ctrl-shift-f", Find, Some("Terminal")),
                KeyBinding::new("ctrl-shift-f", Find, Some("FindInput")),
                KeyBinding::new("f3", FindOlder, Some("Terminal")),
                KeyBinding::new("shift-f3", FindNewer, Some("Terminal")),
                KeyBinding::new("f3", FindOlder, Some("FindInput")),
                KeyBinding::new("shift-f3", FindNewer, Some("FindInput")),
                KeyBinding::new("ctrl-=", IncreaseFontSize, Some("Terminal")),
                KeyBinding::new("ctrl--", DecreaseFontSize, Some("Terminal")),
                KeyBinding::new("ctrl-0", ResetFontSize, Some("Terminal")),
                KeyBinding::new("ctrl-v", Paste, Some("TextInput")),
                KeyBinding::new("ctrl-c", CopySelection, Some("TextInput")),
                KeyBinding::new("ctrl-c", CopySelection, Some("Notifications")),
                KeyBinding::new("ctrl-c", CopySelection, Some("ListeningPorts")),
                KeyBinding::new("ctrl-c", CopySelection, Some("CopyMode")),
                KeyBinding::new("ctrl-a", EditorSelectAll, Some("CopyMode")),
                KeyBinding::new("ctrl-f", Find, Some("CopyMode")),
                KeyBinding::new("ctrl-x", EditorCut, Some("TextInput")),
                KeyBinding::new("ctrl-z", EditorUndo, Some("TextInput")),
                KeyBinding::new("ctrl-shift-z", EditorRedo, Some("TextInput")),
                KeyBinding::new("ctrl-y", EditorRedo, Some("TextInput")),
                KeyBinding::new("ctrl-a", EditorSelectAll, Some("TextInput")),
                KeyBinding::new("ctrl-shift-w", ClosePane, Some("Terminal")),
            ]);
        }
        cx.bind_keys((1..=9).map(|number| {
            let modifier = if cfg!(target_os = "macos") {
                "cmd"
            } else {
                "ctrl-alt"
            };
            KeyBinding::new(
                &format!("{modifier}-{number}"),
                SelectWorkspaceNumber { number },
                None,
            )
        }));
        cx.bind_keys((1..=9).map(|number| {
            let modifier = if cfg!(target_os = "macos") {
                "cmd-alt"
            } else {
                "ctrl-alt-shift"
            };
            KeyBinding::new(
                &format!("{modifier}-{number}"),
                SelectPaneNumber { number },
                Some("Terminal"),
            )
        }));
        cx.bind_keys([KeyBinding::new(if cfg!(target_os = "macos") { "cmd-," } else { "ctrl-," }, ToggleSettings, None)]);
        cx.bind_keys([KeyBinding::new(if cfg!(target_os = "macos") { "cmd-s" } else { "ctrl-s" }, SaveAppearanceSettings, Some("Settings"))]);
        cx.bind_keys([
            KeyBinding::new("shift-f10", ShowTabMenu, Some("Terminal")),
            KeyBinding::new("ctrl-shift-pageup", MoveTabLeft, Some("Terminal")),
            KeyBinding::new("ctrl-shift-pagedown", MoveTabRight, Some("Terminal")),
            KeyBinding::new("ctrl-tab", NextPane, Some("Terminal")),
            KeyBinding::new("ctrl-shift-tab", PreviousPane, Some("Terminal")),
        ]);
        #[cfg(target_os = "macos")]
        cx.bind_keys(shortcuts::bracket_navigation());
        cx.bind_keys((1..=9).map(|number| {
            let modifier = if cfg!(target_os = "macos") {
                "cmd-shift"
            } else {
                "ctrl-shift"
            };
            KeyBinding::new(
                &format!("{modifier}-{number}"),
                SelectTabNumber { number },
                Some("Terminal"),
            )
        }));
        app_menu::initialize(cx);
        cx.bind_keys([KeyBinding::new(if cfg!(target_os = "macos") { "cmd-shift-c" } else { "ctrl-shift-c" }, CopySearchExcerpt, Some("CommandPalette"))]);
        cx.bind_keys([KeyBinding::new("ctrl-alt-w", TogglePreviewWrap, Some("TextPreview"))]);
        cx.bind_keys([
            KeyBinding::new("f5", RefreshFilePreview, Some("FilePreview")),
            KeyBinding::new("alt-up", PreviewDirectory, Some("FilePreview")),
            KeyBinding::new("ctrl-alt-m", SwitchPreviewMode, Some("FilePreview")),
            KeyBinding::new("ctrl-alt-v", ToggleMarkdownPreview, Some("FilePreview")),
        ]);
        cx.bind_keys([
            KeyBinding::new("alt-left", FileBrowserBack, Some("FileBrowser")),
            KeyBinding::new("alt-right", FileBrowserForward, Some("FileBrowser")),
            KeyBinding::new("alt-up", FileBrowserParent, Some("FileBrowser")),
            KeyBinding::new("f5", RefreshFileBrowser, Some("FileBrowser")),
            KeyBinding::new(if cfg!(target_os = "macos") { "cmd-l" } else { "ctrl-l" }, FileBrowserLocation, Some("FileBrowser")),
        ]);
        cx.bind_keys([
            KeyBinding::new("ctrl-alt-e", EditSelectedFile, Some("FileBrowser")),
            KeyBinding::new("ctrl-alt-e", EditSelectedFile, Some("FilePreview")),
        ]);
        #[cfg(all(target_os = "macos", feature = "browser"))]
        browser_window::initialize(cx);
        shortcuts::initialize(cx);
        cx.on_action(|_: &Quit, cx| request_quit(cx));
        cx.on_action(|_: &NewWindow, cx| {
            if let Err(error) = open_window(None, cx) {
                eprintln!("open rmux window: {error:#}");
            }
        });
        #[cfg(all(target_os = "macos", feature = "browser"))]
        cx.on_action(|_: &NewBrowserWindow, cx| {
            if let Err(error) = browser_window::open(cx) { eprintln!("open browser: {error:#}"); }
        });
        let updater_available = updater::initialize();
        if updater_available { cx.on_action(|_: &CheckForUpdates, _| updater::check()); }
        cx.set_menus(vec![
            Menu {
                name: "rmux".into(),
                items: app_menu::items(updater_available),
            },
            #[cfg(all(target_os = "macos", feature = "browser"))]
            browser_window::menu(),
            Menu {
                name: "File".into(),
                items: vec![
                    MenuItem::action("New Window", NewWindow),
                    #[cfg(all(target_os = "macos", feature = "browser"))]
                    MenuItem::action("New Browser Window (Experimental)", NewBrowserWindow),
                    MenuItem::action("New Workspace", NewWorkspace),
                    MenuItem::action("Cancel New Workspace", CancelNewWorkspace),
                    MenuItem::action("Connect via SSH…", ShowSshConnection),
                    MenuItem::action("SSH Hosts…", ShowSshHosts),
                    MenuItem::action("New Empty Workspace Group", NewWorkspaceGroup),
                    MenuItem::action("New Group from Workspace", GroupCurrentWorkspace),
                    MenuItem::action("Group Selected Workspaces", GroupSelectedWorkspaces),
                    MenuItem::action("Rename Workspace Group", RenameWorkspaceGroup),
                    MenuItem::action("Edit Group Color", EditGroupColor),
                    MenuItem::action("Edit Group Icon", EditGroupIcon),
                    MenuItem::action("Pin / Unpin Workspace Group", ToggleGroupPin),
                    MenuItem::action("Ungroup Workspaces", UngroupWorkspaces),
                    MenuItem::action("Close Workspace Group…", CloseWorkspaceGroup),
                    MenuItem::action("Remove Workspace from Group", RemoveWorkspaceFromGroup),
                    MenuItem::action("Use Workspace as Group Anchor", SetGroupAnchor),
                    MenuItem::action("Use Current Terminal Directory for Group", UseCurrentDirectoryForGroup),
                    MenuItem::action("Reset Group Directory to Follow Anchor", ResetGroupDirectory),
                    MenuItem::action("Rename Workspace", RenameWorkspace),
                    MenuItem::action("Edit Workspace Icon", EditWorkspaceIcon),
                    MenuItem::action("Edit Workspace Color", EditWorkspaceColor),
                    MenuItem::action("Edit Workspace Description", EditWorkspaceDescription),
                    MenuItem::action("Save Layout…", SaveLayout),
                    MenuItem::action("Save Session…", SaveSession),
                    MenuItem::action("Open Session in New Windows…", OpenSession),
                    MenuItem::action("Open Layout Preset…", ShowLayoutPresetPicker),
                    MenuItem::action("Run Configured Command…", ShowCustomCommandPicker),
                    MenuItem::action("Reload Settings", ReloadSettings),
                    MenuItem::action("Open Settings File", OpenSettingsFile),
                    MenuItem::action("Import Terminal Theme…", ImportTerminalTheme),
                    MenuItem::action("Open Layout in New Window…", OpenLayout),
                    MenuItem::action("Open Text Preview…", OpenTextPreview),
                    MenuItem::action("Cancel File / SSH Host Loading", CancelFileLoading),
                    MenuItem::action("Open Folder…", OpenFolderBrowser),
                    MenuItem::action("New Tab", NewTab),
                    MenuItem::action("New Local Tab", NewLocalTab),
                    MenuItem::action("Cancel New Terminal Tab", CancelNewTab),
                    MenuItem::action("Cancel Split Terminal", CancelSplit),
                    MenuItem::separator(),
                    MenuItem::action("Close Tab", ClosePane),
                    MenuItem::action("Close Other Tabs…", CloseOtherTabs),
                    MenuItem::action("Close Tabs to the Right…", CloseTabsToRight),
                    MenuItem::action("Close Finished Tabs…", CloseFinishedTabs),
                    MenuItem::action("Close Workspace", CloseWorkspace),
                    MenuItem::action("Close Window", CloseWindow),
                ],
            },
            Menu {
                name: "Edit".into(),
                items: vec![
                    MenuItem::action("Copy", CopySelection),
                    MenuItem::action("Copy with Line Breaks", CopySelectionWithLineBreaks),
                    MenuItem::action("Cancel Queued Paste", CancelQueuedPaste),
                    MenuItem::action("Cut", EditorCut),
                    MenuItem::action("Undo", EditorUndo),
                    MenuItem::action("Redo", EditorRedo),
                    MenuItem::action("Select All", EditorSelectAll),
                    MenuItem::action("Copy Working Directory", CopyWorkingDirectory),
                    MenuItem::action("Reveal Working Directory", RevealWorkingDirectory),
                    MenuItem::action("Clear Scrollback", ClearScrollback),
                    MenuItem::action("Paste", Paste),
                    MenuItem::action("Find", Find),
                    MenuItem::action("Search Terminal Output…", ShowTerminalSearch),
                    MenuItem::action("Find Older Match", FindOlder),
                    MenuItem::action("Find Newer Match", FindNewer),
                    MenuItem::action("Increase Font Size", IncreaseFontSize),
                    MenuItem::action("Decrease Font Size", DecreaseFontSize),
                    MenuItem::action("Reset Font Size", ResetFontSize),
                    MenuItem::action("Enter Copy Mode", EnterCopyMode),
                    MenuItem::action("Paste Named Buffer…", ShowBufferPicker),
                    MenuItem::action("Delete Named Buffer…", ShowDeleteBufferPicker),
                    MenuItem::action("Rename Named Buffer…", ShowRenameBufferPicker),
                    MenuItem::action("Copy Named Buffer…", ShowCopyBufferPicker),
                    MenuItem::action("Browse Terminal Directory…", BrowseTerminalFiles),
                    MenuItem::action("Choose Terminal Font…", ShowFontPicker),
                    MenuItem::action("Choose Default Font…", ShowDefaultFontPicker),
                    MenuItem::action("Use Configured Font", UseConfiguredFont),
                ],
            },
            Menu {
                name: "View".into(),
                items: vec![
                    MenuItem::action("Split Right", Split),
                    MenuItem::action("Command Palette…", ShowCommandPalette),
                    MenuItem::action("Split Below", SplitBelow),
                    MenuItem::action("Split Left", SplitLeft),
                    MenuItem::action("Split Above", SplitAbove),
                    MenuItem::action("Toggle Pane Zoom", TogglePaneZoom),
                    MenuItem::action("Equalize Pane Sizes", EqualizePanes),
                    MenuItem::action("Resize Pane Left", ResizePaneLeft),
                    MenuItem::action("Resize Pane Right", ResizePaneRight),
                    MenuItem::action("Resize Pane Up", ResizePaneUp),
                    MenuItem::action("Resize Pane Down", ResizePaneDown),
                    MenuItem::action("Previous Prompt", PreviousPrompt),
                    MenuItem::action("Next Prompt", NextPrompt),
                    MenuItem::action("Last Prompt", LastPrompt),
                    MenuItem::action("Scroll to Top", ScrollToTop),
                    MenuItem::action("Scroll to Bottom", ScrollToBottom),
                    MenuItem::action("Swap Pane with Next", SwapPaneNext),
                    MenuItem::action("Swap Pane Left", SwapPaneLeft),
                    MenuItem::action("Swap Pane Right", SwapPaneRight),
                    MenuItem::action("Swap Pane Up", SwapPaneUp),
                    MenuItem::action("Swap Pane Down", SwapPaneDown),
                    MenuItem::action("Move Tab to Split Right", BreakTabRight),
                    MenuItem::action("Move Tab to Split Below", BreakTabBelow),
                    MenuItem::action("Move Tab to Split Left", BreakTabLeft),
                    MenuItem::action("Move Tab to Split Above", BreakTabAbove),
                    MenuItem::action("Join Pane into Next", JoinPaneNext),
                    MenuItem::action("Join Pane Left", JoinPaneLeft),
                    MenuItem::action("Join Pane Right", JoinPaneRight),
                    MenuItem::action("Join Pane Up", JoinPaneUp),
                    MenuItem::action("Join Pane Down", JoinPaneDown),
                    MenuItem::action("Move Tab to Next Workspace", MoveTabNextWorkspace),
                    MenuItem::action("Detach Tab to New Window", DetachTab),
                    MenuItem::action("Move Workspace to New Window", DetachWorkspace),
                    MenuItem::action("Merge Workspace into…", ShowWorkspaceMergePicker),
                    MenuItem::action("Move Workspace to Window…", ShowWorkspaceWindowPicker),
                    MenuItem::action("Move Workspace to Next Window", MoveWorkspaceToNextWindow),
                    MenuItem::action("Focus Left", FocusLeft),
                    MenuItem::action("Focus Right", FocusRight),
                    MenuItem::action("Focus Up", FocusUp),
                    MenuItem::action("Focus Down", FocusDown),
                    MenuItem::action("Previous Pane", PreviousPane),
                    MenuItem::action("Next Pane", NextPane),
                    MenuItem::action("Previous Tab", PreviousTab),
                    MenuItem::action("Next Tab", NextTab),
                    MenuItem::action("Toggle Sidebar", ToggleSidebar),
                    MenuItem::action("Next Workspace", NextWorkspace),
                    MenuItem::action("Last Workspace", LastWorkspace),
                    MenuItem::action("Last Terminal", LastTerminal),
                    MenuItem::action("Focus Back", FocusBack),
                    MenuItem::action("Focus Forward", FocusForward),
                    MenuItem::action("Rename Tab…", RenameTab),
                    MenuItem::action("Pin / Unpin Workspace", ToggleWorkspacePin),
                    MenuItem::submenu(Menu {
                        name: "Workspace Icon".into(),
                        items: WORKSPACE_ICONS
                            .iter()
                            .map(|(name, icon)| {
                                MenuItem::action(
                                    *name,
                                    SetWorkspaceIcon {
                                        icon: icon.map(str::to_owned),
                                    },
                                )
                            })
                            .collect(),
                    }),
                    MenuItem::submenu(Menu {
                        name: "Workspace Color".into(),
                        items: WORKSPACE_COLORS
                            .iter()
                            .map(|(name, color)| {
                                MenuItem::action(*name, SetWorkspaceColor { color: *color })
                            })
                            .collect(),
                    }),
                    MenuItem::action("Next Unread Terminal", NextUnreadTerminal),
                    MenuItem::action("Mark All Read in This Window", MarkAllRead),
                    MenuItem::action("Toggle Notifications", ToggleNotifications),
                    MenuItem::action("Toggle Program Status", ToggleProgramStatus),
                    MenuItem::action("Show Listening Ports", ShowListeningPorts),
                    MenuItem::action("Show / Refresh Terminal Memory", RefreshTerminalMemory),
                    MenuItem::action("Previous Workspace", PreviousWorkspace),
                ],
            },
        ]);
        // System-originated quits can bypass our Quit action. This callback
        // cannot veto shutdown; GPUI gives its returned future only 100 ms.
        // Request every process shutdown synchronously before awaiting any one.
        cx.on_app_quit(|cx| {
            let checkpoint = checkpoint_before_quit(cx);
            let handles = begin_shutdown(cx).unwrap_or_default();
            async move {
                if let Some(checkpoint) = checkpoint {
                    if let Err(error) = checkpoint.await { eprintln!("rmux: save session: {error:#}"); }
                }
                for handle in handles {
                    handle.wait().await;
                }
            }
        }).detach();
        cx.on_window_closed(|cx| {
            recovery::changed(cx);
            if cx.windows().is_empty() {
                quit_after_cleanup(cx);
            }
        })
        .detach();
        if let Some(session) = initial_session.take().filter(|session| !session.windows.is_empty()) {
            let active = session.active_window.unwrap_or(0);
            let mut selected = None;
            for (index, prepared) in session.windows.into_iter().enumerate() {
                let handle = cx.open_window(
                    WindowOptions {
                        titlebar: workspace_titlebar(),
                    app_id: Some("rmux".into()),
                        focus: index == active,
                        window_bounds: Some(prepared.snapshot.geometry.map(|geometry| geometry.restore(cx)).unwrap_or_else(|| WindowBounds::centered(size(px(1000.), px(680.)), cx))),
                        ..Default::default()
                    },
                    move |window, cx| cx.new(|cx| window_view::WorkspaceWindow::restored(prepared, window, cx)),
                ).expect("open restored session window");
                if index == active { selected = Some(handle); }
            }
            if let Some(handle) = selected { let _ = handle.update(cx, |_, window, _| window.activate_window()); }
            cx.activate(true);
        } else if let Some(prepared) = initial_layout.take() {
            let bounds = prepared.snapshot.geometry.map(|geometry| geometry.restore(cx)).unwrap_or_else(|| WindowBounds::centered(size(px(1000.), px(680.)), cx));
            cx.open_window(
                WindowOptions {
                        titlebar: workspace_titlebar(),
                    app_id: Some("rmux".into()),
                    window_bounds: Some(bounds),
                    ..Default::default()
                },
                move |window, cx| {
                    cx.new(|cx| window_view::WorkspaceWindow::restored(prepared, window, cx))
                },
            )
            .expect("open restored rmux window");
            cx.activate(true);
        } else {
            let handle = open_window_with_activation(benchmark_panes, true, cx).expect("open rmux window");
            if let Some(notice) = recovery_notice.take() {
                let _ = handle.update(cx, |view, _, cx| view.show_startup_notice(notice, cx));
            }
        }
        let socket_directory = std::env::var_os("RMUX_SOCKET_DIR").map(std::path::PathBuf::from);
        if live_rpc_stdio || socket_directory.is_some() {
            if let Err(error) = automation::attach_live(cx, live_rpc_stdio, socket_directory) {
                eprintln!("rmux: RPC stream: {error}");
            }
        }
    });
}
