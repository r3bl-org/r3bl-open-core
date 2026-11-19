// Copyright (c) 2025-2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`PTY`]-based integration tests for the [`choose()`] interactive selection UI.
//!
//! These tests validate end-to-end behavior in a real pseudoterminal, including
//! [`SharedWriter`] pause/resume signaling and keyboard-driven selection.
//!
//! [`choose()`]: crate::choose
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`SharedWriter`]: crate::SharedWriter

// Attach.
#[cfg(any(all(unix, doc), test))]
mod pty_shared_writer_pause_test;
mod pty_test_select_component;
