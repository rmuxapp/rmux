//! Application behavior shared by native and future browser hosts.
//! Keep this crate independent of rendering, terminal processes and OS services.
pub mod focus_trail;
pub mod layout;
pub mod commands;
pub mod palette_match;
pub mod terminal_frame;
