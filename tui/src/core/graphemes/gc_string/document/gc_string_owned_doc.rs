// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Simple multi-line document implementation using `Vec<GCStringOwned>`.

use std::borrow::Cow;

use miette::miette;

use crate::{ColIndex, GCStringOwned, GraphemeDoc, GraphemeDocMut, GraphemeString,
            GraphemeStringMut, Length, RowIndex, SegIndex, col};

/// The equivalent of a document in the editor, containing multiple lines of
/// [`GCStringOwned`]. This is a very simplistic version of [`crate::ZeroCopyGapBuffer`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GCStringOwnedDoc {
    pub lines: Vec<GCStringOwned>,
}

impl GCStringOwnedDoc {
    /// Create a new empty document.
    #[must_use]
    pub fn new() -> Self {
        Self {
            lines: vec![GCStringOwned::new("")],
        }
    }

    /// Create a document from a string, splitting on newlines.
    pub fn from_string(s: &str) -> Self {
        let lines: Vec<GCStringOwned> = s.lines().map(GCStringOwned::new).collect();

        Self {
            lines: if lines.is_empty() {
                vec![GCStringOwned::new("")]
            } else {
                lines
            },
        }
    }
}

impl Default for GCStringOwnedDoc {
    fn default() -> Self { Self::new() }
}

/// Wrapper type to make &`GCStringOwned` implement `GraphemeString`.
///
/// This wrapper allows us to return references to `GCStringOwned` from
/// `GraphemeDoc` trait methods while still implementing `GraphemeString`.
/// The lifetime parameter `'a` represents the lifetime of the borrowed
/// `GCStringOwned`.
#[derive(Debug)]
pub struct GCStringOwnedRef<'a>(&'a GCStringOwned);

impl std::ops::Deref for GCStringOwnedRef<'_> {
    type Target = GCStringOwned;

    fn deref(&self) -> &Self::Target { self.0 }
}

impl GraphemeString for GCStringOwnedRef<'_> {
    /// Iterator over segments with lifetime `'b`.
    ///
    /// The lifetime `'b` is constrained by `Self: 'b`, which means the iterator
    /// cannot outlive the `GCStringOwnedRef` it borrows from.
    type SegmentIterator<'b>
        = <GCStringOwned as GraphemeString>::SegmentIterator<'b>
    where
        Self: 'b;

    /// String slice with lifetime `'b`.
    ///
    /// The lifetime `'b` is constrained by `Self: 'b`, which means the string slice
    /// cannot outlive the `GCStringOwnedRef` it borrows from.
    type StringSlice<'b>
        = crate::CowInlineString<'b>
    where
        Self: 'b;

    fn as_str(&self) -> &str { self.0.as_str() }
    fn segments(&self) -> &[crate::Seg] { self.0.segments() }
    fn display_width(&self) -> crate::ColWidth { self.0.display_width() }
    fn segment_count(&self) -> crate::SegWidth { self.0.segment_count() }
    fn byte_size(&self) -> crate::ChUnit { self.0.byte_size() }
    fn get_seg(&self, index: crate::SegIndex) -> Option<crate::Seg> {
        self.0.get_seg(index)
    }
    fn check_is_in_middle_of_grapheme(&self, col: ColIndex) -> Option<crate::Seg> {
        self.0.check_is_in_middle_of_grapheme(col)
    }
    fn get_seg_at(&self, col: ColIndex) -> Option<crate::SegContent<'_>> {
        self.0.get_seg_at(col)
    }
    fn get_seg_right_of(&self, col: ColIndex) -> Option<crate::SegContent<'_>> {
        self.0.get_seg_right_of(col)
    }
    fn get_seg_left_of(&self, col: ColIndex) -> Option<crate::SegContent<'_>> {
        self.0.get_seg_left_of(col)
    }
    fn get_seg_at_end(&self) -> Option<crate::SegContent<'_>> { self.0.get_seg_at_end() }
    fn clip(&self, start_col: ColIndex, width: crate::ColWidth) -> Self::StringSlice<'_> {
        // Delegate to the GraphemeString implementation which returns CowInlineString.
        <GCStringOwned as GraphemeString>::clip(self.0, start_col, width)
    }
    fn trunc_end_to_fit(&self, width: crate::ColWidth) -> Self::StringSlice<'_> {
        // Delegate to the GraphemeString implementation which returns CowInlineString.
        <GCStringOwned as GraphemeString>::trunc_end_to_fit(self.0, width)
    }
    fn trunc_end_by(&self, width: crate::ColWidth) -> Self::StringSlice<'_> {
        // Delegate to the GraphemeString implementation which returns CowInlineString.
        <GCStringOwned as GraphemeString>::trunc_end_by(self.0, width)
    }
    fn trunc_start_by(&self, width: crate::ColWidth) -> Self::StringSlice<'_> {
        // Delegate to the GraphemeString implementation which returns CowInlineString.
        <GCStringOwned as GraphemeString>::trunc_start_by(self.0, width)
    }
    fn segments_iter(&self) -> Self::SegmentIterator<'_> { self.0.segments_iter() }
    fn is_empty(&self) -> bool { self.0.is_empty() }
    fn last(&self) -> Option<crate::Seg> { self.0.last() }
    fn contains_wide_segments(&self) -> crate::ContainsWideSegments {
        self.0.contains_wide_segments()
    }
}

/// Iterator for `GCStringOwnedDoc` lines.
///
/// The lifetime parameter `'a` represents the lifetime of the document
/// being iterated over. This ensures the iterator cannot outlive the
/// document it references.
#[derive(Debug)]
pub struct GCStringOwnedDocIterator<'a> {
    iter: std::slice::Iter<'a, GCStringOwned>,
}

impl<'a> Iterator for GCStringOwnedDocIterator<'a> {
    type Item = GCStringOwnedRef<'a>;

    fn next(&mut self) -> Option<Self::Item> { self.iter.next().map(GCStringOwnedRef) }
}

impl GraphemeDoc for GCStringOwnedDoc {
    type Line<'a> = GCStringOwnedRef<'a>;
    type LineIterator<'a> = GCStringOwnedDocIterator<'a>;

    fn line_count(&self) -> Length { Length::from(self.lines.len()) }

    fn get_line(&self, row: RowIndex) -> Option<Self::Line<'_>> {
        self.lines.get(row.as_usize()).map(GCStringOwnedRef)
    }

    fn total_bytes(&self) -> usize {
        self.lines
            .iter()
            .map(|line| line.byte_size().as_usize())
            .sum()
    }

    fn iter_lines(&self) -> Self::LineIterator<'_> {
        GCStringOwnedDocIterator {
            iter: self.lines.iter(),
        }
    }

    fn as_str(&self) -> Cow<'_, str> {
        Cow::Owned(
            self.lines
                .iter()
                .map(super::super::owned::gc_string_owned::GCStringOwned::as_str)
                .collect::<Vec<_>>()
                .join("\n"),
        )
    }

    fn as_bytes(&self) -> Cow<'_, [u8]> {
        Cow::Owned(self.as_str().into_owned().into_bytes())
    }
}

impl GraphemeDocMut for GCStringOwnedDoc {
    type DocMutResult = ();

    fn add_line(&mut self) -> usize {
        let index = self.lines.len();
        self.lines.push(GCStringOwned::new(""));
        index
    }

    fn remove_line(&mut self, row: RowIndex) -> bool {
        if row.as_usize() < self.lines.len() {
            self.lines.remove(row.as_usize());
            // Ensure we always have at least one line.
            if self.lines.is_empty() {
                self.lines.push(GCStringOwned::new(""));
            }
            true
        } else {
            false
        }
    }

    fn insert_line_with_buffer_shift(&mut self, line_idx: usize) {
        if line_idx <= self.lines.len() {
            self.lines.insert(line_idx, GCStringOwned::new(""));
        }
    }

    fn clear(&mut self) {
        self.lines.clear();
        self.lines.push(GCStringOwned::new(""));
    }

    fn insert_text_at_grapheme(
        &mut self,
        row: RowIndex,
        seg_index: SegIndex,
        text: &str,
    ) -> miette::Result<Self::DocMutResult> {
        let line = self
            .lines
            .get_mut(row.as_usize())
            .ok_or_else(|| miette!("Invalid row index: {:?}", row))?;

        // Convert segment index to column index.
        let col_index = if seg_index.as_usize() == 0 {
            col(0)
        } else if let Some(seg) = line.get_seg(seg_index) {
            seg.start_display_col_index
        } else {
            // Insert at end
            col(line.display_width().as_usize())
        };

        if let Some(new_line) = line.insert_text(col_index, text) {
            *line = new_line;
            Ok(())
        } else {
            Err(miette!("Invalid column index: {:?}", col_index))
        }
    }

    fn delete_range_at_grapheme(
        &mut self,
        row: RowIndex,
        start_seg: SegIndex,
        end_seg: SegIndex,
    ) -> miette::Result<Self::DocMutResult> {
        let line = self
            .lines
            .get_mut(row.as_usize())
            .ok_or_else(|| miette!("Invalid row index: {:?}", row))?;

        // Convert segment indices to column indices.
        let start_col = if let Some(seg) = line.get_seg(start_seg) {
            seg.start_display_col_index
        } else {
            return Err(miette!("Index out of bounds"));
        };

        let end_col = if let Some(seg) = line.get_seg(end_seg) {
            seg.start_display_col_index
        } else {
            col(line.display_width().as_usize())
        };

        if let Some(new_line) = line.delete_range(start_col, end_col) {
            *line = new_line;
            Ok(())
        } else {
            Err(miette!("Invalid column index: {:?}", start_col))
        }
    }

    fn insert_empty_line(&mut self, row: RowIndex) -> miette::Result<Self::DocMutResult> {
        if row.as_usize() <= self.lines.len() {
            self.lines.insert(row.as_usize(), GCStringOwned::new(""));
            Ok(())
        } else {
            Err(miette!("Invalid row index: {:?}", row))
        }
    }

    fn merge_lines(&mut self, row: RowIndex) -> miette::Result<Self::DocMutResult> {
        if row.as_usize() + 1 < self.lines.len() {
            let next_line = self.lines.remove(row.as_usize() + 1);
            let current_line = &mut self.lines[row.as_usize()];

            // Append next line to current line.
            let end_col = col(current_line.display_width().as_usize());
            if let Some(merged) = current_line.insert_text(end_col, next_line.as_str()) {
                *current_line = merged;
            }
            Ok(())
        } else {
            Err(miette!("Invalid row index: {:?}", row))
        }
    }

    fn split_line(
        &mut self,
        row: RowIndex,
        col: ColIndex,
    ) -> miette::Result<Self::DocMutResult> {
        let line = self
            .lines
            .get_mut(row.as_usize())
            .ok_or_else(|| miette!("Invalid row index: {:?}", row))?;

        // Use truncate to get the left part.
        if let Some(left_part) = line.truncate(col) {
            // Get the content after the split point for the new line.
            let original_content = line.as_str().to_string();
            let right_content = if col.as_usize() < original_content.len() {
                // Find the byte position for the column.
                let mut byte_pos = 0;
                let mut col_count = 0;
                for seg in line.segments_iter() {
                    if col_count >= col.as_usize() {
                        break;
                    }
                    byte_pos += seg.bytes_size.as_usize();
                    col_count += seg.display_width.as_usize();
                }
                &original_content[byte_pos..]
            } else {
                ""
            };

            // Update current line to be the left part.
            *line = left_part;

            // Insert the right part as a new line.
            self.lines
                .insert(row.as_usize() + 1, GCStringOwned::new(right_content));
            Ok(())
        } else {
            // If truncate fails, just insert an empty line after.
            self.lines
                .insert(row.as_usize() + 1, GCStringOwned::new(""));
            Ok(())
        }
    }
}
