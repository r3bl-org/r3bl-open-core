// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Integration tests for VT-100 terminal input parsing.
//!
//! ## Test Organization
//!
//! ### Interactive Terminal Observation
//!
//! The `observe_real_terminal_input_events.rs` file provides an interactive test that
//! captures raw bytes from real terminal interactions. It is a great way to get and parse
//! the actual sequences in real terminals for making the parser work better for various
//! terminal applications on various OSes.
//!
//! This helps establish ground truth for:
//! - Coordinate system conventions (1-based for VT-100)
//! - Actual ANSI sequences sent by terminal emulators
//! - Terminal-specific behaviors and quirks
//!
//! Run with: `cargo test observe_terminal -- --ignored --nocapture`
//!
//! ### Automated Parser Validation
//!
//! The `input_parser_validation_test` module contains automated tests using real ANSI
//! sequences captured from terminal observation. These tests validate parser correctness
//! against confirmed terminal output for:
//! - Mouse events (clicks, drags, scrolling, modifiers)
//! - Keyboard events (arrows, function keys, modifier combinations)
//! - Edge cases (incomplete sequences, invalid data, boundary conditions)
//!
//! ### PTY-Based DirectToAnsiInputDevice Testing
//!
//! The `pty_based_input_device_test` module tests the complete DirectToAnsiInputDevice
//! in a real PTY context using a bootstrap/slave pattern. This validates:
//! - Async I/O loop behavior
//! - Zero-latency ESC key detection
//! - Buffer management and compaction
//! - End-to-end parsing from raw bytes to InputEvent
//!
//! Run with: `cargo test test_pty -- --ignored --nocapture`

#[cfg(any(test, doc))]
pub mod input_parser_validation_test;
#[cfg(any(test, doc))]
pub mod observe_real_terminal_input_events;
#[cfg(any(test, doc))]
pub mod pty_based_input_device_test;
