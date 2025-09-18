// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Scroll margin operations for VT100/ANSI terminal emulation.
//!
//! This module implements scroll margin operations that correspond to ANSI
//! sequences handled by the `vt_100_ansi_parser::operations::margin_ops` module. These
//! include:
//!
//! - **DECSTBM** (Set Top and Bottom Margins) - `set_scroll_margins`
//! - **Reset margins** - `reset_scroll_margins`
//!
//! All operations maintain VT100 compliance and handle proper scroll region
//! boundaries for terminal operations.

use std::cmp::min;

#[allow(clippy::wildcard_imports)]
use super::super::*;
use crate::core::pty_mux::vt_100_ansi_parser::term_units::TermRow;

impl OffscreenBuffer {
    /// Reset scroll margins to full screen (no restrictions).
    /// This disables any active scroll region and allows operations
    /// to affect the entire buffer.
    pub fn reset_scroll_margins(&mut self) {
        self.ansi_parser_support.scroll_region_top = None;
        self.ansi_parser_support.scroll_region_bottom = None;
    }

    /// Set top and bottom scroll margins for the buffer.
    /// Operations like scrolling and line insertion/deletion will be
    /// restricted to this region.
    ///
    /// Returns true if the margins were set successfully.
    pub fn set_scroll_margins(&mut self, top: TermRow, bottom: TermRow) -> bool {
        let buffer_height: u16 = self.window_size.row_height.into();
        let top_value = top.as_u16();
        let bottom_value = bottom.as_u16();

        // Validate margins against buffer height.
        let clamped_bottom = min(bottom_value, buffer_height);

        if top_value < clamped_bottom && clamped_bottom <= buffer_height {
            self.ansi_parser_support.scroll_region_top = Some(top);
            self.ansi_parser_support.scroll_region_bottom = Some(
                crate::core::pty_mux::vt_100_ansi_parser::term_units::term_row(
                    clamped_bottom,
                ),
            );
            true
        } else {
            tracing::warn!(
                "Invalid scroll margins: top={}, bottom={}, buffer_height={}",
                top_value,
                bottom_value,
                buffer_height
            );
            false
        }
    }
}

#[cfg(test)]
mod tests_margin_ops {
    use super::*;
    use crate::{core::pty_mux::vt_100_ansi_parser::term_units::term_row, height, width};

    fn create_test_buffer() -> OffscreenBuffer {
        let size = width(10) + height(6);
        OffscreenBuffer::new_empty(size)
    }

    #[test]
    fn test_reset_scroll_margins() {
        let mut buffer = create_test_buffer();

        // Set some margins first
        buffer.ansi_parser_support.scroll_region_top = Some(term_row(2));
        buffer.ansi_parser_support.scroll_region_bottom = Some(term_row(4));

        buffer.reset_scroll_margins();

        // Should be reset to None
        assert!(buffer.ansi_parser_support.scroll_region_top.is_none());
        assert!(buffer.ansi_parser_support.scroll_region_bottom.is_none());
    }

    #[test]
    fn test_set_scroll_margins_valid() {
        let mut buffer = create_test_buffer();

        let result = buffer.set_scroll_margins(term_row(2), term_row(4));
        assert!(result);

        // Check that margins were set
        assert_eq!(
            buffer.ansi_parser_support.scroll_region_top,
            Some(term_row(2))
        );
        assert_eq!(
            buffer.ansi_parser_support.scroll_region_bottom,
            Some(term_row(4))
        );
    }

    #[test]
    fn test_set_scroll_margins_invalid_top_greater_than_bottom() {
        let mut buffer = create_test_buffer();

        let result = buffer.set_scroll_margins(term_row(4), term_row(2));
        assert!(!result);

        // Margins should remain unchanged (None)
        assert!(buffer.ansi_parser_support.scroll_region_top.is_none());
        assert!(buffer.ansi_parser_support.scroll_region_bottom.is_none());
    }

    #[test]
    fn test_set_scroll_margins_bottom_exceeds_buffer() {
        let mut buffer = create_test_buffer();

        // Try to set bottom margin beyond buffer height (buffer height is 6)
        let result = buffer.set_scroll_margins(term_row(2), term_row(10));
        assert!(result); // Should succeed with clamping

        // Bottom should be clamped to buffer height
        assert_eq!(
            buffer.ansi_parser_support.scroll_region_top,
            Some(term_row(2))
        );
        assert_eq!(
            buffer.ansi_parser_support.scroll_region_bottom,
            Some(term_row(6))
        );
    }

    #[test]
    fn test_set_scroll_margins_equal_top_and_bottom() {
        let mut buffer = create_test_buffer();

        let result = buffer.set_scroll_margins(term_row(3), term_row(3));
        assert!(!result); // Should fail - no room for content

        // Margins should remain unchanged
        assert!(buffer.ansi_parser_support.scroll_region_top.is_none());
        assert!(buffer.ansi_parser_support.scroll_region_bottom.is_none());
    }
}
