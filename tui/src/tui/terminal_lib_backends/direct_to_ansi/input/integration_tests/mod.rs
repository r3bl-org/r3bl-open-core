// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! **Input** integration tests for [`DirectToAnsiInputDevice`] (PTY-based).
//!
//! This module documents the PTY-based input integration tests. The actual test
//! implementations live in [`vt_100_terminal_input_parser::integration_tests`] because
//! they primarily validate **parser correctness** (raw bytes → [`InputEvent`]).
//!
//! **Looking for output tests?** See [`output::integration_tests`].
//!
//! # PTY Tests for Input Handling
//!
//! End-to-end PTY tests for [`DirectToAnsiInputDevice`] (the input side of
//! [`DirectToAnsi`]) validate the complete input parsing pipeline in real
//! pseudo-terminals:
//!
//! | Test Module                        | What it validates                     |
//! |:-----------------------------------|:--------------------------------------|
//! | [`pty_input_device_test`]          | Basic async I/O and buffer management |
//! | [`pty_keyboard_modifiers_test`]    | Keyboard modifiers (Shift, Ctrl, Alt) |
//! | [`pty_mouse_events_test`]          | Mouse clicks, drags, scrolling        |
//! | [`pty_terminal_events_test`]       | Focus events, window resize           |
//! | [`pty_utf8_text_test`]             | UTF-8 text input handling             |
//! | [`pty_bracketed_paste_test`]       | Bracketed paste mode                  |
//! | [`pty_new_keyboard_features_test`] | Extended keyboard protocol            |
//! | [`pty_sigwinch_test`]              | SIGWINCH signal handling              |
//!
//! # Why Tests Live in the Parser Module
//!
//! The PTY tests live with the parser ([`vt_100_terminal_input_parser::integration_tests`])
//! rather than here because:
//!
//! 1. **Primary focus is parser correctness**: The tests validate that raw terminal bytes
//!    are correctly parsed into [`InputEvent`] variants.
//!
//! 2. **Parser is the core logic**: [`DirectToAnsiInputDevice`] is essentially a thin
//!    async I/O wrapper around the parser. The interesting behavior being tested is the
//!    parsing, not the I/O.
//!
//! 3. **Testing strategy alignment**: The parser module has a comprehensive testing
//!    strategy with three tiers (validation, unit, integration). The PTY tests fit
//!    naturally as the integration tier.
//!
//! See the [parser module's testing strategy] for the full rationale on validation vs.
//! generated sequences.
//!
//! # Platform Support
//!
//! These PTY tests are **Linux-only** (`#[cfg(target_os = "linux")]`) because
//! [`DirectToAnsiInputDevice`] requires Linux-specific PTY/tty polling capabilities.
//! On macOS/Windows, the Crossterm backend is used instead for input handling.
//!
//! [`DirectToAnsi`]: crate::terminal_lib_backends::direct_to_ansi
//! [`DirectToAnsiInputDevice`]: crate::DirectToAnsiInputDevice
//! [`InputEvent`]: crate::InputEvent
//! [`output::integration_tests`]: mod@crate::terminal_lib_backends::direct_to_ansi::output::integration_tests
//! [`vt_100_terminal_input_parser::integration_tests`]: mod@crate::core::ansi::vt_100_terminal_input_parser::integration_tests
//! [parser module's testing strategy]: mod@crate::core::ansi::vt_100_terminal_input_parser#testing-strategy
//! [`pty_input_device_test`]: mod@crate::core::ansi::vt_100_terminal_input_parser::integration_tests::pty_input_device_test
//! [`pty_keyboard_modifiers_test`]: mod@crate::core::ansi::vt_100_terminal_input_parser::integration_tests::pty_keyboard_modifiers_test
//! [`pty_mouse_events_test`]: mod@crate::core::ansi::vt_100_terminal_input_parser::integration_tests::pty_mouse_events_test
//! [`pty_terminal_events_test`]: mod@crate::core::ansi::vt_100_terminal_input_parser::integration_tests::pty_terminal_events_test
//! [`pty_utf8_text_test`]: mod@crate::core::ansi::vt_100_terminal_input_parser::integration_tests::pty_utf8_text_test
//! [`pty_bracketed_paste_test`]: mod@crate::core::ansi::vt_100_terminal_input_parser::integration_tests::pty_bracketed_paste_test
//! [`pty_new_keyboard_features_test`]: mod@crate::core::ansi::vt_100_terminal_input_parser::integration_tests::pty_new_keyboard_features_test
//! [`pty_sigwinch_test`]: mod@crate::core::ansi::vt_100_terminal_input_parser::integration_tests::pty_sigwinch_test
