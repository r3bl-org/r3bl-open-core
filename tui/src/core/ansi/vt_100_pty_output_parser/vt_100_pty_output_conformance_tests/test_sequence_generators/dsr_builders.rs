// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Test convenience builders for DSR (Device Status Report) sequences.
//!
//! This module provides convenience functions for generating DSR response sequences
//! in tests. These are simple wrappers around manual string formatting that could
//! alternatively use `DsrSequence::to_string()`.
//!
//! # Purpose
//!
//! While [`DsrSequence`] provides type-safe bidirectional sequence generation,
//! this builder provides a direct formatting approach that's convenient for
//! test assertions.
//!
//! [`DsrSequence`]: crate::DsrSequence

use crate::{CSI_PARAM_SEPARATOR, DSR_CURSOR_POSITION_RESPONSE_END, DSR_RESPONSE_START,
            TermCol, TermRow};

/// Generate DSR cursor position response: ESC[row;colR
///
/// Creates a properly formatted cursor position report response that would be
/// sent from the terminal back to the application in response to `CSI 6n`.
///
/// # Parameters
/// - `row`: Current cursor row (1-based terminal coordinates)
/// - `col`: Current cursor column (1-based terminal coordinates)
///
/// # Returns
/// Formatted DSR response string: `\x1b[{row};{col}R`
#[must_use]
pub fn dsr_cursor_position_response(row: TermRow, col: TermCol) -> String {
    format!(
        "{DSR_RESPONSE_START}{}{CSI_PARAM_SEPARATOR}{}{DSR_CURSOR_POSITION_RESPONSE_END}",
        row.as_u16(),
        col.as_u16()
    )
}
