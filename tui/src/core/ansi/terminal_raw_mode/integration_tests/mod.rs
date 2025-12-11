// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Automated integration tests for terminal raw mode functionality.
//!
//! This module contains **automated** PTY-based integration tests that verify
//! raw mode behavior without requiring user interaction. All tests in this
//! module can run in CI environments.
//!
//! ## Test Coverage
//!
//! 1. **Basic Enable/Disable** - Verifies fundamental raw mode lifecycle
//! 2. **Flag Verification** - Validates that correct termios flags are set
//! 3. **Input Behavior** - Tests actual character-by-character input processing
//! 4. **Multiple Cycles** - Ensures enable/disable can be called repeatedly
//!
//! ## Manual Tests
//!
//! For tests that require a **real terminal** (e.g., `/dev/tty` fallback with
//! redirected stdin), see [`validation_tests`]. Those tests are marked
//! `#[ignore]` and must be run manually by developers.
//!
//! ## Running Automated Tests
//!
//! Run all integration tests:
//! ```bash
//! cargo test --package r3bl_tui --lib terminal_raw_mode::integration_tests -- --nocapture
//! ```
//!
//! Run a specific test:
//! ```bash
//! cargo test --package r3bl_tui --lib test_raw_mode_pty -- --nocapture
//! ```
//!
//! ## Architecture
//!
//! All tests use the [`generate_pty_test!`] macro which handles:
//! - PTY pair creation (24x80 terminal)
//! - Master/slave process coordination
//! - CI detection (tests skip in CI environments)
//! - Automatic cleanup
//!
//! PTY pairs simulate real terminals, allowing automated verification without
//! user interaction. For edge cases that cannot be simulated with PTYs (like
//! stdin redirection with controlling terminals), see [`validation_tests`].
//!
//! [`generate_pty_test!`]: macro@crate::generate_pty_test
//! [`validation_tests`]: mod@super::validation_tests

// Skip rustfmt for rest of file.
// https://stackoverflow.com/a/75910283/2085356
#![cfg_attr(rustfmt, rustfmt_skip)]

// These tests work on all Unix platforms (no PTY stdin reading).
mod test_basic_enable_disable;
mod test_flag_verification;

// This test verifies exact termios flag restoration which differs between Linux and macOS.
// macOS's tcsetattr sets the PENDIN flag during restoration, causing assertion failures.
#[cfg(target_os = "linux")]
mod test_multiple_cycles;

// This test reads from PTY stdin which hangs on macOS due to kqueue/PTY interaction.
// Linux uses epoll which handles PTY stdin correctly.
#[cfg(target_os = "linux")]
mod test_input_behavior;
