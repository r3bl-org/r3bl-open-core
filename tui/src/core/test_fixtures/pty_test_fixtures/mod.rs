// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

#![rustfmt::skip]

// Attach. Upward #[macro_use] propagation (must precede modules below that invoke their macros).
#[macro_use]
mod generate_pty_test;

// Attach.
mod async_debounced_deadline;
mod constants;
mod deadline;
mod debounced_state;
mod pty_test_child;
mod pty_test_watchdog;
mod spawn_controlled_in_pty;

// Re-export.
pub use async_debounced_deadline::*;
pub use constants::*;
pub use deadline::*;
pub use debounced_state::*;
pub use pty_test_child::*;
pub use pty_test_watchdog::*;
pub use spawn_controlled_in_pty::*;
// Exports everything except macro; #[macro_export] exports it at crate root.
pub use generate_pty_test::*;
