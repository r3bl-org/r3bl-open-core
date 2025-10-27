// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Centralized ANSI/VT100 escape sequence constants.
//!
//! This module consolidates all ANSI terminal constants into a single,
//! discoverable location organized by protocol type.
//!
//! ## Organization
//!
//! Constants are grouped by protocol domain:
//! - **[`generic`]**: Terminal modes, DEC modes, mouse tracking, alternate screen
//! - **[`csi`]**: CSI sequences, cursor movement, SGR parameters, color codes
//! - **[`esc`]**: ESC sequences, character set selection, C0 control characters
//! - **[`dsr`]**: Device Status Report response constants
//! - **[`sgr`]**: SGR byte constants for performance-critical paths
//!
//! ## Usage
//!
//! ```rust
//! use r3bl_tui::core::ansi::constants::*;
//!
//! // All constants available with flat imports
//! let csi_start = CSI_START;
//! let sgr_reset = SGR_RESET_BYTES;
//! let escape = ESC_START;
//! ```

// Public submodules for organized access
pub mod csi;
pub mod dsr;
pub mod esc;
pub mod generic;
pub mod sgr;

// Public re-exports (flat API) for convenience
pub use csi::*;
pub use dsr::*;
pub use esc::*;
pub use generic::*;
pub use sgr::*;
