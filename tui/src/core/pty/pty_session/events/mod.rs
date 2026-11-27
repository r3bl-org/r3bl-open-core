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

// Attach. Crate-wide (Intra-crate).
pub(crate) mod key_press_generator; // Cross-folder rustdoc module link (protocol_conversion).

// Attach. Folder-local (Intra-module).
mod input;
mod output;

// Re-export.
pub use input::*;
pub use output::*;
