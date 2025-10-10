// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Edge case and stress test sequences for robustness validation.
//!
//! This module provides challenging sequences including malformed input,
//! boundary conditions, and complex nested sequences that stress-test
//! the ANSI parser's error handling and performance characteristics.
//!
//! ## Edge Case Categories
//!
//! - Buffer overflow scenarios
//! - Rapid state changes
//! - Malformed sequences
//! - Performance stress tests
//! - Boundary condition validation

use super::super::test_fixtures_vt_100_ansi_conformance::nz;
use crate::{ANSIBasicColor, SgrCode, term_col, term_row,
            vt_100_ansi_parser::protocols::csi_codes::CsiSequence};
use std::fmt::Write;

/// Generate a very long text sequence to test buffer handling.
///
/// **Edge Case**: Tests parser performance with large text blocks
/// and ensures proper memory management under stress.
#[must_use]
pub fn long_text_sequence() -> String {
    format!(
        "{}{}{}{}",
        // Move to start.
        CsiSequence::CursorPosition {
            row: term_row(nz(1)),
            col: term_col(nz(1))
        },
        // Large text block (reduced from 10000 to fit tests)
        "A".repeat(200),
        "\n",
        "B".repeat(200)
    )
}

/// Generate rapid style changes to test state management.
///
/// **Edge Case**: Tests parser's ability to handle rapid SGR transitions
/// without state corruption or performance degradation.
#[must_use]
pub fn rapid_style_changes() -> String {
    let mut sequence = String::new();

    // Rapid color cycling using type-safe builders.
    let colors = [
        ANSIBasicColor::Red,
        ANSIBasicColor::Green,
        ANSIBasicColor::Blue,
        ANSIBasicColor::Yellow,
        ANSIBasicColor::Magenta,
        ANSIBasicColor::Cyan,
    ];

    for (i, color) in colors.iter().enumerate() {
        write!(
            sequence,
            "{}{}{}{}",
            SgrCode::ForegroundBasic(*color),
            SgrCode::Bold,
            char::from(b'A' + u8::try_from(i.min(25)).unwrap_or(25)),
            SgrCode::Reset
        )
        .expect("Writing to String should never fail");
    }

    sequence
}

/// Generate sequences with invalid parameters to test error handling.
///
/// **Edge Case**: Tests parser robustness against malformed sequences
/// and ensures graceful degradation.
#[must_use]
pub fn malformed_sequences() -> String {
    format!(
        "{}{}{}{}{}{}",
        // Valid sequence for comparison.
        CsiSequence::CursorPosition {
            row: term_row(nz(1)),
            col: term_col(nz(1))
        },
        "Valid text\n",
        // Sequences with out-of-range parameters.
        "\x1b[999;999H", // Position far beyond buffer
        "OOB test\n",
        "\x1b[0;0H", // Zero position (should default to 1,1)
        "Zero test"
    )
}

/// Generate deeply nested escape sequences.
///
/// **Edge Case**: Tests parser's handling of complex sequence combinations
/// and ensures proper state machine operation.
#[must_use]
pub fn nested_escape_sequences() -> String {
    format!(
        "{}{}{}{}{}{}{}{}{}{}{}",
        // Save cursor.
        CsiSequence::SaveCursor,
        // Set colors and move.
        SgrCode::ForegroundBasic(ANSIBasicColor::Red),
        SgrCode::BackgroundBasic(ANSIBasicColor::Blue),
        CsiSequence::CursorPosition {
            row: term_row(nz(5)),
            col: term_col(nz(5))
        },
        "Nested",
        // Restore and move again.
        CsiSequence::RestoreCursor,
        CsiSequence::CursorPosition {
            row: term_row(nz(2)),
            col: term_col(nz(2))
        },
        SgrCode::Reset,
        SgrCode::Bold,
        "Complex",
        SgrCode::Reset
    )
}

/// Generate boundary condition tests for cursor positioning.
///
/// **Edge Case**: Tests behavior at buffer boundaries and ensures
/// proper bounds checking and clamping.
#[must_use]
pub fn boundary_cursor_tests() -> String {
    format!(
        "{}{}{}{}{}{}",
        // Test upper-left boundary.
        CsiSequence::CursorPosition {
            row: term_row(nz(1)),
            col: term_col(nz(1))
        },
        "UL",
        // Test lower-right boundary (within 10x10 buffer)
        CsiSequence::CursorPosition {
            row: term_row(nz(10)),
            col: term_col(nz(8)) // Leave room for 2-char text
        },
        "LR",
        // Test beyond boundaries (should clamp)
        CsiSequence::CursorPosition {
            row: term_row(nz(15)),
            col: term_col(nz(15))
        },
        "XX"
    )
}
