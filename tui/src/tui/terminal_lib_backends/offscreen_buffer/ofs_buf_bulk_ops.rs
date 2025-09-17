// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Implementation of bulk operations for `OffscreenBuffer`.
//!
//! This module provides methods for applying multiple changes to the buffer
//! in a single operation, which can be more efficient than individual changes.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::Pos;

impl OffscreenBuffer {
    /// Apply multiple character changes at once.
    /// Returns the number of successful changes applied.
    pub fn apply_changes(&mut self, changes: Vec<(Pos, PixelChar)>) -> usize {
        let mut applied_count = 0;

        for (pos, char) in changes {
            if self.set_char(pos, char) {
                applied_count += 1;
            }
        }

        applied_count
    }
}

#[cfg(test)]
mod tests_bulk_ops {
    use super::*;
    use crate::{TuiStyle, col, height, row, width};

    fn create_test_buffer() -> OffscreenBuffer {
        let size = width(4) + height(4);
        OffscreenBuffer::new_empty(size)
    }

    fn create_test_char(ch: char) -> PixelChar {
        PixelChar::PlainText {
            display_char: ch,
            style: TuiStyle::default(),
        }
    }

    #[test]
    fn test_apply_changes_batch() {
        let mut buffer = create_test_buffer();

        let changes = vec![
            (row(0) + col(0), create_test_char('A')),
            (row(0) + col(1), create_test_char('B')),
            (row(1) + col(0), create_test_char('C')),
            (row(1) + col(1), create_test_char('D')),
        ];

        let applied_count = buffer.apply_changes(changes);
        assert_eq!(applied_count, 4); // All changes should be applied successfully

        // Verify all changes were applied.
        assert_eq!(
            buffer.get_char(row(0) + col(0)).unwrap(),
            create_test_char('A')
        );
        assert_eq!(
            buffer.get_char(row(0) + col(1)).unwrap(),
            create_test_char('B')
        );
        assert_eq!(
            buffer.get_char(row(1) + col(0)).unwrap(),
            create_test_char('C')
        );
        assert_eq!(
            buffer.get_char(row(1) + col(1)).unwrap(),
            create_test_char('D')
        );
    }

    #[test]
    fn test_apply_changes_with_invalid_positions() {
        let mut buffer = create_test_buffer();

        let changes = vec![
            (row(0) + col(0), create_test_char('V')),  // Valid
            (row(10) + col(0), create_test_char('I')), // Invalid row
            (row(0) + col(10), create_test_char('I')), // Invalid column
            (row(2) + col(2), create_test_char('V')),  // Valid
        ];

        let applied_count = buffer.apply_changes(changes);
        assert_eq!(applied_count, 2); // Only 2 valid changes should be applied

        // Verify valid changes were applied.
        assert_eq!(
            buffer.get_char(row(0) + col(0)).unwrap(),
            create_test_char('V')
        );
        assert_eq!(
            buffer.get_char(row(2) + col(2)).unwrap(),
            create_test_char('V')
        );
    }

    #[test]
    fn test_apply_changes_empty_batch() {
        let mut buffer = create_test_buffer();

        let changes = vec![];
        let applied_count = buffer.apply_changes(changes);
        assert_eq!(applied_count, 0);
    }

    #[test]
    fn test_apply_changes_large_batch() {
        let mut buffer = create_test_buffer();

        // Create a large batch of changes.
        let mut changes = vec![];
        for r in 0..4 {
            for c in 0..4 {
                changes.push((row(r) + col(c), create_test_char('*')));
            }
        }

        let applied_count = buffer.apply_changes(changes);
        assert_eq!(applied_count, 16); // All 16 positions in 4x4 buffer

        // Verify all positions were changed.
        for r in 0..4 {
            for c in 0..4 {
                assert_eq!(
                    buffer.get_char(row(r) + col(c)).unwrap(),
                    create_test_char('*')
                );
            }
        }
    }

    #[test]
    fn test_apply_changes_overlapping() {
        let mut buffer = create_test_buffer();

        // Apply changes to same position multiple times.
        let changes = vec![
            (row(1) + col(1), create_test_char('1')),
            (row(1) + col(1), create_test_char('2')),
            (row(1) + col(1), create_test_char('3')),
        ];

        let applied_count = buffer.apply_changes(changes);
        assert_eq!(applied_count, 3); // All changes should be applied

        // The last change should win.
        assert_eq!(
            buffer.get_char(row(1) + col(1)).unwrap(),
            create_test_char('3')
        );
    }
}
