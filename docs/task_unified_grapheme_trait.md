<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

- [Universal Grapheme-Aware Trait Design](#universal-grapheme-aware-trait-design)
  - [Overview](#overview)
  - [Core Types](#core-types)
    - [SegContent - Zero-Copy Segment Reference](#segcontent---zero-copy-segment-reference)
  - [Single-Line Traits](#single-line-traits)
    - [GraphemeString - Core Operations](#graphemestring---core-operations)
    - [GraphemeStringMut - Mutation Operations](#graphemestringmut---mutation-operations)
  - [Multi-Line Traits](#multi-line-traits)
    - [GraphemeDoc - Core Document Operations](#graphemedoc---core-document-operations)
    - [GraphemeDocMut - Document Mutation](#graphemedocmut---document-mutation)
  - [Extension Traits](#extension-traits)
    - [GraphemeStringOwnedExt - Ownership Conversions](#graphemestringownedext---ownership-conversions)
  - [Error Handling](#error-handling)
  - [Implementation Strategy](#implementation-strategy)
    - [For GCStringOwned](#for-gcstringowned)
    - [For ZeroCopyGapBuffer](#for-zerocopygapbuffer)
    - [For GCStringOwnedDoc](#for-gcstringowneddoc)
  - [Key Design Benefits](#key-design-benefits)
  - [Migration Strategy](#migration-strategy)
    - [Phase 1: Add trait implementations](#phase-1-add-trait-implementations)
    - [Phase 2: Create adapters](#phase-2-create-adapters)
    - [Phase 3: Update call sites](#phase-3-update-call-sites)
    - [Phase 4: Remove deprecated code](#phase-4-remove-deprecated-code)
  - [Usage Examples](#usage-examples)
    - [Zero-copy segment access](#zero-copy-segment-access)
    - [Working with documents](#working-with-documents)
    - [When ownership is needed](#when-ownership-is-needed)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

# Universal Grapheme-Aware Trait Design

## Overview

This document describes the design for a unified trait system that provides grapheme-aware string
operations for both single-line and multi-line text structures. The design prioritizes zero-copy
operations while maintaining a consistent API across different implementations.

**Single-line types:**

- `GCStringOwned` - Owned grapheme cluster string with pre-computed segments
- `GapBufferLine` - Zero-copy view into a line of `ZeroCopyGapBuffer`

**Multi-line types:**

- `GCStringOwnedDoc` - Simple `Vec<GCStringOwned>` wrapper
- `ZeroCopyGapBuffer` - Efficient gap buffer with contiguous memory

## Core Types

### SegContent - Zero-Copy Segment Reference

```rust
// Core segment content reference for zero-copy access
#[derive(Debug, Clone, Copy)]
pub struct SegContent<'a> {
    content: &'a str,
    seg: Seg,
}

impl<'a> SegContent<'a> {
    pub fn as_str(&self) -> &str { self.content }
    pub fn width(&self) -> ColWidth { self.seg.display_width }
    pub fn start_col(&self) -> ColIndex { self.seg.start_display_col_index }
    pub fn seg(&self) -> &Seg { &self.seg }

    // Convenience methods matching Seg's types
    pub fn byte_range(&self) -> std::ops::Range<ChUnit> {
        self.seg.start_byte_index..self.seg.end_byte_index
    }
}
```

## Single-Line Traits

### GraphemeString - Core Operations

```rust
// Single-line grapheme-aware string trait
pub trait GraphemeString {
    // Associated type for iterator
    type SegIter<'a>: Iterator<Item = Seg> + 'a where Self: 'a;

    // Core properties
    fn as_str(&self) -> &str;
    fn segments(&self) -> &[Seg];
    fn display_width(&self) -> ColWidth;
    fn grapheme_count(&self) -> Length;
    fn byte_size(&self) -> ChUnit;

    // Segment navigation
    fn get_seg(&self, index: SegIndex) -> Option<Seg>;
    fn check_is_in_middle_of_grapheme(&self, col: ColIndex) -> Option<Seg>;

    // Zero-copy segment content access
    fn get_seg_at(&self, col: ColIndex) -> Option<SegContent<'_>>;
    fn get_seg_right_of(&self, col: ColIndex) -> Option<SegContent<'_>>;
    fn get_seg_left_of(&self, col: ColIndex) -> Option<SegContent<'_>>;
    fn get_seg_at_end(&self) -> Option<SegContent<'_>>;

    // Zero-copy string operations
    fn clip(&self, start_col: ColIndex, width: ColWidth) -> &str;

    // Additional string operations from GCStringOwned
    fn trunc_end_to_fit(&self, width: ColWidth) -> &str;
    fn trunc_end_by(&self, width: ColWidth) -> &str;
    fn trunc_start_by(&self, width: ColWidth) -> &str;

    // Iterator
    fn seg_iter(&self) -> Self::SegIter<'_>;

    // Additional methods from GCStringOwned
    fn is_empty(&self) -> bool;
    fn width(&self) -> ColWidth { self.display_width() } // alias
    fn last(&self) -> Option<Seg>;
    fn contains_wide_segments(&self) -> ContainsWideSegments;
}
```

### GraphemeStringMut - Mutation Operations

```rust
// Note: GCStringOwned has immutable mutation methods that return new instances.
// These are not included in the trait since they don't match ZeroCopyGapBuffer's
// in-place mutation pattern. Instead, mutation happens at the document level.
```

## Multi-Line Traits

### GraphemeDoc - Core Document Operations

```rust
// Multi-line document trait
pub trait GraphemeDoc {
    type Line<'a>: GraphemeString where Self: 'a;
    type LineIter<'a>: Iterator<Item = Self::Line<'a>> + 'a where Self: 'a;

    fn line_count(&self) -> Length;
    fn get_line(&self, row: RowIndex) -> Option<Self::Line<'_>>;
    fn is_empty(&self) -> bool { self.line_count() == len(0) }

    // Document-wide operations
    fn total_bytes(&self) -> usize;
    fn iter_lines(&self) -> Self::LineIter<'_>;

    // Additional methods from ZeroCopyGapBuffer
    fn as_str(&self) -> &str;
    fn as_bytes(&self) -> &[u8];
}
```

### GraphemeDocMut - Document Mutation

```rust
// Mutation operations for multi-line documents
pub trait GraphemeDocMut: GraphemeDoc {
    // Line management (from ZeroCopyGapBuffer)
    fn add_line(&mut self) -> usize;
    fn remove_line(&mut self, row: RowIndex) -> bool;
    fn insert_line_with_buffer_shift(&mut self, line_idx: usize);
    fn clear(&mut self);

    // Text mutation (from ZeroCopyGapBuffer)
    fn insert_text_at_grapheme(
        &mut self,
        row: RowIndex,
        seg_index: SegIndex,
        text: &str
    ) -> miette::Result<()>;

    fn delete_range_at_grapheme(
        &mut self,
        row: RowIndex,
        start_seg: SegIndex,
        end_seg: SegIndex
    ) -> miette::Result<()>;

    // Line operations
    fn insert_empty_line(&mut self, row: RowIndex) -> miette::Result<()>;
    fn merge_lines(&mut self, row: RowIndex) -> miette::Result<()>;
    fn split_line(&mut self, row: RowIndex, col: ColIndex) -> miette::Result<()>;
}
```

## Extension Traits

### GraphemeStringOwnedExt - Ownership Conversions

```rust
// Extension trait for when ownership is needed
pub trait GraphemeStringOwnedExt: GraphemeString {
    fn to_owned(&self) -> GCStringOwned {
        GCStringOwned::new(self.as_str())
    }

    fn get_seg_owned_at(&self, col: ColIndex) -> Option<SegStringOwned> {
        self.get_seg_at(col).map(|seg_content| SegStringOwned {
            string: GCStringOwned::from(seg_content.as_str()),
            width: seg_content.width(),
            start_at: seg_content.start_col(),
        })
    }
}

// Auto-implement the extension for all GraphemeString types
impl<T: GraphemeString + ?Sized> GraphemeStringOwnedExt for T {}
```

## Error Handling

```rust
use miette::Diagnostic;
use thiserror::Error;

// Error type for grapheme operations
#[derive(Debug, Clone, PartialEq, Eq, Error, Diagnostic)]
pub enum GraphemeError {
    #[error("Invalid row index: {0}")]
    InvalidRow(RowIndex),

    #[error("Invalid column index: {0}")]
    InvalidColumn(ColIndex),

    #[error("Cursor position falls in middle of grapheme cluster at column {0}")]
    InMiddleOfGrapheme(ColIndex),

    #[error("Index out of bounds")]
    OutOfBounds,

    #[error("Insufficient capacity in buffer")]
    InsufficientCapacity,

    #[error("Invalid UTF-8 sequence")]
    Utf8Error,
}
```

## Implementation Strategy

### For GCStringOwned

```rust
impl GraphemeString for GCStringOwned {
    type SegIter<'a> = std::iter::Copied<std::slice::Iter<'a, Seg>>;

    fn as_str(&self) -> &str { self.string.as_str() }
    fn segments(&self) -> &[Seg] { &self.segments }
    fn display_width(&self) -> ColWidth { self.display_width }
    fn grapheme_count(&self) -> Length { self.len().into() }
    fn byte_size(&self) -> ChUnit { self.bytes_size }

    fn get_seg(&self, index: SegIndex) -> Option<Seg> {
        self.get(index)
    }

    fn check_is_in_middle_of_grapheme(&self, col: ColIndex) -> Option<Seg> {
        self.check_is_in_middle_of_grapheme(col)
    }

    fn get_seg_at(&self, col: ColIndex) -> Option<SegContent<'_>> {
        self.get_string_at(col).map(|seg_string| {
            // Find the segment at this column
            for seg in &self.segments {
                if col >= seg.start_display_col_index
                    && col < seg.start_display_col_index + seg.display_width {
                    return SegContent {
                        content: seg.get_str(self),
                        seg: *seg,
                    };
                }
            }
            unreachable!()
        })
    }

    fn get_seg_right_of(&self, col: ColIndex) -> Option<SegContent<'_>> {
        self.get_string_at_right_of(col).map(|seg_string| {
            for seg in &self.segments {
                if seg.start_display_col_index == seg_string.start_at {
                    return SegContent {
                        content: seg.get_str(self),
                        seg: *seg,
                    };
                }
            }
            unreachable!()
        })
    }

    fn get_seg_left_of(&self, col: ColIndex) -> Option<SegContent<'_>> {
        self.get_string_at_left_of(col).map(|seg_string| {
            for seg in &self.segments {
                if seg.start_display_col_index == seg_string.start_at {
                    return SegContent {
                        content: seg.get_str(self),
                        seg: *seg,
                    };
                }
            }
            unreachable!()
        })
    }

    fn get_seg_at_end(&self) -> Option<SegContent<'_>> {
        self.last().map(|seg| SegContent {
            content: seg.get_str(self),
            seg,
        })
    }

    fn clip(&self, start_col: ColIndex, width: ColWidth) -> &str {
        self.clip(start_col, width)
    }

    fn trunc_end_to_fit(&self, width: ColWidth) -> &str {
        self.trunc_end_to_fit(width)
    }

    fn trunc_end_by(&self, width: ColWidth) -> &str {
        self.trunc_end_by(width)
    }

    fn trunc_start_by(&self, width: ColWidth) -> &str {
        self.trunc_start_by(width)
    }

    fn seg_iter(&self) -> Self::SegIter<'_> {
        self.segments.iter().copied()
    }

    fn is_empty(&self) -> bool { self.is_empty() }
    fn last(&self) -> Option<Seg> { self.last() }
    fn contains_wide_segments(&self) -> ContainsWideSegments {
        self.contains_wide_segments()
    }
}
```

### For ZeroCopyGapBuffer

```rust
// GapBufferLine already exists, it just needs to implement GraphemeString
impl<'a> GraphemeString for GapBufferLine<'a> {
    type SegIter<'b> = std::iter::Copied<std::slice::Iter<'b, Seg>> where Self: 'b;

    fn as_str(&self) -> &str { self.content() }
    fn segments(&self) -> &[Seg] { self.segments() }
    fn display_width(&self) -> ColWidth { self.display_width() }
    fn grapheme_count(&self) -> Length { self.grapheme_count() }
    fn byte_size(&self) -> ChUnit { ch(self.content().len()) }

    fn get_seg(&self, index: SegIndex) -> Option<Seg> {
        self.info().segments.get(index.as_usize()).copied()
    }

    fn check_is_in_middle_of_grapheme(&self, col: ColIndex) -> Option<Seg> {
        self.check_is_in_middle_of_grapheme(col)
    }

    fn get_seg_at(&self, col: ColIndex) -> Option<SegContent<'_>> {
        // Reuse existing get_string_at but return SegContent
        self.get_string_at(col).map(|seg_string| {
            for seg in self.segments() {
                if seg.start_display_col_index == seg_string.start_at {
                    return SegContent {
                        content: seg.get_str(self.content()),
                        seg: *seg,
                    };
                }
            }
            unreachable!()
        })
    }

    fn get_seg_right_of(&self, col: ColIndex) -> Option<SegContent<'_>> {
        self.get_string_at_right_of(col).map(|seg_string| {
            for seg in self.segments() {
                if seg.start_display_col_index == seg_string.start_at {
                    return SegContent {
                        content: seg.get_str(self.content()),
                        seg: *seg,
                    };
                }
            }
            unreachable!()
        })
    }

    fn get_seg_left_of(&self, col: ColIndex) -> Option<SegContent<'_>> {
        self.get_string_at_left_of(col).map(|seg_string| {
            for seg in self.segments() {
                if seg.start_display_col_index == seg_string.start_at {
                    return SegContent {
                        content: seg.get_str(self.content()),
                        seg: *seg,
                    };
                }
            }
            unreachable!()
        })
    }

    fn get_seg_at_end(&self) -> Option<SegContent<'_>> {
        self.get_string_at_end().map(|seg_string| {
            for seg in self.segments() {
                if seg.start_display_col_index == seg_string.start_at {
                    return SegContent {
                        content: seg.get_str(self.content()),
                        seg: *seg,
                    };
                }
            }
            unreachable!()
        })
    }

    fn clip(&self, start_col: ColIndex, width: ColWidth) -> &str {
        self.info().clip_to_range(self.content(), start_col, width)
    }

    fn trunc_end_to_fit(&self, width: ColWidth) -> &str {
        // Need to implement using LineMetadata's segments
        // Similar to GCStringOwned::trunc_end_to_fit
        todo!()
    }

    fn trunc_end_by(&self, width: ColWidth) -> &str {
        todo!()
    }

    fn trunc_start_by(&self, width: ColWidth) -> &str {
        todo!()
    }

    fn seg_iter(&self) -> Self::SegIter<'_> {
        self.segments().iter().copied()
    }

    fn is_empty(&self) -> bool { self.is_empty() }
    fn last(&self) -> Option<Seg> { self.segments().last().copied() }
    fn contains_wide_segments(&self) -> ContainsWideSegments {
        if self.segments().iter().any(|seg| seg.display_width > width(1)) {
            ContainsWideSegments::Yes
        } else {
            ContainsWideSegments::No
        }
    }
}

impl GraphemeDoc for ZeroCopyGapBuffer {
    type Line<'a> = GapBufferLine<'a>;
    type LineIter<'a> = impl Iterator<Item = GapBufferLine<'a>> + 'a;

    fn get_line(&self, row: RowIndex) -> Option<Self::Line<'_>> {
        self.get_line(row)
    }

    fn line_count(&self) -> Length {
        self.line_count()
    }

    fn total_bytes(&self) -> usize {
        self.buffer.len()
    }

    fn iter_lines(&self) -> Self::LineIter<'_> {
        (0..self.line_count().as_usize())
            .filter_map(move |i| self.get_line(row(i)))
    }

    fn as_str(&self) -> &str {
        self.as_str()
    }

    fn as_bytes(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl GraphemeDocMut for ZeroCopyGapBuffer {
    fn add_line(&mut self) -> usize {
        self.add_line()
    }

    fn remove_line(&mut self, row: RowIndex) -> bool {
        self.remove_line(row)
    }

    fn insert_line_with_buffer_shift(&mut self, line_idx: usize) {
        self.insert_line_with_buffer_shift(line_idx)
    }

    fn clear(&mut self) {
        self.clear()
    }

    fn insert_text_at_grapheme(
        &mut self,
        row: RowIndex,
        seg_index: SegIndex,
        text: &str
    ) -> miette::Result<()> {
        self.insert_text_at_grapheme(row, seg_index, text)
    }

    fn delete_range_at_grapheme(
        &mut self,
        row: RowIndex,
        start_seg: SegIndex,
        end_seg: SegIndex
    ) -> miette::Result<()> {
        self.delete_range_at_grapheme(row, start_seg, end_seg)
    }

    fn insert_empty_line(&mut self, row: RowIndex) -> miette::Result<()> {
        self.insert_empty_line(row)
    }

    fn merge_lines(&mut self, row: RowIndex) -> miette::Result<()> {
        self.merge_lines(row)
    }

    fn split_line(&mut self, row: RowIndex, col: ColIndex) -> miette::Result<()> {
        self.split_line(row, col)
    }
}
```

### For GCStringOwnedDoc

```rust
impl GraphemeDoc for GCStringOwnedDoc {
    type Line<'a> = &'a GCStringOwned;
    type LineIter<'a> = std::slice::Iter<'a, GCStringOwned>;

    fn line_count(&self) -> Length { len(self.lines.len()) }

    fn get_line(&self, row: RowIndex) -> Option<Self::Line<'_>> {
        self.lines.get(row.as_usize())
    }

    fn total_bytes(&self) -> usize {
        self.lines.iter().map(|line| line.byte_size().as_usize()).sum()
    }

    fn iter_lines(&self) -> Self::LineIter<'_> {
        self.lines.iter()
    }

    fn as_str(&self) -> &str {
        // GCStringOwnedDoc doesn't have a single string representation
        // Could join lines with newlines, but that would allocate
        ""
    }

    fn as_bytes(&self) -> &[u8] {
        // Same issue as as_str()
        &[]
    }
}

impl GraphemeDocMut for GCStringOwnedDoc {
    fn add_line(&mut self) -> usize {
        let index = self.lines.len();
        self.lines.push(GCStringOwned::new(""));
        index
    }

    fn remove_line(&mut self, row: RowIndex) -> bool {
        if row.as_usize() < self.lines.len() {
            self.lines.remove(row.as_usize());
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
    }

    fn insert_text_at_grapheme(
        &mut self,
        row: RowIndex,
        seg_index: SegIndex,
        text: &str
    ) -> miette::Result<()> {
        let line = self.lines.get_mut(row.as_usize())
            .ok_or(GraphemeError::InvalidRow(row))?;

        // GCStringOwned uses immutable operations, so we need to replace the line
        let col_index = (line + seg_index)
            .ok_or(GraphemeError::InvalidColumn(col(seg_index.as_usize())))?;

        if let Some(new_line) = line.insert_at_display_col_index(col_index, text) {
            *line = new_line;
            Ok(())
        } else {
            Err(GraphemeError::InvalidColumn(col_index).into())
        }
    }

    fn delete_range_at_grapheme(
        &mut self,
        row: RowIndex,
        start_seg: SegIndex,
        end_seg: SegIndex
    ) -> miette::Result<()> {
        let line = self.lines.get_mut(row.as_usize())
            .ok_or(GraphemeError::InvalidRow(row))?;

        // Convert segment indices to column indices
        let start_col = (line + start_seg)
            .ok_or(GraphemeError::InvalidColumn(col(start_seg.as_usize())))?;
        let end_col = (line + end_seg)
            .ok_or(GraphemeError::InvalidColumn(col(end_seg.as_usize())))?;

        // GCStringOwned doesn't have delete_range, so we'd need to implement it
        // using the existing methods
        if let Some(new_line) = line.delete_at_display_col_index_to_end(start_col) {
            // Then insert the part after end_col back
            // This is a simplified implementation
            *line = new_line;
            Ok(())
        } else {
            Err(GraphemeError::InvalidColumn(start_col).into())
        }
    }

    fn insert_empty_line(&mut self, row: RowIndex) -> miette::Result<()> {
        if row.as_usize() <= self.lines.len() {
            self.lines.insert(row.as_usize(), GCStringOwned::new(""));
            Ok(())
        } else {
            Err(GraphemeError::InvalidRow(row).into())
        }
    }

    fn merge_lines(&mut self, row: RowIndex) -> miette::Result<()> {
        if row.as_usize() + 1 < self.lines.len() {
            let next_line = self.lines.remove(row.as_usize() + 1);
            let current_line = &mut self.lines[row.as_usize()];

            // Append next line to current line
            if let Some(merged) = current_line.push(next_line.as_str()) {
                *current_line = merged;
            }
            Ok(())
        } else {
            Err(GraphemeError::InvalidRow(row).into())
        }
    }

    fn split_line(&mut self, row: RowIndex, col: ColIndex) -> miette::Result<()> {
        let line = self.lines.get_mut(row.as_usize())
            .ok_or(GraphemeError::InvalidRow(row))?;

        if let Some((before, after)) = line.split_at_display_col_index(col) {
            *line = before;
            self.lines.insert(row.as_usize() + 1, after);
            Ok(())
        } else {
            Err(GraphemeError::InvalidColumn(col).into())
        }
    }
}
```

## Key Design Benefits

1. **Zero-copy by default**: All core operations return references, not owned types
2. **Type consistency**: Uses existing types (`ChUnit`, `ColIndex`, etc.) throughout
3. **Minimal abstractions**: Reuses existing `Seg` type instead of creating new traits
4. **Unified interface**: Same operations work on both `GCStringOwned` and `ZeroCopyGapBuffer`
5. **Migration friendly**: Existing code can gradually adopt traits without breaking changes
6. **Performance**: No allocations for common operations like getting segment content
7. **Extensibility**: Extension traits provide owned variants when explicitly needed

## Migration Strategy

### Phase 1: Add trait implementations

- Implement traits for existing types without breaking current APIs
- All existing methods continue to work

### Phase 2: Create adapters

```rust
impl GCStringOwned {
    /// Temporary adapter during migration
    #[deprecated(note = "Use GraphemeString trait methods")]
    pub fn as_grapheme_string(&self) -> &dyn GraphemeString {
        self
    }
}
```

### Phase 3: Update call sites

- Gradually update code to use trait methods
- Compiler warnings guide the migration

### Phase 4: Remove deprecated code

- Remove editor-specific modules from `GCStringOwned`
- All functionality available through traits

## Usage Examples

### Zero-copy segment access

```rust
let line: &dyn GraphemeString = &gc_string;
if let Some(seg_content) = line.get_seg_at(col(5)) {
    println!("Content: {}", seg_content.as_str());
    println!("Width: {}", seg_content.width());
    println!("Byte range: {:?}", seg_content.byte_range());
}
```

### Working with documents

```rust
let doc: &dyn GraphemeDoc = &gap_buffer;
for line in doc.iter_lines() {
    println!("Line width: {}", line.display_width());
    for seg in line.seg_iter() {
        // Process each segment
    }
}
```

### When ownership is needed

```rust
use GraphemeStringOwnedExt;

let owned_seg = line.get_seg_owned_at(col(10));
let owned_copy = line.to_owned();
```
