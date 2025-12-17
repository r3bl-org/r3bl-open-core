// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Display manipulation sequence patterns for screen control operations.
//!
//! This module provides sequences for screen clearing, scrolling, line operations,
//! and other display management functions commonly used by terminal applications.

use crate::{CsiSequence, EraseDisplayMode};

/// Clear entire screen (placeholder - to be expanded).
#[must_use]
pub fn clear_screen() -> String {
    CsiSequence::EraseDisplay(EraseDisplayMode::EntireScreen).to_string()
}

/// Clear from cursor to end of screen (placeholder - to be expanded).
#[must_use]
pub fn clear_to_end_of_screen() -> String {
    CsiSequence::EraseDisplay(EraseDisplayMode::FromCursorToEnd).to_string()
}

// TODO: Post-Step 6 - Expand display sequence library
// (Deferred: Advanced VT-100 features for future implementation)
// Future additions:
//   - Scroll operations (scrolling regions)
//   - Line insertion/deletion (IL/DL commands)
//   - Margin control (DECSTBM margins)
//   - Window operations (Sixel, ReGIS graphics)
