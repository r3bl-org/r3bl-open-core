// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use std::ops::Deref;

use super::PixelChar;
use crate::{List, Pos};

/// This is a wrapper type so the [`std::fmt::Debug`] can be implemented for it, that
/// won't conflict with [List]'s implementation of the trait.
#[derive(Clone, Default, PartialEq)]
pub struct PixelCharDiffChunks {
    pub inner: List<DiffChunk>,
}

pub type DiffChunk = (Pos, PixelChar);

impl Deref for PixelCharDiffChunks {
    type Target = List<DiffChunk>;

    fn deref(&self) -> &Self::Target { &self.inner }
}

impl From<List<DiffChunk>> for PixelCharDiffChunks {
    fn from(list: List<DiffChunk>) -> Self { Self { inner: list } }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{TuiStyle, col, row};

    fn create_test_pixel_char() -> PixelChar {
        PixelChar::PlainText {
            display_char: 'A',
            style: TuiStyle::default(),
        }
    }

    #[test]
    fn test_pixel_char_diff_chunks_creation() {
        let chunks = PixelCharDiffChunks::default();
        assert!(chunks.inner.is_empty());
    }

    #[test]
    fn test_pixel_char_diff_chunks_from_list() {
        let mut list = List::new();
        let pos = row(0) + col(0);
        let pixel_char = create_test_pixel_char();
        list.push((pos, pixel_char));

        let chunks = PixelCharDiffChunks::from(list.clone());
        assert_eq!(chunks.inner.len(), 1);
        assert_eq!(chunks.inner[0].0, pos);
        assert_eq!(chunks.inner[0].1, pixel_char);
    }

    #[test]
    fn test_pixel_char_diff_chunks_deref() {
        let mut list = List::new();
        let pos1 = row(0) + col(0);
        let pos2 = row(1) + col(1);
        let pixel_char = create_test_pixel_char();

        list.push((pos1, pixel_char));
        list.push((pos2, pixel_char));

        let chunks = PixelCharDiffChunks::from(list);

        // Test deref functionality.
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].0, pos1);
        assert_eq!(chunks[1].0, pos2);
    }

    #[test]
    fn test_pixel_char_diff_chunks_equality() {
        let mut list1 = List::new();
        let mut list2 = List::new();
        let pos = row(0) + col(0);
        let pixel_char = create_test_pixel_char();

        list1.push((pos, pixel_char));
        list2.push((pos, pixel_char));

        let chunks1 = PixelCharDiffChunks::from(list1);
        let chunks2 = PixelCharDiffChunks::from(list2);

        assert_eq!(chunks1, chunks2);
    }

    #[test]
    fn test_pixel_char_diff_chunks_clone() {
        let mut list = List::new();
        let pos = row(2) + col(3);
        let pixel_char = create_test_pixel_char();
        list.push((pos, pixel_char));

        let chunks = PixelCharDiffChunks::from(list);
        let cloned = chunks.clone();

        assert_eq!(chunks, cloned);
        assert_eq!(cloned.len(), 1);
        assert_eq!(cloned[0].0, pos);
        assert_eq!(cloned[0].1, pixel_char);
    }

    #[test]
    fn test_pixel_char_diff_chunks_empty_operations() {
        let chunks = PixelCharDiffChunks::default();

        // Test operations on empty chunks.
        assert!(chunks.is_empty());
        assert_eq!(chunks.len(), 0);

        // Clone empty chunks.
        let cloned = chunks.clone();
        assert_eq!(chunks, cloned);
    }
}
