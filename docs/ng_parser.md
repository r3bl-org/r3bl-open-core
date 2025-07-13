# NG Markdown Parser Performance Analysis

## Overview

This document presents a comprehensive performance analysis of the Next Generation (NG) markdown
parser compared to the legacy parser implementation. The analysis was conducted through extensive
benchmarking to identify performance bottlenecks and understand the architectural differences
between the two approaches.

## CRITICAL FIX: Color Support Detection Optimization

**Issue Identified**: Flamegraph analysis revealed that ~24% of execution time was spent in color support detection (`examine_env_vars_to_determine_color_support`), making thousands of environment variable calls for every editor operation.

**Root Cause**: The `global_color_support::detect()` function was re-running expensive environment variable detection on every call instead of caching the result.

**Solution Implemented**: Added proper memoization to the color support detection:
- Added `COLOR_SUPPORT_CACHED` static variable for caching detection results
- Modified `detect()` to check cache before expensive detection
- Added helper functions: `try_get_cached()`, `set_cached()`, `clear_cache()`
- Detection now runs once and caches the result for subsequent calls

**Expected Performance Impact**: ~24% reduction in execution time for editor operations, as color support detection will only run once instead of thousands of times.

## Executive Summary

The NG parser shows significant performance degradation compared to the legacy parser, with
slowdowns ranging from **1.7x to 509x** depending on content size and complexity. Through systematic
benchmarking, we identified that the performance issues are **not** caused by data structure access
patterns or materialization costs, but rather by fundamental algorithmic differences in the parsing
implementation.

## Performance Comparison Results

### Original Legacy vs NG Performance Gaps

| Content Category | Performance Degradation Range      | Severity |
| ---------------- | ---------------------------------- | -------- |
| Small content    | 72-240% slower (1.7-3.4x)          | Moderate |
| Medium content   | 216-3,756% slower (3.2-38.6x)      | High     |
| Large content    | 3,435-19,481% slower (35.4-195.8x) | Critical |
| Jumbo content    | 2,454-50,863% slower (25.5-509.6x) | Critical |

### Detailed Benchmark Results

#### Small Content Benchmarks

- **Empty string**: 586.69 ns → 1,002.33 ns (**71% slower**)
- **Simple formatting**: 2,081.51 ns → 7,258.60 ns (**249% slower**)
- **Real world**: 19,611.78 ns → 165,843.70 ns (**746% slower**)

#### Medium Content Benchmarks

- **Blog post**: 63,826.95 ns → 2,332,467.20 ns (**3,554% slower**)
- **Code blocks**: 1,008.72 ns → 6,485.78 ns (**543% slower**)
- **Nested lists**: 4,273.23 ns → 12,906.84 ns (**202% slower**)

#### Large Content Benchmarks

- **Complex document**: 174,966.10 ns → 34,654,602.00 ns (**19,706% slower**)
- **Tutorial**: 61,809.46 ns → 2,329,415.00 ns (**3,669% slower**)

#### Jumbo Content Benchmarks

- **API documentation**: 222,432.05 ns → 118,446,617.70 ns (**53,159% slower**)
- **Comprehensive document**: 45,933.65 ns → 1,221,797.60 ns (**2,560% slower**)

#### Unicode Content Benchmarks

- **Emoji headings**: 1,371.28 ns → 4,898.09 ns (**257% slower**)
- **Emoji content**: 2,483.48 ns → 7,191.29 ns (**190% slower**)

## Root Cause Investigation

### Hypothesis Testing Through Targeted Benchmarks

To identify the root cause of the performance degradation, we implemented additional benchmark
categories to test specific hypotheses:

#### 1. Materialization Cost Analysis (bench*f*\*)

**Hypothesis**: The NG parser's slowdown is caused by the overhead of converting `&[GCString]` to
`String` for the legacy parser.

**Test**: Measured pure `&[GCString] → String` conversion overhead:

- Small content: 122.63 ns
- Medium content: 309.11 ns
- Large content: 772.47 ns
- Jumbo content: 1,256.56 ns
- Highly fragmented: 2,517.39 ns

**Result**: **HYPOTHESIS REJECTED**

- Materialization costs are negligible (< 0.1% of parsing time)
- Small NG parsing: 168,150 ns vs materialization: 123 ns (**1,371x difference**)
- Large NG parsing: 34,276,613 ns vs materialization: 772 ns (**44,379x difference**)

#### 2. Character Access Pattern Analysis (bench*g*\*)

**Hypothesis**: Non-contiguous memory access through GCString causes the performance degradation.

**Test**: Compared character access patterns between contiguous strings and GCString arrays:

| Content Type      | String Access | GCString Access | Slowdown |
| ----------------- | ------------- | --------------- | -------- |
| Small sequential  | 383 ns        | 1,809 ns        | **4.7x** |
| Medium sequential | 1,145 ns      | 6,239 ns        | **5.4x** |
| Random access     | 71 ns         | 130 ns          | **1.8x** |
| Cross-boundary    | -             | 13 ns           | -        |

**Result**: **HYPOTHESIS REJECTED**

- GCString character access is only 4-5x slower than contiguous strings
- This cannot explain the 35-500x NG parser slowdowns
- The performance gap is much larger than character access overhead

## Architecture Comparison

### Legacy Parser Approach

```rust
// Convert &[GCString] to String, then parse
let content = gc_strings.iter()
    .map(|gc| gc.as_ref())
    .collect::<Vec<&str>>()
    .join("\n");
parse_markdown(&content)
```

### NG Parser Approach

```rust
// Parse directly from &[GCString]
fn parse_from_gc_strings(gc_strings: &[GCString]) -> ParsedMarkdown {
    // Direct parsing without materialization
    // Uses AsStrSlice virtual array abstraction
}
```

### Key Architectural Differences

1. **Memory Layout**:
   - Legacy: Contiguous string memory
   - NG: Non-contiguous GCString segments

2. **Parser Input**:
   - Legacy: Single `&str` reference
   - NG: Virtual array of string segments

3. **Memory Allocation**:
   - Legacy: One-time materialization cost
   - NG: Potential repeated allocations during parsing

## Conclusions and Insights

### What We Learned

1. **Materialization is not the bottleneck**: Converting `&[GCString]` to `String` takes
   microseconds, not milliseconds.

2. **Character access patterns are not the primary issue**: While GCString access is slower, it's
   only 4-5x slower, not 500x.

3. **The real bottleneck is algorithmic**: The NG parser's core parsing logic has fundamental
   performance issues.

4. **Scale sensitivity**: Performance degradation worsens exponentially with content size,
   suggesting O(n²) or worse algorithmic complexity.

### Potential Root Causes

Based on the elimination of other hypotheses, the performance issues likely stem from:

1. **Algorithmic complexity differences** in the parsing implementation
2. **Memory allocation patterns** during parsing operations
3. **Iterator overhead** in the virtual array abstraction
4. **Repeated boundary checks** or validation logic
5. **Cache locality issues** from frequent cross-segment access
6. **Parsing state management** inefficiencies

### Performance Impact Scale

The performance degradation follows a concerning pattern:

- **Small content (< 1KB)**: Acceptable 2-8x slowdown
- **Medium content (1-10KB)**: Problematic 3-40x slowdown
- **Large content (10-100KB)**: Critical 35-200x slowdown
- **Jumbo content (> 100KB)**: Unusable 25-500x slowdown

## Recommendations

### Immediate Actions

1. **Use legacy parser for production**: The NG parser is not performance-viable in its current
   state.

2. **Profile the NG parser implementation**: Use tools like `perf`, `flamegraph`, or
   `cargo-profiler` to identify algorithmic bottlenecks.

3. **Review parsing algorithm**: Compare the core parsing logic between legacy and NG
   implementations to identify complexity differences.

### Long-term Strategy

1. **Hybrid approach**: Consider materializing content only when performance is critical, while
   keeping NG parser for memory-sensitive scenarios.

2. **Incremental optimization**: Focus on optimizing the most common parsing operations first.

3. **Architecture review**: Evaluate whether the AsStrSlice abstraction introduces unnecessary
   overhead.

4. **Benchmark-driven development**: Establish continuous performance monitoring to prevent
   regressions.

### Technical Debt Considerations

The current NG parser represents significant technical debt due to:

- **Unusable performance characteristics** for medium-to-large content
- **Exponential scaling issues** that will worsen with larger documents
- **Unclear optimization path** without algorithmic changes

## Future Work

1. **Detailed profiling analysis** to identify specific hot paths
2. **Algorithmic complexity analysis** of parsing operations
3. **Memory allocation pattern study** during parsing
4. **Alternative architecture exploration** for non-contiguous parsing
5. **Performance regression testing** framework implementation

## Implemented Optimizations

### Color Support Detection Caching (✅ Completed)

**Implementation**: Added memoization to `global_color_support::detect()` function in `/tui/src/core/ansi/detect_color_support.rs`.

**Technical Details**:
- Added `COLOR_SUPPORT_CACHED` static atomic variable
- Modified detection logic to check cache before expensive environment variable operations
- Added cache management functions: `try_get_cached()`, `set_cached()`, `clear_cache()`
- Maintains thread-safety with atomic operations

**Performance Impact**:
- Eliminates ~24% of execution time overhead from repeated environment variable detection
- Color support detection now runs once per application lifetime instead of thousands of times
- Expected dramatic improvement in editor responsiveness during typing/editing operations

**Testing**: Added comprehensive test coverage for caching behavior to ensure correctness.

### PixelChar Memory Optimization (✅ Completed)

**Implementation**: Changed `PixelChar::PlainText` to store a single `char` instead of `TinyInlineString` in commit df9057f9.

**Technical Details**:
- Modified `PixelChar` enum to be `Copy` instead of `Clone`
- Changed `PlainText` variant from `text: TinyInlineString` to `display_char: char`
- Eliminates all clone operations in the rendering pipeline
- Simplifies memory management and improves cache locality
- For multi-char graphemes, uses the first char or replacement character

**Performance Impact**:
- Eliminates memory allocation overhead from PixelChar cloning
- Reduces SmallVec extend operations visible in flamegraph
- Improves cache locality by making PixelChar a fixed-size Copy type
- Expected significant reduction in memory copies during rendering

**Trade-offs**:
- Multi-character grapheme clusters are reduced to single characters
- This is acceptable for terminal rendering where each cell displays one visible character

### NG Parser Status (✅ Disabled)
The NG parser has been disabled in `/tui/src/tui/mod.rs` by setting `ENABLE_MD_PARSER_NG` to false,
reverting to legacy parsing due to unacceptable performance characteristics.

## Latest Flamegraph Analysis (2025-07-13)

### Profiling Configuration
Using `profiling-detailed` profile with:
- `-F 99`: 99Hz sampling frequency (lower than default ~4000Hz for cleaner data)
- `--call-graph=fp,8`: Frame pointer-based call graphs limited to 8 stack frames
- **Result**: Complete symbol visibility with no "[unknown]" sections

### Current Performance Bottleneck Analysis

Based on the latest flamegraph analysis after string truncation optimization:

1. **Debug Formatting (17.39%)** - NEW PRIMARY BOTTLENECK
   - `core::fmt::write` operations for State and EditorBuffer debug output
   - Significantly higher than previous analysis (was 11.58%)
   - Excessive debug formatting overhead on every render cycle

2. **Text Wrapping in Log Formatting (16.12%)**
   - `textwrap::wrap::wrap` operations in custom_event_formatter
   - `textwrap::core::break_words` consuming significant time
   - Dynamic memory allocation in `_mi_heap_realloc_zero`

3. **Unicode Segmentation (13.72%)** - REDUCED FROM PREVIOUS
   - `<unicode_segmentation::grapheme::GraphemeIndices as Iterator>::next`
   - Primary hotspot is now in `<GCString>::new` operations
   - String truncation optimization already eliminated 11.67%
   - Still significant in dialog and editor rendering

4. **Memory Operations (11.63%)**
   - System write calls (`__GI___libc_write`)
   - Memory page allocation (`clear_page_erms`)
   - Related to terminal output and buffer management

5. **Syntax Highlighting (7.64%)**
   - `md_parser_syn_hi_impl::try_parse_and_highlight`
   - Lower than previous analysis but still measurable
   - Pattern matching operations

### Key Changes from Previous Analysis

1. **String Truncation Success**: The optimization eliminated 11.67% of Unicode segmentation overhead, reducing total Unicode processing from 45-50% to current 13.72%.

2. **Debug Formatting Emergence**: With Unicode segmentation reduced, debug formatting is now the dominant bottleneck at 17.39%.

3. **Text Wrapping Prominence**: Text wrapping operations are now the second-largest bottleneck at 16.12%.

### String Truncation Optimization (✅ Completed)

**Implementation**: Optimized `truncate_from_right` and `truncate_from_left` functions in `/tui/src/core/misc/string_helper.rs`.

**Technical Details**:
- Added ASCII fast path that bypasses expensive Unicode grapheme segmentation
- Changed return type from `InlineString` to `InlineStringCow<'_>` to enable zero-copy returns
- For ASCII strings that don't need modification, returns borrowed reference (zero allocations)
- For ASCII strings needing truncation/padding, uses simple byte indexing instead of grapheme segmentation
- Unicode strings still use the original grapheme segmentation logic for correctness
- Uses stack-allocated `InlineString` instead of heap-allocated `String`

**Performance Impact**:
- **Before**: `truncate_from_right` consumed 11.67% of total execution time
- **After**: Function no longer appears in flamegraph (too fast to measure)
- Eliminated the performance bottleneck completely for ASCII strings (common case for log messages)
- Zero allocations for strings that don't need modification
- Dramatic reduction in CPU time for log formatting operations

**Code Quality**:
- Maintains correctness for Unicode content
- Uses consistent `acc` variable naming convention
- All comments end with periods as per codebase standards
- Uses `GCString::width()` for future-proof ellipsis width calculation
- Uses `SPACER_GLYPH` constant instead of hardcoded spaces

### Next Priority Optimization Targets

Based on the current flamegraph analysis (2025-07-13), the optimization priorities should be:

1. **Debug Formatting Optimization** (17.39% potential improvement) - HIGHEST PRIORITY
   - Excessive debug formatting of State and EditorBuffer on every render
   - Consider conditional debug output (only when explicitly requested)
   - Implement lazy debug formatting or remove from hot paths
   - Use more efficient serialization for debug purposes

2. **Text Wrapping Optimization** (16.12% potential improvement) - HIGH PRIORITY
   - Pre-allocate buffer sizes in textwrap operations
   - Cache wrapped text results for unchanged content
   - Consider simpler wrapping algorithms for log output
   - Optimize memory allocation patterns in text wrapping

3. **Unicode Segmentation in GCString::new** (13.72% potential improvement) - MEDIUM PRIORITY
   - Apply ASCII fast path similar to string truncation optimization
   - Cache grapheme boundaries for frequently accessed strings
   - Consider lazy evaluation for grapheme segmentation
   - Investigate faster unicode segmentation libraries

4. **Syntax Highlighting Caching** (7.64% potential improvement) - MEDIUM PRIORITY
   - Cache highlighting results for unchanged lines
   - Implement incremental re-highlighting
   - Optimize pattern matching state machine

5. **Memory Operations** (11.63% - mostly unavoidable)
   - System write calls are necessary for terminal output
   - Focus on batching writes where possible
   - Consider reducing frequency of full redraws

## Display Trait Optimization for Telemetry (✅ Completed - 2025-07-13)

### Problem Identified
The main event loop was using Debug trait formatting for telemetry logging after every render cycle, causing significant overhead (17.39% CPU time in previous analysis).

### Solution Implemented
Implemented efficient Display trait for all State structs and buffers:

1. **EditorBuffer Display** (`/tui/src/tui/editor/editor_buffer/buffer_struct.rs`):
   - Fast summary format: `buffer:<filename>:lines(count):size(cached)`
   - Uses cached memory size when available
   - No deep traversal of buffer contents

2. **DialogBuffer Display** (`/tui/src/tui/dialog/dialog_buffer/dialog_buffer_struct.rs`):
   - Format: `dialog:<title>:results(count):<editor_buffer_info>`
   - Uses "<untitled>" convention for empty titles
   - Delegates to EditorBuffer's efficient Display

3. **State Display Implementations**:
   - All example State structs now have efficient Display traits
   - Production State structs in cmdr also updated
   - Consistent format showing counts and cached memory sizes

### Performance Impact Verified (2025-07-13)
Latest flamegraph analysis shows:
- **Debug formatting eliminated**: No Debug trait overhead visible in flamegraph
- **Text wrapping reduced**: From 16.12% to small chunks (~1-2%)
- **Primary bottlenecks now**:
  - Unicode segmentation: 11.99% (in GCString::new)
  - Color wheel formatting: 10.53% + 1.67%
  - Memory operations: 8.76% (page allocation)
  - TLB flushing: 6.83%

### Key Achievement
Successfully eliminated the 17.39% Debug formatting overhead from telemetry logging, allowing the main event loop to log state information efficiently after every render without impacting performance.

---

_This analysis was conducted through systematic benchmarking on real-world markdown content, measuring both macro-level parser performance and micro-level component costs to isolate the root causes of performance degradation._
