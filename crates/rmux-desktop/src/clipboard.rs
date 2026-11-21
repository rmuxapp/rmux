//! Order asynchronous terminal copies against every clipboard write in rmux.
use gpui::{App, ClipboardItem};
use std::sync::{Arc, atomic::{AtomicBool, AtomicU64, Ordering}};
static GENERATION: AtomicU64 = AtomicU64::new(0);
pub fn begin() -> u64 { GENERATION.fetch_add(1, Ordering::Relaxed).wrapping_add(1) }
pub fn current(ticket: u64) -> bool { GENERATION.load(Ordering::Relaxed) == ticket }

/// Owned by the UI task, not its background export. Dropping even an unpolled
/// task cancels parser-lock retries without invalidating another pane's copy.
pub struct PendingCopy {
    ticket: u64,
    canceled: Arc<AtomicBool>,
}
impl PendingCopy {
    pub fn new() -> Self {
        Self { ticket: begin(), canceled: Arc::new(AtomicBool::new(false)) }
    }
    pub fn current(&self) -> bool { current(self.ticket) }
    pub fn checker(&self) -> impl Fn() -> bool + Send + 'static {
        let canceled = self.canceled.clone();
        let ticket = self.ticket;
        move || !canceled.load(Ordering::Relaxed) && current(ticket)
    }
}
impl Drop for PendingCopy {
    fn drop(&mut self) { self.canceled.store(true, Ordering::Relaxed); }
}
pub fn write(item: ClipboardItem, cx: &mut App) {
    begin();
    cx.write_to_clipboard(item);
}

#[cfg(test)]
mod tests {
    #[test]
    fn newer_request_invalidates_previous_completion() {
        let older = super::begin();
        assert!(super::current(older));
        let newer = super::begin();
        assert!(!super::current(older));
        assert!(super::current(newer));

        let pending = super::PendingCopy::new();
        let check = pending.checker();
        assert!(check());
        let unpolled_task = async move {
            std::future::pending::<()>().await;
            pending.current()
        };
        drop(unpolled_task);
        assert!(!check(), "abandoned UI task left its export running");

        let older = super::PendingCopy::new();
        let check_older = older.checker();
        let newer = super::PendingCopy::new();
        let check_newer = newer.checker();
        assert!(!check_older());
        drop(older);
        assert!(check_newer(), "canceling an older pane canceled the newer copy");
    }
}
