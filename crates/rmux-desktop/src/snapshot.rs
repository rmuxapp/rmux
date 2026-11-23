//! Versioned layout state. This describes fresh shells, never live process resume.
use crate::layout::Axis;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const MAX_BYTES: usize = 2 * 1024 * 1024;
const MAX_SESSION_BYTES: usize = 8 * 1024 * 1024;

/// Recovery describes fresh shells in multiple windows, not process resume.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct SessionSnapshot {
    pub version: u32,
    pub active_window: Option<usize>,
    pub windows: Vec<WindowSnapshot>,
}

impl SessionSnapshot {
    pub fn load(path: &std::path::Path) -> Result<Self> {
        use std::io::Read;
        let mut bytes = Vec::new();
        std::fs::File::open(path)?.take(MAX_SESSION_BYTES as u64 + 1).read_to_end(&mut bytes)?;
        Self::decode(&bytes)
    }
    pub fn save(&self, path: &std::path::Path) -> Result<()> {
        atomic_save(path, &self.encode()?)
    }
    pub fn encode(&self) -> Result<Vec<u8>> {
        self.validate()?;
        let bytes = serde_json::to_vec_pretty(self)?;
        ensure!(bytes.len() <= MAX_SESSION_BYTES, "session file exceeds 8 MiB");
        Ok(bytes)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        ensure!(bytes.len() <= MAX_SESSION_BYTES, "session file exceeds 8 MiB");
        let value: Self = serde_json::from_slice(bytes)?;
        value.validate()?;
        Ok(value)
    }
    fn validate(&self) -> Result<()> {
        ensure!(self.version == 1, "unsupported session version {}", self.version);
        ensure!(self.windows.len() <= 16, "session exceeds 16 windows");
        ensure!(self.active_window.is_none_or(|index| index < self.windows.len()), "invalid active window");
        // Share the budget across windows: restoring sixteen individually valid
        // 256-tab windows must not start thousands of shells.
        let mut total_tabs = 0;
        for window in &self.windows { window.validate_with_budget(&mut total_tabs)?; }
        Ok(())
    }
}

pub(super) fn atomic_save(path: &std::path::Path, bytes: &[u8]) -> Result<()> {
    use std::io::Write;
    let parent = path.parent().filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| std::path::Path::new("."));
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    file.write_all(bytes)?;
    file.as_file().sync_all()?;
    file.persist(path)?;
    Ok(())
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct WindowSnapshot {
    /// Members are indices in the saved workspace array; first is the anchor.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<crate::workspace_groups::Group<usize>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geometry: Option<crate::window_geometry::Geometry>,
    pub version: u32,
    pub sidebar: bool,
    #[serde(default = "default_sidebar_width")]
    pub sidebar_width: f32,
    pub active_workspace: usize,
    pub workspaces: Vec<WorkspaceSnapshot>,
}

fn default_sidebar_width() -> f32 {
    200.
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct WorkspaceSnapshot {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_directory: Option<std::path::PathBuf>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub color: Option<u32>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub pinned: bool,
    /// Leaf index in depth-first layout order.
    pub active_pane: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zoomed_pane: Option<usize>,
    pub layout: Node,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Node {
    Pane {
        active_tab: usize,
        tabs: Vec<TabSnapshot>,
    },
    Split {
        axis: Axis,
        weights: Vec<f32>,
        children: Vec<Node>,
    },
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct TabSnapshot {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shell_history_id: Option<String>,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ssh_destination: Option<String>,
    #[serde(default)]
    pub custom_title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<u32>,
    #[serde(default)]
    pub font_family: Option<String>,
    pub directory: Option<PathBuf>,
    #[serde(default = "default_font_size")]
    pub font_size: f32,
}
fn default_font_size() -> f32 {
    14.
}

impl WindowSnapshot {
    pub fn clear_shell_history(&mut self) {
        fn clear(node: &mut Node) {
            match node {
                Node::Pane { tabs, .. } => for tab in tabs { tab.shell_history_id = None; },
                Node::Split { children, .. } => for child in children { clear(child); },
            }
        }
        for workspace in &mut self.workspaces { clear(&mut workspace.layout); }
    }

    pub fn load(path: &std::path::Path) -> Result<Self> {
        use std::io::Read;
        let mut bytes = Vec::new();
        std::fs::File::open(path)?
            .take(MAX_BYTES as u64 + 1)
            .read_to_end(&mut bytes)?;
        let mut saved = Self::decode(&bytes)?;
        saved.clear_shell_history();
        Ok(saved)
    }
    pub fn save(&self, path: &std::path::Path) -> Result<()> {
        atomic_save(path, &self.encode()?)
    }
    pub fn encode(&self) -> Result<Vec<u8>> {
        self.validate()?;
        let bytes = serde_json::to_vec_pretty(self)?;
        ensure!(bytes.len() <= MAX_BYTES, "layout file exceeds 2 MiB");
        Ok(bytes)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        ensure!(bytes.len() <= MAX_BYTES, "layout file exceeds 2 MiB");
        let value: Self = serde_json::from_slice(bytes)?;
        value.validate()?;
        Ok(value)
    }
    fn validate(&self) -> Result<()> {
        self.validate_with_budget(&mut 0)
    }
    fn validate_with_budget(&self, total_tabs: &mut usize) -> Result<()> {
        crate::workspace_groups::Groups::from_entries(self.groups.clone(), |index| index < self.workspaces.len())?;
        ensure!(self.geometry.is_none_or(|geometry| geometry.valid()), "invalid window geometry");
        ensure!(
            self.sidebar_width.is_finite() && (160. ..=480.).contains(&self.sidebar_width),
            "invalid sidebar width"
        );
        ensure!(
            self.version == 1,
            "unsupported layout version {}",
            self.version
        );
        ensure!(
            !self.workspaces.is_empty() && self.workspaces.len() <= 64,
            "layout must contain 1–64 workspaces"
        );
        ensure!(
            self.active_workspace < self.workspaces.len(),
            "invalid active workspace"
        );
        for workspace in &self.workspaces {
            ensure!(workspace.project_directory.as_ref().is_none_or(|path| path.is_absolute()
                && path.as_os_str().len() <= 8192
                && !path.as_os_str().as_encoded_bytes().iter().any(u8::is_ascii_control)),
                "invalid workspace project directory");
            ensure!(
                !workspace.name.trim().is_empty()
                    && workspace.name.len() <= 1024
                    && !workspace.name.chars().any(char::is_control),
                "invalid workspace name"
            );
            ensure!(
                workspace.description.len() <= 1024
                    && !workspace.description.chars().any(char::is_control),
                "invalid workspace description"
            );
            ensure!(
                workspace.color.is_none_or(|color| color <= 0xffffff),
                "invalid workspace color"
            );
            ensure!(
                workspace.icon.as_deref().is_none_or(valid_workspace_icon),
                "invalid workspace icon"
            );
            let panes = workspace.layout.validate(0, total_tabs)?;
            ensure!(workspace.active_pane < panes, "invalid active pane");
            ensure!(workspace.zoomed_pane.is_none_or(|index| index < panes), "invalid zoomed pane");
        }
        Ok(())
    }
}

impl Node {
    fn validate(&self, depth: usize, total_tabs: &mut usize) -> Result<usize> {
        ensure!(depth <= crate::layout::MAX_SPLIT_DEPTH, "layout nesting exceeds {} splits", crate::layout::MAX_SPLIT_DEPTH);
        match self {
            Self::Pane { active_tab, tabs } => {
                ensure!(
                    !tabs.is_empty() && *active_tab < tabs.len(),
                    "invalid active tab or empty pane"
                );
                *total_tabs += tabs.len();
                ensure!(*total_tabs <= 256, "layout exceeds 256 tabs");
                for tab in tabs {
                    ensure!(tab.shell_history_id.as_deref().is_none_or(rmux_core::shell_history::valid_identity), "invalid terminal history identity");
                    if let Some(destination) = &tab.ssh_destination {
                        ensure!(crate::ssh::command(destination).is_ok(), "invalid saved SSH destination");
                    }
                    if let Some(family) = &tab.font_family {
                        ensure!(
                            !family.trim().is_empty()
                                && family.len() <= 128
                                && !family.chars().any(char::is_control),
                            "invalid terminal font family"
                        );
                    }
                    ensure!(
                        tab.label.len() <= 1024 && !tab.label.chars().any(char::is_control),
                        "invalid tab label"
                    );
                    ensure!(tab.color.is_none_or(|color| color <= 0xffffff), "invalid tab color");
                    if let Some(title) = &tab.custom_title {
                        ensure!(
                            !title.trim().is_empty()
                                && title.len() <= 1024
                                && !title.chars().any(char::is_control),
                            "invalid custom tab title"
                        );
                    }
                    ensure!(
                        tab.font_size.is_finite() && (8. ..=40.).contains(&tab.font_size),
                        "invalid terminal font size"
                    );
                    if let Some(directory) = &tab.directory {
                        ensure!(
                            directory.is_absolute() && directory.as_os_str().len() <= 8192,
                            "invalid saved directory"
                        );
                    }
                }
                Ok(1)
            }
            Self::Split {
                weights, children, ..
            } => {
                ensure!(
                    children.len() >= 2 && children.len() <= 256 && children.len() == weights.len(),
                    "invalid split children/weights"
                );
                ensure!(
                    weights
                        .iter()
                        .all(|weight| weight.is_finite() && *weight > 0.)
                        && weights.iter().sum::<f32>().is_finite(),
                    "invalid split proportions"
                );
                let mut panes = 0;
                for child in children {
                    panes += child.validate(depth + 1, total_tabs)?;
                }
                Ok(panes)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shell_history_identity_survives_sessions_but_not_layout_imports() -> Result<()> {
        let mut window = fixture();
        let Node::Split { children, .. } = &mut window.workspaces[0].layout else { unreachable!() };
        let Node::Pane { tabs, .. } = &mut children[0] else { unreachable!() };
        tabs[0].shell_history_id = Some("0123456789abcdef0123456789abcdef".into());
        let session = SessionSnapshot { version: 1, active_window: Some(0), windows: vec![window] };
        let encoded = session.encode()?;
        assert_eq!(SessionSnapshot::decode(&encoded)?, session);
        let temporary = tempfile::tempdir()?;
        let path = temporary.path().join("layout.json");
        session.windows[0].save(&path)?;
        let layout = WindowSnapshot::load(&path)?;
        assert!(!String::from_utf8(layout.encode()?)?.contains("shell_history_id"));
        let mut invalid: serde_json::Value = serde_json::from_slice(&encoded)?;
        invalid["windows"][0]["workspaces"][0]["layout"]["children"][0]["tabs"][0]["shell_history_id"] = "../../private".into();
        assert!(SessionSnapshot::decode(&serde_json::to_vec(&invalid)?).is_err());
        Ok(())
    }
    fn fixture() -> WindowSnapshot {
        let pane = || Node::Pane {
            active_tab: 0,
            tabs: vec![TabSnapshot {
                shell_history_id: None,
                ssh_destination: None,
                custom_title: None, color: None,
                font_family: None,
                label: "日本".into(),
                directory: None,
                font_size: 18.,
            }],
        };
        WindowSnapshot {
            groups: Vec::new(),
            geometry: None,
            version: 1,
            sidebar: false,
            sidebar_width: 280.,
            active_workspace: 0,
            workspaces: vec![WorkspaceSnapshot {
                project_directory: None,
                name: "Build".into(),
                description: "Build and test services".into(),
                color: Some(0x7aa2f7),
                icon: Some("⚙".into()),
                pinned: true,
                active_pane: 1,
                zoomed_pane: None,
                layout: Node::Split {
                    axis: Axis::Columns,
                    weights: vec![1., 2.],
                    children: vec![pane(), pane()],
                },
            }],
        }
    }
    #[test]
    fn project_identity_round_trips_and_old_layouts_remain_readable() -> anyhow::Result<()> {
        let mut saved = fixture();
        let project = std::env::temp_dir().join("project with spaces");
        saved.workspaces[0].project_directory = Some(project.clone());
        assert_eq!(WindowSnapshot::decode(&saved.encode()?)?.workspaces[0].project_directory, Some(project));
        saved.workspaces[0].project_directory = None;
        assert!(WindowSnapshot::decode(&saved.encode()?)?.workspaces[0].project_directory.is_none());
        saved.workspaces[0].project_directory = Some("relative/project".into());
        assert!(saved.encode().is_err());
        Ok(())
    }
    #[test]
    fn saved_ssh_destinations_round_trip_and_reject_invalid_targets() -> Result<()> {
        let mut value = serde_json::to_value(fixture())?;
        for destination in ["production", "ssh://user@[2001:db8::1]:2222"] {
            value["workspaces"][0]["layout"]["children"][0]["tabs"][0]["ssh_destination"] = destination.into();
            let snapshot = WindowSnapshot::decode(&serde_json::to_vec(&value)?)?;
            let round_trip: serde_json::Value = serde_json::from_slice(&snapshot.encode()?)?;
            assert_eq!(round_trip["workspaces"][0]["layout"]["children"][0]["tabs"][0]["ssh_destination"], destination);
        }
        for destination in ["-oProxyCommand=bad", "host command", "ssh://host:65536"] {
            value["workspaces"][0]["layout"]["children"][0]["tabs"][0]["ssh_destination"] = destination.into();
            assert!(WindowSnapshot::decode(&serde_json::to_vec(&value)?).is_err());
        }
        Ok(())
    }
    #[test]
    fn live_split_depth_boundary_round_trips() -> Result<()> {
        let leaf = || Node::Pane { active_tab: 0, tabs: vec![TabSnapshot {
                shell_history_id: None,
                ssh_destination: None,
            label: "Shell".into(), custom_title: None, color: None, font_family: None,
            directory: None, font_size: 14.,
        }] };
        let mut tree = leaf();
        for _ in 0..crate::layout::MAX_SPLIT_DEPTH {
            tree = Node::Split { axis: Axis::Columns, weights: vec![1., 1.], children: vec![leaf(), tree] };
        }
        let mut saved = fixture();
        saved.workspaces[0].layout = tree;
        assert_eq!(WindowSnapshot::decode(&saved.encode()?)?, saved);
        let tree = std::mem::replace(&mut saved.workspaces[0].layout, leaf());
        saved.workspaces[0].layout = Node::Split { axis: Axis::Rows, weights: vec![1., 1.], children: vec![leaf(), tree] };
        assert!(saved.encode().is_err());
        Ok(())
    }

    #[test]
    fn group_state_round_trips_and_rejects_ambiguous_ownership() -> Result<()> {
        let mut snapshot = fixture();
        assert!(WindowSnapshot::decode(&snapshot.encode()?)?.groups.is_empty());
        let mut groups = crate::workspace_groups::Groups::default();
        let id = groups.create("Build group".into(), vec![0usize], true)?;
        let group = groups.get_mut(id).unwrap();
        group.collapsed = true;
        group.pinned = true;
        group.color = Some(0x7aa2f7);
        group.icon = Some("⚙".into());
        snapshot.groups = groups.entries().to_vec();
        assert_eq!(WindowSnapshot::decode(&snapshot.encode()?)?, snapshot);
        let saved = snapshot.groups[0].clone();
        snapshot.groups[0].color = Some(0x1000000);
        assert!(snapshot.encode().is_err());
        snapshot.groups[0] = saved.clone();
        snapshot.groups[0].icon = Some("two words".into());
        assert!(snapshot.encode().is_err());
        snapshot.groups[0] = saved.clone();
        snapshot.groups[0].members = vec![1];
        assert!(snapshot.encode().is_err());
        snapshot.groups[0] = saved.clone();
        snapshot.groups[0].members.push(0);
        assert!(snapshot.encode().is_err());
        snapshot.groups[0] = saved.clone();
        let mut duplicate = saved.clone();
        duplicate.id += 1;
        snapshot.groups.push(duplicate);
        assert!(snapshot.encode().is_err());
        snapshot.groups = vec![saved];
        snapshot.groups[0].members.clear();
        assert!(snapshot.encode().is_err());
        Ok(())
    }

    #[test]
    fn round_trip_retains_order_focus_and_proportions() -> Result<()> {
        let snapshot = fixture();
        assert_eq!(WindowSnapshot::decode(&snapshot.encode()?)?, snapshot);
        Ok(())
    }
    #[test]
    fn zoom_defaults_off_and_validates_saved_leaf_index() -> Result<()> {
        let mut saved = fixture();
        assert!(WindowSnapshot::decode(&saved.encode()?)?.workspaces[0].zoomed_pane.is_none());
        saved.workspaces[0].zoomed_pane = Some(1);
        assert_eq!(WindowSnapshot::decode(&saved.encode()?)?, saved);
        saved.workspaces[0].zoomed_pane = Some(2);
        assert!(saved.encode().is_err());
        Ok(())
    }
    #[test]
    fn window_geometry_is_optional_validated_and_preserves_mode() -> Result<()> {
        use crate::window_geometry::{Geometry, Mode};
        let mut saved = fixture();
        assert!(WindowSnapshot::decode(&saved.encode()?)?.geometry.is_none());
        for mode in [Mode::Windowed, Mode::Maximized, Mode::Fullscreen] {
            saved.geometry = Some(Geometry { x: -1400., y: 20., width: 900., height: 700., mode });
            assert_eq!(WindowSnapshot::decode(&saved.encode()?)?, saved);
        }
        saved.geometry.as_mut().unwrap().width = 0.;
        assert!(saved.encode().is_err());
        Ok(())
    }
    #[test]
    fn session_round_trip_and_failed_save_preserve_previous_checkpoint() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("session.json");
        let mut session = SessionSnapshot { version: 1, active_window: Some(1), windows: vec![fixture(), fixture()] };
        session.windows[1].workspaces[0].name = "Other window".into();
        session.save(&path)?;
        assert_eq!(SessionSnapshot::load(&path)?, session);
        let previous = std::fs::read(&path)?;
        session.active_window = Some(2);
        assert!(session.save(&path).is_err());
        assert_eq!(std::fs::read(&path)?, previous);
        session.windows.clear();
        session.active_window = None;
        session.save(&path)?;
        assert_eq!(SessionSnapshot::load(&path)?, session);
        Ok(())
    }
    #[test]
    fn session_rejects_aggregate_shell_budget_version_and_oversized_input() -> Result<()> {
        let mut window = fixture();
        window.workspaces = (0..64).map(|_| fixture().workspaces.remove(0)).collect();
        window.validate()?; // 128 tabs per independently valid window.
        let mut session = SessionSnapshot { version: 1, active_window: Some(0), windows: vec![] };
        let encoded = window.encode()?;
        for _ in 0..3 { session.windows.push(WindowSnapshot::decode(&encoded)?); }
        assert!(session.encode().is_err()); // 384 across windows is rejected.
        session.windows.pop();
        assert!(session.encode().is_ok());
        session.version = 2;
        assert!(session.encode().is_err());
        assert!(SessionSnapshot::decode(&vec![b' '; MAX_SESSION_BYTES + 1]).is_err());
        Ok(())
    }
    #[test]
    fn rejects_control_characters_in_workspace_names_and_tab_labels() -> Result<()> {
        for text in [
            "line\nbreak",
            "carriage\rreturn",
            "escape\x1b",
            "null\0",
            "delete\x7f",
        ] {
            let mut value = serde_json::to_value(fixture())?;
            value["workspaces"][0]["name"] = text.into();
            assert!(WindowSnapshot::decode(&serde_json::to_vec(&value)?).is_err());
            let mut value = serde_json::to_value(fixture())?;
            value["workspaces"][0]["layout"]["children"][0]["tabs"][0]["label"] = text.into();
            assert!(WindowSnapshot::decode(&serde_json::to_vec(&value)?).is_err());
        }
        assert!(fixture().encode().is_ok());
        Ok(())
    }
    #[test]
    fn tab_colors_are_optional_validated_and_round_trip() -> Result<()> {
        let mut json = serde_json::to_value(fixture())?;
        let path = "/workspaces/0/layout/children/0/tabs/0";
        assert!(WindowSnapshot::decode(&serde_json::to_vec(&json)?).is_ok());
        for color in [0, 0xffffff, 0x7aa2f7] {
            json.pointer_mut(path).unwrap()["color"] = color.into();
            let restored = WindowSnapshot::decode(&serde_json::to_vec(&json)?)?;
            assert_eq!(serde_json::to_value(&restored)?.pointer(&format!("{path}/color")), Some(&color.into()));
            assert_eq!(WindowSnapshot::decode(&restored.encode()?)?, restored);
        }
        json.pointer_mut(path).unwrap()["color"] = 0x1000000.into();
        assert!(WindowSnapshot::decode(&serde_json::to_vec(&json)?).is_err());
        json.pointer_mut(path).unwrap()["color"] = serde_json::Value::Null;
        assert!(WindowSnapshot::decode(&serde_json::to_vec(&json)?).is_ok());
        Ok(())
    }

    #[test]
    fn custom_tab_titles_are_optional_bounded_and_round_trip() -> Result<()> {
        let mut json = serde_json::to_value(fixture())?;
        let path = "/workspaces/0/layout/children/0/tabs/0";
        json.pointer_mut(path)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove("custom_title");
        let old = WindowSnapshot::decode(&serde_json::to_vec(&json)?)?;
        assert!(
            serde_json::to_value(old)?
                .pointer(&format!("{path}/custom_title"))
                .unwrap()
                .is_null()
        );
        for title in ["Build 世界", "", "   ", "bad\nname"] {
            json.pointer_mut(path).unwrap()["custom_title"] = title.into();
            let result = WindowSnapshot::decode(&serde_json::to_vec(&json)?);
            if title == "Build 世界" {
                let restored = result?;
                assert_eq!(WindowSnapshot::decode(&restored.encode()?)?, restored);
            } else {
                assert!(result.is_err());
            }
        }
        json.pointer_mut(path).unwrap()["custom_title"] = "x".repeat(1025).into();
        assert!(WindowSnapshot::decode(&serde_json::to_vec(&json)?).is_err());
        Ok(())
    }
    #[test]
    fn saved_font_family_round_trips_and_old_layouts_use_default() -> Result<()> {
        let mut json = serde_json::to_value(fixture())?;
        let path = "/workspaces/0/layout/children/0/tabs/0";
        json.pointer_mut(path)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove("font_family");
        let old = WindowSnapshot::decode(&serde_json::to_vec(&json)?)?;
        assert!(
            serde_json::to_value(old)?
                .pointer(&format!("{path}/font_family"))
                .unwrap()
                .is_null()
        );
        json.pointer_mut(path).unwrap()["font_family"] = "Custom Mono 世界".into();
        let saved = WindowSnapshot::decode(&serde_json::to_vec(&json)?)?;
        assert_eq!(WindowSnapshot::decode(&saved.encode()?)?, saved);
        for family in ["".into(), "  ".into(), "bad\nfont".into(), "x".repeat(129)] {
            json.pointer_mut(path).unwrap()["font_family"] = family.into();
            assert!(WindowSnapshot::decode(&serde_json::to_vec(&json)?).is_err());
        }
        Ok(())
    }
    #[test]
    fn workspace_icons_default_unset_and_preserve_one_grapheme() -> Result<()> {
        let mut value = serde_json::to_value(fixture())?;
        value["workspaces"][0]
            .as_object_mut()
            .unwrap()
            .remove("icon");
        let mut snapshot = WindowSnapshot::decode(&serde_json::to_vec(&value)?)?;
        assert_eq!(snapshot.workspaces[0].icon, None);
        snapshot.workspaces[0].icon = Some("👩🏽‍💻".into());
        assert_eq!(WindowSnapshot::decode(&snapshot.encode()?)?, snapshot);
        for icon in ["", " ", "ab", "\n", &"x".repeat(65)] {
            snapshot.workspaces[0].icon = Some(icon.into());
            assert!(snapshot.encode().is_err());
        }
        Ok(())
    }
    #[test]
    fn workspace_colors_default_unset_and_validate_rgb() -> Result<()> {
        let mut value = serde_json::to_value(fixture())?;
        value["workspaces"][0]
            .as_object_mut()
            .unwrap()
            .remove("color");
        let mut snapshot = WindowSnapshot::decode(&serde_json::to_vec(&value)?)?;
        assert_eq!(snapshot.workspaces[0].color, None);
        snapshot.workspaces[0].color = Some(0x1000000);
        assert!(snapshot.encode().is_err());
        for color in [0, 0xffffff, 0x7aa2f7] {
            snapshot.workspaces[0].color = Some(color);
            assert_eq!(WindowSnapshot::decode(&snapshot.encode()?)?, snapshot);
        }
        Ok(())
    }
    #[test]
    fn descriptions_default_empty_and_reject_unbounded_or_control_text() -> Result<()> {
        let mut value = serde_json::to_value(fixture())?;
        value["workspaces"][0]
            .as_object_mut()
            .unwrap()
            .remove("description");
        let mut snapshot = WindowSnapshot::decode(&serde_json::to_vec(&value)?)?;
        assert!(snapshot.workspaces[0].description.is_empty());
        snapshot.workspaces[0].description = "x".repeat(1025);
        assert!(snapshot.encode().is_err());
        snapshot.workspaces[0].description = "line\nbreak".into();
        assert!(snapshot.encode().is_err());
        snapshot.workspaces[0].description = "Build 🦀".into();
        assert_eq!(WindowSnapshot::decode(&snapshot.encode()?)?, snapshot);
        Ok(())
    }
    #[test]
    fn save_replaces_file_and_invalid_state_preserves_previous_copy() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("layout.json");
        let mut snapshot = fixture();
        snapshot.save(&path)?;
        snapshot.workspaces[0].name = "Renamed".into();
        snapshot.save(&path)?;
        let saved = std::fs::read(&path)?;
        assert_eq!(WindowSnapshot::decode(&saved)?, snapshot);
        snapshot.version = 99;
        assert!(snapshot.save(&path).is_err());
        assert_eq!(std::fs::read(&path)?, saved);
        Ok(())
    }
    #[test]
    fn pin_state_round_trips_and_older_layouts_default_unpinned() -> Result<()> {
        let snapshot = fixture();
        assert!(WindowSnapshot::decode(&snapshot.encode()?)?.workspaces[0].pinned);
        let mut json = serde_json::to_value(snapshot)?;
        json["workspaces"][0]
            .as_object_mut()
            .unwrap()
            .remove("pinned");
        assert!(!WindowSnapshot::decode(&serde_json::to_vec(&json)?)?.workspaces[0].pinned);
        Ok(())
    }
    #[test]
    fn font_size_round_trips_defaults_and_rejects_invalid_values() -> Result<()> {
        let mut json = serde_json::to_value(fixture())?;
        let tab = &mut json["workspaces"][0]["layout"]["children"][0]["tabs"][0];
        tab.as_object_mut().unwrap().remove("font_size");
        let restored = WindowSnapshot::decode(&serde_json::to_vec(&json)?)?;
        let encoded = serde_json::to_value(restored)?;
        assert_eq!(
            encoded["workspaces"][0]["layout"]["children"][0]["tabs"][0]["font_size"],
            14.
        );
        assert_eq!(
            encoded["workspaces"][0]["layout"]["children"][1]["tabs"][0]["font_size"],
            18.
        );
        for size in [0., 7., 41., f32::INFINITY, f32::NAN] {
            let mut snapshot = fixture();
            let Node::Split { children, .. } = &mut snapshot.workspaces[0].layout else {
                unreachable!()
            };
            let Node::Pane { tabs, .. } = &mut children[0] else {
                unreachable!()
            };
            tabs[0].font_size = size;
            assert!(snapshot.encode().is_err());
        }
        Ok(())
    }
    #[test]
    fn older_layout_defaults_sidebar_width_and_rejects_invalid_widths() -> Result<()> {
        let mut json = serde_json::to_value(fixture())?;
        json.as_object_mut().unwrap().remove("sidebar_width");
        let restored = WindowSnapshot::decode(&serde_json::to_vec(&json)?)?;
        assert_eq!(restored.sidebar_width, 200.);
        for width in [0., 159., 481., f32::INFINITY, f32::NAN] {
            let mut snapshot = fixture();
            snapshot.sidebar_width = width;
            assert!(snapshot.encode().is_err());
        }
        Ok(())
    }
    #[test]
    fn rejects_invalid_versions_focus_and_weights() {
        let mut snapshot = fixture();
        snapshot.version = 2;
        assert!(snapshot.encode().is_err());
        snapshot.version = 1;
        snapshot.workspaces[0].active_pane = 2;
        assert!(snapshot.encode().is_err());
        snapshot.workspaces[0].active_pane = 0;
        if let Node::Split { weights, .. } = &mut snapshot.workspaces[0].layout {
            weights[0] = f32::NAN;
        }
        assert!(snapshot.encode().is_err());
        assert!(WindowSnapshot::decode(&vec![b' '; MAX_BYTES + 1]).is_err());
    }
}

/// One display grapheme, bounded independently of the layout file size.
pub use rmux_core::workspace_groups::valid_workspace_icon;
