# Task:  Refactor for type-safe bounds checking for scroll, selection, viewport

## 🚨 IMPORTANT USAGE INSTRUCTIONS 🚨

**This file serves as your "external memory" and "external todo list" for this refactoring task.**

### How to Use This File:
1. **Keep this file updated** as you make changes - track your progress in real-time
2. **Use as external memory** - document any discoveries, patterns, or tricky cases you encounter
3. **Update the Progress Tracking section** after completing each file
4. **Add implementation notes** when you find interesting patterns or solutions
5. **This is your working document** - treat it as an active part of your workflow

## 🚨 CRITICAL CONSTRAINTS 🚨

**BEHAVIOR PRESERVATION IS MANDATORY:**
- ❌ **NO functionality changes** - preserve exact current behavior
- ❌ **NO test changes** - tests define the specification and must remain unchanged
- ✅ **DO run tests after each change** - verify nothing breaks with `cargo nextest run`
- ✅ **Goal: Type safety + clarity** while keeping identical behavior

**If any test fails after your changes, you must fix the refactoring, not the test.**

## Overview

This refactoring improves code clarity and type safety by:
1. Renaming bounds checking methods for maximum clarity
2. Adding semantic aliases as default methods in IndexMarker trait
3. Applying these improvements consistently across the codebase

## Method Naming Strategy

### Core Methods (Explicit Names)
- `check_range_bounds_exclusive_end(start, size)` - Range `[start, start+size)` where end is EXCLUDED
- `check_range_bounds_inclusive_end(min, max)` - Range `[min, max]` where end is INCLUDED

### Semantic Aliases (Domain-Specific)
All aliases are default methods in `IndexMarker` trait:
- `is_in_viewport(start, size)` - Uses exclusive end (viewport semantics)
- `is_in_inclusive_range(min, max)` - Uses inclusive end
- `is_in_scroll_region(top, bottom)` - Uses inclusive end (VT-100 semantics)
- `is_in_selection_range(start, end)` - Uses inclusive end (text selection semantics)

## Implementation Plan

### Phase 1: Core Bounds Checking Updates

#### Task 1.1: Rename check_bounds_range
**File**: `tui/src/core/units/bounds_check/length_and_index_markers.rs`

Rename method and update documentation:
```rust
// Old:
fn check_bounds_range(&self, start_pos: Self, width_or_height: L) -> ArrayAccessBoundsStatus

// New:
fn check_range_bounds_exclusive_end(&self, start_pos: Self, width_or_height: L) -> ArrayAccessBoundsStatus
```

Update documentation to emphasize exclusive upper bound:
```rust
/// Checks if this index is within a range [start, start+size).
/// The upper bound is EXCLUSIVE, making this suitable for viewport and window bounds.
///
/// # Examples
/// - Viewport at row 5, height 10 = rows [5, 15) - row 15 is NOT included
/// - Window at col 10, width 20 = cols [10, 30) - col 30 is NOT included
```

#### Task 1.2: Rename check_inclusive_range_bounds
**File**: `tui/src/core/units/bounds_check/length_and_index_markers.rs`

Rename for consistency:
```rust
// Old:
fn check_inclusive_range_bounds(&self, min_index: Self, max_index: Self) -> ArrayAccessBoundsStatus

// New:
fn check_range_bounds_inclusive_end(&self, min_index: Self, max_index: Self) -> ArrayAccessBoundsStatus
```

#### Task 1.3: Add Semantic Aliases to IndexMarker Trait
**File**: `tui/src/core/units/bounds_check/length_and_index_markers.rs`

Add these default methods to the `IndexMarker` trait (used in Tasks 2.1 and 4.1):
```rust
trait IndexMarker {
    // ... existing methods ...

    // ========================================================================================
    // SEMANTIC ALIASES - Use these for simple boolean checks
    // ========================================================================================

    /// Check if this index is visible in a viewport
    /// Uses exclusive upper bound: [viewport_start, viewport_start + viewport_size)
    ///
    /// # When to use
    /// Use this for simple visibility checks where you only care if something is visible.
    /// If you need to handle underflow/overflow differently, use `check_range_bounds_exclusive_end`.
    ///
    /// # Example
    /// ```rust
    /// if col.is_in_viewport(viewport_start, viewport_width) {
    ///     // Render this column
    /// }
    /// ```
    fn is_in_viewport(&self, viewport_start: Self, viewport_size: Self::LengthType) -> bool {
        matches!(
            self.check_range_bounds_exclusive_end(viewport_start, viewport_size),
            ArrayAccessBoundsStatus::Within
        )
    }

    /// Check if this index is within an inclusive range
    /// Uses inclusive bounds: [min, max]
    ///
    /// # When to use
    /// Use this for simple range membership checks with inclusive bounds.
    /// If you need to know whether you're below/above the range, use `check_range_bounds_inclusive_end`.
    ///
    /// # Example
    /// ```rust
    /// if value.is_in_inclusive_range(min_allowed, max_allowed) {
    ///     // Value is valid
    /// }
    /// ```
    fn is_in_inclusive_range(&self, min: Self, max: Self) -> bool {
        matches!(
            self.check_range_bounds_inclusive_end(min, max),
            ArrayAccessBoundsStatus::Within
        )
    }

    /// Check if this index is within a scroll region
    /// Uses inclusive bounds: [top, bottom]
    ///
    /// # When to use
    /// Use this for VT-100 scroll region checks where you only need a boolean result.
    ///
    /// # Example
    /// ```rust
    /// if !row.is_in_scroll_region(scroll_top, scroll_bottom) {
    ///     return; // Skip operation outside scroll region
    /// }
    /// ```
    fn is_in_scroll_region(&self, top: Self, bottom: Self) -> bool {
        self.is_in_inclusive_range(top, bottom)
    }

    /// Check if this index is within a selection range
    /// Uses inclusive bounds: [start, end]
    ///
    /// # When to use
    /// Use this for checking if a position is within a text selection.
    ///
    /// # Example
    /// ```rust
    /// if col.is_in_selection_range(selection_start, selection_end) {
    ///     // Apply selection highlighting
    /// }
    /// ```
    fn is_in_selection_range(&self, start: Self, end: Self) -> bool {
        self.is_in_inclusive_range(start, end)
    }
}
```

**When to Use Each Method:**

| Method Type | Use When | Example |
|-------------|----------|---------|
| **Core Methods** (`check_range_bounds_*_end`) | You need to pattern match on all cases (Underflow/Within/Overflow) | Determining cursor movement direction |
| **Semantic Aliases** (`is_in_*`) | You only need a boolean result for a specific use case | Simple visibility or membership checks |

```rust
// Use CORE METHOD when you need all cases:
match col.check_range_bounds_exclusive_end(start, width) {
    ArrayAccessBoundsStatus::Underflowed => scroll_left(),
    ArrayAccessBoundsStatus::Within => no_scroll(),
    ArrayAccessBoundsStatus::Overflowed => scroll_right(),
}

// Use SEMANTIC ALIAS for simple checks:
if row.is_in_viewport(viewport_start, viewport_height) {
    render_row(row);
}
```

**Usage of These Aliases:**
- `is_in_viewport()` - Used in Task 2.1 Location 2 (line 151)
- `is_in_scroll_region()` - Used in Task 4.1 (line 349)
- `is_in_inclusive_range()` - Could be used in Task 2.1 Location 1, but using core method for clarity
- `is_in_selection_range()` - Not currently used (Task 2.2 needs pattern matching on all cases)

### Phase 2: Editor Module Refactoring

#### Task 2.1: Update validate_scroll_on_resize.rs
**File**: `tui/src/tui/editor/editor_engine/validate_scroll_on_resize.rs`

**Location 1**: Lines 96-97 (Vertical viewport check)
```rust
// Current:
let is_within = caret_scr_adj_row_index >= scr_ofs_row_index
    && caret_scr_adj_row_index <= (scr_ofs_row_index + vp_height);

// Refactor to:
use crate::core::units::bounds_check::{IndexMarker, ArrayAccessBoundsStatus};

let start_idx = scr_ofs_row_index;
let end_idx = scr_ofs_row_index + vp_height;
let is_within = matches!(
    caret_scr_adj_row_index.check_range_bounds_inclusive_end(start_idx, end_idx),
    ArrayAccessBoundsStatus::Within
);
```

**Location 2**: Lines 153-154 (Horizontal viewport check)
```rust
// Current:
let is_within = if caret_scr_adj_col_index >= scr_ofs_col_index
    && caret_scr_adj_col_index < scr_ofs_col_index + viewport_width

// Refactor to:
let is_within = if caret_scr_adj_col_index.is_in_viewport(scr_ofs_col_index, viewport_width)
```

#### Task 2.2: Refactor selection_range.rs
**File**: `tui/src/tui/editor/editor_buffer/selection_range.rs`

**Method: `locate_column`** (lines 224-232)
```rust
// Current:
pub fn locate_column(&self, caret: CaretScrAdj) -> CaretLocationInRange {
    if caret.col_index < self.start.col_index {
        CaretLocationInRange::Underflow
    } else if caret.col_index >= self.end.col_index {
        CaretLocationInRange::Overflow
    } else {
        CaretLocationInRange::Contained
    }
}

// Refactor to:
use crate::core::units::bounds_check::{IndexMarker, ArrayAccessBoundsStatus};

pub fn locate_column(&self, caret: CaretScrAdj) -> CaretLocationInRange {
    let size = width(*(self.end.col_index - self.start.col_index));
    match caret.col_index.check_range_bounds_exclusive_end(self.start.col_index, size) {
        ArrayAccessBoundsStatus::Underflowed => CaretLocationInRange::Underflow,
        ArrayAccessBoundsStatus::Overflowed => CaretLocationInRange::Overflow,
        ArrayAccessBoundsStatus::Within => CaretLocationInRange::Contained,
    }
}
```

**Note**: `locate_scroll_offset_col` method should remain as-is (simple position comparison).

#### Task 2.3: Review scroll_editor_content.rs
**File**: `tui/src/tui/editor/editor_engine/scroll_editor_content.rs`

**Current Good Usage:**
- Line 76: `vp_width.is_overflowed_by(caret_raw.col_index)` - checking viewport overflow
- Line 102: `caret_scr_adj.col_index.overflows(line_display_width)` - checking line width overflow
- Line 205: `caret_raw.col_index.underflows(safe_zone_start)` - checking safe zone underflow
- Line 413: `caret.row_index.overflows(viewport_height)` - checking viewport height overflow

**Refactoring Needed:**

**Lines 384-387**: Replace manual comparison with `overflows()`
```rust
// Current:
let max_row_index = buffer.get_max_row_index();
let is_past_end_of_buffer = *desired_caret_scr_adj_row_index > max_row_index;
if is_past_end_of_buffer {
    *desired_caret_scr_adj_row_index = max_row_index;
}

// Refactor to:
let buffer_height = buffer.len(); // or buffer.get_lines().len()
if desired_caret_scr_adj_row_index.overflows(buffer_height) {
    *desired_caret_scr_adj_row_index = buffer_height.convert_to_index() - row(1);
}
```

**Note**: Since `max_row_index = length - 1`, checking `index > max_row_index` is equivalent to `index >= length`, which is exactly what `overflows()` checks.

#### Task 2.4: Additional refactoring in validate_scroll_on_resize.rs
**File**: `tui/src/tui/editor/editor_engine/validate_scroll_on_resize.rs`

**Additional comparisons found:**

**Lines 73-74**: Check if caret exceeds max row
```rust
// Current:
if caret_scr_adj_row_index > max_row {
    let diff = max_row - buffer.get_caret_scr_adj().row_index;

// Could refactor to:
let buffer_height = max_row.convert_to_length(); // max_row + 1
if caret_scr_adj_row_index.overflows(buffer_height) {
    // Handle overflow
}
```

**Lines 83-84**: Check if scroll offset exceeds max row
```rust
// Current:
if scr_ofs_row_index > max_row {
    let diff = max_row - scr_ofs_row_index;

// Could refactor to:
let buffer_height = max_row.convert_to_length();
if scr_ofs_row_index.overflows(buffer_height) {
    // Handle overflow
}
```

#### Task 2.5: Review other editor module comparisons ✅
**Analysis of remaining comparison operators in editor module**

After comprehensive analysis, the following comparisons were reviewed and determined to NOT need refactoring:

**1. Zero/Position Checks (Keep as-is)**
These check if we're at a specific position, not checking range membership:

- `scroll_editor_content.rs:173`: `scr_ofs.col_index > col(0)` - Is scroll active?
- `scroll_editor_content.rs:179`: `caret_raw.col_index > col(0)` - Are we at start?
- `scroll_editor_content.rs:265`: `scr_ofs.row_index > row(0)` - Is vertical scroll active?
- `scroll_editor_content.rs:271`: `caret_raw.row_index > row(0)` - Are we at top?
- `scroll_editor_content.rs:340`: `while diff > row(0)` - Loop condition

**2. Simple Ordering Comparisons (Keep as-is)**
These are basic value comparisons, not bounds checks:

- `selection_range.rs:152`: `self.start.col_index >= scroll_offset.col_index` - Which comes first?
- `zcgb_delete_ops.rs:201`: `start_seg >= end_seg` - Is range empty?
- `zcgb_delete_ops.rs:276`: `start_index >= end_index` - Range validation
- `cur_index.rs:108`: `index > idx(0)` - Can we decrement?

**3. Capacity/Size Checks (Keep as-is)**
These are checking if we need to grow buffers, not range membership:

- `zcgb_insert_ops.rs:218`: `required_capacity > line_info.capacity` - Need to extend?
- `zcgb_insert_ops.rs:226`: `required_capacity > line_info.capacity` - Recheck after attempt
- `zcgb_line.rs:337`: `seg.display_width > width(1)` - Is segment multi-column?

**4. Search/Algorithm Logic (Keep as-is)**
These are part of search algorithms, not bounds checks:

- `zcgb_line_metadata.rs:188-189`: Byte range check in search loop
- `zcgb_line_metadata.rs:338`: `segment.start_display_col_index > col_index` - Finding first segment to right
- `zcgb_basic_ops.rs:405`: `current_col >= target_col` - Have we reached target?

**5. Test Assertions (Not production code)**
Many comparisons are in test code verifying behavior - these don't need refactoring.

**Conclusion**:
All remaining comparison operators in the editor module are appropriate single-point comparisons or algorithmic logic that would not benefit from our type-safe bounds checking methods. They are clearer and more efficient as direct comparisons.

### Phase 3: Offscreen Buffer Documentation

#### Task 3.1: Document scroll region operations
**File**: `tui/src/tui/terminal_lib_backends/offscreen_buffer/vt_100_ansi_impl/impl_scroll_ops.rs`

Add clarifying comments:
```rust
// Line 71:
// Check if we're at the bottom of the scroll region
// underflows() returns true when current_row < scroll_bottom_boundary
if current_row.underflows(scroll_bottom_boundary) {
    // Not at scroll region bottom - safe to move cursor down
    self.cursor_down(RowHeight::from(1));
    Ok(())
} else {
    // At scroll region bottom - need to scroll buffer content up
    self.scroll_buffer_up()
}

// Line 125:
// Check if we're at the top of the scroll region
// underflows() returns true when scroll_top_boundary < current_row
if scroll_top_boundary.underflows(current_row) {
    // Not at scroll region top - safe to move cursor up
    self.cursor_up(RowHeight::from(1));
    Ok(())
} else {
    // At scroll region top - need to scroll buffer content down
    self.scroll_buffer_down()
}
```

#### Task 3.2: Document cursor clamping operations
**File**: `tui/src/tui/terminal_lib_backends/offscreen_buffer/vt_100_ansi_impl/impl_cursor_ops.rs`

Add documentation around `.clamp()` usage:
```rust
// Document that .clamp() is correct here because we're constraining
// the cursor position to stay within scroll region boundaries
let clamped_row = row.clamp(scroll_top_boundary, scroll_bottom_boundary);
```

### Phase 4: VT-100 Parser Module Simplification

#### Task 4.1: Apply semantic aliases
**File**: `tui/src/core/pty_mux/vt_100_ansi_parser/operations/line_ops.rs`

Update lines 105 and 153 to use semantic alias:
```rust
// Current (already updated to use check_inclusive_range_bounds):
match row_index.check_inclusive_range_bounds(scroll_top, scroll_bottom) {
    ArrayAccessBoundsStatus::Within => { /* ... */ }
    _ => { return; }
}

// After Phase 1.2 rename:
match row_index.check_range_bounds_inclusive_end(scroll_top, scroll_bottom) {
    ArrayAccessBoundsStatus::Within => { /* ... */ }
    _ => { return; }
}

// Final simplification using semantic alias:
if !row_index.is_in_scroll_region(scroll_top, scroll_bottom) {
    return;
}
// Continue with operation...
```

### Phase 5: Search for Additional Patterns

Search the codebase for manual range checks to replace:

```bash
# Search for inclusive range patterns
rg "(\w+)\s*>=\s*(\w+)\s*&&\s*\1\s*<=\s*(\w+)" --type rust

# Search for exclusive range patterns
rg "(\w+)\s*>=\s*(\w+)\s*&&\s*\1\s*<\s*(\w+)" --type rust

# Search for inverted range checks
rg "(\w+)\s*<\s*(\w+)\s*\|\|\s*\1\s*>\s*(\w+)" --type rust
```

Replace found patterns with appropriate bounds checking methods or semantic aliases.

### Phase 6: Documentation Updates

#### Task 6.1: Update CLAUDE.md
**File**: `/home/nazmul/github/r3bl-open-core/CLAUDE.md`

Update the "Use strong type safety in the codebase for bounds checking" section:
1. Replace `check_bounds_range` with `check_range_bounds_exclusive_end`
2. Replace `check_inclusive_range_bounds` with `check_range_bounds_inclusive_end`
3. Add semantic aliases documentation
4. Update all code examples

#### Task 6.2: Update type-safe-bounds-check.md
**File**: `/home/nazmul/github/r3bl-open-core/.claude/commands/type-safe-bounds-check.md`

Update with new method names and add comprehensive examples of semantic aliases.

## Testing Requirements

Run after each phase:
```bash
# Run all tests
cargo nextest run

# Run specific test suites
cargo test --package tui test_line_ops
cargo test --package tui test_scroll_ops
cargo test --package tui validate_scroll
cargo test --package tui editor

# Check compilation
cargo check
cargo build

# Check code quality
cargo clippy --all-targets
```

## Implementation Guidelines

### Choosing the Right Method

| Use Case | Method | Range Type | Example |
|----------|--------|------------|---------|
| Array/Buffer Access | `overflows()` | `[0, length)` | `buffer[index]` |
| Cursor Position | `check_cursor_position_bounds()` | `[0, length]` | Cursor at end of line |
| Viewport/Window | `check_range_bounds_exclusive_end()` or `is_in_viewport()` | `[start, start+size)` | Visible rows/columns |
| Scroll Regions | `check_range_bounds_inclusive_end()` or `is_in_scroll_region()` | `[min, max]` | VT-100 scroll bounds |
| Text Selection | `check_range_bounds_inclusive_end()` or `is_in_selection_range()` | `[start, end]` | Selected text range |

### Import Requirements

```rust
use crate::core::units::bounds_check::{IndexMarker, ArrayAccessBoundsStatus};
```

## Success Criteria

- [x] All instances of `check_bounds_range` renamed to `check_range_bounds_exclusive_end`
- [x] All instances of `check_inclusive_range_bounds` renamed to `check_range_bounds_inclusive_end`
- [x] Semantic aliases added to IndexMarker trait
- [x] All identified manual comparisons replaced with type-safe methods
- [x] Documentation updated with new method names
- [x] All tests passing without modification
- [x] No new clippy warnings
- [x] Code is more self-documenting with clearer intent

## Progress Tracking

### Phase 1: Core Improvements ✅
- [x] Task 1.1: Rename check_bounds_range
- [x] Task 1.2: Rename check_inclusive_range_bounds
- [x] Task 1.3: Add semantic aliases to IndexMarker trait

### Phase 2: Editor Module ✅
- [x] Task 2.1: Update validate_scroll_on_resize.rs (4 locations total)
- [x] Task 2.2: Refactor selection_range.rs (locate_column method)
- [x] Task 2.3: Refactor scroll_editor_content.rs (line 385)
- [x] Task 2.5: Review other editor module comparisons ✅

### Phase 3: Offscreen Buffer Module ✅
- [x] Task 3.1: Review existing documentation (Found well-documented and properly using type-safe bounds checking)

### Phase 4: VT-100 Parser Module ✅
- [x] Task 4.1: Apply semantic aliases (Replaced complex pattern matching with `is_in_scroll_region()`)

### Phase 5: Search for Additional Patterns ✅
- [x] Search and refactor manual range checks (Comprehensive analysis completed)

### Phase 6: Documentation Updates ✅
- [x] Task 6.1: Update CLAUDE.md
- [x] Task 6.2: Update type-safe-bounds-check.md

## Refactoring Summary

### Core Infrastructure Changes
1. **Method Renaming**: Renamed core methods for maximum clarity:
   - `check_bounds_range` → `check_range_bounds_exclusive_end`
   - `check_inclusive_range_bounds` → `check_range_bounds_inclusive_end`

2. **Semantic Aliases**: Added 4 domain-specific boolean methods to `IndexMarker` trait:
   - `is_in_viewport()` - For UI rendering and viewport calculations
   - `is_in_inclusive_range()` - General-purpose inclusive range checking
   - `is_in_scroll_region()` - VT-100 terminal scroll region operations
   - `is_in_selection_range()` - Text selection operations

### Refactored Locations
1. **validate_scroll_on_resize.rs**: 4 locations updated with `overflows()` and `is_in_viewport()`
2. **selection_range.rs**: `locate_column()` method uses pattern matching with `check_range_bounds_inclusive_end()`
3. **scroll_editor_content.rs**: Line 385 uses `overflows()` instead of manual `>` comparison
4. **line_ops.rs**: 2 VT-100 scroll region checks now use `is_in_scroll_region()` semantic alias

### Key Improvements
- **Type Safety**: Eliminated manual index/length comparisons prone to off-by-one errors
- **Self-Documenting Code**: Method names clearly express intent (viewport vs scroll region vs selection)
- **Simplified Logic**: Replaced complex pattern matching with simple boolean checks where appropriate
- **Consistent Patterns**: Unified approach to bounds checking across all modules

### Implementation Notes
<!-- Add discoveries, tricky cases, and patterns found during implementation -->

## References

- Bounds checking utilities: `tui/src/core/units/bounds_check/`
- VT-100 operations: `tui/src/core/pty_mux/vt_100_ansi_parser/operations/`
- Editor viewport: `tui/src/tui/editor/editor_engine/`
- Example of correct usage: `line_ops.rs` lines 105, 153 (after refactoring)