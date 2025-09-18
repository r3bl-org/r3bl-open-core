// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Display manipulation sequence patterns for screen control operations.
//!
//! This module provides sequences for screen clearing, scrolling, line operations,
//! and other display management functions commonly used by terminal applications.

use crate::vt100_ansi_parser::protocols::csi_codes::CsiSequence;

/// Clear entire screen (placeholder - to be expanded).
#[must_use]
pub fn clear_screen() -> String { CsiSequence::EraseDisplay(2).to_string() }

/// Clear from cursor to end of screen (placeholder - to be expanded).
#[must_use]
pub fn clear_to_end_of_screen() -> String { CsiSequence::EraseDisplay(0).to_string() }

// TODO: Add more display sequences:
// - Scroll operations
// - Line insertion/deletion
// - Margin control
// - Window operations
