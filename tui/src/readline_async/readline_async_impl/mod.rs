// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// Attach.
pub mod line_state;
pub mod readline;
pub mod readline_history;
pub mod readline_lock_manager;

// Re-export.
pub use line_state::*;
pub use readline::*;
pub use readline_history::*;
pub use readline_lock_manager::*;

// Integration tests (conditional visibility).
#[cfg(any(test, doc))]
pub mod readline_async_integration_tests;
