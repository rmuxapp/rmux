use std::time::Duration;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use web_time::Instant;

#[derive(Clone, Default)]
pub struct Typeahead {
    query: String,
    last: Option<Instant>,
}

impl Typeahead {
    pub fn select(&mut self, key: &str, labels: &[&str], enabled: &[bool], current: usize, now: Instant) -> Option<usize> {
        let mut chars = key.chars();
        let Some(ch) = chars.next().filter(|ch| !ch.is_control() && !ch.is_whitespace()) else {
            self.query.clear();
            return None;
        };
        if chars.next().is_some() {
            self.query.clear();
            return None;
        }
        let key = ch.to_lowercase().collect::<String>();
        let fresh = self.last.is_none_or(|last| now.duration_since(last) > Duration::from_secs(1));
        self.last = Some(now);
        let cycling = fresh || self.query.is_empty() || self.query == key || self.query.len() >= 64;
        if cycling { self.query = key.clone(); } else { self.query.push_str(&key); }
        let count = labels.len().min(enabled.len());
        if count == 0 { return None; }
        let current = current.min(count - 1);
        let find = |query: &str, skip_current: bool| {
            (0..count).map(|step| (current + step + usize::from(skip_current)) % count)
                .find(|index| enabled[*index] && labels[*index].to_lowercase().starts_with(query))
        };
        if let Some(index) = find(&self.query, cycling) { return Some(index); }
        // An unmatched sequence starts a new prefix with the latest character.
        self.query = key;
        find(&self.query, true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_prefixes_cycle_skip_disabled_and_expire() {
        let labels = ["Copy", "Copy with Line Breaks", "Paste", "Clear Scrollback", "Close"];
        let enabled = [true, false, true, true, true];
        let mut search = Typeahead::default();
        let now = Instant::now();
        assert_eq!(search.select("c", &labels, &enabled, 0, now), Some(3));
        assert_eq!(search.select("c", &labels, &enabled, 3, now), Some(4));
        assert_eq!(search.select("c", &labels, &enabled, 4, now), Some(0));
        assert_eq!(search.select("l", &labels, &enabled, 0, now), Some(3));
        assert_eq!(search.select("o", &labels, &enabled, 3, now), Some(4));
        assert_eq!(search.select("p", &labels, &enabled, 4, now), Some(2));
        assert_eq!(search.select("c", &labels, &enabled, 2, now + Duration::from_secs(2)), Some(3));
        assert_eq!(search.select("down", &labels, &enabled, 3, now + Duration::from_secs(2)), None);
        assert_eq!(search.select("c", &labels, &enabled, 3, now + Duration::from_secs(2)), Some(4));
        assert_eq!(search.select("c", &labels, &[false; 5], 3, now + Duration::from_secs(2)), None);
        assert_eq!(search.select("c", &[], &[], 0, now + Duration::from_secs(2)), None);
    }
}
