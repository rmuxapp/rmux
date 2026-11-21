//! Application behavior shared by native and future browser hosts.
//! Keep this crate independent of rendering, terminal processes and OS services.
pub mod focus_trail;
pub mod layout;
pub mod commands;
pub mod palette_match;
pub mod terminal_frame;
pub mod terminal_runs;
pub mod keyboard;
pub mod workspace_groups;
pub mod scroll;
pub mod tabs;
pub mod terminal_text;
pub mod pane_commands;
pub mod sample_input;
pub mod workspace_lifecycle;
pub mod search_state;
pub mod sample_session;
