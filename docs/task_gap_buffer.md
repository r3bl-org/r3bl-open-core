<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

- [Gap Buffer Implementation for Editor Content Storage](#gap-buffer-implementation-for-editor-content-storage)
  - [Detailed task tracking](#detailed-task-tracking)
    - [Phase 1: Core Infrastructure](#phase-1-core-infrastructure)
      - [1.1 Extract GCString Segment Logic](#11-extract-gcstring-segment-logic)
      - [1.2 Create LineBuffer Core Structure](#12-create-linebuffer-core-structure)
      - [1.3 Basic Buffer Operations](#13-basic-buffer-operations)
      - [1.4 Zero-Copy Access Methods](#14-zero-copy-access-methods)
    - [Phase 2: Text Operations](#phase-2-text-operations)
      - [2.1 Grapheme-Safe Insert Operations](#21-grapheme-safe-insert-operations)
      - [2.2 Grapheme-Safe Delete Operations](#22-grapheme-safe-delete-operations)
      - [2.3 Line Overflow Handling](#23-line-overflow-handling)
      - [2.4 Segment Rebuilding](#24-segment-rebuilding)
    - [Phase 3: Parser Integration](#phase-3-parser-integration)
      - [3.1 Parser Modifications for Padding](#31-parser-modifications-for-padding)
      - [3.2 Main Parser Entry Point](#32-main-parser-entry-point)
      - [3.3 Individual Parser Updates](#33-individual-parser-updates)
      - [3.4 Syntax Highlighting Integration](#34-syntax-highlighting-integration)
    - [Phase 4: Editor Integration](#phase-4-editor-integration)
      - [4.1 Replace VecEditorContentLines](#41-replace-veceditorcontentlines)
      - [4.2 Update Editor Operations](#42-update-editor-operations)
      - [4.3 Cursor Movement Updates](#43-cursor-movement-updates)
      - [4.4 File I/O Updates](#44-file-io-updates)
    - [Phase 5: Optimization](#phase-5-optimization)
      - [5.1 Memory Optimization](#51-memory-optimization)
      - [5.2 Performance Optimization](#52-performance-optimization)
      - [5.3 Advanced Features](#53-advanced-features)
      - [5.4 Tooling and Debugging](#54-tooling-and-debugging)
    - [Phase 6: Benchmarking and Profiling](#phase-6-benchmarking-and-profiling)
      - [6.1 Micro Benchmarks](#61-micro-benchmarks)
      - [6.2 Macro Benchmarks](#62-macro-benchmarks)
      - [6.3 Flamegraph Profiling](#63-flamegraph-profiling)
      - [6.4 Performance Analysis](#64-performance-analysis)
    - [Testing and Documentation](#testing-and-documentation)
      - [7.1 Unit Testing](#71-unit-testing)
      - [7.2 Integration Testing](#72-integration-testing)
      - [7.3 Documentation](#73-documentation)
  - [Overview](#overview)
  - [Summary of the Goal](#summary-of-the-goal)
    - [Core Problem](#core-problem)
    - [Proposed Solution](#proposed-solution)
    - [Benefits](#benefits)
    - [Required Changes](#required-changes)
  - [Current Architecture Analysis](#current-architecture-analysis)
    - [Existing Implementation](#existing-implementation)
    - [Performance Issue](#performance-issue)
  - [Proposed Gap Buffer Architecture](#proposed-gap-buffer-architecture)
    - [Core Data Structure](#core-data-structure)
    - [Key Design Decisions](#key-design-decisions)
  - [Implementation Details](#implementation-details)
    - [1. Buffer Operations](#1-buffer-operations)
    - [2. Unicode-Safe Text Manipulation](#2-unicode-safe-text-manipulation)
    - [3. Efficient Cursor Movement](#3-efficient-cursor-movement)
  - [GCString Refactoring Plan](#gcstring-refactoring-plan)
    - [Current GCString Analysis](#current-gcstring-analysis)
    - [Refactoring Steps](#refactoring-steps)
  - [Parser Modifications](#parser-modifications)
    - [EOL handling with newline followed by many null chars](#eol-handling-with-newline-followed-by-many-null-chars)
  - [Implementation Plan](#implementation-plan)
    - [Phase 1: Core Infrastructure](#phase-1-core-infrastructure-1)
    - [Phase 2: Text Operations](#phase-2-text-operations-1)
    - [Phase 3: Parser Integration](#phase-3-parser-integration-1)
    - [Phase 4: Editor Integration](#phase-4-editor-integration-1)
    - [Phase 5: Optimization](#phase-5-optimization-1)
  - [Benefits](#benefits-1)
  - [Challenges and Solutions](#challenges-and-solutions)
    - [Line Overflow (>256 chars)](#line-overflow-256-chars)
    - [UTF-8 Boundary Safety](#utf-8-boundary-safety)
    - [Parser Compatibility](#parser-compatibility)
  - [Testing Strategy](#testing-strategy)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

# Gap Buffer Implementation for Editor Content Storage

## Detailed task tracking

### Phase 1: Core Infrastructure

#### 1.1 Extract GCString Segment Logic

- [ ] Create new module `tui/src/core/graphemes/segment_builder.rs`
- [ ] Add module declaration in `tui/src/core/graphemes/mod.rs`
- [ ] Extract `build_segments_for_str()` function from GCString
- [ ] Extract ASCII fast path logic into `build_ascii_segments()`
- [ ] Extract `calculate_display_width()` function
- [ ] Add unit tests for segment building with various Unicode inputs
- [ ] Add benchmarks comparing ASCII vs Unicode segment building
- [ ] Make a commit with this progress

#### 1.2 Create LineBuffer Core Structure

- [ ] Create new module `tui/src/tui/editor/line_buffer/mod.rs`
- [ ] Define `LineBuffer` struct with basic fields
- [ ] Define `LineInfo` struct for metadata
- [ ] Implement `LineBuffer::new()` constructor
- [ ] Implement `LineBuffer::with_capacity()` for pre-allocation
- [ ] Add `const LINE_SIZE: usize = 256`
- [ ] Add debug/display traits for LineBuffer
- [ ] Make a commit with this progress

#### 1.3 Basic Buffer Operations

- [ ] Implement `add_line()` method
- [ ] Implement `remove_line()` method
- [ ] Implement `get_line_count()` method
- [ ] Implement `clear()` method to reset buffer
- [ ] Add bounds checking for line operations
- [ ] Add unit tests for basic operations
- [ ] Make a commit with this progress

#### 1.4 Zero-Copy Access Methods

- [ ] Implement `as_str()` -> `&str` for entire buffer
- [ ] Implement `as_bytes()` -> `&[u8]` for raw access
- [ ] Implement `get_line_content()` -> `&str` for single line
- [ ] Implement `get_line_slice()` for range of lines
- [ ] Add UTF-8 validation in debug builds
- [ ] Add tests for zero-copy access
- [ ] Make a commit with this progress

### Phase 2: Text Operations

#### 2.1 Grapheme-Safe Insert Operations

- [ ] Implement `insert_at_grapheme()` method
- [ ] Implement `insert_text_at_byte_pos()` helper
- [ ] Add byte position validation
- [ ] Implement content shifting logic
- [ ] Update newline marker position after insert
- [ ] Handle empty line insertion
- [ ] Add tests for various Unicode insertions
- [ ] Make a commit with this progress

#### 2.2 Grapheme-Safe Delete Operations

- [ ] Implement `delete_at_grapheme()` method
- [ ] Implement `delete_range()` for multiple graphemes
- [ ] Add content shifting for deletions
- [ ] Restore `\0` padding after delete
- [ ] Update line metadata after delete
- [ ] Handle edge cases (delete at line start/end)
- [ ] Add tests for Unicode-aware deletions
- [ ] Make a commit with this progress

#### 2.3 Line Overflow Handling

- [ ] Implement `handle_line_overflow()` method
- [ ] Design overflow strategy (panic vs reallocation)
- [ ] Add `can_insert()` method to check space
- [ ] Implement line splitting for overflow
- [ ] Add configuration for max line size
- [ ] Create tests for overflow scenarios
- [ ] Document overflow behavior
- [ ] Make a commit with this progress

#### 2.4 Segment Rebuilding

- [ ] Implement `rebuild_line_segments()` method
- [ ] Integrate with segment_builder module
- [ ] Update all LineInfo fields after rebuild
- [ ] Add lazy rebuilding flag option
- [ ] Implement batch segment rebuilding
- [ ] Add performance tests for rebuilding
- [ ] Make a commit with this progress

### Phase 3: Parser Integration

#### 3.1 Parser Modifications for Padding

- [ ] Create `parse_editor_line()` nom parser
- [ ] Handle `\n` followed by `\0` padding
- [ ] Extract content without padding
- [ ] Add tests for various padding scenarios
- [ ] Handle edge cases (no newline, all nulls)
- [ ] Make a commit with this progress

#### 3.2 Main Parser Entry Point

- [ ] Create `parse_markdown_with_padding()` function
- [ ] Integrate with existing parse_markdown
- [ ] Add compatibility layer for migration
- [ ] Test with real markdown documents
- [ ] Benchmark parsing performance
- [ ] Make a commit with this progress

#### 3.3 Individual Parser Updates

- [ ] Update `parse_heading_in_single_line`
- [ ] Update `parse_block_markdown_text`
- [ ] Update `parse_smart_list_block`
- [ ] Update `parse_fenced_code_block`
- [ ] Update metadata parsers
- [ ] Test each parser with padded input
- [ ] Make a commit with this progress

#### 3.4 Syntax Highlighting Integration

- [ ] Update `try_parse_and_highlight` function
- [ ] Remove ParserByteCache usage
- [ ] Use LineBuffer's zero-copy access
- [ ] Update highlight span calculations
- [ ] Test syntax highlighting accuracy
- [ ] Benchmark highlighting performance
- [ ] Make a commit with this progress

### Phase 4: Editor Integration

#### 4.1 Replace VecEditorContentLines

- [ ] Create type alias migration strategy
- [ ] Update EditorContent struct definition
- [ ] Migrate from SmallVec to LineBuffer
- [ ] Update all EditorContent methods
- [ ] Fix compilation errors
- [ ] Run existing editor tests
- [ ] Make a commit with this progress

#### 4.2 Update Editor Operations

- [ ] Update `insert_char` operation
- [ ] Update `delete_char` operation
- [ ] Update `insert_string` operation
- [ ] Update `split_line` operation
- [ ] Update `join_lines` operation
- [ ] Update clipboard operations (copy/paste)
- [ ] Update undo/redo to work with new structure
- [ ] Make a commit with this progress

#### 4.3 Cursor Movement Updates

- [ ] Update `move_cursor_left` to use segments
- [ ] Update `move_cursor_right` to use segments
- [ ] Update `move_cursor_up/down` for line nav
- [ ] Update word-based movement
- [ ] Update home/end key handling
- [ ] Cache cursor segment position
- [ ] Test cursor movement with Unicode
- [ ] Make a commit with this progress

#### 4.4 File I/O Updates

- [ ] Update file loading to populate LineBuffer
- [ ] Update file saving from LineBuffer
- [ ] Handle line ending conversions
- [ ] Preserve file encoding
- [ ] Test with various file formats
- [ ] Add progress reporting for large files
- [ ] Make a commit with this progress

### Phase 5: Optimization

#### 5.1 Memory Optimization

- [ ] Implement line pooling for deletions
- [ ] Add memory usage tracking
- [ ] Implement buffer compaction
- [ ] Add growth strategy configuration
- [ ] Profile memory usage patterns
- [ ] Document memory guarantees
- [ ] Make a commit with this progress

#### 5.2 Performance Optimization

- [ ] Add segment caching strategy
- [ ] Implement lazy segment rebuilding
- [ ] Optimize ASCII-only document handling
- [ ] Add SIMD optimizations for padding ops
- [ ] Cache line length calculations
- [ ] Profile and optimize hot paths
- [ ] Make a commit with this progress

#### 5.3 Advanced Features

- [ ] Implement line chaining for >256 chars
- [ ] Add configurable line size
- [ ] Implement view slicing for large docs
- [ ] Add incremental parsing support
- [ ] Implement parallel segment building
- [ ] Add memory-mapped file support
- [ ] Make a commit with this progress

#### 5.4 Tooling and Debugging

- [ ] Add buffer visualization tool
- [ ] Create memory layout debugger
- [ ] Add performance profiling hooks
- [ ] Create buffer integrity checker
- [ ] Add statistics collection
- [ ] Document performance characteristics
- [ ] Make a commit with this progress

### Phase 6: Benchmarking and Profiling

#### 6.1 Micro Benchmarks

- [ ] Create benchmark suite using `cargo bench`. Add these as plain tests with `#[bench]` attribute
      and co-locate them in the file with the source code under test.
- [ ] Benchmark single character insertion (ASCII vs Unicode)
- [ ] Benchmark string insertion (various sizes)
- [ ] Benchmark line deletion operations
- [ ] Benchmark cursor movement operations
- [ ] Benchmark segment building for different text types
- [ ] Compare LineBuffer vs VecEditorContentLines performance
- [ ] Benchmark memory allocation patterns
- [ ] Make a commit with this progress

#### 6.2 Macro Benchmarks

- [ ] Benchmark full document loading (various sizes)
- [ ] Benchmark syntax highlighting performance
- [ ] Benchmark parser performance with padding
- [ ] Benchmark editor responsiveness (keystroke to render)
- [ ] Benchmark memory usage for large documents
- [ ] Benchmark scrolling performance
- [ ] Create automated performance regression tests
- [ ] Make a commit with this progress

#### 6.3 Flamegraph Profiling

- [ ] Use existing `cargo flamegraph` infrastructure from the function
      `run_example_with_flamegraph_profiling_perf_fold` in `script_lib.nu`
- [ ] Profile editor during typical usage patterns using
      `run_example_with_flamegraph_profiling_perf_fold`
- [ ] Profile syntax highlighting hot paths
- [ ] Profile Unicode text handling
- [ ] Generate perf-folded format using `run_example_with_flamegraph_profiling_perf_fold`
- [ ] Create before/after flamegraphs for comparison
- [ ] Compare flamegraph.svg sizes and total sample counts
- [ ] Make a commit with this progress

#### 6.4 Performance Analysis

- [ ] Analyze cache miss patterns
- [ ] Profile branch prediction misses
- [ ] Measure memory bandwidth usage
- [ ] Analyze SIMD utilization opportunities
- [ ] Profile lock contention (if any)
- [ ] Create performance dashboard
- [ ] Set performance budgets/targets
- [ ] Make a commit with this progress

### Testing and Documentation

#### 7.1 Unit Testing

- [ ] Test each LineBuffer method
- [ ] Test Unicode edge cases
- [ ] Test buffer overflow scenarios
- [ ] Test parser with various inputs
- [ ] Test editor operations
- [ ] Add property-based tests
- [ ] Make a commit with this progress

#### 7.2 Integration Testing

- [ ] Test full editor workflow
- [ ] Test with real markdown files
- [ ] Test performance vs old implementation
- [ ] Test memory usage patterns
- [ ] Test with stress scenarios
- [ ] Add regression test suite
- [ ] Make a commit with this progress

#### 7.3 Documentation

- [ ] Document LineBuffer API
- [ ] Document migration guide
- [ ] Document performance characteristics
- [ ] Add code examples
- [ ] Update editor architecture docs
- [ ] Create troubleshooting guide
- [ ] Document benchmark results
- [ ] Make a commit with this progress

## Overview

This document outlines the strategy to replace the current `VecEditorContentLines` (vector of
`GCString`) with a gap buffer implementation that stores lines as fixed-size arrays padded with `\0`
characters. This approach enables zero-copy access as `&str` for the markdown parser while
maintaining efficient Unicode support.

This comes from the work done in the `md_parser_ng` crate which is archive that showed that a `&str`
parser is the fastest. So instead of bringing the mountain to Muhammad, we will bring Muhammad to
the mountain. The mountain is the `&str` parser, and Muhammad is the editor component.

## Summary of the Goal

The goal is to **optimize editor performance by eliminating string serialization** during markdown
parsing. Currently, the `EditorContent::lines: VecEditorContentLines` data structure stores lines as
`GCString` objects, but the markdown parser requires `&str` input, forcing expensive serialization.

### Core Problem

- Editor stores lines in `VecEditorContentLines` (array of `GCString`)
- Markdown parser needs `&str` input (nom parser constraint)
- Current solution serializes the entire data structure to `String` - this is inefficient

### Proposed Solution

Replace `VecEditorContentLines` with a **gap buffer-like data structure** where:

1. **Fixed-size line buffers**: Each line is pre-allocated as a 256-character array
2. **Null-padded storage**: Lines are padded with `\0` characters to fill unused space
3. **In-place editing**: Characters are inserted by overwriting `\0` bytes, avoiding reallocations
4. **Modified line termination**: Lines end with `\n` followed by `\0` padding instead of just `\n`

### Benefits

- **Zero-copy parsing**: The data can be accessed as `&str` directly without serialization
- **Reduced allocations**: Only reallocate when lines exceed 256 chars or lines are added/removed
- **Performance gains**: Especially beneficial for large documents (>1MB)

### Required Changes

- Modify the nom parser to handle `\n` + `\0` padding as line terminators
- Update editor component to work with the new data structure
- Implement gap buffer logic for efficient in-place editing

The approach prioritizes parser performance by adapting the editor's data structure rather than
changing the parser's `&str` requirement.

## Current Architecture Analysis

### Existing Implementation

1. **EditorContent struct** (`tui/src/tui/editor/editor_buffer/buffer_struct.rs`):
   - Contains `lines: VecEditorContentLines` field
   - Manages caret position, scroll offset, and file metadata

2. **VecEditorContentLines type** (`tui/src/tui/editor/editor_buffer/sizing.rs`):
   - Defined as: `SmallVec<[GCString; DEFAULT_EDITOR_LINES_SIZE]>`
   - Stack-allocated vector holding up to 32 lines before heap allocation

3. **GCString type** (`tui/src/core/graphemes/gc_string.rs`):
   - Contains `InlineString` (SmallString with 16-byte inline storage)
   - Stores grapheme cluster metadata in `SegmentArray`
   - Implements `AsRef<str>` for string conversion

4. **Current markdown parsing flow**
   (`tui/src/tui/syntax_highlighting/md_parser_syn_hi/md_parser_syn_hi_impl.rs`):
   - Takes `&[GCString]` as input
   - Materializes lines into a single `String` using `ParserByteCache`
   - Joins lines with newline characters
   - Passes materialized string to `parse_markdown(&str)`

### Performance Issue

The current approach requires allocating and copying all editor content into a new `String` every
time the markdown parser runs, which happens on every keystroke for syntax highlighting.

## Proposed Gap Buffer Architecture

### Core Data Structure

```rust
pub struct LineBuffer {
    // Contiguous buffer storing all lines
    // Each line is exactly LINE_SIZE bytes
    buffer: Vec<u8>,

    // Metadata for each line (grapheme clusters, display width, etc.)
    lines: Vec<LineInfo>,

    // Number of lines currently in the buffer
    line_count: usize,

    // Size of each line in bytes
    line_size: usize, // e.g., 256
}

pub struct LineInfo {
    // Where this line starts in the buffer
    buffer_offset: usize,

    // Actual content length in bytes (before '\n')
    content_len: usize,

    // GCString's segment array for this line
    segments: SegmentArray,  // SmallVec<[Seg; 28]>

    // Display width of the line
    display_width: ColWidth,

    // Number of grapheme clusters
    grapheme_count: usize,
}
```

### Key Design Decisions

1. **Fixed-size lines**: Each line allocated as 256-byte array
2. **Zero padding**: Unused bytes in each line filled with `\0`
3. **Line termination**: Content followed by `\n` then `\0` padding
4. **Metadata caching**: Store grapheme cluster info to avoid scanning
5. **Zero-copy access**: Entire buffer can be passed as `&str` to parser

## Implementation Details

### 1. Buffer Operations

```rust
impl LineBuffer {
    const LINE_SIZE: usize = 256;

    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            lines: Vec::new(),
            line_count: 0,
            line_size: Self::LINE_SIZE,
        }
    }

    // Add a new line to the buffer
    pub fn add_line(&mut self) -> usize {
        let line_index = self.line_count;
        let buffer_offset = line_index * Self::LINE_SIZE;

        // Extend buffer by LINE_SIZE bytes, all initialized to '\0'
        self.buffer.resize(self.buffer.len() + Self::LINE_SIZE, b'\0');

        // Add the newline character at the start (empty line)
        self.buffer[buffer_offset] = b'\n';

        // Create line metadata
        self.lines.push(LineInfo {
            buffer_offset,
            content_len: 0,
            segments: SegmentArray::new(),
            display_width: 0.into(),
            grapheme_count: 0,
        });

        self.line_count += 1;
        line_index
    }

    // Zero-copy access for the parser
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.buffer).unwrap()
    }
}
```

### 2. Unicode-Safe Text Manipulation

```rust
impl LineBuffer {
    // Insert text at a grapheme cluster boundary
    pub fn insert_at_grapheme(&mut self, line_index: usize, seg_index: SegIndex, text: &str) {
        let line_info = &self.lines[line_index];

        // Find byte position for the grapheme position
        let byte_pos = if seg_index.0 < line_info.segments.len() {
            line_info.segments[seg_index.0].start_byte_index.into()
        } else {
            line_info.content_len
        };

        // Insert at the correct byte boundary
        self.insert_text_at_byte_pos(line_index, byte_pos, text);

        // Rebuild segments for this line
        self.rebuild_line_segments(line_index);
    }

    fn insert_text_at_byte_pos(&mut self, line_index: usize, byte_position: usize, text: &str) {
        let line_info = &self.lines[line_index];
        let line_start = line_info.buffer_offset;
        let text_bytes = text.as_bytes();

        // Check if we have space
        if line_info.content_len + text_bytes.len() >= Self::LINE_SIZE - 1 {
            // Handle line overflow (discussed below)
            self.handle_line_overflow(line_index);
        }

        // Shift existing content to make room
        let insert_pos = line_start + byte_position;
        let content_end = line_start + line_info.content_len;

        // Move existing content right
        for i in (insert_pos..content_end).rev() {
            self.buffer[i + text_bytes.len()] = self.buffer[i];
        }

        // Insert new text
        self.buffer[insert_pos..insert_pos + text_bytes.len()]
            .copy_from_slice(text_bytes);

        // Update newline position
        self.buffer[content_end + text_bytes.len()] = b'\n';

        // Update metadata
        self.lines[line_index].content_len += text_bytes.len();
    }

    // Rebuild grapheme cluster segments after modification
    fn rebuild_line_segments(&mut self, line_index: usize) {
        let line_info = &self.lines[line_index];
        let content = self.get_line_content(line_index);

        // Use extracted GCString logic
        let segments = build_segments_for_str(content);
        let display_width = calculate_display_width(&segments);
        let grapheme_count = segments.len();

        let line_info = &mut self.lines[line_index];
        line_info.segments = segments;
        line_info.display_width = display_width;
        line_info.grapheme_count = grapheme_count;
    }
}
```

### 3. Efficient Cursor Movement

```rust
impl LineBuffer {
    // Move cursor by grapheme clusters without scanning
    pub fn move_cursor_right(&self, line_index: usize, current_seg: SegIndex) -> Option<SegIndex> {
        let line_info = &self.lines[line_index];

        if current_seg.0 + 1 < line_info.segments.len() {
            Some(SegIndex(current_seg.0 + 1))
        } else {
            None
        }
    }

    // Get byte position for a grapheme cluster
    pub fn get_grapheme_byte_pos(&self, line_index: usize, seg_index: SegIndex) -> usize {
        let line_info = &self.lines[line_index];
        let seg = &line_info.segments[seg_index.0];
        seg.start_byte_index.into()
    }

    // Get display column for a grapheme cluster
    pub fn get_grapheme_display_col(&self, line_index: usize, seg_index: SegIndex) -> ColIndex {
        let line_info = &self.lines[line_index];
        let seg = &line_info.segments[seg_index.0];
        seg.start_display_col_index
    }
}
```

## GCString Refactoring Plan

### Current GCString Analysis

1. **What's Reusable**:
   - `Seg` struct (already decoupled, contains only indices)
   - Width calculation functions (static methods)
   - Segmentation algorithm logic

2. **What Needs Extraction**:
   - Grapheme segmentation logic from `GCString::new()`
   - ASCII fast path optimization
   - Segment building algorithm

### Refactoring Steps

1. **Create Segment Builder Module**:

```rust
// New module: tui/src/core/graphemes/segment_builder.rs

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// Build grapheme cluster segments for any string slice
pub fn build_segments_for_str(input: &str) -> SegmentArray {
    // ASCII fast path
    if input.is_ascii() {
        return build_ascii_segments(input);
    }

    let mut segments = SegmentArray::new();
    let mut byte_offset = 0;
    let mut display_col = 0;

    for (seg_index, grapheme) in input.graphemes(true).enumerate() {
        let bytes_size = grapheme.len();
        let display_width = UnicodeWidthStr::width(grapheme);

        segments.push(Seg {
            start_byte_index: byte_offset.into(),
            end_byte_index: (byte_offset + bytes_size).into(),
            display_width: display_width.into(),
            seg_index: seg_index.into(),
            bytes_size: bytes_size.into(),
            start_display_col_index: display_col.into(),
        });

        byte_offset += bytes_size;
        display_col += display_width;
    }

    segments
}

fn build_ascii_segments(input: &str) -> SegmentArray {
    let mut segments = SegmentArray::with_capacity(input.len());

    for (i, _) in input.char_indices() {
        segments.push(Seg {
            start_byte_index: i.into(),
            end_byte_index: (i + 1).into(),
            display_width: 1.into(),
            seg_index: i.into(),
            bytes_size: 1.into(),
            start_display_col_index: i.into(),
        });
    }

    segments
}

/// Calculate total display width from segments
pub fn calculate_display_width(segments: &SegmentArray) -> ColWidth {
    segments.last()
        .map(|seg| seg.start_display_col_index + seg.display_width)
        .unwrap_or(0.into())
}
```

2. **Modify GCString to Use Extracted Functions**:

```rust
impl GCString {
    pub fn new(string: String) -> Self {
        let segments = build_segments_for_str(&string);
        let display_width = calculate_display_width(&segments);
        let bytes_size = string.len();

        Self {
            string: string.into(),
            segments,
            display_width,
            bytes_size: bytes_size.into(),
        }
    }
}
```

## Parser Modifications

### EOL handling with newline followed by many null chars

Handling '\n' + many '\0' padding per line.

```rust
// Modified parser to handle the new line format
use nom::{
    bytes::complete::take_while,
    character::complete::char,
    combinator::recognize,
    sequence::tuple,
    Parser,
    IResult,
};

/// Parse a line that ends with '\n' followed by '\0' padding
fn parse_editor_line(input: &str) -> IResult<&str, &str> {
    let (remaining, matched) = recognize(
        tuple((
            take_while(|c| c != '\n' && c != '\0'),  // Line content
            char('\n'),                               // Required newline
            take_while(|c| c == '\0'),               // Zero or more null padding
        ))
    ).parse(input)?;

    // Extract just the content part (before '\n')
    let content_end = matched.find('\n').unwrap_or(matched.len());
    let content = &matched[..content_end];

    Ok((remaining, content))
}

/// Modified markdown parser entry point
pub fn parse_markdown_with_padding(input: &str) -> IResult<&str, MdDocument<'_>> {
    // The input now contains '\0' padding, but we can still parse it directly
    // because our line parsers will handle the padding

    // For block parsers that need clean lines, we can pre-process:
    let lines: Vec<&str> = input
        .split('\n')
        .map(|line| line.trim_end_matches('\0'))
        .collect();

    // Or modify individual parsers to handle padding
    parse_markdown(input)
}
```

## Implementation Plan

### Phase 1: Core Infrastructure

1. Create `segment_builder.rs` module with extracted GCString logic
2. Implement basic `LineBuffer` struct with buffer management
3. Add `LineInfo` struct for metadata tracking
4. Implement zero-copy `as_str()` method

### Phase 2: Text Operations

1. Implement Unicode-safe insert operations
2. Implement Unicode-safe delete operations
3. Add line overflow handling
4. Implement segment rebuilding after modifications

### Phase 3: Parser Integration

1. Modify markdown parser to handle '\0' padding
2. Update syntax highlighting to use new buffer
3. Test with various Unicode content (emoji, CJK, etc.)

### Phase 4: Editor Integration

1. Replace `VecEditorContentLines` with `LineBuffer`
2. Update editor operations to use new API
3. Update cursor movement to use cached segments
4. Performance testing and optimization

### Phase 5: Optimization

1. Implement line pooling for deleted lines
2. Add lazy segment rebuilding
3. Optimize for common cases (ASCII text)
4. Memory usage profiling

## Benefits

1. **Zero-copy parsing**: No string materialization needed
2. **Predictable memory**: Fixed-size line allocations
3. **Fast edits**: No reallocation for typical line edits
4. **Unicode correctness**: Leverages proven GCString logic
5. **Cache efficiency**: Sequential memory layout

## Challenges and Solutions

### Line Overflow (>256 chars)

- **Solution**: Implement line chaining or dynamic reallocation
- For now, can panic and handle in Phase 5

### UTF-8 Boundary Safety

- **Solution**: Always use grapheme-aware operations
- Never split bytes manually

### Parser Compatibility

- **Solution**: Gradual migration with compatibility layer
- Both old and new parsers can coexist during transition

## Testing Strategy

1. **Unit tests**: Each buffer operation
2. **Unicode tests**: Emoji, combining chars, wide chars
3. **Parser tests**: Various markdown documents
4. **Performance benchmarks**: Compare with current implementation
5. **Stress tests**: Large documents, rapid edits
