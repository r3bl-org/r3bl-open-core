// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.
//! ANSI terminal scroll helper operations for `OffscreenBuffer`.
//!
//! This module provides helper methods for ANSI escape sequence scrolling operations,
//! including scroll region boundary detection and row clamping within defined scroll
//! areas.

#[allow(clippy::wildcard_imports)]
use super::super::*;
use crate::{RowIndex,
            core::{pty_mux::vt_100_ansi_parser::term_units::TermRow,
                   units::bounds_check::LengthMarker},
            row};

impl OffscreenBuffer {
    /// Get the top boundary of the scroll region (0 if no region set).
    ///
    /// This resolves the ANSI parser's scroll region top boundary, converting
    /// from 1-based ANSI coordinates to 0-based buffer indices.
    pub fn get_scroll_top_boundary(&self) -> RowIndex {
        self.ansi_parser_support
            .scroll_region_top
            .and_then(TermRow::to_zero_based) // Convert 1 to 0 based.
            .map_or(/* None */ row(0), /* Some */ Into::into)
    }

    /// Get the bottom boundary of the scroll region (screen bottom if no region set).
    ///
    /// This resolves the ANSI parser's scroll region bottom boundary, converting
    /// from 1-based ANSI coordinates to 0-based buffer indices.
    pub fn get_scroll_bottom_boundary(&self) -> RowIndex {
        self.ansi_parser_support
            .scroll_region_bottom
            .and_then(TermRow::to_zero_based) // Convert 1 to 0 based.
            .map_or(
                /* None */ self.window_size.row_height.convert_to_index(),
                /* Some */ Into::into,
            )
    }

    /// Clamp a row to stay within the scroll region boundaries.
    ///
    /// This ensures row positions respect ANSI scroll region settings,
    /// keeping the cursor within the defined scrollable area.
    #[must_use]
    pub fn clamp_row_to_scroll_region(&self, row: RowIndex) -> RowIndex {
        let top = self.get_scroll_top_boundary();
        let bottom = self.get_scroll_bottom_boundary();

        // Use Rust's built-in clamp for type-safe range clamping.
        row.clamp(top, bottom)
    }
}

#[cfg(test)]
mod tests_bounds_check_ops {
    use super::*;
    use crate::{core::pty_mux::vt_100_ansi_parser::term_units::term_row, height, width};

    fn create_test_buffer() -> OffscreenBuffer {
        let size = width(10) + height(6);
        OffscreenBuffer::new_empty(size)
    }

    #[test]
    fn test_get_scroll_top_boundary_no_region() {
        let buffer = create_test_buffer();

        // No scroll region set - should return row 0
        assert_eq!(buffer.get_scroll_top_boundary(), row(0));
    }

    #[test]
    fn test_get_scroll_top_boundary_with_region() {
        let mut buffer = create_test_buffer();

        // Set scroll region top to row 3 (1-based)
        buffer.ansi_parser_support.scroll_region_top = Some(term_row(3));

        // Should return row 2 (0-based)
        assert_eq!(buffer.get_scroll_top_boundary(), row(2));
    }

    #[test]
    fn test_get_scroll_bottom_boundary_no_region() {
        let buffer = create_test_buffer();

        // No scroll region set - should return max row index (height 6 = max index 5)
        assert_eq!(buffer.get_scroll_bottom_boundary(), row(5));
    }

    #[test]
    fn test_get_scroll_bottom_boundary_with_region() {
        let mut buffer = create_test_buffer();

        // Set scroll region bottom to row 4 (1-based)
        buffer.ansi_parser_support.scroll_region_bottom = Some(term_row(4));

        // Should return row 3 (0-based)
        assert_eq!(buffer.get_scroll_bottom_boundary(), row(3));
    }

    #[test]
    fn test_clamp_row_to_scroll_region_no_region() {
        let buffer = create_test_buffer();

        // No scroll region - row should remain unchanged
        assert_eq!(buffer.clamp_row_to_scroll_region(row(3)), row(3));
    }

    #[test]
    fn test_clamp_row_to_scroll_region_within_bounds() {
        let mut buffer = create_test_buffer();

        // Set scroll region from row 2 to row 4 (1-based: 3 to 5)
        buffer.ansi_parser_support.scroll_region_top = Some(term_row(3));
        buffer.ansi_parser_support.scroll_region_bottom = Some(term_row(5));

        // Row 3 (0-based) is within the scroll region
        assert_eq!(buffer.clamp_row_to_scroll_region(row(3)), row(3));
    }

    #[test]
    fn test_clamp_row_to_scroll_region_above_top() {
        let mut buffer = create_test_buffer();

        // Set scroll region from row 2 to row 4 (1-based: 3 to 5)
        buffer.ansi_parser_support.scroll_region_top = Some(term_row(3));
        buffer.ansi_parser_support.scroll_region_bottom = Some(term_row(5));

        // Row 0 is above scroll region - should be clamped to top (row 2)
        assert_eq!(buffer.clamp_row_to_scroll_region(row(0)), row(2));
    }

    #[test]
    fn test_clamp_row_to_scroll_region_below_bottom() {
        let mut buffer = create_test_buffer();

        // Set scroll region from row 2 to row 4 (1-based: 3 to 5)
        buffer.ansi_parser_support.scroll_region_top = Some(term_row(3));
        buffer.ansi_parser_support.scroll_region_bottom = Some(term_row(5));

        // Row 5 is below scroll region - should be clamped to bottom (row 4)
        assert_eq!(buffer.clamp_row_to_scroll_region(row(5)), row(4));
    }
}
