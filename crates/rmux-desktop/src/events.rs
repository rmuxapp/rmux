//! Bounded metadata replay. No terminal output, subscriber locks or idle work.
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

const CAPACITY: usize = 512;
const MAX_PAGE: usize = 128;
const MAX_WAIT_MS: u16 = 3000;
const MAX_WAITERS: usize = 16;
const MAX_KINDS: usize = 16;
fn default_limit() -> usize { MAX_PAGE }
#[derive(Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Read { pub after: Option<String>, pub limit: usize, pub wait_ms: u16, pub surface_id: Option<String>, pub workspace_id: Option<String>, pub kinds: Vec<String> }
impl Default for Read {
    fn default() -> Self { Self { after: None, limit: default_limit(), wait_ms: 0, surface_id: None, workspace_id: None, kinds: Vec::new() } }
}
const KINDS: &[&str] = &["notification.received", "notification.read", "notification.all_read", "notification.dismissed", "notification.cleared",
    "surface.created", "surface.closed", "surface.exited", "surface.title_changed", "surface.directory_changed", "surface.status_changed", "surface.progress_changed", "surface.focus_changed", "surface.bell", "surface.location_invalidated", "workspace.created", "workspace.closed"];
pub fn capabilities() -> serde_json::Value {
    serde_json::json!({
        "version":1, "method":"events.read", "kinds":KINDS,
        "retention_records":CAPACITY, "max_page_records":MAX_PAGE,
        "default_page_records":default_limit(), "max_wait_ms":MAX_WAIT_MS,
        "max_pending_waits":MAX_WAITERS, "max_filter_kinds":MAX_KINDS,
        "filters":["surface_id", "workspace_id", "kinds"], "gap_reasons":["retention", "restart"],
        "cursor_scope":"process", "push_subscriptions":false,
    })
}
impl Read {
    fn matches(&self, event: &Event) -> bool {
        self.surface_id.as_ref().is_none_or(|id| Some(id) == event.surface_id.as_ref())
            && self.workspace_id.as_ref().is_none_or(|id| Some(id) == event.workspace_id.as_ref())
            && (self.kinds.is_empty() || self.kinds.iter().any(|kind| kind == event.kind))
    }
    pub fn valid(&self) -> bool {
        self.surface_id.as_ref().is_none_or(|id| super::automation::target_id(id).is_ok())
            && self.workspace_id.as_ref().is_none_or(|id| super::automation::target_id(id).is_ok())
            && self.kinds.len() <= MAX_KINDS && self.kinds.iter().enumerate().all(|(index, kind)| KINDS.contains(&kind.as_str()) && !self.kinds[..index].contains(kind))
            && (1..=MAX_PAGE).contains(&self.limit) && self.wait_ms <= MAX_WAIT_MS && self.after.as_ref().is_none_or(|cursor| parse_cursor(cursor).is_some())
    }
}
fn parse_cursor(cursor: &str) -> Option<(&str, u64)> {
    if cursor.len() > 96 { return None; }
    let (epoch, sequence) = cursor.split_once(':')?;
    if epoch.is_empty() || !epoch.bytes().all(|b| b.is_ascii_digit() || b == b'-') { return None; }
    let number = sequence.parse::<u64>().ok()?;
    (number.to_string() == sequence).then_some((epoch, number))
}
#[derive(Clone, Serialize)]
pub struct Event {
    pub cursor: String,
    pub kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub surface_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notification_id: Option<String>,
}
#[derive(Serialize)]
pub struct Batch {
    pub events: Vec<Event>,
    pub cursor: String,
    pub has_more: bool,
    pub gap: Option<&'static str>,
}
struct State { epoch: String, sequence: u64, events: VecDeque<(u64, Event)>, waiters: Vec<(u64, async_channel::Sender<()>, Read)>, next_waiter: u64 }
impl Default for State {
    fn default() -> Self {
        let time = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos();
        Self { epoch: format!("{}-{time}", std::process::id()), sequence: 0, events: VecDeque::new(), waiters: Vec::new(), next_waiter: 0 }
    }
}
impl State {
    fn cursor(&self, sequence: u64) -> String { format!("{}:{sequence}", self.epoch) }
    pub fn push(&mut self, kind: &'static str, surface: gpui::EntityId, notification: Option<u64>) {
        self.push_resource(kind, Some(surface), None, notification);
    }
    fn push_resource(&mut self, kind: &'static str, surface: Option<gpui::EntityId>, workspace: Option<gpui::EntityId>, notification: Option<u64>) {
        self.sequence += 1;
        if self.events.len() == CAPACITY { self.events.pop_front(); }
        let event = Event { cursor: self.cursor(self.sequence), kind,
            surface_id: surface.map(|id| id.as_u64().to_string()), workspace_id: workspace.map(|id| id.as_u64().to_string()), notification_id: notification.map(|id| id.to_string()) };
        for (_, wake, filter) in &self.waiters { if filter.matches(&event) { let _ = wake.try_send(()); } }
        self.events.push_back((self.sequence, event));
    }
    pub fn read(&self, request: &Read) -> Result<Batch, &'static str> {
        if !request.valid() { return Err("Expected valid cursor/surface/workspace/kinds, limit 1–128 and wait_ms 0–3000"); }
        let oldest = self.events.front().map_or(self.sequence + 1, |(sequence, _)| *sequence);
        let (after, gap) = match request.after.as_deref().and_then(parse_cursor) {
            None => (oldest - 1, None),
            Some((epoch, _)) if epoch != self.epoch => (oldest - 1, Some("restart")),
            Some((_, sequence)) if sequence > self.sequence => return Err("Event cursor is ahead of this process"),
            Some((_, sequence)) if sequence < oldest - 1 => (oldest - 1, Some("retention")),
            Some((_, sequence)) => (sequence, None),
        };
        let mut events = Vec::new();
        let mut last = after;
        for (sequence, event) in self.events.iter().filter(|(sequence, _)| *sequence > after) {
            // Advance past nonmatching records too; an empty filtered page must
            // not repeatedly rescan unrelated activity on the next request.
            last = *sequence;
            if request.matches(event) { events.push(event.clone()); }
            if events.len() == request.limit { break; }
        }
        Ok(Batch { cursor: self.cursor(last), has_more: last < self.sequence, events, gap })
    }
}
#[derive(Clone, Default)]
pub struct Journal(std::rc::Rc<std::cell::RefCell<State>>);
impl gpui::Global for Journal {}
impl Journal {
    pub fn read(&self, request: &Read) -> Result<Batch, &'static str> { self.0.borrow().read(request) }
    fn push(&self, kind: &'static str, surface: gpui::EntityId, notification: Option<u64>) {
        self.0.borrow_mut().push(kind, surface, notification);
    }
    pub fn subscribe(&self, filter: &Read) -> Result<Waiter, &'static str> {
        let mut state = self.0.borrow_mut();
        if state.waiters.len() >= MAX_WAITERS { return Err("Too many pending event reads"); }
        state.next_waiter += 1;
        let id = state.next_waiter;
        let (sender, receiver) = async_channel::bounded(1);
        state.waiters.push((id, sender, filter.clone()));
        Ok(Waiter { journal: self.clone(), id, receiver })
    }
    pub fn workspace(&self, workspace: gpui::EntityId) -> WorkspaceLifetime {
        self.0.borrow_mut().push_resource("workspace.created", None, Some(workspace), None);
        WorkspaceLifetime { journal: self.clone(), workspace, closed: std::cell::Cell::new(false) }
    }
    pub fn surface(&self, surface: gpui::EntityId) -> SurfaceLifetime {
        self.push("surface.created", surface, None);
        SurfaceLifetime { journal: self.clone(), surface, closed: std::cell::Cell::new(false) }
    }
}
/// Moves with workspace ownership; retained render entities do not delay close.
pub struct WorkspaceLifetime { journal: Journal, workspace: gpui::EntityId, closed: std::cell::Cell<bool> }
impl WorkspaceLifetime {
    pub fn close(&self) {
        if !self.closed.replace(true) {
            self.journal.0.borrow_mut().push_resource("workspace.closed", None, Some(self.workspace), None);
        }
    }
}
impl Drop for WorkspaceLifetime { fn drop(&mut self) { self.close(); } }
pub struct Waiter { journal: Journal, id: u64, pub receiver: async_channel::Receiver<()> }
impl Drop for Waiter {
    fn drop(&mut self) { self.journal.0.borrow_mut().waiters.retain(|(id, _, _)| *id != self.id); }
}
/// Follows terminal ownership, independently of retained GPUI render entities.
pub struct SurfaceLifetime { journal: Journal, surface: gpui::EntityId, closed: std::cell::Cell<bool> }
impl SurfaceLifetime {
    pub fn close(&self) {
        if !self.closed.replace(true) { self.journal.push("surface.closed", self.surface, None); }
    }
}
impl Drop for SurfaceLifetime { fn drop(&mut self) { self.close(); } }
pub fn publish(kind: &'static str, surface: gpui::EntityId, notification: Option<u64>, cx: &mut gpui::App) {
    cx.global_mut::<Journal>().push(kind, surface, notification);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn workspace_lifetime_filters_wake_and_close_only_once() {
        let journal = Journal::default();
        let id = gpui::EntityId::from(4294967297);
        let filter = Read { workspace_id: Some(id.as_u64().to_string()), ..Read::default() };
        let waiter = journal.subscribe(&filter).unwrap();
        let workspace = journal.workspace(id);
        assert!(waiter.receiver.try_recv().is_ok());
        journal.push("surface.title_changed", id, None);
        assert!(waiter.receiver.is_empty());
        workspace.close();
        workspace.close();
        drop(workspace);
        let batch = journal.read(&filter).unwrap();
        assert_eq!(batch.events.iter().map(|event| event.kind).collect::<Vec<_>>(), ["workspace.created", "workspace.closed"]);
        let wire = serde_json::to_value(&batch.events[0]).unwrap();
        assert!(wire.get("surface_id").is_none());
        assert_eq!(wire["workspace_id"], id.as_u64().to_string());
        assert!(journal.read(&Read { surface_id: filter.workspace_id.clone(), ..filter.clone() }).unwrap().events.is_empty());
        assert!(!Read { workspace_id: Some("invalid".into()), ..Read::default() }.valid());
    }
    #[test]
    fn filters_advance_over_skipped_records_and_only_wake_matching_waiters() {
        let journal = Journal::default();
        let first = gpui::EntityId::from(4294967297);
        let second = gpui::EntityId::from(4294967298);
        let mut read = Read { surface_id: Some(first.as_u64().to_string()), kinds: vec!["surface.closed".into()], limit: 1, ..Read::default() };
        read.after = Some(journal.read(&read).unwrap().cursor);
        let waiter = journal.subscribe(&read).unwrap();
        journal.push("surface.created", first, None);
        journal.push("surface.closed", second, None);
        assert!(waiter.receiver.is_empty());
        let skipped = journal.read(&read).unwrap();
        assert!(skipped.events.is_empty() && !skipped.has_more);
        assert_ne!(Some(&skipped.cursor), read.after.as_ref());
        read.after = Some(skipped.cursor);
        journal.push("surface.closed", first, None);
        journal.push("surface.exited", second, None);
        assert_eq!(waiter.receiver.len(), 1);
        let matched = journal.read(&read).unwrap();
        assert_eq!(matched.events.len(), 1);
        assert_eq!(matched.events[0].surface_id, Some(first.as_u64().to_string()));
        assert!(matched.has_more);
        read.after = Some(matched.cursor);
        assert!(journal.read(&read).unwrap().events.is_empty());
        assert!(!Read { kinds: vec!["unknown".into()], ..Read::default() }.valid());
        assert!(!Read { kinds: vec!["surface.closed".into(); 2], ..Read::default() }.valid());
    }
    #[test]
    fn event_waiters_are_bounded_coalesced_and_released_on_drop() {
        let journal = Journal::default();
        let mut waiters: Vec<_> = (0..16).map(|_| journal.subscribe(&Read::default()).unwrap()).collect();
        assert!(journal.subscribe(&Read::default()).is_err());
        for _ in 0..1024 { journal.push("surface.exited", gpui::EntityId::from(4294967297), None); }
        for waiter in &waiters { assert_eq!(waiter.receiver.len(), 1); }
        assert_eq!(journal.0.borrow().events.len(), CAPACITY);
        drop(waiters.pop());
        let replacement = journal.subscribe(&Read::default()).unwrap();
        assert!(replacement.receiver.is_empty());
        drop(replacement); drop(waiters);
        assert!(journal.0.borrow().waiters.is_empty());
        assert!(!Read { wait_ms: 3001, ..Read::default() }.valid());
    }
    #[test]
    fn surface_close_is_immediate_and_drop_does_not_duplicate_it() {
        let journal = Journal::default();
        let surface = journal.surface(gpui::EntityId::from(4294967297));
        surface.close();
        surface.close();
        assert_eq!(journal.read(&Read::default()).unwrap().events.iter().map(|event| event.kind).collect::<Vec<_>>(),
            ["surface.created", "surface.closed"]);
        drop(surface);
        assert_eq!(journal.read(&Read::default()).unwrap().events.len(), 2);
        let fallback = journal.surface(gpui::EntityId::from(4294967298));
        drop(fallback);
        assert_eq!(journal.read(&Read::default()).unwrap().events.last().unwrap().kind, "surface.closed");
    }
    #[test]
    fn replay_pages_report_retention_and_restart_without_duplicates() {
        let mut journal = State::default();
        let start = journal.read(&Read::default()).unwrap().cursor;
        for _ in 0..520 { journal.push("notification.received", gpui::EntityId::from(4294967297), Some(1)); }
        let first = journal.read(&Read { after: Some(start), limit: 128, wait_ms: 0, ..Read::default() }).unwrap();
        assert_eq!(first.gap, Some("retention"));
        assert_eq!(first.events.len(), 128);
        assert!(first.has_more);
        let mut cursor = first.cursor;
        let mut count = first.events.len();
        loop {
            let page = journal.read(&Read { after: Some(cursor.clone()), limit: 128, wait_ms: 0, ..Read::default() }).unwrap();
            assert_eq!(page.gap, None);
            assert!(page.events.iter().all(|event| event.cursor != cursor));
            count += page.events.len(); cursor = page.cursor;
            if !page.has_more { break; }
        }
        assert_eq!(count, CAPACITY);
        assert!(journal.read(&Read { after: Some(cursor.clone()), limit: 128, wait_ms: 0, ..Read::default() }).unwrap().events.is_empty());
        let mut restarted = State::default(); restarted.epoch = "0-0".into();
        assert_eq!(restarted.read(&Read { after: Some(cursor), limit: 128, wait_ms: 0, ..Read::default() }).unwrap().gap, Some("restart"));
        assert!(journal.read(&Read { after: Some(journal.cursor(521)), limit: 128, wait_ms: 0, ..Read::default() }).is_err());
        for invalid in ["junk", "1-1:-1", "1-1:01", "1-1:18446744073709551616"] {
            assert!(!Read { after: Some(invalid.into()), limit: 1, wait_ms: 0, ..Read::default() }.valid());
        }
        for limit in [0, 129, usize::MAX] { assert!(!Read { after: None, limit, wait_ms: 0, ..Read::default() }.valid()); }
    }
}
