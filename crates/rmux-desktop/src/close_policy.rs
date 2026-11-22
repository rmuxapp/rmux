//! Startup policy shared by every user-requested termination scope.
/// Keep user/shell titles readable in native confirmation dialogs.
pub fn display_name(name: &str) -> String {
    let mut chars = name.chars().map(|ch| if ch.is_control() { ' ' } else { ch });
    let mut label: String = chars.by_ref().take(80).collect();
    if chars.next().is_some() { label.push('…'); }
    let label = label.trim();
    if label.is_empty() { "Untitled".into() } else { label.to_owned() }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ClosePolicy {
    Active,
    Always,
    Never,
}
impl ClosePolicy {
    pub fn parse(value: Option<&str>) -> Result<Self, &'static str> {
        match value {
            None | Some("active") => Ok(Self::Active),
            Some("always") => Ok(Self::Always),
            Some("never") => Ok(Self::Never),
            _ => Err("RMUX_CONFIRM_CLOSE must be active, always or never"),
        }
    }
    pub fn confirms(self, has_terminals: bool, active_work: impl FnOnce() -> bool) -> bool {
        has_terminals
            && match self {
                Self::Active => active_work(),
                Self::Always => true,
                Self::Never => false,
            }
    }
}
impl gpui::Global for ClosePolicy {}

/// One termination dialog across all scopes. Dropping its task releases the gate
/// even when a window disappears before the answer can be applied.
#[derive(Default)]
pub struct PromptGate(std::sync::Arc<std::sync::atomic::AtomicBool>);
impl gpui::Global for PromptGate {}
pub struct PromptPermit(std::sync::Arc<std::sync::atomic::AtomicBool>);
impl PromptGate {
    pub fn is_active(&self) -> bool {
        self.0.load(std::sync::atomic::Ordering::Acquire)
    }
    pub fn acquire(&self) -> Option<PromptPermit> {
        use std::sync::atomic::Ordering;
        self.0
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .ok()
            .map(|_| PromptPermit(self.0.clone()))
    }
}
impl Drop for PromptPermit {
    fn drop(&mut self) {
        self.0.store(false, std::sync::atomic::Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn prompt_gate_releases_on_cancellation_and_rejects_overlap() {
        let gate = PromptGate::default();
        let permit = gate.acquire().unwrap();
        assert!(gate.acquire().is_none());
        drop(permit);
        assert!(gate.acquire().is_some());
        assert!(gate.acquire().is_some());
    }
    #[test]
    fn policies_cover_empty_idle_active_and_invalid_configuration() {
        assert_eq!(ClosePolicy::parse(None), Ok(ClosePolicy::Active));
        for (name, idle, active) in [
            ("active", false, true),
            ("always", true, true),
            ("never", false, false),
        ] {
            let policy = ClosePolicy::parse(Some(name)).unwrap();
            assert!(!policy.confirms(false, || panic!("empty scope queried activity")));
            assert_eq!(policy.confirms(true, || false), idle);
            assert_eq!(policy.confirms(true, || true), active);
        }
        assert!(ClosePolicy::parse(Some("")).is_err());
        assert!(ClosePolicy::parse(Some("sometimes")).is_err());
        assert!(ClosePolicy::Always.confirms(true, || panic!("always queried activity")));
        assert!(!ClosePolicy::Never.confirms(true, || panic!("never queried activity")));
    }
}
