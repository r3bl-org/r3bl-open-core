// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! # [`PTY`] Session Events
//!
//! Event definitions and encoders for communicating with child processes running in a
//! [`PTY`].
//!
//! * [`PtyInputEvent`]: Events sent to the child process's standard input.
//! * [`PtyOutputEvent`]: Events received from the child process's standard output.
//! * [`key_press_generator`]: Outbound generation of [`PtyInputEvent`]s from [`KeyPress`].
//!
//! [`KeyPress`]: crate::KeyPress
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`PtyInputEvent`]: crate::PtyInputEvent
//! [`PtyOutputEvent`]: crate::PtyOutputEvent

#![rustfmt::skip]

// Attach.

mod input;
// Conditionally public for documentation and tests.
#[cfg(any(test, doc))]
pub mod key_press_generator;
#[cfg(not(any(test, doc)))]
mod key_press_generator;
mod output;

// Re-export.

pub use input::*;
pub use output::*;
