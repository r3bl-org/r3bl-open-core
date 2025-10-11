// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Test convenience builders for CSI sequences.
//!
//! This module provides convenience functions for working with [`CsiSequence`] in tests.
//! These are simple wrappers that validate or convert CSI sequence variants for
//! ergonomic test code.
//!
//! # Purpose
//!
//! While [`CsiSequence`] can be constructed directly in production code, these builders
//! provide validation and conversion utilities specifically for test scenarios.
//!
//! ## Provided Functions
//!
//! - [`csi_seq_cursor_pos`] - Validates and returns a [`CsiSequence::CursorPosition`]
//! - [`csi_seq_cursor_pos_alt`] - Converts to or validates
//!   [`CsiSequence::CursorPositionAlt`]
//!
//! # Example Usage
//!
//! ```rust,ignore
//! use crate::vt_100_ansi_conformance_tests::test_sequence_builders::csi_builders::*;
//!
//! let cursor_pos = csi_seq_cursor_pos(
//!     CsiSequence::CursorPosition {
//!         row: term_row(nz(5)),
//!         col: term_col(nz(10))
//!     }
//! );
//! ```
//!
//! [`CsiSequence`]: crate::protocols::csi_codes::CsiSequence
//! [`CsiSequence::CursorPosition`]: crate::protocols::csi_codes::CsiSequence::CursorPosition
//! [`CsiSequence::CursorPositionAlt`]: crate::protocols::csi_codes::CsiSequence::CursorPositionAlt

use crate::protocols::csi_codes::CsiSequence;

/// Helper function to create a `CsiSequence::CursorPosition`.
///
/// # Panics
/// Panics if the provided position is not a `CsiSequence::CursorPosition`.
#[must_use]
pub fn csi_seq_cursor_pos(position: CsiSequence) -> CsiSequence {
    match position {
        CsiSequence::CursorPosition { .. } => position,
        _ => panic!("Expected CsiSequence::CursorPosition"),
    }
}

/// Helper function to create a `CsiSequence::CursorPositionAlt`.
///
/// # Panics
/// Panics if the provided position is not a `CsiSequence::CursorPosition` or
/// `CursorPositionAlt`.
#[must_use]
pub fn csi_seq_cursor_pos_alt(position: CsiSequence) -> CsiSequence {
    match position {
        CsiSequence::CursorPosition { row, col } => {
            CsiSequence::CursorPositionAlt { row, col }
        }
        CsiSequence::CursorPositionAlt { .. } => position,
        _ => panic!("Expected CsiSequence::CursorPosition or CursorPositionAlt"),
    }
}
