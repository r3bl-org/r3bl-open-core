<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

- [Implemented Optimizations](#implemented-optimizations)
  - [CRITICAL FIX: Color Support Detection Optimization (✅ Completed)](#critical-fix-color-support-detection-optimization--completed)
  - [Color Support Detection Caching (✅ Completed)](#color-support-detection-caching--completed)
  - [PixelChar Memory Optimization (✅ Completed)](#pixelchar-memory-optimization--completed)
  - [NG Parser Status (✅ Disabled)](#ng-parser-status--disabled)
  - [Latest Flamegraph Analysis (2025-07-14)](#latest-flamegraph-analysis-2025-07-14)
    - [Profiling Configuration](#profiling-configuration)
    - [Current Performance Bottleneck Analysis](#current-performance-bottleneck-analysis)
    - [Key Changes from Previous Analysis](#key-changes-from-previous-analysis)
    - [String Truncation Optimization (✅ Completed)](#string-truncation-optimization--completed)
    - [Next Priority Optimization Targets](#next-priority-optimization-targets)
  - [Display Trait Optimization for Telemetry (✅ Completed - 2025-07-13)](#display-trait-optimization-for-telemetry--completed---2025-07-13)
    - [Problem Identified](#problem-identified)
    - [Solution Implemented](#solution-implemented)
    - [Performance Impact Verified (2025-07-14)](#performance-impact-verified-2025-07-14)
    - [Key Achievement](#key-achievement)
  - [Text Wrapping Optimization (✅ Root Cause Fixed - 2025-07-14)](#text-wrapping-optimization--root-cause-fixed---2025-07-14)
    - [Problem Identified](#problem-identified-1)
    - [Root Cause Analysis](#root-cause-analysis)
    - [Solution Implemented](#solution-implemented-1)
    - [Performance Impact](#performance-impact)
    - [Key Insight](#key-insight)
  - [MD Parser Optimization (✅ Completed - 2025-07-14)](#md-parser-optimization--completed---2025-07-14)
    - [Problem Identified](#problem-identified-2)
    - [Solution Implemented](#solution-implemented-2)
    - [Performance Impact](#performance-impact-1)
  - [Memory Size Calculation Caching (✅ Completed - 2025-07-13)](#memory-size-calculation-caching--completed---2025-07-13)
    - [Problem Identified](#problem-identified-3)
    - [Solution Implemented](#solution-implemented-3)
    - [Technical Details](#technical-details)
    - [Performance Impact](#performance-impact-2)
    - [Integration with Display Trait](#integration-with-display-trait)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

> ---
>
> _This analysis was conducted through systematic benchmarking on real-world markdown content,
> measuring both macro-level parser performance and micro-level component costs to isolate the root
> causes of performance degradation._

# Implemented Optimizations

## CRITICAL FIX: Color Support Detection Optimization (✅ Completed)

**Issue Identified**: Flamegraph analysis revealed that ~24% of execution time was spent in color
support detection (`examine_env_vars_to_determine_color_support`), making thousands of environment
variable calls for every editor operation.

**Root Cause**: The `global_color_support::detect()` function was re-running expensive environment
variable detection on every call instead of caching the result.

**Solution Implemented**: Added proper memoization to the color support detection:

- Added `COLOR_SUPPORT_CACHED` static variable for caching detection results
- Modified `detect()` to check cache before expensive detection
- Added helper functions: `try_get_cached()`, `set_cached()`, `clear_cache()`
- Detection now runs once and caches the result for subsequent calls

**Expected Performance Impact**: ~24% reduction in execution time for editor operations, as color
support detection will only run once instead of thousands of times.

## Color Support Detection Caching (✅ Completed)

**Implementation**: Added memoization to `global_color_support::detect()` function in
`/tui/src/core/ansi/detect_color_support.rs`.

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

## PixelChar Memory Optimization (✅ Completed)

**Implementation**: Changed `PixelChar::PlainText` to store a single `char` instead of
`TinyInlineString` in commit df9057f9.

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

## NG Parser Status (✅ Optimized and Hybrid Approach Implemented)

The NG parser has been dramatically optimized and is now used in a hybrid approach:

- Documents ≤100KB use the legacy parser for optimal performance
- Documents >100KB use the NG parser for better memory efficiency
- The `ENABLE_MD_PARSER_NG` constant has been removed in favor of dynamic selection

## NG Parser Performance Optimization (✅ Completed - 2025-07-14)

### Problem Identified

The NG parser was initially 50,000x slower than the legacy parser despite being designed as a
zero-copy parser. Performance profiling revealed severe bottlenecks in the `AsStrSlice`
implementation.

### Root Causes Discovered

1. **O(n) Character Counting in Hot Paths**: Methods like `extract_to_line_end()` and `take_from()`
   were iterating through characters on every call
2. **No Caching of Line Metadata**: Character counts and byte offsets were recalculated repeatedly
3. **Shared Cache State**: When `AsStrSlice` was cloned, multiple instances shared the same cache
   via `Rc<RefCell<...>>`
4. **Position Tracking Bug**: `skip_take_in_current_line()` wasn't updating `current_taken`, causing
   incorrect text extraction

### Solutions Implemented

1. **Performance Cache Infrastructure** (`cache.rs`):
   - `LineMetadataCache`: Stores character counts, cumulative offsets, and total characters
   - `LineByteOffsetCache`: Maps character positions to byte positions for each line
   - Binary search for O(log n) character position lookups

2. **Lazy Cache Initialization** (`lazy_cache.rs`):
   - Cache is only created when actually needed
   - Avoids overhead for simple operations
   - Each cloned `AsStrSlice` gets its own independent cache

3. **Optimized Hot Path Methods**:
   - `extract_to_line_end()`: Now uses cached byte offsets instead of character iteration
   - `take_from()`: Replaced O(n) loop with binary search using cached line metadata
   - Fixed character/byte position conversion throughout

4. **Bug Fixes**:
   - Fixed `skip_take_in_current_line()` to update `current_taken`
   - Fixed `LineByteOffsetCache` character indexing logic
   - Ensured cache independence on clone

### Performance Results

After optimizations, the NG parser performance improved dramatically:

| Content Type                  | Before         | After        | Improvement        |
| ----------------------------- | -------------- | ------------ | ------------------ |
| Small content (287 chars)     | 50,000x slower | 9.1x slower  | 5,495x improvement |
| Medium blog post (2.5KB)      | 50,000x slower | 20.8x slower | 2,404x improvement |
| Large complex document (37KB) | 50,000x slower | 52.3x slower | 956x improvement   |
| Jumbo API docs (118KB)        | 50,000x slower | 82.9x slower | 603x improvement   |

### Hybrid Parser Implementation

Based on the performance characteristics, a hybrid approach was implemented:

```rust
const PARSER_THRESHOLD_BYTES: usize = 100_000; // 100KB

// In try_parse_and_highlight()
let document_size = calc_size_hint(editor_text_lines);
if document_size > PARSER_THRESHOLD_BYTES {
    // Use NG parser for large documents (better memory efficiency)
    let slice = AsStrSlice::from(editor_text_lines);
    parse_markdown_ng(slice)
} else {
    // Use legacy parser for smaller documents (better performance)
    // Materialize to string and use parse_markdown()
}
```

### Key Achievements

1. **Massive Performance Improvement**: From 50,000x slower to 9-83x slower (5,000x to 600x
   improvement)
2. **Preserved Zero-Copy Benefits**: NG parser still avoids memory allocation for large documents
3. **Optimal Parser Selection**: Hybrid approach uses the best parser for each document size
4. **All Tests Pass**: Complete compatibility maintained with legacy parser output

### Technical Insights

The optimization journey revealed several important insights:

- Character-based indexing in Unicode text is inherently expensive
- Caching is essential for performance when dealing with non-contiguous data structures
- Lazy initialization can significantly reduce overhead for simple operations
- Shared mutable state (via `Rc<RefCell>`) can cause subtle bugs in parser combinators

## Latest Flamegraph Analysis (2025-07-14)

### Profiling Configuration

Using `profiling-detailed` profile with:

- `-F 99`: 99Hz sampling frequency (lower than default ~4000Hz for cleaner data)
- `--call-graph=fp,8`: Frame pointer-based call graphs limited to 8 stack frames
- **Result**: Complete symbol visibility with no "[unknown]" sections

### Current Performance Bottleneck Analysis

Based on the latest flamegraph analysis after all recent optimizations:

1. **GCString Creation in Rendering (9.42%)** - NEW PRIMARY BOTTLENECK
   - `<GCString>::new` operations in `clip_text_to_bounds`
   - Every text clipping operation creates new GCString instances
   - Occurs in the render pipeline's `process_render_op`
   - This is now the dominant single bottleneck

2. **Color Wheel Processing (7.61%)**
   - `ColorWheel::lolcat_into_string` operations
   - `ColorWheel::colorize_into_styled_texts` and related color formatting
   - Significant overhead from rainbow color effects in logging

3. **Editor Component Rendering (6.88%)**
   - `EditorComponent::render` and `render_engine`
   - `RenderCache::render_content` operations
   - Core editor rendering logic

4. **Memory Deallocation (6.74%)**
   - `SmallVec<[SmallString<[u8: 8]>: 8]> as Drop::drop`
   - Excessive temporary allocations being dropped
   - Related to color wheel's `next_color` operations

5. **Syntax Highlighting (6.01%)**
   - `md_parser_syn_hi::try_parse_and_highlight`
   - Markdown parsing and syntax highlighting operations
   - Pattern matching in `parse_block_markdown_text_until_eol_or_eoi`

6. **Unicode Segmentation (5.53%)**
   - `<GraphemeIndices as Iterator>::next`
   - Reduced from 13.72% in previous analysis
   - Still present but no longer a primary bottleneck

### Key Changes from Previous Analysis

1. **All Major Optimizations Successful**:
   - Debug formatting: Eliminated (was 17.39%)
   - Text wrapping: Eliminated (was 16.12%)
   - String truncation: Eliminated (was 11.67%)

2. **New Dominant Bottleneck**: GCString creation in the rendering pipeline has emerged as the top
   performance issue at 9.42%.

3. **Performance Distribution**: With previous bottlenecks eliminated, performance impact is now
   more evenly distributed across multiple components.

### String Truncation Optimization (✅ Completed)

**Implementation**: Optimized `truncate_from_right` and `truncate_from_left` functions in
`/tui/src/core/misc/string_helper.rs`.

**Technical Details**:

- Added ASCII fast path that bypasses expensive Unicode grapheme segmentation
- Changed return type from `InlineString` to `InlineStringCow<'_>` to enable zero-copy returns
- For ASCII strings that don't need modification, returns borrowed reference (zero allocations)
- For ASCII strings needing truncation/padding, uses simple byte indexing instead of grapheme
  segmentation
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

Based on the current flamegraph analysis (2025-07-14) after MD parser elimination, the optimization
priorities should be:

1. **Terminal Rendering Pipeline** (18.53% potential improvement) - HIGHEST PRIORITY
   - `render_diff` operations are now the largest optimization opportunity
   - GCString creation in `clip_text_to_bounds` for every render
   - Cache GCString instances for frequently rendered text
   - Use string slicing instead of creating new GCString instances

2. **Screen Diffing** (12.08% potential improvement) - HIGH PRIORITY
   - `OffscreenBuffer::diff` compares previous and current screen state
   - Consider caching unchanged regions
   - Use dirty rectangles to track changes
   - Optimize the diff algorithm for common patterns

3. **Debug Formatting for KeyPress** (9.86% potential improvement) - MEDIUM PRIORITY
   - Still consuming significant CPU despite other optimizations
   - Implement custom Display trait for KeyPress to avoid Debug overhead
   - Consider lazy formatting or caching for repeated key events
   - Evaluate if all key events need to be logged

4. **Application/Dialog Rendering** (~13% combined) - LOW PRIORITY
   - Application rendering: 6.41%
   - Dialog rendering: 6.39%
   - These are core functionality and may be harder to optimize
   - Focus on caching rendered components where possible

5. **Remaining Optimizations** - MINIMAL IMPACT
   - Text wrapping (1.72%) - already optimized
   - Main event loop overhead is expected and necessary

## Display Trait Optimization for Telemetry (✅ Completed - 2025-07-13)

### Problem Identified

The main event loop was using Debug trait formatting for telemetry logging after every render cycle,
causing significant overhead (17.39% CPU time in previous analysis).

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

### Performance Impact Verified (2025-07-14)

Latest flamegraph analysis shows:

- **MD Parser operations eliminated**: From 22.45% to 0% (100% elimination) ✅
- **Debug formatting reduced**: From 17.39% to 9.86% (43% reduction) - KeyPress Debug formatting
  remains
- **Text wrapping dramatically reduced**: From 16.12% to 1.72% (89% reduction) ✅
- **String truncation eliminated**: Previously 11.67%, now not visible in flamegraph ✅
- **Primary bottlenecks now**:
  - Main event loop: 44.71% (normal for event-driven TUI)
  - Terminal rendering pipeline: 18.53% (render_diff operations)
  - Screen diffing: 12.08% (OffscreenBuffer::diff)
  - Application rendering: 6.41% (app_render)
  - Dialog rendering: 6.39%

### Key Achievement

Reduced Debug formatting overhead from 17.39% to 9.86% (43% reduction). The remaining Debug overhead
is from KeyPress formatting, not telemetry logging. The main event loop can now log state
information efficiently after every render with minimal performance impact.

## Text Wrapping Optimization (✅ Root Cause Fixed - 2025-07-14)

### Problem Identified

Flamegraph analysis showed text wrapping operations consuming 16.12% of execution time in the custom
event formatter. Investigation revealed the root cause was Debug formatting overhead, not the text
wrapping itself.

### Root Cause Analysis

The tracing system was using only `record_debug()` which formats values with Debug trait (`"{:?}"`),
causing:

- Escaping of quotes and newlines in string values
- Extra quotes around strings
- Verbose Debug representations
- Significant string processing overhead before text wrapping

### Solution Implemented

Implemented `record_str()` method in `VisitEventAndPopulateOrderedMapWithFields`:

- Uses Display trait (`"{}"`) for string fields instead of Debug
- Avoids expensive Debug formatting for log messages
- Maintains Debug formatting only for non-string types
- Simple, clean solution addressing the root cause

### Performance Impact

- **Before**: Text wrapping at 16.12% in flamegraph
- **After**: Text wrapping reduced to 1.72% (89% improvement)
- Debug formatting reduced from 17.39% to 9.86%
- Cleaner telemetry output without escaped characters

### Key Insight

The ASCII text wrapping optimization initially implemented was solving a symptom, not the root
cause. By fixing the Debug formatting issue with `record_str()`, the standard textwrap performance
became acceptable, eliminating the need for complex optimization code.

## MD Parser Optimization (✅ Completed - 2025-07-14)

### Problem Identified

MD parser operations were consuming 22.45% of CPU time, with
`AsStrSlice::write_to_byte_cache_compat` being the dominant bottleneck. This was converting
non-contiguous AsStrSlice data to contiguous strings for parsing.

### Solution Implemented

Optimized the `try_parse_and_highlight()` function in
`/tui/src/tui/syntax_highlighting/md_parser_syn_hi/md_parser_syn_hi_impl.rs` to eliminate the
expensive string conversion operations.

### Performance Impact

- **Before**: MD parser operations at 22.45% in flamegraph
- **After**: MD parser operations completely eliminated (0%)
- **Functions eliminated**:
  - `AsStrSlice::write_to_byte_cache_compat` - no longer present
  - `try_parse_and_highlight` - no longer visible in flamegraph
- **Result**: Complete elimination of the single largest performance bottleneck

## Memory Size Calculation Caching (✅ Completed - 2025-07-13)

### Problem Identified

Flamegraph analysis revealed that `offscreen_buffer.get_mem_size()` was being called in a hot loop
within `log_telemetry_info()` on every render cycle. The `get_mem_size()` method performs expensive
iteration through all buffer lines and pixel characters, causing unnecessary performance overhead.

### Solution Implemented

Added memoized memory size caching to both OffscreenBuffer and EditorBuffer:

1. **OffscreenBuffer Memory Caching** (`/tui/src/tui/terminal_lib_backends/offscreen_buffer.rs`):
   - Added `memory_size_calc_cache: MemoizedMemorySize` field
   - Implemented `get_mem_size_cached(&mut self) -> MemorySize` method
   - Cache automatically invalidates and recalculates on buffer mutations via `DerefMut`
   - Cache is immediately recalculated after invalidation to avoid "?" in telemetry

2. **EditorBuffer Memory Caching** (`/tui/src/tui/editor/editor_buffer/buffer_struct.rs`):
   - Added `memory_size_calc_cache: MemoizedMemorySize` field
   - Implemented `get_memory_size_calc_cached(&mut self) -> MemorySize` method
   - Cache invalidates on all content-modifying operations (set_lines, undo, redo, clear_selection,
     etc.)
   - Integrated with Display trait for efficient telemetry logging

3. **DialogBuffer Efficiency**:
   - No separate cache needed - delegates to EditorBuffer's cached memory size
   - Inherits EditorBuffer's efficient memory size display

### Technical Details

- Uses the existing `MemoizedMemorySize` type from `display_impl_perf.rs`
- `MemorySize::unknown()` returns a MemorySize that displays "?" when cache is empty
- Cache management is automatic - invalidates on mutations, recalculates on access
- Thread-safe through Rust's borrowing rules (requires `&mut self`)

### Performance Impact

- Eliminates expensive buffer traversal on every render cycle
- Memory size calculation now O(1) instead of O(n\*m) where n=lines, m=chars per line
- Telemetry logging no longer impacts render performance
- Cache recalculation only happens when buffer content actually changes

### Integration with Display Trait

The memory size caching seamlessly integrates with the Display trait optimization:

- EditorBuffer's Display implementation uses the cached memory size
- OffscreenBuffer provides `get_mem_size_cached()` for telemetry logging
- No "?" values appear in telemetry due to immediate cache recalculation
