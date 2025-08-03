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
    - [Source of Truth Principle](#source-of-truth-principle)
    - [For GCStringOwned](#for-gcstringowned)
    - [For ZeroCopyGapBuffer](#for-zerocopygapbuffer)
    - [For GCStringOwnedDoc](#for-gcstringowneddoc)
  - [Key Design Benefits](#key-design-benefits)
  - [Technical Challenges Addressed](#technical-challenges-addressed)
  - [Migration Strategy](#migration-strategy)
    - [Phase 1: Add segment_count() method and trait implementations](#phase-1-add-segment_count-method-and-trait-implementations)
    - [Phase 2: Create adapters](#phase-2-create-adapters)
    - [Phase 3: Update call sites](#phase-3-update-call-sites)
    - [Phase 4: Remove deprecated code](#phase-4-remove-deprecated-code)
  - [Usage Examples](#usage-examples)
    - [Zero-copy segment access](#zero-copy-segment-access)
    - [Working with associated types for string operations](#working-with-associated-types-for-string-operations)
    - [Working with documents](#working-with-documents)
    - [Mutation with associated types](#mutation-with-associated-types)
    - [When ownership is needed](#when-ownership-is-needed)
  - [File Structure and Implementation Plan](#file-structure-and-implementation-plan)
    - [New Folders and Files to Create](#new-folders-and-files-to-create)
    - [Files to Modify](#files-to-modify)
    - [Implementation Phases](#implementation-phases)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

# Universal Grapheme-Aware Trait Design

## Overview

This document describes the design for a unified trait system that provides grapheme-aware string
operations for both single-line and multi-line text structures. The design prioritizes zero-copy
operations while maintaining a consistent API across different implementations.

**Key Design Principles:**

- **Semantic Clarity**: Use `SegWidth` for grapheme cluster segment counts, `Length` for array
  lengths
- **Zero-Copy**: Prioritize borrowed references over owned types for performance
- **Flexible Returns**: Use `Cow<str>` for document-wide operations to support both borrowed and
  owned data
- **Associated Types**: Handle different mutation paradigms elegantly through associated types
- **Source of Truth**: Use existing implementations (GCStringOwned) as canonical reference for
  algorithms

**Single-line types:**

- `GCStringOwned` - Owned grapheme cluster string with pre-computed segments
- `GapBufferLine` - Zero-copy view into a line of `ZeroCopyGapBuffer`

**Multi-line types:**

- `GCStringOwnedDoc` - Simple `Vec<GCStringOwned>` wrapper
- `ZeroCopyGapBuffer` - Efficient gap buffer with contiguous memory

## Core Types

### SegContent - Zero-Copy Segment Reference

```rust
use std::borrow::Cow;

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
    type SegmentIterator<'a>: Iterator<Item = Seg> + 'a where Self: 'a;

    // Associated type for string slice operations
    // This allows GCStringOwned to return CowInlineString while
    // ZeroCopyGapBuffer returns &str
    type StringSlice<'a>: AsRef<str> + Display where Self: 'a;

    // Core properties
    fn as_str(&self) -> &str;
    fn segments(&self) -> &[Seg];
    fn display_width(&self) -> ColWidth;
    fn segment_count(&self) -> SegWidth;  // ✅ Semantic clarity for grapheme segments
    fn byte_size(&self) -> ChUnit;

    // Segment navigation
    fn get_seg(&self, index: SegIndex) -> Option<Seg>;
    fn check_is_in_middle_of_grapheme(&self, col: ColIndex) -> Option<Seg>;

    // Zero-copy segment content access
    fn get_seg_at(&self, col: ColIndex) -> Option<SegContent<'_>>;
    fn get_seg_right_of(&self, col: ColIndex) -> Option<SegContent<'_>>;
    fn get_seg_left_of(&self, col: ColIndex) -> Option<SegContent<'_>>;
    fn get_seg_at_end(&self) -> Option<SegContent<'_>>;

    // String operations using associated type
    fn clip(&self, start_col: ColIndex, width: ColWidth) -> Self::StringSlice<'_>;
    fn trunc_end_to_fit(&self, width: ColWidth) -> Self::StringSlice<'_>;
    fn trunc_end_by(&self, width: ColWidth) -> Self::StringSlice<'_>;
    fn trunc_start_by(&self, width: ColWidth) -> Self::StringSlice<'_>;

    // Iterator
    fn segments_iter(&self) -> Self::SegmentIterator<'_>;

    // Additional methods from GCStringOwned
    fn is_empty(&self) -> bool;
    fn last(&self) -> Option<Seg>;
    fn contains_wide_segments(&self) -> ContainsWideSegments;
}
```

### GraphemeStringMut - Mutation Operations

```rust
// Mutation operations for single-line strings using associated types
// to handle different paradigms (immutable vs mutable operations)
pub trait GraphemeStringMut: GraphemeString {
    type MutResult;  // Associated type handles paradigm differences elegantly

    fn insert_text(&mut self, col: ColIndex, text: &str) -> Option<Self::MutResult>;
    fn delete_range(&mut self, start: ColIndex, end: ColIndex) -> Option<Self::MutResult>;

    // Additional mutation operations
    fn replace_range(&mut self, start: ColIndex, end: ColIndex, text: &str) -> Option<Self::MutResult>;
    fn truncate(&mut self, col: ColIndex) -> Option<Self::MutResult>;
}
```

## Multi-Line Traits

### GraphemeDoc - Core Document Operations

```rust
// Multi-line document trait
pub trait GraphemeDoc {
    type Line<'a>: GraphemeString where Self: 'a;
    type LineIterator<'a>: Iterator<Item = Self::Line<'a>> + 'a where Self: 'a;

    fn line_count(&self) -> Length;  // ✅ Use Length for array length (line count)
    fn get_line(&self, row: RowIndex) -> Option<Self::Line<'_>>;
    fn is_empty(&self) -> bool { self.line_count() == Length::from(0) }

    // Document-wide operations
    fn total_bytes(&self) -> usize;
    fn iter_lines(&self) -> Self::LineIterator<'_>;

    // ✅ Document-wide string operations with Cow<str> for flexibility
    fn as_str(&self) -> Cow<str>;
    fn as_bytes(&self) -> Cow<[u8]>;
}
```

### GraphemeDocMut - Document Mutation

```rust
// Mutation operations for multi-line documents
pub trait GraphemeDocMut: GraphemeDoc {
    type DocMutResult;  // Associated type for document mutation results

    // Line management (from ZeroCopyGapBuffer)
    fn add_line(&mut self) -> usize;
    fn remove_line(&mut self, row: RowIndex) -> bool;
    fn insert_line_with_buffer_shift(&mut self, line_idx: usize);
    fn clear(&mut self);

    // Text mutation using associated result type
    fn insert_text_at_grapheme(
        &mut self,
        row: RowIndex,
        seg_index: SegIndex,
        text: &str
    ) -> miette::Result<Self::DocMutResult>;

    fn delete_range_at_grapheme(
        &mut self,
        row: RowIndex,
        start_seg: SegIndex,
        end_seg: SegIndex
    ) -> miette::Result<Self::DocMutResult>;

    // Line operations
    fn insert_empty_line(&mut self, row: RowIndex) -> miette::Result<Self::DocMutResult>;
    fn merge_lines(&mut self, row: RowIndex) -> miette::Result<Self::DocMutResult>;
    fn split_line(&mut self, row: RowIndex, col: ColIndex) -> miette::Result<Self::DocMutResult>;
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

Error handling is implementation-specific. Most implementations use `miette::Result` with contextual
error messages created using the `miette!` macro. This provides flexibility for each implementation
to define appropriate error handling strategies without imposing a rigid error type hierarchy.

Example:

```rust
let line = self.lines.get_mut(row.as_usize())
    .ok_or_else(|| miette!("Invalid row index: {:?}", row))?;
```

## Implementation Strategy

### Source of Truth Principle

All implementations follow the **source of truth principle**: use existing, proven implementations
from `GCStringOwned` as the canonical algorithms, then adapt them for other types.

### For GCStringOwned

```rust
impl GraphemeString for GCStringOwned {
    type SegmentIterator<'a> = std::iter::Copied<std::slice::Iter<'a, Seg>>;

    // GCStringOwned can return CowInlineString for string operations
    type StringSlice<'a> = CowInlineString<'a>;

    fn as_str(&self) -> &str { self.string.as_str() }
    fn segments(&self) -> &[Seg] { &self.segments }
    fn display_width(&self) -> ColWidth { self.display_width }

    // ✅ Use SegWidth and delegate to new segment_count() method
    fn segment_count(&self) -> SegWidth { self.segment_count() }
    fn byte_size(&self) -> ChUnit { self.bytes_size }

    fn get_seg(&self, index: SegIndex) -> Option<Seg> {
        self.get(index)
    }

    fn check_is_in_middle_of_grapheme(&self, col: ColIndex) -> Option<Seg> {
        self.check_is_in_middle_of_grapheme(col)
    }

    // ✅ Simplified: Use existing efficient methods instead of complex iteration
    fn get_seg_at(&self, col: ColIndex) -> Option<SegContent<'_>> {
        self.get_string_at(col).and_then(|seg_string| {
            self.segments.iter().find(|seg| {
                seg.start_display_col_index == seg_string.start_at
            }).map(|seg| SegContent {
                content: seg.get_str(self),
                seg: *seg,
            })
        })
    }

    fn get_seg_right_of(&self, col: ColIndex) -> Option<SegContent<'_>> {
        self.get_string_at_right_of(col).and_then(|seg_string| {
            self.segments.iter().find(|seg| {
                seg.start_display_col_index == seg_string.start_at
            }).map(|seg| SegContent {
                content: seg.get_str(self),
                seg: *seg,
            })
        })
    }

    fn get_seg_left_of(&self, col: ColIndex) -> Option<SegContent<'_>> {
        self.get_string_at_left_of(col).and_then(|seg_string| {
            self.segments.iter().find(|seg| {
                seg.start_display_col_index == seg_string.start_at
            }).map(|seg| SegContent {
                content: seg.get_str(self),
                seg: *seg,
            })
        })
    }

    fn get_seg_at_end(&self) -> Option<SegContent<'_>> {
        self.last().map(|seg| SegContent {
            content: seg.get_str(self),
            seg,
        })
    }

    // ✅ String operations can return CowInlineString for efficiency
    fn clip(&self, start_col: ColIndex, width: ColWidth) -> Self::StringSlice<'_> {
        CowInlineString::Borrowed(self.clip(start_col, width))
    }

    fn trunc_end_to_fit(&self, width: ColWidth) -> Self::StringSlice<'_> {
        CowInlineString::Borrowed(self.trunc_end_to_fit(width))
    }

    fn trunc_end_by(&self, width: ColWidth) -> Self::StringSlice<'_> {
        CowInlineString::Borrowed(self.trunc_end_by(width))
    }

    fn trunc_start_by(&self, width: ColWidth) -> Self::StringSlice<'_> {
        CowInlineString::Borrowed(self.trunc_start_by(width))
    }

    fn segments_iter(&self) -> Self::SegmentIterator<'_> {
        self.segments.iter().copied()
    }

    fn is_empty(&self) -> bool { self.is_empty() }
    fn last(&self) -> Option<Seg> { self.last() }
    fn contains_wide_segments(&self) -> ContainsWideSegments {
        self.contains_wide_segments()
    }
}

// ✅ Associated type implementation for mutation paradigm
// Note: GCStringOwned follows an immutable pattern - operations return new strings
impl GraphemeStringMut for GCStringOwned {
    type MutResult = GCStringOwned;  // Returns new instances (immutable paradigm)

    fn insert_text(&mut self, col: ColIndex, text: &str) -> Option<Self::MutResult> {
        // Uses the existing insert_chunk_at_col method
        let (new_string, _width) = self.insert_chunk_at_col(col, text);
        Some(GCStringOwned::new(new_string))
    }

    fn delete_range(&mut self, start: ColIndex, end: ColIndex) -> Option<Self::MutResult> {
        // Combines split_at_display_col operations to delete a range
        if let Some((left, _)) = self.split_at_display_col(start) {
            if let Some((_, right)) = self.split_at_display_col(end) {
                // Combine left part with right part
                let combined = format!("{}{}", left, right);
                Some(GCStringOwned::new(combined))
            } else {
                // Nothing after end, just return left part
                Some(GCStringOwned::new(left))
            }
        } else {
            None
        }
    }

    fn replace_range(&mut self, start: ColIndex, end: ColIndex, text: &str) -> Option<Self::MutResult> {
        // Delete the range first, then insert the replacement text
        self.delete_range(start, end)
            .and_then(|mut deleted| deleted.insert_text(start, text))
    }

    fn truncate(&mut self, col: ColIndex) -> Option<Self::MutResult> {
        // Split at column and return the left part
        if let Some((left, _)) = self.split_at_display_col(col) {
            Some(GCStringOwned::new(left))
        } else {
            None
        }
    }
}
```

### For ZeroCopyGapBuffer

```rust
// GapBufferLine implements GraphemeString by adapting GCStringOwned algorithms
impl<'a> GraphemeString for GapBufferLine<'a> {
    type SegmentIterator<'b> = std::iter::Copied<std::slice::Iter<'b, Seg>> where Self: 'b;

    // ZeroCopyGapBuffer uses &str for zero-copy string operations
    type StringSlice<'b> = &'b str where Self: 'b;

    fn as_str(&self) -> &str { self.content() }
    fn segments(&self) -> &[Seg] { self.segments() }
    fn display_width(&self) -> ColWidth { self.display_width() }

    // ✅ Use SegWidth consistently
    fn segment_count(&self) -> SegWidth {
        SegWidth::from(self.info.grapheme_count.as_usize())
    }
    fn byte_size(&self) -> ChUnit { ch(self.content().len()) }

    fn get_seg(&self, index: SegIndex) -> Option<Seg> {
        self.info().segments.get(index.as_usize()).copied()
    }

    fn check_is_in_middle_of_grapheme(&self, col: ColIndex) -> Option<Seg> {
        self.check_is_in_middle_of_grapheme(col)
    }

    // ✅ Use existing efficient methods instead of complex iteration
    fn get_seg_at(&self, col: ColIndex) -> Option<SegContent<'_>> {
        self.get_string_at(col).and_then(|seg_string| {
            self.segments().iter().find(|seg| {
                seg.start_display_col_index == seg_string.start_at
            }).map(|seg| SegContent {
                content: seg.get_str(self.content()),
                seg: *seg,
            })
        })
    }

    fn get_seg_right_of(&self, col: ColIndex) -> Option<SegContent<'_>> {
        self.get_string_at_right_of(col).and_then(|seg_string| {
            self.segments().iter().find(|seg| {
                seg.start_display_col_index == seg_string.start_at
            }).map(|seg| SegContent {
                content: seg.get_str(self.content()),
                seg: *seg,
            })
        })
    }

    fn get_seg_left_of(&self, col: ColIndex) -> Option<SegContent<'_>> {
        self.get_string_at_left_of(col).and_then(|seg_string| {
            self.segments().iter().find(|seg| {
                seg.start_display_col_index == seg_string.start_at
            }).map(|seg| SegContent {
                content: seg.get_str(self.content()),
                seg: *seg,
            })
        })
    }

    fn get_seg_at_end(&self) -> Option<SegContent<'_>> {
        self.get_string_at_end().and_then(|seg_string| {
            self.segments().iter().find(|seg| {
                seg.start_display_col_index == seg_string.start_at
            }).map(|seg| SegContent {
                content: seg.get_str(self.content()),
                seg: *seg,
            })
        })
    }

    // ✅ Zero-copy string operations return &str directly
    fn clip(&self, start_col: ColIndex, width: ColWidth) -> Self::StringSlice<'_> {
        self.info().clip_to_range(self.content(), start_col, width)
    }

    // ✅ Implement using GCStringOwned algorithms adapted for LineMetadata
    fn trunc_end_to_fit(&self, width: ColWidth) -> Self::StringSlice<'_> {
        // Source of truth: GCStringOwned::trunc_end_to_fit algorithm
        let mut avail_cols = width;
        let mut string_end_byte_index = 0;

        for seg in self.segments() {
            let seg_display_width = seg.display_width;
            if avail_cols < seg_display_width {
                break;
            }
            string_end_byte_index += seg.bytes_size.as_usize();
            avail_cols -= seg_display_width;
        }

        &self.content()[..string_end_byte_index.min(self.content().len())]
    }

    fn trunc_end_by(&self, width: ColWidth) -> Self::StringSlice<'_> {
        // Source of truth: GCStringOwned::trunc_end_by algorithm
        let mut countdown_col_count = width;
        let mut string_end_byte_index = ch(0);

        for seg in self.segments().iter().rev() {
            let seg_display_width = seg.display_width;
            string_end_byte_index = seg.start_byte_index;
            countdown_col_count -= seg_display_width;
            if *countdown_col_count == ch(0) {
                break;
            }
        }

        &self.content()[..string_end_byte_index.as_usize().min(self.content().len())]
    }

    fn trunc_start_by(&self, width: ColWidth) -> Self::StringSlice<'_> {
        // Adapt GCStringOwned algorithm for starting from beginning
        let mut skip_col_count = width;
        let mut string_start_byte_index = 0;

        for seg in self.segments() {
            let seg_display_width = seg.display_width;
            if *skip_col_count == ch(0) {
                break;
            }
            skip_col_count -= seg_display_width;
            string_start_byte_index += seg.bytes_size.as_usize();
        }

        &self.content()[string_start_byte_index.min(self.content().len())..]
    }

    fn segments_iter(&self) -> Self::SegmentIterator<'_> {
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
    type LineIterator<'a> = impl Iterator<Item = GapBufferLine<'a>> + 'a;

    fn get_line(&self, row: RowIndex) -> Option<Self::Line<'_>> {
        self.get_line(row)
    }

    // ✅ Use Length for line count (array length semantics)
    fn line_count(&self) -> Length {
        self.line_count()
    }

    fn total_bytes(&self) -> usize {
        self.buffer.len()
    }

    fn iter_lines(&self) -> Self::LineIterator<'_> {
        (0..self.line_count().as_usize())
            .filter_map(move |i| self.get_line(row(i)))
    }

    // ✅ Use Cow::Borrowed for zero-copy
    fn as_str(&self) -> Cow<str> {
        Cow::Borrowed(self.as_str())
    }

    fn as_bytes(&self) -> Cow<[u8]> {
        Cow::Borrowed(self.as_bytes())
    }
}

impl GraphemeDocMut for ZeroCopyGapBuffer {
    type DocMutResult = ();  // In-place mutations return unit

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
    ) -> miette::Result<Self::DocMutResult> {
        self.insert_text_at_grapheme(row, seg_index, text)
            .map(|_| ())
    }

    fn delete_range_at_grapheme(
        &mut self,
        row: RowIndex,
        start_seg: SegIndex,
        end_seg: SegIndex
    ) -> miette::Result<Self::DocMutResult> {
        self.delete_range_at_grapheme(row, start_seg, end_seg)
            .map(|_| ())
    }

    fn insert_empty_line(&mut self, row: RowIndex) -> miette::Result<Self::DocMutResult> {
        self.insert_empty_line(row)
            .map(|_| ())
    }

    fn merge_lines(&mut self, row: RowIndex) -> miette::Result<Self::DocMutResult> {
        self.merge_lines(row)
            .map(|_| ())
    }

    fn split_line(&mut self, row: RowIndex, col: ColIndex) -> miette::Result<Self::DocMutResult> {
        self.split_line(row, col)
            .map(|_| ())
    }
}
```

### For GCStringOwnedDoc

```rust
impl GraphemeDoc for GCStringOwnedDoc {
    type Line<'a> = &'a GCStringOwned;
    type LineIterator<'a> = std::slice::Iter<'a, GCStringOwned>;

    // ✅ Use Length for line count (array length semantics)
    fn line_count(&self) -> Length {
        Length::from(self.lines.len())
    }

    fn get_line(&self, row: RowIndex) -> Option<Self::Line<'_>> {
        self.lines.get(row.as_usize())
    }

    fn total_bytes(&self) -> usize {
        self.lines.iter().map(|line| line.byte_size().as_usize()).sum()
    }

    fn iter_lines(&self) -> Self::LineIterator<'_> {
        self.lines.iter()
    }

    // ✅ Use Cow::Owned to create meaningful string representation
    fn as_str(&self) -> Cow<str> {
        Cow::Owned(
            self.lines
                .iter()
                .map(|line| line.as_str())
                .collect::<Vec<_>>()
                .join("\n")
        )
    }

    fn as_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(self.as_str().into_owned().into_bytes())
    }
}

impl GraphemeDocMut for GCStringOwnedDoc {
    type DocMutResult = ();  // Simple unit return for document mutations

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
    ) -> miette::Result<Self::DocMutResult> {
        let line = self.lines.get_mut(row.as_usize())
            .ok_or_else(|| miette!("Invalid row index: {:?}", row))?;

        // Convert segment index to column index
        let col_index = if seg_index.as_usize() == 0 {
            col(0)
        } else if let Some(seg) = line.get(seg_index) {
            seg.start_display_col_index
        } else {
            // Insert at end
            line.display_width().into()
        };

        if let Some(new_line) = line.insert_at_display_col_index(col_index, text) {
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
        end_seg: SegIndex
    ) -> miette::Result<Self::DocMutResult> {
        let line = self.lines.get_mut(row.as_usize())
            .ok_or_else(|| miette!("Invalid row index: {:?}", row))?;

        // Convert segment indices to column indices
        let start_col = if let Some(seg) = line.get(start_seg) {
            seg.start_display_col_index
        } else {
            return Err(miette!("Invalid column index: {:?}", col(start_seg.as_usize())));
        };

        let end_col = if let Some(seg) = line.get(end_seg) {
            seg.start_display_col_index
        } else {
            line.display_width().into()
        };

        // Use existing methods to perform the deletion
        if let Some(before) = line.trunc_at_display_col_index(start_col) {
            if let Some(after_part) = line.slice_from_display_col_index(end_col) {
                if let Some(combined) = before.push(after_part) {
                    *line = combined;
                    return Ok(());
                }
            }
            *line = before;
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

            // Append next line to current line
            if let Some(merged) = current_line.push(next_line.as_str()) {
                *current_line = merged;
            }
            Ok(())
        } else {
            Err(miette!("Invalid row index: {:?}", row))
        }
    }

    fn split_line(&mut self, row: RowIndex, col: ColIndex) -> miette::Result<Self::DocMutResult> {
        let line = self.lines.get_mut(row.as_usize())
            .ok_or_else(|| miette!("Invalid row index: {:?}", row))?;

        if let Some(before) = line.trunc_at_display_col_index(col) {
            if let Some(after) = line.slice_from_display_col_index(col) {
                *line = before;
                self.lines.insert(row.as_usize() + 1, after);
                Ok(())
            } else {
                // No content after split point
                *line = before;
                self.lines.insert(row.as_usize() + 1, GCStringOwned::new(""));
                Ok(())
            }
        } else {
            Err(miette!("Invalid column index: {:?}", col))
        }
    }
}
```

## Key Design Benefits

1. **Source of Truth Principle**: All implementations use proven algorithms from `GCStringOwned` as
   the canonical reference, eliminating duplication and ensuring consistency
2. **Semantic Type Safety**: `SegWidth` clearly indicates grapheme segment counts, `Length`
   indicates array lengths
3. **Zero-copy by default**: All core operations return references, not owned types
4. **Flexible Document Operations**: `Cow<str>` allows both zero-copy (`ZeroCopyGapBuffer`) and
   meaningful owned representations (`GCStringOwnedDoc`)
5. **Paradigm-Aware Mutations**: Associated types elegantly handle different mutation patterns
   (immutable vs mutable)
6. **Flexible String Returns**: The `StringSlice` associated type allows `GCStringOwned` to return
   `CowInlineString` for efficiency while `ZeroCopyGapBuffer` returns `&str` for zero-copy
   operations
7. **Type consistency**: Uses existing types (`ChUnit`, `ColIndex`, etc.) throughout
8. **Minimal abstractions**: Reuses existing `Seg` type instead of creating new traits
9. **Unified interface**: Same operations work on both `GCStringOwned` and `ZeroCopyGapBuffer`
10. **Migration friendly**: Existing code can gradually adopt traits without breaking changes
11. **Performance**: No allocations for common operations like getting segment content
12. **Extensibility**: Extension traits provide owned variants when explicitly needed

## Technical Challenges Addressed

1. **Semantic Clarity**:
   - ✅ **Solved**: Use `SegWidth` for grapheme segments, `Length` for array lengths
   - ✅ **Solved**: `segment_count()` clearly indicates what is being counted
   - ✅ **Solved**: Remove confusing `.width()` alias and `.len()` methods

2. **Document String Representation**:
   - ✅ **Solved**: `Cow<str>` allows both zero-copy and meaningful owned representations
   - ✅ **Solved**: `GCStringOwnedDoc::as_str()` returns meaningful `Cow::Owned` using `.join("\n")`

3. **Mutation Paradigm Differences**:
   - ✅ **Solved**: Associated `MutResult` type handles immutable (`GCStringOwned`) vs mutable
     (`ZeroCopyGapBuffer`) paradigms
   - ✅ **Solved**: `GraphemeStringMut` trait elegantly unifies different mutation approaches

4. **String Return Type Differences**:
   - ✅ **Solved**: Associated `StringSlice` type allows `GCStringOwned` to return `CowInlineString`
     while `ZeroCopyGapBuffer` returns `&str`
   - ✅ **Solved**: Both types implement `AsRef<str>` and `Display`, ensuring compatibility
   - ✅ **Solved**: No forced conversions or allocations needed

5. **Associated Type Complexity**:
   - ✅ **Mitigated**: Use descriptive names like `SegmentIterator`, `LineIterator`, `StringSlice`
   - ✅ **Clear patterns**: Associated types have clear, documented purposes and usage patterns

## Migration Strategy

### Phase 1: Add segment_count() method and trait implementations

```rust
impl GCStringOwned {
    /// Get the number of grapheme cluster segments.
    #[must_use]
    pub fn segment_count(&self) -> SegWidth {
        SegWidth::from(self.segments.len())
    }

    /// Get the number of grapheme clusters.
    ///
    /// **Deprecated**: Use `segment_count()` instead for semantic clarity.
    #[must_use]
    #[deprecated(since = "0.1.0", note = "Use `segment_count()` instead for semantic clarity")]
    pub fn len(&self) -> SegWidth {
        self.segment_count()
    }
}

impl<'a> GapBufferLine<'a> {
    /// Get the number of grapheme cluster segments.
    #[must_use]
    pub fn segment_count(&self) -> SegWidth {
        SegWidth::from(self.info.grapheme_count.as_usize())
    }
}
```

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

- Gradually update code to use trait methods instead of `.len()`
- Use `segment_count()` for explicit grapheme counting
- Leverage `Cow<str>` for document-wide operations
- Compiler warnings guide the migration

### Phase 4: Remove deprecated code

- Remove deprecated `.len()` methods and all other deprecated methods
- Remove editor-specific modules from `GCStringOwned`
- All functionality available through clear, semantic trait methods
- Remove all the code marked as deprecated in the previous phases

## Usage Examples

### Zero-copy segment access

```rust
let line: &dyn GraphemeString = &gc_string;
if let Some(seg_content) = line.get_seg_at(col(5)) {
    println!("Content: {}", seg_content.as_str());
    println!("Width: {}", seg_content.width());
    println!("Byte range: {:?}", seg_content.byte_range());
}

// ✅ Clear semantic intent with SegWidth
println!("Seg count: {}", line.segment_count());
```

### Working with associated types for string operations

```rust
// For GCStringOwned - returns CowInlineString
let gc_string = GCStringOwned::new("Hello 世界!");
let clipped: CowInlineString = gc_string.clip(col(0), width(5));
println!("Clipped: {}", clipped); // Uses Display trait

// For GapBufferLine - returns &str
let buffer = ZeroCopyGapBuffer::new();
if let Some(line) = buffer.get_line(row(0)) {
    let clipped: &str = line.clip(col(0), width(5));
    println!("Clipped: {}", clipped);
}

// Both work transparently through AsRef<str>
fn process_text<T: AsRef<str>>(text: T) {
    let s = text.as_ref();
    println!("Processing: {}", s);
}

// Works with both types
process_text(gc_string.clip(col(0), width(10)));
process_text(line.clip(col(0), width(10)));
```

### Working with documents

```rust
let doc: &dyn GraphemeDoc = &gap_buffer;
println!("Document has {} lines", doc.line_count());

// ✅ Cow<str> allows both borrowed and owned representations
let content = doc.as_str(); // Zero-copy for ZeroCopyGapBuffer, meaningful for GCStringOwnedDoc

for line in doc.iter_lines() {
    println!("Line has {} segments, width: {}",
             line.segment_count(),
             line.display_width());
}
```

### Mutation with associated types

```rust
// For GCStringOwned (returns new instances)
let mut gc_string = GCStringOwned::new("Hello");
if let Some(new_string) = gc_string.insert_text(col(5), " World") {
    // new_string is a new GCStringOwned instance
    gc_string = new_string;
}

// For ZeroCopyGapBuffer (in-place mutations)
let mut buffer = ZeroCopyGapBuffer::new();
buffer.add_line();
if let Ok(()) = buffer.insert_text_at_grapheme(row(0), seg(0), "Hello") {
    // Mutation happened in-place, buffer is modified
}
```

### When ownership is needed

```rust
use GraphemeStringOwnedExt;

let owned_seg = line.get_seg_owned_at(col(10));
let owned_copy = line.to_owned();
```

## File Structure and Implementation Plan

### New Folders and Files to Create

```
tui/src/core/graphemes/
├── traits/                                    # NEW - All trait definitions
│   ├── mod.rs                                # NEW - Module exports
│   ├── core_types.rs                         # NEW - SegContent struct
│   ├── single_line.rs                        # NEW - GraphemeString, GraphemeStringMut
│   ├── multi_line.rs                         # NEW - GraphemeDoc, GraphemeDocMut
│   └── extensions.rs                         # NEW - GraphemeStringOwnedExt
└── gc_string/
    └── document/                             # NEW - Document types
        ├── mod.rs                            # NEW - Module exports
        └── gc_string_owned_doc.rs            # NEW - GCStringOwnedDoc type
```

### Files to Modify

1. **Module Integration:**
   - `tui/src/core/graphemes/mod.rs` - Add `pub mod traits;` and re-exports
   - `tui/src/core/graphemes/gc_string/mod.rs` - Add `pub mod document;`

2. **Single-Line Implementations:**
   - `tui/src/core/graphemes/gc_string/owned/gc_string_owned.rs` - Add `segment_count()`, implement
     GraphemeString and GraphemeStringMut traits
   - `tui/src/tui/editor/zero_copy_gap_buffer/core/gap_buffer_line.rs` - Add `segment_count()`,
     implement GraphemeString trait

3. **Multi-Line Implementations:**
   - `tui/src/tui/editor/zero_copy_gap_buffer/core/zero_copy_gap_buffer.rs` - Implement GraphemeDoc
     and GraphemeDocMut traits

### Implementation Phases

1. **Phase 1 - Core Foundation:**
   - Create traits module structure
   - Define SegContent type and core trait definitions
   - Add segment_count() methods to existing types

2. **Phase 2 - Single-Line Traits:**
   - Implement GraphemeString for GCStringOwned
   - Implement GraphemeString for GapBufferLine
   - Implement GraphemeStringMut for GCStringOwned

3. **Phase 3 - Document Types:**
   - Create GCStringOwnedDoc in gc_string/document/
   - Implement GraphemeDoc for GCStringOwnedDoc
   - Implement GraphemeDoc for ZeroCopyGapBuffer

4. **Phase 4 - Document Mutations:**
   - Implement GraphemeDocMut for GCStringOwnedDoc
   - Implement GraphemeDocMut for ZeroCopyGapBuffer

5. **Phase 5 - Extensions and Polish:**
   - Add GraphemeStringOwnedExt extension trait
   - Add comprehensive tests

6. **Phase 6 - Migration Support:**
   - Add deprecation warnings to existing methods
   - Create migration adapters
   - Update documentation

This organization ensures:

- All grapheme-related code stays in `core/graphemes/`
- Clear separation between trait definitions and implementations
- Logical grouping of document types under `gc_string/document/`
- Intuitive imports: `use r3bl_tui::core::graphemes::traits::*;`
