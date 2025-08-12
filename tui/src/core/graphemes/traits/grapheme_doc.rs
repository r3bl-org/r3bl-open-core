// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Multi-line document traits for grapheme-aware text operations.

use std::borrow::Cow;

use crate::{ColIndex, GraphemeString, Length, RowIndex, SegIndex};

/// Multi-line document trait for read-only operations.
///
/// This trait abstracts over different multi-line document implementations,
/// such as `GCStringOwnedDoc` (owned lines) and `ZeroCopyGapBuffer` (zero-copy lines).
pub trait GraphemeDoc {
    /// The type of line returned by this document.
    ///
    /// Each implementation returns an appropriate line type:
    /// - `GCStringOwnedDoc` returns `GCStringOwnedRef<'a>` (a reference wrapper)
    /// - `ZeroCopyGapBuffer` returns `GapBufferLine<'a>` (zero-copy view)
    ///
    /// The lifetime parameter `'a` represents the lifetime of the line reference.
    /// The constraint `Self: 'a` ensures that lines cannot outlive the document
    /// they are borrowed from.
    type Line<'a>: GraphemeString
    where
        Self: 'a;

    /// Iterator over lines in the document.
    ///
    /// The lifetime parameter `'a` represents the lifetime of the iterator and
    /// the lines it yields. The constraint `Self: 'a` ensures that the iterator
    /// cannot outlive the document it iterates over.
    type LineIterator<'a>: Iterator<Item = Self::Line<'a>> + 'a
    where
        Self: 'a;

    /// Get the number of lines in the document.
    /// Uses Length for array length semantics.
    fn line_count(&self) -> Length;

    /// Get a specific line by row index.
    fn get_line(&self, row: RowIndex) -> Option<Self::Line<'_>>;

    /// Check if the document is empty.
    fn is_empty(&self) -> bool { self.line_count() == Length::from(0) }

    /// Get total byte size of the document.
    fn total_bytes(&self) -> usize;

    /// Iterate over all lines in the document.
    fn iter_lines(&self) -> Self::LineIterator<'_>;

    /// Get the entire document as a string.
    /// Returns `Cow<str>` for flexibility - borrowed for `ZeroCopyGapBuffer`,
    /// owned for `GCStringOwnedDoc`.
    fn as_str(&self) -> Cow<'_, str>;

    /// Get the entire document as bytes.
    fn as_bytes(&self) -> Cow<'_, [u8]>;
}

/// Mutation operations for multi-line documents.
pub trait GraphemeDocMut: GraphemeDoc {
    /// Associated type for document mutation results.
    /// This handles paradigm differences - () for in-place mutations,
    /// or other types for immutable patterns.
    type DocMutResult;

    /// Add a new empty line at the end of the document.
    /// Returns the index of the new line.
    fn add_line(&mut self) -> usize;

    /// Remove a line at the specified row.
    /// Returns true if the line was removed, false if row was invalid.
    fn remove_line(&mut self, row: RowIndex) -> bool;

    /// Insert a new empty line at the specified index, shifting existing lines down.
    fn insert_line_with_buffer_shift(&mut self, line_idx: usize);

    /// Clear all content from the document.
    fn clear(&mut self);

    /// Insert text at a specific grapheme position.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The row index is out of bounds
    /// - The segment index is invalid
    fn insert_text_at_grapheme(
        &mut self,
        row: RowIndex,
        seg_index: SegIndex,
        text: &str,
    ) -> miette::Result<Self::DocMutResult>;

    /// Delete a range of graphemes from a line.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The row index is out of bounds
    /// - The start or end segment indices are invalid
    /// - The range is invalid (start > end)
    fn delete_range_at_grapheme(
        &mut self,
        row: RowIndex,
        start_seg: SegIndex,
        end_seg: SegIndex,
    ) -> miette::Result<Self::DocMutResult>;

    /// Insert an empty line at the specified row.
    ///
    /// # Errors
    ///
    /// Returns an error if the row index is out of bounds.
    fn insert_empty_line(&mut self, row: RowIndex) -> miette::Result<Self::DocMutResult>;

    /// Merge the line at row with the next line.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The row index is out of bounds
    /// - There is no next line to merge with
    fn merge_lines(&mut self, row: RowIndex) -> miette::Result<Self::DocMutResult>;

    /// Split a line at the specified column position.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The row index is out of bounds
    /// - The column index is invalid
    fn split_line(
        &mut self,
        row: RowIndex,
        col: ColIndex,
    ) -> miette::Result<Self::DocMutResult>;
}
