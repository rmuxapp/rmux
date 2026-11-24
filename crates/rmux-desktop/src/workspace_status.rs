//! Summarize cached status from every live tab, including inactive panes.
use rmux_terminal::{Progress, ProgressState, program_status::{State, Summary}};

#[derive(Default)]
pub(super) struct WorkspaceStatus {
    selected: Option<(u8, String, u32)>,
    count: usize,
}
impl WorkspaceStatus {
    pub fn include(&mut self, program: Option<&Summary>, progress: Option<Progress>, exited: bool) {
        if exited { return; }
        let (priority, text, tint, count) = if let Some(summary) = program {
            let record = &summary.record;
            let (label, tint) = crate::pane_view::task_badge(record);
            let priority = match record.state {
                State::Blocked => 5, State::Error => 4, State::Working => 3,
                State::Done => 2, State::Idle => 1,
            };
            let mut text = if record.app.is_empty() { label.to_owned() } else { format!("{} · {label}", record.app) };
            if let Some(percent) = record.progress { text.push_str(&format!(" {percent}%")); }
            (priority, text, tint, summary.count.max(1))
        } else if let Some(progress) = progress {
            let (priority, label, tint) = match progress.state {
                ProgressState::Running | ProgressState::Indeterminate => (3, "Working", 0x86b8f5),
                ProgressState::Error => (4, "Error", 0xf18a8a),
                ProgressState::Paused => (5, "Paused", 0xf0b85b),
            };
            let text = match progress.percent {
                Some(percent) if progress.state != ProgressState::Indeterminate => format!("{label} {percent}%"),
                _ => label.to_owned(),
            };
            (priority, text, tint, 1)
        } else { return; };
        self.count += count;
        if self.selected.as_ref().is_none_or(|(current, ..)| priority > *current) {
            self.selected = Some((priority, text, tint));
        }
    }

    pub fn badge(&self) -> Option<(String, u32)> {
        let (_, text, tint) = self.selected.as_ref()?;
        let text = if self.count > 1 { format!("{text} +{}", self.count - 1) } else { text.clone() };
        Some((text, *tint))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmux_terminal::program_status::{Kind, Record};

    fn report(app: &str, state: State) -> Summary {
        Summary { record: Record { id: "task".into(), state, kind: Kind::None,
            progress: None, app: app.into(), title: String::new(), message: String::new() }, count: 1 }
    }

    #[test]
    fn unnamed_and_other_app_reports_are_not_dropped() {
        for (app, expected) in [("", "Done"), ("claude", "claude · Done"), ("Claude", "Claude · Done"), ("build", "build · Done")] {
            let mut summary = WorkspaceStatus::default();
            summary.include(Some(&report(app, State::Done)), None, false);
            assert_eq!(summary.badge().unwrap().0, expected);
        }
    }

    #[test]
    fn all_tabs_contribute_and_attention_then_work_take_priority() {
        let mut summary = WorkspaceStatus::default();
        summary.include(Some(&report("", State::Done)), None, false);
        summary.include(Some(&report("", State::Working)), None, false);
        assert_eq!(summary.badge().unwrap().0, "Working +1");
        summary.include(Some(&report("", State::Error)), None, false);
        let mut blocked = report("Claude", State::Blocked);
        blocked.record.kind = Kind::Permission;
        summary.include(Some(&blocked), None, false);
        assert_eq!(summary.badge().unwrap().0, "Claude · Approval +3");
    }

    #[test]
    fn progress_is_included_without_double_counting_program_status() {
        let progress = Some(Progress { state: ProgressState::Running, percent: Some(42) });
        let mut summary = WorkspaceStatus::default();
        summary.include(None, progress, false);
        assert_eq!(summary.badge().unwrap().0, "Working 42%");
        let mut report = report("", State::Working);
        report.record.progress = Some(60);
        report.count = 2;
        let mut summary = WorkspaceStatus::default();
        summary.include(Some(&report), progress, false);
        assert_eq!(summary.badge().unwrap().0, "Working 60% +1");
    }

    #[test]
    fn exited_and_cleared_terminals_leave_no_stale_badge() {
        let mut summary = WorkspaceStatus::default();
        summary.include(Some(&report("Claude", State::Working)), None, true);
        summary.include(None, Some(Progress { state: ProgressState::Error, percent: None }), true);
        summary.include(None, None, false);
        assert!(summary.badge().is_none());
    }
}
