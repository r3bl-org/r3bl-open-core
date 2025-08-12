// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// Attach files.
pub mod app;
pub mod component;
pub mod default_input_handler;
pub mod event_routing_support;
pub mod main_event_loop;
pub mod manage_focus;
pub mod shared_global_data;
pub mod terminal_window_api;
pub mod terminal_window_type_aliases;

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
