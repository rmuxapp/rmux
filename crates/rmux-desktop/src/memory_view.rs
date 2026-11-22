use super::*;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Sort {
    #[default]
    Storage,
    Images,
    ShellMemory,
    ShellCpu,
    ForegroundMemory,
    ForegroundCpu,
    TreeMemory,
    TreeCpu,
}

impl Sort {
    pub const ALL: [Self; 8] = [Self::Storage, Self::Images, Self::ShellMemory, Self::ShellCpu, Self::ForegroundMemory, Self::ForegroundCpu, Self::TreeMemory, Self::TreeCpu];
    pub fn next(self) -> Self {
        let index = Self::ALL.iter().position(|sort| *sort == self).unwrap_or(0);
        Self::ALL[(index + 1) % Self::ALL.len()]
    }
    pub fn label(self) -> &'static str {
        match self { Self::Storage => "Screen storage ↓", Self::Images => "Image storage ↓", Self::ShellMemory => "Shell RSS ↓", Self::ShellCpu => "Shell CPU ↓", Self::ForegroundMemory => "Foreground RSS ↓", Self::ForegroundCpu => "Foreground CPU ↓", Self::TreeMemory => "Shell tree RSS ↓", Self::TreeCpu => "Shell tree CPU ↓" }
    }
    pub(super) fn indices(self, rows: &[Row]) -> Vec<usize> {
        let mut indices: Vec<_> = (0..rows.len()).collect();
        indices.sort_by(|a, b| {
            let (a, b) = (&rows[*a], &rows[*b]);
            match self {
            Self::Storage => b.usage.map(|s| s.primary_resident_bytes.saturating_add(s.alternate_resident_bytes))
                .cmp(&a.usage.map(|s| s.primary_resident_bytes.saturating_add(s.alternate_resident_bytes))),
            Self::Images => b.usage.map(|s| s.image_bytes).cmp(&a.usage.map(|s| s.image_bytes)),
            Self::ShellMemory => b.process.map(|s| s.resident_bytes).cmp(&a.process.map(|s| s.resident_bytes)),
            Self::ShellCpu => b.process.and_then(|s| s.cpu_percent).partial_cmp(&a.process.and_then(|s| s.cpu_percent)).unwrap_or(std::cmp::Ordering::Equal),
            Self::ForegroundMemory => b.foreground.map(|s| s.resident_bytes).cmp(&a.foreground.map(|s| s.resident_bytes)),
            Self::ForegroundCpu => b.foreground.and_then(|s| s.cpu_percent).partial_cmp(&a.foreground.and_then(|s| s.cpu_percent)).unwrap_or(std::cmp::Ordering::Equal),
            Self::TreeMemory => b.tree.map(|s| s.resident_bytes).cmp(&a.tree.map(|s| s.resident_bytes)),
            Self::TreeCpu => b.tree.and_then(|s| s.cpu_percent).partial_cmp(&a.tree.and_then(|s| s.cpu_percent)).unwrap_or(std::cmp::Ordering::Equal),
            }
        });
        indices
    }
    fn rows(self, rows: &[Row]) -> Vec<&Row> {
        self.indices(rows).into_iter().map(|index| &rows[index]).collect()
    }
}

pub struct Row {
    pub workspace_id: EntityId,
    pub surface_id: EntityId,
    pub name: String,
    pub pid: Option<u32>,
    pub process: Option<process_metrics::Sample>,
    pub tree: Option<process_metrics::TreeSample>,
    pub foreground_pid: Option<u32>,
    pub foreground: Option<process_metrics::Sample>,
    pub usage: Option<rmux_terminal::MemoryUsage>,
    pub status: &'static str,
}

fn mib(bytes: u64) -> String { format!("{:.2} MiB", bytes as f64 / 1_048_576.) }

#[cfg(test)]
mod tests {
    #[test]
    fn image_sort_distinguishes_zero_missing_and_screen_storage() {
        let rows: Vec<_> = [Some((1000, 0)), None, Some((1, 500)), Some((2, 500))]
            .into_iter().enumerate().map(|(index, usage)| super::Row {
                workspace_id: gpui::EntityId::from(0x100000001), surface_id: gpui::EntityId::from(0x100000002 + index as u64),
                name: index.to_string(), pid: None, process: None, tree: None, foreground_pid: None, foreground: None,
                usage: usage.map(|(resident, images)| rmux_terminal::MemoryUsage {
                    primary_resident_bytes: resident, alternate_resident_bytes: 0, virtual_bytes: 0,
                    compressed_bytes: 0, image_bytes: images, compression_supported: false,
                }), status: "Sampled",
            }).collect();
        assert_eq!(super::Sort::Images.indices(&rows), [2, 3, 0, 1]);
        assert_eq!(super::Sort::Storage.indices(&rows), [0, 3, 2, 1]);
        let report: serde_json::Value = serde_json::from_str(&super::report(&rows, None, super::Sort::Images, Some(1234))).unwrap();
        assert_eq!(report["surfaces"][0]["name"], "2");
        assert_eq!(report["sample_completed_at_unix_ms"], 1234);
    }
}

pub fn report(rows: &[Row], application: Option<process_metrics::Sample>, sort: Sort, sampled_at: Option<u64>) -> String {
    let rows: Vec<_> = sort.rows(rows).into_iter().map(|row| serde_json::json!({
        "workspace_id":row.workspace_id.to_string(), "surface_id":row.surface_id.to_string(),
        "name":row.name,"process_id":row.pid,"status":row.status,
        "shell_process":row.process,
        "shell_tree":row.tree,
        "foreground_group_leader":{"process_id":row.foreground_pid,"sample":row.foreground},
        "memory":row.usage.map(|usage| serde_json::json!({
            "primary_resident_bytes":usage.primary_resident_bytes,"alternate_resident_bytes":usage.alternate_resident_bytes,
            "compressed_bytes":usage.compressed_bytes,"image_bytes":usage.image_bytes,
            "virtual_bytes":usage.virtual_bytes,"compression_supported":usage.compression_supported,
        }))
    })).collect();
    serde_json::to_string_pretty(&serde_json::json!({"scope":"ghostty_screen_storage","sample_mode":"manual","sample_completed_at_unix_ms":sampled_at,"application":{"process_id":std::process::id(),"scope":"application_process","sample":application},"surfaces":rows})).expect("JSON memory report")
}

pub fn panel(rows: &[Row], application: Option<process_metrics::Sample>, sampled_at: Option<u64>, sort: Sort, loading: bool, scroll: ListState, focus: Option<&FocusHandle>, selected: usize, cx: &mut Context<window_view::WorkspaceWindow>) -> AnyElement {
    let indices = sort.indices(rows);
    let entity = cx.entity();
    let sampled = rows.iter().filter(|row| row.usage.is_some()).count();
    let resident: u64 = rows.iter().filter_map(|row| row.usage)
        .map(|usage| usage.primary_resident_bytes.saturating_add(usage.alternate_resident_bytes))
        .fold(0, u64::saturating_add);
    div().key_context("TerminalMemory")
        .when_some(focus, |panel, focus| panel.track_focus(focus))
        .on_key_down(cx.listener(window_view::WorkspaceWindow::memory_key))
        .h(px(240.)).max_h(relative(0.45)).flex_shrink_0().min_h_0()
        .flex().flex_col().bg(rgb(0x242424)).border_t_1().border_color(rgb(0x3b4352))
        .child(div().h(px(36.)).flex_shrink_0().px_3().flex().items_center().gap_2()
            .child(div().flex_1().min_w_0().truncate().text_sm()
                .child(if loading { "Terminal memory · Sampling…".into() } else { format!("Terminal memory · {sampled}/{} sampled · {} resident", rows.len(), mib(resident)) }))
            .child(pane_view::pane_control("refresh-memory", "Refresh memory sample")
                .on_click(|event, window, cx| { if event.standard_click() { window.dispatch_action(Box::new(RefreshTerminalMemory), cx); } }).child(icons::icon("refresh")))
            .child(pane_view::pane_control("copy-memory", "Copy sampled memory report as JSON")
                .on_click(|event, window, cx| { if event.standard_click() { window.dispatch_action(Box::new(CopyTerminalMemory), cx); } }).child(icons::icon("copy")))
            .child(pane_view::pane_control("close-memory", "Close memory panel")
                .on_click(|event, window, cx| { if event.standard_click() { window.dispatch_action(Box::new(CloseTerminalMemory), cx); } }).child(icons::icon("close"))))
        .child(div().px_3().pb_1().text_sm()
            .child(application.map(|sample| format!("rmux · RSS {} · CPU {} · all windows", mib(sample.resident_bytes), sample.cpu_percent.map(|cpu| format!("{cpu:.1}%")).unwrap_or_else(|| "— refresh after baseline".into()))).unwrap_or_else(|| "rmux process sample unavailable".into())))
        .child(div().id("memory-sort").mx_3().mb_1().text_xs().cursor_pointer()
            .text_color(rgb(0xb4c7ef)).hover(|style| style.text_color(rgb(0xffffff)))
            .tooltip(|_, cx| cx.new(|_| pane_view::ControlHint("Choose memory sort order".into())).into())
            .on_click(|event, window, cx| { if event.standard_click() { window.dispatch_action(Box::new(ShowTerminalMemorySort), cx); } })
            .child(format!("{} · Choose…{}", sort.label(), sampled_at.map(|ms| {
                let seconds = ms / 1000;
                format!(" · Sampled {:02}:{:02}:{:02} UTC", seconds / 3600 % 24, seconds / 60 % 60, seconds % 60)
            }).unwrap_or_default())))
        .child(div().px_3().pb_2().text_xs().text_color(rgb(0xa0a0a0))
            .child("Shell tree includes descendants; RSS may count shared pages more than once · up to 256 terminals / 4,096 processes · manual refresh"))
        .child(list(scroll, move |index, _, cx| {
            entity.update(cx, |root, cx| root.render_memory_row(indices[index], index == selected, cx))
        }).with_sizing_behavior(ListSizingBehavior::Auto).flex_1().min_h_0().w_full())
        .child(div().px_3().py_1().flex_shrink_0().text_xs().text_color(rgb(0xa0a0a0))
            .child("↑↓ / Tab Select · PgUp/PgDn Page · Enter Focus · R Refresh · S Sort · C Copy · Esc Close"))
        .into_any_element()
}

pub fn render_row(row: &Row, index: usize, selected: bool, cx: &mut Context<window_view::WorkspaceWindow>) -> AnyElement {
    let workspace = row.workspace_id;
    let surface = row.surface_id;
    let hint: SharedString = format!("{} · Click to focus terminal", row.name).into();
    let detail = row.usage.map(|usage| format!("Resident {} · Compressed {} · Images {} · Virtual {}",
        mib(usage.primary_resident_bytes.saturating_add(usage.alternate_resident_bytes)),
        mib(usage.compressed_bytes), mib(usage.image_bytes), mib(usage.virtual_bytes)))
        .unwrap_or_else(|| row.status.into());
    div().id(("terminal-memory-row", index)).px_3().py_2().border_b_1().border_color(rgb(0x383838))
        .when(selected, |row| row.bg(rgb(crate::chrome_style::SELECTED)))
        .cursor_pointer().hover(move |style| style.bg(rgb(if selected { crate::chrome_style::SELECTED_HOVER } else { 0x303030 })))
        .tooltip(move |_, cx| cx.new(|_| pane_view::ControlHint(hint.clone())).into())
        .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
            if event.standard_click() { this.focus_memory_terminal(workspace, surface, window, cx); }
        }))
        .child(div().text_sm().truncate().child(format!("{} · PID {}", row.name, row.pid.map(|pid| pid.to_string()).unwrap_or_else(|| "—".into()))))
        .child(div().text_xs().text_color(rgb(0xa0a0a0)).child(detail))
        .child(div().text_xs().text_color(rgb(0xa0a0a0)).child(row.process.map(|sample| format!("Shell RSS {} · CPU {}", mib(sample.resident_bytes), sample.cpu_percent.map(|cpu| format!("{cpu:.1}%")).unwrap_or_else(|| "— refresh after baseline".into()))).unwrap_or_else(|| "Shell process sample unavailable".into())))
        .child(div().text_xs().text_color(rgb(0xa0a0a0)).child(row.tree.map(|sample| format!("Shell tree RSS {} · CPU {} · {}/{} processes{}", mib(sample.resident_bytes), sample.cpu_percent.map(|cpu| format!("{cpu:.1}%")).unwrap_or_else(|| "—".into()), sample.sampled_processes, sample.expected_processes, if sample.complete { "" } else { " · partial" })).unwrap_or_else(|| "Shell tree unavailable or ownership changed".into())))
        .when_some(row.foreground_pid, |row_element, pid| row_element.child(div().text_xs().text_color(rgb(0xa0a0a0))
            .child(row.foreground.map(|sample| format!("Foreground PID {pid} · RSS {} · CPU {}", mib(sample.resident_bytes), sample.cpu_percent.map(|cpu| format!("{cpu:.1}%")).unwrap_or_else(|| "— refresh after baseline".into())))
                .unwrap_or_else(|| format!("Foreground PID {pid} · unavailable or job changed")))))
        .into_any_element()
}
