// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Manual validation tests for terminal raw mode functionality.
//!
//! These tests **require a real terminal** and cannot be automated in CI. They are
//! marked with `#[ignore]` and must be run manually.
//!
//! ## Why These Tests Cannot Be Automated
//!
//! Unlike the automated tests in [`integration_tests`], these tests verify behavior
//! that fundamentally requires a real controlling terminal provided by a shell:
//!
//! - **`/dev/tty` fallback**: Tests that raw mode works when stdin is redirected
//!   (e.g., `echo "data" | app`). The shell provides a controlling terminal even
//!   when stdin is piped, but test harnesses spawn processes without controlling
//!   terminals, making automated verification impossible.
//!
//! ## Running Validation Tests
//!
//! Run all validation tests:
//! ```bash
//! cargo test --package r3bl_tui --lib terminal_raw_mode::validation_tests -- --ignored --nocapture
//! ```
//!
//! Run a specific validation test:
//! ```bash
//! echo "test" | cargo test --package r3bl_tui --lib test_dev_tty_fallback_manual -- --ignored --nocapture
//! ```
//!
//! ## Architecture: Automated vs Manual Testing
//!
//! ```text
//! terminal_raw_mode/
//! ├── integration_tests/    ← Automated PTY tests (run in CI)
//! │   ├── test_basic_enable_disable.rs
//! │   ├── test_flag_verification.rs
//! │   ├── test_input_behavior.rs
//! │   └── test_multiple_cycles.rs
//! │
//! └── validation_tests/     ← Manual real-terminal tests (run by developers)
//!     └── test_dev_tty_fallback_manual.rs
//! ```
//!
//! **Integration tests** use [`portable_pty`] to create virtual terminals, allowing
//! automated verification of raw mode behavior without user interaction.
//!
//! **Validation tests** require real shell environments and user actions, verifying
//! edge cases that cannot be simulated with PTY pairs.
//!
//! [`integration_tests`]: mod@super::integration_tests
//! [`portable_pty`]: https://docs.rs/portable-pty

// Skip rustfmt for rest of file.
// https://stackoverflow.com/a/75910283/2085356
#![cfg_attr(rustfmt, rustfmt_skip)]

mod test_dev_tty_fallback_manual;
