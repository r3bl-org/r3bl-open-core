// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

#![rustfmt::skip]

// Attach private modules.
mod async_debounced_deadline;
mod constants;
mod deadline;
mod debounced_state;
mod generate_pty_test;
mod pty_test_child;
mod pty_test_watchdog;
mod spawn_controlled_in_pty;

// Export flat public API.
pub use async_debounced_deadline::*;
pub use constants::*;
pub use deadline::*;
pub use debounced_state::*;
pub use pty_test_child::*;
pub use pty_test_watchdog::*;
pub use spawn_controlled_in_pty::*;
// Exports everything except macro; #[macro_export] exports it at crate root.
pub use generate_pty_test::*;
