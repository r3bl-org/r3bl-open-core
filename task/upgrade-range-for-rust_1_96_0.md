# Task: Upgrade Range types for Rust 1.96.0

## Background
Rust 1.96.0 introduced `core::range` which contains `Copy` versions of range types (`Range`, `RangeFrom`, `RangeInclusive`). Legacy ranges in `core::ops` are not `Copy` because they implement `Iterator` directly. The new types implement `IntoIterator` and have public fields (especially `RangeInclusive`).

## Strategic Decision (Audit Results)
During the audit of this plan, we evaluated generalizing our public APIs using `impl RangeBounds<T>` to support both old and new ranges. **This approach (Points 1-3) has been explicitly discarded as a non-starter** due to the following critical issues:
1. **The `RangeBounds` Weakness**: `impl RangeBounds` forces handling of `Bound<&T>` and `Unbounded` cases, which breaks our strict validation assumptions that `start` and `end` always exist.
2. **Trait Coherence (Orphan Rule)**: Implementing our custom traits (`RangeBoundsExt`) for a generic `T: RangeBounds` conflicts with specialized implementations.
3. **Iteration and `Step` Trait**: Relying on `for i in range` for custom indices with the new ranges requires the unstable `Step` trait.

**Decision**: We will **only implement Point 4**. We will perform a surgical optimization to replace places where we currently "split" a range into two fields (`start` and `end`) just to maintain `Copy` behavior, and we will adopt the concrete `core::range` types in internal hot paths (e.g., rendering and scroll regions) to gain the performance benefits of `Copy`.

## Objectives (Point 4 Only)
- [ ] Improve performance in hot paths (terminal rendering) by using concrete `Copy` ranges from `core::range`.
- [ ] Simplify internal structs by using `core::range::Range` instead of manually split `start`/`end` fields.
- [ ] Simplify code by accessing public fields of `core::range::RangeInclusive`.

## Implementation Plan

### Phase 0: Baseline Performance Capture
- [ ] Run existing benchmarks (`cargo bench`) to capture baseline performance metrics.
- [ ] Generate baseline flamegraphs for terminal rendering hot paths using the `analyze-performance` skill.
- [ ] Save baseline data for later comparison in Phase 3.

### Phase 1: Foundational Support for Concrete Types
- [ ] Implement `RangeBoundsExt` for the concrete type `core::range::Range<I>`.
- [ ] Implement `RangeBoundsExt` for the concrete type `core::range::RangeInclusive<I>`.
- [ ] Implement `RangeConvertExt` for `core::range::RangeInclusive<I>`.
- [ ] Implement `ByteIndexRangeExt` for `core::range::Range<ByteIndex>`.

### Phase 2: Surgical Optimization (Hot Paths)
- [ ] Refactor `OffscreenBuffer::get_scroll_range_inclusive` to return `core::range::RangeInclusive<RowIndex>` (which is `Copy`).
- [ ] Update call sites interacting with `get_scroll_range_inclusive` to utilize the `Copy` behavior and direct field access (`.start`, `.end`).
- [ ] Audit and refactor `SelectionRange` (and similar structs) to store a single `core::range::Range` instead of separate `start` and `end` fields.

### Phase 3: Post-Change Performance Analysis
- [ ] **Post-Change Benchmarks**: Run `cargo bench` and generate new flamegraphs after implementing the optimizations.
- [ ] **Comparative Audit**: Use the `analyze-performance` skill to compare the new results against the Phase 0 baseline.
  - [ ] Verify reduction in stack/deref overhead in `get_scroll_range_inclusive`.
  - [ ] Verify improved register allocation/performance in rendering hot paths.
- [ ] **Document Results**: Record performance gains (or lack thereof) in the task file for historical context.

### Phase 4: Validation & Cleanup
- [ ] Run `./check.fish --full` to ensure no regressions.
- [ ] **Mandatory manual review:** Verify every file modified in this task for correct implementation and ensure no regressions.
  - [ ] `tui/src/core/coordinates/bounds_check/range_bounds_check_ext.rs`
  - [ ] `tui/src/core/coordinates/bounds_check/range_convert_ext.rs`
  - [ ] `tui/src/core/coordinates/byte/byte_index.rs`
  - [ ] `tui/src/tui/terminal_lib_backends/offscreen_buffer/vt_100_ansi_impl/vt_100_impl_ansi_scroll_helper.rs`
  - [ ] `tui/src/tui/editor/editor_buffer/selection_range.rs` (if refactored)
