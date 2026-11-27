// Copyright (c) 2025-2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`PTY`]-based integration tests for the [`try_choose()`] interactive selection UI.
//!
//! These tests validate end-to-end behavior in a real pseudoterminal, including
//! [`SharedWriter`] pause/resume signaling and keyboard-driven selection.
//!
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`SharedWriter`]: crate::SharedWriter
//! [`try_choose()`]: crate::try_choose

// Attach.
#[cfg(any(all(unix, doc), test))]
mod pty_shared_writer_pause_test;
mod pty_test_select_component;
