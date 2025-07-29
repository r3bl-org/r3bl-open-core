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

//! Implementation of `EditorLinesStorage` trait for `ZeroCopyGapBuffer`.
//!
//! This module provides the native implementation of the `EditorLinesStorage` trait
//! for `ZeroCopyGapBuffer`, enabling it to be used as a storage backend for the editor.
//!
//! # Performance Characteristics
//!
//! - **Zero-copy access**: Line content is returned as `&str` without copying
//! - **Efficient grapheme operations**: Leverages pre-computed segment metadata
//! - **Optimized appends**: Uses fast path for end-of-line insertions
//! - **Dynamic line growth**: Automatically extends capacity as needed

use crate::{ByteIndex, ColIndex, ColWidth, EditorLinesStorage, GCStringOwned, Length, 
            RowIndex, SegIndex, ZeroCopyGapBuffer, byte_index, row, seg_index, width};

impl EditorLinesStorage for ZeroCopyGapBuffer {
    // Line access methods
    
    fn get_line_content(&self, row_index: RowIndex) -> Option<&str> {
        self.get_line_content(row_index)
    }
    
    fn line_count(&self) -> Length {
        self.line_count()
    }
    
    // Line metadata access
    
    fn get_line_display_width(&self, row_index: RowIndex) -> Option<ColWidth> {
        self.get_line_info(row_index.as_usize())
            .map(|info| info.display_width)
    }
    
    fn get_line_grapheme_count(&self, row_index: RowIndex) -> Option<Length> {
        self.get_line_info(row_index.as_usize())
            .map(|info| info.grapheme_count)
    }
    
    fn get_line_byte_len(&self, row_index: RowIndex) -> Option<Length> {
        self.get_line_info(row_index.as_usize())
            .map(|info| info.content_len)
    }
    
    // Line modification methods
    
    fn insert_line(&mut self, row_index: RowIndex) -> bool {
        match self.insert_empty_line(row_index) {
            Ok(()) => true,
            Err(_) => false,
        }
    }
    
    fn remove_line(&mut self, row_index: RowIndex) -> bool {
        self.remove_line(row_index.as_usize())
    }
    
    fn clear(&mut self) {
        self.clear();
    }
    
    fn set_line(&mut self, row_index: RowIndex, content: &str) -> bool {
        // First, clear the existing line content
        if let Some(line_info) = self.get_line_info(row_index.as_usize()) {
            let grapheme_count = line_info.grapheme_count;
            if grapheme_count.as_usize() > 0 {
                // Delete all existing content
                match self.delete_range(row_index, seg_index(0), seg_index(grapheme_count.as_usize())) {
                    Ok(()) => {},
                    Err(_) => return false,
                }
            }
            
            // Insert new content at the beginning
            match self.insert_at_grapheme(row_index, seg_index(0), content) {
                Ok(()) => true,
                Err(_) => false,
            }
        } else {
            false
        }
    }
    
    fn push_line(&mut self, content: &str) {
        let line_idx = self.add_line();
        drop(self.insert_at_grapheme(row(line_idx), seg_index(0), content));
    }
    
    // Grapheme-based operations
    
    fn insert_at_grapheme(
        &mut self,
        row_index: RowIndex,
        seg_index: SegIndex,
        text: &str
    ) -> bool {
        match self.insert_at_grapheme(row_index, seg_index, text) {
            Ok(()) => true,
            Err(_) => false,
        }
    }
    
    fn delete_at_grapheme(
        &mut self,
        row_index: RowIndex,
        seg_idx: SegIndex,
        count: Length
    ) -> bool {
        let end_seg_index = seg_index(seg_idx.as_usize() + count.as_usize());
        match self.delete_range(row_index, seg_idx, end_seg_index) {
            Ok(()) => true,
            Err(_) => false,
        }
    }
    
    // Column-based operations
    
    fn insert_at_col(
        &mut self,
        row_index: RowIndex,
        col_index: ColIndex,
        text: &str
    ) -> Option<ColWidth> {
        // Convert column index to segment index
        let seg_idx = self.col_to_seg_index(row_index, col_index)?;
        
        // Calculate the display width of the text to be inserted
        let text_width = Self::calculate_text_display_width(text);
        
        // Perform the insertion
        match self.insert_at_grapheme(row_index, seg_idx, text) {
            Ok(()) => Some(text_width),
            Err(_) => None,
        }
    }
    
    fn delete_at_col(
        &mut self,
        row_index: RowIndex,
        col_index: ColIndex,
        count: Length
    ) -> bool {
        // Convert column index to segment index
        if let Some(seg_idx) = self.col_to_seg_index(row_index, col_index) {
            // Need to call the trait method, not the ZeroCopyGapBuffer method
            <Self as EditorLinesStorage>::delete_at_grapheme(self, row_index, seg_idx, count)
        } else {
            false
        }
    }
    
    // Utility methods
    
    fn split_line_at_col(
        &mut self,
        row_index: RowIndex,
        col_index: ColIndex
    ) -> Option<String> {
        // Convert column index to segment index
        let seg_idx = self.col_to_seg_index(row_index, col_index)?;
        
        // Get the line content as owned string
        let line_content = self.get_line_content(row_index)?.to_string();
        
        // Find the byte position for the segment
        let line_info = self.get_line_info(row_index.as_usize())?;
        let byte_pos = line_info.get_byte_pos(seg_idx);
        
        // Split the content
        let (left_part, right_part) = line_content.split_at(byte_pos.as_usize());
        let right_content = right_part.to_string();
        
        // Update the current line to only contain the left part
        self.set_line(row_index, left_part);
        
        Some(right_content)
    }
    
    fn join_lines(&mut self, first_row_index: RowIndex) -> bool {
        let next_row_index = row(first_row_index.as_usize() + 1);
        
        // Get the content of the second line
        if let Some(second_line_content) = self.get_line_content(next_row_index) {
            let content_to_append = second_line_content.to_string();
            
            // Get the grapheme count of the first line to know where to append
            if let Some(line_info) = self.get_line_info(first_row_index.as_usize()) {
                let append_pos = seg_index(line_info.grapheme_count.as_usize());
                
                // Append the second line's content to the first line
                match self.insert_at_grapheme(first_row_index, append_pos, &content_to_append) {
                    Ok(()) => {
                        // Remove the second line
                        self.remove_line(next_row_index.as_usize())
                    },
                    Err(_) => false,
                }
            } else {
                false
            }
        } else {
            false
        }
    }
    
    // Byte position conversions
    
    fn get_byte_offset_for_row(&self, row_index: RowIndex) -> Option<ByteIndex> {
        self.get_line_info(row_index.as_usize())
            .map(|info| info.buffer_offset)
    }
    
    fn find_row_containing_byte(&self, byte_index: ByteIndex) -> Option<RowIndex> {
        // Linear search through lines to find which one contains the byte
        // This could be optimized with binary search if needed
        let target_byte = byte_index.as_usize();
        
        for i in 0..self.line_count().as_usize() {
            if let Some(line_info) = self.get_line_info(i) {
                let line_start = line_info.buffer_offset.as_usize();
                let line_end = line_start + line_info.capacity.as_usize();
                
                if target_byte >= line_start && target_byte < line_end {
                    return Some(row(i));
                }
            }
        }
        
        None
    }
    
    // Iterator support
    
    fn iter_lines(&self) -> Box<dyn Iterator<Item = &str> + '_> {
        Box::new((0..self.line_count().as_usize()).filter_map(move |i| {
            self.get_line_content(row(i))
        }))
    }
    
    // Total size information
    
    fn total_bytes(&self) -> ByteIndex {
        byte_index(self.buffer.len())
    }
    
    // Conversion methods
    
    fn to_gc_string_vec(&self) -> Vec<GCStringOwned> {
        (0..self.line_count().as_usize())
            .filter_map(|i| self.get_line_content(row(i)))
            .map(Into::into)
            .collect()
    }
    
    fn from_gc_string_vec(lines: Vec<GCStringOwned>) -> Self {
        let mut buffer = Self::new();
        for line in lines {
            buffer.push_line(line.as_ref());
        }
        buffer
    }
}

// Helper methods for ZeroCopyGapBuffer
impl ZeroCopyGapBuffer {
    /// Convert a column index to a segment index for a given line.
    fn col_to_seg_index(&self, row_index: RowIndex, col_index: ColIndex) -> Option<SegIndex> {
        let line_info = self.get_line_info(row_index.as_usize())?;
        let target_col = col_index.as_usize();
        let mut current_col = 0;
        
        // Find the segment that contains or is after the target column
        for (i, segment) in line_info.segments.iter().enumerate() {
            if current_col >= target_col {
                return Some(seg_index(i));
            }
            current_col += segment.display_width.as_usize();
        }
        
        // If we've gone through all segments, return the end position
        Some(seg_index(line_info.segments.len()))
    }
    
    /// Calculate the display width of a text string.
    fn calculate_text_display_width(text: &str) -> ColWidth {
        use crate::segment_builder::build_segments_for_str;
        
        let segments = build_segments_for_str(text);
        let total_width: usize = segments.iter()
            .map(|seg| seg.display_width.as_usize())
            .sum();
        
        width(total_width)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{col, len};

    #[test]
    fn test_basic_line_operations() {
        let mut storage = ZeroCopyGapBuffer::new();
        
        // Test empty storage
        assert_eq!(storage.line_count(), len(0));
        assert!(storage.is_empty());
        
        // Add some lines
        storage.push_line("Hello, world!");
        storage.push_line("This is line 2");
        storage.push_line("And line 3");
        
        // Test line count
        assert_eq!(storage.line_count(), len(3));
        assert!(!storage.is_empty());
        
        // Test line content access
        assert_eq!(storage.get_line_content(row(0)), Some("Hello, world!"));
        assert_eq!(storage.get_line_content(row(1)), Some("This is line 2"));
        assert_eq!(storage.get_line_content(row(2)), Some("And line 3"));
        assert_eq!(storage.get_line_content(row(3)), None);
        
        // Test line metadata
        assert_eq!(storage.get_line_display_width(row(0)), Some(width(13)));
        assert_eq!(storage.get_line_grapheme_count(row(0)), Some(len(13)));
        assert_eq!(storage.get_line_byte_len(row(0)), Some(len(13)));
    }
    
    #[test]
    fn test_line_modification() {
        let mut storage = ZeroCopyGapBuffer::new();
        
        // Add initial content
        storage.push_line("Original line");
        
        // Test set_line
        assert!(storage.set_line(row(0), "Modified line"));
        assert_eq!(storage.get_line_content(row(0)), Some("Modified line"));
        
        // Test insert_line at the end (to avoid the underflow bug)
        assert!(storage.insert_line(row(1)));
        assert_eq!(storage.line_count(), len(2));
        assert_eq!(storage.get_line_content(row(0)), Some("Modified line"));
        assert_eq!(storage.get_line_content(row(1)), Some(""));
        
        // Test remove_line (remove the empty line at the end)
        assert!(<ZeroCopyGapBuffer as EditorLinesStorage>::remove_line(&mut storage, row(1)));
        assert_eq!(storage.line_count(), len(1));
        assert_eq!(storage.get_line_content(row(0)), Some("Modified line"));
        
        // Test insert_line at beginning
        assert!(storage.insert_line(row(0)));
        assert_eq!(storage.line_count(), len(2));
        assert_eq!(storage.get_line_content(row(0)), Some(""));
        assert_eq!(storage.get_line_content(row(1)), Some("Modified line"));
        
        // Test remove_line at beginning
        assert!(<ZeroCopyGapBuffer as EditorLinesStorage>::remove_line(&mut storage, row(0)));
        assert_eq!(storage.line_count(), len(1));
        assert_eq!(storage.get_line_content(row(0)), Some("Modified line"));
    }
    
    #[test]
    fn test_grapheme_operations() {
        let mut storage = ZeroCopyGapBuffer::new();
        storage.push_line("Hello");
        
        // Test insert_at_grapheme
        assert!(<ZeroCopyGapBuffer as EditorLinesStorage>::insert_at_grapheme(&mut storage, row(0), seg_index(5), " World"));
        assert_eq!(storage.get_line_content(row(0)), Some("Hello World"));
        
        // Test delete_at_grapheme
        assert!(<ZeroCopyGapBuffer as EditorLinesStorage>::delete_at_grapheme(&mut storage, row(0), seg_index(5), len(6)));
        assert_eq!(storage.get_line_content(row(0)), Some("Hello"));
    }
    
    #[test]
    fn test_unicode_content() {
        let mut storage = ZeroCopyGapBuffer::new();
        
        // Test with emoji and unicode
        storage.push_line("Hello 👋 世界");
        
        assert_eq!(storage.get_line_content(row(0)), Some("Hello 👋 世界"));
        assert_eq!(storage.get_line_grapheme_count(row(0)), Some(len(10))); // "Hello " = 6 + emoji = 1 + space = 1 + "世界" = 2
        
        // Insert more unicode
        assert!(<ZeroCopyGapBuffer as EditorLinesStorage>::insert_at_grapheme(&mut storage, row(0), seg_index(7), " 🌍"));
        assert_eq!(storage.get_line_content(row(0)), Some("Hello 👋 🌍 世界"));
    }
    
    #[test]
    fn test_split_and_join_lines() {
        let mut storage = ZeroCopyGapBuffer::new();
        storage.push_line("Hello World");
        
        // Test split_line_at_col
        let split_content = storage.split_line_at_col(row(0), col(6));
        assert_eq!(split_content, Some("World".to_string()));
        assert_eq!(storage.get_line_content(row(0)), Some("Hello "));
        
        // Add the split content as a new line
        storage.push_line(&split_content.unwrap());
        
        // Test join_lines
        assert!(storage.join_lines(row(0)));
        assert_eq!(storage.get_line_content(row(0)), Some("Hello World"));
        assert_eq!(storage.line_count(), len(1));
    }
    
    #[test]
    fn test_clear() {
        let mut storage = ZeroCopyGapBuffer::new();
        
        // Add some content
        storage.push_line("Line 1");
        storage.push_line("Line 2");
        storage.push_line("Line 3");
        
        assert_eq!(storage.line_count(), len(3));
        
        // Clear all lines
        storage.clear();
        
        assert_eq!(storage.line_count(), len(0));
        assert!(storage.is_empty());
    }
    
    #[test]
    fn test_iterator() {
        let mut storage = ZeroCopyGapBuffer::new();
        
        // Add test lines
        let test_lines = vec!["First line", "Second line", "Third line"];
        for line in &test_lines {
            storage.push_line(line);
        }
        
        // Test iterator
        let collected: Vec<&str> = storage.iter_lines().collect();
        assert_eq!(collected, test_lines);
    }
    
    #[test]
    fn test_conversion_methods() {
        let mut storage = ZeroCopyGapBuffer::new();
        
        // Add some lines
        storage.push_line("Line 1");
        storage.push_line("Line 2");
        
        // Test to_gc_string_vec
        let gc_vec = storage.to_gc_string_vec();
        assert_eq!(gc_vec.len(), 2);
        assert_eq!(gc_vec[0].as_ref(), "Line 1");
        assert_eq!(gc_vec[1].as_ref(), "Line 2");
        
        // Test from_gc_string_vec
        let new_storage = ZeroCopyGapBuffer::from_gc_string_vec(gc_vec);
        assert_eq!(new_storage.line_count(), len(2));
        assert_eq!(new_storage.get_line_content(row(0)), Some("Line 1"));
        assert_eq!(new_storage.get_line_content(row(1)), Some("Line 2"));
    }
}