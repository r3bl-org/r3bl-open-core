// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Integration tests for the bounds checking system.
//!
//! These tests verify that multiple traits work correctly together, testing
//! cross-cutting concerns that span the various bounds checking implementations.
//! Each submodule ([`array_bounds`], [`cursor_bounds`], [`viewport_bounds`], etc.) has
//! its own unit tests, but these integration tests verify the interactions between them.
//!
//! [`array_bounds`]: super::array_bounds_check
//! [`cursor_bounds`]: super::cursor_bounds_check
//! [`viewport_bounds`]: super::viewport_bounds_check

use super::*;
use crate::{RowIndex, height, idx, len, row};

#[test]
fn test_array_cursor_bounds_semantic_difference() {
    // Critical test: Array bounds and cursor bounds have different semantics
    // Array: index must be < length (for element access)
    // Cursor: position can be <= length (cursor after last char)
    let content_len = len(10);

    // Test the boundary where semantics differ
    let idx_9 = idx(9);
    let idx_10 = idx(10);

    // Index 9: valid for both array access and cursor position
    assert_eq!(idx_9.overflows(content_len), ArrayOverflowResult::Within);
    assert!(content_len.is_valid_cursor_position(idx_9));
    assert_eq!(
        content_len.check_cursor_position_bounds(idx_9),
        CursorPositionBoundsStatus::Within
    );

    // Index 10: INVALID for array access but VALID for cursor (EOL position)
    // This semantic difference is critical for text editors
    assert_eq!(
        idx_10.overflows(content_len),
        ArrayOverflowResult::Overflowed
    );
    assert!(content_len.is_valid_cursor_position(idx_10));
    assert_eq!(
        idx_10.overflows(content_len),
        ArrayOverflowResult::Overflowed
    );
    assert_eq!(
        content_len.check_cursor_position_bounds(idx_10),
        CursorPositionBoundsStatus::AtEnd
    );
}

#[test]
fn test_zero_length_consistency_across_traits() {
    // Cross-cutting concern: All traits must handle empty content consistently
    // This tests the edge case that often causes bugs
    let zero_len = len(0);
    let any_idx = idx(0);

    // Array bounds: should reject ALL indices (no elements to access)
    assert_eq!(any_idx.overflows(zero_len), ArrayOverflowResult::Overflowed);
    assert_eq!(any_idx.overflows(zero_len), ArrayOverflowResult::Overflowed);

    // Cursor bounds: position 0 is valid (cursor at start of empty content)
    assert!(zero_len.is_valid_cursor_position(any_idx));
    assert_eq!(
        zero_len.check_cursor_position_bounds(any_idx),
        CursorPositionBoundsStatus::AtStart
    );
    assert_eq!(zero_len.eol_cursor_position(), any_idx);

    // Viewport: zero-size viewport contains nothing
    let zero_viewport_size = len(0);
    assert!(
        any_idx.check_viewport_bounds(idx(0), zero_viewport_size)
            != RangeBoundsResult::Within
    );

    // This consistency is crucial for avoiding special-case code throughout the
    // system
}

#[test]
fn test_vt100_scroll_region_conversion_in_context() {
    use std::ops::RangeInclusive;

    // Real-world scenario: VT-100 terminals use inclusive ranges for scroll regions
    // but Rust iteration needs exclusive ranges
    let scroll_region: RangeInclusive<RowIndex> = row(2)..=row(10);

    // Convert to exclusive for Rust iteration
    let iter_range = scroll_region.to_exclusive();

    // Verify conversion: inclusive 2..=10 becomes exclusive 2..11
    assert_eq!(iter_range.start, row(2));
    assert_eq!(iter_range.end, row(11));

    // Practical application: Check visibility in viewport
    let viewport_start = row(0);
    let viewport_height = height(15);

    // Verify all rows in scroll region are visible (testing the conversion works
    // correctly)
    for i in 2..11 {
        let row_idx = row(i);
        assert!(
            row_idx.check_viewport_bounds(viewport_start, viewport_height)
                == RangeBoundsResult::Within,
            "Row {i} should be visible in viewport"
        );
    }

    // Edge case: single-row scroll region
    let single_row: RangeInclusive<RowIndex> = row(5)..=row(5);
    let single_exclusive = single_row.to_exclusive();
    assert_eq!(single_exclusive.start, row(5));
    assert_eq!(single_exclusive.end, row(6)); // 5..6 includes only row 5
}

#[test]
fn test_real_world_viewport_scrolling() {
    // Simulate actual text editor viewport management with cursor tracking
    let buffer_height = height(100);
    let viewport_height = height(25);
    let mut viewport_start = row(0);
    let cursor_row = row(30);

    // Step 1: Check if cursor is visible
    if cursor_row.check_viewport_bounds(viewport_start, viewport_height)
        != RangeBoundsResult::Within
    {
        // Step 2: Calculate new viewport position to center cursor
        if cursor_row
            .overflows(height(viewport_start.as_u16() + viewport_height.as_u16()))
            == ArrayOverflowResult::Overflowed
        {
            // Cursor is below viewport - scroll down
            viewport_start = row(cursor_row
                .as_u16()
                .saturating_sub(viewport_height.as_u16() / 2));
        }
    }

    // Step 3: Ensure viewport doesn't exceed buffer bounds
    let max_viewport_start = row(buffer_height
        .as_u16()
        .saturating_sub(viewport_height.as_u16()));
    if viewport_start.overflows(height(max_viewport_start.as_u16() + 1))
        == ArrayOverflowResult::Overflowed
    {
        viewport_start = max_viewport_start;
    }

    // Verify cursor is now visible after scrolling
    assert!(
        cursor_row.check_viewport_bounds(viewport_start, viewport_height)
            == RangeBoundsResult::Within
    );

    // Additional verification: viewport should be within buffer
    assert_eq!(
        viewport_start.overflows(buffer_height),
        ArrayOverflowResult::Within
    );

    // Test edge case: cursor near bottom of buffer
    let bottom_cursor = row(95);
    let mut test_viewport = row(70);

    // Scroll to show bottom cursor
    if bottom_cursor.check_viewport_bounds(test_viewport, viewport_height)
        != RangeBoundsResult::Within
    {
        test_viewport = row(buffer_height
            .as_u16()
            .saturating_sub(viewport_height.as_u16()));
    }

    assert!(
        bottom_cursor.check_viewport_bounds(test_viewport, viewport_height)
            == RangeBoundsResult::Within
    );
    assert_eq!(test_viewport, row(75)); // 100 - 25 = 75
}
