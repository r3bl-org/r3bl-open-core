// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Backend compatibility tests for input and output backends.
//!
//! These tests verify that different backends produce consistent results:
//! - **Input tests**: Verify [`DirectToAnsiInputDevice`] and [`CrosstermInputDevice`]
//!   produce identical [`InputEvent`] values for the same ANSI byte sequences.
//! - **Output tests**: Verify [`RenderOpPaintImplDirectToAnsi`] and
//!   [`PaintRenderOpImplCrossterm`] produce visually identical terminal output for the
//!   same [`RenderOpOutput`] sequences.
//!
//! # Test Strategy
//!
//! Both input and output tests use PTY-based process isolation:
//! - Each backend runs in a separate subprocess for isolation
//! - Results are compared by the main comparison test
//!
//! ## Input Tests
//!
//! - A controller process writes ANSI bytes to the PTY master
//! - A controlled process reads from stdin (PTY slave) using the specific backend
//! - The parsed [`InputEvent`] is output for verification
//!
//! ## Output Tests
//!
//! - A controlled process executes [`RenderOpOutput`] via the specific backend
//! - ANSI output is captured and written to stdout (PTY slave)
//! - The main test applies both outputs to [`OffscreenBuffer`] and compares
//!
//! # Platform Support
//!
//! All tests are **Linux-only** because backend compatibility comparison requires
//! both [`DirectToAnsiInputDevice`] and the `DirectToAnsi` output backend, which are
//! only available on Linux.
//!
//! | Test                                  | Linux   | macOS   | Windows   |
//! | :------------------------------------ | :------ | :------ | :-------- |
//! | [`test_pty_backend_direct_to_ansi`]   | ✅      | ❌      | ❌        |
//! | [`test_pty_backend_crossterm`]        | ✅      | ❌      | ❌        |
//! | [`test_backend_compat_input_compare`] | ✅      | ❌      | ❌        |
//! | [`test_backend_compat_output_compare`]| ✅      | ❌      | ❌        |
//!
//! [`CrosstermInputDevice`]: crate::CrosstermInputDevice
//! [`DirectToAnsiInputDevice`]: crate::direct_to_ansi::DirectToAnsiInputDevice
//! [`InputDevice`]: crate::InputDevice
//! [`InputEvent`]: crate::InputEvent
//! [`OffscreenBuffer`]: crate::OffscreenBuffer
//! [`PaintRenderOpImplCrossterm`]: crate::tui::terminal_lib_backends::crossterm_backend::PaintRenderOpImplCrossterm
//! [`RenderOpOutput`]: crate::RenderOpOutput
//! [`RenderOpPaintImplDirectToAnsi`]: crate::direct_to_ansi::RenderOpPaintImplDirectToAnsi
//! [`test_backend_compat_input_compare`]: mod@backend_compat_input_test
//! [`test_backend_compat_output_compare`]: mod@backend_compat_output_test
//! [`test_pty_backend_crossterm`]: mod@backend_compat_input_test
//! [`test_pty_backend_direct_to_ansi`]: mod@backend_compat_input_test

// Public for docs and tests so intra-doc links resolve.
// Doc builds are allowed on Unix platforms (macOS/Linux) where the dependencies exist.
// Windows doc builds exclude these since signal_hook/mio::unix are unavailable.
#[cfg(any(all(unix, doc), all(target_os = "linux", test)))]
pub mod backend_compat_input_test;

#[cfg(any(all(unix, doc), all(target_os = "linux", test)))]
pub mod backend_compat_output_test;
