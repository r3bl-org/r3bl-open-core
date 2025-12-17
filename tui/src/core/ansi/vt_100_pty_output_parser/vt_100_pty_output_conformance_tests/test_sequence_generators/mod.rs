// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// Allow doc_markdown - documentation explains internal test patterns where backticking
// every function/type would reduce readability.
#![allow(clippy::doc_markdown)]

//! Test utilities for building VT100 ANSI sequences.
//!
//! This module provides convenience wrappers around the bidirectional sequence types
//! ([`SgrColorSequence`], [`DsrSequence`], [`CsiSequence`], etc.) that enable
//! ergonomic sequence generation in tests.
//!
//! # Purpose
//!
//! These builders are **test-only convenience functions** (guarded by `#[cfg(any(test,
//! doc))]`) that wrap the underlying production sequence generation capabilities.
//!
//! While the sequence types themselves are production code with bidirectional support
//! (parsing + generation via [`Display`]/[`FastStringify`]), these helpers provide
//! simpler APIs for test code.
//!
//! # Why Separate from Production Code?
//!
//! The underlying types (`SgrColorSequence`, `DsrSequence`, etc.) are production
//! types that can generate sequences using `.to_string()`. However, these convenience
//! functions:
//! - Are only needed in tests (production code constructs enums directly)
//! - Provide shorter, more ergonomic APIs for test readability
//! - Are consolidated here for discoverability and consistency
//!
//! # Example Usage
//!
//! <!-- It is ok to use ignore here - demonstrates usage of test helper functions in
//! conditionally compiled modules -->
//!
//! ```ignore
//! use crate::vt_100_pty_output_conformance_tests::test_sequence_generators::*;
//!
//! // Extended color sequences
//! let fg = fg_ansi256(196);                 // → "\x1b[38:5:196m"
//! let bg = bg_rgb(255, 128, 0);             // → "\x1b[48:2:255:128:0m"
//!
//! // DSR sequences
//! let cursor_pos = dsr_cursor_position_response(
//!     term_row(nz(10)),
//!     term_col(nz(25))
//! );                                         // → "\x1b[10;25R"
//!
//! // CSI sequences
//! let cursor_move = csi_seq_cursor_pos(
//!     CsiSequence::CursorPosition {
//!         row: term_row(nz(5)),
//!         col: term_col(nz(10))
//!     }
//! );
//! ```
//!
//! [`SgrColorSequence`]: crate::SgrColorSequence
//! [`DsrSequence`]: crate::DsrSequence
//! [`CsiSequence`]: crate::CsiSequence
//! [`Display`]: std::fmt::Display
//! [`FastStringify`]: crate::core::common::fast_stringify::FastStringify

pub mod csi_builders;
pub mod dsr_builders;
pub mod extended_color_builders;

// Re-export all builders for convenient wildcard imports
pub use csi_builders::*;
pub use dsr_builders::*;
pub use extended_color_builders::*;
