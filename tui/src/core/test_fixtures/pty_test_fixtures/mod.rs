// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// Attach private modules.
mod async_debounced_deadline;
mod constants;
mod deadline;
mod debounced_state;
mod pty_test_watchdog;
mod single_thread_safe_controlled_child;
mod spawn_controlled_in_pty;
#[macro_use] // Propagate macros textually (order matters).
mod generate_pty_test;

// Export flat public API.
pub use async_debounced_deadline::*;
pub use constants::*;
pub use deadline::*;
pub use debounced_state::*;
pub use generate_pty_test::*; // Exports everything except for the macro.
pub use pty_test_watchdog::*;
pub use single_thread_safe_controlled_child::*;
pub use spawn_controlled_in_pty::*;
