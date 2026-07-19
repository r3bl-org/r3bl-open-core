// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`ANSI`] terminal scroll helper operations for [`OfsBufVT100`].
//!
//! This module provides helper methods for [`ANSI`] escape sequence scrolling operations,
//! including scroll region boundary detection and row clamping within defined scroll
//! areas.
//!
//! [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code

use crate::{LengthOps, OfsBufVT100, VPRow, vp_row};
use std::ops::RangeInclusive;

impl OfsBufVT100 {
    /// Gets the scroll region as an inclusive range.
    ///
    /// Returns [`RangeInclusive<VPRow>`] representing the [`VT-100`] scroll
    /// region boundaries where line operations are confined. The range includes both
    /// the top and bottom boundaries (inclusive on both ends).
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
    /// range.contains(&vp_row(4))  // true (within region)
    /// ```
    ///
    /// [`RangeInclusive<VPRow>`]: std::ops::RangeInclusive
    /// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
    #[must_use]
    pub fn get_scroll_range_inclusive(&self) -> RangeInclusive<VPRow> {
        let scroll_top: VPRow = match self.get_parser_global_state().scroll_region_top {
            Some(term_row) => term_row.into(),
            None => vp_row(0),
        };

        let scroll_bottom: VPRow =
            match self.get_parser_global_state().scroll_region_bottom {
                Some(term_row) => term_row.into(),
                None => self
                    .get_active_screen_buffer()
                    .get_viewport()
                    .get_height()
                    .convert_to_index(),
            };

        scroll_top..=scroll_bottom
    }
}

#[cfg(test)]
mod tests_bounds_check_ops {
    use super::*;
    use crate::{OfsBufVT100, term_row, vp_height, vp_width,
                vt_100_pty_output_conformance_tests::nz};

    fn create_test_buffer() -> OfsBufVT100 {
        let size = vp_width(10) + vp_height(6);
        OfsBufVT100::new_empty(size)
    }

    #[test]
    fn test_get_scroll_range_inclusive_no_region() {
        let buffer = create_test_buffer();

        // No scroll region set - should return full buffer range [0, 5]
        let range = buffer.get_scroll_range_inclusive();
        assert_eq!(*range.start(), vp_row(0));
        assert_eq!(*range.end(), vp_row(5));
    }

    #[test]
    fn test_get_scroll_range_inclusive_with_top_only() {
        let mut buffer = create_test_buffer();

        // Set scroll region top to row 3 (1-based) = row 2 (0-based)
        buffer.get_parser_global_state_mut().scroll_region_top = Some(term_row(nz(3)));

        // Should return [2, 5] (top boundary to end of buffer)
        let range = buffer.get_scroll_range_inclusive();
        assert_eq!(*range.start(), vp_row(2));
        assert_eq!(*range.end(), vp_row(5));
    }

    #[test]
    fn test_get_scroll_range_inclusive_with_bottom_only() {
        let mut buffer = create_test_buffer();

        // Set scroll region bottom to row 4 (1-based) = row 3 (0-based)
        buffer.get_parser_global_state_mut().scroll_region_bottom = Some(term_row(nz(4)));

        // Should return [0, 3] (start of buffer to bottom boundary)
        let range = buffer.get_scroll_range_inclusive();
        assert_eq!(*range.start(), vp_row(0));
        assert_eq!(*range.end(), vp_row(3));
    }

    #[test]
    fn test_get_scroll_range_inclusive_with_both() {
        let mut buffer = create_test_buffer();

        // Set scroll region from row 2 to row 4 (1-based: 3 to 5)
        buffer.get_parser_global_state_mut().scroll_region_top = Some(term_row(nz(3)));
        buffer.get_parser_global_state_mut().scroll_region_bottom = Some(term_row(nz(5)));

        // Should return [2, 4] (0-based)
        let range = buffer.get_scroll_range_inclusive();
        assert_eq!(*range.start(), vp_row(2));
        assert_eq!(*range.end(), vp_row(4));
    }

    #[test]
    fn test_get_scroll_range_inclusive_membership() {
        let mut buffer = create_test_buffer();

        // Set scroll region from row 2 to row 4 (1-based: 3 to 5)
        buffer.get_parser_global_state_mut().scroll_region_top = Some(term_row(nz(3)));
        buffer.get_parser_global_state_mut().scroll_region_bottom = Some(term_row(nz(5)));

        let range = buffer.get_scroll_range_inclusive();

        // Test inclusive range membership
        assert!(!range.contains(&vp_row(1))); // Before range
        assert!(range.contains(&vp_row(2))); // At start
        assert!(range.contains(&vp_row(3))); // Within range
        assert!(range.contains(&vp_row(4))); // At end
        assert!(!range.contains(&vp_row(5))); // After range
    }
}
