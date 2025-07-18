<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

- [Implemented Optimizations](#implemented-optimizations)
  - [Latest Flamegraph Analysis (2025-07-18 - Current)](#latest-flamegraph-analysis-2025-07-18---current)
  - [Immediate Action Items](#immediate-action-items)
    - [Format Change Note](#format-change-note)
    - [Profiling Configuration](#profiling-configuration)
    - [Current Performance Bottleneck Analysis](#current-performance-bottleneck-analysis)
    - [Key Findings from Current Analysis](#key-findings-from-current-analysis)
  - [AnsiStyledText Display Optimization (✅ Completed - 2025-07-17, 2025-07-18)](#ansistyledtext-display-optimization--completed---2025-07-17-2025-07-18)
    - [Problem Identified](#problem-identified)
    - [Root Cause Analysis](#root-cause-analysis)
    - [Solution Implemented](#solution-implemented)
      - [Phase 1: WriteToBuf Trait (2025-07-17)](#phase-1-writetobuf-trait-2025-07-17)
      - [Phase 2: Color Support Detection Optimization (2025-07-18)](#phase-2-color-support-detection-optimization-2025-07-18)
    - [Performance Results](#performance-results)
      - [Phase 1 Results (WriteToBuf optimization)](#phase-1-results-writetobuf-optimization)
      - [Phase 2 Results (Color support detection fix)](#phase-2-results-color-support-detection-fix)
    - [Key Achievements](#key-achievements)
  - [Previous Flamegraph Analysis (2025-07-17 - Post Grapheme Optimization)](#previous-flamegraph-analysis-2025-07-17---post-grapheme-optimization)
    - [Immediate Action Items (Historical - From Post Grapheme Optimization)](#immediate-action-items-historical---from-post-grapheme-optimization)
    - [Profiling Configuration](#profiling-configuration-1)
    - [Performance Bottleneck Analysis (Post Grapheme Optimization)](#performance-bottleneck-analysis-post-grapheme-optimization)
    - [Key Findings from Post Grapheme Analysis](#key-findings-from-post-grapheme-analysis)
  - [Grapheme Segmentation and Dialog Border Optimization (✅ Completed - 2025-07-17)](#grapheme-segmentation-and-dialog-border-optimization--completed---2025-07-17)
    - [Problem Identified](#problem-identified-1)
    - [Root Cause Analysis](#root-cause-analysis-1)
    - [Solutions Implemented](#solutions-implemented)
    - [Performance Results](#performance-results-1)
      - [GCString ASCII Optimization Benchmarks](#gcstring-ascii-optimization-benchmarks)
      - [ColorWheel Caching Benchmarks](#colorwheel-caching-benchmarks)
      - [Final Flamegraph Results (2025-07-17)](#final-flamegraph-results-2025-07-17)
    - [Key Achievements](#key-achievements-1)
  - [Syntax Highlighting Resource Caching (✅ Completed - 2025-07-16)](#syntax-highlighting-resource-caching--completed---2025-07-16)
    - [Problem Identified](#problem-identified-2)
    - [Root Cause Analysis](#root-cause-analysis-2)
    - [Solution Implemented](#solution-implemented-1)
    - [Performance Impact Verified](#performance-impact-verified)
    - [Benchmark Results](#benchmark-results)
      - [Individual Resource Loading](#individual-resource-loading)
      - [Multiple Editor Creation (10 editors)](#multiple-editor-creation-10-editors)
    - [Key Achievement](#key-achievement)
  - [GCString Creation Optimization (✅ Completed - 2025-07-16)](#gcstring-creation-optimization--completed---2025-07-16)
    - [Problem Identified](#problem-identified-3)
    - [Root Cause Analysis](#root-cause-analysis-3)
    - [Solution Implemented](#solution-implemented-2)
    - [Performance Results](#performance-results-2)
    - [Key Achievement](#key-achievement-1)
  - [String Truncation Padding Fix (✅ Completed - 2025-07-16)](#string-truncation-padding-fix--completed---2025-07-16)
    - [Problem Identified](#problem-identified-4)
    - [Root Cause Analysis](#root-cause-analysis-4)
    - [Solution Implemented](#solution-implemented-3)
    - [Performance Results](#performance-results-3)
    - [Verification](#verification)
    - [Key Insight](#key-insight)
  - [Previous Flamegraph Analysis (2025-07-14)](#previous-flamegraph-analysis-2025-07-14)
    - [Profiling Configuration](#profiling-configuration-2)
    - [Current Performance Bottleneck Analysis](#current-performance-bottleneck-analysis-1)
    - [Key Changes from Previous Analysis](#key-changes-from-previous-analysis)
    - [String Truncation Optimization (✅ Completed)](#string-truncation-optimization--completed)
    - [Next Priority Optimization Targets](#next-priority-optimization-targets)
    - [Historical Next Priority Optimization Targets (2025-07-14)](#historical-next-priority-optimization-targets-2025-07-14)
  - [NG Parser Performance Optimization (✅ Completed - 2025-07-14)](#ng-parser-performance-optimization--completed---2025-07-14)
    - [Problem Identified](#problem-identified-5)
    - [Root Causes Discovered](#root-causes-discovered)
    - [Solutions Implemented](#solutions-implemented-1)
    - [Performance Results](#performance-results-4)
    - [Hybrid Parser Implementation](#hybrid-parser-implementation)
    - [Key Achievements](#key-achievements-2)
    - [Technical Insights](#technical-insights)
  - [MD Parser Optimization (✅ Completed - 2025-07-14)](#md-parser-optimization--completed---2025-07-14)
    - [Problem Identified](#problem-identified-6)
    - [Solution Implemented](#solution-implemented-4)
    - [Performance Impact](#performance-impact)
  - [Text Wrapping Optimization (✅ Root Cause Fixed - 2025-07-14)](#text-wrapping-optimization--root-cause-fixed---2025-07-14)
    - [Problem Identified](#problem-identified-7)
    - [Root Cause Analysis](#root-cause-analysis-5)
    - [Solution Implemented](#solution-implemented-5)
    - [Performance Impact](#performance-impact-1)
    - [Key Insight](#key-insight-1)
  - [Display Trait Optimization for Telemetry (✅ Completed - 2025-07-13)](#display-trait-optimization-for-telemetry--completed---2025-07-13)
    - [Problem Identified](#problem-identified-8)
    - [Solution Implemented](#solution-implemented-6)
    - [Performance Impact Verified (2025-07-14)](#performance-impact-verified-2025-07-14)
    - [Key Achievement](#key-achievement-2)
  - [Memory Size Calculation Caching (✅ Completed - 2025-07-13)](#memory-size-calculation-caching--completed---2025-07-13)
    - [Problem Identified](#problem-identified-9)
    - [Solution Implemented](#solution-implemented-7)
    - [Technical Details](#technical-details)
    - [Performance Impact](#performance-impact-2)
    - [Integration with Display Trait](#integration-with-display-trait)
  - [CRITICAL FIX: Color Support Detection Optimization (✅ Completed)](#critical-fix-color-support-detection-optimization--completed)
  - [Color Support Detection Caching (✅ Completed)](#color-support-detection-caching--completed)
  - [PixelChar Memory Optimization (✅ Completed)](#pixelchar-memory-optimization--completed)
  - [NG Parser Status (✅ Optimized and Hybrid Approach Implemented)](#ng-parser-status--optimized-and-hybrid-approach-implemented)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

> ---
>
> _This analysis was conducted through systematic benchmarking on real-world markdown content,
> measuring both macro-level parser performance and micro-level component costs to isolate the root
> causes of performance degradation._
>
> For the latest flamegraph analysis from the `tui/flamegraph.perf-folded` file (we've switched from
> the SVG format to the more compact perf-folded format which is better for Claude Code to analyze).
> In this document, place the latest analysis at the top, and keep the previous analysis below it
> for historical reference. And update the Table of Contents (TOC) accordingly.
>
> ---

# Implemented Optimizations

## Latest Flamegraph Analysis (2025-07-18 - Post PixelChar Optimization)

## Immediate Action Items

Based on the latest flamegraph analysis (2025-07-18) after PixelChar optimization:

1. **Dialog Border Unicode Optimization** (53M samples - HIGHEST PRIORITY):
   - Primary hotspot: `render_border_lines` → `lolcat_from_style` → `GCString::new`
   - Heavy unicode segmentation for dialog borders
   - Consider caching border strings or using ASCII-only borders
   - Expected benefit: Significant reduction in GCString creation overhead

2. **Logging/Formatting Memory Efficiency** (46M samples - HIGH PRIORITY):
   - `lolcat_into_string` → `__memmove_avx_unaligned_erms` in logging
   - Color wheel formatting causing significant memory operations
   - Consider pre-allocating buffers or reducing color formatting in logs
   - Solution: Buffer pooling or simpler formatting for high-frequency logs

3. **ColorWheel Cache Optimization** (38M samples - MEDIUM PRIORITY):
   - `ColorWheelCache::insert` → hash operations taking significant time
   - May need more efficient cache key or different hashing strategy
   - Consider FxHash or other faster hashing algorithms
   - Profile cache hit/miss ratios

4. **MD Parser Memory Operations** (9M samples - LOW PRIORITY):
   - `parse_block_markdown_text` still visible but much less prominent
   - Down from 42M+ samples in previous analysis
   - Pattern matching and string operations
   - Current impact acceptable

5. **Bypass Crossterm for Hot Paths** (16M samples - LOW PRIORITY - PENDING):
   - Already removed format!() calls from queue_render_op! (7.6% improvement achieved)
   - `apply_colors` and ANSI formatting still visible
   - Consider creating direct ANSI writing layer for performance-critical paths
   - Note: Current impact relatively low compared to other bottlenecks

### Prioritized Next Optimization Targets

1. **Dialog Border Caching** (RECOMMENDED NEXT):
   - **Current**: Creates new GCString for every border character
   - **Impact**: 53M samples in unicode segmentation
   - **Approach**: Cache commonly used border strings or use ASCII borders
   - **Expected benefit**: Eliminate repeated GCString creation for static content

2. **Logging Buffer Pooling**:
   - **Current**: Allocates new buffers for each log colorization
   - **Impact**: 46M samples in memory operations
   - **Approach**: Implement buffer pool for log formatting
   - **Expected benefit**: Reduce allocation overhead in hot logging paths

3. **ColorWheel Cache Efficiency**:
   - **Current**: Standard HashMap with default hasher
   - **Impact**: 38M samples in hash operations
   - **Approach**: Use FxHash or optimize cache key structure
   - **Expected benefit**: Faster cache lookups for color calculations

### Recently Completed Optimizations

1. **PixelChar SmallVec Optimization** (✅ COMPLETED - 2025-07-18):
   - Replaced `SmallVec<[PixelChar; 8]>` with `Vec<PixelChar>`
   - **Result**: COMPLETE ELIMINATION of 45M+ sample hotspot
   - Benchmarks showed Vec is 2-4x faster for typical terminal lines
   - Random access patterns now 3-5x faster

2. **RenderOp SmallVec Optimization** (✅ COMPLETED - Benchmarks show SmallVec is optimal):
   - Benchmark results show SmallVec is 2.27x faster for typical usage
   - Iteration is 2.57x faster - critical for render execution
   - Current SmallVec<[RenderOp; 8]> is optimal for our usage patterns
   
3. **ANSI Escape Code Formatting** (✅ PARTIALLY COMPLETED):
   - WriteToBuf optimization successful
   - Remaining overhead is from u8 number formatting
   - Consider lookup table for all u8 values (0-255)

### RenderOp SmallVec Optimization Details

#### Current Implementation
```rust
pub struct RenderOps {
    pub list: InlineVec<RenderOp>,  // SmallVec<[RenderOp; 8]>
}
```

#### Typical Usage Pattern
```rust
// Most common pattern: 3 operations per styled text
render_ops.push(RenderOp::ApplyColors(Some(*style)));
render_ops.push(RenderOp::PaintTextWithAttributes(...));
render_ops.push(RenderOp::ResetColor);
```

#### Why This Is A Good Next Target
1. **Clear usage pattern**: Most RenderOps contain 3-6 operations
2. **No hot path resizing**: Unlike PixelChar which extends frequently
3. **Proven approach**: Similar to successful VecTuiStyledText optimization
4. **Simpler to benchmark**: Fewer edge cases than PixelChar collections

#### Implementation Plan
1. Create benchmarks comparing SmallVec vs Vec for RenderOps
2. Test scenarios:
   - Small (3 operations) - typical styled text
   - Medium (10 operations) - complex renders with multiple styles
   - Large (50+ operations) - stress test
3. Measure: Creation, push performance, memory usage, iteration
4. If Vec proves better, update type alias in `sizes.rs`

#### Benchmark Results (✅ COMPLETED - 2025-07-18)

```
Typical usage (8 operations - within SmallVec capacity):
- SmallVec push:        52.90 ns/iter
- Vec push:             62.09 ns/iter
- Vec with_capacity:    41.71 ns/iter
- SmallVec faster by:   17% (without pre-allocation)

Complex usage (20 operations - exceeds SmallVec capacity):
- SmallVec push:        151.43 ns/iter
- Vec push:             117.20 ns/iter  
- Vec with_capacity:    94.92 ns/iter
- Vec faster by:        29% (SmallVec has spill overhead)

Real-world text line rendering (6 operations):
- SmallVec:             17.63 ns/iter
- Vec:                  40.02 ns/iter
- Vec with_capacity:    18.23 ns/iter
- SmallVec faster by:   127% (without pre-allocation)

Iteration performance:
- SmallVec:             2.42 ns/iter
- Vec:                  6.23 ns/iter
- SmallVec faster by:   157%

Clone performance:
- SmallVec:             47.83 ns/iter
- Vec:                  41.42 ns/iter
- Vec faster by:        15% (simpler clone operation)

Extend operations:
- SmallVec:             46.56 ns/iter
- Vec:                  48.28 ns/iter
- SmallVec faster by:   4% (minor difference)
```

#### Recommendation: KEEP SmallVec<[RenderOp; 8]>

Based on comprehensive benchmarking:

1. **Most operations use 6 or fewer RenderOps** - well within SmallVec's inline capacity
2. **SmallVec is 2.27x faster for typical usage** (17.63ns vs 40.02ns for text line rendering)
3. **Iteration is 2.57x faster with SmallVec** - critical for render execution
4. **Spill overhead only matters for 20+ operations** - rare in practice
5. **Vec::with_capacity matches SmallVec performance** - but requires knowing size upfront

**Conclusion**: The current SmallVec<[RenderOp; 8]> is optimal for our usage patterns. No change needed.

### Format Change Note

We've switched from using `tui/flamegraph.svg` to `tui/flamegraph.perf-folded` format for
performance analysis. The perf-folded format is more compact and better suited for Claude Code to
analyze programmatically.

### Profiling Configuration

Using `profiling-detailed` profile with:

- `-F 99`: 99Hz sampling frequency (lower than default ~4000Hz for cleaner data)
- `--call-graph=fp,8`: Frame pointer-based call graphs limited to 8 stack frames
- **Result**: Complete symbol visibility with no "[unknown]" sections
- **Format**: Using perf-folded format for more efficient analysis

### Current Performance Bottleneck Analysis

Based on the latest flamegraph analysis from `tui/flamegraph.perf-folded` (2025-07-18 - Post PixelChar Optimization):

1. **Dialog Border Rendering** (53M samples) - NEW PRIMARY BOTTLENECK
   - `render_border_lines` → `lolcat_from_style` → `GCString::new`: 53,143,882 samples
   - Heavy unicode segmentation for dialog borders
   - Each border character creates a new GCString
   - Static content being repeatedly processed

2. **Logging/Formatting Operations** (46M samples) - SECONDARY BOTTLENECK
   - `lolcat_into_string` → `__memmove_avx_unaligned_erms`: 46,739,094 samples
   - Color wheel formatting in logging infrastructure
   - Significant memory operations for log colorization
   - Buffer allocations for each formatted log entry

3. **ColorWheel Hash Operations** (38M samples)
   - `ColorWheelCache::insert` → hash operations: 38,232,074 samples
   - Caching overhead for color wheel calculations
   - Hash function may be suboptimal for cache keys
   - Frequent cache insertions suggesting misses

4. **RenderOp Clone Operations** (37M samples)
   - `SmallVec` extend operations for RenderOp: 36,999,339 samples
   - Confirms RenderOp should keep SmallVec (as benchmarked)
   - Expected behavior for render pipeline

5. **Memory Move Operations** (80M samples total - distributed)
   - `__memmove_avx_unaligned_erms` across multiple sites:
     - `clip_text_to_bounds`: 8,891,771 samples
     - `from_block` (styled text): 15,944,501 samples
     - `render_content`: 8,748,700 samples
     - `lolcat_into_string`: 46,739,094 samples
   - More evenly distributed than before

6. **ANSI Formatting** (16M samples)
   - `apply_colors` → `write_command_ansi`: 16,334,701 samples
   - Crossterm ANSI generation overhead
   - Acceptable level after previous optimizations

6. **Other Notable Operations**:
   - TextWrap operations: 53,951,360 samples (in logging/formatting)
   - System operations: 48,955,402 samples (sched_yield)
   - Page faults and memory management: 40,080,071 samples

### Key Findings from Current Analysis

1. **Previous Optimizations Working Well**:
   - AnsiStyledText Display formatting eliminated (was 16.3%)
   - Color support detection no longer appears in flamegraph
   - Format! removal from queue_render_op successful (687M samples vs 638M = 7.6% improvement)
   - Note: Further crossterm bypass optimization identified but deferred

2. **New Performance Profile**:
   - SmallVec operations for PixelChar are now the primary bottleneck (45M+ samples)
   - Memory move operations distributed across rendering pipeline (70M+ total)
   - Unicode segmentation remains significant despite optimizations (44M+ samples)
   - MD parser operations showing unexpected overhead (42M+ samples)

3. **Optimization Strategy**:
   - Focus on SmallVec → Vec conversions (proven approach from VecTuiStyledText)
   - RenderOp SmallVec optimization recommended as next target (cleaner usage pattern)
   - Memory moves can be reduced with strategic reserve() calls
   - Consider caching strategies for repeated operations

4. **Performance Distribution**:
   - More evenly distributed across components after previous optimizations
   - No single dominant bottleneck over 10% of total execution
   - System-level operations (scheduling, page faults) becoming more visible

## AnsiStyledText Display Optimization (✅ Completed - 2025-07-17, 2025-07-18)

### Problem Identified

Flamegraph analysis showed that AnsiStyledText Display formatting was the single largest performance
bottleneck at 16.3% of total execution time, with 103,121,329 samples in
`core::fmt::Display for AnsiStyledText::fmt`. The heavy overhead came from multiple write! calls to
the Formatter for each ANSI escape sequence.

### Root Cause Analysis

1. **Multiple write! calls**: Each ASText made 3+ separate write! calls to Formatter
2. **Formatter overhead**: Each write! call invokes the Formatter's state machine to check for
   formatting flags, precision, alignment, etc.
3. **No buffering**: Direct writes without batching caused repeated state machine overhead
4. **Color support detection overhead**: The `color_to_sgr` helper function was calling
   `global_color_support::detect()` for every color style, even though the result was cached

### Solution Implemented

#### Phase 1: WriteToBuf Trait (2025-07-17)

Introduced a `WriteToBuf` trait that bypasses the Formatter overhead:

```rust
pub type ASTextStorage = String;

pub trait WriteToBuf {
    /// Write to a buffer directly instead of using std::fmt::Formatter to avoid overhead.
    /// The Formatter state machine adds significant overhead when making multiple write! calls.
    fn write_to_buf(&self, buf: &mut ASTextStorage) -> Result;
}

// Display now delegates to WriteToBuf with a single write
impl Display for ASText {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let mut buf = String::new();
        self.write_to_buf(&mut buf)?;
        f.write_str(&buf)  // Single write!
    }
}
```

#### Phase 2: Color Support Detection Optimization (2025-07-18)

Fixed the remaining overhead by moving color support detection outside the helper function:

```rust
// Before: Called detect() for every color style
fn color_to_sgr(color: &ASTColor, is_foreground: bool) -> SgrCode {
    match global_color_support::detect() { // Called repeatedly!
        ColorSupport::Ansi256 => { /* ... */ }
        // ...
    }
}

// After: Detect once and pass the result
fn color_to_sgr(
    color_support: ColorSupport,  // Passed as parameter
    color: &ASTColor,
    is_foreground: bool,
) -> SgrCode {
    match color_support {
        ColorSupport::Ansi256 => { /* ... */ }
        // ...
    }
}

// In write_to_buf implementation:
let color_support = global_color_support::detect(); // Once per write_to_buf
match self {
    ASTStyle::Foreground(color) => {
        color_to_sgr(color_support, color, true).write_to_buf(buf)
    }
    // ...
}
```

### Performance Results

#### Phase 1 Results (WriteToBuf optimization)

| Benchmark             | Before   | After    | Improvement      |
| --------------------- | -------- | -------- | ---------------- |
| ansi_colors           | 172ns    | 149ns    | **13.4% faster** |
| ansi_styles           | 351ns    | 303ns    | **13.7% faster** |
| ansi_format_ast_style | 205ns    | 190ns    | **7.3% faster**  |
| ansi_format_astext    | 277ns    | 271ns    | **2.2% faster**  |
| ansi_large_content    | 16,044ns | 15,521ns | **3.3% faster**  |

#### Phase 2 Results (Color support detection fix)

The flamegraph analysis after the color support detection fix shows:

- **Before fix**: AnsiStyledText Display formatting consumed 16.3% of execution time
- **After fix**: AnsiStyledText Display formatting completely eliminated from flamegraph
- **Total improvement**: Complete elimination of the 16.3% overhead

### Key Achievements

- **Complete elimination of AnsiStyledText overhead** - from 16.3% to 0%
- **Two-phase optimization approach**:
  - Phase 1: WriteToBuf trait reduced formatter overhead
  - Phase 2: Color support detection fix eliminated the remaining overhead
- **Maintained API compatibility** - Display traits still work as before
- **Clean abstraction** - WriteToBuf trait clearly documents the optimization rationale
- **Proper caching utilization** - Color support detection now truly called once per render
- **New performance profile**: Memory operations (`__memmove_avx_unaligned_erms`) are now the top
  bottleneck at 14%+, indicating we've successfully eliminated the AnsiStyledText bottleneck

## Previous Flamegraph Analysis (2025-07-17 - Post Grapheme Optimization)

### Immediate Action Items (Historical - From Post Grapheme Optimization)

1. **Investigate Memory Management Overhead** (18.32% potential improvement):
   - Analyze why `mmput` and page deallocation is so expensive
   - May be related to process spawning or large buffer allocations
   - Consider buffer pooling or reuse strategies
   - **Update**: This is no longer visible in current flamegraph

### Profiling Configuration

Using `profiling-detailed` profile with:

- `-F 99`: 99Hz sampling frequency (lower than default ~4000Hz for cleaner data)
- `--call-graph=fp,8`: Frame pointer-based call graphs limited to 8 stack frames
- **Result**: Complete symbol visibility with no "[unknown]" sections

### Performance Bottleneck Analysis (Post Grapheme Optimization)

Based on the flamegraph analysis from `tui/flamegraph.perf-folded` after grapheme optimization:

1. **Memory Management Operations (18.32%)** - PRIMARY BOTTLENECK
   - `mmput`, `exit_mmap`, `tlb_finish_mmu` operations
   - Large memory deallocations from process lifecycle
   - `folios_put_refs` and page cache release operations

2. **Tracing/Logging Overhead (14.49%)**
   - `<tracing_subscriber...>::event` processing
   - Custom event formatter with grapheme segmentation
   - `<unicode_segmentation::grapheme::GraphemeIndices>::next` (4.23%)
   - String truncation optimization is working - no bottleneck visible

3. **Rendering Pipeline GCString Creation (3.63%)**
   - `<GCString>::new` operations in `clip_text_to_bounds`
   - Occurs in render pipeline's `process_render_op`
   - Already optimized in previous work but still visible

4. **Syntax Highlighting Deserialization (1.82% per instance)**
   - Multiple instances of `SyntaxSet::load_defaults_newlines`
   - Appears in editor engine initialization
   - Suggests repeated loading of syntax definitions

5. **SmallVec Operations in Rendering**
   - Various `SmallVec` extend operations for PixelChar collections
   - Part of normal rendering pipeline overhead

### Key Findings from Post Grapheme Analysis

1. **String Truncation Fix Confirmed Working**:
   - `truncate_from_right` does NOT appear as a bottleneck
   - No `__memmove_avx_unaligned_erms` related to string truncation
   - The padding fix successfully eliminated unnecessary allocations

2. **New Dominant Costs**:
   - Memory management (18.32%) is now the largest cost
   - This appears to be from process/thread lifecycle, not string operations

3. **Improvements Maintained**:
   - Text wrapping remains minimal impact
   - Debug formatting reduced but still present
   - String truncation completely eliminated as bottleneck

## Grapheme Segmentation and Dialog Border Optimization (✅ Completed - 2025-07-17)

### Problem Identified

Flamegraph analysis showed two major performance bottlenecks:

1. Grapheme segmentation in tracing/logging consuming 4.23% of execution time
2. Dialog border colorization consuming 11.71% through `lolcat_from_style`

### Root Cause Analysis

1. **Repeated GCString Creation**: Every colorized log message created a new `GCString` for grapheme
   segmentation
2. **No ASCII Fast Path**: `GCString::new` performed full Unicode segmentation even for simple ASCII
   strings
3. **Dialog Border Issue**: Dialog borders used instance methods (`lolcat_into_string`) that weren't
   benefiting from caching
4. **Cache Design Flaw**: Original cache only worked for static `colorize_into_styled_texts` method,
   not instance methods

### Solutions Implemented

1. **ASCII Fast Path in GCString::new**:

   ```rust
   // Check if string is ASCII and use optimized path
   if str.is_ascii() {
       // For ASCII, each char is exactly 1 byte and 1 display width
       // Direct byte indexing instead of grapheme segmentation
       // 5.4x to 13.7x performance improvement
   }
   ```

2. **Generalized ColorWheel Cache**:

   ```rust
   // Unified cache storing TuiStyledTexts
   static COLOR_WHEEL_CACHE: LazyLock<Arc<Mutex<ColorWheelCache>>> = ...

   // Cache key includes text and all ColorWheel configuration
   struct ColorWheelCacheKey {
       text: String,
       config: ColorWheelConfig,
       policy: GradientGenerationPolicy,
       direction: ColorWheelDirection,
   }
   ```

3. **Manual Hash Implementation for f64**:

   ```rust
   // f64 doesn't implement Hash, so we use to_bits()
   builder.seed.0.to_bits().hash(hasher);
   builder.seed_delta.0.to_bits().hash(hasher);
   ```

4. **Selective Caching Strategy**:
   - Only cache deterministic policies (`ReuseExistingGradientAndResetIndex`,
     `RegenerateGradientAndIndexBasedOnTextLength`)
   - Skip caching for `ReuseExistingGradientAndIndex` (maintains stateful index)

### Performance Results

#### GCString ASCII Optimization Benchmarks

| Scenario                     | Before      | After     | Improvement      |
| ---------------------------- | ----------- | --------- | ---------------- |
| ASCII short (10 chars)       | 73.14 ns    | 13.48 ns  | **5.4x faster**  |
| ASCII medium (50 chars)      | 288.96 ns   | 49.42 ns  | **5.8x faster**  |
| ASCII long (100 chars)       | 635.29 ns   | 95.74 ns  | **6.6x faster**  |
| ASCII very long (1000 chars) | 8,106.62 ns | 591.24 ns | **13.7x faster** |

#### ColorWheel Caching Benchmarks

| Scenario               | Before       | After     | Improvement     |
| ---------------------- | ------------ | --------- | --------------- |
| Dialog border (short)  | 957.83 ns    | 20.24 ns  | **47x faster**  |
| Dialog border (medium) | 19,117.48 ns | 162.74 ns | **117x faster** |
| Dialog border (long)   | 19,292.18 ns | 132.86 ns | **145x faster** |

#### Final Flamegraph Results (2025-07-17)

| Component                  | Before     | After       | Reduction              |
| -------------------------- | ---------- | ----------- | ---------------------- |
| Dialog border colorization | 11.71%     | 2.34%       | **80% reduction**      |
| Grapheme segmentation      | 4.23%      | Not visible | **100% elimination**   |
| **Total CPU reduction**    | **15.94%** | **2.34%**   | **~13.6% improvement** |

### Key Achievements

- **Unified caching solution**: Works for both static and instance ColorWheel methods
- **Manual hash implementation**: Handles f64 fields correctly for deterministic hashing
- **Thread-safe design**: Uses `LazyLock<Arc<Mutex<HashMap>>>` for Tokio compatibility
- **Smart caching**: Only caches deterministic operations, preserving stateful behavior
- **Comprehensive testing**: Added benchmarks and tests for hash implementation
- **All tests pass**: Maintains correctness while achieving dramatic performance gains

## Syntax Highlighting Resource Caching (✅ Completed - 2025-07-16)

### Problem Identified

Flamegraph analysis showed multiple instances of `SyntaxSet::load_defaults_newlines` consuming 1.82%
each. Investigation revealed that every `DialogEngine` creates its own `EditorEngine`, which loads
syntax definitions from scratch. In dialog-heavy applications, this results in repeated
deserialization of the same immutable data.

Creating a SyntaxSet and Theme takes approximately 1ms (~0.65ms for SyntaxSet, ~0.11ms for Theme).
This directly impacts UX responsiveness - when a user presses a keyboard shortcut to open a dialog,
this 1ms delay is noticeable as lag before the dialog appears. For dialog-heavy workflows, this
creates a sluggish user experience.

### Root Cause Analysis

- `DialogEngine::new()` creates `EditorEngine::new()`
- `EditorEngine::new()` calls `SyntaxSet::load_defaults_newlines()` and loads themes
- These operations involve deserializing large amounts of syntax definition data
- The data is immutable once loaded but was being loaded repeatedly

### Solution Implemented

Created a global caching mechanism using `std::sync::OnceLock` to ensure syntax resources are loaded
only once per application lifetime:

1. **Global Resources Module** (`/tui/src/tui/syntax_highlighting/global_syntax_resources.rs`):
   - Uses `OnceLock` for thread-safe lazy initialization
   - Provides `get_cached_syntax_set()` and `get_cached_theme()` functions
   - Resources are loaded on first access and cached permanently
   - Simple, clean implementation without unsafe code

2. **Updated EditorEngine**:
   - Changed from owned `SyntaxSet` and `Theme` to `&'static` references
   - Removed `Clone` derive (never used in practice)
   - Now uses cached resources via global functions

3. **Updated DialogEngine**:
   - Removed `Clone` derive (never used in practice)
   - Benefits from EditorEngine's optimization

### Performance Impact Verified

Latest flamegraph analysis confirms the optimization is working:

- **Before**: 1.82% overhead per dialog/editor instance with multiple
  `SyntaxSet::load_defaults_newlines` calls
- **After**: `SyntaxSet::load_defaults_newlines` no longer appears in flamegraph
- **Eliminated**: All repeated deserialization of syntax definitions
- **Memory**: Reduced overall memory usage by sharing definitions
- **Result**: Complete elimination of this performance bottleneck

### Benchmark Results

The performance improvements from caching are dramatic:

#### Individual Resource Loading

| Resource  | Uncached      | Cached  | Improvement           |
| --------- | ------------- | ------- | --------------------- |
| SyntaxSet | 654,835.90 ns | 0.19 ns | **3,446,504x faster** |
| Theme     | 106,754.70 ns | 0.19 ns | **561,866x faster**   |

#### Multiple Editor Creation (10 editors)

| Scenario   | Uncached        | Cached  | Improvement           |
| ---------- | --------------- | ------- | --------------------- |
| Total time | 3,920,191.40 ns | 1.99 ns | **1,969,945x faster** |

In practical terms:

- Creating a SyntaxSet takes ~0.65ms (expensive deserialization)
- Creating a Theme takes ~0.11ms (file I/O or default theme creation)
- With caching, access is essentially free (0.19 ns)
- For dialog-heavy apps creating 10 editors, we save ~3.92ms per dialog

### Key Achievement

- Syntax definitions now loaded once per application lifetime
- Confirmed elimination from flamegraph - optimization successful
- Uses clean `OnceLock` implementation without unsafe code
- Cleaner API by removing unused `Clone` derives
- All tests pass and code compiles without warnings

## GCString Creation Optimization (✅ Completed - 2025-07-16)

### Problem Identified

GCString creation in the rendering pipeline was consuming 8.61% of total execution time. The
`clip_text_to_bounds` function in
`/tui/src/tui/terminal_lib_backends/render_pipeline_to_offscreen_buffer.rs` was creating up to 3
GCString instances per call.

### Root Cause Analysis

The original implementation created multiple GCString instances unnecessarily:

1. Initial GCString creation from input string
2. Intermediate GCString after first clipping check
3. Final GCString after window bounds check

This resulted in excessive allocations during the render loop, where this function is called for
every text rendering operation.

### Solution Implemented

Optimized `clip_text_to_bounds` to minimize GCString allocations using a fast-path approach:

- Use `GCString::width()` to check string width without creating a GCString instance
- Only create GCString when absolutely necessary (when clipping is required)
- Calculate effective maximum width by combining both constraints
- Fast path: When text fits, create GCString only once at return
- Slow path: When clipping needed, create GCString for truncation operations

### Performance Results

Benchmark results show significant improvements across all scenarios:

| Scenario                   | Old (ns/iter) | New (ns/iter) | Improvement              |
| -------------------------- | ------------- | ------------- | ------------------------ |
| No clipping needed         | 963.73        | 338.80        | **64.8% faster** (2.84x) |
| With clipping              | 2,218.92      | 2,045.76      | **7.8% faster**          |
| Unicode with emoji         | 2,493.88      | 2,013.24      | **19.3% faster**         |
| Repeated calls (5 strings) | 8,386.25      | 5,695.58      | **32.1% faster**         |

### Key Achievement

- Eliminated unnecessary GCString creation in the common case (no clipping needed)
- Fast path uses only width calculation, deferring GCString creation until return
- Most common case (text fits without clipping) now runs nearly 3x faster
- Successfully addressed the 8.61% performance bottleneck identified in flamegraph
- Added comprehensive benchmarks for future performance tracking

## String Truncation Padding Fix (✅ Completed - 2025-07-16)

### Problem Identified

Analysis of the flamegraph showed that string truncation was appearing as a performance bottleneck,
despite the ASCII fast path optimization being correctly implemented. Investigation revealed the
issue was not with the optimization itself, but with unnecessary padding being requested.

### Root Cause Analysis

The custom event formatter in `/tui/src/core/log/custom_event_formatter.rs` was calling
`truncate_from_right` with `pad=true` for all body lines:

```rust
// Before (line 396):
let truncated_body_line = truncate_from_right(body_line, max_display_width, true);
```

This forced allocations even for short strings that didn't need padding, as every line would be
padded to the full terminal width.

### Solution Implemented

Changed the custom event formatter to not pad body lines:

```rust
// After:
let truncated_body_line = truncate_from_right(body_line, max_display_width, false);
```

### Performance Results

Benchmark results confirm the optimization is working:

| Scenario                    | Time (ns/iter) | Description                                 |
| --------------------------- | -------------- | ------------------------------------------- |
| ASCII no truncation, no pad | 3.35           | Zero-copy path - returns borrowed reference |
| ASCII with padding          | 57.81          | Requires allocation for padding             |
| ASCII with truncation       | 28.50          | Requires allocation for truncated string    |

The key achievement: when no processing is needed (common case), the function returns in ~3
nanoseconds with zero allocations.

### Verification

The current flamegraph confirms the fix:

- `truncate_from_right` no longer appears as a bottleneck
- No `__memmove_avx_unaligned_erms` operations related to string truncation
- The function has been effectively eliminated from performance profiles

### Key Insight

The ASCII fast path optimization was working correctly. The performance issue was caused by always
requesting padding, which forced allocations even when unnecessary. This simple one-line fix
eliminated what appeared to be a 7.89% performance regression.

## Previous Flamegraph Analysis (2025-07-14)

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

Based on the latest flamegraph analysis (2025-07-16), the optimization priorities should be:

1. ~~**String Truncation Padding Fix**~~ - ✅ COMPLETED
   - Fixed unnecessary padding in custom event formatter
   - Changed `pad=true` to `pad=false` for body lines
   - Eliminated all string truncation bottlenecks

2. **Memory Management Overhead** (18.32% potential improvement) - CRITICAL PRIORITY
   - Investigate `mmput` and page deallocation costs
   - May be related to process/thread lifecycle
   - Consider buffer pooling or reuse strategies

3. ~~**Syntax Highlighting Deserialization** (1.82% per instance)~~ - ✅ COMPLETED
   - Implemented global caching using `thread_local!` + `OnceCell`
   - Syntax definitions now loaded once per application
   - Eliminated repeated deserialization overhead

4. **Tracing/Logging Grapheme Segmentation** (4.23% potential improvement) - MEDIUM PRIORITY
   - Part of the 14.49% tracing overhead
   - Consider caching grapheme boundaries for repeated strings
   - Optimize Unicode segmentation in hot paths

5. ~~**GCString Creation in Rendering** (3.63% remaining)~~ - ✅ PARTIALLY COMPLETED
   - Already optimized `clip_text_to_bounds` by 34-65%
   - Remaining overhead is acceptable for rendering pipeline
   - Further optimization may have diminishing returns

### Historical Next Priority Optimization Targets (2025-07-14)

Based on the previous flamegraph analysis (2025-07-14) after MD parser elimination, the optimization
priorities were:

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
