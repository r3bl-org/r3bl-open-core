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
//! - **[`input_sequences`]**: Keyboard input, control characters, mouse protocol markers
//! - **[`raw_mode`]**: Raw mode terminal configuration (VMIN, VTIME)
//! - **[`sgr`]**: SGR byte constants for performance-critical paths
//! - **[`utf8`]**: UTF-8 encoding constants for byte-level text parsing
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

// Module is public only when building documentation or tests.
// This allows rustdoc links to work while keeping it private in release builds.
#[cfg(any(test, doc))]
pub mod csi;
#[cfg(not(any(test, doc)))]
mod csi;
#[cfg(any(test, doc))]
pub mod dsr;
#[cfg(not(any(test, doc)))]
mod dsr;
#[cfg(any(test, doc))]
pub mod esc;
#[cfg(not(any(test, doc)))]
mod esc;
#[cfg(any(test, doc))]
pub mod generic;
#[cfg(not(any(test, doc)))]
mod generic;
#[cfg(any(test, doc))]
pub mod input_sequences;
#[cfg(not(any(test, doc)))]
mod input_sequences;
#[cfg(any(test, doc))]
pub mod raw_mode;
#[cfg(not(any(test, doc)))]
mod raw_mode;
#[cfg(any(test, doc))]
pub mod sgr;
#[cfg(not(any(test, doc)))]
mod sgr;
#[cfg(any(test, doc))]
pub mod utf8;
#[cfg(not(any(test, doc)))]
mod utf8;

// Public re-exports (flat API) for convenience.
pub use csi::*;
pub use dsr::*;
pub use esc::*;
pub use generic::*;
pub use input_sequences::*;
pub use raw_mode::*;
pub use sgr::*;
pub use utf8::*;
