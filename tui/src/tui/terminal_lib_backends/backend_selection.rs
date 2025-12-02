// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Platform-specific terminal backend selection and unified raw mode API.
//!
//! This module defines the available terminal backends and selects the optimal one
//! for the current platform at compile time. It also provides unified raw mode
//! functions that dispatch to the correct backend implementation.

use crate::DEBUG_TUI_SHOW_TERMINAL_BACKEND;
use miette::IntoDiagnostic;

/// Terminal library backend selection for the TUI system.
///
/// R3BL TUI supports multiple terminal manipulation libraries, allowing users to choose
/// the backend that best fits their needs. Currently supported backends include:
///
/// - **Crossterm**: Cross-platform terminal library (default and recommended)
/// - **`DirectToAnsi`**: Pure Rust ANSI sequence generation without external dependencies
#[derive(Debug)]
pub enum TerminalLibBackend {
    /// Cross-platform terminal library (default).
    Crossterm,
    /// Pure Rust ANSI sequence generation.
    DirectToAnsi,
}

/// The default terminal library backend for this platform.
///
/// On **Linux**, [`DirectToAnsi`] is selected for pure Rust ANSI sequence generation
/// without external dependencies.
///
/// # Platform Selection
///
/// R3BL TUI uses platform-specific backends:
/// - **Linux**: [`DirectToAnsi`] (pure Rust async I/O)
/// - **macOS/Windows**: Crossterm (cross-platform compatibility)
///
/// # Performance
///
/// [`DirectToAnsi`] achieves ~18% better performance than Crossterm on Linux through:
/// - Stack-allocated number formatting (eliminates heap allocations)
/// - `SmallVec[16]` for render operations (+0.47%)
/// - `StyleUSSpan[16]` for styled text spans (+~5.0%)
///
/// Benchmarked using 8-second continuous workload with 999Hz sampling and scripted
/// input (see `script_lib.fish::run_example_with_flamegraph_profiling_perf_fold`).
///
/// [`DirectToAnsi`]: variant@TerminalLibBackend::DirectToAnsi
#[cfg(target_os = "linux")]
pub const TERMINAL_LIB_BACKEND: TerminalLibBackend = TerminalLibBackend::DirectToAnsi;

/// The default terminal library backend for this platform.
///
/// On **macOS/Windows**, Crossterm is selected for its mature cross-platform
/// support and compatibility across different terminal emulators.
///
/// # Platform Selection
///
/// R3BL TUI uses platform-specific backends:
/// - **Linux**: [`DirectToAnsi`] (pure Rust async I/O)
/// - **macOS/Windows**: Crossterm (cross-platform compatibility)
///
/// [`DirectToAnsi`]: variant@TerminalLibBackend::DirectToAnsi
#[cfg(not(target_os = "linux"))]
pub const TERMINAL_LIB_BACKEND: TerminalLibBackend = TerminalLibBackend::Crossterm;

/// Enable raw mode using the appropriate backend.
///
/// Dispatches to the correct raw mode implementation based on [`TERMINAL_LIB_BACKEND`]:
/// - **Linux** ([`DirectToAnsi`]): Uses rustix-based [`enable_raw_mode()`][rustix_enable]
///   from [`terminal_raw_mode`]
/// - **macOS/Windows** ([`Crossterm`]): Uses [`crossterm::terminal::enable_raw_mode()`]
///
/// This ensures consistent raw mode handling regardless of which terminal backend is
/// active, preventing state corruption when different parts of the codebase manage
/// terminal state.
///
/// # Errors
///
/// Returns an error if the terminal cannot be put into raw mode (e.g., not a TTY).
///
/// [`DirectToAnsi`]: TerminalLibBackend::DirectToAnsi
/// [`Crossterm`]: TerminalLibBackend::Crossterm
/// [`terminal_raw_mode`]: crate::core::ansi::terminal_raw_mode
/// [rustix_enable]: crate::enable_raw_mode
pub fn raw_mode_enable() -> miette::Result<()> {
    DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
        tracing::debug!(
            message = "raw_mode_enable: 🟢 enabling raw mode",
            backend = ?TERMINAL_LIB_BACKEND
        );
    });

    let result = match TERMINAL_LIB_BACKEND {
        TerminalLibBackend::DirectToAnsi => crate::enable_raw_mode(),
        TerminalLibBackend::Crossterm => {
            crossterm::terminal::enable_raw_mode().into_diagnostic()
        }
    };

    match &result {
        Ok(()) => DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
            tracing::debug!(message = "raw_mode_enable: ✅ success");
        }),
        Err(e) => DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
            tracing::error!(message = "raw_mode_enable: ❌ failed", error = %e);
        }),
    };

    result
}

/// Disable raw mode using the appropriate backend.
///
/// Dispatches to the correct raw mode implementation based on [`TERMINAL_LIB_BACKEND`]:
/// - **Linux** ([`DirectToAnsi`]): Uses rustix-based [`disable_raw_mode()`][rustix_disable]
///   from [`terminal_raw_mode`]
/// - **macOS/Windows** ([`Crossterm`]): Uses [`crossterm::terminal::disable_raw_mode()`]
///
/// # Errors
///
/// Returns an error if the terminal state cannot be restored.
///
/// [`DirectToAnsi`]: TerminalLibBackend::DirectToAnsi
/// [`Crossterm`]: TerminalLibBackend::Crossterm
/// [`terminal_raw_mode`]: crate::core::ansi::terminal_raw_mode
/// [rustix_disable]: crate::disable_raw_mode
pub fn raw_mode_disable() -> miette::Result<()> {
    DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
        tracing::debug!(
            message = "raw_mode_disable: 🔴 disabling raw mode",
            backend = ?TERMINAL_LIB_BACKEND
        );
    });

    let result = match TERMINAL_LIB_BACKEND {
        TerminalLibBackend::DirectToAnsi => crate::disable_raw_mode(),
        TerminalLibBackend::Crossterm => {
            crossterm::terminal::disable_raw_mode().into_diagnostic()
        }
    };

    match &result {
        Ok(()) => DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
            tracing::debug!(message = "raw_mode_disable: ✅ success");
        }),
        Err(e) => DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
            tracing::error!(message = "raw_mode_disable: ❌ failed", error = %e);
        }),
    };

    result
}
