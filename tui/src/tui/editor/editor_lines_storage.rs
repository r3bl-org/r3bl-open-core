/*
 *   Copyright (c) 2025 R3BL LLC
 *   All rights reserved.
 *
 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
 */

//! `EditorLinesStorage` trait provides an abstraction layer for editor line storage.
//!
//! This trait serves as a bridge between the legacy `VecEditorContentLines`
//! (`SmallVec`<GCString>) and the new `ZeroCopyGapBuffer` implementation. It enables gradual
//! migration while maintaining compatibility with existing editor code.
//!
//! # Design Goals
//!
//! - **Zero-copy operations**: Enable efficient text parsing without copying data
//! - **Type safety**: Use specific index types (`RowIndex`, `ColIndex`, etc.) instead of
//!   usize
//! - **Performance**: Support optimized operations like batch insertions
//! - **Compatibility**: Allow both legacy and new storage engines to coexist
//!
//! # Migration Strategy
//!
//! 1. Define this trait with all required operations
//! 2. Implement for `ZeroCopyGapBuffer` (native implementation)
//! 3. Create adapter for `VecEditorContentLines` (legacy support)
//! 4. Update editor code to use trait instead of concrete types
//! 5. Gradually switch from legacy to new storage engine
//! 6. Eventually remove legacy implementation

use crate::{ByteIndex, ColIndex, ColWidth, GCString, Length, RowIndex, SegIndex};

/// Trait for abstracting editor line storage operations.
///
/// This trait provides a unified interface for different storage backends,
/// enabling the editor to work with both the legacy `VecEditorContentLines`
/// and the new `ZeroCopyGapBuffer` implementation.
///
/// # Trait Bounds
///
/// ## `Clone`
/// Required for the undo/redo history system. When editor operations are performed,
/// the entire `EditorContent` state (which contains the storage) must be cloned
/// and stored in the history buffer for potential undo operations. This enables
/// rich text editing functionality where users can undo/redo their changes.
///
/// ## `Default`
/// Required for easy initialization and reset functionality. This allows:
/// - Creating empty storage instances without complex setup
/// - Resetting storage to initial state when needed
/// - Simplifying test fixtures and factory patterns
/// - Enabling creation through trait objects
///
/// While these bounds introduce some overhead (cloning isn't zero-cost), they enable
/// essential editor functionality and maintain compatibility during the migration
/// from legacy to zero-copy storage implementations.
pub trait EditorLinesStorage: Clone + Default {
    // Line access methods

    /// Get the content of a line as a string slice.
    /// Returns None if the line index is out of bounds.
    fn get_line_content(&self, row_index: RowIndex) -> Option<&str>;

    /// Get the number of lines in the storage.
    fn line_count(&self) -> Length;

    /// Check if the storage is empty (has no lines).
    fn is_empty(&self) -> bool { self.line_count().as_usize() == 0 }

    // Line metadata access

    /// Get the display width of a line (sum of grapheme widths).
    fn get_line_display_width(&self, row_index: RowIndex) -> Option<ColWidth>;

    /// Get the number of grapheme clusters in a line.
    fn get_line_grapheme_count(&self, row_index: RowIndex) -> Option<Length>;

    /// Get the byte length of a line's content.
    fn get_line_byte_len(&self, row_index: RowIndex) -> Option<Length>;

    // Line modification methods

    /// Insert a new empty line at the specified position.
    /// Lines at and after this position are shifted down.
    fn insert_line(&mut self, row_index: RowIndex) -> bool;

    /// Remove a line at the specified position.
    /// Lines after this position are shifted up.
    fn remove_line(&mut self, row_index: RowIndex) -> bool;

    /// Clear all lines from the storage.
    fn clear(&mut self);

    /// Set the entire content of a line, replacing any existing content.
    fn set_line(&mut self, row_index: RowIndex, content: &str) -> bool;

    /// Push a new line to the end of the storage.
    fn push_line(&mut self, content: &str);

    // Grapheme-based operations

    /// Insert text at a specific grapheme position within a line.
    fn insert_at_grapheme(
        &mut self,
        row_index: RowIndex,
        seg_index: SegIndex,
        text: &str,
    ) -> bool;

    /// Delete graphemes at a specific position within a line.
    fn delete_at_grapheme(
        &mut self,
        row_index: RowIndex,
        seg_index: SegIndex,
        count: Length,
    ) -> bool;

    // Column-based operations (for cursor movement)

    /// Insert text at a specific column position within a line.
    /// Returns the display width of the inserted text if successful.
    fn insert_at_col(
        &mut self,
        row_index: RowIndex,
        col_index: ColIndex,
        text: &str,
    ) -> Option<ColWidth>;

    /// Delete graphemes at a specific column position within a line.
    fn delete_at_col(
        &mut self,
        row_index: RowIndex,
        col_index: ColIndex,
        count: Length,
    ) -> bool;

    // Utility methods

    /// Split a line at the specified column position.
    /// Returns the content after the split point as a new string.
    /// The original line is truncated at the split point.
    fn split_line_at_col(
        &mut self,
        row_index: RowIndex,
        col_index: ColIndex,
    ) -> Option<String>;

    /// Join two consecutive lines.
    /// The content of the second line is appended to the first line.
    fn join_lines(&mut self, first_row_index: RowIndex) -> bool;

    // Byte position conversions (for parser integration)

    /// Get the byte offset where a specific row starts in the overall buffer.
    /// This is useful for parser integration.
    fn get_byte_offset_for_row(&self, row_index: RowIndex) -> Option<ByteIndex>;

    /// Find the row that contains the given byte position.
    fn find_row_containing_byte(&self, byte_index: ByteIndex) -> Option<RowIndex>;

    // Iterator support

    /// Get an iterator over all lines as string slices.
    fn iter_lines(&self) -> Box<dyn Iterator<Item = &str> + '_>;

    // Total size information

    /// Get the total number of bytes across all lines.
    fn total_bytes(&self) -> ByteIndex;

    /// Get the maximum valid row index (`line_count` - 1, or 0 if empty).
    fn max_row_index(&self) -> Option<RowIndex> {
        let count = self.line_count().as_usize();
        if count > 0 {
            Some(RowIndex::from(count - 1))
        } else {
            None
        }
    }

    // Conversion methods (for compatibility during migration)

    /// Convert the entire storage to a vector of `GCString`s.
    /// This is primarily for compatibility with legacy code.
    fn to_gc_string_vec(&self) -> Vec<GCString>;

    /// Create a new storage from a vector of `GCString`s.
    /// This is primarily for compatibility with legacy code.
    fn from_gc_string_vec(lines: Vec<GCString>) -> Self;
}
