// Copyright (c) 2024-2026 R3BL LLC. Licensed under Apache License, Version 2.0.

#![rustfmt::skip]

// Attach.
mod event_conversion;
mod history;
mod line_control_task;
mod line_state;
mod lock_manager;
mod readline_struct;
mod types;

// Re-export.
pub use event_conversion::*;
pub use history::*;
pub use line_control_task::*;
pub use line_state::*;
pub use lock_manager::*;
pub use readline_struct::*;
pub use types::*;

// Integration tests (conditional visibility).
#[cfg(any(test, doc))]
pub(crate) mod readline_async_integration_tests; // Cross-folder rustdoc links (pty_test_child_impl, event_handlers).
