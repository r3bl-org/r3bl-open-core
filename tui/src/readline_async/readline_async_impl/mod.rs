// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// Attach.
mod channel_monitor;
mod event_conversion;
mod history;
mod line_state;
mod lock_manager;
mod readline_struct;
mod types;

// Re-export.
pub use channel_monitor::*;
pub use event_conversion::*;
pub use history::*;
pub use line_state::*;
pub use lock_manager::*;
pub use readline_struct::*;
pub use types::*;

// Integration tests (conditional visibility).
#[cfg(any(test, doc))]
pub mod readline_async_integration_tests;
