use super::*;

/// Selecting/keying a window alone does not foreground the application on macOS.
/// Call only for explicit activation requests; background RPC stays background.
fn activate_destination(window: &mut Window, cx: &mut App) {
    window.activate_window();
    cx.activate(true);
}


/// Resolve current ownership at click time, including detached/moved terminals.
#[cfg(unix)]
pub(super) fn focus_native_notification(terminal: &WeakEntity<TerminalView>, revision: u64, cx: &mut App) {
    if !cx.global::<Preferences>().native_notifications { return; }
    if !terminal.read_with(cx, |view, _| view.notifications.contains(revision)).unwrap_or(false) { return; }
    let target = terminal.entity_id();
    for handle in cx.windows() {
        let Some(handle) = handle.downcast::<WorkspaceWindow>() else { continue; };
        let focused = handle.update(cx, |root, window, cx| {
            if root.automation_available(cx).is_err() { return false; }
            let workspace = root.workspace_state.entries.iter().find(|entry| entry.workspace.read(cx).panes.iter().any(|pane| {
                pane.read(cx).terminals.iter().any(|terminal| terminal.entity_id() == target)
            })).map(|entry| entry.workspace.entity_id());
            let Some(workspace) = workspace else { return false; };
            root.focus_terminal(workspace, target, window, cx);
            window.activate_window();
            cx.activate(true);
            true
        }).unwrap_or(false);
        if focused { break; }
    }
}

/// Notification visibility is structural, not a scan of terminal/parser state.
/// An inactive OS window, inactive workspace/tab or a zoom-hidden pane is invisible.
pub(super) fn terminal_is_visible(terminal: EntityId, cx: &mut App) -> bool {
    cx.windows().into_iter().any(|handle| {
        let Some(handle) = handle.downcast::<WorkspaceWindow>() else { return false; };
        handle.update(cx, |root, window, cx| {
            if !window.is_window_active() { return false; }
            let Some(entry) = root.workspace_state.entries.get(root.workspace_state.active) else { return false; };
            let workspace = entry.workspace.read(cx);
            workspace.panes.iter().any(|pane| {
                workspace.pane_state.zoomed.is_none_or(|zoomed| zoomed == pane.entity_id())
                    && pane.read(cx).active_terminal_id() == Some(terminal)
            })
        }).unwrap_or(false)
    })
}


#[derive(Clone)]
struct NotificationNavigation {
    expanded: Option<(EntityId, u64)>,
    unread_only: bool,
    focus: FocusHandle,
    origin: Option<FocusHandle>,
    selected: Option<(EntityId, u64)>,
    scroll: ScrollHandle,
}

pub(super) fn normalized_workspace_name(text: &str) -> Option<String> {
    rmux_core::workspace_lifecycle::normalized_name(text)
}

#[derive(Clone, Copy, PartialEq)]
enum MetadataField {
    GroupName,
    GroupColor,
    GroupIcon,
    Name,
    Description,
    Icon,
    Color,
}
impl MetadataField {
    fn label(self) -> &'static str {
        match self {
            Self::GroupName => "Group name",
            Self::Name => "Workspace name",
            Self::Description => "Workspace description",
            Self::Icon | Self::GroupIcon => "Icon: one symbol or emoji; empty clears",
            Self::Color | Self::GroupColor => "Color: #RRGGBB; empty clears",
        }
    }
}
use rmux_core::workspace_lifecycle::parse_workspace_color;

struct Entry {
    workspace: Entity<Workspace>,
    name: String,
    description: String,
    project_directory: Option<std::path::PathBuf>,
    color: Option<u32>,
    icon: Option<String>,
    pinned: bool,
    last_pane: Option<EntityId>,
    _closed: Subscription,
    _metadata: Subscription,
}

#[derive(Clone)]
struct WorkspaceMenu {
    target: EntityId,
    group: Option<u64>,
    position: Point<Pixels>,
    focus: FocusHandle,
    return_focus: Option<FocusHandle>,
    selected: usize,
    scroll: ScrollHandle,
    typeahead: menu_navigation::Typeahead,
}

pub(super) struct WorkspaceWindow {
    workspace_menu: Option<WorkspaceMenu>,
    workspace_menu_blur: Option<Subscription>,
    palette: Option<(Entity<command_palette::CommandPalette>, Option<FocusHandle>)>,
    font_picker_job: Option<Task<()>>,
    file_browser_job: Option<Task<()>>,
    preview_restore: Option<command_palette::PreviewState>,
    folder_restore: Option<command_palette::FolderState>,
    file_history: file_browser::History,
    preview_history: command_palette::PreviewHistory,
    file_browser_cancel: Option<file_browser::Cancellation>,
    terminal_search_job: Option<Task<()>>,
    terminal_search_query: String,
    terminal_search_cancel: Option<file_browser::Cancellation>,
    restore_job: Option<restore::Job>,
    workspace_scroll: ScrollHandle,
    notice: Option<String>,
    memory_rows: Option<Vec<memory_view::Row>>,
    memory_scroll: ListState,
    memory_focus: Option<FocusHandle>,
    memory_previous_focus: Option<FocusHandle>,
    memory_selected: usize,
    memory_application: Option<process_metrics::Sample>,
    memory_sampled_at: Option<u64>,
    memory_sort: memory_view::Sort,
    memory_loading: bool,
    memory_generation: u64,
    memory_canceled: Option<Arc<AtomicBool>>,
    process_sampler: Arc<std::sync::Mutex<process_metrics::Sampler>>,
    rename: Option<(EntityId, Entity<text_field::TextField>, MetadataField)>,
    workspace_state: rmux_core::workspace_lifecycle::State<Entry, EntityId>,
    rendered_workspace: Option<WeakEntity<Workspace>>,
    rendered_background: Option<WindowBackgroundAppearance>,
    close_prompt: Option<EntityId>,
    window_close_prompt: bool,
    close_handler_installed: bool,
    focus_history: Vec<EntityId>,
    focus_trail: focus_trail::FocusTrail<EntityId>,
    notification_navigation: Option<NotificationNavigation>,
    settings_state: Option<settings_view::State>,
    command_launching: bool,
    workspace_launch_cancel: Option<file_browser::Cancellation>,
    next_number: usize,
    sidebar: bool,
    sidebar_width: f32,
    sidebar_drag: Option<(Pixels, f32)>,
    benchmark: bool,
}
impl WorkspaceWindow {
    fn observe_settings(cx: &mut Context<Self>) {
        cx.observe_global::<Preferences>(|_, cx| cx.notify()).detach();
        cx.observe_global::<preferences::AppearanceBaseline>(|this, cx| {
            if this.settings_state.is_some() { cx.notify(); }
        }).detach();
        cx.observe_global::<preferences::LastSettingsSave>(|this, cx| {
            if this.settings_state.is_some() { cx.notify(); }
        }).detach();
        cx.observe_global::<preferences::ReloadState>(|this, cx| {
            if this.settings_state.is_some() { cx.notify(); }
        }).detach();
    }
    fn dismiss_workspace_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(menu) = self.workspace_menu.take() {
            if menu.focus.is_focused(window) {
                if let Some(focus) = menu.return_focus { focus.focus(window); }
            }
        }
        self.workspace_menu_blur = None;
        cx.notify();
    }

    fn show_workspace_menu(&mut self, target: EntityId, group: Option<u64>, position: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        if self.automation_available(cx).is_err() { return; }
        let Some(index) = self.workspace_state.entries.iter().position(|entry| entry.workspace.entity_id() == target) else { return; };
        self.workspace_state.sidebar_selection.clear();
        self.workspace_state.sidebar_selection_anchor = Some(target);
        self.select(index, window, cx);
        let return_focus = window.focused(cx);
        let focus = cx.focus_handle();
        let token = focus.clone();
        self.workspace_menu_blur = Some(cx.on_blur(&focus, window, move |this, _, cx| {
            if this.workspace_menu.as_ref().is_some_and(|menu| menu.focus == token) {
                this.workspace_menu = None;
                this.workspace_menu_blur = None;
                cx.notify();
            }
        }));
        focus.focus(window);
        self.workspace_menu = Some(WorkspaceMenu { target, group, position, focus, return_focus, selected: 0, scroll: ScrollHandle::new(), typeahead: Default::default() });
        cx.notify();
    }

    fn activate_workspace_menu(&mut self, item: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some((target, group)) = self.workspace_menu.as_ref().map(|menu| (menu.target, menu.group)) else { return; };
        self.dismiss_workspace_menu(window, cx);
        let Some(index) = self.workspace_state.entries.iter().position(|entry| entry.workspace.entity_id() == target) else { return; };
        // Never apply a captured group command to a new group after a move.
        if let Some(group) = group {
            if !self.workspace_state.groups.containing(target).is_some_and(|current| current.id == group) { return; }
        }
        self.select(index, window, cx);
        if group.is_some() {
            match item {
                0 => self.edit_metadata(MetadataField::GroupName, window, cx),
                1 => self.edit_metadata(MetadataField::GroupColor, window, cx),
                2 => self.edit_metadata(MetadataField::GroupIcon, window, cx),
                3 => self.toggle_group_pin(cx),
                4 => self.ungroup_active(cx),
                5 => self.request_close_group(window, cx),
                _ => {},
            }
            return;
        }
        match item {
            0 => self.edit_metadata(MetadataField::Name, window, cx),
            1 => self.edit_metadata(MetadataField::Description, window, cx),
            2 => self.edit_metadata(MetadataField::Color, window, cx),
            3 => self.edit_metadata(MetadataField::Icon, window, cx),
            4 => self.toggle_workspace_pin(cx),
            5 => self.request_close_workspace(index, window, cx),
            _ => {},
        }
    }

    fn workspace_menu_element(&self, menu: WorkspaceMenu, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let pinned = self.workspace_state.entries.iter().find(|entry| entry.workspace.entity_id() == menu.target).is_some_and(|entry| entry.pinned);
        let pinned = menu.group.map_or(pinned, |id| self.workspace_state.groups.entries().iter().any(|group| group.id == id && group.pinned));
        let labels = crate::workspace_menu_view::labels(menu.group.is_some(), pinned);
        let actions = crate::workspace_menu_view::commands(menu.group.is_some()).map(|command|
            crate::workspace_actions::action(command).expect("workspace menu command"));
        let hints: Vec<_> = actions.iter().map(|action| menu.return_focus.as_ref().and_then(|focus| shortcuts::hint(action.as_ref(), focus, window))).collect();
        deferred(anchored().position(menu.position).snap_to_window().child(
            crate::workspace_menu_view::frame(window).track_focus(&menu.focus).key_context("WorkspaceMenu")
                .on_mouse_down_out(cx.listener(|this, _, window, cx| this.dismiss_workspace_menu(window, cx)))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_mouse_down(MouseButton::Middle, |_, _, cx| cx.stop_propagation())
                .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
                .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
                .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                    let Some(menu) = &mut this.workspace_menu else { return; };
                    let modifiers = event.keystroke.modifiers;
                    let typed = if !modifiers.control && !modifiers.alt && !modifiers.platform && !modifiers.function {
                        menu.typeahead.select(&event.keystroke.key, &labels, &[true; 6], menu.selected, std::time::Instant::now())
                    } else { None };
                    match event.keystroke.key.as_str() {
                        "up" => menu.selected = (menu.selected + 5) % 6,
                        "down" => menu.selected = (menu.selected + 1) % 6,
                        "home" => menu.selected = 0,
                        "end" => menu.selected = 5,
                        "enter" | "space" => { let selected = menu.selected; this.activate_workspace_menu(selected, window, cx); },
                        "escape" => this.dismiss_workspace_menu(window, cx),
                        _ => { if let Some(selected) = typed { menu.selected = selected; } },
                    }
                    if let Some(menu) = &this.workspace_menu { menu.scroll.scroll_to_item(menu.selected); }
                    cx.stop_propagation();
                    cx.notify();
                }))
                .child(div().id("workspace-menu-scroll")
                    .max_h((window.viewport_size().height - px(26.)).max(px(1.)))
                    .overflow_y_scroll().track_scroll(&menu.scroll)
                    .children(labels.into_iter().enumerate().map(|(index, label)| {
                    crate::workspace_menu_view::row(index, menu.selected)
                        .on_mouse_move(cx.listener(move |this, _, _, cx| {
                            if let Some(menu) = &mut this.workspace_menu {
                                if menu.selected != index { menu.selected = index; cx.notify(); }
                            }
                        }))
                        .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
                            if event.standard_click() { this.activate_workspace_menu(index, window, cx); }
                            cx.stop_propagation();
                        }))
                        .child(div().flex_1().min_w_0().truncate().child(label))
                        .when_some(hints[index].clone(), |row, hint| row.child(
                            div().id(("workspace-menu-shortcut", index)).max_w(px(120.)).flex_shrink_0().truncate().text_xs().text_color(rgb(0xa0a0a0))
                                .tooltip({ let hint = hint.clone(); move |_, cx| cx.new(|_| pane_view::ControlHint(hint.clone().into())).into() })
                                .child(hint)
                        ))
                })))
        )).with_priority(2).into_any_element()
    }

    pub(super) fn focus_memory_terminal(&mut self, workspace: EntityId, surface: EntityId, window: &mut Window, cx: &mut Context<Self>) {
        if self.automation_available(cx).is_ok() {
            if let Some(rows) = &self.memory_rows {
                if let Some(index) = self.memory_sort.indices(rows).iter().position(|index| rows[*index].surface_id == surface) { self.memory_selected = index; }
            }
            self.focus_terminal(workspace, surface, window, cx);
        }
    }
    pub(super) fn render_memory_row(&self, index: usize, selected: bool, cx: &mut Context<Self>) -> AnyElement {
        self.memory_rows.as_ref().and_then(|rows| rows.get(index))
            .map(|row| memory_view::render_row(row, index, selected, cx))
            .unwrap_or_else(|| div().into_any_element())
    }
    pub(super) fn memory_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let modifiers = event.keystroke.modifiers;
        if modifiers.control || modifiers.platform || modifiers.alt
            || (modifiers.shift && event.keystroke.key != "tab") { return; }
        let count = self.memory_rows.as_ref().map_or(0, Vec::len);
        let key = if event.keystroke.key == "tab" {
            if modifiers.shift { "up" } else { "down" }
        } else { event.keystroke.key.as_str() };
        let navigation = matches!(key, "up" | "down" | "home" | "end" | "pageup" | "pagedown");
        match key {
            "escape" => self.close_memory(window, cx),
            "r" => self.refresh_memory(window, cx),
            "s" => self.show_memory_sort(window, cx),
            "c" => { if let Some(rows) = &self.memory_rows { clipboard::write(ClipboardItem::new_string(memory_view::report(rows, self.memory_application, self.memory_sort, self.memory_sampled_at)), cx); } },
            "up" if count > 0 => self.memory_selected = (self.memory_selected + count - 1) % count,
            "down" if count > 0 => self.memory_selected = (self.memory_selected + 1) % count,
            "home" => self.memory_selected = 0,
            "end" => self.memory_selected = count.saturating_sub(1),
            "pageup" | "pagedown" => {
                let fallback = self.memory_scroll.bounds_for_item(self.memory_selected)
                    .map(|bounds| f32::from(bounds.size.height)).unwrap_or(100.).max(1.);
                if let Some(selected) = command_palette::paged_result(self.memory_selected, count, key == "pageup",
                    f32::from(self.memory_scroll.viewport_bounds().size.height), |index| {
                        self.memory_scroll.bounds_for_item(index).map(|bounds| f32::from(bounds.size.height)).unwrap_or(fallback).max(1.)
                    }) { self.memory_selected = selected; }
            },
            "enter" => {
                let target = self.memory_rows.as_ref().and_then(|rows| self.memory_sort.indices(rows).get(self.memory_selected).map(|index| (rows[*index].workspace_id, rows[*index].surface_id)));
                if let Some((workspace, surface)) = target { self.focus_memory_terminal(workspace, surface, window, cx); }
            },
            _ => {},
        }
        // Copying or typing an unrelated key must not undo a manual wheel scroll.
        if navigation && count > 0 && self.memory_rows.is_some() { crate::list_navigation::reveal(&self.memory_scroll, self.memory_selected); }
        cx.stop_propagation();
        cx.notify();
    }
    fn sort_memory(&mut self, cx: &mut Context<Self>) {
        self.set_memory_sort(self.memory_sort.next(), cx);
    }
    fn show_memory_sort(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.memory_rows.is_none() || self.automation_available(cx).is_err() { return; }
        let focus = window.focused(cx);
        let palette = cx.new(|cx| command_palette::CommandPalette::memory_sort(self.memory_sort, window, cx));
        self.install_palette(palette, focus, window, cx);
    }
    fn set_memory_sort(&mut self, sort: memory_view::Sort, cx: &mut Context<Self>) {
        let rows = self.memory_rows.as_deref().unwrap_or_default();
        let selected = self.memory_sort.indices(rows).get(self.memory_selected).copied();
        self.memory_sort = sort;
        self.memory_selected = selected.and_then(|selected| self.memory_sort.indices(rows).iter().position(|index| *index == selected)).unwrap_or(0);
        self.memory_scroll.reset(rows.len());
        if !rows.is_empty() { crate::list_navigation::reveal(&self.memory_scroll, self.memory_selected); }
        cx.notify();
    }
    fn close_memory(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(canceled) = self.memory_canceled.take() { canceled.store(true, Ordering::Relaxed); }
        self.process_sampler = Default::default();
        self.memory_rows = None;
        self.memory_scroll.reset(0);
        self.memory_application = None;
        self.memory_sampled_at = None;
        self.memory_loading = false;
        self.memory_generation = self.memory_generation.wrapping_add(1);
        if self.memory_focus.as_ref().is_some_and(|focus| focus.is_focused(window)) {
            if let Some(focus) = self.memory_previous_focus.take() { focus.focus(window); }
            else { self.select(self.workspace_state.active, window, cx); }
        }
        self.memory_previous_focus = None;
        cx.notify();
    }
    fn refresh_memory(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.memory_focus.as_ref().is_some_and(|focus| focus.is_focused(window)) {
            self.memory_previous_focus = window.focused(cx);
        }
        self.memory_focus.get_or_insert_with(|| cx.focus_handle()).focus(window);
        if self.memory_loading { return; }
        let mut jobs = Vec::new();
        'workspaces: for index in self.workspace_order(true) {
            let entry = &self.workspace_state.entries[index];
            for pane in &entry.workspace.read(cx).panes {
                let pane = pane.read(cx);
                for (tab, terminal) in pane.terminals.iter().enumerate() {
                    if jobs.len() == 256 { break 'workspaces; }
                    let view = terminal.read(cx);
                    jobs.push((entry.workspace.entity_id(), terminal.entity_id(), format!("{} / {}", entry.name, pane.tab_title(tab, cx)), view.session.process_id(), view.session.foreground_process_id(), view.session.memory_sample()));
                }
            }
        }
        self.memory_rows.get_or_insert_with(Vec::new);
        self.memory_loading = true;
        self.memory_generation = self.memory_generation.wrapping_add(1);
        let generation = self.memory_generation;
        let executor = cx.background_executor().clone();
        let process_sampler = self.process_sampler.clone();
        let canceled = Arc::new(AtomicBool::new(false));
        self.memory_canceled = Some(canceled.clone());
        cx.spawn(async move |entity, cx| {
            let mut rows = executor.spawn(async move {
                if canceled.load(Ordering::Relaxed) { return (Vec::new(), None, None); }
                let (processes, trees) = {
                    let mut sampler = process_sampler.lock().unwrap_or_else(|error| error.into_inner());
                    if canceled.load(Ordering::Relaxed) { return (Vec::new(), None, None); }
                    sampler.sample_with_trees(std::iter::once(std::process::id()).chain(jobs.iter().flat_map(|(_, _, _, pid, foreground_pid, _)| [*pid, *foreground_pid].into_iter().flatten())), jobs.iter().filter_map(|(_, _, _, pid, _, _)| *pid), &canceled)
                };
                let application = processes.get(&std::process::id()).copied();
                let rows: Vec<_> = jobs.into_iter().take_while(|_| !canceled.load(Ordering::Relaxed)).map(|(workspace_id, surface_id, name, pid, foreground_pid, sample)| {
                    let (usage, status) = match sample() {
                        Ok(Some(usage)) => (Some(usage), "Sampled"),
                        Ok(None) => (None, "Busy · refresh to retry"),
                        Err(_) => (None, "Terminal closed or sample unavailable"),
                    };
                    let process = pid.and_then(|pid| processes.get(&pid).copied());
                    memory_view::Row { workspace_id, surface_id, name, pid, process, tree: pid.and_then(|pid| trees.get(&pid).copied()), foreground_pid, foreground: foreground_pid.and_then(|pid| processes.get(&pid).copied()), usage, status }
                }).collect();
                (rows, application, std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).ok().map(|time| time.as_millis().min(u64::MAX as u128) as u64))
            }).await;
            let _ = entity.update(cx, |this, cx| {
                if this.memory_generation != generation || this.memory_rows.is_none() { return; }
                // A command may finish or change while the OS sampler runs.
                // Keep only foreground samples still owned by this terminal.
                let expected: std::collections::HashMap<_, _> = rows.0.iter().filter_map(|row| row.foreground_pid.map(|pid| (row.surface_id, pid))).collect();
                let shells: std::collections::HashMap<_, _> = rows.0.iter().filter_map(|row| row.pid.map(|pid| (row.surface_id, pid))).collect();
                let mut retained = std::collections::HashSet::new();
                let mut retained_shells = std::collections::HashSet::new();
                if !expected.is_empty() || !shells.is_empty() {
                    for entry in &this.workspace_state.entries {
                        for pane in &entry.workspace.read(cx).panes {
                            for terminal in &pane.read(cx).terminals {
                                if let Some(pid) = shells.get(&terminal.entity_id()) {
                                    if terminal.read(cx).session.process_id() == Some(*pid) { retained_shells.insert(terminal.entity_id()); }
                                }
                                if let Some(pid) = expected.get(&terminal.entity_id()) {
                                    if terminal.read(cx).session.foreground_process_id() == Some(*pid) { retained.insert(terminal.entity_id()); }
                                }
                            }
                        }
                    }
                }
                for row in &mut rows.0 {
                    if row.foreground_pid.is_some() && !retained.contains(&row.surface_id) { row.foreground = None; }
                    if !retained_shells.contains(&row.surface_id) { row.tree = None; row.process = None; }
                }
                let selected = this.memory_rows.as_ref().and_then(|previous| this.memory_sort.indices(previous).get(this.memory_selected).map(|index| previous[*index].surface_id));
                this.memory_selected = selected.and_then(|surface| this.memory_sort.indices(&rows.0).iter().position(|index| rows.0[*index].surface_id == surface))
                    .unwrap_or_else(|| this.memory_selected.min(rows.0.len().saturating_sub(1)));
                this.memory_scroll.reset(rows.0.len());
                if !rows.0.is_empty() { crate::list_navigation::reveal(&this.memory_scroll, this.memory_selected); }
                this.memory_rows = Some(rows.0);
                this.memory_application = rows.1;
                this.memory_sampled_at = rows.2;
                this.memory_loading = false;
                this.memory_canceled = None;
                cx.notify();
            });
        }).detach();
        cx.notify();
    }

    fn group_selected(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.automation_available(cx).is_err() { return; }
        match self.workspace_state.group_sidebar_selection(|entry| (entry.workspace.entity_id(), entry.pinned)) {
            Ok(Some(_)) => { self.sidebar = true; self.scroll_to_workspace(self.workspace_state.active); }
            Ok(None) => { cx.propagate(); return; }
            Err(error) => self.notice = Some(error.to_string().into()),
        }
        cx.notify();
    }

    fn extend_sidebar_selection(&mut self, target: EntityId, range: bool, cx: &mut Context<Self>) {
        self.workspace_state.extend_sidebar_selection(target, range, |entry| (entry.workspace.entity_id(), entry.pinned));
        cx.notify();
    }

    pub(super) fn automation_group_create(&mut self, create: &automation::GroupCreate, window: &mut Window, cx: &mut Context<Self>) -> Result<serde_json::Value, automation::Failure> {
        self.automation_available(cx)?;
        if let Some(external) = &create.external_id {
            if let Some(group) = self.workspace_state.groups.entries().iter().find(|group| group.external_id.as_ref() == Some(external)) {
                let id = format!("workspace_group:{}", group.id);
                return Ok(self.automation_snapshot(create.window_id.clone(), false, cx)["groups"].as_array().unwrap().iter().find(|group| group["id"] == id).unwrap().clone());
            }
        }
        let generated = create.workspace_ids.is_none();
        let mut members = Vec::new();
        if let Some(ids) = &create.workspace_ids {
            for id in ids {
                members.push(self.workspace_state.entries.iter().find(|entry| entry.workspace.entity_id().to_string() == *id)
                    .map(|entry| entry.workspace.entity_id()).ok_or(automation::Failure { code: -32004, message: "Workspace not found in window" })?);
            }
        } else if self.workspace_state.entries.len() >= 64 {
            return Err(automation::Failure { code: -32005, message: "Window workspace limit reached" });
        }
        let selected = self.workspace_state.entries.get(self.workspace_state.active).map(|entry| entry.workspace.entity_id());
        if generated {
            let previous = self.workspace_state.previous;
            let next_number = self.next_number;
            self.add_with_directory(1, false, Some(workspace_groups::Placement::End), create.cwd.clone(), window, cx);
            let index = self.workspace_state.entries.len() - 1;
            if self.workspace_state.entries[index].workspace.read(cx).panes.is_empty() {
                // No shell or pane was created, so no focus changed. Discard
                // only the empty entry; normal close would select/acknowledge a
                // neighboring terminal and could expand a collapsed group.
                self.workspace_state.entries.pop();
                self.workspace_state.active = selected.and_then(|id| self.workspace_state.entries.iter().position(|entry| entry.workspace.entity_id() == id)).unwrap_or(0);
                self.workspace_state.previous = previous;
                self.next_number = next_number;
                if let Some(entry) = self.workspace_state.entries.get(self.workspace_state.active) { window.set_window_title(&format!("{} — rmux", entry.name)); }
                self.scroll_to_workspace(self.workspace_state.active);
                cx.notify();
                return Err(automation::Failure { code: -32005, message: "Could not start group anchor shell" });
            }
            members.push(self.workspace_state.entries[index].workspace.entity_id());
        }
        let id = match self.workspace_state.create_group_members(create.name.clone(), members, generated, |entry| entry.workspace.entity_id()) {
            Ok(id) => id,
            Err(_) => {
                if generated { self.close_workspace(self.workspace_state.entries.len() - 1, window, cx); }
                return Err(automation::Failure { code: -32005, message: "Could not create group" });
            }
        };
        self.workspace_state.groups.get_mut(id).unwrap().external_id = create.external_id.clone();
        self.workspace_state.groups.get_mut(id).unwrap().cwd = create.cwd.clone();
        self.sidebar = true;
        self.scroll_to_workspace(self.workspace_state.active);
        cx.notify();
        let id = format!("workspace_group:{id}");
        Ok(self.automation_snapshot(create.window_id.clone(), false, cx)["groups"].as_array().unwrap().iter().find(|group| group["id"] == id).unwrap().clone())
    }

    pub(super) fn automation_close_workspace(&mut self, target: EntityId, window: &mut Window, cx: &mut Context<Self>) -> Result<serde_json::Value, automation::Failure> {
        self.automation_available(cx)?;
        let index = self.workspace_state.entries.iter().position(|entry| entry.workspace.entity_id() == target)
            .ok_or(automation::Failure { code: -32004, message: "Workspace not found" })?;
        let workspace = self.workspace_state.entries[index].workspace.read(cx);
        let has_terminals = workspace.panes.iter().any(|pane| !pane.read(cx).terminals.is_empty());
        if cx.global::<ClosePolicy>().confirms(has_terminals, || workspace.panes.iter().any(|pane| pane.read(cx).terminals.iter().any(|terminal| terminal.read(cx).has_active_work()))) {
            return Err(automation::Failure { code: -32003, message: "Workspace requires close confirmation; close it in the UI" });
        }
        let closes_window = self.workspace_state.entries.len() == 1;
        self.close_workspace(index, window, cx);
        Ok(serde_json::json!({"workspace_id":target.to_string(),"closed":true,"window_closed":closes_window}))
    }

    pub(super) fn automation_open_workspaces(&self, name: &str, project: Option<&std::path::Path>) -> Option<Vec<EntityId>> {
        (!self.benchmark).then(|| self.workspace_state.entries.iter().filter(|entry| open_project::matches_workspace(&entry.name, entry.project_directory.as_deref(), name, project))
            .map(|entry| entry.workspace.entity_id()).collect())
    }
    pub(super) fn automation_legacy_project_workspaces(&self, name: &str, project: Option<&std::path::Path>, cx: &App) -> Vec<EntityId> {
        let Some(project) = project else { return Vec::new(); };
        if self.benchmark { return Vec::new(); }
        self.workspace_state.entries.iter().filter(|entry| entry.project_directory.is_none() && entry.name == name)
            .filter_map(|entry| {
                let workspace = entry.workspace.read(cx);
                let pane = workspace.panes.first()?.read(cx);
                let terminal = pane.terminals.first()?.read(cx);
                (terminal.session.remote_destination().is_none()
                    && open_project::matches_legacy_workspace(&entry.name, terminal.session.initial_directory(), name, project))
                    .then_some(entry.workspace.entity_id())
            }).collect()
    }
    pub(super) fn automation_reuse_workspace(&mut self, create: &automation::WorkspaceCreate, id: EntityId, window: &mut Window, cx: &mut Context<Self>) -> Result<serde_json::Value, automation::Failure> {
        self.automation_available(cx)?;
        let index = self.workspace_state.entries.iter().position(|entry| entry.workspace.entity_id() == id)
            .ok_or(automation::Failure { code: -32004, message: "Workspace no longer exists" })?;
        if self.workspace_state.entries[index].project_directory.is_none() {
            self.workspace_state.entries[index].project_directory = create.project_directory.clone();
        }
        if create.focus || create.activate { self.select(index, window, cx); }
        if create.activate { activate_destination(window, cx); }
        let mut result = self.automation_snapshot(create.window_id.clone(), true, cx)["workspaces"].as_array().unwrap()
            .iter().find(|row| row["id"] == id.to_string()).unwrap().clone();
        result["reused"] = true.into();
        cx.notify();
        Ok(result)
    }
    pub(super) fn automation_prepare_workspace(&mut self, create: &automation::WorkspaceCreate, cx: &mut Context<Self>) -> Result<Box<dyn FnOnce() -> Result<PreparedTerminal> + Send>, automation::Failure> {
        self.automation_available(cx)?;
        if self.command_launching || self.workspace_state.entries.len() >= 64 { return Err(automation::Failure { code: -32005, message: "Command startup busy or workspace limit reached" }); }
        let destination = create.ssh_destination.clone();
        if let Some(destination) = &destination {
            ssh::command(destination).map_err(|message| automation::Failure { code: -32602, message })?;
        }
        let directory = create.cwd.clone().or_else(|| {
            if destination.is_some() { return None; }
            self.workspace_state.entries.get(self.workspace_state.active).and_then(|entry| {
                let workspace = entry.workspace.read(cx);
                workspace.panes.iter().find(|pane| Some(pane.entity_id()) == entry.last_pane)
                    .or_else(|| workspace.panes.first()).and_then(|pane| pane.read(cx).cached_directory(cx))
            }).or_else(|| cx.global::<InitialDirectory>().0.clone())
        });
        let scrollback = cx.global::<Preferences>().scrollback_bytes();
        let colors = cx.global::<Preferences>().terminal_colors();
        let palette = cx.global::<Preferences>().effective_palette();
        self.command_launching = true;
        Ok(Box::new(move || {
            let session = ssh::launch_sibling(destination.as_deref(), directory.as_deref(), scrollback)?;
            TerminalView::prepare(session, colors, palette)
        }))
    }
    pub(super) fn automation_end_workspace_startup(&mut self) {
        self.command_launching = false;
    }
    pub(super) fn automation_finish_workspace(&mut self, create: &automation::WorkspaceCreate, prepared: Result<PreparedTerminal>, window: &mut Window, cx: &mut Context<Self>) -> Result<serde_json::Value, automation::Failure> {
        self.command_launching = false;
        if create.reuse {
            let mut matches = self.automation_open_workspaces(create.name.as_deref().unwrap(), create.project_directory.as_deref()).unwrap_or_default();
            if matches.is_empty() { matches = self.automation_legacy_project_workspaces(create.name.as_deref().unwrap(), create.project_directory.as_deref(), cx); }
            if matches.len() > 1 { return Err(automation::Failure { code: -32006, message: "Workspace match became ambiguous during startup" }); }
            if let Some(id) = matches.first() { return self.automation_reuse_workspace(create, *id, window, cx); }
        }
        let prepared = prepared.map_err(|_| automation::Failure { code: -32005, message: "Could not prepare workspace terminal; check the executable and launch directory" })?;
        let settings = cx.global::<Preferences>();
        let seed = TerminalSeed::Ready(prepared, CellMetrics::new(settings.font_size), settings.font_family.clone().into());
        self.automation_publish_workspace(create, (seed, create.ssh_destination.as_ref().map(|destination| format!("SSH · {destination}"))), window, cx)
    }
    fn automation_publish_workspace(&mut self, create: &automation::WorkspaceCreate, prepared: (TerminalSeed, Option<String>), window: &mut Window, cx: &mut Context<Self>) -> Result<serde_json::Value, automation::Failure> {
        self.automation_available(cx)?;
        if self.workspace_state.entries.len() >= 64 { return Err(automation::Failure { code: -32005, message: "Window workspace limit reached" }); }
        let next_number = self.next_number;
        let index = self.add_configured(1, false, None, create.cwd.clone(), false, Some(prepared), window, cx)
            .ok_or(automation::Failure { code: -32005, message: "Window is shutting down" })?;
        if self.workspace_state.entries[index].workspace.read(cx).panes.is_empty() {
            self.workspace_state.entries.remove(index);
            if index < self.workspace_state.active { self.workspace_state.active -= 1; }
            self.next_number = next_number;
            cx.notify();
            return Err(automation::Failure { code: -32005, message: "Could not start workspace shell" });
        }
        self.workspace_state.entries[index].project_directory = create.project_directory.clone();
        if let Some(name) = &create.name { self.workspace_state.entries[index].name = name.clone(); }
        else if let Some(destination) = &create.ssh_destination { self.workspace_state.entries[index].name = format!("SSH · {destination}"); }
        if let Some(description) = &create.description { self.workspace_state.entries[index].description = description.clone(); }
        let id = self.workspace_state.entries[index].workspace.entity_id().to_string();
        if create.focus || create.activate { self.select(index, window, cx); }
        if create.activate { activate_destination(window, cx); }
        cx.notify();
        let mut result = self.automation_snapshot(create.window_id.clone(), true, cx)["workspaces"].as_array().unwrap().iter().find(|row| row["id"] == id).unwrap().clone();
        if create.reuse { result["reused"] = false.into(); }
        Ok(result)
    }

    pub(super) fn automation_group_mutation(&mut self, change: &automation::GroupMutation, method: &str, window: &mut Window, cx: &mut Context<Self>) -> Result<serde_json::Value, automation::Failure> {
        self.automation_available(cx)?;
        let id = change.group_id.strip_prefix("workspace_group:").and_then(|id| id.parse::<u64>().ok())
            .ok_or(automation::Failure { code: -32602, message: "Invalid group ID" })?;
        let group = self.workspace_state.groups.get_mut(id).ok_or(automation::Failure { code: -32004, message: "Group not found in window" })?;
        if method == "workspace.group.new_workspace" {
            let anchor = group.members[0];
            if self.workspace_state.entries.len() >= 64 { return Err(automation::Failure { code: -32005, message: "Window workspace limit reached" }); }
            let index = self.workspace_state.entries.iter().position(|entry| entry.workspace.entity_id() == anchor)
                .ok_or(automation::Failure { code: -32004, message: "Group anchor not found" })?;
            self.select(index, window, cx);
            self.add_with_placement(1, true, change.placement, window, cx);
            let index = self.workspace_state.active;
            if self.workspace_state.entries[index].workspace.read(cx).panes.is_empty() {
                self.close_workspace(index, window, cx);
                return Err(automation::Failure { code: -32005, message: "Could not start group member shell" });
            }
            let workspace_id = self.workspace_state.entries[index].workspace.entity_id().to_string();
            return Ok(serde_json::json!({"window_id":change.window_id,"group_id":change.group_id,"workspace_id":workspace_id}));
        }
        if method == "workspace.group.ungroup" {
            if change.remove_generated_anchor == Some(true) {
                let anchor = group.removable_generated_anchor().copied().ok_or(automation::Failure { code: -32005, message: "Cleanup requires a sole generated anchor" })?;
                let index = self.workspace_state.entries.iter().position(|entry| entry.workspace.entity_id() == anchor)
                    .ok_or(automation::Failure { code: -32004, message: "Group anchor not found" })?;
                let workspace = self.workspace_state.entries[index].workspace.read(cx);
                let has_terminals = workspace.panes.iter().any(|pane| !pane.read(cx).terminals.is_empty());
                if cx.global::<ClosePolicy>().confirms(has_terminals, || workspace.panes.iter().any(|pane| pane.read(cx).terminals.iter().any(|terminal| terminal.read(cx).has_active_work()))) {
                    return Err(automation::Failure { code: -32003, message: "Anchor cleanup requires confirmation; close it in the UI" });
                }
                self.close_workspace(index, window, cx);
                return Ok(serde_json::json!({"window_id":change.window_id,"group_id":change.group_id,"ungrouped":true,"closed_workspace_id":anchor.to_string()}));
            }
            let members = group.members.iter().map(|id| id.to_string()).collect::<Vec<_>>();
            self.ungroup_preserving_order(id);
            cx.notify();
            return Ok(serde_json::json!({"group_id":change.group_id,"window_id":change.window_id,"workspace_ids":members,"ungrouped":true}));
        }
        if let Some(workspace_id) = &change.workspace_id {
            let member = self.workspace_state.entries.iter().find(|entry| entry.workspace.entity_id().to_string() == *workspace_id)
                .map(|entry| entry.workspace.entity_id()).ok_or(automation::Failure { code: -32004, message: "Workspace not found in window" })?;
            if method != "workspace.group.add" && !group.members.contains(&member) {
                return Err(automation::Failure { code: -32004, message: "Workspace is not a member of this group" });
            }
            let slot = self.group_anchor_slot(member);
            let selected = self.workspace_state.entries.get(self.workspace_state.active).map(|entry| entry.workspace.entity_id());
            let result = match method {
                "workspace.group.add" => self.workspace_state.groups.add(id, member, None),
                "workspace.group.remove" => { self.workspace_state.groups.remove(member); Ok(()) },
                "workspace.group.set_anchor" => self.workspace_state.groups.set_anchor(id, member),
                _ => unreachable!("validated membership method"),
            };
            result.map_err(|_| automation::Failure { code: -32004, message: "Group membership target no longer exists" })?;
            self.preserve_group_slot(slot, selected);
            self.scroll_to_workspace(self.workspace_state.active);
            cx.notify();
            let snapshot = self.automation_snapshot(change.window_id.clone(), false, cx);
            let group = snapshot["groups"].as_array().unwrap().iter().find(|group| group["id"] == change.group_id).cloned();
            return Ok(serde_json::json!({"window_id":change.window_id,"workspace_id":workspace_id,"group":group}));
        }
        if let Some(cwd) = &change.cwd { group.cwd = Some(cwd.clone()); }
        if change.clear_cwd == Some(true) { group.cwd = None; }
        if let Some(name) = &change.name { group.name = name.clone(); }
        if let Some(collapsed) = change.collapsed { group.collapsed = collapsed; }
        if let Some(pinned) = change.pinned { group.pinned = pinned; }
        self.scroll_to_workspace(self.workspace_state.active);
        cx.notify();
        let result = self.automation_snapshot(change.window_id.clone(), false, cx);
        Ok(result["groups"].as_array().unwrap().iter().find(|group| group["id"] == change.group_id).unwrap().clone())
    }

    fn set_group_directory(&mut self, reset: bool, window: &Window, cx: &mut Context<Self>) {
        let Some(entry) = self.workspace_state.entries.get(self.workspace_state.active) else { return; };
        let Some(id) = self.workspace_state.groups.containing(entry.workspace.entity_id()).map(|group| group.id) else {
            self.notice = Some("Select a workspace in a group first.".into()); cx.notify(); return;
        };
        let directory = if reset { None } else {
            let workspace = entry.workspace.read(cx);
            let Some(directory) = workspace.focused_index(window, cx).and_then(|index| workspace.panes.get(index))
                .and_then(|pane| pane.read(cx).cached_directory(cx)) else {
                self.notice = Some("The current terminal has not reported its directory yet.".into()); cx.notify(); return;
            };
            Some(directory)
        };
        self.notice = Some(directory.as_ref().map_or_else(|| "New group workspaces will follow the anchor directory.".into(), |path| format!("New group workspaces will start in {}", path.display())));
        self.workspace_state.groups.get_mut(id).unwrap().cwd = directory;
        cx.notify();
    }

    fn group_anchor_slot(&self, member: EntityId) -> Option<(u64, EntityId, usize)> {
        self.workspace_state.group_anchor_slot(member, |entry| entry.workspace.entity_id())
    }
    fn preserve_group_slot(&mut self, slot: Option<(u64, EntityId, usize)>, selected: Option<EntityId>) {
        self.workspace_state.preserve_group_slot(slot, selected, |entry| entry.workspace.entity_id());
    }

    fn workspace_order(&self, include_collapsed: bool) -> Vec<usize> {
        self.workspace_state.order(include_collapsed, |entry| (entry.workspace.entity_id(), entry.pinned))
    }

    fn ungroup_preserving_order(&mut self, id: u64) {
        if !self.workspace_state.ungroup(id,
            |entry| (entry.workspace.entity_id(), entry.pinned),
        ) { return; }
        self.scroll_to_workspace(self.workspace_state.active);
    }

    fn sidebar_order(&self) -> Vec<usize> { self.workspace_order(false) }

    fn scroll_to_workspace(&self, index: usize) {
        if let Some(row) = self.workspace_state.sidebar_row(index, |entry| (entry.workspace.entity_id(), entry.pinned)) {
            self.workspace_scroll.scroll_to_item(row);
        }
    }

    fn create_group(&mut self, empty: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.automation_available(cx).is_err() { return; }
        if empty { self.add_with_group(1, false, window, cx); }
        if self.workspace_state.entries.get(self.workspace_state.active).is_none() { return; }
        match self.workspace_state.create_group(self.workspace_state.active, empty, |entry| entry.workspace.entity_id()) {
            Ok(_) => { self.sidebar = true; self.scroll_to_workspace(self.workspace_state.active); }
            Err(error) => self.notice = Some(format!("Create group: {error}")),
        }
        cx.notify();
    }

    fn ungroup_active(&mut self, cx: &mut Context<Self>) {
        if self.automation_available(cx).is_err() { return; }
        if let Some(id) = self.workspace_state.entries.get(self.workspace_state.active).and_then(|entry| self.workspace_state.groups.containing(entry.workspace.entity_id())).map(|group| group.id) {
            self.ungroup_preserving_order(id);
            cx.notify();
        }
    }

    fn request_close_group(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.automation_available(cx).is_err() { return; }
        let Some(group) = self.workspace_state.entries.get(self.workspace_state.active).and_then(|entry| self.workspace_state.groups.containing(entry.workspace.entity_id())) else { return; };
        if group.members.len() == 1 {
            self.request_close_workspace(self.workspace_state.active, window, cx);
            return;
        }
        let id = group.id;
        let anchor = group.members[0];
        let members = group.members.clone();
        let message = format!("Close ‘{}’ and its {} workspaces?", group.name, members.len());
        let Some(permit) = cx.global::<PromptGate>().acquire() else { return; };
        self.close_prompt = Some(anchor);
        let answer = window.prompt(PromptLevel::Warning, &message,
            Some("This terminates the group's terminal processes and discards their output history. To keep the terminals, cancel and choose Ungroup Workspaces."),
            &["Cancel", "Close Group"], cx);
        cx.spawn_in(window, async move |entity, cx| {
            let confirmed = matches!(answer.await, Ok(1));
            drop(permit);
            let _ = cx.update(|window, cx| {
                entity.update(cx, |this, cx| {
                    if this.close_prompt != Some(anchor) { return; }
                    this.close_prompt = None;
                    if confirmed {
                        // Close only the members named by the original prompt
                        // which still belong to this group. Anchor goes last.
                        for member in members.into_iter().rev() {
                            if let Some(index) = this.workspace_state.group_close_index(id, member, |entry| entry.workspace.entity_id()) {
                                this.close_workspace(index, window, cx);
                            }
                        }
                    }
                    cx.notify();
                })
            });
        }).detach();
    }

    fn toggle_group_pin(&mut self, cx: &mut Context<Self>) {
        if self.automation_available(cx).is_err() { return; }
        match self.workspace_state.edit_group(self.workspace_state.active, rmux_core::workspace_lifecycle::GroupEdit::TogglePin, |entry| entry.workspace.entity_id()) {
            Ok(false) => return,
            Ok(true) => self.scroll_to_workspace(self.workspace_state.active),
            Err(error) => self.notice = Some(format!("Update group: {error}")),
        }
        cx.notify();
    }

    pub(super) fn show_startup_notice(&mut self, notice: String, cx: &mut Context<Self>) {
        self.notice = Some(notice);
        cx.notify();
    }

    fn move_workspace_to_group(&mut self, group: u64, cx: &mut Context<Self>) {
        if self.automation_available(cx).is_err() { return; }
        if self.workspace_state.entries.get(self.workspace_state.active).is_none() { return; }
        match self.workspace_state.move_to_group(self.workspace_state.active, group, |entry| entry.workspace.entity_id()) {
            Ok(()) => self.scroll_to_workspace(self.workspace_state.active),
            Err(error) => self.notice = Some(format!("Move workspace to group: {error}")),
        }
        cx.notify();
    }

    fn remove_workspace_from_group(&mut self, cx: &mut Context<Self>) {
        if self.automation_available(cx).is_err() { return; }
        match self.workspace_state.edit_group(self.workspace_state.active, rmux_core::workspace_lifecycle::GroupEdit::Remove, |entry| entry.workspace.entity_id()) {
            Ok(false) => return,
            Ok(true) => self.scroll_to_workspace(self.workspace_state.active),
            Err(error) => self.notice = Some(format!("Update group: {error}")),
        }
        cx.notify();
    }

    fn set_group_anchor(&mut self, cx: &mut Context<Self>) {
        if self.automation_available(cx).is_err() { return; }
        match self.workspace_state.edit_group(self.workspace_state.active, rmux_core::workspace_lifecycle::GroupEdit::SetAnchor, |entry| entry.workspace.entity_id()) {
            Ok(false) => return,
            Ok(true) => self.scroll_to_workspace(self.workspace_state.active),
            Err(error) => self.notice = Some(format!("Update group: {error}")),
        }
        cx.notify();
    }
    pub(super) fn automation_text_export(&self, target: EntityId, max_bytes: usize, selected: bool, preserve_wraps: bool, lines: Option<usize>, cx: &App) -> Option<rmux_terminal::session::HistoryExport> {
        self.workspace_state.entries.iter().find_map(|entry| entry.workspace.read(cx).panes.iter().find_map(|pane| {
            pane.read(cx).terminals.iter().find(|terminal| terminal.entity_id() == target)
                .map(|terminal| if selected { terminal.read(cx).session.selection_export(max_bytes, !preserve_wraps) } else if let Some(lines) = lines { terminal.read(cx).session.history_tail_export(lines, max_bytes) } else { terminal.read(cx).session.history_export(max_bytes) })
        }))
    }
    pub(super) fn automation_memory_sample(&self, target: EntityId, cx: &App) -> Option<rmux_terminal::session::MemorySample> {
        self.workspace_state.entries.iter().find_map(|entry| entry.workspace.read(cx).panes.iter().find_map(|pane| {
            pane.read(cx).terminals.iter().find(|terminal| terminal.entity_id() == target)
                .map(|terminal| terminal.read(cx).session.memory_sample())
        }))
    }
    pub(super) fn automation_read_text(&self, read: &automation::SurfaceRead, target: EntityId, cx: &App) -> Result<serde_json::Value, automation::Failure> {
        let terminal = self.workspace_state.entries.iter().find_map(|entry| entry.workspace.read(cx).panes.iter().find_map(|pane| {
            pane.read(cx).terminals.iter().find(|terminal| terminal.entity_id() == target).cloned()
        })).ok_or(automation::Failure { code: -32004, message: "Surface not found" })?;
        let frame = terminal.read(cx).session.try_frame()
            .map_err(|_| automation::Failure { code: -32007, message: "Terminal snapshot failed" })?
            .ok_or(automation::Failure { code: -32007, message: "Terminal is busy; retry snapshot" })?;
        let (text, truncated) = frame.text_with_limit(read.max_bytes);
        let mut result = serde_json::json!({"surface_id":read.surface_id,"scope":"viewport","columns":frame.columns,"rows":frame.rows.len(),"scrollback_offset":frame.scrollback_offset,"truncated":truncated});
        result["text"] = serde_json::Value::String(text);
        Ok(result)
    }
    pub(super) fn automation_notification(&mut self, change: &automation::NotificationChange, target: EntityId, cx: &mut Context<Self>) -> Result<serde_json::Value, automation::Failure> {
        let terminal = self.workspace_state.entries.iter().find_map(|entry| entry.workspace.read(cx).panes.iter().find_map(|pane| {
            pane.read(cx).terminals.iter().find(|terminal| terminal.entity_id() == target).cloned()
        })).ok_or(automation::Failure { code: -32004, message: "Surface not found" })?;
        if let Some(revision) = change.revision {
            if change.mark_read {
                let unread = terminal.read(cx).notifications.iter().any(|record| record.message.revision == revision && record.unread);
                if unread { self.change_notification(target, revision, true, cx); }
                return Ok(serde_json::json!({"surface_id":change.surface_id,"revision":revision,"marked_read":unread}));
            }
            let matches = terminal.read(cx).notifications.contains(revision);
            if matches { self.dismiss_notification(target, revision, cx); }
            Ok(serde_json::json!({"surface_id":change.surface_id,"revision":revision,"dismissed":matches}))
        } else {
            terminal.update(cx, |terminal, cx| {
                let revision = terminal.store_notification(change.title.clone().unwrap_or_default(), change.body.clone().unwrap_or_default(), cx);
                cx.emit(TabMetadataChanged);
                cx.notify();
                Ok(serde_json::json!({"surface_id":change.surface_id,"revision":revision}))
            })
        }
    }
    pub(super) fn automation_send_key(&self, input: &automation::SurfaceKey, target: EntityId, cx: &App) -> Result<serde_json::Value, automation::Failure> {
        self.automation_available(cx)?;
        let terminal = self.workspace_state.entries.iter().find_map(|entry| entry.workspace.read(cx).panes.iter().find_map(|pane| {
            pane.read(cx).terminals.iter().find(|terminal| terminal.entity_id() == target).cloned()
        })).ok_or(automation::Failure { code: -32004, message: "Surface not found" })?;
        let mods = rmux_terminal::keyboard::Modifiers { control: input.control, alt: input.alt, shift: input.shift };
        let queued = terminal.read(cx).session.try_key_tap_without_scroll(&input.key, mods)
            .map_err(|_| automation::Failure { code: -32006, message: "Key unavailable, terminal exited or input queue unavailable" })?
            .ok_or(automation::Failure { code: -32007, message: "Terminal is busy; no key queued" })?;
        Ok(serde_json::json!({"surface_id":input.surface_id,"queued_bytes":queued}))
    }
    pub(super) fn automation_send_text(&self, input: &automation::SurfaceText, target: EntityId, cx: &App) -> Result<serde_json::Value, automation::Failure> {
        self.automation_available(cx)?;
        let terminal = self.workspace_state.entries.iter().find_map(|entry| entry.workspace.read(cx).panes.iter().find_map(|pane| {
            pane.read(cx).terminals.iter().find(|terminal| terminal.entity_id() == target).cloned()
        })).ok_or(automation::Failure { code: -32004, message: "Surface not found" })?;
        let session = &terminal.read(cx).session;
        if input.paste {
            let queued = session.try_paste_without_scroll(&input.text)
                .map_err(|_| automation::Failure { code: -32006, message: "Terminal exited, paste pending or input queue unavailable" })?;
            if !queued { return Err(automation::Failure { code: -32007, message: "Terminal is busy; no paste queued" }); }
        } else {
            session.input_without_scroll(input.text.as_bytes())
                .map_err(|_| automation::Failure { code: -32006, message: "Terminal exited or input queue unavailable" })?;
        }
        Ok(serde_json::json!({"surface_id":input.surface_id,"queued_bytes":input.text.len(),"paste":input.paste}))
    }
    pub(super) fn automation_rename(&mut self, rename: &automation::WorkspaceRename, target: EntityId, window: &mut Window, cx: &mut Context<Self>) -> Result<serde_json::Value, automation::Failure> {
        self.automation_available(cx)?;
        let Some(index) = self.workspace_state.entries.iter().position(|entry| entry.workspace.entity_id() == target) else {
            return Err(automation::Failure { code: -32004, message: "Workspace not found" });
        };
        if self.workspace_state.entries[index].name != rename.name {
            self.workspace_state.entries[index].name.clone_from(&rename.name);
            if index == self.workspace_state.active { window.set_window_title(&format!("{} — rmux", rename.name)); }
            cx.notify();
        }
        Ok(serde_json::json!({"workspace_id":rename.workspace_id,"name":self.workspace_state.entries[index].name}))
    }
    pub(super) fn automation_pane_target(&self, id: EntityId, cx: &App) -> Option<(Entity<Workspace>, Entity<PaneView>)> {
        self.workspace_state.entries.iter().find_map(|entry| entry.workspace.read(cx).panes.iter()
            .find(|pane| pane.entity_id() == id).map(|pane| (entry.workspace.clone(), pane.clone())))
    }
    pub(super) fn automation_equalize_workspace(&mut self, target: EntityId, cx: &mut Context<Self>) -> Result<serde_json::Value, automation::Failure> {
        self.automation_available(cx)?;
        let workspace = self.workspace_state.entries.iter().find(|entry| entry.workspace.entity_id() == target)
            .map(|entry| entry.workspace.clone())
            .ok_or(automation::Failure { code: -32004, message: "Workspace not found" })?;
        let changed = workspace.update(cx, |workspace, cx| {
            if workspace.pane_state.zoomed.is_some() || workspace.spawning_split.is_some() || workspace.split_drag.is_some() {
                return Err(automation::Failure { code: -32005, message: "Workspace layout is busy or zoomed" });
            }
            let changed = workspace.pane_state.layout.as_mut().is_some_and(|tree| tree.equalize());
            if changed { cx.notify(); }
            Ok(changed)
        })?;
        Ok(serde_json::json!({"workspace_id":target.to_string(), "changed":changed}))
    }

    pub(super) fn automation_swap_panes(&mut self, swap: &automation::PaneSwap, target: EntityId, cx: &mut Context<Self>) -> Result<serde_json::Value, automation::Failure> {
        self.automation_available(cx)?;
        let (workspace, _) = self.automation_pane_target(target, cx)
            .ok_or(automation::Failure { code: -32004, message: "Pane not found" })?;
        let changed = workspace.update(cx, |workspace, cx| {
            if workspace.pane_state.zoomed.is_some() || workspace.spawning_split.is_some() || workspace.split_drag.is_some() {
                return Err(automation::Failure { code: -32005, message: "Pane layout is busy or zoomed" });
            }
            let current = workspace.panes.iter().position(|pane| pane.entity_id() == target).unwrap();
            let other = workspace.panes.iter().position(|pane| pane.entity_id().to_string() == swap.other_pane_id)
                .ok_or(automation::Failure { code: -32004, message: "Other pane not found in the same workspace" })?;
            Ok(current != other && workspace.swap_panes(current, other, cx))
        })?;
        Ok(serde_json::json!({"pane_id":swap.pane_id, "other_pane_id":swap.other_pane_id,
            "workspace_id":workspace.entity_id().to_string(), "changed":changed}))
    }

    pub(super) fn automation_resize_pane(&mut self, resize: &automation::PaneResize, target: EntityId, cx: &mut Context<Self>) -> Result<serde_json::Value, automation::Failure> {
        self.automation_available(cx)?;
        let (workspace, pane) = self.automation_pane_target(target, cx)
            .ok_or(automation::Failure { code: -32004, message: "Pane not found" })?;
        let changed = workspace.update(cx, |workspace, cx| {
            if workspace.pane_state.zoomed.is_some() || workspace.spawning_split.is_some() || workspace.split_drag.is_some() {
                return Err(automation::Failure { code: -32005, message: "Pane layout is busy or zoomed" });
            }
            let changed = workspace.pane_state.layout.as_mut().is_some_and(|tree| tree.resize_direction(&pane, resize.direction));
            if changed { cx.notify(); }
            Ok(changed)
        })?;
        if changed { cx.notify(); }
        Ok(serde_json::json!({"pane_id":resize.pane_id,"workspace_id":workspace.entity_id().to_string(),"changed":changed}))
    }

    pub(super) fn automation_prepare_split(&mut self, split: &automation::PaneSplit, target: EntityId, cx: &mut Context<Self>) -> Result<Box<dyn FnOnce() -> Result<PreparedTerminal> + Send>, automation::Failure> {
        self.automation_available(cx)?;
        if self.command_launching { return Err(automation::Failure { code: -32005, message: "Command startup busy" }); }
        let (workspace, pane) = self.automation_pane_target(target, cx)
            .ok_or(automation::Failure { code: -32004, message: "Pane not found" })?;
        if workspace.read(cx).panes.len() >= 64 || workspace.read(cx).spawning_split.is_some()
            || workspace.read(cx).pane_state.layout.as_ref().is_none_or(|tree| !tree.can_split(&pane)) {
            return Err(automation::Failure { code: -32005, message: "Pane split busy or limit reached" });
        }
        let destination = split.ssh_destination.clone();
        if let Some(destination) = &destination {
            ssh::command(destination).map_err(|message| automation::Failure { code: -32602, message })?;
        }
        let directory = split.cwd.clone().or_else(|| pane.read(cx).cached_directory(cx))
            .or_else(|| cx.global::<InitialDirectory>().0.clone());
        let scrollback = cx.global::<Preferences>().scrollback_bytes();
        let colors = cx.global::<Preferences>().terminal_colors();
        let palette = cx.global::<Preferences>().effective_palette();
        self.command_launching = true;
        Ok(Box::new(move || {
            let session = ssh::launch_sibling(destination.as_deref(), directory.as_deref(), scrollback)?;
            TerminalView::prepare(session, colors, palette)
        }))
    }
    pub(super) fn automation_finish_split(&mut self, split: &automation::PaneSplit, target: EntityId, prepared: Result<PreparedTerminal>, window: &mut Window, cx: &mut Context<Self>) -> Result<serde_json::Value, automation::Failure> {
        self.command_launching = false;
        let prepared = prepared.map_err(|_| automation::Failure { code: -32005, message: "Could not prepare terminal; check the executable and launch directory" })?;
        self.automation_publish_split(split, target, prepared, window, cx)
    }
    fn automation_publish_split(&mut self, split: &automation::PaneSplit, target: EntityId, prepared: PreparedTerminal, window: &mut Window, cx: &mut Context<Self>) -> Result<serde_json::Value, automation::Failure> {
        self.automation_available(cx)?;
        let Some((workspace, target_pane)) = self.automation_pane_target(target, cx) else {
            return Err(automation::Failure { code: -32004, message: "Pane not found" });
        };
        if workspace.read(cx).panes.len() >= 64 { return Err(automation::Failure { code: -32005, message: "Workspace pane limit reached" }); }
        if workspace.read(cx).pane_state.layout.as_ref().is_none_or(|tree| !tree.can_split(&target_pane)) {
            return Err(automation::Failure { code: -32005, message: "Pane split nesting limit reached" });
        }
        let axis = match split.direction { automation::SplitDirection::Left | automation::SplitDirection::Right => SplitAxis::Columns, automation::SplitDirection::Up | automation::SplitDirection::Down => SplitAxis::Rows };
        let before = matches!(split.direction, automation::SplitDirection::Left | automation::SplitDirection::Up);
        let prepared = {
            let pane = target_pane.read(cx);
            (TerminalSeed::Ready(prepared, pane.active_cell_metrics(cx), pane.active_font_family(cx)),
                split.ssh_destination.as_ref().map(|destination| format!("SSH · {destination}")))
        };
        let pane_id = workspace.update(cx, |workspace, cx| workspace.add_seed_at(Some(target), axis, before, split.cwd.clone(), false, Some(prepared), window, cx))
            .ok_or(automation::Failure { code: -32005, message: "Could not start pane shell" })?;
        let pane = workspace.read(cx).panes.iter().find(|pane| pane.entity_id() == pane_id).unwrap();
        let surface = pane.read(cx).active_terminal_id().unwrap();
        if split.focus || split.activate { self.focus_terminal(workspace.entity_id(), surface, window, cx); }
        if split.activate { activate_destination(window, cx); }
        cx.notify();
        Ok(serde_json::json!({"pane_id":pane_id.to_string(),"surface_id":surface.to_string(),"workspace_id":workspace.entity_id().to_string()}))
    }

    pub(super) fn automation_zoom_pane(&mut self, zoom: &automation::PaneZoom, target: EntityId, window: &mut Window, cx: &mut Context<Self>) -> Result<serde_json::Value, automation::Failure> {
        self.automation_available(cx)?;
        let Some((workspace, pane)) = self.automation_pane_target(target, cx) else {
            return Err(automation::Failure { code: -32004, message: "Pane not found" });
        };
        if zoom.zoomed {
            let Some(terminal) = pane.read(cx).active_terminal_id() else {
                return Err(automation::Failure { code: -32004, message: "Pane has no terminal" });
            };
            self.focus_terminal(workspace.entity_id(), terminal, window, cx);
        }
        workspace.update(cx, |workspace, cx| {
            let next = if zoom.zoomed { Some(pane.entity_id()) }
                else { workspace.pane_state.zoomed.filter(|id| *id != pane.entity_id()) };
            if workspace.pane_state.zoomed != next { workspace.pane_state.zoomed = next; cx.notify(); }
        });
        Ok(serde_json::json!({"pane_id":zoom.pane_id,"zoomed":workspace.read(cx).pane_state.zoomed == Some(pane.entity_id())}))
    }
    pub(super) fn automation_children(&self, window_id: String, surfaces: bool, workspace_filter: Option<&str>, history: bool, cx: &App) -> Vec<serde_json::Value> {
        let mut items = Vec::new();
        for entry in &self.workspace_state.entries {
            let workspace_id = entry.workspace.entity_id().to_string();
            if workspace_filter.is_some_and(|id| id != workspace_id) { continue; }
            let workspace = entry.workspace.read(cx);
            for (index, entity) in workspace.panes.iter().enumerate() {
                let pane = entity.read(cx);
                let pane_id = entity.entity_id().to_string();
                if surfaces {
                    items.extend(pane.automation_surfaces(&window_id, &workspace_id, &pane_id, history, cx));
                } else {
                    items.push(serde_json::json!({
                        "id":pane_id, "window_id":window_id, "workspace_id":workspace_id,
                        "index":index, "surface_count":pane.terminals.len(),
                        "selected_surface_id":pane.active_terminal_id().map(|id| id.to_string()),
                        "zoomed":workspace.pane_state.zoomed == Some(entity.entity_id()),
                    }));
                }
            }
        }
        items
    }
    pub(super) fn automation_contains_workspace(&self, id: EntityId) -> bool {
        self.workspace_state.entries.iter().any(|entry| entry.workspace.entity_id() == id)
    }
    pub(super) fn automation_surface_target(&self, id: EntityId, cx: &App) -> Option<(EntityId, EntityId)> {
        self.workspace_state.entries.iter().find_map(|entry| {
            entry.workspace.read(cx).panes.iter().find_map(|pane| {
                pane.read(cx).terminals.iter().find(|terminal| terminal.entity_id() == id)
                    .map(|terminal| (entry.workspace.entity_id(), terminal.entity_id()))
            })
        })
    }
    pub(super) fn automation_focus_surface(&mut self, focus: &automation::SurfaceFocus, target: EntityId, window: &mut Window, cx: &mut Context<Self>) -> Result<serde_json::Value, automation::Failure> {
        self.automation_available(cx)?;
        let Some((workspace, terminal)) = self.automation_surface_target(target, cx) else {
            return Err(automation::Failure { code: -32004, message: "Surface not found" });
        };
        self.focus_terminal(workspace, terminal, window, cx);
        if focus.activate { activate_destination(window, cx); }
        Ok(serde_json::json!({"surface_id":focus.surface_id,"workspace_id":workspace.to_string(),"selected":true}))
    }
    fn automation_available(&self, cx: &App) -> Result<(), automation::Failure> {
        if self.benchmark || self.restore_job.is_some() || self.window_close_prompt
            || self.close_prompt.is_some() || self.rename.is_some() || self.palette.is_some() || self.workspace_menu.is_some()
            || cx.global::<Shutdowns>().quitting || cx.global::<Shutdowns>().quit_prompt
            || cx.global::<PromptGate>().is_active()
            || self.workspace_state.entries.iter().any(|entry| entry.workspace.read(cx).panes.iter().any(|pane| pane.read(cx).automation_busy(cx))) {
            return Err(automation::Failure { code: -32003, message: "Window is busy; retry after the current interaction" });
        }
        Ok(())
    }
    pub(super) fn automation_focus_history(&mut self, navigation: &automation::FocusHistory, window: &mut Window, cx: &mut Context<Self>) -> Result<serde_json::Value, automation::Failure> {
        self.automation_available(cx)?;
        let target = self.navigate_focus(matches!(navigation.direction, automation::FocusDirection::Forward), window, cx);
        if target.is_some() && navigation.activate { activate_destination(window, cx); }
        Ok(serde_json::json!({"window_id": navigation.window_id, "moved": target.is_some(),
            "surface_id": target.map(|id| id.as_u64().to_string())}))
    }
    pub(super) fn automation_select(&mut self, selection: &automation::Selection, target: EntityId, window: &mut Window, cx: &mut Context<Self>) -> Result<serde_json::Value, automation::Failure> {
        self.automation_available(cx)?;
        let Some(index) = self.workspace_state.entries.iter().position(|entry| entry.workspace.entity_id() == target) else {
            return Err(automation::Failure { code: -32004, message: "Workspace not found" });
        };
        self.select(index, window, cx);
        if selection.activate { activate_destination(window, cx); }
        Ok(serde_json::json!({"workspace_id":selection.workspace_id,"selected":true}))
    }
    pub(super) fn automation_snapshot(&self, window_id: String, include_workspaces: bool, cx: &App) -> serde_json::Value {
        let order = self.workspace_order(true);
        let workspaces: Vec<_> = order.iter().enumerate().filter(|_| include_workspaces).map(|(position, index)| {
            let entry = &self.workspace_state.entries[*index];
            let group = self.workspace_state.groups.containing(entry.workspace.entity_id());
            let workspace = entry.workspace.read(cx);
            let mut result = serde_json::json!({
                "id":entry.workspace.entity_id().to_string(), "window_id":window_id,
                "index":position, "name":entry.name, "description":entry.description,
                "group_id":group.map(|group| format!("workspace_group:{}", group.id)),
                "group_anchor":group.is_some_and(|group| group.members.first() == Some(&entry.workspace.entity_id())),
                "icon":entry.icon, "color":entry.color.map(|color| format!("#{color:06x}")),
                "pinned":entry.pinned, "selected":*index == self.workspace_state.active,
                "pane_count":workspace.panes.len(),
                "zoomed_pane_id":workspace.pane_state.zoomed.map(|id| id.to_string()),
                "terminal_count":workspace.panes.iter().map(|pane| pane.read(cx).terminals.len()).sum::<usize>(),
            });
            result["layout"] = workspace.pane_state.layout.as_ref().map(|tree| automation::layout_snapshot(tree, &|pane| pane.entity_id().to_string())).unwrap_or(serde_json::Value::Null);
            result
        }).collect();
        let groups: Vec<_> = order.iter().filter_map(|index| {
            let id = self.workspace_state.entries[*index].workspace.entity_id();
            self.workspace_state.groups.containing(id).filter(|group| group.members.first() == Some(&id))
        }).map(|group| serde_json::json!({
            "id":format!("workspace_group:{}", group.id), "window_id":window_id,
            "name":group.name, "color":group.color.map(|color| format!("#{color:06x}")),
            "external_id":group.external_id,
            "icon":group.icon, "pinned":group.pinned, "collapsed":group.collapsed,
            "generated_anchor":group.generated_anchor,
            "cwd":group.cwd,
            "anchor_workspace_id":group.members.first().map(|id| id.to_string()),
            "workspace_ids":group.members.iter().map(|id| id.to_string()).collect::<Vec<_>>(),
        })).collect();
        let mut result = serde_json::json!({
            "id":window_id, "workspace_count":self.workspace_state.entries.len(),
            "selected_workspace_id":self.workspace_state.entries.get(self.workspace_state.active).map(|entry| entry.workspace.entity_id().to_string()),
            "groups":groups,
        });
        result["workspaces"] = serde_json::Value::Array(workspaces);
        result
    }
    fn run_custom_command(&mut self, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(command) = cx.global::<Preferences>().commands.iter().find(|command| command.name == name).cloned() else {
            self.notice = Some("Custom command is no longer configured".into()); cx.notify(); return;
        };
        if command.mode == preferences::CommandMode::Background {
            let directory = self.workspace_state.entries.get(self.workspace_state.active).and_then(|entry| {
                let workspace = entry.workspace.read(cx);
                workspace.focused_index(window, cx).and_then(|index| workspace.panes[index].read(cx).cached_directory(cx))
            });
            let name = command.name.clone();
            let executor = cx.background_executor().clone();
            cx.spawn(async move |entity, cx| {
                let result = match executor.spawn(async move { crate::background_command::launch(&command, directory.as_deref()) }).await {
                    Ok(child) => executor.spawn(child.reap()).await,
                    Err(error) => Err(error),
                };
                if let Err(error) = result {
                    let _ = entity.update(cx, |this, cx| { this.notice = Some(format!("{name}: {error:#}")); cx.notify(); });
                }
            }).detach();
        } else {
            self.launch_command(command, None, window, cx);
        }
    }
    fn launch_command(&mut self, command: preferences::CustomCommand, remote: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        if self.command_launching { self.notice = Some("A command is already starting in this window".into()); cx.notify(); return; }
        if self.automation_available(cx).is_err() || self.workspace_state.entries.len() >= 64 { self.notice = Some("Window is busy or has reached its workspace limit".into()); cx.notify(); return; }
        let directory = command.cwd.clone().or_else(|| self.workspace_state.entries.get(self.workspace_state.active).and_then(|entry| {
            let workspace = entry.workspace.read(cx);
            workspace.panes.iter().find(|pane| Some(pane.entity_id()) == entry.last_pane).or_else(|| workspace.panes.first())
                .and_then(|pane| pane.read(cx).cached_directory(cx))
        }));
        let scrollback = cx.global::<Preferences>().scrollback_bytes();
        let colors = cx.global::<Preferences>().terminal_colors();
        let palette = cx.global::<Preferences>().effective_palette();
        let executor = cx.background_executor().clone();
        self.command_launching = true;
        self.notice = Some(format!("Starting {}…", command.name));
        cx.notify();
        cx.spawn_in(window, async move |entity, cx| {
            let name = command.name;
            let mut result = executor.spawn(async move {
                let argv: Vec<_> = command.argv.into_iter().map(std::ffi::OsString::from).collect();
                let environment: Vec<_> = command.env.into_iter().map(|(name, value)| (name.into(), value.into())).collect();
                let mut session = Session::command_in_with_environment(&argv, 80, 24, directory.as_deref(), scrollback, &environment)?;
                if let Some(destination) = remote { session.set_remote_destination(&destination)?; }
                TerminalView::prepare(session, colors, palette)
            }).await;
            loop {
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
                this.command_launching = false;
                match result {
                    Ok(prepared) if this.automation_available(cx).is_ok() && this.workspace_state.entries.len() < 64 => {
                        let settings = cx.global::<Preferences>();
                        let seed = TerminalSeed::Ready(prepared, CellMetrics::new(settings.font_size), settings.font_family.clone().into());
                        let Some(index) = this.add_configured(1, false, None, None, false, Some((seed, Some(name.clone()))), window, cx) else { return; };
                        this.workspace_state.entries[index].name = name;
                        this.select(index, window, cx);
                        this.notice = None;
                    }
                    Ok(_) => this.notice = Some("Command terminal closed because the window became unavailable".into()),
                    Err(error) => this.notice = Some(format!("Could not start {name}: {error:#}")),
                }
                cx.notify();
            }));
        }).detach();
    }

    fn toggle_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(state) = self.settings_state.take() {
            if state.focus.is_focused(window) {
                if let Some(origin) = state.origin { origin.focus(window); } else { self.select(self.workspace_state.active, window, cx); }
            }
        } else {
            if self.notification_navigation.is_some() { self.toggle_notifications(window, cx); }
            let state = settings_view::State { focus: cx.focus_handle(), origin: window.focused(cx), selected: 0, scroll: ScrollHandle::new() };
            state.focus.focus(window);
            self.settings_state = Some(state);
        }
        cx.notify();
    }
    pub(super) fn settings_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let Some(state) = &mut self.settings_state else { return; };
        let key = event.keystroke.key.as_str();
        if event.keystroke.modifiers.platform || event.keystroke.modifiers.control || event.keystroke.modifiers.alt
            || (event.keystroke.modifiers.shift && key != "tab") { return; }
        let next = if matches!(key, "pageup" | "pagedown") {
            command_palette::paged_result(state.selected, settings_view::KEYS.len() + settings_view::FOOTER_COUNT,
                key == "pageup", f32::from(state.scroll.bounds().size.height), |index| {
                    let item = settings_view::scroll_index(index);
                    let Some(bounds) = state.scroll.bounds_for_item(item) else { return 1.; };
                    let next = state.scroll.bounds_for_item(settings_view::scroll_index(index + 1));
                    f32::from(next.map(|next| next.top() - bounds.top()).unwrap_or(bounds.size.height)).max(1.)
                })
        } else {
            settings_view::next_control(state.selected, key, event.keystroke.modifiers.shift)
        };
        if let Some(index) = next {
            state.selected = index;
            // Footer actions stay visible without moving the settings list.
            if index < settings_view::KEYS.len() {
                state.scroll.scroll_to_item(settings_view::scroll_index(index));
            }
            cx.notify();
        } else if key == "escape" {
            self.toggle_settings(window, cx);
        } else if matches!(key, "left" | "right" | "enter" | "space") {
            let selected = state.selected;
            let control = settings_view::KEYS.iter().chain(settings_view::FOOTER_KEYS).nth(selected).copied();
            let input = control.map(|control| settings_view::control_input(control, key))
                .unwrap_or(settings_view::ControlInput::None);
            if input == settings_view::ControlInput::None {
                cx.stop_propagation();
                return;
            }
            if selected < settings_view::KEYS.len() {
                if settings_view::KEYS[selected] == "file_editor" {
                    self.open_settings_file(cx);
                    cx.stop_propagation();
                    return;
                }
                if settings_view::KEYS[selected] == "import_theme" {
                    self.choose_terminal_theme(window, cx);
                    cx.stop_propagation();
                    return;
                }
                if settings_view::KEYS[selected] == "font_family" {
                    self.show_font_picker(true, window, cx);
                    cx.stop_propagation();
                    return;
                }
                if settings_view::KEYS[selected] == "close" && cx.global::<Preferences>().close_policy_overridden() {
                    self.notice = Some("Close confirmation is controlled by RMUX_CONFIRM_CLOSE. Remove the override and restart rmux to change it here.".into());
                    cx.notify();
                    cx.stop_propagation();
                    return;
                }
                let mut settings = cx.global::<Preferences>().clone();
                if let settings_view::ControlInput::Adjust(delta) = input {
                    settings_view::adjust(&mut settings, settings_view::KEYS[selected], delta);
                }
                if !settings.same_saved_appearance(cx.global::<Preferences>()) {
                    cx.set_global(settings.confirm_close);
                    cx.set_global(settings);
                }
                cx.notify();
            } else if !cx.global::<preferences::ReloadState>().0 {
                if let Some(action) = settings_view::footer_action(selected - settings_view::KEYS.len()) {
                    window.dispatch_action(action, cx);
                }
            }
        }
        // Plain typing belongs to this panel, never the shell behind it.
        cx.stop_propagation();
    }

    fn save_appearance_settings(&mut self, cx: &mut Context<Self>) {
        if cx.global::<preferences::ReloadState>().0 { return; }
        cx.global_mut::<preferences::ReloadState>().0 = true;
        let settings = cx.global::<Preferences>().clone();
        let executor = cx.background_executor().clone();
        self.notice = Some("Saving appearance settings…".into());
        cx.notify();
        cx.spawn(async move |entity, cx| {
            let snapshot = settings.clone();
            let result = executor.spawn(async move { snapshot.save_appearance_with_receipt() }).await;
            let _ = cx.update(|cx| {
                cx.global_mut::<preferences::ReloadState>().0 = false;
                if let Ok(receipt) = &result {
                    if receipt.has_changes() { cx.set_global(preferences::LastSettingsSave(Some(receipt.clone()))); }
                    cx.set_global(preferences::AppearanceBaseline(settings.clone()));
                }
                let _ = entity.update(cx, |this, cx| {
                    this.notice = Some(match result {
                        Ok(receipt) if settings.same_saved_appearance(cx.global::<Preferences>()) => format!("Settings saved to {}", receipt.path.display()),
                        Ok(_) => "Earlier settings saved. Your newer changes are not saved yet; choose Save again to keep them.".into(),
                        Err(error) => format!("Settings not saved: {error:#}"),
                    });
                    cx.notify();
                });
            });
        }).detach();
    }

    fn choose_terminal_theme(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if cx.global::<preferences::ReloadState>().0 { return; }
        let Some(permit) = cx.global::<PromptGate>().acquire() else { return; };
        let picker = cx.prompt_for_paths(PathPromptOptions {
            files: true, directories: false, multiple: false, prompt: Some("Import Terminal Theme".into()),
        });
        cx.spawn_in(window, async move |entity, cx| {
            let result: anyhow::Result<Option<std::path::PathBuf>> = async {
                Ok(picker.await??.and_then(|paths| paths.into_iter().next()))
            }.await;
            drop(permit);
            let _ = cx.update(|_, cx| entity.update(cx, |this, cx| match result {
                Ok(Some(path)) => this.import_terminal_theme(path, cx),
                Ok(None) => {},
                Err(error) => { this.notice = Some(format!("Choose theme: {error:#}")); cx.notify(); },
            }));
        }).detach();
    }
    fn import_terminal_theme(&mut self, path: std::path::PathBuf, cx: &mut Context<Self>) {
        if cx.global::<preferences::ReloadState>().0 { return; }
        cx.global_mut::<preferences::ReloadState>().0 = true;
        let current = cx.global::<Preferences>().clone();
        let executor = cx.background_executor().clone();
        self.notice = Some("Importing terminal theme…".into());
        cx.notify();
        cx.spawn(async move |entity, cx| {
            let snapshot = current.clone();
            let result = executor.spawn(async move { snapshot.import_theme(path) }).await;
            let _ = cx.update(|cx| {
                cx.global_mut::<preferences::ReloadState>().0 = false;
                // A closed originating window must not apply a delayed import.
                let _ = entity.update(cx, |this, cx| {
                    this.notice = Some(match result {
                        Ok(_) if !current.same_saved_appearance(cx.global::<Preferences>()) =>
                            "Settings changed during import; your newer edits were kept.".into(),
                        Ok(next) => {
                            cx.set_global(next);
                            "Theme imported for this session. Save these settings to keep it; configured color overrides remain active.".into()
                        },
                        Err(error) => format!("Theme unchanged: {error:#}"),
                    });
                    cx.notify();
                });
            });
        }).detach();
    }
    fn revert_appearance_settings(&mut self, cx: &mut Context<Self>) {
        if cx.global::<preferences::ReloadState>().0 { return; }
        let mut settings = cx.global::<Preferences>().clone();
        let baseline = cx.global::<preferences::AppearanceBaseline>();
        if !baseline.has_changes(&settings) { return; }
        settings.restore_appearance(&baseline.0);
        cx.set_global(settings.confirm_close);
        cx.set_global(settings);
        self.notice = Some("Unsaved settings reverted. The settings file was not changed.".into());
        cx.notify();
    }
    fn undo_settings_save(&mut self, cx: &mut Context<Self>) {
        if cx.global::<preferences::ReloadState>().0 { return; }
        let Some(receipt) = cx.global::<preferences::LastSettingsSave>().0.clone() else { return; };
        let current = cx.global::<Preferences>().clone();
        let unsaved = cx.global::<preferences::AppearanceBaseline>().has_changes(&current);
        cx.global_mut::<preferences::ReloadState>().0 = true;
        self.notice = Some("Undoing last settings save…".into());
        cx.notify();
        let executor = cx.background_executor().clone();
        cx.spawn(async move |entity, cx| {
            let snapshot = current.clone();
            let result = executor.spawn(async move {
                receipt.undo().map(|counts| (counts, snapshot.reload()))
            }).await;
            let _ = cx.update(|cx| {
                cx.global_mut::<preferences::ReloadState>().0 = false;
                let notice = match result {
                    Err(error) => format!("Settings save not undone: {error:#}"),
                    Ok(((restored, skipped), loaded)) => {
                        cx.set_global(preferences::LastSettingsSave::default());
                        let summary = format!("Undid {restored} saved settings; kept {skipped} values changed since the save.");
                        if unsaved || !current.same_saved_appearance(cx.global::<Preferences>()) {
                            format!("{summary} Your session edits were kept. Reload Settings to apply the restored file.")
                        } else {
                            match loaded {
                                Ok(next) => {
                                    let socket_notice = cx.global::<preferences::StartupSocket>().reload_notice(&next.socket_directory)
                            .or_else(|| shell_history::reload_notice(&next.shell_history_directory));
                                    cx.set_global(preferences::AppearanceBaseline(next.clone()));
                                    cx.set_global(next.confirm_close);
                                    cx.set_global(next);
                                    shortcuts::install(cx);
                                    if let Some(notice) = socket_notice { format!("{summary} {notice}") }
                                    else { summary }
                                },
                                Err(error) => format!("{summary} The file was restored, but session settings could not reload: {error:#}"),
                            }
                        }
                    },
                };
                let _ = entity.update(cx, |this, cx| { this.notice = Some(notice); cx.notify(); });
            });
        }).detach();
    }
    fn reload_settings(&mut self, cx: &mut Context<Self>) {
        if cx.global::<preferences::ReloadState>().0 {
            return;
        }
        cx.global_mut::<preferences::ReloadState>().0 = true;
        let current = cx.global::<Preferences>().clone();
        let executor = cx.background_executor().clone();
        self.notice = Some("Reloading settings…".into());
        cx.notify();
        cx.spawn(async move |entity, cx| {
            let snapshot = current.clone();
            let result = executor.spawn(async move { snapshot.reload() }).await;
            let _ = cx.update(|cx| {
                cx.global_mut::<preferences::ReloadState>().0 = false;
                let notice = match result {
                    Ok(_) if !current.same_saved_appearance(cx.global::<Preferences>()) => {
                        "Settings changed while reloading. Your newer edits were kept; reload again to replace them with the file settings.".into()
                    }
                    Ok(next) => {
                        let socket_notice = cx.global::<preferences::StartupSocket>().reload_notice(&next.socket_directory)
                            .or_else(|| shell_history::reload_notice(&next.shell_history_directory));
                        cx.set_global(preferences::AppearanceBaseline(next.clone()));
                        cx.set_global(next.confirm_close);
                        cx.set_global(next);
                        shortcuts::install(cx);
                        if let Some(notice) = socket_notice {
                            format!("Settings reloaded. {notice}")
                        } else {
                            "Settings reloaded. Fonts using previous defaults updated; per-terminal overrides retained.".into()
                        }
                    }
                    Err(error) => format!("Settings unchanged: {error:#}"),
                };
                let _ = entity.update(cx, |this, cx| { this.notice = Some(notice); cx.notify(); });
            });
        }).detach();
    }
    fn open_settings_file(&mut self, cx: &mut Context<Self>) {
        if cx.global::<preferences::ReloadState>().0 {
            return;
        }
        cx.global_mut::<preferences::ReloadState>().0 = true;
        let preferences = cx.global::<Preferences>().clone();
        let executor = cx.background_executor().clone();
        cx.spawn(async move |entity, cx| {
            let result = executor
                .spawn(async move { preferences.prepare_file() })
                .await;
            let _ = cx.update(|cx| {
                cx.global_mut::<preferences::ReloadState>().0 = false;
                match result {
                    Ok(path) => {
                        let editor = external_editor::settings_editor(cx.global::<Preferences>().file_editor.as_ref());
                        if let Some(editor) = editor {
                            let _ = entity.update(cx, |this, cx| {
                                this.launch_external_editor(editor, EditLocalFile { path, line: None, column: None }, cx);
                            });
                        } else { cx.open_with_system(&path); }
                    },
                    Err(error) => {
                        let _ = entity.update(cx, |this, cx| {
                            this.notice = Some(format!("Could not open settings: {error:#}"));
                            cx.notify();
                        });
                    }
                }
            });
        })
        .detach();
    }
    pub(super) fn should_confirm_close(&self, cx: &App) -> bool {
        let has_terminals = self.workspace_state.entries.iter().any(|entry| {
            entry
                .workspace
                .read(cx)
                .panes
                .iter()
                .any(|pane| !pane.read(cx).terminals.is_empty())
        });
        cx.global::<ClosePolicy>()
            .confirms(has_terminals, || self.has_active_work(cx))
    }
    pub(super) fn has_active_work(&self, cx: &App) -> bool {
        self.workspace_state.entries.iter().any(|entry| {
            entry.workspace.read(cx).panes.iter().any(|pane| {
                pane.read(cx)
                    .terminals
                    .iter()
                    .any(|terminal| terminal.read(cx).has_active_work())
            })
        })
    }
    fn request_process_close(&self, cx: &App) {
        if let Some(cancel) = &self.workspace_launch_cancel { cancel.0.store(true, Ordering::Release); }
        for entry in &self.workspace_state.entries { entry.workspace.read(cx).request_process_close(cx); }
    }

    fn request_close_window(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.should_confirm_close(cx) || cx.global::<Shutdowns>().quitting {
            self.request_process_close(cx);
            window.remove_window();
            return;
        }
        if self.window_close_prompt || self.close_prompt.is_some() {
            return;
        }
        let Some(permit) = cx.global::<PromptGate>().acquire() else {
            return;
        };
        self.window_close_prompt = true;
        let answer = window.prompt(PromptLevel::Warning, "Close window?",
            Some("Closing this window terminates the terminal processes in all of its workspaces and discards their output history."),
            &["Cancel", "Close Window"], cx);
        cx.spawn_in(window, async move |entity, cx| {
            let confirmed = matches!(answer.await, Ok(1));
            drop(permit);
            let _ = cx.update(|window, cx| {
                entity.update(cx, |this, cx| {
                    this.window_close_prompt = false;
                    if confirmed {
                        this.request_process_close(cx);
                        window.remove_window();
                    }
                })
            });
        })
        .detach();
    }
    fn observe_metadata(
        workspace: &Entity<Workspace>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Subscription {
        cx.subscribe_in(
            workspace,
            window,
            |this, _, _: &WorkspaceMetadataChanged, window, cx| {
                this.record_terminal_focus(window, cx);
                cx.notify();
            },
        )
    }

    fn record_terminal_focus(&mut self, window: &Window, cx: &App) {
        let Some(entry) = self.workspace_state.entries.get(self.workspace_state.active) else {
            return;
        };
        let workspace = entry.workspace.read(cx);
        let Some(index) = workspace.focused_index(window, cx) else {
            return;
        };
        let Some(id) = workspace.panes[index].read(cx).active_terminal_id() else {
            return;
        };
        self.record_terminal_visit(id);
    }

    fn record_terminal_visit(&mut self, id: EntityId) {
        self.focus_trail.record(id);
        if self.focus_history.last() == Some(&id) {
            return;
        }
        self.focus_history.retain(|previous| *previous != id);
        self.focus_history.push(id);
        if self.focus_history.len() > 32 {
            self.focus_history.remove(0);
        }
    }

    fn navigate_focus(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) -> Option<EntityId> {
        self.record_terminal_focus(window, cx);
        let mut live = Vec::new();
        for (index, entry) in self.workspace_state.entries.iter().enumerate() {
            for pane in &entry.workspace.read(cx).panes {
                for (tab, terminal) in pane.read(cx).terminals.iter().enumerate() {
                    live.push((terminal.entity_id(), index, pane.clone(), tab));
                }
            }
        }
        let Some(target) = self.focus_trail.step(forward, |id| live.iter().any(|(candidate, ..)| *candidate == id)) else { return None; };
        let Some((_, index, pane, tab)) = live.into_iter().find(|(id, ..)| *id == target) else { return None; };
        self.remember_focus(window, cx);
        self.workspace_state.entries[index].last_pane = Some(pane.entity_id());
        self.workspace_state.entries[index].workspace.update(cx, |workspace, cx| {
            workspace.pane_state.zoomed = None;
            cx.notify();
        });
        pane.update(cx, |pane, cx| pane.select(tab, window, cx));
        self.select(index, window, cx);
        self.record_terminal_focus(window, cx);
        Some(target)
    }

    fn last_terminal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.record_terminal_focus(window, cx);
        let current = self.focus_history.last().copied();
        let mut live = Vec::new();
        for (index, entry) in self.workspace_state.entries.iter().enumerate() {
            for pane in &entry.workspace.read(cx).panes {
                for (tab, terminal) in pane.read(cx).terminals.iter().enumerate() {
                    live.push((terminal.entity_id(), index, pane.clone(), tab));
                }
            }
        }
        self.focus_history
            .retain(|id| live.iter().any(|(candidate, ..)| candidate == id));
        let target = self
            .focus_history
            .iter()
            .rev()
            .filter(|id| Some(**id) != current)
            .find_map(|id| live.iter().find(|(candidate, ..)| candidate == id))
            .cloned();
        if let Some((_, index, pane, tab)) = target {
            self.remember_focus(window, cx);
            self.workspace_state.entries[index].last_pane = Some(pane.entity_id());
            self.workspace_state.entries[index].workspace.update(cx, |workspace, cx| {
                workspace.pane_state.zoomed = None;
                cx.notify();
            });
            // Select the intended tab before select() focuses this pane. Otherwise
            // its old active tab would briefly gain focus and lose unread state.
            pane.update(cx, |pane, cx| pane.select(tab, window, cx));
            self.select(index, window, cx);
            self.record_terminal_focus(window, cx);
        }
    }
    fn move_workspace_to_next_window(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.workspace_state.entries.is_empty() || cx.global::<Shutdowns>().quitting {
            return;
        }
        let current = window.window_handle();
        let windows = cx.windows();
        let start = windows.iter().position(|candidate| *candidate == current).unwrap_or(0);
        let target = (1..=windows.len()).find_map(|offset| {
            let candidate = windows[(start + offset) % windows.len()];
            if candidate == current { return None; }
            candidate.downcast::<Self>().filter(|candidate| {
                candidate.read(cx).is_ok_and(|root| !root.benchmark)
            })
        });
        let Some(target) = target else {
            self.notice = Some("Open another rmux window to move this workspace there.".into());
            cx.notify();
            return;
        };
        let source = self.workspace_state.entries[self.workspace_state.active].workspace.entity_id();
        self.move_workspace_to_window(source, target.into(), window, cx);
    }
    fn move_workspace_to_window(
        &mut self,
        source: EntityId,
        target: AnyWindowHandle,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Err(error) = self.try_move_workspace_to_window(source, target, true, window, cx) {
            self.notice = Some(format!("Move workspace: {error}"));
            cx.notify();
        }
    }
    pub(super) fn try_move_workspace_to_window(&mut self, source: EntityId, target: AnyWindowHandle, activate: bool, window: &mut Window, cx: &mut Context<Self>) -> Result<()> {
        self.automation_available(cx).map_err(|failure| anyhow::anyhow!(failure.message))?;
        let index = self.workspace_state.entries.iter().position(|entry| entry.workspace.entity_id() == source)
            .ok_or_else(|| anyhow::anyhow!("workspace not found"))?;
        if target == window.window_handle() { return Ok(()); }
        let target = target.downcast::<Self>().ok_or_else(|| anyhow::anyhow!("invalid destination window"))?;
        self.remember_focus(window, cx);
        let entry = &self.workspace_state.entries[index];
        let workspace = entry.workspace.clone();
        let name = entry.name.clone();
        let description = entry.description.clone();
        let project_directory = entry.project_directory.clone();
        let color = entry.color;
        let icon = entry.icon.clone();
        let pinned = entry.pinned;
        let last_pane = entry.last_pane;
        let result = target.update(cx, move |destination, window, cx| -> Result<()> {
            destination.automation_available(cx).map_err(|failure| anyhow::anyhow!(failure.message))?;
            destination.remember_focus(window, cx);
            workspace.update(cx, |workspace, cx| workspace.rebind_window(window, cx));
            let closed = Self::observe_workspace(&workspace, window, cx);
            let metadata = Self::observe_metadata(&workspace, window, cx);
            let index = destination.workspace_state.entries.len();
            destination.workspace_state.entries.insert(index, Entry {
                workspace, name, description, project_directory, color, icon, pinned, last_pane,
                _closed: closed, _metadata: metadata,
            });
            // Keep the previous active entry's identity until select records it.
            if destination.workspace_state.active >= index { destination.workspace_state.active += 1; }
            destination.select(index, window, cx);
            Ok(())
        });
        result.and_then(|result| result)?;
        self.remove_workspace_entry(index, window, cx);
        if activate { let _ = target.update(cx, |_, window, cx| activate_destination(window, cx)); }
        Ok(())
    }
    fn detach_workspace(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.workspace_state.entries.is_empty() || cx.global::<Shutdowns>().quitting {
            return;
        }
        self.remember_focus(window, cx);
        let entry = &self.workspace_state.entries[self.workspace_state.active];
        let workspace = entry.workspace.clone();
        let rollback = workspace.clone();
        let name = entry.name.clone();
        let description = entry.description.clone();
        let project_directory = entry.project_directory.clone();
        let color = entry.color;
        let icon = entry.icon.clone();
        let pinned = entry.pinned;
        let last_pane = entry.last_pane;
        let sidebar_width = self.sidebar_width;
        let bounds = Bounds::centered(None, size(px(1000.), px(680.)), cx);
        let result = cx.open_window(
            WindowOptions {
                            titlebar: workspace_titlebar(),
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            move |window, cx| {
                cx.new(|cx| {
                    workspace.update(cx, |workspace, cx| workspace.rebind_window(window, cx));
                    let closed = Self::observe_workspace(&workspace, window, cx);
                    let metadata = Self::observe_metadata(&workspace, window, cx);
                    let mut this = Self::empty(None, cx);
                    this.sidebar_width = sidebar_width;
                    this.next_number = 2;
                    this.workspace_state.entries.push(Entry {
                        workspace,
                        name,
                        description,
                        project_directory,
                        color,
                        icon,
                        pinned,
                        last_pane,
                        _closed: closed,
                        _metadata: metadata,
                    });
                    this.select(0, window, cx);
                    this
                })
            },
        );
        match result {
            Ok(_) => {
                self.remove_workspace_entry(self.workspace_state.active, window, cx);
                cx.activate(true);
            }
            Err(error) => {
                rollback.update(cx, |workspace, cx| workspace.rebind_window(window, cx));
                self.select(self.workspace_state.active, window, cx);
                self.notice = Some(format!("Move workspace: {error}"));
                cx.notify();
            }
        }
    }
    fn detach_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if cx.global::<Shutdowns>().quitting {
            return;
        }
        let workspace = self.workspace_state.entries[self.workspace_state.active].workspace.clone();
        let Some(index) = workspace.read(cx).focused_index(window, cx) else {
            return;
        };
        let source = workspace.read(cx).panes[index].clone();
        let Some((id, tab)) = source.read(cx).active_tab_transfer() else {
            return;
        };
        let terminal = tab.terminal.clone();
        let name = self.workspace_state.entries[self.workspace_state.active].name.clone();
        let bounds = Bounds::centered(None, size(px(1000.), px(680.)), cx);
        let result = cx.open_window(
            WindowOptions {
                            titlebar: workspace_titlebar(),
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            move |window, cx| cx.new(|cx| Self::from_tab(tab, name, window, cx)),
        );
        match result {
            Ok(_) => {
                source.update(cx, |pane, cx| pane.take_tab(id, cx));
                if source.read(cx).terminals.is_empty() {
                    workspace.update(cx, |workspace, cx| workspace.remove_pane(&source, cx));
                }
                cx.activate(true);
                cx.notify();
            }
            Err(error) => {
                terminal.update(cx, |terminal, cx| terminal.rebind_window(window, cx));
                source.read(cx).focus(window, cx);
                self.notice = Some(format!("Detach tab: {error}"));
                cx.notify();
            }
        }
    }
    fn from_tab(
        tab: pane_view::TabTransfer,
        name: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        tab.terminal
            .update(cx, |terminal, cx| terminal.rebind_window(window, cx));
        let mut this = Self::empty(None, cx);
        let workspace = cx.new(|cx| {
            let id = cx.entity_id();
            let pane = cx.new(|cx| PaneView::from_transfer(tab, id, cx));
            let mut workspace = Workspace {
                    event_lifetime: cx.global::<events::Journal>().workspace(cx.entity_id()),
                spawning_split: None,
                split_launch_canceled: None,
                pane_subscriptions: Default::default(),
                split_drag: None,
                panes: vec![pane.clone()],
                pane_state: rmux_core::pane_commands::State { layout: Some(PaneTree::Pane(pane.clone())), ..Default::default() },
                error: None,
                benchmark: false,
            };
            workspace.observe_pane(&pane, window, cx);
            pane.read(cx).focus(window, cx);
            workspace
        });
        let closed = Self::observe_workspace(&workspace, window, cx);
        let metadata = Self::observe_metadata(&workspace, window, cx);
        let last_pane = Some(workspace.read(cx).panes[0].entity_id());
        this.workspace_state.entries.push(Entry {
            workspace,
            name: name.clone(),
            description: String::new(),
            project_directory: None,
            color: None,
            icon: None,
            pinned: false,
            last_pane,
            _closed: closed,
            _metadata: metadata,
        });
        this.next_number = 2;
        window.set_window_title(&format!("{name} — rmux"));
        this
    }
    fn set_workspace_icon(&mut self, icon: Option<String>, cx: &mut Context<Self>) {
        if icon
            .as_deref()
            .is_some_and(|icon| !snapshot::valid_workspace_icon(icon))
        {
            return;
        }
        if let Some(entry) = self.workspace_state.entries.get_mut(self.workspace_state.active) {
            entry.icon = icon;
            cx.notify();
        }
    }
    fn set_workspace_color(&mut self, color: Option<u32>, cx: &mut Context<Self>) {
        if color.is_some_and(|color| color > 0xffffff) {
            return;
        }
        if let Some(entry) = self.workspace_state.entries.get_mut(self.workspace_state.active) {
            entry.color = color;
            cx.notify();
        }
    }
    fn toggle_workspace_pin(&mut self, cx: &mut Context<Self>) {
        if self.workspace_state.entries.is_empty() {
            return;
        }
        self.workspace_state.entries[self.workspace_state.active].pinned = !self.workspace_state.entries[self.workspace_state.active].pinned;
        self.scroll_to_workspace(self.workspace_state.active);
        cx.notify();
    }
    fn dismiss_notification(&mut self, id: EntityId, revision: u64, cx: &mut Context<Self>) {
        self.change_notification(id, revision, false, cx);
    }
    fn change_notification(&mut self, id: EntityId, revision: u64, mark_read: bool, cx: &mut Context<Self>) {
        // Resolve the effective selection, including the first-row fallback used
        // when a previously selected message has expired from the bounded history.
        let removed_selection = self.notification_navigation.as_ref().and_then(|state| {
            if mark_read && !state.unread_only { return None; }
            let targets = self.notification_targets(cx);
            let index = targets.iter().position(|row| Some((row.1, row.2)) == state.selected).unwrap_or(0);
            targets.get(index).filter(|row| (row.1, row.2) == (id, revision)).map(|_| index)
        });
        let target = self.workspace_state.entries.iter().find_map(|entry| {
            entry.workspace.read(cx).panes.iter().find_map(|pane| {
                pane.read(cx)
                    .terminals
                    .iter()
                    .find(|terminal| terminal.entity_id() == id)
                    .cloned()
            })
        });
        if let Some(target) = target {
            target.update(cx, |terminal, cx| {
                if mark_read { terminal.mark_notification_read(revision, cx); }
                else { terminal.dismiss_notification(revision, cx); }
            });
            if let Some(index) = removed_selection {
                let targets = self.notification_targets(cx);
                if let Some(state) = &mut self.notification_navigation {
                    let index = index.min(targets.len().saturating_sub(1));
                    state.selected = targets.get(index).map(|row| (row.1, row.2));
                    if state.expanded == Some((id, revision)) { state.expanded = None; }
                    state.scroll.scroll_to_item(index);
                    cx.notify();
                }
            }
        }
    }
    fn focus_terminal(
        &mut self,
        workspace: EntityId,
        terminal: EntityId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(index) = self
            .workspace_state.entries
            .iter()
            .position(|entry| entry.workspace.entity_id() == workspace)
        else {
            return;
        };
        let target = self.workspace_state.entries[index]
            .workspace
            .read(cx)
            .panes
            .iter()
            .find_map(|pane| {
                pane.read(cx)
                    .terminals
                    .iter()
                    .position(|view| view.entity_id() == terminal)
                    .map(|tab| (pane.clone(), tab))
            });
        let Some((pane, tab)) = target else {
            return;
        };
        self.record_terminal_focus(window, cx);
        self.remember_focus(window, cx);
        self.activate_workspace(index);
        self.rename = None;
        self.workspace_state.entries[index].last_pane = Some(pane.entity_id());
        self.scroll_to_workspace(index);
        window.set_window_title(&format!("{} — rmux", self.workspace_state.entries[index].name));
        self.workspace_state.entries[index].workspace.update(cx, |workspace, cx| {
            workspace.pane_state.focus_changed(pane.entity_id());
            cx.notify();
        });
        pane.update(cx, |pane, cx| pane.select(tab, window, cx));
        self.record_terminal_visit(terminal);
        cx.notify();
    }
    fn mark_all_read(&mut self, cx: &mut Context<Self>) {
        // Cached UI state only: no parser locks, input, focus changes or pane reveals.
        let terminals: Vec<_> = self.workspace_state.entries.iter().flat_map(|entry| {
            entry.workspace.read(cx).panes.iter().flat_map(|pane| {
                pane.read(cx).terminals.iter().filter(|terminal| terminal.read(cx).attention.any()).cloned()
            })
        }).collect();
        for terminal in terminals {
            terminal.update(cx, |view, cx| view.acknowledge_attention(cx));
        }
        if let Some(state) = &mut self.notification_navigation {
            if state.unread_only {
                state.selected = None;
                state.expanded = None;
                state.scroll.scroll_to_item(0);
            }
        }
        cx.notify();
    }

    fn dismiss_all_notifications(&mut self, cx: &mut Context<Self>) {
        let mut pending = Vec::new();
        for entry in &self.workspace_state.entries {
            for pane in &entry.workspace.read(cx).panes {
                for terminal in &pane.read(cx).terminals {
                    let view = terminal.read(cx);
                    if view.notifications.latest().is_some() { pending.push(terminal.clone()); }
                }
            }
        }
        for terminal in pending {
            terminal.update(cx, |terminal, cx| {
                if terminal.notifications.iter().next().is_some() {
                    events::publish("notification.cleared", cx.entity_id(), None, cx);
                }
                terminal.notifications.clear();
                terminal.attention.dismiss_notification();
                cx.emit(TabMetadataChanged);
                cx.notify();
            });
        }
        if let Some(state) = &mut self.notification_navigation {
            state.selected = None;
            state.expanded = None;
            state.scroll.scroll_to_item(0);
        }
        cx.notify();
    }

    fn notification_targets(&self, cx: &App) -> Vec<(EntityId, EntityId, u64)> {
        let mut targets = Vec::new();
        let unread_only = self.notification_navigation.as_ref().is_some_and(|state| state.unread_only);
        for index in self.workspace_order(true) {
            let entry = &self.workspace_state.entries[index];
            for pane in &entry.workspace.read(cx).panes {
                for terminal in &pane.read(cx).terminals {
                    let view = terminal.read(cx);
                    for record in view.notifications.iter().rev().filter(|record| !unread_only || record.unread) {
                        targets.push((entry.workspace.entity_id(), terminal.entity_id(), record.message.revision, record.arrival));
                    }
                }
            }
        }
        targets.sort_unstable_by_key(|row| std::cmp::Reverse(row.3));
        targets.into_iter().map(|(workspace, terminal, revision, _)| (workspace, terminal, revision)).collect()
    }
    fn toggle_notifications(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(state) = self.notification_navigation.take() {
            if state.focus.is_focused(window) {
                if let Some(origin) = state.origin { origin.focus(window); } else { self.select(self.workspace_state.active, window, cx); }
            }
        } else {
            if self.settings_state.is_some() { self.toggle_settings(window, cx); }
            let state = NotificationNavigation { expanded: None, unread_only: false, focus: cx.focus_handle(), origin: window.focused(cx),
                selected: self.notification_targets(cx).first().map(|row| (row.1, row.2)), scroll: ScrollHandle::new() };
            state.focus.focus(window);
            self.notification_navigation = Some(state);
        }
        cx.notify();
    }
    fn set_notification_filter(&mut self, unread_only: bool, cx: &mut Context<Self>) {
        let Some(state) = &mut self.notification_navigation else { return; };
        if state.unread_only == unread_only { return; }
        state.unread_only = unread_only;
        let targets = self.notification_targets(cx);
        let state = self.notification_navigation.as_mut().expect("open notification panel");
        let index = targets.iter().position(|row| Some((row.1, row.2)) == state.selected).unwrap_or(0);
        state.selected = targets.get(index).map(|row| (row.1, row.2));
        state.scroll.scroll_to_item(index);
        cx.notify();
    }
    fn notification_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if event.keystroke.modifiers.platform || event.keystroke.modifiers.control || event.keystroke.modifiers.alt
            || (event.keystroke.modifiers.shift && event.keystroke.key != "tab") { return; }
        if event.keystroke.key == "u" {
            let unread_only = !self.notification_navigation.as_ref().is_some_and(|state| state.unread_only);
            self.set_notification_filter(unread_only, cx);
            cx.stop_propagation();
            return;
        }
        let targets = self.notification_targets(cx);
        let Some(state) = &mut self.notification_navigation else { return; };
        let key = event.keystroke.key.as_str();
        let selected = targets.iter().position(|row| Some((row.1, row.2)) == state.selected).unwrap_or(0);
        let direction = if key == "tab" { if event.keystroke.modifiers.shift { "up" } else { "down" } } else { key };
        let destination = if matches!(direction, "pageup" | "pagedown") {
            command_palette::paged_result(selected, targets.len(), direction == "pageup",
                f32::from(state.scroll.bounds().size.height), |index| {
                    // Include the inter-card gap; expanded cards can exceed a viewport.
                    state.scroll.bounds_for_item(index).map(|bounds| {
                        state.scroll.bounds_for_item(index + 1)
                            .map(|next| f32::from(next.top() - bounds.top()))
                            .unwrap_or_else(|| f32::from(bounds.size.height) + 8.)
                    }).unwrap_or(1.).max(1.)
                })
        } else if !targets.is_empty() && matches!(direction, "home" | "end") {
            Some(if direction == "home" { 0 } else { targets.len() - 1 })
        } else {
            command_palette::navigated_result(selected, targets.len(), direction)
        };
        if let Some(index) = destination {
            state.selected = Some((targets[index].1, targets[index].2));
            state.scroll.scroll_to_item(index);
            cx.notify();
        } else if key == "escape" {
            self.toggle_notifications(window, cx);
        } else if let Some(&(workspace, terminal, revision)) = targets.get(selected) {
            if matches!(key, "enter" | "space") {
                self.notification_navigation = None;
                self.focus_terminal(workspace, terminal, window, cx);
            } else if matches!(key, "backspace" | "delete") {
                self.dismiss_notification(terminal, revision, cx);
            } else if key == "r" {
                self.change_notification(terminal, revision, true, cx);
            } else if key == "e" {
                state.expanded = if state.expanded == Some((terminal, revision)) { None } else { Some((terminal, revision)) };
                cx.notify();
            }
        }
        cx.stop_propagation();
    }

    fn copy_selected_notification(&self, cx: &mut Context<Self>) {
        let Some(state) = &self.notification_navigation else { return; };
        let targets = self.notification_targets(cx);
        let Some((_, terminal, revision)) = targets.iter()
            .find(|row| Some((row.1, row.2)) == state.selected).or_else(|| targets.first()) else { return; };
        let message = self.workspace_state.entries.iter().flat_map(|entry| &entry.workspace.read(cx).panes)
            .flat_map(|pane| &pane.read(cx).terminals)
            .find(|view| view.entity_id() == *terminal)
            .and_then(|view| view.read(cx).notifications.iter().find(|record| record.message.revision == *revision))
            .map(|record| record.message.clone());
        if let Some(message) = message {
            clipboard::write(ClipboardItem::new_string(notifications::copy_text(&message)), cx);
        }
    }

    fn notification_panel(&self, overlay: bool, cx: &mut Context<Self>) -> AnyElement {
        let navigation = self.notification_navigation.as_ref().expect("open notification panel").clone();
        let has_unread = self.workspace_state.entries.iter().any(|entry| entry.workspace.read(cx).needs_attention(cx));
        let mut messages = Vec::new();
        let mut total_count = 0;
        let mut unread_count = 0;
        for index in self.workspace_order(true) {
            let entry = &self.workspace_state.entries[index];
            for pane in &entry.workspace.read(cx).panes {
                for (tab, terminal) in pane.read(cx).terminals.iter().enumerate() {
                    let view = terminal.read(cx);
                    let mut source_label: Option<SharedString> = None;
                    for record in view.notifications.iter().rev() {
                        total_count += 1;
                        unread_count += usize::from(record.unread);
                        if navigation.unread_only && !record.unread { continue; }
                        let label = source_label.get_or_insert_with(|| {
                            format!("{} · {}", entry.name, pane.read(cx).tab_title(tab, cx)).into()
                        });
                        let message = &record.message;
                        messages.push((
                            entry.workspace.entity_id(),
                            terminal.entity_id(),
                            label.clone(),
                            record.unread,
                            message.clone(),
                            record.arrival,
                        ));
                    }
                }
            }
        }
        messages.sort_unstable_by_key(|row| std::cmp::Reverse(row.5));
        let selected = messages.iter().find(|row| Some((row.1, row.4.revision)) == navigation.selected).or_else(|| messages.first()).map(|row| (row.1, row.4.revision));
        chrome_style::inspector(overlay).track_focus(&navigation.focus).key_context("Notifications")
            .on_mouse_down(MouseButton::Left, { let focus = navigation.focus.clone(); move |_, window, _| focus.focus(window) })
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| this.notification_key(event, window, cx)))
            .on_action(cx.listener(|this, _: &CopySelection, _, cx| this.copy_selected_notification(cx)))
            .child(
                div()
                    .h(px(44.))
                    .px_3()
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(div().flex_1().min_w_0().truncate().text_size(px(13.)).font_weight(FontWeight::SEMIBOLD).child("Notifications"))
                    .when(total_count > 0, |header| header.child(
                        div().id("dismiss-all-notifications").px_2().py_1().rounded_sm().text_xs().cursor_pointer()
                            .hover(|style| style.bg(rgb(0x353535)))
                            .tooltip(|_, cx| cx.new(|_| pane_view::ControlHint("Dismiss all notifications in this window".into())).into())
                            .on_click(cx.listener(|this, event: &ClickEvent, _, cx| {
                                if event.standard_click() { this.dismiss_all_notifications(cx); }
                            })).child("Clear all")
                    ))
                    .child(
                        crate::pane_presentation::action_control("close-notifications", "Close Notifications", Box::new(ToggleNotifications), None, cfg!(target_os = "macos"))
                            .on_click(cx.listener(|this, event: &ClickEvent, window, cx| {
                                if event.standard_click() { this.toggle_notifications(window, cx); }
                            }))
                            .child(icons::icon("close")),
                    ),
            )
            .child(div().flex().gap_1().px_3().py_2().flex_shrink_0()
                .children([(false, format!("All {total_count}")), (true, format!("Unread {unread_count}"))].into_iter().map(|(unread_only, label)| {
                    div().id(if unread_only { "notification-filter-unread" } else { "notification-filter-all" })
                        .px_2().py_1().rounded_sm().text_xs().cursor_pointer()
                        .bg(if unread_only == navigation.unread_only { rgb(crate::chrome_style::SELECTED) } else { rgb(0x242424) })
                        .text_color(if unread_only == navigation.unread_only { rgb(0xffffff) } else { rgb(0xa0a0a0) })
                        .hover(|style| style.bg(rgb(0x353535)))
                        .on_click(cx.listener(move |this, event: &ClickEvent, _, cx| {
                            if event.standard_click() { this.set_notification_filter(unread_only, cx); }
                        })).child(label)
                })))
            .when(has_unread, |panel| panel.child(
                        div().id("mark-all-read").flex_shrink_0().mx_3().mb_2().px_2().py_1().rounded_sm().text_xs()
                            .text_color(rgb(crate::chrome_style::ACCENT)).cursor_pointer().hover(|style| style.bg(rgb(0x353535)))
                            .tooltip(|_, cx| cx.new(|_| pane_view::ControlHint("Acknowledge all alerts in this window; keep messages".into())).into())
                            .on_click(cx.listener(|this, event: &ClickEvent, _, cx| {
                                if event.standard_click() { this.mark_all_read(cx); }
                            })).child("Mark all read")
                    ))

            .child(
                div()
                    .id("notification-list")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .track_scroll(&navigation.scroll)
                    .p_2()
                    .when(messages.is_empty(), |list| {
                        list.child(div().p_3().text_sm()
                            .child(if navigation.unread_only { "No unread notifications" } else { "No notifications yet" })
                            .child(div().mt_2().text_xs().text_color(rgb(0xa0a0a0)).child(
                                if navigation.unread_only && total_count > 0 { "Choose All to see earlier messages." }
                                else { "Messages from your terminals appear here." }
                            )))
                    })
                    .children(messages.into_iter().map(
                        |(workspace, terminal, source_label, unread, message, _)| {
                            let revision = message.revision;
                            let copy_message = message.clone();
                            let expanded = navigation.expanded == Some((terminal, revision));
                            div()
                                .id(SharedString::from(format!("notification-{terminal}-{revision}")))
                                .p_3()
                                .mb_2()
                                .rounded_md()
                                .border_1()
                                .border_color(if selected == Some((terminal, revision)) { rgb(crate::chrome_style::ACCENT) } else { rgb(0x383838) })
                                .text_sm()
                                .cursor_pointer()
                                .bg(if selected == Some((terminal, revision)) { rgb(crate::chrome_style::SELECTED) } else { rgb(0x292929) })
                                .hover(|style| style.bg(rgb(crate::chrome_style::SELECTED_HOVER)))
                                .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
                                    if let Some(state) = &mut this.notification_navigation {
                                        state.selected = Some((terminal, revision));
                                        cx.notify();
                                    }
                                }))
                                .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
                                    if !event.standard_click() { return; }
                                    this.notification_navigation = None;
                                    this.focus_terminal(workspace, terminal, window, cx)
                                }))
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_2()
                                        .when(unread, |row| {
                                            row.child(
                                                div()
                                                    .w(px(6.))
                                                    .h(px(6.))
                                                    .flex_shrink_0()
                                                    .rounded_full()
                                                    .bg(rgb(0xf0b85b)),
                                            )
                                        })
                                        .child(
                                            div()
                                                .min_w_0()
                                                .truncate()
                                                .text_xs()
                                                .text_color(rgb(0xa0a0a0))
                                                .child(source_label),
                                        ),
                                )
                                .when(!message.title.is_empty(), |card| {
                                    card.child(
                                        div()
                                            .mt_1()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .when(!expanded, |title| title.line_clamp(2))
                                            .child(message.title.clone()),
                                    )
                                })
                                .child(div().mt_1().when(!expanded, |body| body.line_clamp(4)).child(message.body.clone()))
                                .child(
                                    div().mt_2().flex().flex_wrap().justify_end().gap_2()
                                        .when(unread, |controls| controls.child(
                                            div().id(SharedString::from(format!("read-notification-{terminal}-{revision}")))
                                                .px_2().py_1().rounded_sm().text_xs()
                                                .hover(|style| style.bg(rgb(crate::chrome_style::SELECTED_HOVER)))
                                                .on_click(cx.listener(move |this, event: &ClickEvent, _, cx| {
                                                    cx.stop_propagation();
                                                    if event.standard_click() { this.change_notification(terminal, revision, true, cx); }
                                                })).child("Mark read")
                                        ))
                                        .child(div().id(SharedString::from(format!("expand-notification-{terminal}-{revision}")))
                                            .px_2().py_1().rounded_sm().text_xs()
                                            .hover(|style| style.bg(rgb(crate::chrome_style::SELECTED_HOVER)))
                                            .on_click(cx.listener(move |this, event: &ClickEvent, _, cx| {
                                                cx.stop_propagation();
                                                if !event.standard_click() { return; }
                                                if let Some(state) = &mut this.notification_navigation {
                                                    state.expanded = if state.expanded == Some((terminal, revision)) { None } else { Some((terminal, revision)) };
                                                    cx.notify();
                                                }
                                            })).child(if expanded { "Less" } else { "More" }))
                                        .child(div().id(SharedString::from(format!("copy-notification-{terminal}-{revision}")))
                                            .px_2().py_1().rounded_sm().text_xs()
                                            .hover(|style| style.bg(rgb(crate::chrome_style::SELECTED_HOVER)))
                                            .on_click(move |event, _, cx| {
                                                cx.stop_propagation();
                                                if event.standard_click() {
                                                    clipboard::write(ClipboardItem::new_string(notifications::copy_text(&copy_message)), cx);
                                                }
                                            }).child("Copy"))
                                        .child(
                                        div()
                                            .id(SharedString::from(format!("dismiss-notification-{terminal}-{revision}")))
                                            .px_2()
                                            .py_1()
                                            .rounded_sm()
                                            .text_xs()
                                            .hover(|style| style.bg(rgb(crate::chrome_style::SELECTED_HOVER)))
                                            .on_click(cx.listener(move |this, event: &ClickEvent, _, cx| {
                                                cx.stop_propagation();
                                                if event.standard_click() { this.dismiss_notification(terminal, revision, cx); }
                                            }))
                                            .child("Dismiss"),
                                    ),
                                )
                        },
                    )),
            )
            .child(div().flex_shrink_0().px_3().py_2().border_t_1().border_color(rgb(0x383838))
                .text_xs().text_color(rgb(0xa0a0a0))
                .child("↑↓ / Tab select · Home/End first/last · Enter open")
                .child(div().mt_1().child("U filter · R mark read · E expand · Delete dismiss · Esc close")))
            .into_any_element()
    }
    fn activate_workspace(&mut self, index: usize) {
        self.workspace_state.select(index, |entry| entry.workspace.entity_id(),
        );
    }

    fn last_workspace(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(index) = self.workspace_state.navigation_target(rmux_core::workspace_lifecycle::Navigation::Previous, |entry| (entry.workspace.entity_id(), entry.pinned)) {
            self.select(index, window, cx);
        }
    }
    fn request_close_workspace(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(entry) = self.workspace_state.entries.get(index) else {
            return;
        };
        let active_work = || {
            entry.workspace.read(cx).panes.iter().any(|pane| {
                pane.read(cx)
                    .terminals
                    .iter()
                    .any(|terminal| terminal.read(cx).has_active_work())
            })
        };
        let has_terminals = entry
            .workspace
            .read(cx)
            .panes
            .iter()
            .any(|pane| !pane.read(cx).terminals.is_empty());
        if !cx
            .global::<ClosePolicy>()
            .confirms(has_terminals, active_work)
        {
            self.close_workspace(index, window, cx);
            return;
        }
        if self.close_prompt.is_some() {
            return;
        }
        let id = entry.workspace.entity_id();
        let Some(permit) = cx.global::<PromptGate>().acquire() else {
            return;
        };
        self.close_prompt = Some(id);
        let answer = window.prompt(PromptLevel::Warning, "Close workspace?",
            Some("Closing this workspace terminates all of its owned terminal processes and discards their output history."),
            &["Cancel", "Close Workspace"], cx);
        cx.spawn_in(window, async move |entity, cx| {
            let confirmed = matches!(answer.await, Ok(1));
            drop(permit);
            let _ = cx.update(|window, cx| {
                entity.update(cx, |this, cx| {
                    if this.close_prompt != Some(id) {
                        return;
                    }
                    this.close_prompt = None;
                    if confirmed {
                        if let Some(index) = this
                            .workspace_state.entries
                            .iter()
                            .position(|entry| entry.workspace.entity_id() == id)
                        {
                            this.close_workspace(index, window, cx);
                        }
                    }
                })
            });
        })
        .detach();
    }
    fn close_workspace(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(entry) = self.workspace_state.entries.get(index) else { return; };
        entry.workspace.read(cx).request_process_close(cx);
        self.remove_workspace_entry(index, window, cx);
    }

    /// Remove window ownership only: move/detach keeps the same live workspace.
    fn remove_workspace_entry(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(closed) = self.workspace_state.close(index, |entry| (entry.workspace.entity_id(), entry.pinned),
        ) else { return; };
        self.rename = None;
        if self.workspace_state.entries.is_empty() {
            window.remove_window();
            return;
        }
        if closed.was_active {
            self.select(self.workspace_state.active, window, cx);
        } else {
            cx.notify();
        }
    }
    fn entry_directory(entry: &Entry, window: &Window, cx: &App) -> Option<(Option<String>, Option<String>)> {
        let workspace = entry.workspace.read(cx);
        workspace
            .focused_index(window, cx)
            .and_then(|index| workspace.panes.get(index))
            .or_else(|| {
                workspace
                    .panes
                    .iter()
                    .find(|pane| Some(pane.entity_id()) == entry.last_pane)
            })
            .or_else(|| workspace.panes.first())?
            .read(cx)
            .context_label(cx)
    }
    fn close_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some((_, focus)) = self.palette.take() {
            if let Some(focus) = focus {
                focus.focus(window);
            } else {
                self.select(self.workspace_state.active, window, cx);
            }
            cx.notify();
        }
    }
    // Build searchable context only when opening the palette, not on each frame.
    fn workspace_palette_context(&self, entry: &Entry) -> Option<String> {
        let mut context = Vec::new();
        if let Some(group) = self.workspace_state.groups.containing(entry.workspace.entity_id()) {
            context.push(format!("Group: {}", group.name));
        }
        if !entry.description.is_empty() { context.push(entry.description.clone()); }
        if let Some(path) = &entry.project_directory {
            context.push(format!("Project · {}", path.display()));
        }
        (!context.is_empty()).then(|| context.join(" · "))
    }

    fn show_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.palette.is_some() {
            self.close_palette(window, cx);
            return;
        }
        let focus = window.focused(cx);
        let workspaces = self.workspace_order(true)
            .into_iter()
            .map(|index| { let entry = &self.workspace_state.entries[index]; (entry.workspace.entity_id(), entry.name.clone(), self.workspace_palette_context(entry)) })
            .collect();
        let active_workspace = self.workspace_state.entries[self.workspace_state.active].workspace.entity_id();
        let workspace = self.workspace_state.entries[self.workspace_state.active].workspace.read(cx);
        let (tab, paste) = workspace.focused_index(window, cx)
            .map(|index| workspace.panes[index].read(cx).pending_palette_operations(cx))
            .unwrap_or_default();
        let pending = command_palette::PaletteContext {
            workspace: self.workspace_launch_cancel.is_some(),
            split: workspace.spawning_split.is_some(), tab, paste,
            file_loading: self.file_browser_job.is_some(),
            restore: self.restore_job.is_some(),
            active_group: self.workspace_state.groups.containing(active_workspace).map(|group| group.id),
            selected_workspaces: self.workspace_state.sidebar_selection.len(),
        };
        let mut terminals = Vec::new();
        for entry in &self.workspace_state.entries {
            for (pane_index, pane) in entry.workspace.read(cx).panes.iter().enumerate() {
                let pane = pane.read(cx);
                for (tab, terminal) in pane.terminals.iter().enumerate() {
                    terminals.push((
                        None,
                        entry.workspace.entity_id(),
                        terminal.entity_id(),
                        format!(
                            "{} / Pane {} / Tab {} — {}",
                            entry.name,
                            pane_index + 1,
                            tab + 1,
                            pane.terminal_search_title(tab, cx)
                        ),
                        terminal.read(cx).snapshot_directory.as_ref().map(|path| path.to_string_lossy().into_owned()),
                    ));
                }
            }
        }
        let current_window = window.window_handle();
        let mut external_workspaces = Vec::new();
        let windows = cx.windows().into_iter().enumerate().filter_map(|(index, handle)| {
            if handle == current_window { return None; }
            let root = handle.downcast::<Self>()?.read(cx).ok()?;
            if root.benchmark { return None; }
            for entry in &root.workspace_state.entries {
                external_workspaces.push((handle, entry.workspace.entity_id(), format!("Window {} / Workspace: {}", index + 1, entry.name), root.workspace_palette_context(entry)));
                for (pane_index, pane) in entry.workspace.read(cx).panes.iter().enumerate() {
                    let pane = pane.read(cx);
                    for (tab, terminal) in pane.terminals.iter().enumerate() {
                        terminals.push((Some(handle), entry.workspace.entity_id(), terminal.entity_id(),
                            format!("Window {} / {} / Pane {} / Tab {} — {}", index + 1, entry.name, pane_index + 1, tab + 1, pane.terminal_search_title(tab, cx)),
                            terminal.read(cx).snapshot_directory.as_ref().map(|path| path.to_string_lossy().into_owned())));
                    }
                }
            }
            let name = &root.workspace_state.entries.get(root.workspace_state.active)?.name;
            Some((handle, format!("Window {} — {name}", index + 1)))
        }).collect();
        let palette = cx.new(|cx| {
            command_palette::CommandPalette::new(
                workspaces,
                terminals,
                windows,
                external_workspaces,
                self.workspace_state.groups.entries().iter().filter(|group| !group.members.contains(&active_workspace))
                    .map(|group| (group.id, group.name.clone())).collect(),
                active_workspace,
                pending,
                window,
                cx,
            )
        });
        self.install_palette(palette, focus, window, cx);
    }
    fn show_keyboard_shortcuts(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.palette.is_some() { self.close_palette(window, cx); }
        let origin = window.focused(cx);
        self.show_palette(window, cx);
        if let Some((palette, _)) = &self.palette {
            palette.update(cx, |palette, cx| palette.show_shortcuts(origin.as_ref(), window, cx));
        }
    }
    fn show_workspace_merge_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.automation_available(cx).is_err() { return; }
        let Some(source) = self.workspace_state.entries.get(self.workspace_state.active).map(|entry| entry.workspace.entity_id()) else { return; };
        let focus = window.focused(cx);
        let current = window.window_handle();
        let mut entries: Vec<_> = self.workspace_order(true).into_iter().filter_map(|index| {
            let entry = &self.workspace_state.entries[index];
            (entry.workspace.entity_id() != source).then(|| (current, entry.workspace.entity_id(), format!("This window / {}", entry.name)))
        }).collect();
        for (number, handle) in cx.windows().into_iter().enumerate() {
            if handle == current { continue; }
            let Some(handle) = handle.downcast::<Self>() else { continue; };
            let Ok(root) = handle.read(cx) else { continue; };
            if root.benchmark { continue; }
            for index in root.workspace_order(true) {
                let entry = &root.workspace_state.entries[index];
                entries.push((handle.into(), entry.workspace.entity_id(), format!("Window {} / {}", number + 1, entry.name)));
            }
        }
        let palette = cx.new(|cx| command_palette::CommandPalette::workspace_merge(entries, source, window, cx));
        self.install_palette(palette, focus, window, cx);
    }
    pub(super) fn merge_workspace_to_window(&mut self, source_id: EntityId, destination_id: EntityId, target: AnyWindowHandle, activate: bool, window: &mut Window, cx: &mut Context<Self>) -> Result<()> {
        if target == window.window_handle() {
            self.merge_workspaces(source_id, destination_id, window, cx)?;
            if activate { activate_destination(window, cx); }
            return Ok(());
        }
        self.automation_available(cx).map_err(|failure| anyhow::anyhow!(failure.message))?;
        let source_index = self.workspace_state.entries.iter().position(|entry| entry.workspace.entity_id() == source_id)
            .ok_or_else(|| anyhow::anyhow!("Source workspace closed"))?;
        let source = self.workspace_state.entries[source_index].workspace.clone();
        let target = target.downcast::<Self>().ok_or_else(|| anyhow::anyhow!("Invalid destination window"))?;
        target.update(cx, |destination, window, cx| -> Result<()> {
            destination.automation_available(cx).map_err(|failure| anyhow::anyhow!(failure.message))?;
            let index = destination.workspace_state.entries.iter().position(|entry| entry.workspace.entity_id() == destination_id)
                .ok_or_else(|| anyhow::anyhow!("Destination workspace closed"))?;
            let count_tabs = |workspace: &Entity<Workspace>| workspace.read(cx).panes.iter().map(|pane| pane.read(cx).terminals.len()).sum::<usize>();
            let tabs = count_tabs(&source) + destination.workspace_state.entries.iter().map(|entry| count_tabs(&entry.workspace)).sum::<usize>();
            anyhow::ensure!(tabs <= 256, "Destination window would exceed 256 terminals");
            let workspace = destination.workspace_state.entries[index].workspace.clone();
            workspace.update(cx, |workspace, cx| workspace.absorb_workspace(&source, true, window, cx))?;
            destination.select(index, window, cx);
            if activate { activate_destination(window, cx); }
            Ok(())
        })??;
        // The destination now owns every pane; removing this entry cannot stop
        // their PTYs. Existing cleanup handles source groups and the last window.
        self.close_workspace(source_index, window, cx);
        Ok(())
    }
    pub(super) fn merge_workspaces(&mut self, source: EntityId, destination: EntityId, window: &mut Window, cx: &mut Context<Self>) -> Result<()> {
        self.automation_available(cx).map_err(|failure| anyhow::anyhow!(failure.message))?;
        anyhow::ensure!(source != destination, "Choose another workspace");
        let source_index = self.workspace_state.entries.iter().position(|entry| entry.workspace.entity_id() == source).ok_or_else(|| anyhow::anyhow!("Source workspace closed"))?;
        let destination_index = self.workspace_state.entries.iter().position(|entry| entry.workspace.entity_id() == destination).ok_or_else(|| anyhow::anyhow!("Destination workspace closed"))?;
        let source = self.workspace_state.entries[source_index].workspace.clone();
        let destination = self.workspace_state.entries[destination_index].workspace.clone();
        destination.update(cx, |workspace, cx| workspace.absorb_workspace(&source, false, window, cx))?;
        self.select(destination_index, window, cx);
        self.close_workspace(source_index, window, cx);
        Ok(())
    }
    fn show_workspace_window_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.automation_available(cx).is_err() { return; }
        let Some(source) = self.workspace_state.entries.get(self.workspace_state.active).map(|entry| entry.workspace.entity_id()) else { return; };
        let current = window.window_handle();
        let focus = window.focused(cx);
        let entries = cx.windows().into_iter().enumerate().filter_map(|(index, handle)| {
            if handle == current { return None; }
            let root = handle.downcast::<Self>()?.read(cx).ok()?;
            if root.benchmark { return None; }
            let active = root.workspace_state.entries.get(root.workspace_state.active)?;
            let count = root.workspace_state.entries.len();
            let subtitle = if root.automation_available(cx).is_err() {
                format!("{count} workspaces · Destination is busy")
            } else {
                format!("{count} {} · Move here and activate window", if count == 1 { "workspace" } else { "workspaces" })
            };
            Some((handle, format!("Window {} — {}", index + 1, active.name), subtitle))
        }).collect();
        let palette = cx.new(|cx| command_palette::CommandPalette::workspace_windows(entries, source, window, cx));
        self.install_palette(palette, focus, window, cx);
    }
    fn show_buffer_picker(&mut self, mode: command_palette::BufferMode, window: &mut Window, cx: &mut Context<Self>) {
        if self.automation_available(cx).is_err() { return; }
        let surface = self.workspace_state.entries.get(self.workspace_state.active).and_then(|entry| {
            let workspace = entry.workspace.read(cx);
            let index = workspace.focused_index(window, cx)?;
            workspace.panes[index].read(cx).active_terminal_id()
        });
        if surface.is_none() && mode == command_palette::BufferMode::Paste { self.notice = Some("Focus a terminal before choosing a buffer".into()); cx.notify(); return; }
        let surface = surface.map(|id| id.to_string()).unwrap_or_default();
        let focus = window.focused(cx);
        let store = cx.global::<copy_buffers::Store>();
        let (entries, revision) = (store.metadata(), store.revision());
        let palette = cx.new(|cx| command_palette::CommandPalette::buffers(entries, revision, surface, mode, window, cx));
        self.install_palette(palette, focus, window, cx);
    }
    fn cancel_file_loading(&mut self, cx: &mut Context<Self>) {
        if self.file_browser_job.is_none() { return; }
        // Signal the worker before dropping the UI continuation. Its permit
        // remains owned by the worker until an in-flight OS read returns.
        self.file_browser_cancel = None;
        self.file_browser_job = None;
        self.preview_restore = None;
        self.folder_restore = None;
        if self.notice.as_deref().is_some_and(file_browser::is_loading_notice) { self.notice = None; }
        cx.notify();
    }
    fn browse_directory(&mut self, path: std::path::PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        self.browse_directory_at(path, None, window, cx);
    }
    fn browse_history(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        if let Some((index, path)) = self.file_history.adjacent(forward) {
            self.browse_directory_at(path, Some(index), window, cx);
        }
    }
    fn browse_directory_at(&mut self, path: std::path::PathBuf, navigation: Option<usize>, window: &mut Window, cx: &mut Context<Self>) {
        let restore = self.folder_restore.take().filter(|state| navigation.is_none() && state.matches(&path));
        if self.file_browser_job.is_some() || self.automation_available(cx).is_err() { return; }
        let Some(focus) = window.focused(cx) else { return; };
        let permit = match file_browser::Permit::acquire() {
            Ok(permit) => permit,
            Err(error) => { self.notice = Some(error.to_string()); cx.notify(); return; }
        };
        self.notice = Some("Loading local directory…".into());
        let executor = cx.background_executor().clone();
        let cancellation = file_browser::Cancellation::default();
        let canceled = cancellation.0.clone();
        self.file_browser_cancel = Some(cancellation);
        self.file_browser_job = Some(cx.spawn_in(window, async move |entity, cx| {
            let target = path.clone();
            let result = executor.spawn(async move { let _permit = permit; file_browser::list_cancellable(path, &canceled) }).await;
            let _ = cx.update(|window, cx| entity.update(cx, |this, cx| {
                this.file_browser_job = None;
                this.file_browser_cancel = None;
                if this.notice.as_deref() == Some("Loading local directory…") { this.notice = None; }
                if focus.is_focused(window) && this.automation_available(cx).is_ok() {
                    match result {
                        Ok(listing) => {
                            this.file_history.commit(listing.path.clone(), navigation);
                            let back = this.file_history.adjacent(false).is_some();
                            let forward = this.file_history.adjacent(true).is_some();
                            let palette = cx.new(|cx| {
                                let mut palette = command_palette::CommandPalette::files(listing, back, forward, window, cx);
                                if let Some(state) = restore { palette.restore_folder(state, cx); }
                                palette
                            });
                            this.install_palette(palette, Some(focus), window, cx);
                        }
                        Err(error) => {
                            let palette = cx.new(|cx| command_palette::CommandPalette::file_error(target, format!("{error:#}"), true, window, cx));
                            this.install_palette(palette, Some(focus), window, cx);
                        }
                    }
                }
                cx.notify();
            }));
        }));
        cx.notify();
    }
    fn show_ssh_hosts(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.file_browser_job.is_some() || self.automation_available(cx).is_err() { return; }
        let Some(focus) = window.focused(cx) else { return; };
        let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(std::path::PathBuf::from);
        let path = home.filter(|path| path.is_absolute()).map(|home| home.join(".ssh/config"));
        let permit = match file_browser::Permit::acquire() {
            Ok(permit) => permit,
            Err(error) => { self.notice = Some(error.to_string()); cx.notify(); return; }
        };
        self.notice = Some("Loading SSH hosts…".into());
        let executor = cx.background_executor().clone();
        let cancellation = file_browser::Cancellation::default();
        let canceled = cancellation.0.clone();
        self.file_browser_cancel = Some(cancellation);
        self.file_browser_job = Some(cx.spawn_in(window, async move |entity, cx| {
            let result = executor.spawn(async move { let _permit = permit;
                match path {
                    Some(path) => ssh::read_aliases(&path, &canceled),
                    None => Err(anyhow::anyhow!("Home directory unavailable for SSH configuration")),
                } }).await;
            let _ = cx.update(|window, cx| entity.update(cx, |this, cx| {
                this.file_browser_job = None;
                this.file_browser_cancel = None;
                if this.notice.as_deref() == Some("Loading SSH hosts…") { this.notice = None; }
                if focus.is_focused(window) && this.automation_available(cx).is_ok() {
                    let mut targets = Vec::new();
                    let mut inspected = 0;
                    let mut truncated = false;
                    'workspaces: for entry in &this.workspace_state.entries {
                        for pane in &entry.workspace.read(cx).panes {
                            for terminal in &pane.read(cx).terminals {
                                if inspected == 256 { truncated = true; break 'workspaces; }
                                inspected += 1;
                                let terminal = terminal.read(cx);
                                let target = terminal.session.remote_destination().map(|destination| ssh::HostTarget {
                                    destination: destination.to_owned(), source: ssh::TargetSource::Session,
                                }).or_else(|| terminal.saved_ssh_destination.as_ref().map(|destination| ssh::HostTarget {
                                    destination: destination.clone(), source: ssh::TargetSource::Saved,
                                }));
                                if let Some(target) = target { targets.push(target); }
                            }
                        }
                    }
                    let palette = cx.new(|cx| command_palette::CommandPalette::ssh_hosts(result, targets, truncated, window, cx));
                    this.install_palette(palette, Some(focus), window, cx);
                }
                cx.notify();
            }));
        }));
        cx.notify();
    }
    fn focus_search_result(&mut self, workspace: EntityId, terminal: EntityId, query: &str, window: &mut Window, cx: &mut Context<Self>) -> Result<()> {
        self.automation_available(cx).map_err(|error| anyhow::anyhow!(error.message))?;
        let target = self.workspace_state.entries.iter().find(|entry| entry.workspace.entity_id() == workspace)
            .and_then(|entry| entry.workspace.read(cx).panes.iter().find_map(|pane|
                pane.read(cx).terminals.iter().find(|entity| entity.entity_id() == terminal).cloned()))
            .ok_or_else(|| anyhow::anyhow!("Terminal moved or closed; search again"))?;
        // Thread creation can fail. Prepare before changing selection or
        // dismissing an existing Find session in the destination.
        let worker = target.read(cx).session.search_worker()?;
        self.focus_terminal(workspace, terminal, window, cx);
        target.update(cx, |view, cx| {
            view.dismiss_find(cx);
            view.install_find(worker, query.to_owned(), window, cx);
            if let Some(find) = &view.find {
                find.update(cx, |find, cx| find.navigate(rmux_terminal::search::Direction::Older, cx));
            }
        });
        Ok(())
    }
    fn collect_terminal_search_jobs(&self, target: Option<AnyWindowHandle>, label: &str, jobs: &mut Vec<terminal_search::Job>, extra: &mut usize, cx: &App) {
        for index in self.workspace_order(true) {
            let entry = &self.workspace_state.entries[index];
            for pane in &entry.workspace.read(cx).panes {
                for (tab, terminal) in pane.read(cx).terminals.iter().enumerate() {
                    if jobs.len() == terminal_search::MAX_TERMINALS { *extra += 1; continue; }
                    jobs.push(terminal_search::Job { window: target, workspace: entry.workspace.entity_id(), terminal: terminal.entity_id(),
                        name: format!("{label} · {} / {}", entry.name, pane.read(cx).tab_title(tab, cx)),
                        probe: terminal.read(cx).session.history_probe(),
                        read: terminal.read(cx).session.history_export(terminal_search::MAX_BYTES) });
                }
            }
        }
    }
    fn search_terminal_output(&mut self, query: String, window: &mut Window, cx: &mut Context<Self>) {
        if self.terminal_search_job.is_some() || self.automation_available(cx).is_err() { return; }
        if let Err(error) = terminal_search::validate(&query) { self.notice = Some(error.into()); cx.notify(); return; }
        let Some(focus) = window.focused(cx) else { return; };
        let Some(permit) = terminal_search::Permit::acquire() else {
            self.notice = Some("Another terminal search is running; retry shortly.".into()); cx.notify(); return;
        };
        self.terminal_search_query.clone_from(&query);
        let mut jobs = Vec::new();
        let mut extra = 0;
        self.collect_terminal_search_jobs(None, "This window", &mut jobs, &mut extra, cx);
        let current = window.window_handle();
        for (index, handle) in cx.windows().into_iter().enumerate() {
            if handle == current { continue; }
            let Some(root) = handle.downcast::<Self>() else { continue; };
            if let Ok(root) = root.read(cx) {
                root.collect_terminal_search_jobs(Some(handle), &format!("Window {}", index + 1), &mut jobs, &mut extra, cx);
            }
        }
        let cancellation = file_browser::Cancellation::default();
        let canceled = cancellation.0.clone();
        self.terminal_search_cancel = Some(cancellation);
        self.notice = Some("Searching terminal output…".into());
        let executor = cx.background_executor().clone();
        self.terminal_search_job = Some(cx.spawn_in(window, async move |entity, cx| {
            let scan_query = query.clone();
            let (progress_tx, progress_rx) = async_channel::bounded(1);
            let scan = executor.spawn(async move {
                let _permit = permit;
                let mut published = None;
                terminal_search::scan_with_progress(jobs, &scan_query, &canceled, |progress| {
                    let now = std::time::Instant::now();
                    if published.is_none_or(|then| now.duration_since(then) >= std::time::Duration::from_millis(50)) {
                        if progress_tx.try_send(progress).is_ok() { published = Some(now); }
                    }
                })
            });
            while let Ok(progress) = progress_rx.recv().await {
                let _ = cx.update(|_, cx| entity.update(cx, |this, cx| {
                    if this.notice.as_deref().is_some_and(|notice| notice.starts_with("Searching terminal output…")) {
                        this.notice = Some(format!("Searching terminal output… {}/{} previews · {} matches · {} deep scans remaining",
                            progress.previews, progress.total, progress.matches, progress.deep_remaining));
                        cx.notify();
                    }
                }));
            }
            let report = scan.await;
            let _ = cx.update(|window, cx| entity.update(cx, |this, cx| {
                this.terminal_search_job = None;
                this.terminal_search_cancel = None;
                if this.notice.as_deref().is_some_and(|notice| notice.starts_with("Searching terminal output…")) { this.notice = None; }
                if focus.is_focused(window) && this.automation_available(cx).is_ok() {
                    let palette = cx.new(|cx| command_palette::CommandPalette::terminal_search_results(report, query, extra, window, cx));
                    this.install_palette(palette, Some(focus), window, cx);
                }
                cx.notify();
            }));
        }));
        cx.notify();
    }
    fn choose_local_path(&mut self, directory: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.file_browser_job.is_some() || self.automation_available(cx).is_err() { return; }
        let Some(permit) = cx.global::<PromptGate>().acquire() else { return; };
        let picker = cx.prompt_for_paths(PathPromptOptions {
            files: !directory, directories: directory, multiple: false,
            prompt: Some(if directory { "Browse Folder" } else { "Preview Text File" }.into()),
        });
        cx.spawn_in(window, async move |entity, cx| {
            let result: anyhow::Result<Option<std::path::PathBuf>> = async {
                Ok(picker.await??.and_then(|paths| paths.into_iter().next()))
            }.await;
            drop(permit);
            let _ = cx.update(|window, cx| entity.update(cx, |this, cx| {
                match result {
                    Ok(Some(path)) => {
                        if directory { this.browse_directory(path, window, cx); }
                        else { this.preview_local_file(path, window, cx); }
                    },
                    Ok(None) => {},
                    Err(error) => {
                        let kind = if directory { "folder" } else { "preview file" };
                        this.notice = Some(format!("Choose {kind}: {error:#}")); cx.notify();
                    },
                }
            }));
        }).detach();
    }
    fn edit_local_file(&mut self, action: EditLocalFile, cx: &mut Context<Self>) {
        let Some(editor) = cx.global::<Preferences>().file_editor.clone() else {
            self.notice = Some("Configure file_editor.program and file_editor.args in Settings first".into()); cx.notify(); return;
        };
        self.launch_external_editor(editor, action, cx);
    }
    fn launch_external_editor(&mut self, editor: external_editor::Editor, action: EditLocalFile, cx: &mut Context<Self>) {
        let executor = cx.background_executor().clone();
        cx.spawn(async move |entity, cx| {
            let result = match executor.spawn(async move { editor.launch_at(&action.path, action.line, action.column) }).await {
                Ok(child) => executor.spawn(child.reap()).await,
                Err(error) => Err(error),
            };
            if let Err(error) = result {
                let _ = entity.update(cx, |this, cx| { this.notice = Some(format!("External editor: {error:#}")); cx.notify(); });
            }
        }).detach();
    }
    fn preview_local_file(&mut self, path: std::path::PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        self.preview_file_mode(path, false, window, cx);
    }
    fn preview_file_mode(&mut self, path: std::path::PathBuf, hex: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.preview_file_kind(path, hex, false, window, cx);
    }
    fn preview_history(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        if let Some((index, state)) = self.preview_history.adjacent(forward) {
            let (path, hex, markdown) = state.target();
            self.preview_file_at(path, hex, markdown, Some((index, state)), window, cx);
        }
    }
    fn preview_file_kind(&mut self, path: std::path::PathBuf, hex: bool, markdown: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.preview_file_at(path, hex, markdown, None, window, cx);
    }
    fn preview_file_at(&mut self, path: std::path::PathBuf, hex: bool, markdown: bool,
        navigation: Option<(usize, command_palette::PreviewState)>, window: &mut Window, cx: &mut Context<Self>) {
        if self.file_browser_job.is_some() || self.automation_available(cx).is_err() { return; }
        let source = self.preview_restore.take().or_else(|| self.palette.as_ref().and_then(|(palette, _)| palette.read(cx).preview_state(cx)));
        if let Some(state) = &source { self.preview_history.update(state.clone()); }
        let restore = navigation.as_ref().map(|(_, state)| state.clone()).or_else(|| source.filter(|state| state.matches(&path, hex, markdown)));
        let navigation = navigation.map(|(index, _)| index);
        let Some(focus) = window.focused(cx) else { return; };
        let permit = match file_browser::Permit::acquire() {
            Ok(permit) => permit,
            Err(error) => { self.notice = Some(error.to_string()); cx.notify(); return; }
        };
        let executor = cx.background_executor().clone();
        self.notice = Some("Loading file preview…".into());
        let cancellation = file_browser::Cancellation::default();
        let canceled = cancellation.0.clone();
        self.file_browser_cancel = Some(cancellation);
        self.file_browser_job = Some(cx.spawn_in(window, async move |entity, cx| {
            let target = path.clone();
            let result = executor.spawn(async move {
                let _permit = permit;
                let preview = if hex { file_browser::hex_preview_cancellable(path, &canceled) } else { file_browser::preview_cancellable(path, &canceled) }?;
                let blocks = markdown.then(|| markdown_preview::blocks(&preview.contents));
                Ok::<_, anyhow::Error>((preview, blocks))
            }).await;
            let _ = cx.update(|window, cx| entity.update(cx, |this, cx| {
                this.file_browser_job = None;
                this.file_browser_cancel = None;
                if this.notice.as_deref() == Some("Loading file preview…") { this.notice = None; }
                if focus.is_focused(window) && this.automation_available(cx).is_ok() {
                    this.preview_history.commit(command_palette::PreviewState::new(target.clone(), hex, markdown), navigation);
                    match result {
                        Ok((preview, blocks)) => {
                            let palette = cx.new(|cx| {
                                let mut palette = if let Some((blocks, limited)) = blocks {
                                    command_palette::CommandPalette::markdown_file(preview, blocks, limited, window, cx)
                                } else { command_palette::CommandPalette::file_preview(preview, window, cx) };
                                palette.set_preview_history(&this.preview_history);
                                if let Some(state) = restore { palette.restore_preview(state, cx); }
                                palette
                            });
                            this.install_palette(palette, Some(focus), window, cx);
                        }
                        Err(error) => {
                            let palette = cx.new(|cx| {
                                let mut palette = command_palette::CommandPalette::file_error(target, format!("{error:#}"), false, window, cx);
                                palette.set_preview_history(&this.preview_history);
                                palette
                            });
                            this.install_palette(palette, Some(focus), window, cx);
                        }
                    }
                }
                cx.notify();
            }));
        }));
        cx.notify();
    }
    fn show_font_picker(&mut self, default: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.font_picker_job.is_some() || self.automation_available(cx).is_err() { return; }
        let Some(focus) = window.focused(cx) else { return; };
        let text = cx.text_system().clone();
        let executor = cx.background_executor().clone();
        self.notice = Some("Loading installed fonts…".into());
        self.font_picker_job = Some(cx.spawn_in(window, async move |entity, cx| {
            let families = executor.spawn(async move { text.all_font_names() }).await;
            let _ = cx.update(|window, cx| entity.update(cx, |this, cx| {
                this.font_picker_job = None;
                if this.notice.as_deref() == Some("Loading installed fonts…") { this.notice = None; }
                if focus.is_focused(window) && this.automation_available(cx).is_ok() {
                    let palette = cx.new(|cx| command_palette::CommandPalette::fonts(families, default, window, cx));
                    this.install_palette(palette, Some(focus), window, cx);
                }
                cx.notify();
            }));
        }));
        cx.notify();
    }
    fn install_palette(&mut self, palette: Entity<command_palette::CommandPalette>, focus: Option<FocusHandle>, window: &mut Window, cx: &mut Context<Self>) {
        // A clicked document link can replace a palette without closing it
        // first. Escape must return to the original terminal, not the search
        // field of the palette that is about to be dropped.
        let focus = self.palette.as_ref().map(|(_, origin)| origin.clone()).unwrap_or(focus);
        cx.subscribe_in(
            &palette,
            window,
            |this, _, event: &command_palette::PaletteEvent, window, cx| {
                // External editing does not navigate the rmux file view. Keep
                // its query, selection, wrapping, scroll and focus owner intact
                // so returning from the editor can refresh the same document.
                if let command_palette::PaletteEvent::Run(action) = event
                    && let Some(action) = action.as_any().downcast_ref::<EditLocalFile>() {
                    this.edit_local_file(action.clone(), cx);
                    return;
                }
                this.folder_restore = match event {
                    command_palette::PaletteEvent::Run(action) if action.as_any().is::<BrowseDirectory>() =>
                        this.palette.as_ref().and_then(|(palette, _)| palette.read(cx).folder_state(cx)),
                    _ => None,
                };
                this.preview_restore = match event {
                    command_palette::PaletteEvent::Run(action) if action.as_any().is::<PreviewLocalFile>() || action.as_any().is::<PreviewHexFile>() || action.as_any().is::<PreviewMarkdownFile>() || action.as_any().is::<PreviewBack>() || action.as_any().is::<PreviewForward>() =>
                        this.palette.as_ref().and_then(|(palette, _)| palette.read(cx).preview_state(cx)),
                    _ => None,
                };
                this.close_palette(window, cx);
                match event {
                    command_palette::PaletteEvent::Run(action) => {
                        window.dispatch_action(action.boxed_clone(), cx)
                    }
                    command_palette::PaletteEvent::SelectWorkspace(id) => {
                        if let Some(index) = this
                            .workspace_state.entries
                            .iter()
                            .position(|entry| entry.workspace.entity_id() == *id)
                        {
                            this.select(index, window, cx);
                        }
                    }
                    command_palette::PaletteEvent::MoveTabToWorkspace(id) => {
                        this.move_tab_to_workspace(*id, window, cx)
                    }
                    command_palette::PaletteEvent::MoveTabToWindowWorkspace(destination, workspace) => {
                        if let Err(error) = this.move_tab_to_window_workspace(*destination, *workspace, window, cx) {
                            this.notice = Some(format!("Move tab: {error}"));
                            cx.notify();
                        }
                    }
                    command_palette::PaletteEvent::MergeWorkspaces(source, destination, target) => {
                        if let Err(error) = this.merge_workspace_to_window(*source, *destination, *target, true, window, cx) {
                            this.notice = Some(format!("Merge workspace: {error}")); cx.notify();
                        }
                    }
                    command_palette::PaletteEvent::MoveWorkspaceToWindow(source, destination) => {
                        this.move_workspace_to_window(*source, *destination, window, cx);
                    }
                    command_palette::PaletteEvent::SelectTerminal(workspace, terminal) => {
                        this.focus_terminal(*workspace, *terminal, window, cx);
                    }
                    command_palette::PaletteEvent::SearchTerminal { window: target, workspace, terminal, query, .. } => {
                        let result = if let Some(target) = target {
                            target.downcast::<Self>().ok_or_else(|| anyhow::anyhow!("Search result window is unavailable"))
                                .and_then(|target| target.update(cx, |destination, window, cx| -> Result<()> {
                                    destination.focus_search_result(*workspace, *terminal, query, window, cx)?;
                                    window.activate_window();
                                    Ok(())
                                })?)
                        } else { this.focus_search_result(*workspace, *terminal, query, window, cx) };
                        if let Err(error) = result { this.notice = Some(format!("Open search result: {error}")); cx.notify(); }
                    }
                    command_palette::PaletteEvent::SelectWindowTerminal(target, workspace, terminal) => {
                        let result = target.downcast::<Self>().ok_or_else(|| anyhow::anyhow!("window unavailable"))
                            .and_then(|target| target.update(cx, |destination, window, cx| -> Result<()> {
                                destination.automation_available(cx).map_err(|failure| anyhow::anyhow!(failure.message))?;
                                anyhow::ensure!(destination.automation_surface_target(*terminal, cx).is_some_and(|(owner, _)| owner == *workspace), "terminal moved or closed");
                                destination.focus_terminal(*workspace, *terminal, window, cx);
                                window.activate_window();
                                Ok(())
                            })?);
                        if let Err(error) = result { this.notice = Some(format!("Switch terminal: {error}")); cx.notify(); }
                    }
                    command_palette::PaletteEvent::SelectWindowWorkspace(target, workspace) => {
                        let result = target.downcast::<Self>().ok_or_else(|| anyhow::anyhow!("window unavailable"))
                            .and_then(|target| target.update(cx, |destination, window, cx| -> Result<()> {
                                destination.automation_available(cx).map_err(|failure| anyhow::anyhow!(failure.message))?;
                                let index = destination.workspace_state.entries.iter().position(|entry| entry.workspace.entity_id() == *workspace)
                                    .ok_or_else(|| anyhow::anyhow!("workspace moved or closed"))?;
                                destination.select(index, window, cx);
                                window.activate_window();
                                Ok(())
                            })?);
                        if let Err(error) = result { this.notice = Some(format!("Switch workspace: {error}")); cx.notify(); }
                    }
                    command_palette::PaletteEvent::Close => {}
                }
            },
        )
        .detach();
        self.palette = Some((palette, focus));
        cx.notify();
    }
    fn move_tab_to_window_workspace(&mut self, target: AnyWindowHandle, destination_id: EntityId, window: &mut Window, cx: &mut Context<Self>) -> Result<()> {
        let workspace = self.workspace_state.entries.get(self.workspace_state.active).ok_or_else(|| anyhow::anyhow!("source workspace closed"))?.workspace.clone();
        let index = workspace.read(cx).focused_index(window, cx).ok_or_else(|| anyhow::anyhow!("no focused pane"))?;
        let id = workspace.read(cx).panes[index].read(cx).active_terminal_id().ok_or_else(|| anyhow::anyhow!("source tab closed"))?;
        self.move_surface_to_workspace(id, target, destination_id, true, window, cx)
    }
    pub(super) fn move_surface_to_workspace(&mut self, id: EntityId, target: AnyWindowHandle, destination_id: EntityId, activate: bool, window: &mut Window, cx: &mut Context<Self>) -> Result<()> {
        self.automation_available(cx).map_err(|failure| anyhow::anyhow!(failure.message))?;
        let (workspace, source) = self.workspace_state.entries.iter().find_map(|entry| {
            entry.workspace.read(cx).panes.iter().find(|pane| pane.read(cx).terminals.iter().any(|terminal| terminal.entity_id() == id))
                .map(|pane| (entry.workspace.clone(), pane.clone()))
        }).ok_or_else(|| anyhow::anyhow!("source tab closed"))?;
        if target == window.window_handle() {
            anyhow::ensure!(self.automation_contains_workspace(destination_id), "destination workspace closed");
            self.transfer_tab(workspace.entity_id(), source.entity_id(), id, destination_id, window, cx);
            if activate { activate_destination(window, cx); }
            return Ok(());
        }
        let target = target.downcast::<Self>().ok_or_else(|| anyhow::anyhow!("invalid destination window"))?;
        let (_, tab) = source.read(cx).tab_transfer(id).ok_or_else(|| anyhow::anyhow!("source tab closed"))?;
        // Validate the live destination before rebinding or removing the source.
        // GPUI updates are synchronous; no other command can interleave here.
        target.update(cx, move |destination, window, cx| -> Result<()> {
            destination.automation_available(cx).map_err(|failure| anyhow::anyhow!(failure.message))?;
            let index = destination.workspace_state.entries.iter().position(|entry| entry.workspace.entity_id() == destination_id)
                .ok_or_else(|| anyhow::anyhow!("destination workspace closed"))?;
            let entry = &destination.workspace_state.entries[index];
            let workspace = entry.workspace.clone();
            let pane = workspace.read(cx).panes.iter()
                .find(|pane| Some(pane.entity_id()) == entry.last_pane)
                .or_else(|| workspace.read(cx).panes.first()).cloned()
                .ok_or_else(|| anyhow::anyhow!("destination has no pane"))?;
            destination.remember_focus(window, cx);
            tab.terminal.update(cx, |terminal, cx| terminal.rebind_window(window, cx));
            workspace.update(cx, |workspace, cx| {
                workspace.pane_state.focus_changed(pane.entity_id());
                cx.notify();
            });
            destination.activate_workspace(index);
            destination.workspace_state.entries[index].last_pane = Some(pane.entity_id());
            destination.scroll_to_workspace(index);
            window.set_window_title(&format!("{} — rmux", destination.workspace_state.entries[index].name));
            pane.update(cx, |pane, cx| pane.append_tabs(vec![tab], 0, window, cx));
            cx.notify();
            Ok(())
        })??;
        source.update(cx, |pane, cx| pane.take_tab(id, cx));
        if source.read(cx).terminals.is_empty() {
            workspace.update(cx, |workspace, cx| workspace.remove_pane(&source, cx));
        } else if self.workspace_state.entries.get(self.workspace_state.active).is_some_and(|entry| entry.workspace == workspace) {
            source.read(cx).focus(window, cx);
        }
        if activate { let _ = target.update(cx, |_, window, cx| activate_destination(window, cx)); }
        cx.notify();
        Ok(())
    }
    fn move_tab_next_workspace(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.workspace_state.entries.len() < 2 {
            return;
        }
        let order = self.workspace_order(true);
        let Some(position) = order.iter().position(|index| *index == self.workspace_state.active) else { return; };
        let destination = order[(position + 1) % order.len()];
        self.move_tab_to_workspace(self.workspace_state.entries[destination].workspace.entity_id(), window, cx);
    }
    fn move_tab_to_workspace(&mut self, id: EntityId, window: &mut Window, cx: &mut Context<Self>) {
        let source_workspace = self.workspace_state.entries[self.workspace_state.active].workspace.clone();
        let Some(source_index) = source_workspace.read(cx).focused_index(window, cx) else {
            return;
        };
        let source = &source_workspace.read(cx).panes[source_index];
        let Some(terminal) = source.read(cx).active_terminal_id() else {
            return;
        };
        self.transfer_tab(
            source_workspace.entity_id(),
            source.entity_id(),
            terminal,
            id,
            window,
            cx,
        );
    }
    fn transfer_tab(
        &mut self,
        workspace_id: EntityId,
        pane_id: EntityId,
        terminal: EntityId,
        destination_id: EntityId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.workspace_state.entries.len() < 2 || cx.global::<Shutdowns>().quitting {
            return;
        }
        let Some(destination) = self
            .workspace_state.entries
            .iter()
            .position(|entry| entry.workspace.entity_id() == destination_id)
        else {
            return;
        };
        let Some(origin) = self
            .workspace_state.entries
            .iter()
            .position(|entry| entry.workspace.entity_id() == workspace_id)
        else {
            return;
        };
        if destination == origin {
            return;
        }
        let source_workspace = self.workspace_state.entries[origin].workspace.clone();
        let Some(source) = source_workspace
            .read(cx)
            .panes
            .iter()
            .find(|pane| pane.entity_id() == pane_id)
            .cloned()
        else {
            return;
        };
        let entry = &self.workspace_state.entries[destination];
        let destination_workspace = entry.workspace.clone();
        let Some(target) = destination_workspace
            .read(cx)
            .panes
            .iter()
            .find(|pane| Some(pane.entity_id()) == entry.last_pane)
            .or_else(|| destination_workspace.read(cx).panes.first())
            .cloned()
        else {
            return;
        };
        let ready = source_workspace.update(cx, |workspace, cx| workspace.tab_transfer_ready(&source, None, cx))
            && destination_workspace.update(cx, |workspace, cx| workspace.tab_transfer_ready(&target, None, cx));
        if !ready {
            self.notice = Some("Finish pending splits, tab creation and pane editing before moving tabs between workspaces.".into());
            cx.notify();
            return;
        }
        self.workspace_state.entries[origin].last_pane = Some(source.entity_id());
        let Some(tab) = source.update(cx, |pane, cx| pane.take_tab(terminal, cx)) else {
            return;
        };
        if source.read(cx).terminals.is_empty() {
            source_workspace.update(cx, |workspace, cx| workspace.remove_pane(&source, cx));
        }
        destination_workspace.update(cx, |workspace, cx| {
            workspace.pane_state.focus_changed(target.entity_id());
            cx.notify();
        });
        // Set the destination before selecting the transferred terminal, without
        // briefly focusing (and acknowledging) the destination's old active tab.
        self.rename = None;
        self.activate_workspace(destination);
        self.workspace_state.entries[destination].last_pane = Some(target.entity_id());
        self.scroll_to_workspace(destination);
        window.set_window_title(&format!("{} — rmux", self.workspace_state.entries[destination].name));
        target.update(cx, |pane, cx| pane.append_tabs(vec![tab], 0, window, cx));
        cx.notify();
    }

    fn next_unread(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let order = self.workspace_order(true);
        let start = order.iter().position(|index| *index == self.workspace_state.active).unwrap_or(0);
        let target = order.iter().cycle().skip(start).take(order.len()).find_map(|index| {
            let workspace = self.workspace_state.entries[*index].workspace.read(cx);
            workspace.panes.iter().find_map(|pane| {
                pane.read(cx).terminals.iter()
                    .find(|terminal| terminal.read(cx).attention.any())
                    .map(|terminal| (self.workspace_state.entries[*index].workspace.entity_id(), terminal.entity_id()))
            })
        });
        let Some((workspace, terminal)) = target else { return; };
        if let Some(id) = self.workspace_state.groups.containing(workspace).map(|group| group.id) {
            self.workspace_state.groups.get_mut(id).unwrap().collapsed = false;
        }
        // Shared direct focus avoids acknowledging a previously active tab,
        // reveals the target pane under zoom and remembers the destination.
        self.focus_terminal(workspace, terminal, window, cx);
    }

    pub(super) fn restored(
        prepared: restore::Prepared,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        Self::observe_settings(cx);
        let (snapshot, mut sessions) = prepared.into_parts();
        let mut this = Self {
            palette: None,
            font_picker_job: None,
            file_browser_job: None,
            preview_restore: None,
            folder_restore: None,
            file_history: Default::default(),
            preview_history: Default::default(),
            file_browser_cancel: None,
            terminal_search_job: None,
            terminal_search_query: String::new(),
            terminal_search_cancel: None,
            restore_job: None,
            workspace_scroll: ScrollHandle::new(),
            notice: None,
            memory_rows: None,
            memory_scroll: ListState::new(0, ListAlignment::Top, px(80.)),
            memory_focus: None,
            memory_previous_focus: None,
            memory_selected: 0,
            memory_application: None,
            memory_sampled_at: None,
            memory_sort: Default::default(),
            memory_loading: false,
            memory_generation: 0,
            memory_canceled: None,
            process_sampler: Default::default(),
            workspace_menu: None,
            workspace_menu_blur: None,
            rename: None,
            workspace_state: rmux_core::workspace_lifecycle::State { active: snapshot.active_workspace, ..Default::default() },
            rendered_workspace: None,
            rendered_background: None,
            close_prompt: None,
            window_close_prompt: false,
            close_handler_installed: false,
            focus_history: Vec::new(),
            focus_trail: Default::default(),
            notification_navigation: None,
            settings_state: None,
            command_launching: false,
            workspace_launch_cancel: None,
            next_number: snapshot.workspaces.len() + 1,
            sidebar: snapshot.sidebar,
            sidebar_width: snapshot.sidebar_width,
            sidebar_drag: None,
            benchmark: false,
        };
        for saved in snapshot.workspaces {
            let workspace = cx.new(|cx| {
                let mut workspace = Workspace {
                    event_lifetime: cx.global::<events::Journal>().workspace(cx.entity_id()),
                    spawning_split: None,
                    split_launch_canceled: None,
                    pane_subscriptions: Default::default(),
                    split_drag: None,
                    panes: Vec::new(),
                    pane_state: Default::default(),
                    error: None,
                    benchmark: false,
                };
                workspace.pane_state.layout =
                    Some(workspace.restore_node(saved.layout, &mut sessions, window, cx));
                workspace.pane_state.zoomed = saved.zoomed_pane.map(|index| workspace.panes[index].entity_id());
                workspace
            });
            let last_pane = Some(workspace.read(cx).panes[saved.zoomed_pane.unwrap_or(saved.active_pane)].entity_id());
            let closed = Self::observe_workspace(&workspace, window, cx);
            let metadata = Self::observe_metadata(&workspace, window, cx);
            this.workspace_state.entries.push(Entry {
                workspace,
                name: saved.name,
                description: saved.description,
                project_directory: saved.project_directory,
                color: saved.color,
                icon: saved.icon,
                pinned: saved.pinned,
                last_pane,
                _closed: closed,
                _metadata: metadata,
            });
        }
        // Resolve persisted indices to live workspace identities.
        let groups = snapshot.groups.iter().map(|group| group.map_members(|index| {
            Ok(this.workspace_state.entries[*index].workspace.entity_id())
        })).collect::<Result<Vec<_>>>().expect("validated group indices");
        this.workspace_state.groups = workspace_groups::Groups::from_entries(groups, |_| true).expect("validated groups");
        // Saved entry order is authoritative; sidebar pin tiers are a projection.
        // Terminal constructors may focus the final prepared tab. Establish the
        // saved focus before select() records the current workspace's focus.
        let entry = &this.workspace_state.entries[this.workspace_state.active];
        let workspace = entry.workspace.read(cx);
        if let Some(pane) = workspace
            .panes
            .iter()
            .find(|pane| Some(pane.entity_id()) == entry.last_pane)
        {
            pane.read(cx).focus(window, cx);
        }
        this.select(this.workspace_state.active, window, cx);
        // Selecting a hidden member normally expands its group. Startup instead
        // preserves the saved collapsed state while keeping that terminal active.
        for saved in &snapshot.groups {
            if let Some(group) = this.workspace_state.groups.get_mut(saved.id) { group.collapsed = saved.collapsed; }
        }
        this.scroll_to_workspace(this.workspace_state.active);
        this
    }

    fn show_custom_command_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.automation_available(cx).is_err() { return; }
        self.close_palette(window, cx);
        let focus = window.focused(cx);
        let palette = cx.new(|cx| command_palette::CommandPalette::custom_commands(window, cx));
        self.install_palette(palette, focus, window, cx);
    }
    fn show_layout_preset_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.restore_job.is_some() || cx.global::<Shutdowns>().quitting { return; }
        self.close_palette(window, cx);
        let focus = window.focused(cx);
        let palette = cx.new(|cx| command_palette::CommandPalette::layout_presets(window, cx));
        self.install_palette(palette, focus, window, cx);
    }
    fn cancel_restore(&mut self, cx: &mut Context<Self>) {
        if let Some(job) = &self.restore_job {
            job.cancel();
            self.notice = Some("Cancelling restore…".into());
            cx.notify();
        }
    }
    fn open_layout(&mut self, cx: &mut Context<Self>) { self.open_snapshot(false, None, None, cx); }
    fn open_snapshot(&mut self, session: bool, path: Option<std::path::PathBuf>, directory: Option<std::path::PathBuf>, cx: &mut Context<Self>) {
        let kind = if session { "session" } else { "layout" };
        if cx.global::<Shutdowns>().quitting || self.restore_job.is_some() {
            return;
        }
        let job = restore::Job::default();
        let token = job.0.clone();
        self.restore_job = Some(job);
        cx.notify();
        let picker = path.is_none().then(|| cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some(if session { "Open Session" } else { "Open Layout" }.into()),
        }));
        let scrollback_bytes = cx.global::<Preferences>().scrollback_bytes();
        let executor = cx.background_executor().clone();
        let (progress_sender, progress_receiver) = async_channel::bounded(1);
        let progress_token = token.clone();
        cx.spawn(async move |entity, cx| {
            while let Ok((completed, total)) = progress_receiver.recv().await {
                let _ = entity.update(cx, |this, cx| {
                    if !restore::cancelled(&progress_token) && this.restore_job.as_ref().is_some_and(|job| Arc::ptr_eq(&job.0, &progress_token)) {
                        this.notice = Some(format!("Opening {kind} · Preparing terminals {completed}/{total}…"));
                        cx.notify();
                    }
                });
            }
        }).detach();
        cx.spawn(async move |entity, cx| {
            let result = async {
                let path = match path {
                    Some(path) => path,
                    None => {
                        let Some(paths) = picker.expect("picker for interactive import").await?? else { return Ok::<_, anyhow::Error>(()); };
                        let Some(path) = paths.into_iter().next() else { return Ok(()); };
                        path
                    }
                };
                let _ = entity.update(cx, |this, cx| {
                    this.notice = Some(format!("Opening {kind}…"));
                    cx.notify();
                });
                let worker_token = token.clone();
                let prepared = executor
                    .spawn(async move {
                        restore::PreparedSession::read_import(&path, session, directory.as_deref(), &worker_token, scrollback_bytes,
                            |completed, total| { let _ = progress_sender.try_send((completed, total)); })
                    })
                    .await?;
                cx.update(|cx| -> anyhow::Result<()> {
                    if cx.global::<Shutdowns>().quitting || restore::cancelled(&token) {
                        return Ok(());
                    }
                    restore::open_windows(prepared, cx)?;
                    Ok(())
                })??;
                Ok(())
            }
            .await;
            let was_cancelled = restore::cancelled(&token);
            let _ = entity.update(cx, |this, cx| {
                this.restore_job = None;
                this.notice = if was_cancelled {
                    Some(format!("{kind} restore cancelled"))
                } else {
                    result.err().map(|error| format!("Open {kind}: {error}"))
                };
                cx.notify();
            });
        })
        .detach();
    }

    pub fn new(panes: Option<usize>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut this = Self::empty(panes, cx);
        this.add(panes.unwrap_or(1), window, cx);
        this
    }
    fn empty(panes: Option<usize>, cx: &mut Context<Self>) -> Self {
        Self::observe_settings(cx);
        Self {
            palette: None,
            font_picker_job: None,
            file_browser_job: None,
            preview_restore: None,
            folder_restore: None,
            file_history: Default::default(),
            preview_history: Default::default(),
            file_browser_cancel: None,
            terminal_search_job: None,
            terminal_search_query: String::new(),
            terminal_search_cancel: None,
            restore_job: None,
            workspace_scroll: ScrollHandle::new(),
            notice: None,
            memory_rows: None,
            memory_scroll: ListState::new(0, ListAlignment::Top, px(80.)),
            memory_focus: None,
            memory_previous_focus: None,
            memory_selected: 0,
            memory_application: None,
            memory_sampled_at: None,
            memory_sort: Default::default(),
            memory_loading: false,
            memory_generation: 0,
            memory_canceled: None,
            process_sampler: Default::default(),
            workspace_menu: None,
            workspace_menu_blur: None,
            rename: None,
            workspace_state: Default::default(),
            rendered_workspace: None,
            rendered_background: None,
            close_prompt: None,
            window_close_prompt: false,
            close_handler_installed: false,
            focus_history: Vec::new(),
            focus_trail: Default::default(),
            notification_navigation: None,
            settings_state: None,
            command_launching: false,
            workspace_launch_cancel: None,
            next_number: 1,
            sidebar: panes.is_none(),
            sidebar_width: 200.,
            sidebar_drag: None,
            benchmark: panes.is_some(),
        }
    }

    fn remember_focus(&mut self, window: &Window, cx: &App) {
        if let Some(entry) = self.workspace_state.entries.get_mut(self.workspace_state.active) {
            let workspace = entry.workspace.read(cx);
            if let Some(index) = workspace.focused_index(window, cx) {
                entry.last_pane = Some(workspace.panes[index].entity_id());
            }
        }
    }

    pub(super) fn layout_snapshot(&mut self, window: &Window, cx: &App) -> Result<snapshot::WindowSnapshot> {
        let mut saved = self.window_session_snapshot(window, cx)?;
        saved.clear_shell_history();
        Ok(saved)
    }

    fn window_session_snapshot(&mut self, window: &Window, cx: &App) -> Result<snapshot::WindowSnapshot> {
        self.remember_focus(window, cx);
        Ok(snapshot::WindowSnapshot {
                groups: self.workspace_state.groups.entries().iter().map(|group| group.map_members(|id| {
                    self.workspace_state.entries.iter().position(|entry| entry.workspace.entity_id() == *id)
                        .ok_or_else(|| anyhow::anyhow!("group references a missing workspace"))
                })).collect::<Result<_>>()?,
                geometry: Some(window_geometry::Geometry::capture(window)),
                version: 1,
                sidebar: self.sidebar,
                sidebar_width: self.sidebar_width,
                active_workspace: self.workspace_state.active,
                workspaces: self
                    .workspace_state.entries
                    .iter()
                    .map(|entry| {
                        let workspace = entry.workspace.read(cx);
                        let layout = workspace
                            .pane_state.layout
                            .as_ref()
                            .ok_or_else(|| anyhow::anyhow!("workspace has no terminal layout"))?;
                        Ok(snapshot::WorkspaceSnapshot {
                            name: entry.name.clone(),
                            description: entry.description.clone(),
                            project_directory: entry.project_directory.clone(),
                            color: entry.color,
                            icon: entry.icon.clone(),
                            pinned: entry.pinned,
                            active_pane: workspace
                                .panes
                                .iter()
                                .find(|pane| Some(pane.entity_id()) == entry.last_pane)
                                .and_then(|pane| layout.pane_index(pane))
                                .unwrap_or(0),
                            zoomed_pane: workspace.pane_state.zoomed.and_then(|id| workspace.panes.iter().find(|pane| pane.entity_id() == id))
                                .and_then(|pane| layout.pane_index(pane)),
                            layout: Workspace::snapshot_tree(layout, cx),
                        })
                    })
                    .collect::<anyhow::Result<_>>()?,
        })
    }

    pub(super) fn session_snapshot(&mut self, window: &Window, cx: &mut Context<Self>) -> Result<snapshot::SessionSnapshot> {
        self.automation_available(cx).map_err(|failure| anyhow::anyhow!(failure.message))?;
        let current = window.window_handle();
        let active = cx.active_window();
        let mut snapshot = snapshot::SessionSnapshot { version: 1, active_window: None, windows: Vec::new() };
        for handle in cx.windows() {
            let Some(typed) = handle.downcast::<Self>() else { continue; };
            let saved = if handle == current {
                self.window_session_snapshot(window, cx)?
            } else {
                if typed.read(cx).is_ok_and(|view| view.benchmark) { continue; }
                typed.update(cx, |view, window, cx| -> Result<_> {
                    view.automation_available(cx).map_err(|failure| anyhow::anyhow!(failure.message))?;
                    view.window_session_snapshot(window, cx)
                })??
            };
            if active == Some(handle) { snapshot.active_window = Some(snapshot.windows.len()); }
            snapshot.windows.push(saved);
        }
        Ok(snapshot)
    }

    fn save_session(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let snapshot = match self.session_snapshot(window, cx) {
            Ok(snapshot) => snapshot,
            Err(error) => { self.notice = Some(format!("Save session: {error}")); cx.notify(); return; }
        };
        let directory = std::env::current_dir().unwrap_or_default();
        let picker = cx.prompt_for_new_path(&directory, Some("rmux-session.json"));
        let executor = cx.background_executor().clone();
        cx.spawn(async move |entity, cx| {
            let result = async {
                let Some(path) = picker.await?? else { return Ok::<_, anyhow::Error>(None); };
                let label = path.display().to_string();
                executor.spawn(async move { snapshot.save(&path) }).await?;
                Ok(Some(format!("Saved session to {label}")))
            }.await;
            let _ = entity.update(cx, |this, cx| {
                this.notice = match result { Ok(message) => message, Err(error) => Some(format!("Save session: {error}")) };
                cx.notify();
            });
        }).detach();
    }

    fn save_layout(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let snapshot = match self.layout_snapshot(window, cx) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                self.notice = Some(error.to_string());
                cx.notify();
                return;
            }
        };
        let directory = std::env::current_dir().unwrap_or_default();
        let picker = cx.prompt_for_new_path(&directory, Some("rmux-layout.json"));
        let executor = cx.background_executor().clone();
        cx.spawn(async move |entity, cx| {
            let result = async {
                let Some(path) = picker.await?? else {
                    return Ok::<_, anyhow::Error>(None);
                };
                let label = path.display().to_string();
                executor.spawn(async move { snapshot.save(&path) }).await?;
                Ok(Some(format!("Saved layout to {label}")))
            }
            .await;
            let _ = entity.update(cx, |this, cx| {
                this.notice = match result {
                    Ok(message) => message,
                    Err(error) => Some(format!("Save layout: {error}")),
                };
                cx.notify();
            });
        })
        .detach();
    }

    fn select(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if index >= self.workspace_state.entries.len() {
            return;
        }
        self.record_terminal_focus(window, cx);
        self.rename = None;
        self.remember_focus(window, cx);
        self.activate_workspace(index);
        self.scroll_to_workspace(index);
        let entry = &self.workspace_state.entries[index];
        window.set_window_title(&format!("{} — rmux", entry.name));
        let workspace = entry.workspace.read(cx);
        if let Some(pane) = workspace
            .panes
            .iter()
            .find(|pane| Some(pane.entity_id()) == entry.last_pane)
            .or_else(|| workspace.panes.first())
        {
            pane.read(cx).focus(window, cx);
            let terminal = pane.read(cx).active_terminal_id();
            if let Some(terminal) = terminal { self.record_terminal_visit(terminal); }
        }
        cx.notify();
    }

    fn add(&mut self, panes: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.add_with_group(panes, true, window, cx);
    }

    fn add_with_group(&mut self, panes: usize, inherit_group: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.add_with_placement(panes, inherit_group, None, window, cx);
    }

    fn add_with_placement(&mut self, panes: usize, inherit_group: bool, placement: Option<workspace_groups::Placement>, window: &mut Window, cx: &mut Context<Self>) {
        self.add_with_directory(panes, inherit_group, placement, None, window, cx);
    }

    fn cancel_workspace_launch(&mut self, cx: &mut Context<Self>) {
        if let Some(cancel) = &self.workspace_launch_cancel {
            cancel.0.store(true, std::sync::atomic::Ordering::Release);
            self.notice = Some("Canceling workspace startup…".into());
            cx.notify();
        }
    }

    fn start_workspace(&mut self, directory: Option<std::path::PathBuf>, window: &mut Window, cx: &mut Context<Self>) {
        if self.command_launching || self.automation_available(cx).is_err() || self.workspace_state.entries.len() >= 64 { return; }
        let Some(entry) = self.workspace_state.entries.get(self.workspace_state.active) else { return; };
        let origin = entry.workspace.entity_id();
        let group = self.workspace_state.groups.containing(origin);
        let group_id = group.map(|group| group.id);
        let placement = if group.is_some() { cx.global::<Preferences>().group_new_workspace_placement }
            else { cx.global::<Preferences>().new_workspace_placement };
        let directory_entry = group.and_then(|group| group.members.first())
            .and_then(|id| self.workspace_state.entries.iter().find(|entry| entry.workspace.entity_id() == *id)).unwrap_or(entry);
        let directory = directory.or_else(|| group.and_then(|group| group.cwd.clone())).or_else(|| {
            let workspace = directory_entry.workspace.read(cx);
            workspace.panes.iter().find(|pane| Some(pane.entity_id()) == directory_entry.last_pane)
                .or_else(|| workspace.panes.first()).and_then(|pane| pane.read(cx).cached_directory(cx))
        }).or_else(|| cx.global::<InitialDirectory>().0.clone());
        let focus = window.focused(cx);
        let settings = cx.global::<Preferences>();
        let scrollback = settings.scrollback_bytes();
        let colors = settings.terminal_colors();
        let palette = settings.effective_palette();
        let executor = cx.background_executor().clone();
        let cancellation = file_browser::Cancellation::default();
        let canceled = cancellation.0.clone();
        let launch_canceled = canceled.clone();
        self.workspace_launch_cancel = Some(cancellation);
        self.command_launching = true;
        self.notice = Some("Starting workspace…".into());
        cx.notify();
        cx.spawn_in(window, async move |entity, cx| {
            let mut result = executor.spawn(async move {
                pane_view::prepare_terminal_launch(&launch_canceled, || {
                    shell_history::shell(80, 24, directory.as_deref(), scrollback, None)
                }, colors, palette)
            }).await;
            loop {
                if canceled.load(std::sync::atomic::Ordering::Acquire) { break; }
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
                let was_canceled = canceled.load(std::sync::atomic::Ordering::Acquire);
                this.workspace_launch_cancel = None;
                this.command_launching = false;
                if matches!(this.notice.as_deref(), Some("Starting workspace…" | "Canceling workspace startup…")) { this.notice = None; }
                if was_canceled { cx.notify(); return; }
                match result {
                    Ok(prepared) if this.automation_available(cx).is_ok() && this.workspace_state.entries.len() < 64
                        && this.workspace_state.entries.iter().any(|entry| entry.workspace.entity_id() == origin)
                        && this.workspace_state.groups.containing(origin).map(|group| group.id) == group_id => {
                        let focus = focus.as_ref().is_some_and(|focus| focus.is_focused(window));
                        let settings = cx.global::<Preferences>();
                        let seed = TerminalSeed::Ready(prepared, CellMetrics::new(settings.font_size), settings.font_family.clone().into());
                        this.add_configured_from(1, true, Some(placement), None, focus, Some((seed, None)), Some(origin), window, cx);
                    }
                    Ok(_) => this.notice = Some("Workspace startup canceled because its origin changed or became unavailable".into()),
                    Err(error) => this.notice = Some(format!("Could not start workspace: {error:#}")),
                }
                cx.notify();
            }));
        }).detach();
    }

    fn add_with_directory(&mut self, panes: usize, inherit_group: bool, placement: Option<workspace_groups::Placement>, directory_override: Option<std::path::PathBuf>, window: &mut Window, cx: &mut Context<Self>) {
        self.add_configured(panes, inherit_group, placement, directory_override, true, None, window, cx);
    }

    fn add_configured(&mut self, panes: usize, inherit_group: bool, placement: Option<workspace_groups::Placement>, directory_override: Option<std::path::PathBuf>, focus: bool, prepared: Option<(TerminalSeed, Option<String>)>, window: &mut Window, cx: &mut Context<Self>) -> Option<usize> {
        self.add_configured_from(panes, inherit_group, placement, directory_override, focus, prepared, None, window, cx)
    }
    fn add_configured_from(&mut self, panes: usize, inherit_group: bool, placement: Option<workspace_groups::Placement>, directory_override: Option<std::path::PathBuf>, focus: bool, mut prepared: Option<(TerminalSeed, Option<String>)>, origin: Option<EntityId>, window: &mut Window, cx: &mut Context<Self>) -> Option<usize> {
        if cx.global::<Shutdowns>().quitting {
            return None;
        }
        let origin_index = match origin {
            Some(id) => self.workspace_state.entries.iter().position(|entry| entry.workspace.entity_id() == id)?,
            None => self.workspace_state.active,
        };
        if focus { self.remember_focus(window, cx); }
        let parent_group = self.workspace_state.entries.get(origin_index).filter(|_| inherit_group).and_then(|entry| {
            let member = entry.workspace.entity_id();
            self.workspace_state.groups.containing(member).map(|group| (group.id, member))
        });
        let directory_entry = parent_group
            .and_then(|(_, member)| self.workspace_state.groups.containing(member))
            .and_then(|group| group.members.first())
            .and_then(|anchor| self.workspace_state.entries.iter().find(|entry| entry.workspace.entity_id() == *anchor))
            .or_else(|| self.workspace_state.entries.get(origin_index));
        let directory = directory_override.or_else(|| parent_group.and_then(|(_, member)| self.workspace_state.groups.containing(member)).and_then(|group| group.cwd.clone())).or_else(|| directory_entry.and_then(|entry| {
            let workspace = entry.workspace.read(cx);
            workspace.panes.iter().find(|pane| Some(pane.entity_id()) == entry.last_pane)
                .or_else(|| workspace.panes.first())
                .and_then(|pane| pane.read(cx).cached_directory(cx))
        }));
        self.rename = None;
        let workspace = cx.new(|cx| {
            let mut workspace = Workspace {
                    event_lifetime: cx.global::<events::Journal>().workspace(cx.entity_id()),
                spawning_split: None,
                split_launch_canceled: None,
                pane_subscriptions: Default::default(),
                split_drag: None,
                panes: Vec::new(),
                pane_state: Default::default(),
                error: None,
                benchmark: self.benchmark,
            };
            for _ in 0..panes {
                workspace.add_seed_at(None, SplitAxis::Columns, false, directory.clone(), focus, prepared.take(), window, cx);
            }
            workspace
        });
        let closed = Self::observe_workspace(&workspace, window, cx);
        let metadata = Self::observe_metadata(&workspace, window, cx);
        let last_pane = workspace.read(cx).panes.last().map(Entity::entity_id);
        let origin = self.workspace_state.entries.get(origin_index).map(|entry| entry.workspace.entity_id());
        let placement = placement.unwrap_or(if parent_group.is_some() {
            cx.global::<Preferences>().group_new_workspace_placement
        } else { cx.global::<Preferences>().new_workspace_placement });
        let entry = Entry {
            workspace,
            name: format!("Workspace {}", self.next_number),
            description: String::new(),
            project_directory: None,
            color: None,
            icon: None,
            pinned: false,
            last_pane,
            _closed: closed,
            _metadata: metadata,
        };
        let index = match self.workspace_state.insert(entry,
            rmux_core::workspace_lifecycle::Insertion { origin, inherit_group, placement },
            |entry| (entry.workspace.entity_id(), entry.pinned),
        ) {
            Ok(index) => index,
            Err(error) => { self.notice = Some(format!("Add workspace: {error}")); cx.notify(); return None; }
        };
        self.next_number += 1;
        if focus {
        self.activate_workspace(index);
        self.scroll_to_workspace(self.workspace_state.active);
        window.set_window_title(&format!("{} — rmux", self.workspace_state.entries[self.workspace_state.active].name));
        }
        cx.notify();
        Some(index)
    }

    fn observe_workspace(
        workspace: &Entity<Workspace>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Subscription {
        cx.subscribe_in(
            &workspace,
            window,
            |this, workspace, _: &WorkspaceEmpty, window, cx| {
                if let Some(index) = this
                    .workspace_state.entries
                    .iter()
                    .position(|entry| &entry.workspace == workspace)
                {
                    this.close_workspace(index, window, cx);
                }
            },
        )
    }

    fn edit_metadata(&mut self, field: MetadataField, window: &mut Window, cx: &mut Context<Self>) {
        if let Some((id, input, _)) = self.rename.clone() {
            self.workspace_state.groups.reveal(id);
            self.sidebar = true;
            self.scroll_to_workspace(self.workspace_state.active);
            input.read(cx).focus.focus(window);
            cx.notify();
            return;
        }
        let Some(entry) = self.workspace_state.entries.get(self.workspace_state.active) else {
            return;
        };
        let id = entry.workspace.entity_id();
        let name = match field {
            MetadataField::GroupColor => {
                let Some(group) = self.workspace_state.groups.containing(id) else { return; };
                group.color.map(|color| format!("#{color:06X}")).unwrap_or_default()
            }
            MetadataField::GroupIcon => {
                let Some(group) = self.workspace_state.groups.containing(id) else { return; };
                group.icon.clone().unwrap_or_default()
            }
            MetadataField::GroupName => {
                let Some(group) = self.workspace_state.groups.containing(id) else { return; };
                group.name.clone()
            }
            MetadataField::Name => entry.name.clone(),
            MetadataField::Description => entry.description.clone(),
            MetadataField::Icon => entry.icon.clone().unwrap_or_default(),
            MetadataField::Color => entry
                .color
                .map(|color| format!("#{color:06X}"))
                .unwrap_or_default(),
        };
        self.remember_focus(window, cx);
        let input = cx.new(|cx| text_field::TextField::new(&name, cx));
        cx.subscribe_in(
            &input,
            window,
            move |this, _, event: &text_field::EditEvent, window, cx| {
                match event {
                    text_field::EditEvent::Submitted { text, .. } => {
                        let text = text.trim();
                        if matches!(field, MetadataField::Icon | MetadataField::GroupIcon) && !text.is_empty() && !snapshot::valid_workspace_icon(text) {
                            this.notice = Some("Use one symbol or emoji (up to 64 bytes), or leave it empty to clear.".into());
                            cx.notify();
                            return;
                        }
                        let color = if matches!(field, MetadataField::Color | MetadataField::GroupColor) {
                            match parse_workspace_color(text) {
                                Ok(color) => color,
                                Err(error) => { this.notice = Some(error.into()); cx.notify(); return; }
                            }
                        } else { None };
                        if matches!(field, MetadataField::GroupName | MetadataField::GroupColor | MetadataField::GroupIcon) {
                            if let Some(group_id) = this.workspace_state.groups.containing(id).map(|group| group.id) {
                                let group = this.workspace_state.groups.get_mut(group_id).unwrap();
                                match field {
                                    MetadataField::GroupName => if let Some(name) = normalized_workspace_name(text) { group.name = name; },
                                    MetadataField::GroupColor => group.color = color,
                                    MetadataField::GroupIcon => group.icon = (!text.is_empty()).then(|| text.to_owned()),
                                    _ => unreachable!(),
                                }
                            }
                        }
                        if let Some(entry) = this.workspace_state.entries.iter_mut().find(|entry| entry.workspace.entity_id() == id) {
                            match field {
                                MetadataField::GroupName | MetadataField::GroupColor | MetadataField::GroupIcon => {}
                                MetadataField::Icon => entry.icon = (!text.is_empty()).then(|| text.to_owned()),
                                MetadataField::Color => entry.color = color,
                                MetadataField::Description => entry.description = rmux_core::workspace_lifecycle::normalized_description(text),
                                MetadataField::Name => {
                                    if let Some(name) = normalized_workspace_name(text) { entry.name = name; }
                                }
                            }
                        }
                        this.notice = None;
                    }
                    text_field::EditEvent::Cancelled => {}
                    text_field::EditEvent::Changed(_) => { cx.notify(); return; },
                }
                this.rename = None;
                this.select(this.workspace_state.active, window, cx);
            },
        )
        .detach();
        input.read(cx).focus.focus(window);
        self.rename = Some((id, input, field));
        self.workspace_state.groups.reveal(id);
        self.sidebar = true;
        self.scroll_to_workspace(self.workspace_state.active);
        cx.notify();
    }

    fn cycle(&mut self, delta: isize, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(index) = self.workspace_state.navigation_target(rmux_core::workspace_lifecycle::Navigation::Cycle(delta), |entry| (entry.workspace.entity_id(), entry.pinned)) {
            self.select(index, window, cx);
        }
    }

    fn reorder(&mut self, delta: isize, cx: &mut Context<Self>) {
        if self.workspace_state.reorder(delta, |entry| (entry.workspace.entity_id(), entry.pinned)) {
            self.scroll_to_workspace(self.workspace_state.active);
            cx.notify();
        }
    }

    fn accepts_tab_drop(&self, item: &drag_item::DragItem, target: EntityId, cx: &App) -> bool {
        item.scope != target
            && self.workspace_state.entries.iter().any(|entry| {
                entry.workspace.entity_id() == item.scope
                    && entry.workspace.read(cx).panes.iter().any(|pane| {
                        pane.entity_id() == item.container
                            && pane
                                .read(cx)
                                .terminals
                                .iter()
                                .any(|terminal| terminal.entity_id() == item.item)
                    })
            })
    }
    fn drop_workspace(
        &mut self,
        item: &drag_item::DragItem,
        target: EntityId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if item.container != cx.entity_id() {
            if self.accepts_tab_drop(item, target, cx) {
                self.transfer_tab(item.scope, item.container, item.item, target, window, cx);
            }
            return;
        }
        self.remember_focus(window, cx);
        match self.workspace_state.drop_workspace(item.item, target, |entry| (entry.workspace.entity_id(), entry.pinned)) {
            Ok(Some(index)) => self.select(index, window, cx),
            Ok(None) => {},
            Err(error) => { self.notice = Some(format!("Move workspace: {error}")); cx.notify(); }
        }
    }
}

impl crate::workspace_actions::WorkspaceCommandHost for WorkspaceWindow {
    fn move_workspace_to_group(&mut self, group: u64, cx: &mut Context<Self>) { WorkspaceWindow::move_workspace_to_group(self, group, cx); }
    fn run_workspace_command(&mut self, command: rmux_core::commands::CommandId, window: &mut Window, cx: &mut Context<Self>) {
        use rmux_core::commands::CommandId;
        match command {
            CommandId::CloseWorkspace => self.request_close_workspace(self.workspace_state.active, window, cx),
            CommandId::ShowWorkspaceMenu => {
                if let Some(entry) = self.workspace_state.entries.get(self.workspace_state.active) {
                    let target = entry.workspace.entity_id();
                    let bounds = self.workspace_scroll.bounds();
                    self.show_workspace_menu(target, None, point(bounds.left(), bounds.top()), window, cx);
                }
            }
            CommandId::EditWorkspaceIcon => self.edit_metadata(MetadataField::Icon, window, cx),
            CommandId::EditGroupIcon => self.edit_metadata(MetadataField::GroupIcon, window, cx),
            CommandId::EditWorkspaceColor => self.edit_metadata(MetadataField::Color, window, cx),
            CommandId::EditGroupColor => self.edit_metadata(MetadataField::GroupColor, window, cx),
            CommandId::EditWorkspaceDescription => self.edit_metadata(MetadataField::Description, window, cx),
            CommandId::RenameWorkspace => self.edit_metadata(MetadataField::Name, window, cx),
            CommandId::CloseWorkspaceGroup => self.request_close_group(window, cx),
            CommandId::ToggleWorkspacePin => self.toggle_workspace_pin(cx),
            CommandId::RenameWorkspaceGroup => self.edit_metadata(MetadataField::GroupName, window, cx),
            CommandId::NewWorkspace => self.start_workspace(None, window, cx),
            CommandId::NewWorkspaceGroup => self.create_group(true, window, cx),
            CommandId::GroupCurrentWorkspace => self.create_group(false, window, cx),
            CommandId::GroupSelectedWorkspaces => self.group_selected(window, cx),
            CommandId::ToggleGroupPin => self.toggle_group_pin(cx),
            CommandId::RemoveWorkspaceFromGroup => self.remove_workspace_from_group(cx),
            CommandId::SetGroupAnchor => self.set_group_anchor(cx),
            CommandId::MoveWorkspaceUp => self.reorder(-1, cx),
            CommandId::MoveWorkspaceDown => self.reorder(1, cx),
            CommandId::UngroupWorkspaces => self.ungroup_active(cx),
            CommandId::NextWorkspace => self.cycle(1, window, cx),
            CommandId::PreviousWorkspace => self.cycle(-1, window, cx),
            CommandId::LastWorkspace => self.last_workspace(window, cx),
            CommandId::ToggleSidebar => {
                if self.rename.is_some() { self.select(self.workspace_state.active, window, cx); }
                self.sidebar = !self.sidebar;
                self.sidebar_drag = None;
                cx.notify();
            }
            _ => {}
        }
    }
    fn select_workspace_number(&mut self, number: usize, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(index) = self.workspace_state.navigation_target(rmux_core::workspace_lifecycle::Navigation::Number(number), |entry| (entry.workspace.entity_id(), entry.pinned)) {
            self.select(index, window, cx);
        }
    }
}

impl Render for WorkspaceWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let background = cx.global::<Preferences>().window_background();
        if self.rendered_background != Some(background) {
            window.set_background_appearance(background);
            self.rendered_background = Some(background);
        }
        // Sidebar-only summaries must not scan hidden terminals when the sidebar
        // is closed. A visible sidebar reads each terminal's cached metadata once.
        let shortcut_focus = self.workspace_state.entries.get(self.workspace_state.active).and_then(|entry| {
            let workspace = entry.workspace.read(cx);
            let pane = workspace.panes.iter().find(|pane| Some(pane.entity_id()) == entry.last_pane)
                .or_else(|| workspace.panes.first())?.read(cx);
            Some(pane.terminals.active()?.read(cx).focus.clone())
        });
        let sidebar_shortcuts: [Option<String>; 9] = std::array::from_fn(|index| {
            if !self.sidebar { return None; }
            shortcut_focus.as_ref().and_then(|focus| shortcuts::hint(&SelectWorkspaceNumber { number: index + 1 }, focus, window))
        });
        let mut navigation_positions = Vec::new();
        let mut unread_workspaces = std::collections::HashSet::new();
        let mut notification_previews = std::collections::HashMap::new();
        if self.sidebar {
            navigation_positions.resize(self.workspace_state.entries.len(), 0);
            for (position, index) in self.workspace_order(true).into_iter().enumerate() {
                navigation_positions[index] = position;
            }
            for entry in &self.workspace_state.entries {
                let mut unread = false;
                let mut preview = None;
                for pane in &entry.workspace.read(cx).panes {
                    for terminal in &pane.read(cx).terminals {
                        let terminal = terminal.read(cx);
                        unread |= terminal.attention.any();
                        if let Some(candidate) = terminal.notifications.unread_preview()
                            && preview.as_ref().is_none_or(|(arrival, _)| candidate.0 > *arrival) {
                            preview = Some(candidate);
                        }
                    }
                }
                if unread { unread_workspaces.insert(entry.workspace.entity_id()); }
                if let Some((arrival, message)) = preview {
                    notification_previews.insert(entry.workspace.entity_id(), (arrival, message.clone()));
                }
            }
        }
        let current = self
            .workspace_state.entries
            .get(self.workspace_state.active)
            .map(|entry| entry.workspace.clone());
        if let Some(previous) = self
            .rendered_workspace
            .take()
            .and_then(|weak| weak.upgrade())
        {
            if current.as_ref().map(Entity::entity_id) != Some(previous.entity_id()) {
                previous.update(cx, |workspace, cx| workspace.release_render_caches(cx));
            }
        }
        self.rendered_workspace = current.as_ref().map(Entity::downgrade);
        if !self.close_handler_installed {
            native_notifications::record_activation(window, cx);
            cx.observe_window_activation(window, |_, window, cx| native_notifications::record_activation(window, cx)).detach();
            cx.observe_pending_input(window, |_, _, cx| cx.notify()).detach();
            let entity = cx.weak_entity();
            window.on_window_should_close(cx, move |window, cx| {
                entity
                    .update(cx, |this, cx| {
                        if cx.global::<Shutdowns>().quitting || !this.should_confirm_close(cx) {
                            this.request_process_close(cx);
                            true
                        } else {
                            this.request_close_window(window, cx);
                            false
                        }
                    })
                    .unwrap_or(true)
            });
            self.close_handler_installed = true;
        }

        let sidebar_width = crate::sidebar_interaction::visible_width(self.sidebar_width, window);
        let overlay_inspector = chrome_style::overlay_inspector(f32::from(window.viewport_size().width), if self.sidebar { sidebar_width } else { 0. });
        div()
            .size_full()
            .relative()
            .flex()
            .when(background == WindowBackgroundAppearance::Opaque, |root| root.bg(rgb(0x12141a)))
            .text_color(rgb(0xdcdfe6))
            .child(crate::sidebar_interaction::drag_capture(cx))
            .on_action(cx.listener(|this, _: &CloseWindow, window, cx| this.request_close_window(window, cx)))
            .on_action(cx.listener(|this, _: &ShowBufferPicker, window, cx| this.show_buffer_picker(command_palette::BufferMode::Paste, window, cx)))
            .on_action(cx.listener(|this, _: &ShowDeleteBufferPicker, window, cx| this.show_buffer_picker(command_palette::BufferMode::Delete, window, cx)))
            .on_action(cx.listener(|this, _: &ShowRenameBufferPicker, window, cx| this.show_buffer_picker(command_palette::BufferMode::Rename, window, cx)))
            .on_action(cx.listener(|this, _: &ShowCopyBufferPicker, window, cx| this.show_buffer_picker(command_palette::BufferMode::Copy, window, cx)))
            .on_action(cx.listener(|this, action: &CopyNamedBuffer, _, cx| {
                let result = cx.global::<copy_buffers::Store>().text_at_revision(&action.name, action.revision)
                    .map(str::to_owned);
                this.notice = Some(match result {
                    Ok(text) => {
                        clipboard::write(ClipboardItem::new_string(text), cx);
                        format!("Copied buffer {}", action.name)
                    }
                    Err(error) => error.into(),
                });
                cx.notify();
            }))
            .on_action(cx.listener(|this, action: &DeleteNamedBuffer, _, cx| {
                this.notice = Some(match cx.global_mut::<copy_buffers::Store>().delete_if_unchanged(&action.name, action.revision) {
                    Ok(_) => format!("Deleted buffer {}", action.name),
                    Err(error) => error.into(),
                });
                cx.notify();
            }))
            .on_action(cx.listener(|this, action: &PasteNamedBuffer, _, cx| {
                let result = (|| {
                    let text = cx.global::<copy_buffers::Store>().text_at_revision(&action.name, action.revision)
                        .map_err(|message| automation::Failure { code: -32008, message })?.to_owned();
                    let target = automation::target_id(&action.surface_id)?;
                    this.automation_send_text(&automation::SurfaceText { surface_id: action.surface_id.clone(), text, paste: true }, target, cx)
                })();
                this.notice = Some(match result { Ok(_) => format!("Queued buffer {}", action.name), Err(error) => error.message.into() });
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &BrowseTerminalFiles, window, cx| {
                let directory = this.workspace_state.entries.get(this.workspace_state.active).and_then(|entry| {
                    let workspace = entry.workspace.read(cx);
                    workspace.focused_index(window, cx).and_then(|index| workspace.panes.get(index))
                        .and_then(|pane| pane.read(cx).cached_directory(cx))
                });
                if let Some(path) = directory { this.browse_directory(path, window, cx); }
                else { this.notice = Some("The current terminal has not reported a local directory yet.".into()); cx.notify(); }
            }))
            .on_action(cx.listener(|this, action: &BrowseDirectory, window, cx| this.browse_directory(action.path.clone(), window, cx)))
            .on_action(cx.listener(|this, action: &NewWorkspaceInDirectory, window, cx| {
                if this.automation_available(cx).is_err() { return; }
                if this.workspace_state.entries.len() >= 64 {
                    this.notice = Some("Window workspace limit reached".into()); cx.notify(); return;
                }
                this.start_workspace(Some(action.path.clone()), window, cx);
            }))
            .on_action(cx.listener(|this, action: &EditLocalFile, _, cx| this.edit_local_file(action.clone(), cx)))
            .on_action(cx.listener(|this, _: &PreviewBack, window, cx| this.preview_history(false, window, cx)))
            .on_action(cx.listener(|this, _: &PreviewForward, window, cx| this.preview_history(true, window, cx)))
            .on_action(cx.listener(|this, action: &PreviewMarkdownFile, window, cx| this.preview_file_kind(action.path.clone(), false, true, window, cx)))
            .on_action(cx.listener(|this, action: &PreviewHexFile, window, cx| this.preview_file_mode(action.path.clone(), true, window, cx)))
            .on_action(cx.listener(|this, action: &PreviewLocalFile, window, cx| this.preview_local_file(action.path.clone(), window, cx)))
            .on_action(cx.listener(|this, _: &OpenTextPreview, window, cx| this.choose_local_path(false, window, cx)))
            .on_action(cx.listener(|this, _: &ShowTerminalSearch, window, cx| {
                if this.automation_available(cx).is_err() || this.terminal_search_job.is_some() { return; }
                let focus = window.focused(cx);
                let palette = cx.new(|cx| command_palette::CommandPalette::terminal_search(&this.terminal_search_query, window, cx));
                this.install_palette(palette, focus, window, cx);
            }))
            .on_action(cx.listener(|this, _: &ShowSshConnection, window, cx| {
                if this.automation_available(cx).is_err() || this.command_launching { return; }
                let focus = window.focused(cx);
                let destination = this.workspace_state.entries.get(this.workspace_state.active).and_then(|entry| {
                    let workspace = entry.workspace.read(cx);
                    let pane = workspace.panes.iter().find(|pane| Some(pane.entity_id()) == entry.last_pane)
                        .or_else(|| workspace.panes.first())?.read(cx);
                    pane.ssh_destination(cx)
                }).unwrap_or_default();
                let palette = cx.new(|cx| command_palette::CommandPalette::ssh_connection(&destination, window, cx));
                this.install_palette(palette, focus, window, cx);
            }))
            .on_action(cx.listener(|this, _: &ShowSshHosts, window, cx| this.show_ssh_hosts(window, cx)))
            .on_action(cx.listener(|this, action: &ConnectSsh, window, cx| {
                match ssh::command(&action.destination) {
                    Ok(command) => this.launch_command(command, Some(action.destination.clone()), window, cx),
                    Err(error) => { this.notice = Some(error.into()); cx.notify(); }
                }
            }))
            .on_action(cx.listener(|this, action: &SearchTerminalOutput, window, cx| this.search_terminal_output(action.query.clone(), window, cx)))
            .on_action(cx.listener(|this, _: &FileBrowserBack, window, cx| this.browse_history(false, window, cx)))
            .on_action(cx.listener(|this, _: &FileBrowserForward, window, cx| this.browse_history(true, window, cx)))
            .on_action(cx.listener(|this, _: &OpenFolderBrowser, window, cx| this.choose_local_path(true, window, cx)))
            .on_action(cx.listener(|this, action: &RevealLocalPath, _, cx| {
                if this.automation_available(cx).is_ok() && action.path.is_absolute() {
                    if cfg!(target_os = "macos") && action.path.to_str().is_none() {
                        this.notice = Some("This path cannot be represented by the Finder integration.".into());
                        cx.notify();
                        return;
                    }
                    cx.reveal_path(&action.path);
                }
            }))
            .on_action(cx.listener(|this, action: &CopyLocalPath, _, cx| {
                this.notice = Some(match action.path.to_str() {
                    Some(path) => { clipboard::write(ClipboardItem::new_string(path.into()), cx); "Copied file path".into() }
                    None => "This path cannot be represented as UTF-8 text.".into(),
                });
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &ShowFontPicker, window, cx| this.show_font_picker(false, window, cx)))
            .on_action(cx.listener(|this, _: &ShowDefaultFontPicker, window, cx| this.show_font_picker(true, window, cx)))
            .on_action(cx.listener(|_, action: &SetDefaultFont, _, cx| {
                if !action.family.trim().is_empty() && action.family.len() <= 128 && !action.family.chars().any(char::is_control) {
                    let mut settings = cx.global::<Preferences>().clone();
                    if settings.font_family != action.family {
                        settings.font_family = action.family.clone();
                        cx.set_global(settings);
                    }
                }
            }))
            .on_action(cx.listener(|this, _: &ShowCommandPalette, window, cx| this.show_palette(window, cx)))
            .on_action(cx.listener(|this, _: &ShowKeyboardShortcuts, window, cx| this.show_keyboard_shortcuts(window, cx)))
            .on_action(cx.listener(|this, _: &SaveLayout, window, cx| this.save_layout(window, cx)))
            .on_action(cx.listener(|this, _: &SaveSession, window, cx| this.save_session(window, cx)))
            .on_action(cx.listener(|this, _: &OpenLayout, _, cx| this.open_layout(cx)))
            .on_action(cx.listener(|this, _: &OpenSession, _, cx| this.open_snapshot(true, None, None, cx)))
            .on_action(cx.listener(|this, action: &OpenLayoutPreset, _, cx| {
                if let Some(preset) = cx.global::<Preferences>().layout_presets.iter().find(|preset| preset.name == action.name).cloned() {
                    this.open_snapshot(preset.session, Some(preset.path), preset.cwd, cx);
                } else { this.notice = Some("Layout preset is no longer configured".into()); cx.notify(); }
            }))
            .on_action(cx.listener(|this, _: &CancelRestore, _, cx| this.cancel_restore(cx)))
            .on_action(cx.listener(|this, _: &CancelFileLoading, _, cx| this.cancel_file_loading(cx)))
            .on_action(cx.listener(|this, _: &ShowCustomCommandPicker, window, cx| this.show_custom_command_picker(window, cx)))
            .on_action(cx.listener(|this, _: &ShowLayoutPresetPicker, window, cx| this.show_layout_preset_picker(window, cx)))
            .on_action(cx.listener(|this, command: &RunCustomCommand, window, cx| this.run_custom_command(&command.name, window, cx)))
            .on_action(cx.listener(|this, _: &ToggleSettings, window, cx| this.toggle_settings(window, cx)))
            .on_action(cx.listener(|this, action: &SetTerminalTheme, window, cx| {
                if let Some(state) = &mut this.settings_state {
                    state.selected = settings_view::KEYS.iter().position(|key| *key == "theme").unwrap_or(0);
                    state.focus.focus(window);
                }
                if cx.global::<Preferences>().terminal_theme != action.theme || cx.global::<Preferences>().theme_file.is_some() {
                    let mut settings = cx.global::<Preferences>().clone();
                    settings.select_theme(action.theme);
                    cx.set_global(settings);
                }
                this.notice = None;
                cx.notify();
            }))
            .on_action(cx.listener(|this, selected: &SelectSetting, window, cx| {
                if let Some(state) = &mut this.settings_state {
                    if let Some(index) = settings_view::control_index(&selected.key) {
                        state.selected = index;
                        state.focus.focus(window);
                        cx.notify();
                    }
                }
            }))
            .on_action(cx.listener(|this, change: &AdjustSetting, window, cx| {
                if let Some(state) = &mut this.settings_state {
                    if let Some(index) = settings_view::KEYS.iter().position(|key| *key == change.key) { state.selected = index; }
                    state.focus.focus(window);
                }
                let mut settings = cx.global::<Preferences>().clone();
                if change.key == "file_editor" {
                    this.open_settings_file(cx);
                    return;
                }
                if change.key == "import_theme" {
                    this.choose_terminal_theme(window, cx);
                    return;
                }
                if change.key == "font_family" {
                    this.show_font_picker(true, window, cx);
                    return;
                }
                settings_view::adjust(&mut settings, &change.key, change.delta);
                if !settings.same_saved_appearance(cx.global::<Preferences>()) {
                    cx.set_global(settings.confirm_close);
                    cx.set_global(settings);
                }
                this.notice = None;
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &SaveAppearanceSettings, _, cx| this.save_appearance_settings(cx)))
            .on_action(cx.listener(|this, _: &ReloadSettings, _, cx| this.reload_settings(cx)))
            .on_action(cx.listener(|this, _: &RevertAppearanceSettings, _, cx| this.revert_appearance_settings(cx)))
            .on_action(cx.listener(|this, _: &UndoSettingsSave, _, cx| this.undo_settings_save(cx)))
            .on_action(cx.listener(|this, _: &OpenSettingsFile, _, cx| this.open_settings_file(cx)))
            .on_action(cx.listener(|this, _: &ImportTerminalTheme, window, cx| this.choose_terminal_theme(window, cx)))
            .on_action(cx.listener(|this, _: &CancelNewWorkspace, _, cx| this.cancel_workspace_launch(cx)))
            .on_action(cx.listener(|this, _: &RefreshTerminalMemory, window, cx| this.refresh_memory(window, cx)))
            .on_action(cx.listener(|this, _: &CycleTerminalMemorySort, _, cx| this.sort_memory(cx)))
            .on_action(cx.listener(|this, _: &ShowTerminalMemorySort, window, cx| this.show_memory_sort(window, cx)))
            .on_action(cx.listener(|this, action: &SetTerminalMemorySort, _, cx| this.set_memory_sort(action.sort, cx)))
            .on_action(cx.listener(|this, _: &CopyTerminalMemory, _, cx| {
                if let Some(rows) = &this.memory_rows { clipboard::write(ClipboardItem::new_string(memory_view::report(rows, this.memory_application, this.memory_sort, this.memory_sampled_at)), cx); }
            }))
            .on_action(cx.listener(|this, _: &CloseTerminalMemory, window, cx| this.close_memory(window, cx)))
            .on_action(cx.listener(|this, _: &UseCurrentDirectoryForGroup, window, cx| this.set_group_directory(false, window, cx)))
            .on_action(cx.listener(|this, _: &ResetGroupDirectory, window, cx| this.set_group_directory(true, window, cx)))
            .on_action(cx.listener(|this, _: &LastTerminal, window, cx| this.last_terminal(window, cx)))
            .on_action(cx.listener(|this, _: &FocusBack, window, cx| { this.navigate_focus(false, window, cx); }))
            .on_action(cx.listener(|this, _: &FocusForward, window, cx| { this.navigate_focus(true, window, cx); }))
            .on_action(cx.listener(|this, _: &DetachTab, window, cx| this.detach_tab(window, cx)))
            .on_action(cx.listener(|this, _: &DetachWorkspace, window, cx| this.detach_workspace(window, cx)))
            .on_action(cx.listener(|this, _: &ShowWorkspaceMergePicker, window, cx| this.show_workspace_merge_picker(window, cx)))
            .on_action(cx.listener(|this, _: &ShowWorkspaceWindowPicker, window, cx| this.show_workspace_window_picker(window, cx)))
            .on_action(cx.listener(|this, _: &MoveWorkspaceToNextWindow, window, cx| this.move_workspace_to_next_window(window, cx)))
            .on_action(cx.listener(|this, action: &SetWorkspaceColor, _, cx| this.set_workspace_color(action.color, cx)))
            .on_action(cx.listener(|this, action: &SetWorkspaceIcon, _, cx| this.set_workspace_icon(action.icon.clone(), cx)))
            .on_action(cx.listener(|this, _: &MoveTabNextWorkspace, window, cx| this.move_tab_next_workspace(window, cx)))
            .on_action(cx.listener(|this, _: &NextUnreadTerminal, window, cx| this.next_unread(window, cx)))
            .on_action(cx.listener(|this, _: &ToggleNotifications, window, cx| this.toggle_notifications(window, cx)))
            .on_action(cx.listener(|this, _: &DismissAllNotifications, _, cx| this.dismiss_all_notifications(cx)))
            .on_action(cx.listener(|this, _: &MarkAllRead, _, cx| this.mark_all_read(cx)))
            .map(|root| crate::workspace_actions::bind(root, cx))
            .when(self.sidebar, |div| {
                div.child(
                    crate::sidebar_presentation::sidebar(sidebar_width)
                        .child(
                            crate::sidebar_presentation::header()
                                .when(cfg!(target_os = "macos"), |header| header.pl(px(80.)))
                                 .child(gpui::div().id("sidebar-title-drag").flex_1().min_w_0().truncate().text_xs().text_color(rgb(0xa0a0a0))
                                    .on_mouse_down(MouseButton::Left, |event, window, _| { if cfg!(target_os = "macos") { if event.click_count == 2 { window.zoom_window(); } else { window_geometry::drag_titlebar(window); } } })
                                    .child(if self.workspace_state.sidebar_selection.len() > 1 { format!("{} selected", self.workspace_state.sidebar_selection.len()) } else { "Workspaces".into() }))
                                .child(pane_view::action_control("sidebar-new-workspace", "New Workspace", NewWorkspace, None)
                                    .on_click(cx.listener(|this, event: &ClickEvent, window, cx| { if event.standard_click() { this.start_workspace(None, window, cx); } })).child(icons::icon("plus")))
                                .tooltip(|window, cx| {
                                    let selection = if cfg!(target_os = "macos") { "⌘-click to select · Shift-click for range" }
                                        else { "Ctrl-click to select · Shift-click for range" };
                                    let group = window.focused(cx).and_then(|focus| shortcuts::hint(&GroupSelectedWorkspaces, &focus, window));
                                    let text = match group {
                                        Some(key) => format!("{selection} · {key} to group"),
                                        None => format!("{selection} · Group Selected Workspaces in the command palette"),
                                    };
                                    cx.new(|_| pane_view::ControlHint(text.into())).into()
                                }),
                        )
                        .child(
                            gpui::div()
                                .id("workspace-list")
                                .flex_1()
                                .min_h_0()
                                .overflow_y_scroll()
                                .track_scroll(&self.workspace_scroll)
                                .children(self.sidebar_order().into_iter().map(|index| {
                                    let position = navigation_positions[index];
                                    let entry = &self.workspace_state.entries[index];
                                    let group = self.workspace_state.groups.containing(entry.workspace.entity_id());
                                    let header = group.filter(|group| group.members.first() == Some(&entry.workspace.entity_id()));
                                    let unread_count = header.map(|group| group.members.iter().filter(|id| unread_workspaces.contains(id)).count())
                                        .unwrap_or_else(|| usize::from(unread_workspaces.contains(&entry.workspace.entity_id())));
                                    let group_summary = header.map(|group| {
                                        let count = group.members.len();
                                        let label = if count == 1 { "workspace" } else { "workspaces" };
                                        if unread_count == 0 { format!("{count} {label}") }
                                        else { format!("{count} {label} · {unread_count} unread") }
                                    });
                                    let display_name = header.map(|group| group.name.clone()).unwrap_or_else(|| entry.name.clone());
                                    // Expanded members show their own previews. Only a
                                    // collapsed header needs to summarize hidden members.
                                    let preview = header.filter(|group| group.collapsed).and_then(|group| group.members.iter()
                                        .filter_map(|member| notification_previews.get(member))
                                        .max_by_key(|(arrival, _)| *arrival))
                                        .or_else(|| notification_previews.get(&entry.workspace.entity_id()))
                                        .map(|(_, message)| message);
                                    let container = cx.entity_id();
                                    let target = entry.workspace.entity_id();
                                    let menu_group = header.map(|group| group.id);
                                    let selected = self.workspace_state.sidebar_selection.contains(&target) || index == self.workspace_state.active || header.is_some_and(|group| group.collapsed && self.workspace_state.entries.get(self.workspace_state.active).is_some_and(|entry| group.members.contains(&entry.workspace.entity_id())));
                                    let drop_owner = cx.weak_entity();
                                    let hover_owner = drop_owner.clone();
                                    crate::sidebar_presentation::row(("workspace", entry.workspace.entity_id()), selected)
                                        .when(group.is_some() && header.is_none(), |row| row.ml(px(20.)))
                                        .when(self.rename.is_none(), |row| row
                                            .on_drag(drag_item::DragItem { scope: container, container, item: target, label: entry.name.clone().into() }, |item, _, _, cx| cx.new(|_| item.clone()))
                                            .can_drop(move |item, _, cx| item.downcast_ref::<drag_item::DragItem>().is_some_and(|item| drop_owner.upgrade().is_some_and(|owner| {
                                                let owner = owner.read(cx);
                                                if item.container == container { owner.workspace_state.can_drop_workspace(item.item, target, |entry| (entry.workspace.entity_id(), entry.pinned)) }
                                                else { owner.accepts_tab_drop(item, target, cx) }
                                            })))
                                            .drag_over(move |style, item: &drag_item::DragItem, _, cx| {
                                                if item.container == container || hover_owner.upgrade().is_some_and(|owner| owner.read(cx).accepts_tab_drop(item, target, cx)) { style.bg(rgb(0x435a7b)) } else { style }
                                            })
                                            .on_drop(cx.listener(move |this, item: &drag_item::DragItem, window, cx| this.drop_workspace(item, target, window, cx))))
                                        .on_mouse_down(MouseButton::Right, cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                                            this.show_workspace_menu(target, menu_group, event.position, window, cx);
                                            cx.stop_propagation();
                                        }))
                                        .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
                                            if !event.standard_click() { return; }
                                            let Some(index) = this.workspace_state.entries.iter().position(|entry| entry.workspace.entity_id() == target) else { return; };
                                            if this.rename.is_none() {
                                                if let ClickEvent::Mouse(mouse) = event {
                                                    let modifiers = mouse.down.modifiers;
                                                    let toggle = if cfg!(target_os = "macos") { modifiers.platform } else { modifiers.control };
                                                    if toggle || modifiers.shift {
                                                        this.extend_sidebar_selection(target, modifiers.shift, cx);
                                                        return;
                                                    }
                                                }
                                                this.workspace_state.sidebar_selection.clear();
                                                this.workspace_state.sidebar_selection_anchor = Some(target);
                                            }
                                            if let Some((id, input, _)) = &this.rename
                                                && this.workspace_state.entries.get(index).is_some_and(|entry| {
                                                    entry.workspace.entity_id() == *id
                                                })
                                            {
                                                input.read(cx).focus.focus(window);
                                            } else {
                                                this.select(index, window, cx);
                                                if matches!(event, ClickEvent::Mouse(mouse) if mouse.down.click_count == 2) {
                                                    let field = if this.workspace_state.groups.containing(target).is_some_and(|group| group.members.first() == Some(&target)) { MetadataField::GroupName } else { MetadataField::Name };
                                                    this.edit_metadata(field, window, cx);
                                                }
                                            }
                                        }))
                                        .child(
                                            if let Some((_, input, field)) =
                                                self.rename.as_ref().filter(|(id, _, _)| {
                                                    *id == entry.workspace.entity_id()
                                                })
                                            {
                                                crate::metadata_presets::editor(input, field.label(),
                                                    match field {
                                                        MetadataField::Color | MetadataField::GroupColor => Some(false),
                                                        MetadataField::Icon | MetadataField::GroupIcon => Some(true),
                                                        _ => None,
                                                    }, cx)
                                                    .into_any_element()
                                            } else {
                                                gpui::div().w_full().min_w_0().flex().flex_col().gap_1()
                                                    .child(gpui::div().w_full().min_w_0().min_h(px(24.)).flex().items_center().gap_1()
                                                    .when_some(header, |row, group| {
                                                        let id = group.id;
                                                        row.child(sidebar_presentation::group_toggle(("group-collapse", target), group.collapsed)
                                                            .on_click(cx.listener(move |this, event: &ClickEvent, _, cx| {
                                                                if !event.standard_click() { return; }
                                                                cx.stop_propagation();
                                                                if let Some(group) = this.workspace_state.groups.get_mut(id) { group.collapsed = !group.collapsed; }
                                                                cx.notify();
                                                            })))
                                                    })
                                                    .when(unread_count > 0, |row| row.child(
                                                        gpui::div().min_w(px(16.)).h(px(16.)).px_1().flex_shrink_0().rounded_full()
                                                            .flex().items_center().justify_center().text_size(px(10.)).font_weight(FontWeight::SEMIBOLD)
                                                            .bg(rgb(crate::sidebar_presentation::ACCENT)).text_color(rgb(0x211d2c))
                                                            .child(unread_count.to_string())
                                                    ))
                                                        .child(gpui::div().flex_1().min_w_0().flex().items_center().gap_1()
                                                            .when_some(header.map(|group| group.color).unwrap_or(entry.color), |row, color| row.child(gpui::div().w(px(8.)).h(px(8.)).flex_shrink_0().rounded_sm().bg(rgb(color))))
                                                            .when_some(sidebar_presentation::workspace_icon(header.map(|group| group.icon.clone()).unwrap_or_else(|| entry.icon.clone()), header.is_some(), selected), |row, icon| row.child(icon))
                                                            .child(sidebar_presentation::title_frame(("workspace-name", target))
                                                                .tooltip({
                                                                    let root = cx.weak_entity();
                                                                    let fallback = display_name.clone();
                                                                    let is_group = header.is_some();
                                                                    move |_, cx| {
                                                                        let hint = if is_group { fallback.clone() } else {
                                                                            root.read_with(cx, |root, _| root.workspace_state.entries.iter()
                                                                                .find(|entry| entry.workspace.entity_id() == target)
                                                                                .map(|entry| entry.project_directory.as_ref().map_or_else(|| entry.name.clone(),
                                                                                    |path| format!("{}\nProject: {}", entry.name, path.display()))))
                                                                                .ok().flatten().unwrap_or_else(|| fallback.clone())
                                                                        };
                                                                        cx.new(|_| pane_view::ControlHint(hint.into())).into()
                                                                    }
                                                                }).child(display_name))
                                                            .when(header.map(|group| group.pinned).unwrap_or(entry.pinned), |row| row.child(
                                                                sidebar_presentation::pin(("workspace-pin", target), selected, header.is_some()))))
                                                    .when_some(header, |row, _| row.child(
                                                        sidebar_presentation::group_add(("group-add", target))
                                                            .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
                                                                if !event.standard_click() || this.automation_available(cx).is_err() { return; }
                                                                cx.stop_propagation();
                                                                if let Some(index) = this.workspace_state.entries.iter().position(|entry| entry.workspace.entity_id() == target) {
                                                                    this.select(index, window, cx);
                                                                    this.start_workspace(None, window, cx);
                                                                }
                                                            }))
                                                    ))
                                                    .when(position < 8 || position + 1 == self.workspace_state.entries.len(), |row| {
                                                        let number = if position < 8 { position + 1 } else { 9 };
                                                        row.when_some(sidebar_shortcuts[number - 1].clone(), |row, hint| row.child(
                                                            gpui::div().id(("workspace-shortcut", target)).flex_shrink_0().max_w(px(64.)).truncate().text_xs()
                                                                .text_color(rgb(if selected { crate::sidebar_presentation::SELECTED_DETAIL } else { 0x969696 }))
                                                                .tooltip({ let hint = hint.clone(); move |_, cx| cx.new(|_| pane_view::ControlHint(hint.clone().into())).into() })
                                                                .child(hint)))
                                                    }))
                                                        .when_some(group_summary, |column, summary| column.child(gpui::div().text_xs().text_color(rgb(if selected { crate::sidebar_presentation::SELECTED_DETAIL } else if unread_count > 0 { 0xd6d6d6 } else { 0xa0a0a0 })).truncate().child(summary)))
                                                        .when(!entry.description.is_empty(), |column| column.child(gpui::div().text_xs().text_color(rgb(if selected { crate::sidebar_presentation::SELECTED_DETAIL } else { 0xa0a0a0 })).truncate().child(entry.description.clone())))
                                                        .when_some(preview, |column, message| {
                                                            let text: SharedString = if message.title.is_empty() { message.body.clone() } else if message.body.is_empty() { message.title.clone() } else { format!("{} — {}", message.title, message.body) }.into();
                                                            let hint = text.clone();
                                                            column.child(gpui::div().id(("workspace-message", target)).text_size(px(12.)).line_clamp(1).text_ellipsis()
                                                                .text_color(rgb(if selected { crate::chrome_style::SELECTED_DETAIL } else { 0xb5b5b5 }))
                                                                .tooltip(move |_, cx| cx.new(|_| pane_view::ControlHint(hint.clone())).into()).child(text))
                                                        })

                                                        .when_some(Self::entry_directory(entry, window, cx), |column, (branch, directory)| {
                                                            let hint: SharedString = [branch.as_deref(), directory.as_deref()].into_iter().flatten().collect::<Vec<_>>().join("\n").into();
                                                            let has_branch = branch.is_some();
                                                            column.child(gpui::div().id(("workspace-context", target)).flex().items_center().gap_1().min_w_0().text_size(px(11.))
                                                                .text_color(rgb(if selected { crate::sidebar_presentation::SELECTED_DETAIL } else { 0x969696 }))
                                                                .tooltip(move |_, cx| cx.new(|_| pane_view::ControlHint(hint.clone())).into())
                                                                .when_some(branch, |row, branch| row
                                                                    .child(icons::icon("branch"))
                                                                    .child(gpui::div().min_w_0().max_w(relative(0.45)).truncate().child(branch)))
                                                                .when_some(directory, |row, directory| row
                                                                    .when(has_branch, |row| row.child(gpui::div().flex_shrink_0().px_1().text_color(rgb(0x74717c)).child("·")))
                                                                    .child(gpui::div().flex_1().min_w_0().truncate().child(sidebar_presentation::directory_label(&directory)))))
                                                        })
                                                    .into_any_element()
                                            },
                                        )
                                })),
                        ),
                ).child(crate::sidebar_interaction::divider(sidebar_width, cx))
            })
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .flex().flex_col()
                    .when_some(self.workspace_state.entries.get(self.workspace_state.active), |div, entry| {
                        div.child(
                            crate::workspace_presentation::toolbar()
                                .when(cfg!(target_os = "macos") && !self.sidebar, |header| header.pl(px(80.)))
                                .child(crate::workspace_presentation::sidebar_toggle(cfg!(target_os = "macos")))
                                .child(crate::workspace_presentation::title("workspace-title-drag", entry.name.clone())
                                    .on_mouse_down(MouseButton::Left, |event, window, _| { if cfg!(target_os = "macos") { if event.click_count == 2 { window.zoom_window(); } else { window_geometry::drag_titlebar(window); } } }))
                                .child(pane_view::pane_control("toolbar-notifications", "Toggle Notifications")
                                    .on_click(|event, window, cx| { if event.standard_click() { window.dispatch_action(Box::new(ToggleNotifications), cx); } })
                                    .child(icons::icon("bell")))
                        ).child(gpui::div().flex_1().min_h_0().child(entry.workspace.clone()))
                    })
                    .when_some(self.memory_rows.as_ref(), |div, rows| div.child(memory_view::panel(rows, self.memory_application, self.memory_sampled_at, self.memory_sort, self.memory_loading, self.memory_scroll.clone(), self.memory_focus.as_ref(), self.memory_selected, cx)))
                    .when_some(self.notice.clone(), |div, notice| div.child(gpui::div().flex().items_start().flex_shrink_0().min_w_0()
                        .gap_2().px_3().py_2().text_xs().bg(rgb(0x292929)).border_t_1().border_color(rgb(0x3b4557))
                        .child(gpui::div().id("window-notice-text").flex_1().min_w_0().max_h(px(96.)).overflow_scroll().whitespace_normal()
                            .text_color(rgb(0xd7deeb)).child(notice.clone()))
                        .child(pane_view::pane_control("copy-notice", "Copy message")
                            .on_click(move |event, _, cx| {
                                if event.standard_click() { clipboard::write(ClipboardItem::new_string(notice.clone()), cx); }
                            }).child(icons::icon("copy")))
                        .when(self.restore_job.is_none() && self.file_browser_job.is_none() && self.terminal_search_job.is_none() && self.workspace_launch_cancel.is_none(), |row| row.child(pane_view::pane_control("dismiss-notice", "Dismiss message")
                            .on_click(cx.listener(|this, event: &ClickEvent, _, cx| {
                                if event.standard_click() { this.notice = None; cx.notify(); }
                            })).child(icons::icon("close"))))
                        .when(self.workspace_launch_cancel.is_some(), |row| row.child(pane_view::pane_control("cancel-workspace-startup", "Cancel New Workspace")
                            .on_click(cx.listener(|this, event: &ClickEvent, _, cx| {
                                if event.standard_click() { this.cancel_workspace_launch(cx); }
                            })).child(icons::icon("close"))))
                        .when(self.terminal_search_job.is_some(), |row| row.child(pane_view::pane_control("cancel-terminal-search", "Cancel terminal search")
                            .on_click(cx.listener(|this, event: &ClickEvent, _, cx| {
                                if !event.standard_click() { return; }
                                this.terminal_search_cancel = None;
                                this.terminal_search_job = None;
                                if this.notice.as_deref().is_some_and(|notice| notice.starts_with("Searching terminal output…")) { this.notice = None; }
                                cx.notify();
                            })).child(icons::icon("close"))))
                        .when(self.file_browser_job.is_some(), |row| row.child(pane_view::pane_control("cancel-file-load", "Cancel file loading")
                            .on_click(cx.listener(|this, event: &ClickEvent, _, cx| {
                                if !event.standard_click() { return; }
                                this.cancel_file_loading(cx);
                            })).child(icons::icon("close"))))
                        .when(self.restore_job.is_some(), |row| row.child(gpui::div().id("cancel-restore").flex_shrink_0().px_2().py_1().rounded_sm().cursor_pointer()
                            .hover(|style| style.bg(rgb(0x353535)))
                            .on_click(cx.listener(|this, event: &ClickEvent, _, cx| {
                                if !event.standard_click() { return; }
                                this.cancel_restore(cx);
                            })).child("Cancel"))))),
            )
            .when_some(window.pending_input_keystrokes().filter(|keys| !keys.is_empty()), |root, keys| root.child(
                div().absolute().bottom(px(8.)).right(px(8.)).max_w(relative(0.8))
                    .px_3().py_2().rounded_md().border_1().border_color(rgb(0x465063))
                    .bg(rgb(0x292929)).text_color(rgb(0xdcdfe6)).text_size(px(12.)).shadow_md()
                    .child(format!("{}  ·  Waiting for next key…", keys.iter().map(shortcuts::key_label).collect::<Vec<_>>().join(" ")))
            ))
            .when_some(self.workspace_menu.clone(), |root, menu| root.child(self.workspace_menu_element(menu, window, cx)))
            .when_some(self.settings_state.clone(), |root, state| root.child(settings_view::panel(state, overlay_inspector, window, cx)))
            .when(self.notification_navigation.is_some(), |root| root.child(self.notification_panel(overlay_inspector, cx)))
            .when_some(self.palette.as_ref(), |root, (palette, _)| root.child(
                div().id("command-palette-overlay").occlude().absolute().inset_0().flex().justify_center().items_start()
                    .pt(px(command_palette::top_inset(f32::from(window.viewport_size().height))))
                    .bg(rgba(0x00000066))
                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| {
                        this.close_palette(window, cx);
                        cx.stop_propagation();
                    }))
                    .child(palette.clone())
            ))
    }
}

impl crate::sidebar_interaction::SidebarHost for WorkspaceWindow {
    fn sidebar_state(&mut self) -> (&mut f32, &mut Option<(Pixels, f32)>) {
        (&mut self.sidebar_width, &mut self.sidebar_drag)
    }
}
