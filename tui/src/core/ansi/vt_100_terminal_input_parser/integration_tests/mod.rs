// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! System-level PTY tests - end-to-end validation in real pseudo-terminals.
//!
//! These tests validate the complete input handling stack in a real PTY environment:
//! - `DirectToAnsiInputDevice` async I/O and buffer management
//! - Zero-latency `ESC` key detection
//! - Full parsing from raw bytes to `InputEvent`
//!
//! All tests use **generated sequences** to verify the system can handle its own output.
//!
//! Run with: `cargo test test_pty -- --nocapture`
//!
//! # Testing Philosophy
//!
//! **PTY tests use generated sequences** because:
//! - **Real-world testing**: In production, our generator creates sequences and our
//!   parser consumes them - we test this integration
//! - **System compatibility**: If both generator and parser have matching bugs, this is
//!   still OK - the system works end-to-end
//! - **Scalability**: Easy to test many input combinations without hardcoding hundreds of
//!   test cases
//!
//! **Protocol conformance** is tested separately in [`validation_tests`], which use
//! hardcoded sequences to validate against the VT-100 spec.
//!
//! See the [parent module documentation] for the overall testing strategy.
//!
//! [`validation_tests`]: mod@crate::core::ansi::vt_100_terminal_input_parser::validation_tests
//! [parent module documentation]: mod@super#testing-strategy

// Skip rustfmt for rest of file.
// https://stackoverflow.com/a/75910283/2085356
#![cfg_attr(rustfmt, rustfmt_skip)]

// These PTY tests use DirectToAnsiInputDevice which is Linux-only.
// On macOS/Windows, Crossterm backend is used instead and these tests are skipped.
// Doc builds are allowed on all platforms so documentation can be read anywhere.
#[cfg(any(doc, all(target_os = "linux", test)))]
pub mod pty_bracketed_paste_test;

#[cfg(any(doc, all(target_os = "linux", test)))]
pub mod pty_input_device_test;

#[cfg(any(doc, all(target_os = "linux", test)))]
pub mod pty_keyboard_modifiers_test;

#[cfg(any(doc, all(target_os = "linux", test)))]
pub mod pty_mouse_events_test;

#[cfg(any(doc, all(target_os = "linux", test)))]
pub mod pty_terminal_events_test;

#[cfg(any(doc, all(target_os = "linux", test)))]
pub mod pty_utf8_text_test;

#[cfg(any(doc, all(target_os = "linux", test)))]
pub mod pty_new_keyboard_features_test;

#[cfg(any(doc, all(target_os = "linux", test)))]
pub mod pty_sigwinch_test;

#[cfg(any(doc, all(target_os = "linux", test)))]
pub mod pty_mio_poller_thread_lifecycle_test;

#[cfg(any(doc, all(target_os = "linux", test)))]
pub mod pty_mio_poller_thread_reuse_test;

#[cfg(any(doc, all(target_os = "linux", test)))]
pub mod pty_mio_poller_subscribe_test;

#[cfg(any(doc, all(target_os = "linux", test)))]
pub mod pty_mio_poller_singleton_test;
