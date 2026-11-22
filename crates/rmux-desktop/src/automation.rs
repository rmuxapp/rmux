//! Bounded JSON-RPC discovery over explicit inherited pipes.
use serde_json::{Value, json};
use std::io::{self, BufRead, Write};
use std::sync::{
    Arc,
    atomic::{AtomicU8, Ordering},
};
use std::time::{Duration, Instant};

const MAX_REQUEST_BYTES: usize = 64 * 1024;
const METHODS: &[&str] = &["system.ping", "system.capabilities"];
const LIVE_METHODS: &[&str] = &[
    "events.read",
    "buffer.list", "buffer.get", "buffer.set", "buffer.delete", "buffer.paste", "buffer.rename",
    "system.quit",
    "system.ping",
    "system.capabilities",
    "window.list",
    "window.focus_history",
    "window.create",
    "workspace.list",
    "workspace.create",
    "workspace.open",
    "workspace.close",
    "pane.list",
    "pane.zoom.set",
    "pane.split",
    "pane.resize",
    "pane.swap",
    "workspace.equalize",
    "surface.list",
    "notification.list",
    "notification.send",
    "notification.dismiss",
    "notification.mark_read",
    "surface.focus",
    "surface.send_text",
    "surface.send_key",
    "surface.read_text",
    "surface.memory_usage",
    "surface.move_to_workspace",
    "workspace.select",
    "workspace.rename",
    "workspace.group.create",
    "workspace.group.update",
    "workspace.group.ungroup",
    "workspace.group.add",
    "workspace.group.remove",
    "workspace.group.set_anchor",
    "workspace.group.new_workspace",
    "workspace.move_to_window",
    "workspace.merge",
];

#[derive(Debug)]
pub(super) struct Failure {
    pub code: i32,
    pub message: &'static str,
}
type RpcResult = Result<Value, Failure>;

fn array_result(key: &str, values: Vec<Value>) -> Value {
    let mut object = serde_json::Map::new();
    object.insert(key.into(), Value::Array(values));
    Value::Object(object)
}

pub(super) fn target_id(text: &str) -> Result<gpui::EntityId, Failure> {
    let missing = || Failure {
        code: -32004,
        message: "Target not found",
    };
    if text.is_empty() || text.starts_with('0') || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(missing());
    }
    let value: u64 = text.parse().map_err(|_| missing())?;
    let id = gpui::EntityId::from(value);
    // Slot-map conversion can normalize invalid generation bits. Never let an
    // alternate textual spelling alias a real, generation-bearing entity ID.
    if id.as_u64() != value {
        return Err(missing());
    }
    Ok(id)
}

fn send_surface_input(input: &SurfaceText, cx: &gpui::App) -> RpcResult {
    let target = target_id(&input.surface_id)?;
    for handle in cx.windows() {
        let Some(handle) = handle.downcast::<super::window_view::WorkspaceWindow>() else { continue; };
        if let Ok(view) = handle.read(cx) {
            if view.automation_surface_target(target, cx).is_some() {
                return view.automation_send_text(input, target, cx);
            }
        }
    }
    Err(Failure { code: -32004, message: "Surface not found" })
}

pub(super) fn layout_snapshot<T>(
    tree: &super::layout::PaneTree<T>,
    id: &impl Fn(&T) -> String,
) -> Value {
    use super::layout::{Axis, PaneTree};
    match tree {
        PaneTree::Pane(pane) => json!({"type":"pane", "pane_id":id(pane)}),
        PaneTree::Split {
            axis,
            children,
            weights,
        } => {
            let mut result = json!({
            "type":"split", "axis":match axis { Axis::Rows => "rows", Axis::Columns => "columns" },
            "weights":weights,
            });
            result["children"] = Value::Array(
                children
                    .iter()
                    .map(|child| layout_snapshot(child, id))
                    .collect(),
            );
            result
        }
    }
}

#[derive(serde::Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub(super) struct Selection {
    pub workspace_id: String,
    #[serde(default)]
    pub activate: bool,
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub(super) enum FocusDirection { Back, Forward }
#[derive(serde::Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub(super) struct FocusHistory {
    pub window_id: String,
    pub direction: FocusDirection,
    #[serde(default)]
    pub activate: bool,
}

#[derive(serde::Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub(super) struct SurfaceFocus {
    pub surface_id: String,
    #[serde(default)]
    pub activate: bool,
}

#[derive(serde::Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub(super) struct SurfaceText {
    pub surface_id: String,
    pub text: String,
    #[serde(default)]
    pub paste: bool,
}

/// Resolve exact names across normal windows; cwd and startup parameters are
/// creation-only. Never guess among duplicate names.
fn resolve_open_workspace(create: &mut WorkspaceCreate, cx: &mut gpui::App) -> Result<Option<Value>, Failure> {
    let target = if create.window_id.is_empty() { None } else { Some(target_id(&create.window_id)?) };
    let name = create.name.as_deref().expect("validated workspace.open name");
    let mut windows = Vec::new();
    let mut matches = Vec::new();
    for handle in cx.windows() {
        let Some(handle) = handle.downcast::<super::window_view::WorkspaceWindow>() else { continue; };
        let any: gpui::AnyWindowHandle = handle.into();
        let Ok(Some((id, workspaces))) = any.read::<super::window_view::WorkspaceWindow, _, _>(cx, |entity, cx| {
            entity.read(cx).automation_named_workspaces(name).map(|workspaces| (entity.entity_id(), workspaces))
        }) else { continue; };
        if target.is_some_and(|target| target != id) { continue; }
        for workspace in workspaces { matches.push((id, handle, workspace)); }
        windows.push((id, handle));
    }
    if matches.len() > 1 { return Err(Failure { code: -32006, message: "Workspace name is ambiguous; use --window or rename duplicates" }); }
    if let Some((id, handle, workspace)) = matches.pop() {
        create.window_id = id.to_string();
        return handle.update(cx, |view, window, cx| view.automation_reuse_workspace(create, workspace, window, cx))
            .map_err(|_| Failure { code: -32004, message: "Window closed" })?.map(Some);
    }
    windows.sort_by_key(|(id, _)| id.as_u64());
    let preferred = cx.active_window().and_then(|active| windows.iter().find(|(_, handle)| gpui::AnyWindowHandle::from(*handle) == active));
    let Some((id, _)) = preferred.or_else(|| windows.first()) else {
        return Err(Failure { code: -32004, message: "No matching rmux window; open an rmux window first" });
    };
    create.window_id = id.to_string();
    Ok(None)
}

fn default_read_limit() -> usize {
    64 * 1024
}
#[derive(serde::Deserialize, Debug, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(super) enum ReadScope {
    #[default]
    Viewport,
    History,
    Selection,
}
#[derive(serde::Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub(super) struct SurfaceRead {
    pub surface_id: String,
    #[serde(default)]
    pub scope: ReadScope,
    #[serde(default)]
    pub lines: Option<usize>,
    #[serde(default)]
    pub preserve_wraps: bool,
    #[serde(default = "default_read_limit")]
    pub max_bytes: usize,
}

#[derive(serde::Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub(super) struct PaneZoom {
    pub pane_id: String,
    pub zoomed: bool,
}

#[derive(serde::Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub(super) struct PaneSwap {
    pub pane_id: String,
    pub other_pane_id: String,
}

#[derive(serde::Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub(super) struct PaneResize {
    pub pane_id: String,
    pub direction: super::layout::Direction,
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub(super) enum SplitDirection { Left, Right, Up, Down }
#[derive(serde::Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub(super) struct PaneSplit {
    pub ssh_destination: Option<String>,
    pub pane_id: String,
    pub direction: SplitDirection,
    pub cwd: Option<std::path::PathBuf>,
    #[serde(default)] pub focus: bool,
    #[serde(default)] pub activate: bool,
}

#[derive(serde::Deserialize, Debug, Default)]
#[serde(deny_unknown_fields)]
struct WindowCreate {
    #[serde(default)]
    activate: bool,
}

#[derive(Clone, serde::Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub(super) struct WorkspaceCreate {
    #[serde(default)]
    pub window_id: String,
    #[serde(skip)]
    pub reuse: bool,
    pub ssh_destination: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub cwd: Option<std::path::PathBuf>,
    #[serde(default)]
    pub focus: bool,
    #[serde(default)]
    pub activate: bool,
}

#[derive(serde::Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub(super) struct WorkspaceRename {
    pub workspace_id: String,
    pub name: String,
}

#[derive(serde::Deserialize, Debug)]
#[serde(deny_unknown_fields)]
struct WorkspaceMerge {
    workspace_id: String,
    target_workspace_id: String,
    #[serde(default)]
    activate: bool,
}

#[derive(serde::Deserialize, Debug)]
#[serde(deny_unknown_fields)]
struct WorkspaceMove {
    workspace_id: String,
    window_id: String,
    #[serde(default)]
    activate: bool,
}

#[derive(serde::Deserialize, Debug)]
#[serde(deny_unknown_fields)]
struct SurfaceMove {
    surface_id: String,
    workspace_id: String,
    #[serde(default)]
    activate: bool,
}

// A timed-out queued mutation must never run after the client resumes work.
// Once execution starts, timeout reports an explicitly unknown outcome.
struct Pending {
    state: AtomicU8, // 0 queued, 1 started, 2 canceled
    deadline: Instant,
}
impl Pending {
    fn new() -> Self {
        Self {
            state: AtomicU8::new(0),
            deadline: Instant::now() + Duration::from_secs(5),
        }
    }
    fn start(&self) -> bool {
        Instant::now() < self.deadline
            && self
                .state
                .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
    }
    fn cancel(&self) -> bool {
        self.state
            .compare_exchange(0, 2, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }
}

#[derive(serde::Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub(super) struct GroupMutation {
    pub cwd: Option<std::path::PathBuf>,
    pub clear_cwd: Option<bool>,
    pub remove_generated_anchor: Option<bool>,
    pub window_id: String,
    pub group_id: String,
    pub placement: Option<super::workspace_groups::Placement>,
    pub workspace_id: Option<String>,
    pub name: Option<String>,
    pub collapsed: Option<bool>,
    pub pinned: Option<bool>,
}

#[derive(serde::Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub(super) struct GroupCreate {
    pub cwd: Option<std::path::PathBuf>,
    pub window_id: String,
    pub name: String,
    pub workspace_ids: Option<Vec<String>>,
    pub external_id: Option<String>,
}

#[derive(serde::Deserialize, Debug)]
#[serde(deny_unknown_fields)]
struct SurfaceMemory { surface_id: String }

#[derive(Default, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ListFilter {
    window_id: Option<String>,
    workspace_id: Option<String>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NotificationChange {
    #[serde(skip)]
    pub mark_read: bool,
    pub surface_id: String,
    pub title: Option<String>,
    pub body: Option<String>,
    pub revision: Option<u64>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SurfaceKey {
    pub surface_id: String,
    pub key: String,
    #[serde(default)] pub control: bool,
    #[serde(default)] pub alt: bool,
    #[serde(default)] pub shift: bool,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkspaceTarget { workspace_id: String }

struct LiveRequest {
    events_read: Option<super::events::Read>,
    buffer_command: Option<super::copy_buffers::Command>,
    list_filter: ListFilter,
    notification_change: Option<NotificationChange>,
    surface_key: Option<SurfaceKey>,
    workspace_close: Option<WorkspaceTarget>,
    workspace_equalize: Option<WorkspaceTarget>,
    method: String,
    selection: Option<Selection>,
    focus_history: Option<FocusHistory>,
    surface_focus: Option<SurfaceFocus>,
    surface_text: Option<SurfaceText>,
    surface_read: Option<SurfaceRead>,
    surface_memory: Option<SurfaceMemory>,
    surface_move: Option<SurfaceMove>,
    pane_zoom: Option<PaneZoom>,
    pane_split: Option<PaneSplit>,
    pane_resize: Option<PaneResize>,
    pane_swap: Option<PaneSwap>,
    window_create: Option<WindowCreate>,
    workspace_create: Option<WorkspaceCreate>,
    workspace_rename: Option<WorkspaceRename>,
    group_mutation: Option<GroupMutation>,
    group_create: Option<GroupCreate>,
    workspace_move: Option<WorkspaceMove>,
    workspace_merge: Option<WorkspaceMerge>,
    pending: Arc<Pending>,
    reply: std::sync::mpsc::SyncSender<RpcResult>,
}

/// The worker owns blocking pipe IO; GPUI receives bounded, validated requests.
/// No work is scheduled until a request arrives. Closing stdin detaches RPC only.
pub fn attach_live(cx: &mut gpui::App, stdio: bool, socket_directory: Option<std::path::PathBuf>) -> io::Result<()> {
    let (sender, receiver) = async_channel::bounded::<LiveRequest>(8);
    if let Some(directory) = socket_directory {
        #[cfg(not(unix))]
        { let _ = directory; return Err(io::Error::new(io::ErrorKind::Unsupported, "local socket transport currently requires Unix")); }
        #[cfg(unix)]
        {
            let (endpoint, listener) = super::socket_transport::Endpoint::bind(&directory)?;
            let sender = sender.clone();
            super::socket_transport::listen(listener, move |stream| {
                if let Ok(reader) = stream.try_clone() {
                    let _ = serve_inner(io::BufReader::new(reader), stream, Some(&sender));
                }
            })?;
            cx.on_app_quit(move |_| { endpoint.cleanup(); async {} }).detach();
            eprintln!("rmux: RPC socket: {}", directory.join("rpc.sock").display());
        }
    }
    if stdio {
    let sender = sender.clone();
    std::thread::Builder::new()
        .name("rmux-rpc".into())
        .spawn(move || {
            if let Err(error) = serve_inner(
                std::io::stdin().lock(),
                std::io::stdout().lock(),
                Some(&sender),
            ) {
                eprintln!("rmux: live RPC stream: {error}");
            }
        })?;
    }
    cx.spawn(async move |cx| {
        while let Ok(request) = receiver.recv().await {
            if let Some(read) = request.events_read.as_ref() {
                let setup = cx.update(|cx| {
                    if !request.pending.start() { return Err(Failure { code: -32001, message: "Request expired before execution" }); }
                    let journal = cx.global::<super::events::Journal>().clone();
                    let batch = journal.read(read).map_err(|message| Failure { code: -32602, message })?;
                    let waiter = if read.wait_ms > 0 && batch.events.is_empty() && batch.gap.is_none() {
                        Some(journal.subscribe(read).map_err(|message| Failure { code: -32000, message })?)
                    } else { None };
                    // Reading and registering occur in one UI update: a publish
                    // cannot slip between the empty read and wake registration.
                    Ok((journal, batch, waiter))
                }).unwrap_or(Err(Failure { code: -32004, message: "Application closed" }));
                match setup {
                    Err(error) => { let _ = request.reply.try_send(Err(error)); }
                    Ok((_, batch, None)) => { let _ = request.reply.try_send(Ok(serde_json::to_value(batch).expect("event batch"))); }
                    Ok((journal, batch, Some(waiter))) => {
                        let mut read = read.clone();
                        // An initial empty read establishes a cursor for this
                        // wait instead of replaying older records on wake.
                        read.after = Some(batch.cursor);
                        let duration = Duration::from_millis(u64::from(read.wait_ms))
                            .min(request.pending.deadline.saturating_duration_since(Instant::now()).saturating_sub(Duration::from_millis(100)));
                        cx.spawn(async move |cx| {
                            let timer = cx.background_executor().timer(duration);
                            futures_lite::future::race(async { let _ = waiter.receiver.recv().await; }, async { timer.await; }).await;
                            let result = journal.read(&read)
                                .map(|batch| serde_json::to_value(batch).expect("event batch"))
                                .map_err(|message| Failure { code: -32602, message });
                            drop(waiter);
                            let _ = request.reply.try_send(result);
                        }).detach();
                    }
                }
                // Waiting is a separate foreground task, never an await in the
                // shared dispatcher. Other clients can mutate/read immediately.
                continue;
            }
            if let Some(create) = request.workspace_create.as_ref() {
                let mut create = create.clone();
                let prepared = cx.update(|cx| {
                    if !request.pending.start() { return Err(Failure { code: -32001, message: "Request expired before execution" }); }
                    if create.reuse {
                        if let Some(workspace) = resolve_open_workspace(&mut create, cx)? { return Ok(Err(workspace)); }
                    }
                    let target = target_id(&create.window_id)?;
                    for handle in cx.windows() {
                        let Some(handle) = handle.downcast::<super::window_view::WorkspaceWindow>() else { continue; };
                        let any: gpui::AnyWindowHandle = handle.into();
                        if any.read::<super::window_view::WorkspaceWindow, _, _>(cx, |entity, _| entity.entity_id() == target).unwrap_or(false) {
                            let job = handle.update(cx, |view, _, cx| view.automation_prepare_workspace(&create, cx))
                                .map_err(|_| Failure { code: -32004, message: "Window closed" })??;
                            return Ok(Ok((handle, job)));
                        }
                    }
                    Err(Failure { code: -32004, message: "Window not found" })
                }).unwrap_or(Err(Failure { code: -32004, message: "Application closed" }));
                let result = match prepared {
                    Ok(Err(workspace)) => Ok(workspace),
                    Ok(Ok((handle, job))) => {
                        let mut prepared = cx.background_executor().spawn(async move { job() }).await;
                        loop {
                            let refresh = cx.update(|cx| {
                                let prepared = prepared.as_ref().ok()?;
                                let settings = cx.global::<super::Preferences>();
                                let current = (settings.terminal_colors(), settings.effective_palette());
                                ((prepared.colors, prepared.palette) != current).then_some(current)
                            }).ok().flatten();
                            let Some((colors, palette)) = refresh else { break; };
                            let session = prepared.expect("successful preparation checked above").session;
                            prepared = cx.background_executor().spawn(async move {
                                super::TerminalView::prepare(session, colors, palette)
                            }).await;
                        }
                        handle.update(cx, |view, window, cx| view.automation_finish_workspace(&create, prepared, window, cx))
                            .map_err(|_| Failure { code: -32004, message: "Window closed during terminal startup" })
                            .and_then(|result| result)
                    }
                    Err(error) => Err(error),
                };
                let _ = request.reply.try_send(result);
                continue;
            }
            if let Some(split) = request.pane_split.as_ref() {
                let prepared = cx.update(|cx| {
                    if !request.pending.start() { return Err(Failure { code: -32001, message: "Request expired before execution" }); }
                    let target = target_id(&split.pane_id)?;
                    for handle in cx.windows() {
                        let Some(handle) = handle.downcast::<super::window_view::WorkspaceWindow>() else { continue; };
                        if handle.read(cx).is_ok_and(|view| view.automation_pane_target(target, cx).is_some()) {
                            let job = handle.update(cx, |view, _, cx| view.automation_prepare_split(split, target, cx))
                                .map_err(|_| Failure { code: -32004, message: "Pane window closed" })??;
                            return Ok((handle, target, job));
                        }
                    }
                    Err(Failure { code: -32004, message: "Pane not found" })
                }).unwrap_or(Err(Failure { code: -32004, message: "Application closed" }));
                let result = match prepared {
                    Ok((handle, target, job)) => {
                        let mut prepared = cx.background_executor().spawn(async move { job() }).await;
                        loop {
                            let refresh = cx.update(|cx| {
                                let prepared = prepared.as_ref().ok()?;
                                let settings = cx.global::<super::Preferences>();
                                let current = (settings.terminal_colors(), settings.effective_palette());
                                ((prepared.colors, prepared.palette) != current).then_some(current)
                            }).ok().flatten();
                            let Some((colors, palette)) = refresh else { break; };
                            let session = prepared.expect("successful preparation checked above").session;
                            prepared = cx.background_executor().spawn(async move {
                                super::TerminalView::prepare(session, colors, palette)
                            }).await;
                        }
                        handle.update(cx, |view, window, cx| view.automation_finish_split(split, target, prepared, window, cx))
                            .map_err(|_| Failure { code: -32004, message: "Pane window closed during terminal startup" })
                            .and_then(|result| result)
                    }
                    Err(error) => Err(error),
                };
                let _ = request.reply.try_send(result);
                continue;
            }
            if let Some(sample) = &request.surface_memory {
                let job = cx.update(|cx| {
                    if !request.pending.start() { return Err(Failure { code: -32001, message: "Request expired before execution" }); }
                    let target = target_id(&sample.surface_id)?;
                    for handle in cx.windows() {
                        let Some(handle) = handle.downcast::<super::window_view::WorkspaceWindow>() else { continue; };
                        if let Ok(view) = handle.read(cx) {
                            if let Some(job) = view.automation_memory_sample(target, cx) { return Ok(job); }
                        }
                    }
                    Err(Failure { code: -32004, message: "Surface not found" })
                }).unwrap_or(Err(Failure { code: -32000, message: "Application unavailable" }));
                let result = match job {
                    Ok(job) => {
                        let surface_id = sample.surface_id.clone();
                        cx.background_executor().spawn(async move {
                            let usage = job().map_err(|_| Failure { code: -32007, message: "Memory sample failed or terminal closed" })?
                                .ok_or(Failure { code: -32007, message: "Terminal is busy; retry sample" })?;
                            Ok(json!({"surface_id":surface_id,"scope":"ghostty_screen_storage",
                                "primary_resident_bytes":usage.primary_resident_bytes,
                                "alternate_resident_bytes":usage.alternate_resident_bytes,
                                "virtual_bytes":usage.virtual_bytes,"compressed_bytes":usage.compressed_bytes,
                                "image_bytes":usage.image_bytes,"compression_supported":usage.compression_supported}))
                        }).await
                    }
                    Err(error) => Err(error),
                };
                let _ = request.reply.try_send(result);
                continue;
            }
            if let Some(read) = request.surface_read.as_ref().filter(|read| read.scope != ReadScope::Viewport) {
                let job = cx.update(|cx| {
                    if !request.pending.start() {
                        return Err(Failure { code: -32001, message: "Request expired before execution" });
                    }
                    let target = target_id(&read.surface_id)?;
                    for handle in cx.windows() {
                        let Some(handle) = handle.downcast::<super::window_view::WorkspaceWindow>() else { continue; };
                        if let Ok(view) = handle.read(cx) {
                            if let Some(job) = view.automation_text_export(target, read.max_bytes, read.scope == ReadScope::Selection, read.preserve_wraps, read.lines, cx) {
                                return Ok(job);
                            }
                        }
                    }
                    Err(Failure { code: -32004, message: "Surface not found" })
                }).unwrap_or(Err(Failure { code: -32000, message: "Application unavailable" }));
                let result = match job {
                    Ok(job) => {
                        let surface_id = read.surface_id.clone();
                        let scope = if read.scope == ReadScope::Selection { "selection" } else { "history" };
                        cx.background_executor().spawn(async move {
                            let (text, truncated) = job()
                                .map_err(|_| Failure { code: -32007, message: "Text export failed or terminal closed" })?
                                .ok_or(Failure { code: -32007, message: "Terminal is busy; retry export" })?;
                            let mut result = json!({"surface_id":surface_id,"scope":scope,"truncated":truncated});
                            result["text"] = Value::String(text);
                            Ok(result)
                        }).await
                    }
                    Err(error) => Err(error),
                };
                let _ = request.reply.try_send(result);
                continue;
            }
            let result = cx.update(|cx| {
                if !request.pending.start() {
                    return Err(Failure {
                        code: -32001,
                        message: "Request expired before execution",
                    });
                }
                if let Some(command) = &request.buffer_command {
                    if let Some(input) = cx.global::<super::copy_buffers::Store>().paste_input(command)
                        .map_err(|message| Failure { code: -32008, message })? {
                        return send_surface_input(&input, cx);
                    }
                    return cx.global_mut::<super::copy_buffers::Store>().execute(command)
                        .map_err(|message| Failure { code: -32008, message });
                }
                if request.method == "system.quit" {
                    super::request_quit(cx);
                    return Ok(json!({"requested":true}));
                }
                if let Some(movement) = &request.surface_move {
                    let surface = target_id(&movement.surface_id)?;
                    let workspace = target_id(&movement.workspace_id)?;
                    let windows = cx.windows();
                    let destination = windows.iter().copied().find(|handle| {
                        handle.downcast::<super::window_view::WorkspaceWindow>().is_some_and(|handle| handle.read(cx).is_ok_and(|view| view.automation_contains_workspace(workspace)))
                    }).ok_or(Failure { code: -32004, message: "Destination workspace not found" })?;
                    for handle in windows {
                        let Some(handle) = handle.downcast::<super::window_view::WorkspaceWindow>() else { continue; };
                        if handle.read(cx).is_ok_and(|view| view.automation_surface_target(surface, cx).is_some()) {
                            handle.update(cx, |view, window, cx| view.move_surface_to_workspace(surface, destination, workspace, movement.activate, window, cx))
                                .map_err(|_| Failure { code: -32004, message: "Source window closed" })?
                                .map_err(|_| Failure { code: -32003, message: "Tab transfer unavailable; source or destination is busy" })?;
                            return Ok(json!({"surface_id":movement.surface_id,"workspace_id":movement.workspace_id}));
                        }
                    }
                    return Err(Failure { code: -32004, message: "Surface not found" });
                }
                if let Some(create) = &request.window_create {
                    if cx.global::<super::Shutdowns>().quitting
                        || cx.global::<super::PromptGate>().is_active()
                    {
                        return Err(Failure {
                            code: -32003,
                            message: "Application is quitting or waiting for a close prompt",
                        });
                    }
                    let handle = super::open_window_with_activation(None, create.activate, cx)
                        .map_err(|_| Failure {
                            code: -32005,
                            message: "Could not create window",
                        })?;
                    let handle: gpui::AnyWindowHandle = handle.into();
                    return handle
                        .read::<super::window_view::WorkspaceWindow, _, _>(cx, |entity, cx| {
                            entity.read(cx).automation_snapshot(
                                entity.entity_id().to_string(),
                                true,
                                cx,
                            )
                        })
                        .map_err(|_| Failure {
                            code: -32002,
                            message: "Window created but response unavailable; inspect window.list",
                        });
                }
                if let Some(equalize) = &request.workspace_equalize {
                    let target = target_id(&equalize.workspace_id)?;
                    for handle in cx.windows() {
                        let Some(handle) = handle.downcast::<super::window_view::WorkspaceWindow>() else { continue; };
                        if handle.read(cx).is_ok_and(|view| view.automation_contains_workspace(target)) {
                            return handle.update(cx, |view, _, cx| view.automation_equalize_workspace(target, cx))
                                .map_err(|_| Failure { code: -32004, message: "Window closed" })?;
                        }
                    }
                    return Err(Failure { code: -32004, message: "Workspace not found" });
                }
                if let Some(close) = &request.workspace_close {
                    let target = target_id(&close.workspace_id)?;
                    for handle in cx.windows() {
                        let Some(handle) = handle.downcast::<super::window_view::WorkspaceWindow>() else { continue; };
                        if handle.read(cx).is_ok_and(|view| view.automation_contains_workspace(target)) {
                            return handle.update(cx, |view, window, cx| view.automation_close_workspace(target, window, cx))
                                .map_err(|_| Failure { code: -32004, message: "Window closed" })?;
                        }
                    }
                    return Err(Failure { code: -32004, message: "Workspace not found" });
                }
                if let Some(input) = &request.surface_key {
                    let target = target_id(&input.surface_id)?;
                    for handle in cx.windows() {
                        let Some(handle) = handle.downcast::<super::window_view::WorkspaceWindow>() else { continue; };
                        if let Ok(view) = handle.read(cx) {
                            if view.automation_surface_target(target, cx).is_some() { return view.automation_send_key(input, target, cx); }
                        }
                    }
                    return Err(Failure { code: -32004, message: "Surface not found" });
                }
                if let Some(change) = &request.notification_change {
                    let target = target_id(&change.surface_id)?;
                    for handle in cx.windows() {
                        let Some(handle) = handle.downcast::<super::window_view::WorkspaceWindow>() else { continue; };
                        if handle.read(cx).is_ok_and(|view| view.automation_surface_target(target, cx).is_some()) {
                            return handle.update(cx, |view, _, cx| view.automation_notification(change, target, cx))
                                .map_err(|_| Failure { code: -32004, message: "Window closed" })?;
                        }
                    }
                    return Err(Failure { code: -32004, message: "Surface not found" });
                }
                if let Some(create) = &request.group_create {
                    let window_id = target_id(&create.window_id)?;
                    for handle in cx.windows() {
                        let Some(handle) = handle.downcast::<super::window_view::WorkspaceWindow>() else { continue; };
                        let any: gpui::AnyWindowHandle = handle.into();
                        if any.read::<super::window_view::WorkspaceWindow, _, _>(cx, |entity, _| entity.entity_id() == window_id).unwrap_or(false) {
                            return handle.update(cx, |view, window, cx| view.automation_group_create(create, window, cx))
                                .map_err(|_| Failure { code: -32004, message: "Window closed" })?;
                        }
                    }
                    return Err(Failure { code: -32004, message: "Window not found" });
                }
                if let Some(mutation) = &request.group_mutation {
                    let window_id = target_id(&mutation.window_id)?;
                    for handle in cx.windows() {
                        let Some(handle) = handle.downcast::<super::window_view::WorkspaceWindow>() else { continue; };
                        let any: gpui::AnyWindowHandle = handle.into();
                        if any.read::<super::window_view::WorkspaceWindow, _, _>(cx, |entity, _| entity.entity_id() == window_id).unwrap_or(false) {
                            return handle.update(cx, |view, window, cx| view.automation_group_mutation(mutation, &request.method, window, cx))
                                .map_err(|_| Failure { code: -32004, message: "Window closed" })?;
                        }
                    }
                    return Err(Failure { code: -32004, message: "Window not found" });
                }
                if let Some(merge) = &request.workspace_merge {
                    let source = target_id(&merge.workspace_id)?;
                    let destination = target_id(&merge.target_workspace_id)?;
                    let windows = cx.windows();
                    let target = windows.iter().copied().find(|handle| {
                        handle.downcast::<super::window_view::WorkspaceWindow>()
                            .is_some_and(|handle| handle.read(cx).is_ok_and(|view| view.automation_contains_workspace(destination)))
                    }).ok_or(Failure { code: -32004, message: "Merge destination not found" })?;
                    for handle in windows {
                        let Some(handle) = handle.downcast::<super::window_view::WorkspaceWindow>() else { continue; };
                        if handle.read(cx).is_ok_and(|view| view.automation_contains_workspace(source)) {
                            return handle.update(cx, |view, window, cx| {
                                view.merge_workspace_to_window(source, destination, target, merge.activate, window, cx)
                                    .map_err(|_| Failure { code: -32003, message: "Merge unavailable: workspace busy, destination changed, or layout/terminal limit reached" })?;
                                Ok(json!({"workspace_id":merge.workspace_id,"target_workspace_id":merge.target_workspace_id,"merged":true}))
                            }).unwrap_or(Err(Failure { code: -32004, message: "Workspace window closed" }));
                        }
                    }
                    return Err(Failure { code: -32004, message: "Source workspace not found" });
                }
                if let Some(movement) = &request.workspace_move {
                    let workspace_id = target_id(&movement.workspace_id)?;
                    let window_id = target_id(&movement.window_id)?;
                    let windows = cx.windows();
                    let destination = windows.iter().copied().find(|handle| {
                        handle.read::<super::window_view::WorkspaceWindow, _, _>(cx, |entity, _| entity.entity_id() == window_id).unwrap_or(false)
                    }).ok_or(Failure { code: -32004, message: "Destination window not found" })?;
                    for handle in windows {
                        let Some(handle) = handle.downcast::<super::window_view::WorkspaceWindow>() else { continue; };
                        if handle.read(cx).is_ok_and(|view| view.automation_contains_workspace(workspace_id)) {
                            handle.update(cx, |view, window, cx| view.try_move_workspace_to_window(workspace_id, destination, movement.activate, window, cx))
                                .map_err(|_| Failure { code: -32004, message: "Source window closed" })?
                                .map_err(|_| Failure { code: -32003, message: "Workspace transfer unavailable; source or destination is busy" })?;
                            return Ok(json!({"workspace_id":movement.workspace_id,"window_id":movement.window_id}));
                        }
                    }
                    return Err(Failure { code: -32004, message: "Workspace not found" });
                }
                if let Some(rename) = &request.workspace_rename {
                    let target = target_id(&rename.workspace_id)?;
                    for handle in cx.windows() {
                        let Some(handle) = handle.downcast::<super::window_view::WorkspaceWindow>()
                        else {
                            continue;
                        };
                        if handle
                            .read(cx)
                            .is_ok_and(|view| view.automation_contains_workspace(target))
                        {
                            return handle
                                .update(cx, |view, window, cx| {
                                    view.automation_rename(rename, target, window, cx)
                                })
                                .unwrap_or(Err(Failure {
                                    code: -32004,
                                    message: "Workspace window closed",
                                }));
                        }
                    }
                    return Err(Failure {
                        code: -32004,
                        message: "Workspace not found",
                    });
                }
                if let Some(navigation) = &request.focus_history {
                    let target = target_id(&navigation.window_id)?;
                    for handle in cx.windows() {
                        let Some(handle) = handle.downcast::<super::window_view::WorkspaceWindow>() else { continue; };
                        let any: gpui::AnyWindowHandle = handle.into();
                        if any.read::<super::window_view::WorkspaceWindow, _, _>(cx, |entity, _| entity.entity_id() == target).unwrap_or(false) {
                            return handle.update(cx, |view, window, cx| view.automation_focus_history(navigation, window, cx))
                                .map_err(|_| Failure { code: -32004, message: "Window closed" })?;
                        }
                    }
                    return Err(Failure { code: -32004, message: "Window not found" });
                }
                if let Some(selection) = &request.selection {
                    let target = target_id(&selection.workspace_id)?;
                    for handle in cx.windows() {
                        let Some(handle) = handle.downcast::<super::window_view::WorkspaceWindow>()
                        else {
                            continue;
                        };
                        if handle
                            .read(cx)
                            .is_ok_and(|view| view.automation_contains_workspace(target))
                        {
                            return handle
                                .update(cx, |view, window, cx| {
                                    view.automation_select(selection, target, window, cx)
                                })
                                .unwrap_or(Err(Failure {
                                    code: -32004,
                                    message: "Workspace window closed",
                                }));
                        }
                    }
                    return Err(Failure {
                        code: -32004,
                        message: "Workspace not found",
                    });
                }
                if let Some(read) = &request.surface_read {
                    let target = target_id(&read.surface_id)?;
                    for handle in cx.windows() {
                        let Some(handle) = handle.downcast::<super::window_view::WorkspaceWindow>() else { continue; };
                        if let Ok(view) = handle.read(cx) {
                            if view.automation_surface_target(target, cx).is_some() {
                                return view.automation_read_text(read, target, cx);
                            }
                        }
                    }
                    return Err(Failure { code: -32004, message: "Surface not found" });
                }
                if let Some(input) = &request.surface_text {
                    return send_surface_input(input, cx);
                }
                if let Some(focus) = &request.surface_focus {
                    let target = target_id(&focus.surface_id)?;
                    for handle in cx.windows() {
                        let Some(handle) = handle.downcast::<super::window_view::WorkspaceWindow>()
                        else {
                            continue;
                        };
                        if handle
                            .read(cx)
                            .is_ok_and(|view| view.automation_surface_target(target, cx).is_some())
                        {
                            return handle
                                .update(cx, |view, window, cx| {
                                    view.automation_focus_surface(focus, target, window, cx)
                                })
                                .unwrap_or(Err(Failure {
                                    code: -32004,
                                    message: "Surface window closed",
                                }));
                        }
                    }
                    return Err(Failure {
                        code: -32004,
                        message: "Surface not found",
                    });
                }
                if let Some(swap) = &request.pane_swap {
                    let target = target_id(&swap.pane_id)?;
                    for handle in cx.windows() {
                        let Some(handle) = handle.downcast::<super::window_view::WorkspaceWindow>() else { continue; };
                        if handle.read(cx).is_ok_and(|view| view.automation_pane_target(target, cx).is_some()) {
                            return handle.update(cx, |view, _, cx| view.automation_swap_panes(swap, target, cx))
                                .map_err(|_| Failure { code: -32004, message: "Pane window closed" })?;
                        }
                    }
                    return Err(Failure { code: -32004, message: "Pane not found" });
                }
                if let Some(resize) = &request.pane_resize {
                    let target = target_id(&resize.pane_id)?;
                    for handle in cx.windows() {
                        let Some(handle) = handle.downcast::<super::window_view::WorkspaceWindow>() else { continue; };
                        if handle.read(cx).is_ok_and(|view| view.automation_pane_target(target, cx).is_some()) {
                            return handle.update(cx, |view, _, cx| view.automation_resize_pane(resize, target, cx))
                                .map_err(|_| Failure { code: -32004, message: "Pane window closed" })?;
                        }
                    }
                    return Err(Failure { code: -32004, message: "Pane not found" });
                }
                if let Some(zoom) = &request.pane_zoom {
                    let target = target_id(&zoom.pane_id)?;
                    for handle in cx.windows() {
                        let Some(handle) = handle.downcast::<super::window_view::WorkspaceWindow>()
                        else {
                            continue;
                        };
                        if handle
                            .read(cx)
                            .is_ok_and(|view| view.automation_pane_target(target, cx).is_some())
                        {
                            return handle
                                .update(cx, |view, window, cx| {
                                    view.automation_zoom_pane(zoom, target, window, cx)
                                })
                                .unwrap_or(Err(Failure {
                                    code: -32004,
                                    message: "Pane window closed",
                                }));
                        }
                    }
                    return Err(Failure {
                        code: -32004,
                        message: "Pane not found",
                    });
                }
                if matches!(request.method.as_str(), "pane.list" | "surface.list" | "notification.list") {
                    let surfaces = request.method != "pane.list";
                    let mut items = Vec::new();
                    for handle in cx.windows() {
                        if let Ok(mut values) = handle.read::<super::window_view::WorkspaceWindow, _, _>(cx, |entity, cx| {
                            let window_id = entity.entity_id().to_string();
                            if request.list_filter.window_id.as_ref().is_some_and(|id| id != &window_id) { return Vec::new(); }
                            entity.read(cx).automation_children(window_id, surfaces, request.list_filter.workspace_id.as_deref(), request.method == "notification.list", cx)
                        }) { items.append(&mut values); }
                    }
                    if request.method == "notification.list" {
                        let messages = items.into_iter().flat_map(|item| {
                            item["notifications"].as_array().into_iter().flatten().map(|message| {
                                json!({"surface_id":item["id"],"workspace_id":item["workspace_id"],"window_id":item["window_id"],
                                    "revision":message["revision"],"title":message["title"],"body":message["body"],"unread":message["unread"]})
                            }).collect::<Vec<_>>()
                        }).collect();
                        return Ok(array_result("notifications", messages));
                    }
                    return Ok(array_result(
                        if surfaces { "surfaces" } else { "panes" },
                        items,
                    ));
                }
                let mut windows = Vec::new();
                for handle in cx.windows() {
                    if let Ok(Some(value)) = handle.read::<super::window_view::WorkspaceWindow, _, _>(
                        cx,
                        |entity, cx| {
                            let window_id = entity.entity_id().to_string();
                            if request.list_filter.window_id.as_ref().is_some_and(|id| id != &window_id) { return None; }
                            Some(entity.read(cx).automation_snapshot(window_id, request.method == "workspace.list", cx))
                        },
                    ) {
                        windows.push(value);
                    }
                }
                if request.method == "window.list" {
                    for window in &mut windows {
                        window.as_object_mut().unwrap().remove("workspaces");
                    }
                    Ok(array_result("windows", windows))
                } else {
                    let workspaces: Vec<Value> = windows
                        .into_iter()
                        .flat_map(|mut window| match window["workspaces"].take() {
                            Value::Array(workspaces) => workspaces,
                            _ => Vec::new(),
                        })
                        .collect();
                    Ok(array_result("workspaces", workspaces))
                }
            });
            match result {
                Ok(result) => {
                    let _ = request.reply.try_send(result);
                }
                Err(_) => break,
            }
        }
    })
    .detach();
    Ok(())
}

fn error(id: Value, code: i32, message: &str) -> Value {
    json!({"jsonrpc":"2.0", "id":id, "error":{"code":code,"message":message}})
}

fn dispatch_inner(
    bytes: &[u8],
    live: Option<&async_channel::Sender<LiveRequest>>,
) -> Option<Value> {
    let value: Value = match serde_json::from_slice(bytes) {
        Ok(value) => value,
        Err(_) => return Some(error(Value::Null, -32700, "Parse error")),
    };
    let Some(request) = value.as_object() else {
        return Some(error(Value::Null, -32600, "Invalid Request"));
    };
    let id = request.get("id");
    let valid_id = id.is_none_or(|id| {
        id.is_null() || id.is_number() || id.as_str().is_some_and(|text| text.len() <= 128)
    });
    let method = request.get("method").and_then(Value::as_str);
    if !valid_id
        || request.get("jsonrpc").and_then(Value::as_str) != Some("2.0")
        || method.is_none_or(|method| method.is_empty() || method.len() > 128)
    {
        return Some(error(Value::Null, -32600, "Invalid Request"));
    }
    let method = method.unwrap();
    let notification = id.is_none();
    // Read-only notifications have no observable effect. Selection notifications
    // still execute, but must never produce success or error responses.
    if notification
        && !matches!(
            method,
            "buffer.set" | "buffer.delete" | "buffer.paste" | "buffer.rename" | "workspace.select"
                | "window.focus_history"
                | "surface.focus"
                | "surface.send_text"
                | "surface.send_key"
                | "notification.send"
                | "notification.dismiss"
                | "notification.mark_read"
                | "pane.zoom.set"
                | "pane.resize"
                | "pane.swap"
                | "workspace.equalize"
                | "pane.split"
                | "window.create"
                | "workspace.create"
                | "workspace.open"
                | "workspace.close"
                | "workspace.rename"
                | "workspace.group.create"
                | "workspace.group.update"
                | "workspace.group.ungroup"
                | "workspace.group.add"
                | "workspace.group.remove"
                | "workspace.group.set_anchor"
                | "workspace.group.new_workspace"
                | "workspace.move_to_window"
                | "workspace.merge"
                | "surface.move_to_workspace"
                | "system.quit"
        )
    {
        return None;
    }
    let id = id.cloned().unwrap_or(Value::Null);
    let respond = |response| if notification { None } else { Some(response) };
    let methods = if live.is_some() {
        LIVE_METHODS
    } else {
        METHODS
    };
    if !methods.contains(&method) {
        return respond(error(id, -32601, "Method not found"));
    }
    let events_read = if method == "events.read" {
        match serde_json::from_value::<super::events::Read>(request.get("params").cloned().unwrap_or_else(|| json!({}))) {
            Ok(read) if read.valid() => Some(read),
            _ => return respond(error(id, -32602, "Expected valid cursor/surface/kinds, limit 1–128 and wait_ms 0–3000")),
        }
    } else { None };
    let buffer_command = match super::copy_buffers::parse(method, request.get("params")) {
        Ok(command) => command,
        Err(message) => return respond(error(id, -32602, message)),
    };
    let is_filtered_list = matches!(method, "workspace.list" | "pane.list" | "surface.list" | "notification.list");
    let list_filter = if is_filtered_list {
        let params = request.get("params").cloned().unwrap_or_else(|| json!({}));
        // Keep the established empty-array request form compatible.
        let params = if params.as_array().is_some_and(|v| v.is_empty()) { json!({}) } else { params };
        match serde_json::from_value::<ListFilter>(params) {
            Ok(filter) if filter.window_id.as_ref().is_none_or(|id| target_id(id).is_ok())
                && filter.workspace_id.as_ref().is_none_or(|id| target_id(id).is_ok())
                && (method != "workspace.list" || filter.workspace_id.is_none()) => filter,
            _ => return respond(error(id, -32602, "Expected optional window_id and, for pane/surface lists, workspace_id")),
        }
    } else { ListFilter::default() };
    let focus_history = if method == "window.focus_history" {
        match serde_json::from_value::<FocusHistory>(request.get("params").cloned().unwrap_or(Value::Null)) {
            Ok(navigation) if target_id(&navigation.window_id).is_ok() => Some(navigation),
            _ => return respond(error(id, -32602, "Expected window_id, direction back|forward and optional activate boolean")),
        }
    } else { None };
    let selection = if method == "workspace.select" {
        match request
            .get("params")
            .cloned()
            .and_then(|params| serde_json::from_value::<Selection>(params).ok())
        {
            Some(selection)
                if !selection.workspace_id.is_empty() && selection.workspace_id.len() <= 128 =>
            {
                Some(selection)
            }
            _ => {
                return respond(error(
                    id,
                    -32602,
                    "Expected workspace_id string and optional activate boolean",
                ));
            }
        }
    } else {
        None
    };
    let surface_focus = if method == "surface.focus" {
        match request
            .get("params")
            .cloned()
            .and_then(|params| serde_json::from_value::<SurfaceFocus>(params).ok())
        {
            Some(focus) if !focus.surface_id.is_empty() && focus.surface_id.len() <= 128 => {
                Some(focus)
            }
            _ => {
                return respond(error(
                    id,
                    -32602,
                    "Expected surface_id string and optional activate boolean",
                ));
            }
        }
    } else {
        None
    };
    let pane_swap = if method == "pane.swap" {
        match request.get("params").cloned().and_then(|params| serde_json::from_value::<PaneSwap>(params).ok()) {
            Some(swap) if target_id(&swap.pane_id).is_ok() && target_id(&swap.other_pane_id).is_ok() => Some(swap),
            _ => return respond(error(id, -32602, "Expected pane_id and other_pane_id")),
        }
    } else { None };
    let pane_resize = if method == "pane.resize" {
        match request.get("params").cloned().and_then(|params| serde_json::from_value::<PaneResize>(params).ok()) {
            Some(resize) if target_id(&resize.pane_id).is_ok() => Some(resize),
            _ => return respond(error(id, -32602, "Expected pane_id and direction left|right|up|down")),
        }
    } else { None };
    let pane_split = if method == "pane.split" {
        match request.get("params").cloned().and_then(|params| serde_json::from_value::<PaneSplit>(params).ok()) {
            Some(split) if target_id(&split.pane_id).is_ok()
                && split.ssh_destination.as_ref().is_none_or(|destination| crate::ssh::command(destination).is_ok())
                && split.cwd.as_ref().is_none_or(|path| path.is_absolute() && path.as_os_str().len() <= 8192) => Some(split),
            _ => return respond(error(id, -32602, "Expected pane_id, direction left|right|up|down, optional SSH destination, absolute cwd, focus and activate booleans")),
        }
    } else { None };
    let pane_zoom = if method == "pane.zoom.set" {
        match request
            .get("params")
            .cloned()
            .and_then(|params| serde_json::from_value::<PaneZoom>(params).ok())
        {
            Some(zoom) if !zoom.pane_id.is_empty() && zoom.pane_id.len() <= 128 => Some(zoom),
            _ => {
                return respond(error(
                    id,
                    -32602,
                    "Expected pane_id string and zoomed boolean",
                ));
            }
        }
    } else {
        None
    };
    let window_create = if method == "window.create" {
        match serde_json::from_value::<WindowCreate>(
            request.get("params").cloned().unwrap_or_else(|| json!({})),
        ) {
            Ok(create) => Some(create),
            Err(_) => return respond(error(id, -32602, "Expected optional activate boolean")),
        }
    } else {
        None
    };
    let workspace_equalize = if method == "workspace.equalize" {
        match serde_json::from_value::<WorkspaceTarget>(request.get("params").cloned().unwrap_or_else(|| json!({}))) {
            Ok(close) if target_id(&close.workspace_id).is_ok() => Some(close),
            _ => return respond(error(id, -32602, "Expected workspace_id")),
        }
    } else { None };
    let workspace_close = if method == "workspace.close" {
        match serde_json::from_value::<WorkspaceTarget>(request.get("params").cloned().unwrap_or_else(|| json!({}))) {
            Ok(close) if target_id(&close.workspace_id).is_ok() => Some(close),
            _ => return respond(error(id, -32602, "Expected workspace_id")),
        }
    } else { None };
    let surface_key = if method == "surface.send_key" {
        match serde_json::from_value::<SurfaceKey>(request.get("params").cloned().unwrap_or_else(|| json!({}))) {
            Ok(input) if target_id(&input.surface_id).is_ok() && input.key.len() <= 32
                && !input.key.chars().any(char::is_control)
                && rmux_terminal::keyboard::key_identity(&input.key).is_some() => Some(input),
            _ => return respond(error(id, -32602, "Expected surface_id, supported key and optional control/alt/shift booleans")),
        }
    } else { None };
    let notification_change = if matches!(method, "notification.send" | "notification.dismiss" | "notification.mark_read") {
        match serde_json::from_value::<NotificationChange>(request.get("params").cloned().unwrap_or_else(|| json!({}))) {
            Ok(mut change) if target_id(&change.surface_id).is_ok() => {
                change.mark_read = method == "notification.mark_read";
                if matches!(method, "notification.dismiss" | "notification.mark_read") {
                    if change.revision.is_none() || change.title.is_some() || change.body.is_some() { return respond(error(id, -32602, "Expected surface_id and revision")); }
                } else {
                    if change.revision.is_some() || change.title.as_ref().is_some_and(|s| s.len() > 1024) || change.body.as_ref().is_some_and(|s| s.len() > 4096) { return respond(error(id, -32602, "Notification title/body exceeds limits or unexpected revision")); }
                    change.title = change.title.map(|s| s.chars().filter(|c| !c.is_control()).take(256).collect());
                    change.body = change.body.map(|s| s.chars().filter(|c| !c.is_control()).take(1024).collect());
                    if change.title.as_ref().is_none_or(|s| s.trim().is_empty()) && change.body.as_ref().is_none_or(|s| s.trim().is_empty()) { return respond(error(id, -32602, "Notification must contain title or body")); }
                }
                Some(change)
            }
            _ => return respond(error(id, -32602, "Expected notification surface_id and content or revision")),
        }
    } else { None };
    let workspace_create = if matches!(method, "workspace.create" | "workspace.open") {
        match serde_json::from_value::<WorkspaceCreate>(request.get("params").cloned().unwrap_or_else(|| json!({}))) {
            Ok(mut create) if (target_id(&create.window_id).is_ok() || (method == "workspace.open" && create.window_id.is_empty()))
                && (method != "workspace.open" || create.name.is_some())
                && create.ssh_destination.as_ref().is_none_or(|destination| super::ssh::command(destination).is_ok())
                && create.cwd.as_ref().is_none_or(|path| path.is_absolute())
                && create.description.as_ref().is_none_or(|text| text.len() <= 4096 && !text.contains('\0')) => {
                if let Some(name) = &create.name {
                    let Some(name) = super::window_view::normalized_workspace_name(name) else { return respond(error(id, -32602, "Workspace name must not be empty")); };
                    create.name = Some(name);
                }
                create.reuse = method == "workspace.open";
                if create.reuse && request.get("params").and_then(|params| params.get("focus")).is_none() { create.focus = true; }
                Some(create)
            }
            _ => return respond(error(id, -32602, "Expected window_id (optional for workspace.open), name (required for workspace.open), optional valid ssh_destination, name, description, absolute cwd, focus and activate booleans")),
        }
    } else { None };
    let surface_memory = if method == "surface.memory_usage" {
        match request.get("params").cloned().and_then(|params| serde_json::from_value::<SurfaceMemory>(params).ok()) {
            Some(sample) if !sample.surface_id.is_empty() && sample.surface_id.len() <= 128 => Some(sample),
            _ => return respond(error(id, -32602, "Expected surface_id string")),
        }
    } else { None };
    let group_create = if method == "workspace.group.create" {
        let Some(mut create) = request.get("params").cloned().and_then(|params| serde_json::from_value::<GroupCreate>(params).ok()) else {
            return respond(error(id, -32602, "Expected window_id, name and optional workspace_ids/external_id"));
        };
        let Some(name) = super::window_view::normalized_workspace_name(&create.name) else { return respond(error(id, -32602, "Group name must not be empty")); };
        create.name = name;
        if create.window_id.is_empty() || create.window_id.len() > 128
            || create.cwd.as_ref().is_some_and(|path| !path.is_absolute() || path.as_os_str().len() > 8192)
            || create.workspace_ids.as_ref().is_some_and(|ids| ids.is_empty() || ids.len() > 64 || ids.iter().enumerate().any(|(index, id)| id.is_empty() || id.len() > 128 || ids[..index].contains(id)))
            || create.external_id.as_ref().is_some_and(|id| id.trim().is_empty() || id.len() > 1024 || id.chars().any(char::is_control)) {
            return respond(error(id, -32602, "Invalid group creation parameters"));
        }
        Some(create)
    } else { None };
    let group_mutation = if matches!(method, "workspace.group.update" | "workspace.group.ungroup" | "workspace.group.add" | "workspace.group.remove" | "workspace.group.set_anchor" | "workspace.group.new_workspace") {
        let Some(mut change) = request.get("params").cloned().and_then(|params| serde_json::from_value::<GroupMutation>(params).ok()) else {
            return respond(error(id, -32602, "Expected window_id, group_id and optional name, collapsed, pinned"));
        };
        let valid_group = change.group_id.strip_prefix("workspace_group:").and_then(|id| id.parse::<u64>().ok())
            .is_some_and(|id| id > 0 && format!("workspace_group:{id}") == change.group_id);
        if !valid_group || change.window_id.is_empty() || change.window_id.len() > 128 {
            return respond(error(id, -32602, "Invalid window_id or group_id"));
        }
        if change.cwd.as_ref().is_some_and(|path| !path.is_absolute() || path.as_os_str().len() > 8192)
            || (change.cwd.is_some() && change.clear_cwd == Some(true)) {
            return respond(error(id, -32602, "Invalid or conflicting group directory"));
        }
        let has_changes = change.cwd.is_some() || change.clear_cwd.is_some() || change.name.is_some() || change.collapsed.is_some() || change.pinned.is_some();
        if has_changes != (method == "workspace.group.update") {
            return respond(error(id, -32602, "Update requires changes; ungroup accepts only IDs"));
        }
        if change.placement.is_some() && method != "workspace.group.new_workspace" {
            return respond(error(id, -32602, "placement is only supported for new_workspace"));
        }
        if change.remove_generated_anchor.is_some() && method != "workspace.group.ungroup" {
            return respond(error(id, -32602, "remove_generated_anchor is only supported for ungroup"));
        }
        let membership = matches!(method, "workspace.group.add" | "workspace.group.remove" | "workspace.group.set_anchor");
        if change.workspace_id.is_some() != membership || change.workspace_id.as_ref().is_some_and(|id| id.is_empty() || id.len() > 128) {
            return respond(error(id, -32602, "Membership methods require workspace_id; other methods do not accept it"));
        }
        if let Some(name) = &change.name {
            let Some(name) = super::window_view::normalized_workspace_name(name) else { return respond(error(id, -32602, "Group name must not be empty")); };
            change.name = Some(name);
        }
        Some(change)
    } else { None };
    let workspace_rename = if method == "workspace.rename" {
        match request
            .get("params")
            .cloned()
            .and_then(|params| serde_json::from_value::<WorkspaceRename>(params).ok())
        {
            Some(mut rename)
                if !rename.workspace_id.is_empty() && rename.workspace_id.len() <= 128 =>
            {
                let Some(name) = super::window_view::normalized_workspace_name(&rename.name) else {
                    return respond(error(id, -32602, "Workspace name must not be empty"));
                };
                rename.name = name;
                Some(rename)
            }
            _ => return respond(error(id, -32602, "Expected workspace_id and name strings")),
        }
    } else {
        None
    };
    let workspace_merge = if method == "workspace.merge" {
        match request.get("params").cloned().and_then(|params| serde_json::from_value::<WorkspaceMerge>(params).ok()) {
            Some(merge) if !merge.workspace_id.is_empty() && merge.workspace_id.len() <= 128
                && !merge.target_workspace_id.is_empty() && merge.target_workspace_id.len() <= 128
                && merge.workspace_id != merge.target_workspace_id => Some(merge),
            _ => return respond(error(id, -32602, "Expected distinct workspace_id and target_workspace_id strings, and optional activate boolean")),
        }
    } else { None };
    let workspace_move = if method == "workspace.move_to_window" {
        match request
            .get("params")
            .cloned()
            .and_then(|params| serde_json::from_value::<WorkspaceMove>(params).ok())
        {
            Some(movement)
                if !movement.workspace_id.is_empty()
                    && movement.workspace_id.len() <= 128
                    && !movement.window_id.is_empty()
                    && movement.window_id.len() <= 128 =>
            {
                Some(movement)
            }
            _ => {
                return respond(error(
                    id,
                    -32602,
                    "Expected workspace_id, window_id and optional activate boolean",
                ));
            }
        }
    } else {
        None
    };
    let surface_text = if method == "surface.send_text" {
        match request
            .get("params")
            .cloned()
            .and_then(|params| serde_json::from_value::<SurfaceText>(params).ok())
        {
            Some(input)
                if !input.surface_id.is_empty()
                    && input.surface_id.len() <= 128
                    && !input.text.is_empty()
                    && input.text.len() <= 16 * 1024 =>
            {
                Some(input)
            }
            _ => {
                return respond(error(
                    id,
                    -32602,
                    "Expected surface_id and nonempty text up to 16 KiB UTF-8",
                ));
            }
        }
    } else {
        None
    };
    let surface_read = if method == "surface.read_text" {
        match request
            .get("params")
            .cloned()
            .and_then(|params| serde_json::from_value::<SurfaceRead>(params).ok())
        {
            Some(read)
                if !read.surface_id.is_empty()
                    && read.surface_id.len() <= 128
                    && (!read.preserve_wraps || read.scope == ReadScope::Selection)
                    && read.lines.is_none_or(|lines| read.scope == ReadScope::History && (1..=65536).contains(&lines))
                    && (1..=1024 * 1024).contains(&read.max_bytes) =>
            {
                Some(read)
            }
            _ => {
                return respond(error(
                    id,
                    -32602,
                    "Expected surface_id, optional scope (viewport, history or selection), max_bytes from 1 to 1048576; preserve_wraps requires selection; lines requires history and 1 to 65536",
                ));
            }
        }
    } else {
        None
    };
    let surface_move = if method == "surface.move_to_workspace" {
        match request.get("params").cloned().and_then(|params| serde_json::from_value::<SurfaceMove>(params).ok()) {
            Some(movement) if !movement.surface_id.is_empty() && movement.surface_id.len() <= 128
                && !movement.workspace_id.is_empty() && movement.workspace_id.len() <= 128 => Some(movement),
            _ => return respond(error(id, -32602, "Expected surface_id, workspace_id and optional activate boolean")),
        }
    } else { None };
    if buffer_command.is_none()
        && surface_memory.is_none()
        && group_create.is_none()
        && group_mutation.is_none()
        && selection.is_none()
        && surface_move.is_none()
        && surface_read.is_none()
        && surface_text.is_none()
        && workspace_move.is_none()
        && workspace_merge.is_none()
        && workspace_rename.is_none()
        && workspace_close.is_none()
        && workspace_equalize.is_none()
        && surface_key.is_none()
        && notification_change.is_none()
        && window_create.is_none()
        && workspace_create.is_none()
        && pane_zoom.is_none()
        && pane_split.is_none()
        && pane_resize.is_none()
        && pane_swap.is_none()
        && focus_history.is_none()
        && surface_focus.is_none()
        && events_read.is_none()
        && !is_filtered_list
        && request.get("params").is_some_and(|params| {
            !(params.as_object().is_some_and(|params| params.is_empty())
                || params.as_array().is_some_and(|params| params.is_empty()))
        })
    {
        return respond(error(id, -32602, "Expected empty params"));
    }
    let result = match method {
        "system.ping" => json!({"pong":true}),
        "system.capabilities" => json!({
            "application":"rmux", "version":env!("CARGO_PKG_VERSION"),
            "mode":if live.is_some() { "live" } else { "standalone" },
            "live_workspace_access":live.is_some(), "read_only":live.is_none(),
            "methods":methods, "max_request_bytes":MAX_REQUEST_BYTES,
            "events":live.is_some().then(super::events::capabilities),
        }),
        _ => {
            let (reply, receiver) = std::sync::mpsc::sync_channel(1);
            let pending = Arc::new(Pending::new());
            if live
                .unwrap()
                .try_send(LiveRequest {
                    method: method.into(),
                    events_read,
                    buffer_command,
                    list_filter,
                    notification_change,
                    surface_key,
                    workspace_close,
                    workspace_equalize,
                    selection,
                    focus_history,
                    surface_focus,
                    surface_text,
                    surface_read,
                    surface_memory,
                    surface_move,
                    pane_zoom,
                    pane_split,
                    pane_resize,
                    pane_swap,
                    window_create,
                    workspace_create,
                    workspace_rename,
                    group_mutation,
                    group_create,
                    workspace_move,
                    workspace_merge,
                    pending: pending.clone(),
                    reply,
                })
                .is_err()
            {
                return respond(error(id, -32000, "UI unavailable or request queue full"));
            }
            match receiver.recv_timeout(pending.deadline.saturating_duration_since(Instant::now()))
            {
                Ok(Ok(result)) => result,
                Ok(Err(failure)) => return respond(error(id, failure.code, failure.message)),
                Err(_) if pending.cancel() => {
                    return respond(error(id, -32001, "Request canceled before execution"));
                }
                Err(_) => {
                    return respond(error(
                        id,
                        -32002,
                        "Reply unavailable after execution started; outcome unknown",
                    ));
                }
            }
        }
    };
    let mut response = json!({"jsonrpc":"2.0", "id":id});
    response["result"] = result;
    respond(response)
}

pub fn serve(input: impl BufRead, output: impl Write) -> io::Result<()> {
    serve_inner(input, output, None)
}

fn serve_inner(
    mut input: impl BufRead,
    mut output: impl Write,
    live: Option<&async_channel::Sender<LiveRequest>>,
) -> io::Result<()> {
    let mut bytes = Vec::new();
    loop {
        bytes.clear();
        let count = std::io::Read::take(&mut input, MAX_REQUEST_BYTES as u64 + 1)
            .read_until(b'\n', &mut bytes)?;
        if count == 0 {
            return Ok(());
        }
        if count > MAX_REQUEST_BYTES {
            serde_json::to_writer(
                &mut output,
                &error(Value::Null, -32600, "Request exceeds 64 KiB"),
            )?;
            output.write_all(b"\n")?;
            output.flush()?;
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "RPC request exceeds 64 KiB",
            ));
        }
        if let Some(response) = dispatch_inner(&bytes, live) {
            serde_json::to_writer(&mut output, &response)?;
            output.write_all(b"\n")?;
            output.flush()?;
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn events_read_validates_before_queueing_and_preserves_cursor() {
        let (sender, receiver) = async_channel::bounded::<LiveRequest>(1);
        for params in [json!({"limit":0}), json!({"limit":129}), json!({"after":"bad"}), json!({"unknown":true})] {
            let request = json!({"jsonrpc":"2.0","id":1,"method":"events.read","params":params});
            assert_eq!(dispatch_inner(&serde_json::to_vec(&request).unwrap(), Some(&sender)).unwrap()["error"]["code"], -32602);
            assert!(receiver.try_recv().is_err());
        }
        let worker = std::thread::spawn(move || {
            let request = receiver.recv_blocking().unwrap();
            let read = request.events_read.unwrap();
            assert_eq!(read.after.as_deref(), Some("123-456:7"));
            assert_eq!(read.limit, 16);
            request.reply.send(Ok(json!({"events":[],"cursor":"123-456:7","has_more":false,"gap":null}))).unwrap();
        });
        let request = br#"{"jsonrpc":"2.0","id":1,"method":"events.read","params":{"after":"123-456:7","limit":16}}"#;
        assert_eq!(dispatch_inner(request, Some(&sender)).unwrap()["result"]["cursor"], "123-456:7");
        worker.join().unwrap();
        assert_eq!(dispatch_inner(request, None).unwrap()["error"]["code"], -32601);
    }
    #[test]
    fn workspace_merge_validates_before_queueing_and_routes_once() {
        let (sender, receiver) = async_channel::bounded::<LiveRequest>(1);
        for params in [json!({}), json!({"workspace_id":"a","target_workspace_id":"a"}),
            json!({"workspace_id":"a","target_workspace_id":"b","activate":"yes"}),
            json!({"workspace_id":"a","target_workspace_id":"b","force":true})] {
            let response = dispatch_inner(&serde_json::to_vec(&json!({"jsonrpc":"2.0","id":1,"method":"workspace.merge","params":params})).unwrap(), Some(&sender)).unwrap();
            assert_eq!(response["error"]["code"], -32602);
            assert!(receiver.is_empty());
        }
        let worker = std::thread::spawn(move || {
            let request = receiver.recv_blocking().unwrap();
            assert!(request.pending.start());
            let merge = request.workspace_merge.unwrap();
            assert_eq!(merge.workspace_id, "a");
            assert_eq!(merge.target_workspace_id, "b");
            assert!(!merge.activate);
            request.reply.send(Ok(json!({"routed":true}))).unwrap();
        });
        let response = dispatch_inner(&serde_json::to_vec(&json!({"jsonrpc":"2.0","id":1,"method":"workspace.merge","params":{"workspace_id":"a","target_workspace_id":"b"}})).unwrap(), Some(&sender)).unwrap();
        assert_eq!(response["result"]["routed"], true);
        worker.join().unwrap();
    }
    #[test]
    fn buffer_requests_route_through_the_live_queue() {
        let (sender, receiver) = async_channel::bounded::<LiveRequest>(1);
        let worker = std::thread::spawn(move || {
            let mut store = super::super::copy_buffers::Store::default();
            for _ in 0..5 {
                let request = receiver.recv_blocking().unwrap();
                assert!(request.pending.start());
                let result = store.execute(request.buffer_command.as_ref().unwrap())
                    .map_err(|message| Failure { code: -32008, message });
                request.reply.send(result).unwrap();
            }
        });
        for (method, params, key, expected) in [
            ("buffer.set", json!({"text":"世界"}), "bytes", json!(6)),
            ("buffer.get", json!({}), "text", json!("世界")),
            ("buffer.list", json!({}), "bytes", json!(6)),
            ("buffer.rename", json!({"new_name":"renamed 世界"}), "changed", json!(true)),
            ("buffer.delete", json!({"name":"renamed 世界"}), "deleted", json!(true)),
        ] {
            let request = json!({"jsonrpc":"2.0","id":1,"method":method,"params":params});
            let response = dispatch_inner(&serde_json::to_vec(&request).unwrap(), Some(&sender)).unwrap();
            assert_eq!(response["result"][key], expected, "{response}");
        }
        worker.join().unwrap();
    }
    #[test]
    fn invalid_group_mutations_never_queue_ui_work() {
        let (sender, receiver) = async_channel::bounded(1);
        for (method, params) in [
            ("workspace.group.create", json!({"window_id":"42","name":"Build","cwd":"relative"})),
            ("workspace.group.create", json!({"window_id":"42","name":"Build","workspace_ids":[]})),
            ("workspace.group.create", json!({"window_id":"42","name":"Build","workspace_ids":["43","43"]})),
            ("workspace.group.update", json!({"window_id":"42","group_id":"workspace_group:1"})),
            ("workspace.group.update", json!({"window_id":"42","group_id":"workspace_group:1","cwd":"relative"})),
            ("workspace.group.update", json!({"window_id":"42","group_id":"workspace_group:1","cwd":std::env::temp_dir(),"clear_cwd":true})),
            ("workspace.group.new_workspace", json!({"window_id":"42","group_id":"workspace_group:1","clear_cwd":true})),
            ("workspace.group.update", json!({"window_id":"42","group_id":"workspace_group:01","pinned":true})),
            ("workspace.group.update", json!({"window_id":"42","group_id":"workspace_group:1","collapsed":"yes"})),
            ("workspace.group.update", json!({"window_id":"42","group_id":"workspace_group:1","name":" "})),
            ("workspace.group.ungroup", json!({"window_id":"42","group_id":"workspace_group:1","pinned":true})),
            ("workspace.group.add", json!({"window_id":"42","group_id":"workspace_group:1"})),
            ("workspace.group.remove", json!({"window_id":"42","group_id":"workspace_group:1","workspace_id":""})),
            ("workspace.group.set_anchor", json!({"window_id":"42","group_id":"workspace_group:1","workspace_id":"43","name":"invalid extra"})),
            ("workspace.group.update", json!({"window_id":"42","group_id":"workspace_group:1","workspace_id":"43","pinned":true})),
        ] {
            let request = json!({"jsonrpc":"2.0","id":1,"method":method,"params":params});
            assert_eq!(dispatch_inner(&serde_json::to_vec(&request).unwrap(), Some(&sender)).unwrap()["error"]["code"], -32602);
        }
        assert!(receiver.try_recv().is_err());
    }
    use super::*;
    #[test]
    fn notification_mutations_validate_before_queueing_and_keep_revision_precision() {
        // Use a canonical generation-bearing ID so invalid-field cases exercise
        // their own validation rather than failing surface ID validation first.
        let (sender, receiver) = async_channel::bounded::<LiveRequest>(1);
        for method in ["notification.mark_read", "notification.dismiss"] {
            for params in [
                json!({"surface_id":"4294967297"}),
                json!({"surface_id":"4294967297","revision":null}),
                json!({"surface_id":"4294967297","revision":-1}),
                json!({"surface_id":"4294967297","revision":1.5}),
                json!({"surface_id":"4294967297","revision":"1"}),
                json!({"surface_id":"4294967297","revision":1,"title":"unexpected"}),
                json!({"surface_id":"4294967297","revision":1,"body":"unexpected"}),
                json!({"surface_id":"4294967297","revision":1,"mark_read":true}),
                json!({"surface_id":"04294967297","revision":1}),
            ] {
                let request = json!({"jsonrpc":"2.0","id":9,"method":method,"params":params});
                let response = dispatch_inner(&serde_json::to_vec(&request).unwrap(), Some(&sender)).unwrap();
                assert_eq!(response["error"]["code"], -32602, "{request}: {response}");
                assert!(receiver.try_recv().is_err(), "invalid mutation reached UI queue");
            }
        }
        let worker = std::thread::spawn(move || {
            for mark_read in [true, false] {
                let request = receiver.recv_blocking().unwrap();
                assert!(request.pending.start());
                let change = request.notification_change.unwrap();
                assert_eq!(change.mark_read, mark_read);
                assert_eq!(change.revision, Some(u64::MAX));
                assert_eq!(change.surface_id, "4294967297");
                assert!(change.title.is_none() && change.body.is_none());
                request.reply.send(Ok(json!({"revision":change.revision}))).unwrap();
            }
        });
        for method in ["notification.mark_read", "notification.dismiss"] {
            let request = json!({"jsonrpc":"2.0","id":"notification-test","method":method,
                "params":{"surface_id":"4294967297","revision":u64::MAX}});
            let response = dispatch_inner(&serde_json::to_vec(&request).unwrap(), Some(&sender)).unwrap();
            assert_eq!(response["id"], "notification-test");
            assert_eq!(response["result"]["revision"].as_u64(), Some(u64::MAX), "{response}");
        }
        worker.join().unwrap();
    }

    #[test]
    fn ssh_split_rejects_invalid_destinations_before_queueing() {
        let (sender, receiver) = async_channel::bounded::<LiveRequest>(1);
        for destination in ["", "-oProxyCommand=bad", "host command", "ssh://host:0", "ssh://host/path"] {
            let request = json!({"jsonrpc":"2.0","id":1,"method":"pane.split", "params":{"pane_id":"4294967297","direction":"right","ssh_destination":destination}});
            assert_eq!(dispatch_inner(&serde_json::to_vec(&request).unwrap(), Some(&sender)).unwrap()["error"]["code"], -32602);
            assert!(receiver.is_empty());
        }
    }

    #[test]
    fn open_workspace_requires_name_and_defaults_to_reuse_and_selection() {
        let (sender, receiver) = async_channel::bounded::<LiveRequest>(1);
        for params in [json!({}), json!({"name":" "}), json!({"name":"a","cwd":"relative"}), json!({"name":"a","window_id":"invalid"})] {
            let request = json!({"jsonrpc":"2.0","id":1,"method":"workspace.open","params":params});
            assert_eq!(dispatch_inner(&serde_json::to_vec(&request).unwrap(), Some(&sender)).unwrap()["error"]["code"], -32602);
            assert!(receiver.is_empty());
        }
        let worker = std::thread::spawn(move || {
            let request = receiver.recv_blocking().unwrap();
            assert!(request.pending.start());
            let create = request.workspace_create.unwrap();
            assert!(create.reuse && create.focus && !create.activate);
            assert!(create.window_id.is_empty());
            assert_eq!(create.name.as_deref(), Some("Editor 世界"));
            request.reply.send(Ok(json!({"reused":true}))).unwrap();
        });
        let request = json!({"jsonrpc":"2.0","id":2,"method":"workspace.open","params":{"name":" Editor 世界 "}});
        assert_eq!(dispatch_inner(&serde_json::to_vec(&request).unwrap(), Some(&sender)).unwrap()["result"]["reused"], true);
        worker.join().unwrap();
    }

    #[test]
    fn ssh_workspace_requests_validate_before_queueing() {
        let (sender, receiver) = async_channel::bounded::<LiveRequest>(1);
        for destination in ["-oProxyCommand=bad", "host command", "ssh://host:0", "ssh://host/path"] {
            let request = json!({"jsonrpc":"2.0","id":1,"method":"workspace.create", "params":{"window_id":"4294967297","ssh_destination":destination}});
            let response = dispatch_inner(&serde_json::to_vec(&request).unwrap(), Some(&sender)).unwrap();
            assert_eq!(response["error"]["code"], -32602);
            assert!(receiver.try_recv().is_err());
        }
        let worker = std::thread::spawn(move || {
            let request = receiver.recv_blocking().unwrap();
            assert!(request.pending.start());
            let create = request.workspace_create.unwrap();
            assert_eq!(create.ssh_destination.as_deref(), Some("ssh://user@[2001:db8::1]:2222"));
            assert!(!create.focus && !create.activate);
            request.reply.send(Ok(json!({"id":"fixture"}))).unwrap();
        });
        let request = json!({"jsonrpc":"2.0","id":2,"method":"workspace.create", "params":{"window_id":"4294967297","ssh_destination":"ssh://user@[2001:db8::1]:2222"}});
        assert_eq!(dispatch_inner(&serde_json::to_vec(&request).unwrap(), Some(&sender)).unwrap()["result"]["id"], "fixture");
        worker.join().unwrap();
    }

    #[test]
    fn target_ids_preserve_generations_and_reject_alternate_spellings() {
        let first = gpui::EntityId::from(0x100000001);
        let later = gpui::EntityId::from(0x300000001);
        assert_eq!(target_id(&first.to_string()).unwrap(), first);
        assert_eq!(target_id(&later.to_string()).unwrap(), later);
        assert_ne!(first, later);
        for text in [
            "",
            "0",
            "+4294967297",
            "04294967297",
            " 4294967297",
            "18446744073709551616",
            "missing",
        ] {
            assert_eq!(target_id(text).unwrap_err().code, -32004);
        }
    }
    #[test]
    fn surface_focus_validates_and_routes_requests_and_notifications() {
        let (sender, receiver) = async_channel::bounded::<LiveRequest>(1);
        for params in [
            json!({"workspace_id":"42"}),
            json!({"surface_id":""}),
            json!({"surface_id":42}),
            json!({"surface_id":"42", "activate":null}),
        ] {
            let request = json!({"jsonrpc":"2.0","id":1,"method":"surface.focus","params":params});
            assert_eq!(
                dispatch_inner(&serde_json::to_vec(&request).unwrap(), Some(&sender)).unwrap()["error"]
                    ["code"],
                -32602
            );
        }
        assert!(receiver.try_recv().is_err());
        let worker = std::thread::spawn(move || {
            for activate in [false, true] {
                let request = receiver.recv_blocking().unwrap();
                assert_eq!(request.method, "surface.focus");
                assert!(request.selection.is_none());
                assert!(request.pending.start());
                let focus = request.surface_focus.unwrap();
                assert_eq!(focus.surface_id, "42");
                assert_eq!(focus.activate, activate);
                request
                    .reply
                    .send(Ok(json!({"surface_id":"42","selected":true})))
                    .unwrap();
            }
        });
        let request =
            br#"{"jsonrpc":"2.0","id":7,"method":"surface.focus","params":{"surface_id":"42"}}"#;
        assert_eq!(dispatch(request).unwrap()["error"]["code"], -32601);
        let response = dispatch_inner(request, Some(&sender)).unwrap();
        assert_eq!(response["id"], 7);
        assert_eq!(response["result"]["selected"], true);
        assert!(dispatch_inner(br#"{"jsonrpc":"2.0","method":"surface.focus","params":{"surface_id":"42","activate":true}}"#, Some(&sender)).is_none());
        worker.join().unwrap();
    }
    #[test]
    fn layout_discovery_preserves_split_axes_weights_and_pane_identity() {
        use super::super::layout::{Axis, PaneTree};
        let tree = PaneTree::Split {
            axis: Axis::Columns,
            weights: vec![2., 1.],
            children: vec![
                PaneTree::Pane(11),
                PaneTree::Split {
                    axis: Axis::Rows,
                    weights: vec![1., 3.],
                    children: vec![PaneTree::Pane(22), PaneTree::Pane(33)],
                },
            ],
        };
        let value = layout_snapshot(&tree, &|id| id.to_string());
        assert_eq!(value["axis"], "columns");
        assert_eq!(value["weights"], json!([2., 1.]));
        assert_eq!(value["children"][0]["pane_id"], "11");
        assert_eq!(value["children"][1]["axis"], "rows");
        assert_eq!(value["children"][1]["weights"], json!([1., 3.]));
        assert_eq!(value["children"][1]["children"][1]["pane_id"], "33");
    }
    fn dispatch(bytes: &[u8]) -> Option<Value> {
        dispatch_inner(bytes, None)
    }
    #[test]
    fn expired_or_canceled_requests_cannot_start() {
        let pending = Pending::new();
        assert!(pending.cancel());
        assert!(!pending.start());
        let pending = Pending::new();
        assert!(pending.start());
        assert!(!pending.cancel());
        assert!(!pending.start());
        let pending = Pending {
            state: AtomicU8::new(0),
            deadline: Instant::now() - Duration::from_secs(1),
        };
        assert!(!pending.start());
    }
    #[test]
    fn selection_validates_parameters_before_queueing() {
        let (sender, receiver) = async_channel::bounded::<LiveRequest>(1);
        for params in [
            json!({}),
            json!({"workspace_id":1}),
            json!({"workspace_id":""}),
            json!({"workspace_id":"42", "activate":"yes"}),
            json!({"workspace_id":"42", "extra":true}),
        ] {
            let request =
                json!({"jsonrpc":"2.0","id":4,"method":"workspace.select","params":params});
            assert_eq!(
                dispatch_inner(&serde_json::to_vec(&request).unwrap(), Some(&sender)).unwrap()["error"]
                    ["code"],
                -32602
            );
        }
        assert!(receiver.try_recv().is_err());
    }
    #[test]
    fn focus_history_validates_and_routes() {
        let (sender, receiver) = async_channel::bounded::<LiveRequest>(1);
        for params in [json!({}), json!({"window_id":"0","direction":"back"}),
            json!({"window_id":"4294967297","direction":"sideways"}),
            json!({"window_id":"4294967297","direction":"back","activate":"yes"}),
            json!({"window_id":"4294967297","direction":"back","extra":true})] {
            let bytes = serde_json::to_vec(&json!({"jsonrpc":"2.0","id":1,"method":"window.focus_history","params":params})).unwrap();
            assert_eq!(dispatch_inner(&bytes, Some(&sender)).unwrap()["error"]["code"], -32602);
        }
        assert!(receiver.try_recv().is_err());
        let worker = std::thread::spawn(move || {
            let request = receiver.recv_blocking().unwrap();
            assert!(request.pending.start());
            let navigation = request.focus_history.unwrap();
            assert_eq!(navigation.window_id, "4294967297");
            assert!(matches!(navigation.direction, FocusDirection::Back));
            assert!(!navigation.activate);
            request.reply.send(Ok(json!({"window_id":"4294967297","moved":false,"surface_id":null}))).unwrap();
        });
        let response = dispatch_inner(br#"{"jsonrpc":"2.0","id":"back","method":"window.focus_history","params":{"window_id":"4294967297","direction":"back"}}"#, Some(&sender)).unwrap();
        assert_eq!(response["id"], "back");
        assert_eq!(response["result"]["moved"], false);
        worker.join().unwrap();
    }
    #[test]
    fn selection_notifications_execute_and_ui_errors_preserve_request_ids() {
        let (sender, receiver) = async_channel::bounded::<LiveRequest>(1);
        let worker = std::thread::spawn(move || {
            for activate in [false, true] {
                let request = receiver.recv_blocking().unwrap();
                assert!(request.pending.start());
                let selection = request.selection.unwrap();
                assert_eq!(selection.workspace_id, "42");
                assert_eq!(selection.activate, activate);
                request
                    .reply
                    .send(Err(Failure {
                        code: -32004,
                        message: "Workspace not found",
                    }))
                    .unwrap();
            }
        });
        assert!(
            dispatch_inner(
                br#"{"jsonrpc":"2.0","method":"workspace.select","params":{"workspace_id":"42"}}"#,
                Some(&sender)
            )
            .is_none()
        );
        let response = dispatch_inner(br#"{"jsonrpc":"2.0","id":"select-1","method":"workspace.select","params":{"workspace_id":"42","activate":true}}"#, Some(&sender)).unwrap();
        assert_eq!(response["id"], "select-1");
        assert_eq!(response["error"]["code"], -32004);
        worker.join().unwrap();
    }
    #[test]
    fn live_bridge_routes_requests_and_reports_disconnection() {
        let (sender, receiver) = async_channel::bounded::<LiveRequest>(1);
        let worker = std::thread::spawn(move || {
            let request = receiver.recv_blocking().unwrap();
            assert_eq!(request.method, "workspace.list");
            request
                .reply
                .send(Ok(json!({"workspaces":[{"id":"42"}]})))
                .unwrap();
        });
        let request = br#"{"jsonrpc":"2.0","id":3,"method":"workspace.list"}"#;
        let response = dispatch_inner(request, Some(&sender)).unwrap();
        assert_eq!(response["id"], 3);
        assert_eq!(response["result"]["workspaces"][0]["id"], "42");
        worker.join().unwrap();
        assert_eq!(
            dispatch_inner(request, Some(&sender)).unwrap()["error"]["code"],
            -32000
        );
    }
    #[test]
    fn live_discovery_and_notifications_do_not_queue_ui_work() {
        let (sender, receiver) = async_channel::bounded::<LiveRequest>(1);
        let response = dispatch_inner(
            br#"{"jsonrpc":"2.0","id":1,"method":"system.capabilities"}"#,
            Some(&sender),
        )
        .unwrap();
        assert_eq!(response["result"]["methods"], json!(LIVE_METHODS));
        assert_eq!(response["result"]["live_workspace_access"], true);
        let events = &response["result"]["events"];
        assert_eq!(events["method"], "events.read");
        assert_eq!(events["cursor_scope"], "process");
        assert_eq!(events["push_subscriptions"], false);
        let advertised = json!({"kinds":events["kinds"], "limit":events["max_page_records"], "wait_ms":events["max_wait_ms"]});
        assert!(serde_json::from_value::<super::super::events::Read>(advertised).unwrap().valid());
        let standalone = dispatch_inner(br#"{"jsonrpc":"2.0","id":1,"method":"system.capabilities"}"#, None).unwrap();
        assert!(standalone["result"]["events"].is_null());
        assert!(
            dispatch_inner(
                br#"{"jsonrpc":"2.0","method":"window.list"}"#,
                Some(&sender)
            )
            .is_none()
        );
        assert!(receiver.try_recv().is_err());
        let (reply, _) = std::sync::mpsc::sync_channel(1);
        sender
            .try_send(LiveRequest {
                method: "window.list".into(),
                events_read: None,
                buffer_command: None,
                list_filter: ListFilter::default(),
                notification_change: None,
                surface_key: None,
                workspace_close: None,
                workspace_equalize: None,
                selection: None,
                focus_history: None,
                surface_focus: None,
                surface_text: None,
                surface_read: None,
                surface_memory: None,
                surface_move: None,
                pane_zoom: None,
                pane_split: None,
                pane_resize: None,
                pane_swap: None,
                window_create: None,
                workspace_create: None,
                workspace_rename: None,
                group_mutation: None,
                group_create: None,
                workspace_move: None,
                workspace_merge: None,
                pending: Arc::new(Pending::new()),
                reply,
            })
            .unwrap();
        assert_eq!(
            dispatch_inner(
                br#"{"jsonrpc":"2.0","id":2,"method":"window.list"}"#,
                Some(&sender)
            )
            .unwrap()["error"]["code"],
            -32000
        );
    }
    #[test]
    fn capabilities_and_parameter_validation() {
        let response =
            dispatch(br#"{"jsonrpc":"2.0","id":-1,"method":"system.capabilities","params":[]}"#)
                .unwrap();
        assert_eq!(response["id"], -1);
        assert_eq!(response["result"]["methods"], json!(METHODS));
        assert_eq!(response["result"]["live_workspace_access"], false);
        assert_eq!(response["result"]["max_request_bytes"], MAX_REQUEST_BYTES);
        for params in [json!(null), json!([1]), json!({"extra":true})] {
            let request = json!({"jsonrpc":"2.0","id":1,"method":"system.ping","params":params});
            assert_eq!(
                dispatch(&serde_json::to_vec(&request).unwrap()).unwrap()["error"]["code"],
                -32602
            );
        }
    }
    #[test]
    fn discovery_errors_notifications_and_stream_bounds() -> io::Result<()> {
        let input = b"{\"jsonrpc\":\"2.0\",\"id\":7,\"method\":\"system.ping\"}\n{\"jsonrpc\":\"2.0\",\"method\":\"system.ping\"}\n{\"jsonrpc\":\"2.0\",\"id\":\"x\",\"method\":\"workspace.list\"}\n";
        let mut output = Vec::new();
        serve(&input[..], &mut output)?;
        let responses: Vec<Value> = output
            .split(|byte| *byte == b'\n')
            .filter(|line| !line.is_empty())
            .map(|line| serde_json::from_slice(line).unwrap())
            .collect();
        assert_eq!(responses.len(), 2);
        assert_eq!(responses[0]["result"]["pong"], true);
        assert_eq!(responses[0]["id"], 7);
        assert_eq!(responses[1]["error"]["code"], -32601);
        assert_eq!(responses[1]["id"], "x");
        assert_eq!(dispatch(b"bad").unwrap()["error"]["code"], -32700);
        assert_eq!(dispatch(b"[]").unwrap()["error"]["code"], -32600);
        assert!(serve(&vec![b' '; MAX_REQUEST_BYTES + 1][..], Vec::new()).is_err());
        Ok(())
    }
}
