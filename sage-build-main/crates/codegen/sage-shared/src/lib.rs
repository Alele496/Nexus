//! Shared utilities used by both `sage-shell` and its downstream clients
//! (e.g. `sage-pager-render`). This crate sits upstream of `sage-shell`
//! so it must never depend on it.

pub mod clipboard;
pub mod placeholder_images;
pub mod session;
pub mod stderr;
pub mod ui_config;
