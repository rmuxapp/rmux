//! Bounded shell-approved command records; no input capture or filesystem access.
use std::collections::VecDeque;
pub const ENTRY_LIMIT: usize = 512;
pub const TEXT_LIMIT: usize = 128 * 1024;
pub const COMMAND_LIMIT: usize = 8 * 1024;

#[derive(Default, Debug, PartialEq, Eq)]
pub struct History {
    entries: VecDeque<String>,
    bytes: usize,
}
impl History {
    pub fn clear(&mut self) -> bool {
        let changed = !self.entries.is_empty();
        self.entries.clear();
        self.bytes = 0;
        changed
    }
    pub fn text_bytes(&self) -> usize { self.bytes }
    pub fn entries(&self) -> impl DoubleEndedIterator<Item = &str> + ExactSizeIterator { self.entries.iter().map(String::as_str) }
    pub fn record(&mut self, command: &str) -> bool {
        if command.trim().is_empty() || command.len() > COMMAND_LIMIT || command.contains('\0')
            || self.entries.back().is_some_and(|last| last == command) { return false; }
        while self.entries.len() >= ENTRY_LIMIT || self.bytes + command.len() > TEXT_LIMIT {
            if let Some(old) = self.entries.pop_front() { self.bytes -= old.len(); }
        }
        self.bytes += command.len();
        self.entries.push_back(command.to_owned());
        true
    }
}

