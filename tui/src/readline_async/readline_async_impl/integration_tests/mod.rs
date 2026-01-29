// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! System-level PTY tests - end-to-end validation of readline editing in real pseudo-terminals.
//!
//! These tests validate the complete readline editing stack in a real PTY environment:
//! - [`LineState`] async event handling and state management
//! - Line editing operations (insert, delete, navigation)
//! - Word boundary detection with Unicode support
//! - Terminal rendering and cursor positioning
//!
//! All tests use **real keyboard input sequences** to verify the system handles actual user input correctly.
//!
//! Run with: `cargo test test_pty_readline -- --nocapture`
//!
//! # Testing Philosophy
//!
//! **PTY tests validate end-to-end behavior** because:
//! - **Real-world testing**: Tests run in an actual pseudo-terminal, matching production environment
//! - **Integration validation**: Verifies the complete stack from keyboard input → line state → terminal output
//! - **Unicode safety**: Validates multi-byte character handling in actual terminal environment
//!
//! **Unit tests** (in [`line_state`]) validate individual handler logic with mocked I/O.
//! **PTY tests** validate the full system works correctly in a real terminal.
//!
//! See the [parent module] for the overall testing strategy.
//!
//! [`LineState`]: super::LineState
//! [`line_state`]: super::line_state
//! [parent module]: mod@super

// Skip rustfmt for rest of file.
// https://stackoverflow.com/a/75910283/2085356
#![cfg_attr(rustfmt, rustfmt_skip)]

// These PTY tests use DirectToAnsiInputDevice which is Linux-only.
// On macOS/Windows, Crossterm backend is used instead and these tests are skipped.
// Doc builds are allowed on all platforms so documentation can be read anywhere.
#[cfg(any(doc, all(target_os = "linux", test)))]
pub mod pty_ctrl_d_eof_test;
#[cfg(any(doc, all(target_os = "linux", test)))]
pub mod pty_ctrl_d_delete_test;
#[cfg(any(doc, all(target_os = "linux", test)))]
pub mod pty_ctrl_u_test;
#[cfg(any(doc, all(target_os = "linux", test)))]
pub mod pty_ctrl_w_test;
#[cfg(any(doc, all(target_os = "linux", test)))]
pub mod pty_ctrl_navigation_test;
#[cfg(any(doc, all(target_os = "linux", test)))]
pub mod pty_alt_navigation_test;
#[cfg(any(doc, all(target_os = "linux", test)))]
pub mod pty_alt_kill_test;
// These PTY tests use only portable_pty (no DirectToAnsiInputDevice) and work cross-platform.
#[cfg(any(test, doc))]
pub mod pty_shared_writer_no_blank_line_test;
#[cfg(any(test, doc))]
pub mod pty_multiline_output_test;
