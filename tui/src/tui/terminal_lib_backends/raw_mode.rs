// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words VMIN VTIME

//! High-level raw mode management via the render pipeline.
//!
//! This module provides [`RawMode`], a struct that manages terminal raw mode through
//! the render operation pipeline rather than calling low-level terminal APIs directly.
//!
//! # Backend Dispatch
//!
//! [`RawMode`] does **not** directly enable or disable raw mode. Instead, it creates
//! [`RenderOpOutput::Common(EnterRawMode/ExitRawMode)`][RenderOpOutput] operations that
//! are executed through the render pipeline (Stage 5: Backend Executor).
//!
//! The actual raw mode implementation is selected at compile time via
//! - **Linux** ([`DirectToAnsi`]): Uses rustix-based [`terminal_raw_mode`] module
//! - **macOS/Windows** ([`Crossterm`]): Uses [`crossterm::terminal`] functions
//!
//! # Direct Raw Mode Access
//!
//! For code that needs to enable/disable raw mode directly (outside the render pipeline),
//! use the unified [`enable_raw_mode()`] and [`disable_raw_mode()`] functions instead.
//! These functions dispatch based on [`TERMINAL_LIB_BACKEND`] and are used by
//! readline and other components that manage their own terminal state.
//!
//!
//! # Architecture Context
//!
//! This module is part of a 3-layer raw mode architecture:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │  terminal_lib_backends/raw_mode.rs (This module - High)  ◄──│
//! │  └─ RawMode struct for render pipeline integration          │
//! ├─────────────────────────────────────────────────────────────┤
//! │  terminal_raw_mode/ (Mid-level)                             │
//! │  └─ enable_raw_mode(), disable_raw_mode(), RawModeGuard     │
//! ├─────────────────────────────────────────────────────────────┤
//! │  constants/raw_mode.rs (Low-level)                          │
//! │  └─ VMIN_RAW_MODE, VTIME_RAW_MODE                           │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! **You are here**: The render pipeline layer. This module does **not**
//! directly call terminal APIs—it creates [`RenderOpOutput`] operations
//! executed by the backend.
//!
//! **See also**:
//! - [`terminal_raw_mode`] - Direct raw mode control (for code outside the pipeline)
//! - [`VMIN_RAW_MODE`][vmin] / [`VTIME_RAW_MODE`][vtime] - POSIX termios constants
//!
//! [`Crossterm`]: crate::TerminalLibBackend::Crossterm
//! [`DirectToAnsi`]: crate::TerminalLibBackend::DirectToAnsi
//! [`TERMINAL_LIB_BACKEND`]: crate::TERMINAL_LIB_BACKEND
//! [`disable_raw_mode()`]: crate::disable_raw_mode
//! [`enable_raw_mode()`]: crate::enable_raw_mode
//! [`terminal_raw_mode`]: crate::core::ansi::terminal_raw_mode
//! [vmin]: crate::VMIN_RAW_MODE
//! [vtime]: crate::VTIME_RAW_MODE

use super::RenderOpCommon;
use crate::{LockedOutputDevice, RenderOpOutput, RenderOpOutputVec, RenderOpsExec, Size};

/// High-level raw mode manager for the TUI framework.
///
/// This struct manages terminal raw mode transitions through the render operation
/// pipeline, ensuring proper integration with the 6-stage rendering architecture.
///
/// # Important
///
/// **This struct does not directly call terminal raw mode APIs.** It creates
/// [`RenderOpOutput`] operations that are executed by the backend (Crossterm or
/// `DirectToAnsi`) based on [`TERMINAL_LIB_BACKEND`].
///
/// For direct raw mode control outside the render pipeline, use [`enable_raw_mode()`]
/// and [`disable_raw_mode()`] instead.
///
/// # Usage
///
/// ```no_run
/// # use r3bl_tui::{RawMode, OutputDevice, width, height, lock_output_device_as_mut};
/// let window_size = width(80) + height(24);
/// let output = OutputDevice::new_stdout();
///
/// // Enter raw mode through the render pipeline.
/// RawMode::start(window_size, lock_output_device_as_mut!(&output), false);
///
/// // ... application code ...
///
/// // Exit raw mode through the render pipeline.
/// RawMode::end(window_size, lock_output_device_as_mut!(&output), false);
/// ```
///
/// [`TERMINAL_LIB_BACKEND`]: crate::TERMINAL_LIB_BACKEND
/// [`disable_raw_mode()`]: crate::disable_raw_mode
/// [`enable_raw_mode()`]: crate::enable_raw_mode
#[derive(Debug, Clone)]
pub struct RawMode;

impl RawMode {
    pub fn start(
        window_size: Size,
        locked_output_device: LockedOutputDevice<'_>,
        is_mock: bool,
    ) {
        // Create Output operations for entering raw mode.
        // Raw mode is a terminal state change, so it goes through the Output pipeline.
        let mut ops = RenderOpOutputVec::new();
        ops += RenderOpOutput::Common(RenderOpCommon::EnterRawMode);

        // Execute the operations using the ExecutableRenderOps trait.
        let mut skip_flush = false;
        ops.execute_all(&mut skip_flush, window_size, locked_output_device, is_mock);
    }

    pub fn end(
        window_size: Size,
        locked_output_device: LockedOutputDevice<'_>,
        is_mock: bool,
    ) {
        // Create Output operations for exiting raw mode.
        let mut ops = RenderOpOutputVec::new();
        ops += RenderOpOutput::Common(RenderOpCommon::ExitRawMode);

        // Execute the operations using the ExecutableRenderOps trait.
        let mut skip_flush = false;
        ops.execute_all(&mut skip_flush, window_size, locked_output_device, is_mock);
    }
}
