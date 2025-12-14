// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Platform-specific terminal backend selection.
//!
//! This module defines the available terminal backends and selects the optimal one
//! for the current platform at compile time.
//!
//! # Raw Mode
//!
//! For raw mode operations, use [`enable_raw_mode()`] and [`disable_raw_mode()`] from
//! the [`terminal_raw_mode`] module. These functions automatically dispatch to the
//! correct implementation based on [`TERMINAL_LIB_BACKEND`].
//!
//! [`enable_raw_mode()`]: crate::enable_raw_mode
//! [`disable_raw_mode()`]: crate::disable_raw_mode
//! [`terminal_raw_mode`]: crate::core::ansi::terminal_raw_mode

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
