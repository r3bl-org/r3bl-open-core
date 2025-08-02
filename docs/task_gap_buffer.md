<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

- [Gap Buffer Implementation for Editor Content Storage](#gap-buffer-implementation-for-editor-content-storage)
  - [Detailed task tracking](#detailed-task-tracking)
    - [✅ Phase 1: Core Infrastructure](#-phase-1-core-infrastructure)
      - [✅ 1.1 Extract GCString Segment Logic](#-11-extract-gcstring-segment-logic)
      - [✅ 1.2 Create ZeroCopyGapBuffer Core Structure](#-12-create-zerocopygapbuffer-core-structure)
      - [✅ 1.3 Basic Buffer Operations](#-13-basic-buffer-operations)
      - [✅ 1.4 Zero-Copy Access Methods](#-14-zero-copy-access-methods)
      - [✅ 1.5 Use newtype and dynamic line sizing](#-15-use-newtype-and-dynamic-line-sizing)
    - [✅ Phase 2: Text Operations](#-phase-2-text-operations)
      - [✅ 2.1 Grapheme-Safe Insert Operations](#-21-grapheme-safe-insert-operations)
      - [✅ 2.2 Grapheme-Safe Delete Operations](#-22-grapheme-safe-delete-operations)
      - [✅ 2.3 Segment Rebuilding](#-23-segment-rebuilding)
      - [✅ 2.4 Validation and benchmarking](#-24-validation-and-benchmarking)
      - [✅ 2.5 Optimize Segment Rebuild for common editor use case](#-25-optimize-segment-rebuild-for-common-editor-use-case)
    - [✅ Phase 3: Parser Integration](#-phase-3-parser-integration)
      - [✅ 3.1 Parser Modifications for Padding](#-31-parser-modifications-for-padding)
      - [✅ 3.2 Main Parser Entry Point](#-32-main-parser-entry-point)
      - [✅ 3.3 Individual Parser Updates](#-33-individual-parser-updates)
      - [✅ 3.4 VecEditorContentLines Adapter](#-34-veceditorcontentlines-adapter)
      - [✅ 3.5 ZeroCopyGapBuffer and parse_markdown() Integration](#-35-zerocopygapbuffer-and-parse_markdown-integration)
      - [✅ 3.6 Syntax Highlighting Integration - Stepping Stone Approach](#-36-syntax-highlighting-integration---stepping-stone-approach)
    - [Phase 4: Editor Integration](#phase-4-editor-integration)
      - [✅ 4.1 EditorLinesStorage Trait](#-41-editorlinesstorage-trait)
      - [4.2 Direct Migration to EditorLinesStorage](#42-direct-migration-to-editorlinesstorage)
        - [4.2.1 GCString and GapBufferLineInfo Integration Strategy](#421-gcstring-and-gapbufferlineinfo-integration-strategy)
        - [4.2.2 Implementation Approach: Hybrid Strategy](#422-implementation-approach-hybrid-strategy)
        - [4.2.3 Core Structure Changes](#423-core-structure-changes)
        - [4.2.4 Migration of Direct Line Access Patterns](#424-migration-of-direct-line-access-patterns)
        - [4.2.5 Benefits of This Approach](#425-benefits-of-this-approach)
        - [4.2.6 Migration Tasks](#426-migration-tasks)
    - [Phase 5: Optimization](#phase-5-optimization)
      - [5.1 Benchmark current implementation](#51-benchmark-current-implementation)
      - [5.2 Memory Optimization](#52-memory-optimization)
      - [5.3 Performance Optimization](#53-performance-optimization)
      - [5.4 Advanced Features](#54-advanced-features)
      - [5.5 Tooling and Debugging](#55-tooling-and-debugging)
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
    - [Invariant](#invariant)
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
    - [Phase 1: Core Infrastructure](#phase-1-core-infrastructure)
    - [Phase 2: Text Operations](#phase-2-text-operations)
    - [Phase 3: Parser Integration](#phase-3-parser-integration)
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

For benchmarks don't use `criterion`. Use `cargo bench`, and add bench tests are regular tests in
the file with the code under test, and mark them with `#[bench]`. This project uses nightly rust
toolchain, and is already configured to support cargo bench.

### ✅ Phase 1: Core Infrastructure

#### ✅ 1.1 Extract GCString Segment Logic

- [x] Update documentation to articulate why three types of index are needed: display, col, and
      segment
- [x] Create new module `tui/src/core/graphemes/segment_builder.rs`
- [x] Add module declaration in `tui/src/core/graphemes/mod.rs`
- [x] Extract `build_segments_for_str()` function from GCString
- [x] Extract ASCII fast path logic into `build_ascii_segments()`
- [x] Extract `calculate_display_width()` function
- [x] Add unit tests for segment building with various Unicode inputs
- [x] Add benchmarks comparing ASCII vs Unicode segment building
  - ASCII short (13 chars): ~54ns
  - ASCII long (240 chars): ~287ns
  - Unicode with emojis: ~592ns
  - Unicode mixed (accents, CJK): ~666ns
  - Unicode complex (skin tones): ~812ns
- [x] Make a commit with this progress

#### ✅ 1.2 Create ZeroCopyGapBuffer Core Structure

- [x] Create new module `tui/src/tui/editor/zero_copy_gap_buffer/mod.rs`
- [x] Define `ZeroCopyGapBuffer` struct with basic fields
- [x] Define `GapBufferLineInfo` struct for metadata
- [x] Implement `ZeroCopyGapBuffer::new()` constructor
- [x] Implement `ZeroCopyGapBuffer::with_capacity()` for pre-allocation
- [x] Add `const LINE_SIZE: usize = 256`
- [x] Add debug/display traits for ZeroCopyGapBuffer
- [x] Make a commit with this progress

#### ✅ 1.3 Basic Buffer Operations

- [x] Implement `add_line()` method
- [x] Implement `remove_line()` method
- [x] Implement `get_line_count()` method
- [x] Implement `clear()` method to reset buffer
- [x] Add bounds checking for line operations
- [x] Add unit tests for basic operations
- [x] Make a commit with this progress

#### ✅ 1.4 Zero-Copy Access Methods

- [x] Implement `as_str()` -> `&str` for entire buffer
  - Uses `unsafe` for zero-copy guarantee with comprehensive safety documentation
  - Debug builds validate UTF-8 before conversion
- [x] Implement `as_bytes()` -> `&[u8]` for raw access
  - Direct access to underlying buffer
- [x] Implement `get_line_content()` -> `&str` for single line
  - Returns content without null padding or newline
- [x] Implement `get_line_slice()` for range of lines
  - Returns lines including null padding
- [x] Add UTF-8 validation in debug builds
  - Panics on invalid UTF-8 in debug mode
- [x] Add tests for zero-copy access
  - Including tests for invalid UTF-8 handling
- [x] Additional methods for parser support:
  - `get_line_with_newline()` - Get line including newline character
  - `find_line_containing_byte()` - Map byte offset to line number
  - `get_line_raw()` - Get raw line bytes for debugging
  - `is_valid_utf8()` - Check buffer UTF-8 validity
- [x] Created separate `zero_copy_access.rs` module
- [x] Made buffer field public for direct access when needed
- [x] Make a commit with this progress

#### ✅ 1.5 Use newtype and dynamic line sizing

- [x] Instead of `usize`, use specific types like `ByteIndex`, `Length`, `SegIndex`, and `ColIndex`
- [x] Implement dynamic line resizing, start with `INITIAL_LINE_SIZE`, extend by `LINE_PAGE_SIZE`
- [x] Move `ByteIndex` from `graphemes` to `units` mod, since it is generic (not domain specific)

### ✅ Phase 2: Text Operations

#### ✅ 2.1 Grapheme-Safe Insert Operations

- [x] Implement `insert_at_grapheme()` method
- [x] Implement `insert_text_at_byte_pos()` helper
- [x] Add byte position validation
- [x] Implement content shifting logic
- [x] Update newline marker position after insert
- [x] Handle empty line insertion
- [x] Ensure that `\0` (null) padding preservation
- [x] Add tests for various Unicode insertions
- [x] Make a commit with this progress

#### ✅ 2.2 Grapheme-Safe Delete Operations

- [x] Implement `delete_at_grapheme()` method in `text_deletion.rs`
- [x] Implement `delete_range()` for multiple graphemes
- [x] Add content shifting for deletions
- [x] Restore `\0` padding after delete
- [x] Update line metadata after delete
- [x] Handle edge cases (delete at line start/end)
- [x] Add tests for Unicode-aware deletions
- [x] Ensure that `\0` (null) padding preservation
- [x] Make sure that all docs in module are up to date with the latest changes added here
- [x] Make a commit with this progress

#### ✅ 2.3 Segment Rebuilding

- [x] Create `segment_construction.rs` file in `zero_copy_gap_buffer` module
- [x] Implement `rebuild_line_segments()` method in `segment_construction.rs`
  - Get line content as UTF-8 string (validate UTF-8 in debug mode)
  - Use `build_segments_for_str` from segment_builder module
  - Calculate display width with `calculate_display_width`
  - Update all GapBufferLineInfo fields (segments, display_width, grapheme_count)
- [x] Implement batch segment rebuilding with `rebuild_line_segments_batch()`
  - Iterate through multiple lines and rebuild segments
  - Useful for bulk operations like file loading or large pastes
- [x] Integrate with segment_builder module from `core::graphemes`
- [x] Consolidate duplicate implementations from text_insertion.rs and text_deletion.rs
  - Replace both implementations with calls to the new centralized version
- [x] Add content boundary correctness tests
  - Ensure we only read content up to `content_len` (not into null padding)
  - Test that segment calculations exclude null bytes
  - Verify correct metadata updates (segments, display_width, grapheme_count)
- [x] Add UTF-8 safety architecture with debug-mode validation
  - Uses `unsafe { from_utf8_unchecked() }` for performance in release builds
  - Debug mode validates UTF-8 with clear panic messages
  - Architectural contract ensures only valid UTF-8 enters the buffer
- [x] Add unit tests for single line and batch rebuilding
  - Test with various Unicode content (ASCII, emoji, combining characters)
  - Verify off-by-one errors don't occur in content extraction
- [x] Add performance benchmarks using `#[bench]` attribute
  - Establish baseline performance for segment rebuilding operations
- [x] Make sure that all docs in module are up to date with the latest changes added here
  - Added comprehensive documentation about UTF-8 safety architecture
  - Updated use case descriptions for both single-line and batch operations
- [x] Make a commit with this progress

#### ✅ 2.4 Validation and benchmarking

- [x] Make sure that all the tests check for the "Null-Padding Invariant" in each file in
      `zero_copy_gap_buffer` mod
- [x] Make sure all the docs are up to date with the implementations in `zero_copy_gap_buffer` mod
- [x] Add benchmarking tests in each of the files in this `zero_copy_gap_buffer` mod to record a
      baseline of performance for CRUD operations. Add bench tests as regular tests in the files
      that contain the source under test using `#[bench]` attribute that can be run with
      `cargo bench`
- [x] Document the results from running all the benchmarks in the `zero_copy_gap_buffer/mod.rs`
      module level rustdoc comments
- [x] Run `cargo clippy --all-targets` and fix all the lint warnings generated by this tool
- [x] Ask the user to deeply review this code, when they have made their changes, then make a commit
      with this progress

#### ✅ 2.5 Optimize Segment Rebuild for common editor use case

- [x] Make end of line optimization for `segment_construction.rs` to speed up common use case in
      editor of typing characters at the end of the line
- [x] Document the results from running all the benchmarks in the `zero_copy_gap_buffer/mod.rs`
      module level rustdoc comments
- [x] Run `cargo clippy --all-targets` and fix all the lint warnings generated by this tool
- [x] Ask the user to deeply review this code, when they have made their changes, then make a commit
      with this progress

### ✅ Phase 3: Parser Integration

**The parser now takes `&ZeroCopyGapBuffer` as input for type safety.** We will change the signature
to `pub fn parse_markdown(input: &ZeroCopyGapBuffer) -> IResult<&str, MdDocument<'_>>` which
enforces at compile-time that only `ZeroCopyGapBuffer` can be used. Internally, it calls
`input.as_str()` for zero-copy access. The parser handles `\0` (null) characters that appear as line
padding.

#### ✅ 3.1 Parser Modifications for Padding

- [x] Add NULL constants to `md_parser_types.rs`: `NULL_CHAR`, `NULL_STR`, `NEWLINE_OR_NULL`
- [x] Create `parse_null_padded_line.rs` with `is()` and `is_any_of()` helper functions
- [x] Implement `parse_null_padded_line()` nom parser using idiomatic patterns
- [x] Add comprehensive tests for various padding scenarios
- [ ] Make sure that all docs in module are up to date with the latest changes added here
- [ ] Make a commit with this progress

#### ✅ 3.2 Main Parser Entry Point

- [x] Change `parse_markdown` signature to take `&ZeroCopyGapBuffer` parameter
- [x] Rename existing function to `parse_markdown_str` for internal use
- [x] Update module exports and documentation
- [x] Update all test files to use `parse_markdown_str`
- [x] Update syntax highlighting to use `parse_markdown_str`
- [ ] Test with real markdown documents containing null padding
- [ ] Benchmark parsing performance with null-padded vs clean input
- [ ] Make sure that all docs in module are up to date with the latest changes added here
- [x] Make a commit with this progress

#### ✅ 3.3 Individual Parser Updates

- [x] Update `parse_heading_in_single_line` to use `is_any_of()` for null handling
  - Modified `parse_anychar_in_heading_no_new_line` to handle null chars
  - Added `NULL_STR` to terminator tags in heading parser
  - Added comprehensive test `test_parse_header_with_null_padding`
- [x] Update `plain_parser_catch_all` to use `is_any_of(&[NEW_LINE_CHAR, NULL_CHAR])`
  - Refactored to use idiomatic `is_any_of()` helper function
  - Updated `get_sp_char_set_3()` to include `NULL_STR`
  - Added test `test_parse_plain_text_with_null_padding`
- [x] Update `parse_fenced_code_block` to use `is_not(NEWLINE_OR_NULL)`
  - Changed language tag parsing to stop at null chars
  - Added test `test_parse_codeblock_with_null_padding`
- [x] Update `parse_smart_list_block` to handle null padding
  - Updated content parsing to use `is_not(NEWLINE_OR_NULL)`
  - Added test `test_parse_smart_list_with_null_padding`
- [x] Test each parser with null-padded input strings
- [x] Make a commit with this progress

#### ✅ 3.4 VecEditorContentLines Adapter

- [x] Create `vec_to_gap_buffer_adapter.rs` with `convert_vec_lines_to_gap_buffer()` function
  - Created adapter module in `tui/src/tui/md_parser/vec_to_gap_buffer_adapter.rs`
  - Implemented conversion function that:
    - Takes `&[GCString]` as input
    - Creates a new `ZeroCopyGapBuffer`
    - Adds lines and inserts text using `insert_at_grapheme` API
    - Returns properly formatted buffer with null padding
- [x] Add module declarations and exports
  - Added to `md_parser/mod.rs`
  - Re-exported from main parser module
- [x] Add tests for the adapter function
  - `test_convert_empty_lines` - verifies empty input handling
  - `test_convert_single_line` - checks single line conversion
  - `test_convert_multiple_lines` - tests multi-line content
  - `test_convert_with_unicode` - validates Unicode preservation
  - `test_convert_code_block` - ensures complex markdown structures work
- [x] Fixed API usage issues:
  - Discovered `insert_text_at_byte_pos` is private
  - Updated to use public `insert_at_grapheme` API with `SegIndex::from(0)`
  - Fixed type conversions for `RowIndex`

#### ✅ 3.5 ZeroCopyGapBuffer and parse_markdown() Integration

- [x] ByteIndex to SegIndex conversion
  - [x] Forward conversion: `GapBufferLineInfo::get_byte_pos(SegIndex) -> ByteIndex`
  - [x] Reverse conversion: `GapBufferLineInfo::get_seg_index(ByteIndex) -> SegIndex`
- [x] Add more adapters to convert into ZeroCopyGapBuffer:
  - [x] Rename `vec_to_gap_buffer_adapter.rs` to `gap_buffer_adapters.rs`
  - [x] Add `convert_str_to_gap_buffer()`. This is for legacy tests that load string content
        directly using `include_str!` from files in `conformance_test_data` folder
  - [x] We should have support for converting all legacy test data into `ZeroCopyGapBuffer` format,
        so that we can run all the parse_markdown tests with the new gap buffer
  - [x] Implemented `gap_buffer_adapters.rs` module with:
    - `convert_vec_lines_to_gap_buffer()` - converts `&[GCString]` to `ZeroCopyGapBuffer`
    - `convert_str_to_gap_buffer()` - converts `&str` to `ZeroCopyGapBuffer`
    - Both functions properly handle newlines, empty lines, and null padding
    - Comprehensive tests for both adapters including Unicode content
- [x] Update `try_parse_and_highlight` in `md_parser_syn_hi_impl.rs`:
  - [x] Import `convert_vec_lines_to_gap_buffer` and `parse_markdown`
  - [x] Convert `editor_text_lines` to `ZeroCopyGapBuffer` using the adapter
  - [x] Pass gap buffer directly to `parse_markdown(&gap_buffer)`
  - [x] Remove ParserByteCache parameter entirely (breaking change)
  - [x] Update all callers to remove `parser_byte_cache` parameter
  - [x] Fix unused imports (PARSER_BYTE_CACHE_PAGE_SIZE, ParserByteCache, etc.)
  - [x] Update documentation to mark parser_byte_cache as deprecated
  - [x] Verify code compiles without warnings
  - [x] Converts &[GCString] to ZeroCopyGapBuffer using convert_vec_lines_to_gap_buffer()
  - [x] Passes the gap buffer directly to parse_markdown()
  - [x] Removed ParserByteCache parameter entirely from function signature
  - [x] All callers updated to remove parser_byte_cache parameter (engine_public_api.rs)
  - [x] Fixed structural issues in code (duplicate impl blocks)
  - [x] All tests in md_parser_syn_hi module pass (16 tests)
  - [x] Code compiles cleanly without warnings
- [x] Break compatibility with `VecEditorContentLines` (this is intentional and OK)
- [x] Remove `ParserByteCache` entirely (no fallback needed)

#### ✅ 3.6 Syntax Highlighting Integration - Stepping Stone Approach

We are in the middle of migrating `parse_markdown()` to use `ZeroCopyGapBuffer` as input, instead of
`&str` that is loaded from `VecEditorContentLines`. This is a stepping stone towards full zero-copy
integration with the parser. We're not trying to achieve full zero-copy benefits yet, just proving
that the conversion pipeline works.

What is "null padding" invariant?

- In the existing `parse_markdown` module, the parser expects that each line ends with a newline
  character (`\n`), and that there is no null padding after the newline. This is the legacy
  behavior.
- With `ZeroCopyGapBuffer` we introduce the new concept that "EOL" is not just `\n`, but also [0 ..
  more] `\0` (null) characters, ie, the `\n` is always followed by a "null padding". This is the
  "null padding invariant" that is enforced by `ZeroCopyGapBuffer` due to the way in which it stores
  unicode UTF-8 characters in its internal buffer.

What is the goal of this phase?

- The parser and sub-parser functions need to be changed to work with this "null padding" invariant
  introduced and enforced by `ZeroCopyGapBuffer`.
- `ZeroCopyGapBuffer` is the only type that can be used as input to `parse_markdown()`.
- `ZeroCopyGapBuffer::as_str()` returns a `&str` that upholds the "null padding invariant", meaning
  it will always return a string that has null padding after each line.

To better understand how to handle "null padding" in the parser:

- Review `parse_null_padded_line.rs` to see how to handle null padding invariant, and working with
  nom and markdown parser functions `is_not()`, `is()`, `is_any_of()` that work with `take_while()`,
  `take_till1()` .
- Don't use `\0`, `\n`, ` `, etc hard-coded characters in the parser functions, instead use
  `NULL_CHAR`, `NEWLINE_OR_NULL`, `NEW_LINE_CHAR` constants defined in `md_parser_types.rs`.

How to use existing tests:

- The current tests in `parse_markdown` module are correct.
- The code in the `markdown_parser()` and its sub-parser functions in the module, have to be updated
  to handle null padding correctly.
- Do not change the expected output of the tests, they are correct. They should not have null
  padding in the output, but the input to the parser should be a `ZeroCopyGapBuffer` that has null
  padding after each line. The parser should handle this null padding correctly.

Our key goals are:

- Prove the pipeline works: `VecEditorContentLines` → `ZeroCopyGapBuffer` → `parse_markdown()`
- We must guarantee that the `parse_markdown()` function works with the `ZeroCopyGapBuffer` without
  any issues, including null padding handling. Where necessary, we can take legacy content that
  would be converted into `VecEditorContentLines` and convert it to `ZeroCopyGapBuffer` using
  `convert_vec_lines_to_gap_buffer()`
- Convert `VecEditorContentLines` to `ZeroCopyGapBuffer` when needed:
  - The editor creates a `VecEditorContentLines`,
  - Which we then convert to `ZeroCopyGapBuffer` (we don't care about performance or efficiency in
    this phase),
  - Then we pass this `ZeroCopyGapBuffer` to `parse_markdown()`, which internally uses
    `ZeroCopyGapBuffer::as_str()` to call `parse_markdown_str()`.

Tasks:

- [x] Ensure that `parse_markdown()` works with the `zero_copy_gap_buffer` module and
      `ZeroCopyGapBuffer`.
  - [x] Verify that it handles null padding correctly - all parser tests include null padding tests
  - [x] Ensure that all parser tests pass with the new gap buffer input - 222 md_parser tests pass
    - For test data that is loaded using `include_str!` from `conformance_test_data` folder, we can
      use `gap_buffer_from_str()` to convert the string content into `ZeroCopyGapBuffer`
    - For test data that is loaded using `&[GCString]` aka `VecEditorContentLines`, we can use
      `gap_buffer_from_lines()` to convert the content into `ZeroCopyGapBuffer`
  - [x] Make `parse_markdown_str` private and document it as an internal function (should not be
        used in any public API or rustdoc comments). This is to ensure that the Rust type system
        enforces that only `ZeroCopyGapBuffer` can be used as input to `parse_markdown()`. This is
        the only way we can guarantee that the `&str` used in `parse_markdown()` is always derived
        from `ZeroCopyGapBuffer::as_str()`, which is guaranteed to uphold the null padding
        invariant.
  - [x] Replace calls to it with `parse_markdown` in the codebase and tests, use the appropriate
        adapter to convert `&str` or `&[GCString]` to `ZeroCopyGapBuffer` before passing it to
        `parse_markdown`
  - [x] Renamed adapter functions to more idiomatic names:
    - `convert_vec_lines_to_gap_buffer` → `gap_buffer_from_lines`
    - `convert_str_to_gap_buffer` → `gap_buffer_from_str`
  - [x] Implemented `From<&str>` and `From<&[GCString]>` traits for `ZeroCopyGapBuffer`
  - [x] Updated all usages to use the From trait instead of calling adapter functions directly
- [x] Add tests to verify:
  - [x] The conversion from VecEditorContentLines works correctly
  - [x] The gap buffer can be parsed despite null padding
  - [x] Syntax highlighting works with the gap buffer approach
- [x] Run `cargo clippy --all-targets` and fix all the lint warnings
- [x] Make sure that all docs in module are up to date with the latest changes
- [x] Make gap*buffer_from*\* functions private and use From trait everywhere
- [x] Replace hardcoded characters with constants from md_parser_types.rs
- [x] Ensure idiomatic nom usage with is_not(), is(), is_any_of()
- [x] Add proper null padding documentation to all parser functions
- [x] Make a commit with this progress

**Note:** We are committed to moving everything over to use ZeroCopyGapBuffer, but we are doing this
one step at a time. VecEditorContentLines will be abandoned in Phase 4.

### Phase 4: Editor Integration

Currently the editor uses `VecEditorContentLines` as the main content storage, which is a legacy
implementation that does not support zero-copy access and has performance issues with large files.
`VecEditorContentLines` is transformed into `ZeroCopyGapBuffer` for parsing, but the editor itself
still uses the legacy storage. We want to transition the editor to use `ZeroCopyGapBuffer` directly
for all content storage and operations, while maintaining compatibility with existing editor
functionality. And we want to do this in a way that allows us to gradually migrate the codebase
without breaking existing features.

**We are anchoring on the ZeroCopyGapBuffer architecture as the desired future state.** The structs
in `/tui/src/tui/editor/zero_copy_gap_buffer/` (particularly `ZeroCopyGapBuffer` and
`GapBufferLineInfo`) represent the target architecture that everything else will adapt to.

- **ZeroCopyGapBuffer is the future** - all new code targets this architecture
- **GapBufferLineInfo is the standard** line metadata format
- **VecEditorContentLines/GCString are legacy** - will be adapted temporarily then deprecated
- **Zero-copy access** is the performance goal

#### ✅ 4.1 EditorLinesStorage Trait

- [x] Clean up zero_copy_gap_buffer.rs so that it does not use ambiguous types like `usize`

  - Use specific types like `ByteIndex`, `ColWidth`, `Length`, etc in GapBufferLineInfo and
    ZeroCopyGapBuffer

- [x] Define `EditorLinesStorage` trait based on ZeroCopyGapBuffer's API:

  - Try not to use usize for arguments and return types
    - Here are some types that should be used instead of usize: ByteIndex, ColWidth, Length,
      RowIndex, SegIndex
    - Here are some functions that make it easy to create these types: byte_index, len
  - ZeroCopyGapBuffer uses different types for 3 different indices instead of just usize:
  - `RowIndex` for line access
  - `ColIndex` for column access
  - `ByteIndex` for byte access
  - Use specific index types and not usize
    - eg, in the line access methods: `fn get_line_content(&self, index: usize) -> Option<&str>;`
    - the index type should be RowIndex, not usize
  - The same applies to line metadata access methods, utility methods, etc.
  - Don't use usize in return types
    - eg in: `fn line_count(&self) -> usize;`
    - use Length instead of usize
  - In methods be clear about the index type
    - eg in: `fn insert_line(&mut self, index: usize, content: &str);`
    - is the index a RowIndex or a SegIndex or ByteIndex?

- [x] Implement EditorLinesStorage for ZeroCopyGapBuffer (native implementation - "NG storage")

  - Study in great detail how the existing VecEditorContentLines is used by the editor component
    (engine and buffer) to figure out what methods are needed for this trait. This our benchmark or
    baseline or target for existing functionality
  - ZeroCopyGapBuffer will be used to implement this trait, so if there is methods that are not
    implemented in ZeroCopyGapBuffer, they will need to be added there first
  - Ensure that all methods retain the zero-copy and efficiency provided by ZeroCopyGapBuffer

- [x] Ask the user to deeply review this code, when they have made their changes, then make a commit
      with this progress

#### 4.2 Direct Migration to EditorLinesStorage

This phase migrates the editor to use the `EditorLinesStorage` trait, enabling a hybrid approach
that supports both direct usage of `(&str, &GapBufferLineInfo)` and backward-compatible `GCString`
conversion when needed.

##### 4.2.1 GCString and GapBufferLineInfo Integration Strategy

**Analysis of Current GCString Usage:**

Based on comprehensive codebase analysis, `GCString` (now `GCStringOwned`) has two distinct usage
patterns:

1. **Editor-specific operations** (candidates for deprecation):

   - `mutate` module: `insert_chunk_at_col()`, `delete_char_at_col()`, `split_at_display_col()` -
     Only used in editor content mutation
   - `at_display_col_index` module: `check_is_in_middle_of_grapheme()` - Only used for cursor
     validation in editor

2. **General TUI operations** (keep permanently):
   - `pad` module: Used by general TUI string formatting utilities
   - `clip` module: Widely used across 9 files for viewport management in dialogs, components, ANSI
     rendering
   - `convert` module: Used in terminal rendering pipeline
   - `trunc_start`/`trunc_end` modules: Mixed usage - syntax highlighting (can migrate) + string
     helpers (keep)

**Migration Strategy:** The `GCStringRef` implementation and `gc_string_common.rs` module provide
the perfect bridge:

- **ZeroCopyGapBuffer** can create `GCStringRef` instances for compatibility with existing
  non-editor code
- **Editor-specific modules** will be deprecated and their functionality moved to
  ZeroCopyGapBuffer/GapBufferLineInfo
- **General TUI modules** will be moved to a separate file to clearly separate concerns

**Key Insight:** `GapBufferLineInfo` already contains most of what `GCString` provides:

- `segments: SegmentArray` - Identical to GCString!
- `display_width: ColWidth` - Identical to GCString!
- `grapheme_count: Length` - Number of graphemes
- `content_len: Length` - Equivalent to `bytes_size`

The only missing piece is the actual string data, which we get from `&str` through the trait.

##### 4.2.2 Implementation Approach: Hybrid Strategy

We'll implement three complementary approaches to maximize flexibility and enable gradual migration:

**1. Direct Usage (Preferred for New Code)** Work directly with `(&str, &GapBufferLineInfo)` without
creating GCString

**2. Enhanced EditorLinesStorage Trait** Add method to get both content and metadata in one call

**3. GapBufferLineInfo Methods and Conversions** Extend `GapBufferLineInfo` with GCString-compatible
methods

##### 4.2.3 Core Structure Changes

**Make EditorContent Generic:** **Update EditorBuffer and EditorHistory:**

##### 4.2.4 Migration of Direct Line Access Patterns

**Key Patterns to Update:**

- `buffer.lines.get(index)` → `buffer.lines.get_line_content(row(index))`
- `buffer.lines.len()` → `buffer.lines.line_count().as_usize()`
- `buffer.lines.is_empty()` → `buffer.lines.is_empty()`
- `buffer.lines.insert(idx, gcstring)` → `buffer.lines.insert_line(row(idx))` + `set_line()`
- `buffer.lines.remove(idx)` → `buffer.lines.remove_line(row(idx))`
- `buffer.lines.push(gcstring)` → `buffer.lines.push_line(string)`
- `buffer.lines.clear()` → `buffer.lines.clear()`

**Files with Heavy Line Access:**

1. `content_mut.rs` - Core mutation operations (~20 occurrences)
2. `validate_buffer_mut.rs` - Validation logic
3. `buffer_struct.rs` - Buffer operations accessing line data
4. `clipboard_support.rs` - Copy/paste operations

##### 4.2.5 Benefits of This Approach

1. **Zero-copy Performance**: New code works directly with `&str` + `GapBufferLineInfo`
2. **No Redundant Parsing**: Segments are already computed in `GapBufferLineInfo`
3. **Backward Compatibility**: Existing code can still get `GCString` when needed
4. **Gradual Migration**: Update code incrementally without breaking functionality
5. **Type Safety**: Compiler ensures correct data access patterns

##### 4.2.6 Migration Tasks

- [x] Refactor `gc_string.rs` in preparation for moving to `ZeroCopyGapBuffer` and
      `GapBufferLineInfo`
  - Rename `GCString` to `GCStringOwned`
  - Add a new trait `GCString` with associated type
  - Move `GCStringOwned` into its own file `gc_string_owned.rs`
- [x] Create new `GCStringRef` type that implements the `GCString` trait in `gc_string_ref.rs`
  - introduce a new GCStringRef type that will be used in places where we need a reference to a
    string-like type that does NOT own the string data, but can still be used with graphemes.
  - This will be useful in cases in the editor code which uses ZeroCopyGapBuffer and
    GapBufferLineInfo, which can be converted to GCStringRef (cheaply)
- [x] Create `gc_string_common.rs` module to share implementation between `GCStringRef` and
      `GCStringOwned`
  - Implement `GCStringData` trait for data access abstraction
  - Share common functionality to eliminate code duplication
  - Ensure both implementations produce identical results
- [x] Refactor both `GCStringRef` and `GCStringOwned` to use shared common functions
  - Fix all clippy warnings and ensure clean compilation
  - Verify compatibility with comprehensive test suite
- [x] Move non-editor usage modules from `gc_string_owned.rs` to separate file
      `gc_string_owned_non_editor_impl.rs`:
  - `pad` module (used by general TUI string formatting)
  - `clip` module (widely used across TUI components)
  - `convert` module (used in terminal rendering pipeline)
  - `trunc_start`/`trunc_end` modules (general string utilities)
- [x] Add Unicode correctness functionality to `GapBufferLineInfo`:
  - `at_display_col_index` module (`check_is_in_middle_of_grapheme`) -> built into GapBufferLineInfo
    for Unicode correctness
- [x] Move remaining editor-specific modules from `gc_string_owned.rs` to separate file
      `gc_string_owned_editor_impl.rs`:
  - `mutate` module (`insert_chunk_at_col`, `delete_char_at_col`, `split_at_display_col`) ->
    candidates for `ZeroCopyGapBuffer` methods
  - Remaining `at_display_col_index` methods (`get_string_at`, `get_string_at_right_of`, etc.) ->
    candidates for `GapBufferLineInfo` methods
  - Clear separation of editor-specific code for Phase 4.3 migration
- [x] make sure that the usages of the `GCString` trait are consistent. you might see some places
      use `dyn GCString` and some places use `impl GCString`. i like the `impl GCString` syntax
      more, so use this if possible
- [x] Think deeply about the following ideas and review with the user before proceeding:
  - for code that uses `ZeroCopyGapBuffer`, like the new editor code, i think we should eliminate
    the use of `GCString` trait in favor of `GapBufferLineInfo`
  - there might still be code that is not editor related (like lolcat) that still uses `GCString`;
    in these cases it makes sense to still keep `GCString` around and use it, since this is not
    editor related code, which probably does not have `ZeroCopyGapBuffer`.
  - also i have renamed `GCString` -> `GCStringOwned`, and have added a new trait called `GCString`,
    which GCStringOwned implements. i have also added a `GCStringRef` struct which implements this
    `GCString` trait as well. this will provide more flexibility in whether we are making a
    `GCString` that owns its inner string or not.
  - so `ZeroCopyGapBuffer` + `GapBufferLineInfo` can use `GCStringRef` (which works for referenced
    strings) instead of using `GCStringOwned`, in case there is a need to use code that expects
    `GCString` (eg `color_wheel` code that is used in the syntax highlighting heading formatting
    code in the editor component)
  - **Analysis Complete**: Decided on hybrid strategy that enables:
    - Direct usage: `let (content, info) = buffer.lines.get_line_with_info(row_index)?;`
    - Interface compatibility: `let gc_ref = info.to_gc_string_ref(content);`
    - Zero-copy performance for editor operations
    - Backward compatibility with non-editor code (lolcat, color_wheel)
- [x] Add `get_line_with_info()` method to `EditorLinesStorage` trait
  - Added method to return `Option<(&str, &GapBufferLineInfo)>` for efficient dual access
  - Located in `tui/src/tui/editor/editor_lines_storage.rs:75`
- [x] Implement `get_line_with_info()` for `ZeroCopyGapBuffer`
  - Implemented efficient access method using existing `get_line_content()` and `get_line_info()`
  - Located in `tui/src/tui/editor/zero_copy_gap_buffer/editor_lines_storage_impl.rs:40-44`
  - Added comprehensive test `test_get_line_with_info()` with GCString compatibility verification
- [x] Add `GCString`-compatible methods to `GapBufferLineInfo`
  - Added `get_string_at()`, `get_string_at_right_of()`, `get_string_at_left_of()`,
    `get_string_at_end()` methods
  - All methods marked with `#[must_use]` for clippy compliance
  - Located in `tui/src/tui/editor/zero_copy_gap_buffer/buffer_storage.rs`
- [x] Implement fast `GCString` conversion (`to_gc_string_ref()` method)
  - Added efficient `to_gc_string_ref<'a>(&'a self, content: &'a str) -> GCStringRef<'a>` method
  - Uses existing `GCStringRef::from_gap_buffer_line()` for zero-allocation conversion
  - Enables seamless interface compatibility: `let gc_ref = info.to_gc_string_ref(content);`
  - Located in `tui/src/tui/editor/zero_copy_gap_buffer/buffer_storage.rs:513-515`
- [x] Implement `From<(&str, &GapBufferLineInfo)> for GCStringRef` trait for idiomatic Rust
      conversion
  - Added trait implementation that calls `GCStringRef::from_gap_buffer_line()` internally
  - Enables idiomatic conversion: `let gc_ref: GCStringRef = (content, info).into();`
  - Located in `tui/src/core/graphemes/gc_string/borrowed/gc_string_ref.rs:327-330`
  - Added comprehensive test `test_from_gap_buffer_line_info()` verifying both approaches produce
    identical results
  - Provides both named method (`to_gc_string_ref()`) and trait-based (`.into()`) conversion for
    maximum flexibility
- [x] Ask the user to deeply review this code, when they have made their changes, then make a commit
      with this progress

- [x] Direct Migration to ZeroCopyGapBuffer (No VecEditorContentLinesWrapper)

  - [x] Remove the trait `EditorLinesStorage`
  - [x] Remove generics from EditorContent and EditorBuffer, they only use ZeroCopyGapBuffer
  - [x] Remove optimization from `segment_builder.rs` which was causing tests to fail due to bugs;
        favor correctness over performance
  - [x] Run comprehensive checks
    - `cargo check` - ensure compilation
    - `cargo clippy --all-targets` - fix all lint warnings
    - `cargo doc --no-deps` - fix documentation warnings
    - `cargo nextest run` - ensure all tests pass

- [ ] Post-Migration Validation
  - [x] Manual testing validation: run editor with real content to ensure functionality works
    - [x] Undo/redo is not working (undo/redo does not update the UI even though the editor content
          bounds are updated correctly)
    - [x] Add tests to editor_engine/engine_public_api.rs which is has no tests (major test coverage
          blind spot)!
  - [x] Remove old type definitions and deprecated code
    - Remove `VecEditorContentLines` from `sizing.rs`
    - Clean up any remaining legacy type usage
  - [x] Investigate `tui/src/tui/editor/editor_buffer/clipboard_support.rs` to see how it uses
        `GCStringOwned`, it should not use `GCStringOwned` anymore, the `clip_to_range()` method
        should be dropped and replaced with `clip_to_range_str()` method.
  - [x] Replace all the uses of `(&str, &GapBufferLineInfo)` in existing function arguments, with
        `LineWithInfo` type alias, for better ergonomics
  - [x] Rename `buffer_storage.rs` to `gap_buffer_core.rs`
  - [x] Convert LineWithInfo type alias to GapBufferLine struct
  - [x] Check whether `GCString`, `GCStringData`, `GCStringOwned`, `GCStringRef` can be dropped now
        editor has fully transitioned to `ZeroCopyGapBuffer` and `GapBufferLineInfo`
    - [x] Drop `GCStringRef` struct
    - [x] Drop `GCString` and `GCStringData` traits
  - [x] Refactor `gap_buffer_core.rs` into many files in the `core` module to manage complexity
  - [x] Create unified grapheme trait for `Vec<GCStringOwned>` and `ZeroCopyGapBuffer`; plan in
        [done/task_unified_grapheme_trait.md](done/task_unified_grapheme_trait.md)
  - [x] Evaluate the need to use the new unified grapheme traits `GraphemeString` and `GraphemeDoc`
        in the editor codebase, but only with `ZeroCopyGapBuffer`. Currently there is no value in
        doing this, it's more for future-proofing the codebase.
  - [x] Ask the user to deeply review this code, when they have made their changes, then make a
        commit with this progress
  - [ ] In all Rust files in the codebase that have nested modules, that have complex multi-line
        import statements (`use super::{...}`) replace the complex import statement with
        `#[allow(clippy::wildcard_imports)] use super::*;`
  - [ ] Test all major editor operations work correctly in `edi`
    - Text insertion, deletion, cursor movement
    - Undo/redo functionality
    - Selection operations
    - File loading and saving
  - [ ] Ask the user to deeply review this code, when they have made their changes, then make a
        commit with this progress

### Phase 5: Optimization

#### 5.1 Benchmark current implementation

- [ ] Performance spot check: ensure no significant regression
- [ ] Memory usage verification: ensure memory characteristics are reasonable

#### 5.2 Memory Optimization

- [ ] Implement line pooling for deletions
- [ ] Add memory usage tracking
- [ ] Implement buffer compaction
- [ ] Add growth strategy configuration
- [ ] Profile memory usage patterns
- [ ] Document memory guarantees
- [ ] Make sure that all docs in module are up to date with the latest changes added here
- [ ] Ask the user to deeply review this code, when they have made their changes, then make a commit
      with this progress

#### 5.3 Performance Optimization

- [ ] Add segment caching strategy
- [ ] Implement lazy segment rebuilding
- [ ] Optimize ASCII-only document handling
- [ ] Add SIMD optimizations for padding ops
- [ ] Cache line length calculations
- [ ] Profile and optimize hot paths
- [ ] Make sure that all docs in module are up to date with the latest changes added here
- [ ] Ask the user to deeply review this code, when they have made their changes, then make a commit
      with this progress

#### 5.4 Advanced Features

- [ ] Implement line chaining for >256 chars
- [ ] Add configurable line size
- [ ] Implement view slicing for large docs
- [ ] Add incremental parsing support
- [ ] Implement parallel segment building
- [ ] Add memory-mapped file support
- [ ] Make sure that all docs in module are up to date with the latest changes added here
- [ ] Ask the user to deeply review this code, when they have made their changes, then make a commit
      with this progress

#### 5.5 Tooling and Debugging

- [ ] Add buffer visualization tool
- [ ] Create memory layout debugger
- [ ] Add performance profiling hooks
- [ ] Create buffer integrity checker
- [ ] Add statistics collection
- [ ] Document performance characteristics
- [ ] Make sure that all docs in module are up to date with the latest changes added here
- [ ] Run `cargo clippy --all-targets` and fix all the lint warnings generated by this tool-
- [ ] Ask the user to deeply review this code, when they have made their changes, then make a commit
      with this progress

### Phase 6: Benchmarking and Profiling

#### 6.1 Micro Benchmarks

- [x] Create benchmark suite using `cargo bench`. Add these as plain tests with `#[bench]` attribute
      and co-locate them in the file with the source code under test.
- [ ] Benchmark single character insertion (ASCII vs Unicode)
- [ ] Benchmark string insertion (various sizes)
- [ ] Benchmark line deletion operations
- [ ] Benchmark cursor movement operations
- [x] Benchmark segment building for different text types
- [ ] Compare ZeroCopyGapBuffer vs VecEditorContentLines performance
- [ ] Benchmark memory allocation patterns
- [ ] Make sure that all docs in module are up to date with the latest changes added here
- [ ] Make a commit with this progress

#### 6.2 Macro Benchmarks

- [ ] Benchmark full document loading (various sizes)
- [ ] Benchmark syntax highlighting performance
- [ ] Benchmark parser performance with padding
- [ ] Benchmark editor responsiveness (keystroke to render)
- [ ] Benchmark memory usage for large documents
- [ ] Benchmark scrolling performance
- [ ] Create automated performance regression tests
- [ ] Make sure that all docs in module are up to date with the latest changes added here
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
- [ ] Make sure that all docs in module are up to date with the latest changes added here
- [ ] Make a commit with this progress

#### 6.4 Performance Analysis

- [ ] Analyze cache miss patterns
- [ ] Profile branch prediction misses
- [ ] Measure memory bandwidth usage
- [ ] Analyze SIMD utilization opportunities
- [ ] Profile lock contention (if any)
- [ ] Create performance dashboard
- [ ] Set performance budgets/targets
- [ ] Make sure that all docs in module are up to date with the latest changes added here
- [ ] Run `cargo clippy --all-targets` and fix all the lint warnings generated by this tool
- [ ] Make a commit with this progress

### Testing and Documentation

#### 7.1 Unit Testing

- [ ] Test each ZeroCopyGapBuffer method
- [ ] Test Unicode edge cases
- [ ] Test buffer overflow scenarios
- [ ] Test parser with various inputs
- [ ] Test editor operations
- [ ] Add property-based tests
- [ ] Make sure that all docs in module are up to date with the latest changes added here
- [ ] Make a commit with this progress

#### 7.2 Integration Testing

- [ ] Add tests in engine_public_api.rs for render_engine() (which is a fair amount of work) but
      valuable to have (there are current no tests for this function)
- [ ] Test full editor workflow
- [ ] Test with real markdown files
- [ ] Test performance vs old implementation
- [ ] Test memory usage patterns
- [ ] Test with stress scenarios
- [ ] Add regression test suite
- [ ] Make sure that all docs in module are up to date with the latest changes added here
- [ ] Make a commit with this progress

#### 7.3 Documentation

- [ ] Document ZeroCopyGapBuffer API
- [ ] Document migration guide
- [ ] Document performance characteristics
- [ ] Add code examples
- [ ] Update editor architecture docs
- [ ] Create troubleshooting guide
- [ ] Document benchmark results
- [ ] Make sure that all docs in module are up to date with the latest changes added here
- [ ] Run `cargo clippy --all-targets` and fix all the lint warnings generated by this tool
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

### Invariant

- `parse_markdown` works with `&str` and this can not be changed! We must work around this
  invariant, which is why we are implementing this gap buffer in the first place, so the nature of
  the backing store for the lines of (unicode) text makes it trivial to access it as a `&str`

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
pub struct ZeroCopyGapBuffer {
    // Contiguous buffer storing all lines
    // Each line is exactly LINE_SIZE bytes
    buffer: Vec<u8>,

    // Metadata for each line (grapheme clusters, display width, etc.)
    lines: Vec<GapBufferLineInfo>,

    // Number of lines currently in the buffer
    line_count: usize,

    // Size of each line in bytes
    line_size: usize, // e.g., 256
}

pub struct GapBufferLineInfo {
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
impl ZeroCopyGapBuffer {
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
        self.lines.push(GapBufferLineInfo {
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
impl ZeroCopyGapBuffer {
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
impl ZeroCopyGapBuffer {
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
2. Implement basic `ZeroCopyGapBuffer` struct with buffer management
3. Add `GapBufferLineInfo` struct for metadata tracking
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

1. Replace `VecEditorContentLines` with `ZeroCopyGapBuffer`
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
