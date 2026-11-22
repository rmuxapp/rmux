//! Bounded metadata replay. No terminal output, subscriber locks or idle work.
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

const CAPACITY: usize = 512;
fn default_limit() -> usize { 128 }
#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Read { pub after: Option<String>, pub limit: usize }
impl Default for Read {
    fn default() -> Self { Self { after: None, limit: default_limit() } }
}
impl Read {
    pub fn valid(&self) -> bool {
        (1..=128).contains(&self.limit) && self.after.as_ref().is_none_or(|cursor| parse_cursor(cursor).is_some())
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
    pub surface_id: String,
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
pub struct Journal { epoch: String, sequence: u64, events: VecDeque<(u64, Event)> }
impl gpui::Global for Journal {}
impl Default for Journal {
    fn default() -> Self {
        let time = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos();
        Self { epoch: format!("{}-{time}", std::process::id()), sequence: 0, events: VecDeque::new() }
    }
}
impl Journal {
    fn cursor(&self, sequence: u64) -> String { format!("{}:{sequence}", self.epoch) }
    pub fn push(&mut self, kind: &'static str, surface: gpui::EntityId, notification: Option<u64>) {
        self.sequence += 1;
        if self.events.len() == CAPACITY { self.events.pop_front(); }
        self.events.push_back((self.sequence, Event { cursor: self.cursor(self.sequence), kind,
            surface_id: surface.as_u64().to_string(), notification_id: notification.map(|id| id.to_string()) }));
    }
    pub fn read(&self, request: &Read) -> Result<Batch, &'static str> {
        if !request.valid() { return Err("Expected after cursor and limit between 1 and 128"); }
        let oldest = self.events.front().map_or(self.sequence + 1, |(sequence, _)| *sequence);
        let (after, gap) = match request.after.as_deref().and_then(parse_cursor) {
            None => (oldest - 1, None),
            Some((epoch, _)) if epoch != self.epoch => (oldest - 1, Some("restart")),
            Some((_, sequence)) if sequence > self.sequence => return Err("Event cursor is ahead of this process"),
            Some((_, sequence)) if sequence < oldest - 1 => (oldest - 1, Some("retention")),
            Some((_, sequence)) => (sequence, None),
        };
        let events: Vec<_> = self.events.iter().filter(|(sequence, _)| *sequence > after)
            .take(request.limit).map(|(_, event)| event.clone()).collect();
        let last = after + events.len() as u64;
        Ok(Batch { cursor: self.cursor(last), has_more: last < self.sequence, events, gap })
    }
}
pub fn publish(kind: &'static str, surface: gpui::EntityId, notification: Option<u64>, cx: &mut gpui::App) {
    cx.global_mut::<Journal>().push(kind, surface, notification);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replay_pages_report_retention_and_restart_without_duplicates() {
        let mut journal = Journal::default();
        let start = journal.read(&Read::default()).unwrap().cursor;
        for _ in 0..520 { journal.push("notification.received", gpui::EntityId::from(4294967297), Some(1)); }
        let first = journal.read(&Read { after: Some(start), limit: 128 }).unwrap();
        assert_eq!(first.gap, Some("retention"));
        assert_eq!(first.events.len(), 128);
        assert!(first.has_more);
        let mut cursor = first.cursor;
        let mut count = first.events.len();
        loop {
            let page = journal.read(&Read { after: Some(cursor.clone()), limit: 128 }).unwrap();
            assert_eq!(page.gap, None);
            assert!(page.events.iter().all(|event| event.cursor != cursor));
            count += page.events.len(); cursor = page.cursor;
            if !page.has_more { break; }
        }
        assert_eq!(count, CAPACITY);
        assert!(journal.read(&Read { after: Some(cursor.clone()), limit: 128 }).unwrap().events.is_empty());
        let mut restarted = Journal::default(); restarted.epoch = "0-0".into();
        assert_eq!(restarted.read(&Read { after: Some(cursor), limit: 128 }).unwrap().gap, Some("restart"));
        assert!(journal.read(&Read { after: Some(journal.cursor(521)), limit: 128 }).is_err());
        for invalid in ["junk", "1-1:-1", "1-1:01", "1-1:18446744073709551616"] {
            assert!(!Read { after: Some(invalid.into()), limit: 1 }.valid());
        }
        for limit in [0, 129, usize::MAX] { assert!(!Read { after: None, limit }.valid()); }
    }
}
