//! Shared wire vocabulary. No runtime, rendering or OS dependencies.
pub const EVENT_KINDS: &[&str] = &[
    "notification.received", "notification.read", "notification.all_read",
    "notification.dismissed", "notification.cleared",
    "surface.created", "surface.closed", "surface.exited",
    "surface.title_changed", "surface.directory_changed", "surface.status_changed",
    "surface.progress_changed", "surface.focus_changed", "surface.bell",
    "surface.location_invalidated", "workspace.created", "workspace.closed",
    "workspace.location_invalidated",
];
