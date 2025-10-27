// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.
//! ANSI terminal scroll helper operations for `OffscreenBuffer`.
//!
//! This module provides helper methods for ANSI escape sequence scrolling operations,
//! including scroll region boundary detection and row clamping within defined scroll
//! areas.

#[allow(clippy::wildcard_imports)]
use super::super::*;
use crate::{LengthOps, RowIndex, core::coordinates::bounds_check::IndexOps, row};

impl OffscreenBuffer {
    /// Get the scroll region as an inclusive range.
    ///
    /// Returns `RangeInclusive<RowIndex>` representing the VT-100 scroll region
    /// boundaries where line operations are confined. The range includes both
    /// the top and bottom boundaries (inclusive on both ends).
    ///
    /// # Returns
    ///
    /// - If no scroll region is set: `[0, max_row_index]` (entire buffer)
    /// - If scroll region is set: `[scroll_top, scroll_bottom]` (confined region)
    ///
    /// # Examples
    ///
    /// ```text
    /// Terminal Buffer (height=6, max_index=5):
    /// ┌─────────────────┐
    /// │ Line 0 (fixed)  │  ← Outside scroll region
    /// │ Line 1 (fixed)  │  ← Outside scroll region
    /// ├─────────────────┤  ← scroll_top = 2
    /// │ Line 2          │  ← ┐
    /// │ Line 3          │  ← │ Scroll Region
    /// │ Line 4          │  ← │ [2, 5] inclusive
    /// │ Line 5          │  ← ┘
    /// ├─────────────────┤  ← scroll_bottom = 5
    /// │ Line 6 (fixed)  │  ← Outside scroll region
    /// └─────────────────┘
    ///
    /// range = get_scroll_range_inclusive();  // Returns 2..=5
    /// *range.start()  // 2 (scroll_top)
    /// *range.end()    // 5 (scroll_bottom)
    /// range.contains(&row(4))  // true (within region)
    /// ```
    #[must_use]
    pub fn get_scroll_range_inclusive(&self) -> std::ops::RangeInclusive<RowIndex> {
        let scroll_top = self.ansi_parser_support.scroll_region_top.map_or(
            /* None */ row(0),
            /* Some */ |term_row| term_row.to_zero_based(),
        );

        let scroll_bottom = self.ansi_parser_support.scroll_region_bottom.map_or(
            /* None */ self.window_size.row_height.convert_to_index(),
            /* Some */ |term_row| term_row.to_zero_based(),
        );

        scroll_top..=scroll_bottom
    }

    /// Clamp a row to stay within the scroll region boundaries.
    ///
    /// This ensures row positions respect ANSI scroll region settings,
    /// keeping the cursor within the defined scrollable area.
    ///
    /// ```text
    /// Terminal Buffer:
    /// ┌─────────────────┐
    /// │ Line 0 (fixed)  │  ← Outside scroll region
    /// │ Line 1 (fixed)  │  ← Outside scroll region
    /// ├─────────────────┤  ← scroll_top = 2
    /// │ Line 2          │  ← ┐
    /// │ Line 3          │  ← │ Scroll Region
    /// │ Line 4          │  ← │ [2, 5] inclusive
    /// │ Line 5          │  ← ┘
    /// ├─────────────────┤  ← scroll_bottom = 5
    /// │ Line 6 (fixed)  │  ← Outside scroll region
    /// └─────────────────┘
    ///
    /// clamp_row_to_scroll_region() behavior:
    /// - row=1 → clamped to 2 (below scroll_top, clamped up)
    /// - row=2 → returns 2    (at top boundary)
    /// - row=4 → returns 4    (within scroll region)
    /// - row=5 → returns 5    (at bottom boundary)
    /// - row=6 → clamped to 5 (above scroll_bottom, clamped down)
    /// ```
    #[must_use]
    pub fn clamp_row_to_scroll_region(&self, row: RowIndex) -> RowIndex {
        let scroll_region = self.get_scroll_range_inclusive();

        // Use IndexOps's clamp_to_range for semantic clarity with inclusive ranges.
        row.clamp_to_range(scroll_region)
    }
}

#[cfg(test)]
mod tests_bounds_check_ops {
    use super::*;
    use crate::{height, term_row, width,
                core::ansi::parser::vt_100_ansi_conformance_tests::test_fixtures_vt_100_ansi_conformance::nz};

    fn create_test_buffer() -> OffscreenBuffer {
        let size = width(10) + height(6);
        OffscreenBuffer::new_empty(size)
    }

    #[test]
    fn test_get_scroll_range_inclusive_no_region() {
        let buffer = create_test_buffer();

        // No scroll region set - should return full buffer range [0, 5]
        let range = buffer.get_scroll_range_inclusive();
        assert_eq!(*range.start(), row(0));
        assert_eq!(*range.end(), row(5));
    }

    #[test]
    fn test_get_scroll_range_inclusive_with_top_only() {
        let mut buffer = create_test_buffer();

        // Set scroll region top to row 3 (1-based) = row 2 (0-based)
        buffer.ansi_parser_support.scroll_region_top = Some(term_row(nz(3)));

        // Should return [2, 5] (top boundary to end of buffer)
        let range = buffer.get_scroll_range_inclusive();
        assert_eq!(*range.start(), row(2));
        assert_eq!(*range.end(), row(5));
    }

    #[test]
    fn test_get_scroll_range_inclusive_with_bottom_only() {
        let mut buffer = create_test_buffer();

        // Set scroll region bottom to row 4 (1-based) = row 3 (0-based)
        buffer.ansi_parser_support.scroll_region_bottom = Some(term_row(nz(4)));

        // Should return [0, 3] (start of buffer to bottom boundary)
        let range = buffer.get_scroll_range_inclusive();
        assert_eq!(*range.start(), row(0));
        assert_eq!(*range.end(), row(3));
    }

    #[test]
    fn test_get_scroll_range_inclusive_with_both() {
        let mut buffer = create_test_buffer();

        // Set scroll region from row 2 to row 4 (1-based: 3 to 5)
        buffer.ansi_parser_support.scroll_region_top = Some(term_row(nz(3)));
        buffer.ansi_parser_support.scroll_region_bottom = Some(term_row(nz(5)));

        // Should return [2, 4] (0-based)
        let range = buffer.get_scroll_range_inclusive();
        assert_eq!(*range.start(), row(2));
        assert_eq!(*range.end(), row(4));
    }

    #[test]
    fn test_get_scroll_range_inclusive_membership() {
        let mut buffer = create_test_buffer();

        // Set scroll region from row 2 to row 4 (1-based: 3 to 5)
        buffer.ansi_parser_support.scroll_region_top = Some(term_row(nz(3)));
        buffer.ansi_parser_support.scroll_region_bottom = Some(term_row(nz(5)));

        let range = buffer.get_scroll_range_inclusive();

        // Test inclusive range membership
        assert!(!range.contains(&row(1))); // Before range
        assert!(range.contains(&row(2))); // At start
        assert!(range.contains(&row(3))); // Within range
        assert!(range.contains(&row(4))); // At end
        assert!(!range.contains(&row(5))); // After range
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
        buffer.ansi_parser_support.scroll_region_top = Some(term_row(nz(3)));
        buffer.ansi_parser_support.scroll_region_bottom = Some(term_row(nz(5)));

        // Row 3 (0-based) is within the scroll region
        assert_eq!(buffer.clamp_row_to_scroll_region(row(3)), row(3));
    }

    #[test]
    fn test_clamp_row_to_scroll_region_above_top() {
        let mut buffer = create_test_buffer();

        // Set scroll region from row 2 to row 4 (1-based: 3 to 5)
        buffer.ansi_parser_support.scroll_region_top = Some(term_row(nz(3)));
        buffer.ansi_parser_support.scroll_region_bottom = Some(term_row(nz(5)));

        // Row 0 is above scroll region - should be clamped to top (row 2)
        assert_eq!(buffer.clamp_row_to_scroll_region(row(0)), row(2));
    }

    #[test]
    fn test_clamp_row_to_scroll_region_below_bottom() {
        let mut buffer = create_test_buffer();

        // Set scroll region from row 2 to row 4 (1-based: 3 to 5)
        buffer.ansi_parser_support.scroll_region_top = Some(term_row(nz(3)));
        buffer.ansi_parser_support.scroll_region_bottom = Some(term_row(nz(5)));

        // Row 5 is below scroll region - should be clamped to bottom (row 4)
        assert_eq!(buffer.clamp_row_to_scroll_region(row(5)), row(4));
    }
}
