// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Centralized ANSI/VT100 escape sequence constants.
//!
//! This module consolidates all ANSI terminal constants into a single,
//! discoverable location organized by protocol type.
//!
//! ## Organization
//!
//! Constants are grouped by protocol domain:
//! - **generic**: Terminal modes, DEC modes, mouse tracking, alternate screen
//! - **csi**: CSI sequences, cursor movement, SGR parameters, color codes
//! - **esc**: ESC sequences, character set selection, C0 control characters
//! - **dsr**: Device Status Report response constants
//! - **sgr**: SGR byte constants for performance-critical paths
//!
//! ## Usage
//!
//! ```rust
//! use r3bl_tui::{CSI_START, SGR_RESET_BYTES, ESC_START};
//!
//! // All constants available with flat imports
//! let csi_start = CSI_START;
//! let sgr_reset = SGR_RESET_BYTES;
//! let escape = ESC_START;
//! ```

// Skip rustfmt for rest of file to preserve manual alignment.
// https://stackoverflow.com/a/75910283/2085356
#![cfg_attr(rustfmt, rustfmt_skip)]

// Private modules (hide internal structure).
mod csi;
mod dsr;
mod esc;
mod generic;
mod input_sequences;
mod sgr;

// Public re-exports (flat API) for convenience.
pub use csi::*;
pub use dsr::*;
pub use esc::*;
pub use generic::*;
pub use input_sequences::*;
pub use sgr::*;
