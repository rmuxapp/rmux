//! Bounded startup configuration. No watcher or automatic file creation.
use crate::close_policy::ClosePolicy;
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{io::Read, path::{Path, PathBuf}};

/// Unlike reloaded preferences, this snapshot describes the startup-only socket
/// choice. Keep restart feedback until the user restores it or restarts.
pub struct StartupSocket {
    pub configured: Option<PathBuf>,
    pub environment_override: bool,
}
impl gpui::Global for StartupSocket {}
impl StartupSocket {
    pub fn reload_notice(&self, configured: &Option<PathBuf>) -> Option<&'static str> {
        if *configured == self.configured { return None; }
        Some(if self.environment_override {
            "The automation socket is controlled by RMUX_SOCKET_DIR. Remove that override and restart rmux to use socket_directory."
        } else {
            "Restart rmux to apply the socket directory change; the current automation endpoint is unchanged."
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(try_from = "String", into = "String")]
pub struct HexColor(u32);
impl TryFrom<String> for HexColor {
    type Error = &'static str;
    fn try_from(text: String) -> std::result::Result<Self, Self::Error> {
        let hex = text.strip_prefix('#').ok_or("color must use #RRGGBB")?;
        if hex.len() != 6 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err("color must use #RRGGBB");
        }
        Ok(Self(u32::from_str_radix(hex, 16).map_err(|_| "invalid color")?))
    }
}
impl From<HexColor> for String {
    fn from(color: HexColor) -> Self { format!("#{:06x}", color.0) }
}
impl From<rmux_terminal::Color> for HexColor {
    fn from(rmux_terminal::Color(r, g, b): rmux_terminal::Color) -> Self {
        Self((u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b))
    }
}
impl HexColor {
    fn rgb(self) -> rmux_terminal::Color {
        rmux_terminal::Color((self.0 >> 16) as u8, (self.0 >> 8) as u8, self.0 as u8)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(try_from = "String", into = "String")]
pub enum CellColor { Literal(HexColor), CellForeground, CellBackground }
impl TryFrom<String> for CellColor {
    type Error = &'static str;
    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        match value.as_str() {
            "cell-foreground" => Ok(Self::CellForeground),
            "cell-background" => Ok(Self::CellBackground),
            _ => HexColor::try_from(value).map(Self::Literal),
        }
    }
}
impl From<CellColor> for String {
    fn from(value: CellColor) -> Self {
        match value {
            CellColor::Literal(color) => color.into(),
            CellColor::CellForeground => "cell-foreground".into(),
            CellColor::CellBackground => "cell-background".into(),
        }
    }
}
impl CellColor {
    pub fn parse_theme(value: &str) -> Result<Self> {
        match value {
            "cell-foreground" => Ok(Self::CellForeground),
            "cell-background" => Ok(Self::CellBackground),
            _ => Ok(Self::Literal(rmux_terminal::parse_color(value)?.into())),
        }
    }
    pub fn resolve(self, foreground: rmux_terminal::Color, background: rmux_terminal::Color) -> rmux_terminal::Color {
        match self { Self::Literal(color) => color.rgb(), Self::CellForeground => foreground, Self::CellBackground => background }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TerminalTheme {
    #[default]
    Dark,
    Light,
    SolarizedDark,
    SolarizedLight,
    Nord,
    CatppuccinMocha,
    CatppuccinLatte,
}
impl TerminalTheme {
    pub const ALL: [Self; 7] = [Self::Dark, Self::Light, Self::SolarizedDark, Self::SolarizedLight, Self::Nord, Self::CatppuccinMocha, Self::CatppuccinLatte];
    pub fn label(self) -> &'static str {
        match self {
            Self::Dark => "Dark", Self::Light => "Light",
            Self::SolarizedDark => "Solarized Dark", Self::SolarizedLight => "Solarized Light",
            Self::Nord => "Nord",
            Self::CatppuccinMocha => "Catppuccin Mocha",
            Self::CatppuccinLatte => "Catppuccin Latte",
        }
    }
    fn bundled(self) -> Option<&'static crate::theme_file::Theme> {
        match self {
            Self::CatppuccinMocha => Some(crate::theme_file::catppuccin(false)),
            Self::CatppuccinLatte => Some(crate::theme_file::catppuccin(true)),
            _ => None,
        }
    }
    pub fn cycle(self, delta: i32) -> Self {
        let index = Self::ALL.iter().position(|theme| *theme == self).unwrap_or(0) as i32;
        Self::ALL[(index + delta.signum()).rem_euclid(Self::ALL.len() as i32) as usize]
    }
}

#[derive(Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FocusedCursor {
    #[default]
    Application,
    Block,
    Bar,
    Underline,
}
impl FocusedCursor {
    pub fn label(self) -> &'static str {
        match self { Self::Application => "Application", Self::Block => "Block", Self::Bar => "Bar", Self::Underline => "Underline" }
    }
    pub fn cycle(self, backwards: bool) -> Self {
        let choices = [Self::Application, Self::Block, Self::Bar, Self::Underline];
        let index = choices.iter().position(|choice| *choice == self).unwrap_or(0);
        choices[(index + if backwards { 3 } else { 1 }) % choices.len()]
    }
    fn shape(self, application: rmux_terminal::CursorShape) -> rmux_terminal::CursorShape {
        use rmux_terminal::CursorShape;
        match self { Self::Application => application, Self::Block => CursorShape::Block, Self::Bar => CursorShape::Bar, Self::Underline => CursorShape::Underline }
    }
}

#[derive(Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UnfocusedCursor {
    #[default]
    Hollow,
    Hidden,
}
impl UnfocusedCursor {
    pub fn shape(
        self,
        focused: bool,
        composing: bool,
        terminal: rmux_terminal::CursorShape,
    ) -> Option<rmux_terminal::CursorShape> {
        use rmux_terminal::CursorShape;
        if focused {
            Some(if composing { CursorShape::Bar } else { terminal })
        } else {
            match self {
                Self::Hollow => Some(CursorShape::Hollow),
                Self::Hidden => None,
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NotificationFocus {
    #[default]
    UnfocusedTerminal,
    BackgroundApp,
}
impl NotificationFocus {
    pub fn allows(self, terminal_focused: bool, app_active: bool) -> bool {
        !terminal_focused && (self == Self::UnfocusedTerminal || !app_active)
    }
    pub fn label(self) -> &'static str {
        match self { Self::UnfocusedTerminal => "Terminal unfocused", Self::BackgroundApp => "App in background" }
    }
    pub fn toggle(self) -> Self {
        match self { Self::UnfocusedTerminal => Self::BackgroundApp, Self::BackgroundApp => Self::UnfocusedTerminal }
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandMode { #[default] Terminal, Background }

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CustomCommand {
    #[serde(default)]
    pub mode: CommandMode,
    #[serde(default)]
    pub env: std::collections::BTreeMap<String, String>,
    pub name: String,
    pub argv: Vec<String>,
    pub cwd: Option<std::path::PathBuf>,
}

impl CustomCommand {
    /// Resolve only complete argument tokens; path contents are never reparsed.
    /// A shell's mutable cwd is deliberately not a substitute for project identity.
    pub fn resolve_project(mut self, project: Option<&std::path::Path>) -> Result<Self> {
        for argument in self.argv.iter_mut().skip(1) {
            if argument == "{project_dir}" {
                let path = project.context("This command needs a project workspace. Open the project folder in rmux first; standalone --command has no project identity")?;
                anyhow::ensure!(path.is_absolute(), "Project directory must be absolute");
                let path = path.to_str().context("Project directory is not valid UTF-8")?;
                anyhow::ensure!(!path.contains('\0'), "Project directory contains a NUL");
                *argument = path.to_owned();
            } else if argument == "{{project_dir}}" {
                *argument = "{project_dir}".into();
            }
        }
        anyhow::ensure!(self.argv.iter().map(String::len).sum::<usize>() <= 16 * 1024,
            "Resolved command arguments exceed 16 KiB");
        Ok(self)
    }
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LayoutPreset {
    pub name: String,
    pub path: std::path::PathBuf,
    pub cwd: Option<std::path::PathBuf>,
    #[serde(default)]
    pub session: bool,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Preferences {
    pub commands: Vec<CustomCommand>,
    pub layout_presets: Vec<LayoutPreset>,
    pub keybindings: Vec<crate::shortcuts::Binding>,
    pub new_workspace_placement: crate::workspace_groups::Placement,
    pub group_new_workspace_placement: crate::workspace_groups::Placement,
    pub restore_session: bool,
    pub session_file: Option<std::path::PathBuf>,
    pub shell_history_directory: Option<std::path::PathBuf>,
    pub socket_directory: Option<std::path::PathBuf>,
    pub font_size: f32,
    pub terminal_padding: u8,
    pub font_family: String,
    pub scrollback_mib: u16,
    pub copy_on_select: bool,
    pub native_notifications: bool,
    pub notification_focus: NotificationFocus,
    pub macos_option_as_alt: bool,
    pub unfocused_cursor: UnfocusedCursor,
    pub focused_cursor: FocusedCursor,
    pub cursor_blink: bool,
    pub controls_on_hover: bool,
    pub hide_top_bar: bool,
    pub compact_sidebar: bool,
    pub cursor_opacity: u8,
    pub foreground_color: Option<HexColor>,
    pub background_color: Option<HexColor>,
    pub background_opacity: u8,
    pub background_blur: bool,
    pub cursor_color: Option<CellColor>,
    pub cursor_text: Option<CellColor>,
    pub selection_background: Option<CellColor>,
    pub selection_opacity: Option<u8>,
    pub selection_foreground: Option<CellColor>,
    pub ansi_palette: Option<[HexColor; 16]>,
    pub terminal_theme: TerminalTheme,
    pub theme_file: Option<std::path::PathBuf>,
    pub file_editor: Option<crate::external_editor::Editor>,
    #[serde(skip)]
    imported_theme: Option<crate::theme_file::Theme>,
    pub confirm_close: ClosePolicy,
    #[serde(skip)]
    source: Option<std::path::PathBuf>,
    #[serde(skip)]
    required: bool,
    #[serde(skip)]
    policy_override: Option<ClosePolicy>,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            commands: Vec::new(),
            layout_presets: Vec::new(),
            keybindings: Vec::new(),
            restore_session: true,
            session_file: None,
            shell_history_directory: None,
            socket_directory: None,
            new_workspace_placement: crate::workspace_groups::Placement::End,
            group_new_workspace_placement: crate::workspace_groups::Placement::default(),
            font_size: 14.,
            terminal_padding: 8,
            font_family: crate::FONT_FAMILY.into(),
            scrollback_mib: 4,
            copy_on_select: false,
            native_notifications: false,
            notification_focus: NotificationFocus::default(),
            macos_option_as_alt: true,
            unfocused_cursor: UnfocusedCursor::default(),
            focused_cursor: FocusedCursor::default(),
            cursor_blink: false,
            controls_on_hover: false,
            hide_top_bar: false,
            compact_sidebar: false,
            cursor_opacity: 100,
            foreground_color: None,
            background_color: None,
            background_opacity: 100,
            background_blur: false,
            cursor_color: None,
            cursor_text: None,
            selection_background: None,
            selection_opacity: None,
            selection_foreground: None,
            ansi_palette: None,
            terminal_theme: TerminalTheme::default(),
            theme_file: None,
            file_editor: None,
            imported_theme: None,
            confirm_close: ClosePolicy::Active,
            source: None,
            required: false,
            policy_override: None,
        }
    }
}
/// Follow changed defaults only for font attributes still using the previous
/// defaults. Per-terminal size and family overrides are independent.
pub fn updated_font(current_size: f32, current_family: &str, previous_size: f32, previous_family: &str, next: &Preferences) -> (f32, String) {
    let size = if current_size == previous_size { next.font_size } else { current_size };
    let family = if current_family == previous_family { &next.font_family } else { current_family };
    (size, family.to_owned())
}

impl gpui::Global for Preferences {}
#[derive(Default)]
pub struct ReloadState(pub bool);
impl gpui::Global for ReloadState {}

#[derive(Clone)]
pub struct SaveReceipt {
    pub path: PathBuf,
    source: PathBuf,
    changes: Vec<(String, Option<serde_json::Value>, serde_json::Value)>,
}
#[derive(Default)]
pub struct LastSettingsSave(pub Option<SaveReceipt>);
impl gpui::Global for LastSettingsSave {}

fn config_bytes(path: &Path) -> Result<Option<Vec<u8>>> {
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let mut bytes = Vec::new();
    file.take(64 * 1024 + 1).read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= 64 * 1024, "configuration exceeds 64 KiB");
    Ok(Some(bytes))
}

fn replace_config(path: &Path, expected: Option<&[u8]>, bytes: &[u8]) -> Result<()> {
    use std::io::Write;
    Preferences::decode(bytes)?;
    ensure!(bytes.len() < 64 * 1024, "configuration including its final newline exceeds 64 KiB");
    let parent = path.parent().context("Configuration path has no parent")?;
    std::fs::create_dir_all(parent)?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(bytes)?;
    temporary.write_all(b"\n")?;
    temporary.as_file().sync_all()?;
    ensure!(config_bytes(path)?.as_deref() == expected,
        "Settings file changed during this operation; retry after reviewing the file");
    temporary.persist(path)?;
    Ok(())
}

impl SaveReceipt {
    pub fn has_changes(&self) -> bool { !self.changes.is_empty() }
    /// Restore only keys whose current value still equals the saved value.
    pub fn undo(&self) -> Result<(usize, usize)> {
        ensure!(self.source.canonicalize()? == self.path,
            "Settings path now points to a different file; undo was not applied");
        let bytes = config_bytes(&self.path)?.context("Settings file no longer exists")?;
        Preferences::decode(&bytes)?;
        let mut current: serde_json::Value = serde_json::from_slice(&bytes)?;
        let object = current.as_object_mut().context("Settings must be an object")?;
        let mut restored = 0;
        for (key, before, after) in &self.changes {
            if object.get(key) != Some(after) { continue; }
            match before {
                Some(value) => { object.insert(key.clone(), value.clone()); },
                None => { object.remove(key); },
            }
            restored += 1;
        }
        ensure!(restored > 0, "Saved values have changed since the save; nothing was undone");
        replace_config(&self.path, Some(&bytes), &serde_json::to_vec_pretty(&current)?)?;
        Ok((restored, self.changes.len() - restored))
    }
}

/// Last loaded or successfully saved appearance, shared by every window.
/// This is a session baseline, not a claim that the file has not changed externally.
pub struct AppearanceBaseline(pub Preferences);
impl gpui::Global for AppearanceBaseline {}
impl AppearanceBaseline {
    pub fn has_changes(&self, current: &Preferences) -> bool {
        !self.0.same_panel_values(current)
    }
}
impl Preferences {
    pub fn cursor_shape(&self, focused: bool, composing: bool, application: rmux_terminal::CursorShape) -> Option<rmux_terminal::CursorShape> {
        self.unfocused_cursor.shape(focused, composing, self.focused_cursor.shape(application))
    }
    pub fn import_theme(mut self, path: std::path::PathBuf) -> Result<Self> {
        ensure!(path.is_absolute() && path.as_os_str().len() <= 8192, "theme_file must be an absolute path up to 8192 bytes");
        let mut theme = crate::theme_file::read(&path).with_context(|| format!("theme {}", path.display()))?;
        if theme.palette.is_none() && theme.palette_entries.iter().any(Option::is_some) {
            let preset = Self { terminal_theme: self.terminal_theme, ..Self::default() };
            let base = match preset.terminal_palette() {
                Some(colors) => colors,
                None => rmux_terminal::default_ansi_palette(),
            };
            theme.palette = Some(std::array::from_fn(|index| theme.palette_entries[index].unwrap_or_else(|| {
                let rmux_terminal::Color(r, g, b) = base[index];
                HexColor((u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b))
            })));
        }
        self.theme_file = Some(path);
        self.imported_theme = Some(theme);
        Ok(self)
    }
    pub fn select_theme(&mut self, theme: TerminalTheme) {
        self.terminal_theme = theme;
        self.theme_file = None;
        self.imported_theme = None;
    }
    /// Complete defaults for the engine; ANSI JSON overrides only affect slots 0–15.
    pub fn effective_palette(&self) -> Option<[rmux_terminal::Color; 256]> {
        let ansi = self.terminal_palette();
        let imported = self.imported_theme.as_ref();
        if ansi.is_none() && imported.is_none_or(|theme| !theme.palette_entries.iter().any(Option::is_some)) { return None; }
        let mut palette = rmux_terminal::default_palette();
        if let Some(theme) = imported {
            for (slot, entry) in palette.iter_mut().zip(theme.palette_entries.iter()) {
                if let Some(color) = entry { *slot = color.rgb(); }
            }
        }
        if let Some(ansi) = ansi { palette[..16].copy_from_slice(&ansi); }
        Some(palette)
    }
    pub fn terminal_palette(&self) -> Option<[rmux_terminal::Color; 16]> {
        self.ansi_palette.or_else(|| self.imported_theme.as_ref().and_then(|theme| theme.palette)).map(|colors| colors.map(HexColor::rgb)).or_else(|| {
            match self.terminal_theme {
                TerminalTheme::CatppuccinMocha | TerminalTheme::CatppuccinLatte => self.terminal_theme.bundled().and_then(|theme| theme.palette).map(|colors| colors.map(HexColor::rgb)),
                TerminalTheme::Dark => None,
                TerminalTheme::Light => Some([
                    0x24292f, 0xa82020, 0x176b35, 0x805c00,
                    0x0755a8, 0x7839a5, 0x096b72, 0x57606a,
                    0x626b75, 0xc12c2c, 0x24833f, 0x916a00,
                    0x1669bf, 0x9047b5, 0x147e84, 0x6e7781,
                ].map(|value| HexColor(value).rgb())),
                // https://github.com/nordtheme/alacritty/blob/main/src/nord.yaml
                TerminalTheme::Nord => Some([
                    0x3b4252, 0xbf616a, 0xa3be8c, 0xebcb8b,
                    0x81a1c1, 0xb48ead, 0x88c0d0, 0xe5e9f0,
                    0x4c566a, 0xbf616a, 0xa3be8c, 0xebcb8b,
                    0x81a1c1, 0xb48ead, 0x8fbcbb, 0xeceff4,
                ].map(|value| HexColor(value).rgb())),
                // Canonical ANSI slot mapping: https://ethanschoonover.com/solarized/#the-values
                TerminalTheme::SolarizedDark | TerminalTheme::SolarizedLight => Some([
                    0x073642, 0xdc322f, 0x859900, 0xb58900,
                    0x268bd2, 0xd33682, 0x2aa198, 0xeee8d5,
                    0x002b36, 0xcb4b16, 0x586e75, 0x657b83,
                    0x839496, 0x6c71c4, 0x93a1a1, 0xfdf6e3,
                ].map(|value| HexColor(value).rgb())),
            }
        })
    }
    fn cursor_color_setting(&self) -> Option<CellColor> {
        self.cursor_color.or_else(|| self.imported_theme.as_ref().and_then(|theme| theme.cursor_color)).or_else(|| self.terminal_theme.bundled().and_then(|theme| theme.cursor_color))
    }
    pub fn cursor_fill(&self, foreground: rmux_terminal::Color, background: rmux_terminal::Color, engine_color: rmux_terminal::Color, engine_explicit: bool) -> rmux_terminal::Color {
        if engine_explicit { return engine_color; }
        self.cursor_color_setting().map(|setting| setting.resolve(foreground, background)).unwrap_or(engine_color)
    }
    pub fn cursor_text(&self, foreground: rmux_terminal::Color, background: rmux_terminal::Color) -> rmux_terminal::Color {
        let target = self.cursor_text.or_else(|| self.imported_theme.as_ref().and_then(|theme| theme.cursor_text)).or_else(|| self.terminal_theme.bundled().and_then(|theme| theme.cursor_text))
            .unwrap_or(CellColor::CellBackground).resolve(foreground, background);
        // Fade the block's inverted glyph back to the original terminal text.
        // At zero opacity the cursor must not leave an inverted glyph behind.
        let opacity = u32::from(self.cursor_opacity.min(100));
        let blend = |original: u8, target: u8| ((u32::from(original) * (100 - opacity) + u32::from(target) * opacity + 50) / 100) as u8;
        rmux_terminal::Color(blend(foreground.0, target.0), blend(foreground.1, target.1), blend(foreground.2, target.2))
    }
    pub fn cursor_paint(&self, fill: rmux_terminal::Color) -> gpui::Hsla {
        let mut color = crate::color(fill);
        color.a = f32::from(self.cursor_opacity.min(100)) / 100.;
        color
    }
    pub fn selection_foreground(&self) -> Option<CellColor> {
        self.selection_foreground.or_else(|| self.imported_theme.as_ref().and_then(|theme| theme.selection_foreground)).or_else(|| self.terminal_theme.bundled().and_then(|theme| theme.selection_foreground))
    }
    pub fn selection_background_fill(&self, foreground: rmux_terminal::Color, background: rmux_terminal::Color) -> gpui::Hsla {
        let mut fill: gpui::Hsla = self.selection_background().map(|setting| crate::color(setting.resolve(foreground, background))).unwrap_or_else(|| gpui::rgba(0x668fca80).into());
        if let Some(opacity) = self.selection_opacity { fill.a = f32::from(opacity.min(100)) / 100.; }
        fill
    }
    pub fn selection_background(&self) -> Option<CellColor> {
        self.selection_background.or_else(|| self.imported_theme.as_ref().and_then(|theme| theme.selection_background)).or_else(|| self.terminal_theme.bundled().and_then(|theme| theme.selection_background))
    }
    pub fn chrome_palette(&self) -> crate::chrome_style::Palette {
        let pack = |rmux_terminal::Color(r, g, b)| (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b);
        let colors = self.terminal_colors();
        let palette = self.effective_palette();
        let ansi = |index, fallback| palette.map(|colors| pack(colors[index])).unwrap_or(fallback);
        crate::chrome_style::Palette::new(
            pack(colors[1].unwrap_or(rmux_terminal::DEFAULT_BACKGROUND)),
            pack(colors[0].unwrap_or(rmux_terminal::DEFAULT_FOREGROUND)),
            ansi(4, 0xa99be8), ansi(3, 0xf0b85b), ansi(1, 0xf18a8a), ansi(2, 0x9fceb0),
        )
    }

    pub fn terminal_colors(&self) -> [Option<rmux_terminal::Color>; 3] {
        let defaults = match self.terminal_theme {
            TerminalTheme::CatppuccinMocha | TerminalTheme::CatppuccinLatte => self.terminal_theme.bundled().unwrap().colors,
            TerminalTheme::Dark => [None; 3],
            TerminalTheme::Nord => [Some(HexColor(0xd8dee9)), Some(HexColor(0x2e3440)), Some(HexColor(0xd8dee9))],
            TerminalTheme::Light => [Some(HexColor(0x24292f)), Some(HexColor(0xffffff)), Some(HexColor(0x0755a8))],
            TerminalTheme::SolarizedDark => [Some(HexColor(0x839496)), Some(HexColor(0x002b36)), Some(HexColor(0x839496))],
            TerminalTheme::SolarizedLight => [Some(HexColor(0x657b83)), Some(HexColor(0xfdf6e3)), Some(HexColor(0x657b83))],
        };
        let overrides = [self.foreground_color, self.background_color, None];
        let mut colors = std::array::from_fn(|index| overrides[index].or_else(|| self.imported_theme.as_ref().and_then(|theme| theme.colors[index])).or(defaults[index]).map(HexColor::rgb));
        if let Some(setting) = self.cursor_color_setting() {
            // Dynamic colors resolve in the renderer; leave the engine default
            // unset so its explicit-color flag identifies application OSC colors.
            colors[2] = match setting { CellColor::Literal(color) => Some(color.rgb()), _ => None };
        }
        colors
    }
    /// Called only by the explicit Open Settings File UI action.
    pub fn prepare_file(&self) -> Result<std::path::PathBuf> {
        use std::io::Write;
        let path = self
            .source
            .clone()
            .context("No configuration path is available")?;
        if path.try_exists()? {
            ensure!(path.is_file(), "configuration path is not a file");
            return Ok(path);
        }
        let parent = path.parent().context("configuration path has no parent")?;
        std::fs::create_dir_all(parent)?;
        let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
        temporary.write_all(&serde_json::to_vec_pretty(&Self::default())?)?;
        temporary.write_all(b"\n")?;
        temporary.as_file().sync_all()?;
        match temporary.persist_noclobber(&path) {
            Ok(_) => {}
            Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => {
                ensure!(path.is_file(), "configuration path is not a file");
            }
            Err(error) => return Err(error.into()),
        }
        Ok(path)
    }
    pub fn close_policy_overridden(&self) -> bool { self.policy_override.is_some() }

    /// Keep opaque compositing at 100%, even when the blur preference is on.
    pub fn window_background(&self) -> gpui::WindowBackgroundAppearance {
        if self.background_opacity == 100 { gpui::WindowBackgroundAppearance::Opaque }
        else if self.background_blur { gpui::WindowBackgroundAppearance::Blurred }
        else { gpui::WindowBackgroundAppearance::Transparent }
    }
    /// Compare precisely the session values written by Save Settings. A close
    /// policy supplied by the environment is intentionally not persisted.
    pub fn same_panel_values(&self, other: &Self) -> bool {
        self.same_saved_appearance(other)
            && self.foreground_color == other.foreground_color
            && self.background_color == other.background_color
            && self.cursor_color == other.cursor_color
            && self.cursor_text == other.cursor_text
            && self.selection_background == other.selection_background
            && self.selection_foreground == other.selection_foreground
            && self.session_file == other.session_file
            && self.socket_directory == other.socket_directory
            && self.shell_history_directory == other.shell_history_directory
            && self.restore_session == other.restore_session
    }
    pub fn restore_panel(&mut self, baseline: &Self) {
        self.restore_appearance(baseline);
        self.foreground_color = baseline.foreground_color.clone();
        self.background_color = baseline.background_color.clone();
        self.cursor_color = baseline.cursor_color.clone();
        self.cursor_text = baseline.cursor_text.clone();
        self.selection_background = baseline.selection_background.clone();
        self.selection_foreground = baseline.selection_foreground.clone();
        self.session_file = baseline.session_file.clone();
        self.socket_directory = baseline.socket_directory.clone();
        self.shell_history_directory = baseline.shell_history_directory.clone();
        self.restore_session = baseline.restore_session.clone();
    }
    pub fn edit_panel_value(&self, key: &str, value: serde_json::Value) -> Result<Self> {
        ensure!(matches!(key, "foreground_color" | "background_color" | "cursor_color" | "cursor_text"
            | "selection_background" | "selection_foreground" | "font_family" | "session_file"
            | "socket_directory" | "shell_history_directory" | "restore_session"), "Setting is not editable here");
        let mut object = serde_json::to_value(self)?;
        object[key] = value;
        let mut next = Self::decode(&serde_json::to_vec(&object)?)?;
        next.source = self.source.clone();
        next.required = self.required;
        next.policy_override = self.policy_override;
        next.imported_theme = self.imported_theme.clone();
        Ok(next)
    }
    /// Merge only this draft's changes. Untouched settings edited on disk are
    /// preserved; conflicting edits to the same setting are reported.
    pub fn save_panel_with_receipt(&self, baseline: &Self) -> Result<SaveReceipt> {
        let source = self.source.clone().context("No configuration path is available")?;
        let path = match std::fs::symlink_metadata(&source) {
            Ok(_) => source.canonicalize().context("Could not resolve configuration target")?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => source.clone(),
            Err(error) => return Err(error.into()),
        };
        let before = config_bytes(&path)?;
        let latest = before.as_deref().map(Self::decode).transpose()?.unwrap_or_default();
        let old_values = serde_json::to_value(&latest)?;
        let base_values = serde_json::to_value(baseline)?;
        let mut staged = baseline.clone();
        staged.restore_panel(self);
        let draft_values = serde_json::to_value(&staged)?;
        let mut merged: serde_json::Map<String, serde_json::Value> = before.as_deref()
            .map(serde_json::from_slice).transpose()?.unwrap_or_default();
        let mut changes = Vec::new();
        for (key, value) in draft_values.as_object().context("Invalid settings object")? {
            if base_values.get(key) == Some(value) { continue; }
            ensure!(old_values.get(key) == base_values.get(key) || old_values.get(key) == Some(value),
                "{key} changed in the settings file. Reload Settings before saving this value.");
            if merged.get(key) != Some(value) {
                let prior = merged.insert(key.clone(), value.clone());
                changes.push((key.clone(), prior, value.clone()));
            }
        }
        let bytes = serde_json::to_vec_pretty(&merged)?;
        Self::decode(&bytes)?;
        replace_config(&path, before.as_deref(), &bytes)?;
        Ok(SaveReceipt { path: path.canonicalize()?, source, changes })
    }

    pub fn same_saved_appearance(&self, other: &Self) -> bool {
        self.font_size == other.font_size
            && self.font_family == other.font_family
            && self.background_opacity == other.background_opacity
            && self.background_blur == other.background_blur
            && self.terminal_padding == other.terminal_padding
            && self.scrollback_mib == other.scrollback_mib
            && self.terminal_theme == other.terminal_theme
            && self.theme_file == other.theme_file
            && self.unfocused_cursor == other.unfocused_cursor
            && self.focused_cursor == other.focused_cursor
            && self.cursor_blink == other.cursor_blink
            && self.controls_on_hover == other.controls_on_hover
            && self.hide_top_bar == other.hide_top_bar
            && self.compact_sidebar == other.compact_sidebar
            && self.cursor_opacity == other.cursor_opacity
            && self.selection_opacity == other.selection_opacity
            && self.macos_option_as_alt == other.macos_option_as_alt
            && self.copy_on_select == other.copy_on_select
            && self.native_notifications == other.native_notifications
            && self.notification_focus == other.notification_focus
            && self.new_workspace_placement == other.new_workspace_placement
            && self.group_new_workspace_placement == other.group_new_workspace_placement
            && (self.close_policy_overridden() || self.confirm_close == other.confirm_close)
    }

    /// Restore panel-controlled values without replacing commands, bindings,
    /// paths, explicit color overrides, or the environment's close policy.
    pub fn restore_appearance(&mut self, baseline: &Self) {
        self.font_size = baseline.font_size;
        self.font_family = baseline.font_family.clone();
        self.background_opacity = baseline.background_opacity;
        self.background_blur = baseline.background_blur;
        self.terminal_padding = baseline.terminal_padding;
        self.scrollback_mib = baseline.scrollback_mib;
        self.terminal_theme = baseline.terminal_theme;
        self.theme_file = baseline.theme_file.clone();
        self.unfocused_cursor = baseline.unfocused_cursor;
        self.focused_cursor = baseline.focused_cursor;
        self.cursor_blink = baseline.cursor_blink;
        self.controls_on_hover = baseline.controls_on_hover;
        self.hide_top_bar = baseline.hide_top_bar;
        self.compact_sidebar = baseline.compact_sidebar;
        self.cursor_opacity = baseline.cursor_opacity;
        self.selection_opacity = baseline.selection_opacity;
        self.macos_option_as_alt = baseline.macos_option_as_alt;
        self.copy_on_select = baseline.copy_on_select;
        self.native_notifications = baseline.native_notifications;
        self.notification_focus = baseline.notification_focus;
        self.new_workspace_placement = baseline.new_workspace_placement;
        self.group_new_workspace_placement = baseline.group_new_workspace_placement;
        if !self.close_policy_overridden() && !baseline.close_policy_overridden() { self.confirm_close = baseline.confirm_close; }
        self.imported_theme = baseline.imported_theme.clone();
    }

    #[cfg(test)]
    pub fn save_appearance(&self) -> Result<std::path::PathBuf> {
        Ok(self.save_appearance_with_receipt()?.path)
    }
    #[cfg(test)]
    pub fn save_appearance_with_receipt(&self) -> Result<SaveReceipt> {
        let path = self.source.clone().context("No configuration path is available")?;
        // Inspect the link itself: exists()/try_exists() follow it and mistake
        // a dangling user-managed symlink for a new configuration file.
        let path = match std::fs::symlink_metadata(&path) {
            Ok(_) => path.canonicalize().context("Could not resolve configuration target; existing link was left unchanged")?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => path,
            Err(error) => return Err(error.into()),
        };
        let before = config_bytes(&path)?;
        let mut latest = before.as_deref().map(Self::decode).transpose()?.unwrap_or_default();
        latest.restore_appearance(self);
        let bytes = serde_json::to_vec_pretty(&latest)?;
        let old: serde_json::Map<String, serde_json::Value> = before.as_deref()
            .map(serde_json::from_slice).transpose()?.unwrap_or_default();
        let new: serde_json::Map<String, serde_json::Value> = serde_json::from_slice(&bytes)?;
        let changes = new.into_iter().filter(|(key, value)| old.get(key) != Some(value))
            .map(|(key, value)| { let prior = old.get(&key).cloned(); (key, prior, value) }).collect();
        replace_config(&path, before.as_deref(), &bytes)?;
        Ok(SaveReceipt { path: path.canonicalize()?, source: self.source.clone().unwrap(), changes })
    }

    pub fn scrollback_bytes(&self) -> usize {
        usize::from(self.scrollback_mib) * 1024 * 1024
    }
    pub fn reload(&self) -> Result<Self> {
        let mut next = match &self.source {
            Some(path) => Self::read(path, self.required)?,
            None => Self::default(),
        };
        if let Some(policy) = self.policy_override {
            next.confirm_close = policy;
        }
        next.source = self.source.clone();
        next.required = self.required;
        next.policy_override = self.policy_override;
        Ok(next)
    }
    fn decode(bytes: &[u8]) -> Result<Self> {
        ensure!(bytes.len() <= 64 * 1024, "configuration exceeds 64 KiB");
        let value: Self = serde_json::from_slice(bytes)?;
        ensure!(value.layout_presets.len() <= 32, "at most 32 layout presets are allowed");
        let mut preset_names = std::collections::HashSet::new();
        for (index, preset) in value.layout_presets.iter().enumerate() {
            (|| -> Result<()> {
                ensure!(!preset.name.trim().is_empty() && preset.name.len() <= 128 && !preset.name.chars().any(char::is_control), "preset name must be nonempty, at most 128 bytes and contain no controls");
                ensure!(preset_names.insert(preset.name.trim().to_lowercase()), "duplicate layout preset name");
                ensure!(preset.path.is_absolute() && preset.path.as_os_str().len() <= 8192 && !preset.path.to_string_lossy().chars().any(char::is_control), "preset path must be absolute, at most 8192 bytes and contain no controls");
                ensure!(preset.cwd.as_ref().is_none_or(|path| path.is_absolute() && path.as_os_str().len() <= 8192 && !path.to_string_lossy().chars().any(char::is_control)), "preset cwd must be absolute, at most 8192 bytes and contain no controls");
                Ok(())
            })().with_context(|| format!("layout_presets[{index}] ({:?})", preset.name))?;
        }
        ensure!(value.commands.len() <= 32, "at most 32 custom commands are allowed");
        let mut names = std::collections::HashSet::new();
        for (index, command) in value.commands.iter().enumerate() {
            (|| -> Result<()> {
                ensure!(!command.name.trim().is_empty() && command.name.len() <= 128 && !command.name.chars().any(char::is_control), "command name must be nonempty, at most 128 bytes and contain no controls");
                ensure!(names.insert(command.name.trim().to_lowercase()), "duplicate custom command name");
                ensure!(!command.argv.is_empty() && command.argv.len() <= 64 && !command.argv[0].is_empty(), "command argv must contain a program and at most 64 entries");
                ensure!(command.argv.iter().all(|arg| !arg.contains('\0')) && command.argv.iter().map(String::len).sum::<usize>() <= 16 * 1024, "command arguments must fit 16 KiB without NULs");
                ensure!(command.env.len() <= 64 && command.env.iter().map(|(key, value)| key.len() + value.len()).sum::<usize>() <= 16 * 1024, "command environment must fit 64 entries and 16 KiB");
                for (key, value) in &command.env {
                    ensure!(!key.is_empty() && key.len() <= 128 && key.bytes().enumerate().all(|(i, byte)| byte == b'_' || byte.is_ascii_alphabetic() || (i > 0 && byte.is_ascii_digit())) && !value.contains('\0'), "command environment requires portable variable names and values without NULs");
                }
                #[cfg(windows)]
                {
                    let mut keys = std::collections::HashSet::new();
                    ensure!(command.env.keys().all(|key| keys.insert(key.to_ascii_lowercase())), "duplicate case-insensitive environment variable");
                }
                ensure!(command.cwd.as_ref().is_none_or(|path| path.is_absolute() && path.as_os_str().len() <= 8192), "command cwd must be an absolute path up to 8192 bytes");
                Ok(())
            })().with_context(|| format!("commands[{index}] ({:?})", command.name))?;
        }
        if let Some(editor) = &value.file_editor { editor.validate().context("file_editor")?; }
        ensure!(value.theme_file.as_ref().is_none_or(|path| path.is_absolute() && path.as_os_str().len() <= 8192), "theme_file must be an absolute path up to 8192 bytes");
        ensure!(value.terminal_padding <= 32, "terminal_padding must be between 0 and 32");
        ensure!(value.shell_history_directory.as_ref().is_none_or(|path| path.is_absolute() && path.as_os_str().len() <= 8192), "shell_history_directory must be an absolute path up to 8192 bytes");
        ensure!(value.session_file.as_ref().is_none_or(|path| path.is_absolute() && path.as_os_str().len() <= 8192), "session_file must be an absolute path up to 8192 bytes");
        ensure!(value.socket_directory.as_ref().is_none_or(|path| path.is_absolute() && path.as_os_str().len() <= 8192), "socket_directory must be an absolute path up to 8192 bytes");
        ensure!(
            value.scrollback_mib <= 256,
            "scrollback_mib must be between 0 and 256"
        );
        ensure!(
            !value.font_family.trim().is_empty()
                && value.font_family.len() <= 128
                && !value.font_family.chars().any(char::is_control),
            "font_family must be a nonempty name up to 128 bytes without controls"
        );
        ensure!(
            value.font_size.is_finite() && (8. ..=40.).contains(&value.font_size),
            "font_size must be between 8 and 40"
        );
        ensure!(value.background_opacity <= 100, "background_opacity must be between 0 and 100");
        ensure!(value.selection_opacity.is_none_or(|opacity| opacity <= 100), "selection_opacity must be null or between 0 and 100");
        ensure!(value.cursor_opacity <= 100, "cursor_opacity must be between 0 and 100");
        crate::shortcuts::compile(&value)?;
        Ok(value)
    }
    fn config_bytes(path: &Path, required: bool) -> Result<Option<Vec<u8>>> {
        let file = match std::fs::File::open(path) {
            Err(error) if !required && error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            result => result.with_context(|| format!("configuration {}", path.display()))?,
        };
        let mut bytes = Vec::new();
        file.take(64 * 1024 + 1).read_to_end(&mut bytes)?;
        ensure!(bytes.len() <= 64 * 1024, "configuration exceeds 64 KiB");
        Ok(Some(bytes))
    }
    fn read_config(path: &Path, required: bool) -> Result<Self> {
        match Self::config_bytes(path, required)? {
            Some(bytes) => Self::decode(&bytes).with_context(|| format!("configuration {}", path.display())),
            None => Ok(Self::default()),
        }
    }
    /// Only this optional integration may be disabled during startup. Other
    /// configuration errors and explicit reloads retain strict validation.
    fn decode_startup(bytes: &[u8]) -> Result<(Self, Option<String>)> {
        ensure!(bytes.len() <= 64 * 1024, "configuration exceeds 64 KiB");
        let mut json: serde_json::Value = serde_json::from_slice(bytes)?;
        let invalid = json.get("file_editor").filter(|value| !value.is_null()).and_then(|value| {
            serde_json::from_value::<crate::external_editor::Editor>(value.clone())
                .map_err(anyhow::Error::from).and_then(|editor| editor.validate()).err()
        });
        let Some(error) = invalid else { return Self::decode(bytes).map(|value| (value, None)); };
        json.as_object_mut().context("configuration must be an object")?.remove("file_editor");
        let value = Self::decode(&serde_json::to_vec(&json)?)?;
        Ok((value, Some(format!("External editor disabled: {error}. Open Settings → Edit settings without configured editor to repair file_editor, then Reload Settings. Your configuration file was not changed."))))
    }
    fn read(path: &Path, required: bool) -> Result<Self> {
        let value = Self::read_config(path, required)?;
        if let Some(path) = value.theme_file.clone() { value.import_theme(path) } else { Ok(value) }
    }
    /// Keep recovery beside the selected config, isolating alternate profiles
    /// and test launches from the user's normal saved workspace.
    pub fn recovery_path(&self) -> Option<std::path::PathBuf> {
        if !self.restore_session { return None; }
        self.session_file.clone().or_else(|| self.source.as_ref().map(|path| path.with_extension("session.json")))
    }
    pub fn load() -> Result<(Self, Option<String>)> {
        let explicit = std::env::var_os("RMUX_CONFIG");
        let path = if let Some(path) = &explicit {
            ensure!(!path.is_empty(), "RMUX_CONFIG must not be empty");
            Some(std::path::PathBuf::from(path))
        } else if cfg!(windows) {
            std::env::var_os("APPDATA").map(|root| Path::new(&root).join("rmux/config.json"))
        } else {
            std::env::var_os("XDG_CONFIG_HOME")
                .filter(|root| !root.is_empty())
                .map(std::path::PathBuf::from)
                .or_else(|| std::env::var_os("HOME").map(|root| Path::new(&root).join(".config")))
                .map(|root| root.join("rmux/config.json"))
        };
        let mut preferences = Self {
            source: path.map(std::path::absolute).transpose()?,
            required: explicit.is_some(),
            ..Self::default()
        };
        if let Some(policy) = std::env::var_os("RMUX_CONFIRM_CLOSE") {
            preferences.policy_override = Some(
                ClosePolicy::parse(Some(policy.to_str().unwrap_or("invalid")))
                    .map_err(anyhow::Error::msg)?,
            );
        }
        let (mut next, notice) = match &preferences.source {
            Some(path) => match Self::config_bytes(path, preferences.required)? {
                Some(bytes) => Self::decode_startup(&bytes).with_context(|| format!("configuration {}", path.display()))?,
                None => (Self::default(), None),
            },
            None => (Self::default(), None),
        };
        if let Some(path) = next.theme_file.clone() { next = next.import_theme(path)?; }
        if let Some(policy) = preferences.policy_override { next.confirm_close = policy; }
        next.source = preferences.source;
        next.required = preferences.required;
        next.policy_override = preferences.policy_override;
        Ok((next, notice))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn panel_save_merges_only_draft_changes_and_undo_preserves_external_edits() -> Result<()> {
        let root = tempfile::tempdir()?;
        let path = root.path().join("config.json");
        std::fs::write(&path, br##"{"font_size":13,"foreground_color":"#ffffff"}"##)?;
        let mut baseline = Preferences::read(&path, true)?;
        baseline.source = Some(path.clone());
        let draft = baseline.edit_panel_value("foreground_color", serde_json::json!("#aabbcc"))?;
        std::fs::write(&path, br##"{"font_size":18,"foreground_color":"#ffffff"}"##)?;
        let receipt = draft.save_panel_with_receipt(&baseline)?;
        let saved = Preferences::read(&path, true)?;
        assert_eq!(saved.font_size, 18.);
        assert_eq!(saved.foreground_color, draft.foreground_color);
        receipt.undo()?;
        let restored = Preferences::read(&path, true)?;
        assert_eq!(restored.font_size, 18.);
        assert_eq!(restored.foreground_color, baseline.foreground_color);
        Ok(())
    }

    #[test]
    fn panel_save_rejects_same_field_conflicts_without_overwriting() -> Result<()> {
        let root = tempfile::tempdir()?;
        let path = root.path().join("config.json");
        std::fs::write(&path, b"{}")?;
        let mut baseline = Preferences::read(&path, true)?;
        baseline.source = Some(path.clone());
        let draft = baseline.edit_panel_value("font_family", serde_json::json!("Monaco"))?;
        let external = br#"{"font_family":"Courier"}"#;
        std::fs::write(&path, external)?;
        assert!(draft.save_panel_with_receipt(&baseline).err().expect("same-field conflict").to_string().contains("font_family changed"));
        assert_eq!(std::fs::read(&path)?, external);
        Ok(())
    }
    #[test]
    fn chrome_theme_follows_presets_and_explicit_background() -> Result<()> {
        for theme in TerminalTheme::ALL {
            let settings = Preferences::decode(&serde_json::to_vec(&serde_json::json!({"terminal_theme": theme}))?)?;
            let rmux_terminal::Color(r, g, b) = settings.terminal_colors()[1].unwrap_or(rmux_terminal::DEFAULT_BACKGROUND);
            let background = (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b);
            assert_eq!(settings.chrome_palette().surface, background);
        }
        let settings = Preferences::decode(br##"{"terminal_theme":"nord","background_color":"#fff1dd"}"##)?;
        assert_eq!(settings.chrome_palette().surface, 0xfff1dd);
        Ok(())
    }

    #[test]
    fn startup_disables_only_invalid_editor_without_changing_config() -> anyhow::Result<()> {
        use super::Preferences;
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("config.json");
        for editor in [serde_json::json!({"program":"code","args":["{file}"]}),
            serde_json::json!({"program":12}), serde_json::json!({"program":"/bin/echo","args":[]})] {
            let bytes = serde_json::to_vec(&serde_json::json!({"font_size":19,"file_editor":editor}))?;
            std::fs::write(&path, &bytes)?;
            let (mut loaded, notice) = Preferences::decode_startup(&bytes)?;
            assert_eq!(loaded.font_size, 19.);
            assert!(loaded.file_editor.is_none());
            assert!(notice.unwrap().contains("External editor disabled"));
            loaded.source = Some(path.clone());
            assert_eq!(loaded.prepare_file()?, path);
            assert_eq!(std::fs::read(&path)?, bytes);
            assert!(loaded.reload().is_err());
            std::fs::write(&path, b"{\"font_size\":20}")?;
            assert_eq!(loaded.reload()?.font_size, 20.);
        }
        assert!(Preferences::decode_startup(br#"{"font_size":0,"file_editor":{"program":"code","args":[]}}"#).is_err());
        assert!(Preferences::decode_startup(b"invalid json").is_err());
        assert!(Preferences::decode_startup(b"{}").unwrap().1.is_none());
        Ok(())
    }
    #[test]
    fn socket_restart_notice_survives_reloads_and_clears_on_revert() {
        let original = Some(std::env::temp_dir().join("rmux-original-socket"));
        let changed = Some(std::env::temp_dir().join("rmux-new-socket"));
        let startup = super::StartupSocket { configured: original.clone(), environment_override: false };
        assert_eq!(startup.reload_notice(&original), None);
        for _ in 0..3 {
            assert!(startup.reload_notice(&changed).unwrap().starts_with("Restart rmux"));
        }
        assert!(startup.reload_notice(&None).is_some());
        assert_eq!(startup.reload_notice(&original), None);
        let overridden = super::StartupSocket { configured: original, environment_override: true };
        assert!(overridden.reload_notice(&changed).unwrap().contains("RMUX_SOCKET_DIR"));
        let disabled = super::StartupSocket { configured: None, environment_override: false };
        assert!(disabled.reload_notice(&changed).is_some());
        assert_eq!(disabled.reload_notice(&None), None);
    }
    #[test]
    fn settings_write_limit_includes_the_trailing_newline() -> anyhow::Result<()> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("config.json");
        let mut bytes = b"{}".to_vec();
        bytes.resize(64 * 1024 - 1, b' ');
        super::replace_config(&path, None, &bytes)?;
        let saved = std::fs::read(&path)?;
        assert_eq!(saved.len(), 64 * 1024);
        super::Preferences::read_config(&path, true)?;
        bytes.push(b' ');
        let error = super::replace_config(&path, Some(&saved), &bytes).unwrap_err();
        assert!(error.to_string().contains("final newline"));
        assert_eq!(std::fs::read(&path)?, saved);
        Ok(())
    }

    #[test]
    fn settings_validation_identifies_the_failing_entry_without_command_contents() {
        let commands = br#"{"commands":[{"name":"Valid","argv":["echo"]},{"name":"Build","argv":[],"env":{"TOKEN":"private-value"}}]}"#;
        let error = format!("{:#}", super::Preferences::decode(commands).err().unwrap());
        assert!(error.contains("commands[1] (\"Build\")"), "{error}");
        assert!(error.contains("argv"), "{error}");
        assert!(!error.contains("private-value"));
        let presets = br#"{"layout_presets":[{"name":"Review","path":"relative.json"}]}"#;
        let error = format!("{:#}", super::Preferences::decode(presets).err().unwrap());
        assert!(error.contains("layout_presets[0] (\"Review\")"), "{error}");
        assert!(error.contains("absolute"), "{error}");
    }

    #[test]
    fn settings_undo_of_first_save_leaves_a_valid_default_file() -> anyhow::Result<()> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("config.json");
        let mut settings = super::Preferences::default();
        settings.source = Some(path.clone());
        settings.font_size = 20.;
        let receipt = settings.save_appearance_with_receipt()?;
        let (restored, skipped) = receipt.undo()?;
        assert!(restored > 0);
        assert_eq!(skipped, 0);
        assert_eq!(serde_json::from_slice::<serde_json::Value>(&std::fs::read(&path)?)?, serde_json::json!({}));
        assert_eq!(super::Preferences::read_config(&path, true)?.font_size, super::Preferences::default().font_size);
        Ok(())
    }
    #[test]
    fn settings_undo_preserves_later_edits_and_restores_missing_keys() -> anyhow::Result<()> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("config.json");
        let original = serde_json::json!({"font_size": 13, "terminal_padding": 4});
        std::fs::write(&path, serde_json::to_vec(&original)?)?;
        let mut settings = super::Preferences::read_config(&path, true)?;
        settings.source = Some(path.clone());
        settings.font_size = 16.;
        settings.terminal_padding = 8;
        let receipt = settings.save_appearance_with_receipt()?;
        let mut edited: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
        edited["font_size"] = 19.into();
        std::fs::write(&path, serde_json::to_vec(&edited)?)?;
        let (restored, skipped) = receipt.undo()?;
        assert!(restored > 0);
        assert_eq!(skipped, 1);
        let undone: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
        assert_eq!(undone, serde_json::json!({"font_size":19,"terminal_padding":4}));
        assert!(receipt.undo().is_err());
        assert_eq!(serde_json::from_slice::<serde_json::Value>(&std::fs::read(&path)?)?, undone);
        Ok(())
    }

    #[test]
    fn settings_undo_rejects_invalid_or_retargeted_files_and_save_conflicts() -> anyhow::Result<()> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("config.json");
        let mut settings = super::Preferences::default();
        settings.source = Some(path.clone());
        let receipt = settings.save_appearance_with_receipt()?;
        std::fs::write(&path, b"invalid external edit")?;
        assert!(receipt.undo().is_err());
        assert_eq!(std::fs::read(&path)?, b"invalid external edit");
        assert!(super::replace_config(&path, Some(b"{}"), b"{}").is_err());
        assert_eq!(std::fs::read(&path)?, b"invalid external edit");
        #[cfg(unix)] {
            let other = directory.path().join("other.json");
            std::fs::write(&other, b"{}")?;
            std::fs::remove_file(&path)?;
            std::os::unix::fs::symlink(&other, &path)?;
            assert!(receipt.undo().is_err());
            assert_eq!(std::fs::read(&other)?, b"{}");
        }
        Ok(())
    }
    use super::*;
    #[test]
    fn sparse_theme_palette_uses_preset_not_previous_import() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("theme");
        std::fs::write(&path, "palette=1=010203\n")?;
        for theme in [TerminalTheme::Dark, TerminalTheme::Nord] {
            let original = Preferences { terminal_theme: theme, ..Preferences::default() };
            let defaults = original.terminal_palette().unwrap_or(rmux_terminal::default_ansi_palette());
            let first = original.import_theme(path.clone())?;
            let colors = first.terminal_palette().unwrap();
            assert_eq!(colors[1], rmux_terminal::Color(1,2,3));
            assert_eq!(colors[2], defaults[2]);
            std::fs::write(&path, "palette=2=040506\n")?;
            let second = first.import_theme(path.clone())?;
            assert_eq!(second.terminal_palette().unwrap()[1], defaults[1]);
            assert_eq!(second.terminal_palette().unwrap()[2], rmux_terminal::Color(4,5,6));
            std::fs::write(&path, "palette=1=010203\n")?;
        }
        Ok(())
    }

    #[test]
    fn dynamic_cursor_fill_preserves_application_color_priority() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("theme");
        std::fs::write(&path, "cursor-color=cell-background\n")?;
        let imported = Preferences::default().import_theme(path.clone())?;
        let fg = rmux_terminal::Color(1,2,3);
        let bg = rmux_terminal::Color(4,5,6);
        let app = rmux_terminal::Color(7,8,9);
        assert_eq!(imported.terminal_colors()[2], None);
        assert_eq!(imported.cursor_fill(fg, bg, app, false), bg);
        assert_eq!(imported.cursor_fill(fg, bg, app, true), app);
        let explicit = Preferences::decode(br#"{"cursor_color":"cell-foreground","terminal_theme":"nord"}"#)?.import_theme(path)?;
        assert_eq!(explicit.terminal_colors()[2], None);
        assert_eq!(explicit.cursor_fill(fg, bg, app, false), fg);
        assert_eq!(serde_json::to_value(&explicit)?["cursor_color"], "cell-foreground");
        Ok(())
    }

    #[test]
    fn dynamic_cursor_text_tracks_cells_and_explicit_override() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("theme");
        std::fs::write(&path, "cursor-text=cell-foreground\n")?;
        let imported = Preferences::default().import_theme(path.clone())?;
        for fg in [rmux_terminal::Color(1,2,3), rmux_terminal::Color(7,8,9)] {
            let bg = rmux_terminal::Color(4,5,6);
            assert_eq!(imported.cursor_text(fg, bg), fg);
            let explicit = Preferences::decode(br#"{"cursor_text":"cell-background"}"#)?.import_theme(path.clone())?;
            assert_eq!(explicit.cursor_text(fg, bg), bg);
            assert_eq!(serde_json::to_value(&explicit)?["cursor_text"], "cell-background");
        }
        assert!(Preferences::decode(br#"{"cursor_text":"unknown-reference"}"#).is_err());
        Ok(())
    }

    #[test]
    fn cursor_text_import_override_and_preset_fallback() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("theme");
        std::fs::write(&path, "cursor-text=112233\n")?;
        let fg = rmux_terminal::Color(1,2,3);
        let bg = rmux_terminal::Color(4,5,6);
        assert_eq!(Preferences::default().cursor_text(fg, bg), bg);
        let mut imported = Preferences::default().import_theme(path.clone())?;
        assert_eq!(imported.cursor_text(fg, bg), rmux_terminal::Color(17,34,51));
        imported.select_theme(TerminalTheme::Dark);
        assert_eq!(imported.cursor_text(fg, bg), bg);
        let explicit = Preferences::decode(br##"{"cursor_text":"#abcdef"}"##)?.import_theme(path)?;
        assert_eq!(explicit.cursor_text(fg, bg), rmux_terminal::Color(171,205,239));
        assert_eq!(serde_json::to_value(&explicit)?["cursor_text"], "#abcdef");
        Ok(())
    }

    #[test]
    fn extended_palette_import_override_and_replacement() -> Result<()> {
        use rmux_terminal::Color;
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("theme");
        std::fs::write(&path, "palette=1=010203\npalette=200=112233\npalette=255=abcdef\n")?;
        let settings = Preferences::decode(&serde_json::to_vec(&serde_json::json!({"ansi_palette": vec!["#445566";16]}))?)?.import_theme(path.clone())?;
        let colors = settings.effective_palette().unwrap();
        assert_eq!(colors[1], Color(68,85,102));
        assert_eq!(colors[200], Color(17,34,51));
        assert_eq!(colors[255], Color(171,205,239));
        assert_eq!(colors[199], rmux_terminal::default_palette()[199]);
        std::fs::write(&path, "palette=16=123456\n")?;
        let mut replaced = settings.import_theme(path)?;
        assert_eq!(replaced.effective_palette().unwrap()[200], rmux_terminal::default_palette()[200]);
        assert_eq!(replaced.effective_palette().unwrap()[16], Color(18,52,86));
        replaced.select_theme(TerminalTheme::Dark);
        assert_eq!(replaced.effective_palette().unwrap()[16], rmux_terminal::default_palette()[16]);
        assert!(Preferences::default().effective_palette().is_none());
        Ok(())
    }

    #[test]
    fn dynamic_selection_foreground_import_override_and_round_trip() -> Result<()> {
        use rmux_terminal::Color;
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("theme");
        std::fs::write(&path, "selection-foreground=cell-background\n")?;
        let imported = Preferences::default().import_theme(path.clone())?;
        let fg = Color(1,2,3);
        let bg = Color(4,5,6);
        assert_eq!(imported.selection_foreground().unwrap().resolve(fg, bg), bg);
        let explicit = Preferences::decode(br#"{"selection_foreground":"cell-foreground"}"#)?.import_theme(path)?;
        assert_eq!(explicit.selection_foreground().unwrap().resolve(fg, bg), fg);
        assert_eq!(serde_json::to_value(&explicit)?["selection_foreground"], "cell-foreground");
        assert!(Preferences::decode(br#"{"selection_foreground":"cell-invalid"}"#).is_err());
        Ok(())
    }

    #[test]
    fn dynamic_selection_background_import_and_override() -> Result<()> {
        use rmux_terminal::Color;
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("theme");
        std::fs::write(&path, "selection-background=cell-foreground\n")?;
        let imported = Preferences::default().import_theme(path.clone())?;
        let fg = Color(1,2,3);
        let bg = Color(4,5,6);
        assert_eq!(imported.selection_background().unwrap().resolve(fg, bg), fg);
        let explicit = Preferences::decode(br#"{"selection_background":"cell-background"}"#)?.import_theme(path)?;
        assert_eq!(explicit.selection_background().unwrap().resolve(fg, bg), bg);
        assert_eq!(serde_json::to_value(&explicit)?["selection_background"], "cell-background");
        assert!(Preferences::decode(br#"{"selection_background":"cell-invalid"}"#).is_err());
        Ok(())
    }

    #[test]
    fn selection_colors_import_override_and_preset_fallback() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("theme");
        std::fs::write(&path, "selection-background=112233\nselection-foreground=445566\n")?;
        assert_eq!(Preferences::default().selection_background().map(|color| color.resolve(rmux_terminal::Color(0,0,0), rmux_terminal::Color(0,0,0))), None);
        assert_eq!(Preferences::default().selection_foreground().map(|color| color.resolve(rmux_terminal::Color(0,0,0), rmux_terminal::Color(0,0,0))), None);
        let mut imported = Preferences::default().import_theme(path.clone())?;
        assert_eq!(imported.selection_background().map(|color| color.resolve(rmux_terminal::Color(0,0,0), rmux_terminal::Color(0,0,0))), Some(rmux_terminal::Color(17,34,51)));
        assert_eq!(imported.selection_foreground().map(|color| color.resolve(rmux_terminal::Color(0,0,0), rmux_terminal::Color(0,0,0))), Some(rmux_terminal::Color(68,85,102)));
        imported.select_theme(TerminalTheme::Dark);
        assert_eq!(imported.selection_background().map(|color| color.resolve(rmux_terminal::Color(0,0,0), rmux_terminal::Color(0,0,0))), None);
        assert_eq!(imported.selection_foreground().map(|color| color.resolve(rmux_terminal::Color(0,0,0), rmux_terminal::Color(0,0,0))), None);
        let explicit = Preferences::decode(br##"{"selection_background":"#abcdef","selection_foreground":"#010203"}"##)?.import_theme(path)?;
        assert_eq!(explicit.selection_background().map(|color| color.resolve(rmux_terminal::Color(0,0,0), rmux_terminal::Color(0,0,0))), Some(rmux_terminal::Color(171,205,239)));
        assert_eq!(explicit.selection_foreground().map(|color| color.resolve(rmux_terminal::Color(0,0,0), rmux_terminal::Color(0,0,0))), Some(rmux_terminal::Color(1,2,3)));
        assert_eq!(serde_json::to_value(&explicit)?["selection_foreground"], "#010203");
        assert_eq!(serde_json::to_value(&explicit)?["selection_background"], "#abcdef");
        Ok(())
    }

    #[test]
    fn theme_file_load_reload_overrides_and_preset_save() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let theme = directory.path().join("colors.theme");
        let config = directory.path().join("config.json");
        std::fs::write(&theme, "foreground=112233\nbackground=445566\ncursor-color=778899\n")?;
        std::fs::write(&config, serde_json::to_vec(&serde_json::json!({"theme_file":theme, "foreground_color":"#aabbcc"}))?)?;
        let imported = Preferences::default().import_theme(theme.clone())?;
        assert_eq!(imported.theme_file.as_ref(), Some(&theme));
        assert_eq!(imported.terminal_colors()[0], Some(rmux_terminal::Color(17,34,51)));
        assert!(Preferences::default().import_theme("relative.theme".into()).is_err());
        let mut settings = Preferences::read(&config, true)?;
        settings.source = Some(config.clone());
        assert_eq!(settings.terminal_colors(), [Some(rmux_terminal::Color(170,187,204)), Some(rmux_terminal::Color(68,85,102)), Some(rmux_terminal::Color(119,136,153))]);
        settings.save_appearance()?;
        assert_eq!(Preferences::read(&config, true)?.theme_file.as_ref(), Some(&theme));
        std::fs::write(&theme, "background=010203\n")?;
        let updated = settings.reload()?;
        assert_eq!(updated.terminal_colors()[1], Some(rmux_terminal::Color(1,2,3)));
        std::fs::write(&theme, "background=invalid\n")?;
        assert!(settings.reload().is_err());
        assert_eq!(settings.terminal_colors()[1], Some(rmux_terminal::Color(68,85,102)));
        // A broken imported file must not prevent saving a built-in replacement.
        settings.select_theme(TerminalTheme::Nord);
        assert_eq!(settings.terminal_colors()[1], Some(rmux_terminal::Color(46,52,64)));
        settings.save_appearance()?;
        assert!(Preferences::read(&config, true)?.theme_file.is_none());
        assert!(Preferences::decode(br#"{"theme_file":"relative.theme"}"#).is_err());
        Ok(())
    }

    #[test]
    fn background_appearance_controls_validate_cycle_and_persist() -> Result<()> {
        use gpui::WindowBackgroundAppearance::{Opaque, Transparent, Blurred};
        let mut settings = Preferences::decode(b"{}")?;
        assert_eq!(settings.background_opacity, 100);
        assert_eq!(settings.window_background(), Opaque);
        crate::settings_view::adjust(&mut settings, "opacity", -1);
        assert_eq!(settings.background_opacity, 95);
        assert_eq!(settings.window_background(), Transparent);
        crate::settings_view::adjust(&mut settings, "blur", 1);
        assert_eq!(settings.window_background(), Blurred);
        assert!(!settings.same_saved_appearance(&Preferences::default()));
        for _ in 0..30 { crate::settings_view::adjust(&mut settings, "opacity", -1); }
        assert_eq!(settings.background_opacity, 0);
        for _ in 0..30 { crate::settings_view::adjust(&mut settings, "opacity", 1); }
        assert_eq!(settings.background_opacity, 100);
        assert_eq!(settings.window_background(), Opaque);
        for value in ["-1", "101", "256", "0.5", "null"] {
            assert!(Preferences::decode(format!("{{\"background_opacity\":{value}}}").as_bytes()).is_err());
        }
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("config.json");
        settings.source = Some(path.clone());
        settings.background_opacity = 75;
        settings.save_appearance()?;
        let saved = Preferences::read(&path, false)?;
        assert_eq!(saved.background_opacity, 75);
        assert!(saved.background_blur);
        assert_eq!(saved.window_background(), Blurred);
        Ok(())
    }

    #[test]
    fn panel_restore_keeps_unrelated_config_and_policy_override() -> Result<()> {
        let baseline = Preferences::default();
        let mut current = Preferences::decode(br##"{
            "font_size":20,"terminal_padding":16,"copy_on_select":true,
            "background_opacity":70,"foreground_color":"#abcdef",
            "commands":[{"name":"Example","argv":["echo","hello"]}],
            "keybindings":[{"key":"cmd-alt-x","action":"settings"}]
        }"##)?;
        current.source = Some("/fixture/settings.json".into());
        current.policy_override = Some(ClosePolicy::Never);
        current.confirm_close = ClosePolicy::Never;
        assert!(AppearanceBaseline(baseline.clone()).has_changes(&current));
        current.restore_panel(&baseline);
        assert!(!AppearanceBaseline(current.clone()).has_changes(&baseline));
        assert!(current.same_saved_appearance(&baseline));
        assert_eq!(current.commands[0].name, "Example");
        assert_eq!(current.keybindings.len(), 1);
        assert_eq!(current.foreground_color, baseline.foreground_color);
        assert_eq!(current.source, Some("/fixture/settings.json".into()));
        assert_eq!(current.confirm_close, ClosePolicy::Never);
        // Saving an environment override must still retain the on-disk policy.
        let mut disk = Preferences::default();
        disk.restore_appearance(&current);
        assert_eq!(disk.confirm_close, ClosePolicy::Active);
        Ok(())
    }

    #[test]
    fn appearance_revert_restores_loaded_theme_without_reading_changed_file() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("theme");
        std::fs::write(&path, "background=123456\nselection-background=334455\npalette=2=#abcdef\n")?;
        let baseline = Preferences::default().import_theme(path.clone())?;
        let mut current = baseline.clone();
        current.select_theme(TerminalTheme::Nord);
        current.foreground_color = Some(HexColor(0xfedcba));
        assert!(AppearanceBaseline(baseline.clone()).has_changes(&current));
        // A broken or moved file must not make an in-session revert depend on I/O.
        std::fs::write(&path, "background=invalid\n")?;
        current.restore_panel(&baseline);
        assert_eq!(current.terminal_colors()[1], Some(rmux_terminal::Color(0x12, 0x34, 0x56)));
        assert_eq!(current.terminal_palette(), baseline.terminal_palette());
        assert_eq!(current.selection_background(), baseline.selection_background());
        assert_eq!(current.foreground_color, baseline.foreground_color);
        assert!(!AppearanceBaseline(baseline.clone()).has_changes(&current));
        std::fs::remove_file(&path)?;
        current.select_theme(TerminalTheme::Light);
        current.restore_panel(&baseline);
        assert_eq!(current.terminal_colors()[1], baseline.terminal_colors()[1]);
        assert_eq!(current.theme_file.as_ref(), Some(&path));
        Ok(())
    }

    #[test]
    fn socket_directory_is_opt_in_and_requires_absolute_path() -> Result<()> {
        assert!(Preferences::decode(b"{}")?.socket_directory.is_none());
        for value in ["", "relative/rmux"] {
            assert!(Preferences::decode(&serde_json::to_vec(&serde_json::json!({"socket_directory":value}))?).is_err());
        }
        let directory = tempfile::tempdir()?;
        let socket = directory.path().join("endpoint");
        let config = directory.path().join("settings.json");
        std::fs::write(&config, serde_json::to_vec(&serde_json::json!({"socket_directory":socket}))?)?;
        let mut preferences = Preferences::read(&config, true)?;
        preferences.source = Some(config.clone());
        preferences.font_size = 18.;
        preferences.save_appearance()?;
        assert_eq!(Preferences::read(&config, true)?.socket_directory, Some(socket.clone()));
        assert!(!socket.exists(), "reading or saving config must not start automation");
        Ok(())
    }

    #[test]
    fn saved_appearance_comparison_tracks_only_persisted_controls() {
        let saved = Preferences::default();
        let mut current = saved.clone();
        assert!(saved.same_saved_appearance(&current));
        current.font_size += 1.;
        assert!(!saved.same_saved_appearance(&current));
        current = saved.clone();
        current.copy_on_select = !current.copy_on_select;
        assert!(!saved.same_saved_appearance(&current));
        current = saved.clone();
        current.native_notifications = true;
        assert!(!saved.same_saved_appearance(&current));
        current = saved.clone();
        current.scrollback_mib += 1;
        assert!(!saved.same_saved_appearance(&current));
        current = saved.clone();
        current.font_family = "Other Mono".into();
        assert!(!saved.same_saved_appearance(&current));
    }

    #[test]
    fn group_placement_defaults_and_rejects_unknown_values() -> Result<()> {
        use crate::workspace_groups::Placement;
        assert_eq!(Preferences::decode(b"{}")?.group_new_workspace_placement, Placement::AfterCurrent);
        assert_eq!(Preferences::decode(br#"{"group_new_workspace_placement":"top"}"#)?.group_new_workspace_placement, Placement::Top);
        assert_eq!(Preferences::decode(br#"{"group_new_workspace_placement":"end"}"#)?.group_new_workspace_placement, Placement::End);
        assert!(Preferences::decode(br#"{"group_new_workspace_placement":"random"}"#).is_err());
        Ok(())
    }
    #[test]
    fn recovery_defaults_to_config_profile_and_can_be_disabled() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let config = directory.path().join("config.json");
        let mut preferences = Preferences::decode(b"{}")?;
        preferences.source = Some(config.clone());
        assert_eq!(preferences.recovery_path(), Some(config.with_extension("session.json")));
        let custom = directory.path().join("custom.json");
        preferences.session_file = Some(custom.clone());
        assert_eq!(preferences.recovery_path(), Some(custom));
        preferences.restore_session = false;
        assert_eq!(preferences.recovery_path(), None);
        assert!(!Preferences::decode(br#"{"restore_session":false}"#)?.restore_session);
        Ok(())
    }
    #[test]
    fn recovery_path_override_requires_an_absolute_path() -> Result<()> {
        assert!(Preferences::decode(b"{}")?.session_file.is_none());
        assert!(Preferences::decode(br#"{"session_file":"relative.json"}"#).is_err());
        assert!(Preferences::decode(br#"{"session_file":""}"#).is_err());
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("session.json");
        let bytes = serde_json::to_vec(&serde_json::json!({"session_file":path}))?;
        assert_eq!(Preferences::decode(&bytes)?.session_file.as_ref(), Some(&path));
        Ok(())
    }
    #[test]
    fn theme_defaults_allow_explicit_color_and_palette_overrides() -> Result<()> {
        let light = Preferences::decode(br#"{"terminal_theme":"light"}"#)?;
        assert_eq!(light.terminal_colors()[1], Some(rmux_terminal::Color(255, 255, 255)));
        assert!(light.terminal_palette().is_some());
        let overridden = Preferences::decode(br##"{"terminal_theme":"light","background_color":"#010203"}"##)?;
        assert_eq!(overridden.terminal_colors()[1], Some(rmux_terminal::Color(1, 2, 3)));
        assert_eq!(overridden.terminal_colors()[0], light.terminal_colors()[0]);
        let value = serde_json::json!({"terminal_theme":"light", "ansi_palette": vec!["#112233"; 16]});
        assert_eq!(Preferences::decode(&serde_json::to_vec(&value)?)?.terminal_palette(), Some([rmux_terminal::Color(17, 34, 51); 16]));
        let dark = Preferences::decode(br#"{"terminal_theme":"dark"}"#)?;
        assert_eq!(dark.terminal_colors(), [None; 3]);
        assert_eq!(dark.terminal_palette(), None);
        for (name, background) in [("solarized_dark", rmux_terminal::Color(0, 43, 54)), ("solarized_light", rmux_terminal::Color(253, 246, 227))] {
            let value = serde_json::json!({"terminal_theme": name});
            let theme = Preferences::decode(&serde_json::to_vec(&value)?)?;
            assert_eq!(theme.terminal_colors()[1], Some(background));
            assert_eq!(theme.terminal_palette().unwrap()[1], rmux_terminal::Color(220, 50, 47));
            assert_eq!(serde_json::to_value(&theme)?["terminal_theme"], name);
            assert!(theme.terminal_theme.cycle(1).cycle(-1) == theme.terminal_theme);
            let overridden = serde_json::json!({"terminal_theme": name, "background_color":"#010203", "ansi_palette": vec!["#112233"; 16]});
            let overridden = Preferences::decode(&serde_json::to_vec(&overridden)?)?;
            assert_eq!(overridden.terminal_colors()[1], Some(rmux_terminal::Color(1, 2, 3)));
            assert_eq!(overridden.terminal_palette(), Some([rmux_terminal::Color(17, 34, 51); 16]));
        }
        let nord = Preferences::decode(br#"{"terminal_theme":"nord"}"#)?;
        assert_eq!(nord.terminal_colors(), [Some(rmux_terminal::Color(216, 222, 233)), Some(rmux_terminal::Color(46, 52, 64)), Some(rmux_terminal::Color(216, 222, 233))]);
        assert_eq!(nord.terminal_palette().unwrap()[14], rmux_terminal::Color(143, 188, 187));
        assert_eq!(serde_json::to_value(&nord)?["terminal_theme"], "nord");
        let overridden = serde_json::json!({"terminal_theme":"nord", "foreground_color":"#010203", "background_color":"#040506", "cursor_color":"#070809", "ansi_palette": vec!["#112233"; 16]});
        let overridden = Preferences::decode(&serde_json::to_vec(&overridden)?)?;
        assert_eq!(overridden.terminal_colors(), [Some(rmux_terminal::Color(1, 2, 3)), Some(rmux_terminal::Color(4, 5, 6)), Some(rmux_terminal::Color(7, 8, 9))]);
        assert_eq!(overridden.terminal_palette(), Some([rmux_terminal::Color(17, 34, 51); 16]));
        for theme in TerminalTheme::ALL {
            assert_eq!(theme.cycle(1).cycle(-1), theme);
        }
        assert_eq!(TerminalTheme::CatppuccinLatte.cycle(1), TerminalTheme::Dark);
        assert!(Preferences::decode(br#"{"terminal_theme":"unknown"}"#).is_err());
        Ok(())
    }
    #[test]
    fn catppuccin_presets_include_cursor_selection_and_override_precedence() -> Result<()> {
        use rmux_terminal::Color;
        let (fg, bg) = (Color(200, 100, 20), Color(20, 40, 60));
        for (name, background, cursor, text, selection) in [
            ("catppuccin_mocha", Color(30,30,46), Color(245,224,220), Color(17,17,27), Color(53,55,73)),
            ("catppuccin_latte", Color(239,241,245), Color(220,138,120), Color(239,241,245), Color(216,218,225)),
        ] {
            let mut settings = Preferences::decode(&serde_json::to_vec(&serde_json::json!({"terminal_theme":name}))?)?;
            assert_eq!(settings.terminal_colors()[1], Some(background));
            assert_eq!(settings.terminal_colors()[2], Some(cursor));
            assert_eq!(settings.cursor_text(fg, bg), text);
            assert_eq!(settings.selection_background().unwrap().resolve(fg,bg), selection);
            assert!(settings.terminal_palette().is_some());
            assert_eq!(serde_json::to_value(&settings)?["terminal_theme"], name);
            let directory = tempfile::tempdir()?;
            let path = directory.path().join("theme");
            std::fs::write(&path, "cursor-color = 010203\ncursor-text = cell-background\nselection-background = cell-foreground")?;
            settings = settings.import_theme(path)?;
            assert_eq!(settings.cursor_fill(fg,bg,cursor,false), Color(1,2,3));
            assert_eq!(settings.cursor_fill(fg,bg,cursor,true), cursor);
            assert_eq!(settings.cursor_text(fg,bg), bg);
            assert_eq!(settings.selection_background().unwrap().resolve(fg,bg), fg);
            settings.cursor_color = Some(CellColor::CellBackground);
            assert_eq!(settings.cursor_fill(fg,bg,cursor,false), bg);
        }
        Ok(())
    }
    #[test]
    fn terminal_color_settings_validate_and_round_trip() -> Result<()> {
        let preferences = Preferences::decode(br##"{"foreground_color":"#AABBCC","background_color":"#010203","cursor_color":null}"##)?;
        assert_eq!(preferences.terminal_colors(), [Some(rmux_terminal::Color(170, 187, 204)), Some(rmux_terminal::Color(1, 2, 3)), None]);
        let serialized = serde_json::to_vec(&preferences)?;
        assert_eq!(Preferences::decode(&serialized)?.terminal_colors(), preferences.terminal_colors());
        for invalid in ["red", "123456", "#fff", "#11223344", "#gg0000"] {
            let value = serde_json::json!({"background_color": invalid});
            assert!(Preferences::decode(&serde_json::to_vec(&value)?).is_err());
        }
        assert_eq!(Preferences::decode(b"{}")?.terminal_colors(), [None; 3]);
        for count in [0, 15, 16, 17] {
            let value = serde_json::json!({"ansi_palette": vec!["#010203"; count]});
            let decoded = Preferences::decode(&serde_json::to_vec(&value)?);
            if count == 16 {
                assert_eq!(decoded?.terminal_palette(), Some([rmux_terminal::Color(1, 2, 3); 16]));
            } else {
                assert!(decoded.is_err());
            }
        }
        Ok(())
    }
    #[test]
    fn selection_opacity_keeps_theme_defaults_and_round_trips() -> Result<()> {
        use rmux_terminal::Color;
        let (fg, bg) = (Color(200, 100, 20), Color(20, 40, 60));
        let mut settings = Preferences::decode(b"{}")?;
        assert!((settings.selection_background_fill(fg, bg).a - 128. / 255.).abs() < 0.001);
        settings.selection_background = Some(CellColor::parse_theme("cell-foreground")?);
        let original = settings.selection_background_fill(fg, bg);
        assert_eq!(original.a, 1.);
        crate::settings_view::adjust(&mut settings, "selection_opacity", -1);
        assert_eq!(settings.selection_opacity, Some(95));
        let fill = settings.selection_background_fill(fg, bg);
        assert_eq!((fill.h, fill.s, fill.l), (original.h, original.s, original.l));
        assert_eq!(fill.a, 0.95);
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("config.json");
        settings.source = Some(path.clone());
        settings.save_appearance()?;
        assert_eq!(Preferences::read(&path, false)?.selection_opacity, Some(95));
        for _ in 0..30 { crate::settings_view::adjust(&mut settings, "selection_opacity", -1); }
        assert_eq!(settings.selection_background_fill(fg, bg).a, 0.);
        crate::settings_view::adjust(&mut settings, "selection_opacity", 0);
        assert_eq!(settings.selection_background_fill(fg, bg).a, 1.);
        assert_eq!(settings.selection_opacity, None);
        for value in ["-1", "101", "256", "0.5"] {
            assert!(Preferences::decode(format!("{{\"selection_opacity\":{value}}}").as_bytes()).is_err());
        }
        Ok(())
    }
    #[test]
    fn cursor_opacity_preserves_text_and_round_trips_settings() -> Result<()> {
        use rmux_terminal::Color;
        let mut settings = Preferences::decode(b"{}")?;
        let (foreground, background) = (Color(200, 100, 20), Color(20, 40, 60));
        assert_eq!(settings.cursor_opacity, 100);
        assert_eq!(settings.cursor_text(foreground, background), background);
        settings.cursor_opacity = 50;
        assert_eq!(settings.cursor_text(foreground, background), Color(110, 70, 40));
        assert_eq!(settings.cursor_paint(foreground).a, 0.5);
        assert!(!settings.same_saved_appearance(&Preferences::default()));
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("config.json");
        settings.source = Some(path.clone());
        settings.save_appearance()?;
        assert_eq!(Preferences::read(&path, false)?.cursor_opacity, 50);
        for _ in 0..30 { crate::settings_view::adjust(&mut settings, "cursor_opacity", -1); }
        assert_eq!(settings.cursor_opacity, 0);
        assert_eq!(settings.cursor_text(foreground, background), foreground);
        assert_eq!(settings.cursor_paint(foreground).a, 0.);
        for _ in 0..30 { crate::settings_view::adjust(&mut settings, "cursor_opacity", 1); }
        assert_eq!(settings.cursor_opacity, 100);
        for value in ["-1", "101", "256", "0.5", "null"] {
            assert!(Preferences::decode(format!("{{\"cursor_opacity\":{value}}}").as_bytes()).is_err());
        }
        Ok(())
    }
    #[test]
    fn minimal_chrome_preferences_save_reload_and_revert() -> Result<()> {
        let root = tempfile::tempdir()?;
        let path = root.path().join("settings.json");
        std::fs::write(&path, br#"{"font_size":17,"commands":[{"name":"Keep","argv":["echo","keep"]}]}"#)?;
        let mut settings = Preferences::read(&path, false)?;
        settings.source = Some(path.clone());
        let baseline = settings.clone();
        for key in ["cursor_blink", "controls_on_hover", "hide_top_bar", "compact_sidebar"] {
            crate::settings_view::adjust(&mut settings, key, 1);
        }
        assert!(!settings.same_saved_appearance(&baseline));
        settings.save_appearance()?;
        let saved = Preferences::read(&path, false)?;
        assert!(saved.cursor_blink && saved.controls_on_hover && saved.hide_top_bar && saved.compact_sidebar);
        assert!(saved.same_saved_appearance(&settings));
        assert_eq!(saved.commands[0].name, "Keep");
        settings.restore_appearance(&baseline);
        assert!(!settings.cursor_blink && !settings.controls_on_hover && !settings.hide_top_bar && !settings.compact_sidebar);
        assert!(settings.same_saved_appearance(&baseline));
        for key in ["cursor_blink", "controls_on_hover", "hide_top_bar", "compact_sidebar"] {
            assert!(Preferences::decode(format!("{{\"{key}\":\"yes\"}}").as_bytes()).is_err());
        }
        Ok(())
    }

    #[test]
    fn cursor_preferences_preserve_application_and_composition_shapes() -> Result<()> {
        use rmux_terminal::CursorShape;
        let defaults = Preferences::decode(b"{}")?;
        let hidden = Preferences::decode(br#"{"unfocused_cursor":"hidden"}"#)?;
        for shape in [CursorShape::Block, CursorShape::Bar, CursorShape::Underline, CursorShape::Hollow] {
            assert_eq!(defaults.cursor_shape(true, false, shape), Some(shape));
            assert_eq!(hidden.cursor_shape(true, false, shape), Some(shape));
            assert_eq!(defaults.cursor_shape(true, true, shape), Some(CursorShape::Bar));
            assert_eq!(defaults.cursor_shape(false, false, shape), Some(CursorShape::Hollow));
            assert_eq!(hidden.cursor_shape(false, false, shape), None);
        }
        assert!(Preferences::decode(br#"{"unfocused_cursor":"blink"}"#).is_err());
        assert!(Preferences::decode(br#"{"unfocused_cursor":false}"#).is_err());
        for (name, expected) in [("block", CursorShape::Block), ("bar", CursorShape::Bar), ("underline", CursorShape::Underline)] {
            let value = serde_json::json!({"focused_cursor":name, "unfocused_cursor":"hidden"});
            let settings = Preferences::decode(&serde_json::to_vec(&value)?)?;
            assert_eq!(settings.cursor_shape(true, false, CursorShape::Hollow), Some(expected));
            assert_eq!(settings.cursor_shape(true, true, CursorShape::Block), Some(CursorShape::Bar));
            assert_eq!(settings.cursor_shape(false, false, CursorShape::Block), None);
            assert!(!settings.same_saved_appearance(&hidden));
            assert_eq!(serde_json::to_value(&settings)?["focused_cursor"], name);
            assert!(settings.focused_cursor.cycle(false).cycle(true) == settings.focused_cursor);
        }
        assert!(Preferences::decode(br#"{"focused_cursor":"unknown"}"#).is_err());
        Ok(())
    }
    #[test]
    fn layout_presets_validate_names_paths_and_shortcut_targets() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("workspace 世界.json");
        let preset = serde_json::json!({"name":"Build","path":path,"session":true});
        let valid = serde_json::json!({"layout_presets":[preset.clone()],"keybindings":[{"key":"ctrl-k b","preset":"Build"}]});
        let preferences = Preferences::decode(&serde_json::to_vec(&valid)?)?;
        assert_eq!(preferences.layout_presets[0].path, path);
        assert!(preferences.layout_presets[0].session);
        assert!(!path.exists(), "config validation must not create or require preset files");
        for invalid in [
            serde_json::json!({"layout_presets":[preset.clone(),{"name":" build ","path":path}]}),
            serde_json::json!({"layout_presets":[{"name":"Build","path":"relative.json"}]}),
            serde_json::json!({"layout_presets":[{"name":" ","path":path}]}),
            serde_json::json!({"layout_presets":[{"name":"Build","path":path,"cwd":"relative"}]}),
            serde_json::json!({"layout_presets":vec![preset.clone();33]}),
            serde_json::json!({"keybindings":[{"key":"ctrl-k b","preset":"Missing"}]}),
            serde_json::json!({"layout_presets":[preset],"keybindings":[{"key":"ctrl-k b","preset":"Build","text":"echo"}]}),
        ] { assert!(Preferences::decode(&serde_json::to_vec(&invalid)?).is_err()); }
        Ok(())
    }

    #[test]
    fn custom_commands_validate_limits_identity_and_literal_arguments() -> Result<()> {
        let valid = serde_json::json!({"commands":[{"name":"Build 世界", "argv":["mise","exec","--","echo","a b","", "$(literal)"], "cwd":"/tmp"}]});
        let preferences = Preferences::decode(&serde_json::to_vec(&valid)?)?;
        assert_eq!(preferences.commands[0].argv[5], "");
        assert_eq!(preferences.commands[0].argv[6], "$(literal)");
        let with_env = serde_json::json!({"commands":[{"name":"Env","argv":["tool"],"env":{"VALID_1":"世界","EMPTY":""}}]});
        assert_eq!(Preferences::decode(&serde_json::to_vec(&with_env)?)?.commands[0].env["VALID_1"], "世界");
        for commands in [
            serde_json::json!([{"name":"Build","argv":[]}]),
            serde_json::json!([{"name":"Build","argv":[""]}]),
            serde_json::json!([{"name":"Build","argv":["tool"],"cwd":"relative"}]),
            serde_json::json!([{"name":"Build","argv":["tool"]},{"name":"build","argv":["other"]}]),
            serde_json::json!([{"name":"Build","argv":["tool","bad\u{0}argument"]}]),
            serde_json::json!([{"name":"Build","argv":vec!["tool";65]}]),
            serde_json::json!([{"name":"Build","argv":["tool"],"env":{"1BAD":"value"}}]),
            serde_json::json!([{"name":"Build","argv":["tool"],"env":{"BAD=KEY":"value"}}]),
            serde_json::json!([{"name":"Build","argv":["tool"],"env":{"VALID":"bad\u{0}value"}}]),
        ] { assert!(Preferences::decode(&serde_json::to_vec(&serde_json::json!({"commands":commands}))?).is_err()); }
        Ok(())
    }

    #[test]
    fn font_defaults_update_inherited_attributes_and_preserve_overrides() {
        let next = Preferences { font_size: 18., font_family: "Next Mono".into(), ..Preferences::default() };
        assert_eq!(updated_font(14., "Old Mono", 14., "Old Mono", &next), (18., "Next Mono".into()));
        assert_eq!(updated_font(22., "Old Mono", 14., "Old Mono", &next), (22., "Next Mono".into()));
        assert_eq!(updated_font(14., "Custom Mono", 14., "Old Mono", &next), (18., "Custom Mono".into()));
        assert_eq!(updated_font(22., "Custom Mono", 14., "Old Mono", &next), (22., "Custom Mono".into()));
    }

    #[test]
    fn appearance_save_merges_other_settings_and_preserves_invalid_file() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("config.json");
        let mut preferences = Preferences { source: Some(path.clone()), ..Preferences::default() };
        preferences.font_size = 19.;
        preferences.terminal_padding = 12;
        preferences.scrollback_mib = 8;
        preferences.macos_option_as_alt = false;
        preferences.copy_on_select = true;
        preferences.native_notifications = true;
        preferences.notification_focus = NotificationFocus::BackgroundApp;
        preferences.new_workspace_placement = crate::workspace_groups::Placement::Top;
        preferences.group_new_workspace_placement = crate::workspace_groups::Placement::End;
        preferences.confirm_close = ClosePolicy::Never;
        std::fs::write(&path, br##"{"font_family":"Custom Mono","scrollback_mib":16,"background_color":"#112233","confirm_close":"always"}"##)?;
        preferences.save_appearance()?;
        let saved = Preferences::read(&path, true)?;
        assert_eq!(saved.font_size, 19.);
        assert_eq!(saved.terminal_padding, 12);
        assert!(!saved.macos_option_as_alt);
        assert!(saved.copy_on_select);
        assert!(saved.native_notifications);
        assert_eq!(saved.notification_focus, NotificationFocus::BackgroundApp);
        assert_eq!(saved.new_workspace_placement, crate::workspace_groups::Placement::Top);
        assert_eq!(saved.group_new_workspace_placement, crate::workspace_groups::Placement::End);
        assert_eq!(saved.font_family, preferences.font_family);
        assert_eq!(saved.scrollback_mib, 8);
        assert_eq!(saved.confirm_close, ClosePolicy::Never);
        preferences.policy_override = Some(ClosePolicy::Always);
        preferences.confirm_close = ClosePolicy::Always;
        crate::settings_view::adjust(&mut preferences, "close", 1);
        assert_eq!(preferences.confirm_close, ClosePolicy::Always);
        preferences.save_appearance()?;
        assert_eq!(Preferences::read(&path, true)?.confirm_close, ClosePolicy::Never);
        assert_eq!(saved.terminal_colors()[1], Some(rmux_terminal::Color(0x11, 0x22, 0x33)));
        std::fs::write(&path, b"unfinished user edit")?;
        assert!(preferences.save_appearance().is_err());
        assert_eq!(std::fs::read(&path)?, b"unfinished user edit");
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn appearance_save_follows_valid_links_and_preserves_dangling_links() -> Result<()> {
        use std::os::unix::fs::symlink;
        let directory = tempfile::tempdir()?;
        let target = directory.path().join("target.json");
        let link = directory.path().join("config.json");
        std::fs::write(&target, b"{}")?;
        symlink("target.json", &link)?;
        let preferences = Preferences { source: Some(link.clone()), font_size: 20., ..Preferences::default() };
        preferences.save_appearance()?;
        assert_eq!(std::fs::read_link(&link)?, std::path::PathBuf::from("target.json"));
        assert_eq!(Preferences::read(&target, true)?.font_size, 20.);
        std::fs::remove_file(&target)?;
        assert!(preferences.save_appearance().is_err());
        assert_eq!(std::fs::read_link(&link)?, std::path::PathBuf::from("target.json"));
        assert!(!target.exists());
        Ok(())
    }

    #[test]
    fn explicit_settings_open_creates_defaults_without_overwriting() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("nested/config.json");
        let preferences = Preferences {
            source: Some(path.clone()),
            ..Preferences::default()
        };
        assert_eq!(preferences.prepare_file()?, path);
        assert_eq!(Preferences::read(&path, true)?.font_size, 14.);
        std::fs::write(&path, b"user text, even if temporarily invalid")?;
        preferences.prepare_file()?;
        assert_eq!(
            std::fs::read(&path)?,
            b"user text, even if temporarily invalid"
        );
        assert!(Preferences::default().prepare_file().is_err());
        Ok(())
    }
    #[test]
    fn file_editor_reload_validates_and_retains_previous_configuration() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("config.json");
        let program = std::env::current_exe()?;
        let source = Preferences { source: Some(path.clone()), required: true, ..Preferences::default() };
        let valid = serde_json::json!({"file_editor":{"program":program,"args":["--","{file}"]}});
        std::fs::write(&path, serde_json::to_vec(&valid)?)?;
        let current = source.reload()?;
        assert_eq!(current.file_editor.as_ref().unwrap().program, program);
        for invalid in [
            serde_json::json!({"program":"relative","args":["{file}"]}),
            serde_json::json!({"program":program,"args":[]}),
            serde_json::json!({"program":format!("{}\0", program.display()),"args":["{file}"]}),
        ] {
            std::fs::write(&path, serde_json::to_vec(&serde_json::json!({"file_editor":invalid}))?)?;
            assert!(current.reload().is_err());
            assert_eq!(current.file_editor.as_ref().unwrap().program, program);
        }
        std::fs::write(&path, b"{}")?;
        assert!(current.reload()?.file_editor.is_none());
        Ok(())
    }
    #[test]
    fn reload_keeps_source_override_and_previous_valid_settings() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("config.json");
        let source = Preferences {
            source: Some(path.clone()),
            required: true,
            policy_override: Some(ClosePolicy::Never),
            ..Preferences::default()
        };
        std::fs::write(&path, r#"{"font_size":18,"confirm_close":"always"}"#)?;
        let current = source.reload()?;
        assert_eq!(current.font_size, 18.);
        assert_eq!(current.confirm_close, ClosePolicy::Never);
        std::fs::write(&path, r#"{"font_size":22}"#)?;
        assert_eq!(current.reload()?.font_size, 22.);
        std::fs::write(&path, r#"{"font_size":100}"#)?;
        assert!(current.reload().is_err());
        assert_eq!(current.font_size, 18.);
        std::fs::remove_file(&path)?;
        assert!(current.reload().is_err());
        Ok(())
    }
    #[test]
    fn config_defaults_validation_and_file_limits() -> Result<()> {
        let defaults = Preferences::decode(b"{}")?;
        assert_eq!(defaults.font_size, 14.);
        assert_eq!(defaults.terminal_padding, 8);
        assert!(!defaults.copy_on_select);
        assert!(defaults.macos_option_as_alt);
        assert!(!Preferences::decode(br#"{"macos_option_as_alt":false}"#)?.macos_option_as_alt);
        assert!(Preferences::decode(br#"{"copy_on_select":true}"#)?.copy_on_select);
        assert_eq!(defaults.confirm_close, ClosePolicy::Active);
        let custom = Preferences::decode(br#"{"font_size":18,"confirm_close":"always"}"#)?;
        assert_eq!(custom.font_size, 18.);
        assert_eq!(custom.confirm_close, ClosePolicy::Always);
        for invalid in [
            r#"{"font_size":7}"#,
            r#"{"terminal_padding":33}"#,
            r#"{"terminal_padding":-1}"#,
            r#"{"copy_on_select":"true"}"#,
            r#"{"macos_option_as_alt":"false"}"#,
            r#"{"font_size":41}"#,
            r#"{"confirm_close":"bad"}"#,
            r#"{"font_szie":18}"#,
            r#"{"scrollback_mib":257}"#,
            r#"{"scrollback_mib":-1}"#,
            r#"{"font_family":""}"#,
            r#"{"font_family":"bad\nname"}"#,
        ] {
            assert!(Preferences::decode(invalid.as_bytes()).is_err());
        }
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("config.json");
        assert_eq!(Preferences::read(&path, false)?.font_size, 14.);
        assert!(Preferences::read(&path, true).is_err());
        std::fs::write(&path, vec![b' '; 64 * 1024 + 1])?;
        assert!(Preferences::read(&path, true).is_err());
        Ok(())
    }
}

impl From<CellColor> for rmux_core::terminal_frame::CellPaint {
    fn from(value: CellColor) -> Self {
        match value {
            CellColor::Literal(color) => Self::Literal(color.rgb()),
            CellColor::CellForeground => Self::CellForeground,
            CellColor::CellBackground => Self::CellBackground,
        }
    }
}
