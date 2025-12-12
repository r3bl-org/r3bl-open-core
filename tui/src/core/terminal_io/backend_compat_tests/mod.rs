// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Backend compatibility tests for [`InputDevice`] implementations.
//!
//! These tests verify that different input backends produce consistent [`InputEvent`]
//! values when given identical ANSI byte sequences.
//!
//! # Test Strategy
//!
//! Each backend is tested in isolation using PTY-based integration tests:
//! - A controller process writes ANSI bytes to the PTY master
//! - A controlled process reads from stdin (PTY slave) using the specific backend
//! - The parsed [`InputEvent`] is output for verification
//!
//! # Platform Support
//!
//! All tests are **Linux-only** because backend compatibility comparison requires
//! both [`DirectToAnsiInputDevice`] and [`CrosstermInputDevice`], and [`DirectToAnsi`]
//! is only available on Linux.
//!
//! | Test                                      | Linux   | macOS   | Windows   |
//! | :---------------------------------------- | :------ | :------ | :-------- |
//! | [`test_pty_backend_direct_to_ansi`]       | ✅      | ❌      | ❌        |
//! | [`test_pty_backend_crossterm`]            | ✅      | ❌      | ❌        |
//! | [`test_backend_compatibility_comparison`] | ✅      | ❌      | ❌        |
//!
//! [`CrosstermInputDevice`]: crate::CrosstermInputDevice
//! [`DirectToAnsi`]: mod@crate::tui::terminal_lib_backends::direct_to_ansi
//! [`DirectToAnsiInputDevice`]: crate::tui::terminal_lib_backends::direct_to_ansi::DirectToAnsiInputDevice
//! [`InputDevice`]: crate::InputDevice
//! [`InputEvent`]: crate::InputEvent
//! [`test_backend_compatibility_comparison`]: backend_compatibility_test::test_backend_compatibility_comparison
//! [`test_pty_backend_crossterm`]: backend_compatibility_test::test_pty_backend_crossterm
//! [`test_pty_backend_direct_to_ansi`]: backend_compatibility_test::test_pty_backend_direct_to_ansi

#[cfg(test)]
pub mod backend_compatibility_test;
