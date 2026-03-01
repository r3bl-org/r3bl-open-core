// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// Attach private modules.
mod async_debounced_deadline;
mod deadline;
mod debounced_state;
mod normalize_pty_output;
mod pty_test_watchdog;
mod read_lines_and_drain;
mod single_thread_safe_controlled_child;
mod spawn_controlled_in_pty;

// Macro module - #[macro_export] makes it available at crate root.
// PtyTestMode enum is exported via the wildcard re-export below.
#[macro_use]
mod generate_pty_test;
// Export flat public API.
pub use async_debounced_deadline::*;
pub use deadline::*;
pub use debounced_state::*;
pub use generate_pty_test::*;
pub use normalize_pty_output::*;
pub use pty_test_watchdog::*;
pub use read_lines_and_drain::*;
pub use single_thread_safe_controlled_child::*;
pub use spawn_controlled_in_pty::*;
