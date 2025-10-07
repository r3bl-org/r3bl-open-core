// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Test helper functions for CSI sequences.
//!
//! This module provides utility functions for testing and working with [`CsiSequence`]
//! in test and documentation contexts. These helpers are used across the
//! `vt_100_ansi_parser` module for validating CSI sequence behavior.
//!
//! ## Provided Functions
//!
//! - [`csi_seq_cursor_pos`] - Validates and returns a [`CsiSequence::CursorPosition`]
//! - [`csi_seq_cursor_pos_alt`] - Converts to or validates
//!   [`CsiSequence::CursorPositionAlt`]
//!
//! ## Cross-Module Usage
//!
//! These test helpers are used in:
//! - [`crate::vt_100_ansi_parser::ansi_parser_public_api`] - Unit tests
//! - [`crate::vt_100_ansi_parser::vt_100_ansi_conformance_tests::tests`] - Integration
//!   tests (collection of operation-specific test modules)
//!
//! The module is conditionally compiled with `#[cfg(any(test, doc))]` to ensure
//! test utilities are only available during testing and documentation generation.
//!
//! [`CsiSequence`]: super::sequence::CsiSequence
//! [`CsiSequence::CursorPosition`]: super::sequence::CsiSequence::CursorPosition
//! [`CsiSequence::CursorPositionAlt`]: super::sequence::CsiSequence::CursorPositionAlt

use super::sequence::CsiSequence;

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
