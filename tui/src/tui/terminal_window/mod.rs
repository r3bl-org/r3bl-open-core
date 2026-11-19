// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// Attach. Crate-wide (Intra-crate).
pub(crate) mod terminal_window_api; // Cross-folder rustdoc module link (lib.rs).

// Attach. Folder-local (Intra-module).
mod app;
mod component;
mod default_input_handler;
mod event_routing_support;
mod main_event_loop;
mod manage_focus;
mod shared_global_data;
mod terminal_window_type_aliases;

// Integration tests (PTY tests).
#[cfg(any(all(unix, doc), test))]
mod terminal_window_integration_tests;

// Re-export.
pub use app::*;
pub use component::*;
pub use default_input_handler::*;
pub use event_routing_support::*;
pub use main_event_loop::*;
pub use manage_focus::*;
pub use shared_global_data::*;
pub use terminal_window_api::*;
pub use terminal_window_type_aliases::*;
