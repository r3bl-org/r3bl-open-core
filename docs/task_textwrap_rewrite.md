<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

- [Task: Rewrite textwrap in TUI Codebase](#task-rewrite-textwrap-in-tui-codebase)
  - [Detailed task tracking](#detailed-task-tracking)
    - [Phase 1: Setup and Basic Implementation](#phase-1-setup-and-basic-implementation)
    - [Phase 2: Core Components](#phase-2-core-components)
      - [Word Finding Implementation](#word-finding-implementation)
      - [Text Wrapping Engine](#text-wrapping-engine)
      - [Display Width Calculation](#display-width-calculation)
    - [Phase 3: Options and Configuration](#phase-3-options-and-configuration)
    - [Phase 4: Integration and Testing](#phase-4-integration-and-testing)
    - [Phase 5: Migration and Cleanup](#phase-5-migration-and-cleanup)
    - [Phase 6: Performance Verification](#phase-6-performance-verification)
    - [Phase 7: Future Enhancements (Optional)](#phase-7-future-enhancements-optional)
  - [Overview](#overview)
  - [Current Performance Issue](#current-performance-issue)
  - [Current textwrap Usage Analysis](#current-textwrap-usage-analysis)
    - [Location](#location)
    - [Features Used](#features-used)
  - [Implementation Plan](#implementation-plan)
    - [1. Core Components to Implement](#1-core-components-to-implement)
      - [a) Word Finding/Separation](#a-word-findingseparation)
      - [b) Text Wrapping Algorithm](#b-text-wrapping-algorithm)
      - [c) Display Width Calculation](#c-display-width-calculation)
    - [2. Integration with Existing Graphemes Code](#2-integration-with-existing-graphemes-code)
    - [3. Implementation Structure](#3-implementation-structure)
    - [4. Key Implementation Details](#4-key-implementation-details)
      - [Word Finding Algorithm](#word-finding-algorithm)
      - [Wrapping Engine](#wrapping-engine)
      - [Performance Optimizations](#performance-optimizations)
    - [5. Migration Path](#5-migration-path)
    - [6. Benefits of This Approach](#6-benefits-of-this-approach)
  - [Testing Strategy](#testing-strategy)
  - [Implementation Notes](#implementation-notes)
    - [ANSI Escape Sequence Handling](#ansi-escape-sequence-handling)
    - [Unicode Line Breaking](#unicode-line-breaking)
    - [Display Width vs Byte Length](#display-width-vs-byte-length)
  - [Future Enhancements](#future-enhancements)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

# Task: Rewrite textwrap in TUI Codebase

## Detailed task tracking

### Phase 1: Setup and Basic Implementation

- [ ] Create module structure at `tui/src/core/textwrap/`
  - [ ] Create `mod.rs` with module exports
  - [ ] Create `word_finder.rs` for word separation logic
  - [ ] Create `wrap_engine.rs` for core wrapping algorithm
  - [ ] Create `options.rs` for configuration
  - [ ] Create `utils.rs` for utilities (ANSI handling, width calc)

### Phase 2: Core Components

#### Word Finding Implementation

- [ ] Implement basic ASCII space-based word finding in `word_finder.rs`
  - [ ] Create `find_words_ascii_space()` function
  - [ ] Return iterator of word segments with whitespace attached
  - [ ] Add unit tests for ASCII word finding

- [ ] Add Unicode line breaking support
  - [ ] Integrate `unicode_segmentation` for grapheme clusters
  - [ ] Implement `find_words_unicode_break_properties()`
  - [ ] Handle Unicode line breaking rules (UAX #14)
  - [ ] Add unit tests for Unicode word finding

- [ ] Implement ANSI escape sequence handling
  - [ ] Port CSI detection logic from textwrap
  - [ ] Port OSC sequence handling
  - [ ] Create `skip_ansi_escape_sequence()` utility
  - [ ] Add tests for colored text handling

#### Text Wrapping Engine

- [ ] Implement core wrapping algorithm in `wrap_engine.rs`
  - [ ] Create main `wrap()` function
  - [ ] Implement `wrap_single_line()` for individual lines
  - [ ] Add support for line width limits
  - [ ] Return vector of wrapped lines

- [ ] Add indentation support
  - [ ] Implement initial indent handling
  - [ ] Implement subsequent indent handling
  - [ ] Calculate available width after indentation
  - [ ] Add tests for indented text

#### Display Width Calculation

- [ ] Integrate with existing grapheme utilities
  - [ ] Use `UnicodeWidthStr` from graphemes module
  - [ ] Use `UnicodeWidthChar` for character width
  - [ ] Create `display_width()` function in utils
  - [ ] Add caching mechanism for repeated calculations

- [ ] Implement performance optimizations
  - [ ] Add ASCII fast path for common cases
  - [ ] Use `SmallVec` for small allocations
  - [ ] Minimize string allocations by using slices
  - [ ] Cache width calculations for repeated characters

### Phase 3: Options and Configuration

- [ ] Implement Options struct in `options.rs`
  - [ ] Add width field
  - [ ] Add initial_indent field
  - [ ] Add subsequent_indent field
  - [ ] Add word_separator enum (AsciiSpace, UnicodeBreakProperties)
  - [ ] Add builder pattern for Options

### Phase 4: Integration and Testing

- [ ] Create comprehensive test suite
  - [ ] Unit tests for each component
  - [ ] Integration tests with log formatter scenarios
  - [ ] Unicode handling tests (emojis, CJK, combining chars)
  - [ ] ANSI escape sequence tests

- [ ] Create benchmark infrastructure
  - [ ] Create `textwrap_bench_tests.rs` module with `#[bench]` tests
  - [ ] Add baseline benchmarks (before removing textwrap):
    - [ ] ASCII text wrapping benchmark
    - [ ] Unicode text wrapping benchmark
    - [ ] ANSI escape sequence handling benchmark
    - [ ] Large text (1000+ lines) benchmark
    - [ ] Text with indentation benchmark
    - [ ] Display width calculation benchmark
    - [ ] Word finding performance benchmark
  - [ ] Save baseline benchmark results for comparison

- [ ] Create profiling infrastructure
  - [ ] Create `profile_textwrap` example in `tui/examples/`
  - [ ] Profile baseline with current textwrap crate
  - [ ] Save baseline flamegraph in `tui/flamegraph.perf-folded`

- [ ] Integrate with existing codebase
  - [ ] Update `custom_event_formatter.rs` to use new module
  - [ ] Ensure API compatibility with current usage
  - [ ] Test with existing log formatting

### Phase 5: Migration and Cleanup

- [ ] Remove textwrap dependency
  - [ ] Update `Cargo.toml` to remove textwrap
  - [ ] Update all imports to use new module
  - [ ] Run full test suite to ensure compatibility

- [ ] Documentation and examples
  - [ ] Add module documentation
  - [ ] Create usage examples
  - [ ] Document performance improvements

### Phase 6: Performance Verification

- [ ] Run benchmarks after implementation
  - [ ] Compare against baseline benchmarks
  - [ ] Verify at least 50% reduction in execution time
  - [ ] Document benchmark results in this file

- [ ] Profile the new implementation
  - [ ] Run `nu run.nu examples-with-flamegraph-profiling-detailed-perf-fold`
  - [ ] Select `profile_textwrap` example
  - [ ] Analyze flamegraph for hot paths
  - [ ] Verify reduction from 45M samples baseline
  - [ ] Focus on:
    - [ ] Unicode word breaking (target: <13M samples, down from 25.9M)
    - [ ] wrap_single_line equivalent (target: <10M samples, down from 18.8M)
  - [ ] Document profiling results and improvements

- [ ] Performance optimization iteration
  - [ ] Identify any remaining bottlenecks
  - [ ] Implement targeted optimizations
  - [ ] Re-run benchmarks and profiling
  - [ ] Update documentation with final results

### Phase 7: Future Enhancements (Optional)

- [ ] Add caching layer
  - [ ] Cache frequently wrapped text
  - [ ] Implement cache invalidation strategy
  - [ ] Add cache hit/miss metrics

- [ ] Advanced algorithms
  - [ ] Implement optimal-fit algorithm
  - [ ] Add hyphenation support
  - [ ] Support different line ending styles

- [ ] Performance tuning
  - [ ] Profile new implementation
  - [ ] Optimize hot paths
  - [ ] Consider parallel processing for large texts

## Overview

This document outlines the plan to remove the `textwrap` crate dependency and integrate its
functionality directly into the TUI codebase at
`/home/nazmul/github/r3bl-open-core/tui/src/core/textwrap/`. This will improve performance and give
us full control over text wrapping behavior.

## Current Performance Issue

From profiling data in `docs/task_tui_perf_optimize.md`:

```
**Text Wrapping Operations** (45M samples - HIGHEST PRIORITY):
- `textwrap::wrap_single_line`: 18.8M samples
- `find_words_unicode_break_properties`: 25.9M samples
- Heavy overhead in log formatting paths
- Consider caching wrapped text or optimizing unicode word breaking
```

## Current textwrap Usage Analysis

### Location

- **Single usage point**:
  `/home/nazmul/github/r3bl-open-core/tui/src/core/log/custom_event_formatter.rs` (line 164)
- Import: `use textwrap::{Options, WordSeparator, wrap};`

### Features Used

1. **Functions/APIs**:
   - `wrap()` - Main text wrapping function
   - `Options` - Configuration for wrapping behavior
   - `WordSeparator::UnicodeBreakProperties` - Unicode-aware word breaking

2. **Configuration**:
   - Terminal width-aware wrapping: `Options::new(usize(*max_display_width))`
   - Initial indent: `FIRST_LINE_PREFIX` (fancy bullet glyph + spacer)
   - Subsequent indent: `SUBSEQUENT_LINE_PREFIX` (spacer only)
   - Word separator: `WordSeparator::UnicodeBreakProperties` for proper Unicode word breaking

3. **Cargo.toml feature**: `features = ["unicode-linebreak"]`

## Implementation Plan

### 1. Core Components to Implement

#### a) Word Finding/Separation

- **Current usage**: `WordSeparator::UnicodeBreakProperties` for Unicode-aware word breaking
- **Implementation needed**:
  - Basic ASCII space-based word finding
  - Unicode line breaking algorithm (similar to textwrap's `find_words_unicode_break_properties`)
  - Leverage existing `unicode_segmentation` crate that's already used in graphemes module
  - Handle ANSI escape sequences (for colored text)

#### b) Text Wrapping Algorithm

- **Current usage**: `wrap()` function with `Options` configuration
- **Implementation needed**:
  - Core wrapping logic that takes words and fits them into lines
  - Support for initial indent and subsequent indent
  - Line width calculation considering Unicode display width
  - Integration with existing `GCString` and grapheme cluster handling

#### c) Display Width Calculation

- **Current issue**: Performance bottleneck in `find_words_unicode_break_properties` ( 25.9M
  samples)
- **Implementation approach**:
  - Use existing `unicode_width` functionality from graphemes module
  - Cache width calculations where possible
  - Fast path for ASCII characters

### 2. Integration with Existing Graphemes Code

The existing `GCString` and grapheme handling already provides:

- Proper Unicode grapheme cluster segmentation
- Display width calculation (`UnicodeWidthStr`, `UnicodeWidthChar`)
- Segment-based string representation

We can leverage this to:

- Use `Seg` for representing word fragments
- Use existing width calculations from graphemes module
- Handle complex Unicode (emojis, combining characters) correctly

### 3. Implementation Structure

```
tui/src/core/textwrap/
├── mod.rs                    # Module exports and main wrap() function
├── word_finder.rs            # Word separation logic
├── wrap_engine.rs            # Core wrapping algorithm
├── options.rs                # Configuration options
├── utils.rs                  # ANSI escape handling, width calculations
└── textwrap_bench_tests.rs   # Benchmark tests with #[bench] attribute

tui/examples/
└── profile_textwrap.rs       # Example for flamegraph profiling
```

### 4. Key Implementation Details

#### Word Finding Algorithm

- Start with ASCII space separation for simplicity
- Add Unicode line breaking using `unicode_segmentation`
- Skip ANSI escape sequences during processing
- Return iterator of word segments with whitespace attached

#### Wrapping Engine

- Take words and fit them into lines based on width
- Handle indentation (first line vs subsequent lines)
- Calculate actual display width using grapheme utilities
- Return vector of wrapped lines

#### Performance Optimizations

- Use `SmallVec` (already in deps) for small allocations
- Minimize string allocations by working with slices
- Cache width calculations for repeated characters
- Consider ASCII fast path for common cases

### 5. Migration Path

1. Create baseline benchmarks and profiling before starting
2. Create the new module structure
3. Implement basic ASCII wrapping first
4. Add Unicode support using grapheme utilities
5. Run benchmarks and profiling after each major component
6. Test with existing usage in `custom_event_formatter.rs`
7. Verify performance improvements meet targets
8. Remove textwrap dependency from Cargo.toml
9. Update imports to use new module
10. Final performance verification and documentation

### 6. Benefits of This Approach

- **Performance**: Direct integration with grapheme handling should be faster
- **Consistency**: Uses same Unicode handling as rest of TUI
- **Control**: Can optimize specifically for our use cases
- **Future-proof**: Can add features like caching, specialized algorithms
- **No external dependency**: One less crate to maintain compatibility with

## Testing Strategy

1. **Unit tests**: Test each component (word finder, wrapper, width calculation)
2. **Integration tests**: Test with actual log formatting scenarios
3. **Performance tests**: Benchmark against current textwrap implementation
4. **Unicode tests**: Ensure proper handling of:
   - Emojis (single and multi-codepoint)
   - CJK characters
   - Combining characters
   - Zero-width joiners
   - ANSI escape sequences

## Implementation Notes

### ANSI Escape Sequence Handling

Textwrap handles ANSI escape sequences by:

1. Detecting CSI (Control Sequence Introducer): `\x1b[`
2. Skipping until final byte in range `\x40`..=`\x7e`
3. Also handling OSC (Operating System Command) sequences

We need to implement similar logic to ensure colored text wraps correctly.

### Unicode Line Breaking

The Unicode line breaking algorithm (UAX #14) is complex. Key points:

1. Don't break at hyphens (handled by word splitter)
2. Allow breaks between emoji
3. Allow breaks in CJK text
4. Respect zero-width joiners

### Display Width vs Byte Length

Remember the distinction:

- Byte length: How much memory the text uses
- Display width: How many columns it takes in terminal
- Grapheme count: Number of user-perceived characters

Example:

| Character | Bytes | Display Width | Graphemes |
| --------- | ----- | ------------- | --------- |
| `a`       | 1     | 1             | 1         |
| `😀`      | 4     | 2             | 1         |
| `🙏🏽`      | 8     | 2             | 1         |

## Future Enhancements

Once the basic implementation is working:

1. Add caching for frequently wrapped text
2. Implement optimal-fit algorithm (vs first-fit)
3. Add hyphenation support
4. Support for different line ending styles
5. Parallel processing for large texts
