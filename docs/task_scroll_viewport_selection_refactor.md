# Task: Refactor for type-safe bounds checking for scroll, selection, viewport

## 🚨 IMPORTANT USAGE INSTRUCTIONS 🚨

**This file serves as your "external memory" and "external todo list" for this refactoring task.**

### How to Use This File:
1. **Keep this file updated** as you make changes - track your progress in real-time
2. **Use as external memory** - document any discoveries, patterns, or tricky cases you encounter
3. **Update the Progress Tracking section** after completing each file
4. **Add implementation notes** when you find interesting patterns or solutions
5. **This is your working document** - treat it as an active part of your workflow

### 🔒 CRITICAL CONSTRAINTS - READ FIRST 🔒

**BEHAVIOR PRESERVATION IS MANDATORY:**
- ❌ **NO functionality changes** - preserve exact current behavior
- ❌ **NO test changes** - tests define the specification and must remain unchanged
- ✅ **DO run tests after each change** - verify nothing breaks with `cargo nextest run`
- ✅ **Goal: Type safety + maintainability** while keeping identical behavior
- ✅ **Tests are your safety net** - they validate that refactoring preserves semantics

**If any test fails after your changes, you must fix the refactoring, not the test.**

<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

- [Overview](#overview)
- [Background: Bounds Checking Patterns](#background-bounds-checking-patterns)
  - [Pattern 1: Array Access Bounds Checking](#pattern-1-array-access-bounds-checking)
  - [Pattern 2: Cursor Position Bounds Checking](#pattern-2-cursor-position-bounds-checking)
  - [Pattern 3: Range Membership Checking](#pattern-3-range-membership-checking)
    - [Pattern 3a: Viewport/Window Bounds (Exclusive)](#pattern-3a-viewportwindow-bounds-exclusive)
    - [Pattern 3b: Inclusive Range Bounds](#pattern-3b-inclusive-range-bounds)
- [Completed Work](#completed-work)
  - [Added `check_inclusive_range_bounds()` method](#added-check_inclusive_range_bounds-method)
  - [Refactored VT-100 line operations](#refactored-vt-100-line-operations)
- [Remaining Work](#remaining-work)
  - [Phase 1: Core Improvements to Bounds Checking Methods](#phase-1-core-improvements-to-bounds-checking-methods)
    - [Task 1.1: Rename `check_bounds_range` to `check_viewport_bounds`](#task-11-rename-check_bounds_range-to-check_viewport_bounds)
    - [Task 1.2: Add Semantic Aliases](#task-12-add-semantic-aliases)
  - [Phase 2: Editor Module Refactoring](#phase-2-editor-module-refactoring)
    - [Task 2.1: Update validate_scroll_on_resize.rs](#task-21-update-validate_scroll_on_resizers)
    - [Task 2.2: Refactor selection_range.rs](#task-22-refactor-selection_rangers)
    - [Task 2.3: Review scroll_editor_content.rs](#task-23-review-scroll_editor_contentrs)
  - [Phase 3: Offscreen Buffer Module Updates](#phase-3-offscreen-buffer-module-updates)
    - [Task 3.1: Document scroll region operations](#task-31-document-scroll-region-operations)
    - [Task 3.2: Document cursor clamping operations](#task-32-document-cursor-clamping-operations)
  - [Phase 4: VT-100 Parser Module Simplification](#phase-4-vt-100-parser-module-simplification)
    - [Task 4.1: Apply semantic aliases](#task-41-apply-semantic-aliases)
  - [Phase 5: Search for Additional Patterns](#phase-5-search-for-additional-patterns)
  - [Phase 6: Documentation Updates](#phase-6-documentation-updates)
    - [Task 6.1: Update CLAUDE.md](#task-61-update-claudemd)
- [Testing Requirements](#testing-requirements)
- [Implementation Guidelines](#implementation-guidelines)
  - [Choosing the Right Method](#choosing-the-right-method)
  - [Import Requirements](#import-requirements)
  - [Verification Steps](#verification-steps)
  - [Edge Cases and Special Handling](#edge-cases-and-special-handling)
    - [Pattern: Offset-Based Range Checks](#pattern-offset-based-range-checks)
    - [Pattern: Mixed Comparison Operators](#pattern-mixed-comparison-operators)
    - [Pattern: Converting Between Representations](#pattern-converting-between-representations)
    - [Critical: Zero-Based vs One-Based Indexing](#critical-zero-based-vs-one-based-indexing)
    - [Common Pitfalls to Avoid](#common-pitfalls-to-avoid)
    - [Negated Range Checks](#negated-range-checks)
    - [Consistency Guidelines](#consistency-guidelines)
    - [Testing Boundary Conditions](#testing-boundary-conditions)
  - [Code Style Examples](#code-style-examples)
- [Success Criteria](#success-criteria)
- [Priority Order](#priority-order)
- [Notes for Developer](#notes-for-developer)
- [Detailed Analysis Summary](#detailed-analysis-summary)
  - [Current State of Bounds Checking in Codebase](#current-state-of-bounds-checking-in-codebase)
    - [✅ Already Using `check_inclusive_range_bounds` (Correct)](#-already-using-check_inclusive_range_bounds-correct)
    - [🔄 Areas That Need `check_bounds_range` → `check_viewport_bounds` Rename](#-areas-that-need-check_bounds_range-%E2%86%92-check_viewport_bounds-rename)
    - [🎯 High-Value Refactoring Opportunities](#-high-value-refactoring-opportunities)
    - [📝 Documentation Improvements Needed](#-documentation-improvements-needed)
  - [Key Insight: Semantic Separation](#key-insight-semantic-separation)
- [Progress Tracking](#progress-tracking)
  - [Phase 1: Core Improvements ⏳](#phase-1-core-improvements-)
  - [Phase 2: Editor Module 🔲](#phase-2-editor-module-)
  - [Phase 3: Offscreen Buffer Module 🔲](#phase-3-offscreen-buffer-module-)
  - [Phase 4: VT-100 Parser Module 🔲](#phase-4-vt-100-parser-module-)
  - [Phase 5: Search for Additional Patterns 🔲](#phase-5-search-for-additional-patterns-)
  - [Phase 6: Documentation Updates 🔲](#phase-6-documentation-updates-)
  - [Implementation Notes](#implementation-notes)
- [References](#references)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

## Overview
This task involves improving the type safety and clarity of bounds checking throughout the r3bl-open-core codebase by:
1. Renaming `check_bounds_range` to `check_viewport_bounds` for clarity
2. Adding semantic aliases for common use cases
3. Applying the new `check_inclusive_range_bounds` method where appropriate
4. Ensuring consistent patterns across editor, offscreen_buffer, and VT-100 parser modules

## Background: Bounds Checking Patterns

The codebase uses distinct patterns for different types of bounds checking:

### Pattern 1: Array Access Bounds Checking
- **Method**: `index.overflows(length)` or `index.check_array_access_bounds(length)`
- **Range**: `[0, length)` - exclusive upper bound
- **Use Case**: Direct array/buffer access where index must be < length
- **Example**: `buffer[row_index]` requires `row_index < buffer_height`

### Pattern 2: Cursor Position Bounds Checking
- **Method**: `cursor_pos.check_cursor_position_bounds(content_length)`
- **Range**: `[0, length]` - inclusive upper bound
- **Use Case**: Cursor positioning where cursor can be placed at end for insertion
- **Example**: Cursor at end of line for appending text

### Pattern 3: Range Membership Checking

#### Pattern 3a: Viewport/Window Bounds (Exclusive)
- **Current Method**: `check_bounds_range(start, size)`
- **New Name**: `check_viewport_bounds(start, size)`
- **Range**: `[start, start+size)` - exclusive upper bound
- **Use Case**: Checking if something is visible in a viewport or window
- **Example**: Viewport at row 5 with height 10 covers rows [5, 15]

#### Pattern 3b: Inclusive Range Bounds
- **Method**: `check_inclusive_range_bounds(min, max)`
- **Range**: `[min, max]` - inclusive both bounds
- **Use Case**: VT-100 scroll regions, text selections
- **Example**: Scroll region from row 2 to row 8 includes both rows 2 and 8

## Completed Work

### Added `check_inclusive_range_bounds()` method
**File**: `tui/src/core/units/bounds_check/length_and_index_markers.rs`

```rust
fn check_inclusive_range_bounds(&self, min_index: Self, max_index: Self) -> ArrayAccessBoundsStatus
where
    Self: PartialOrd + Copy,
{
    if *self < min_index {
        ArrayAccessBoundsStatus::Underflowed
    } else if *self > max_index {
        ArrayAccessBoundsStatus::Overflowed
    } else {
        ArrayAccessBoundsStatus::Within
    }
}
```

### Refactored VT-100 line operations
**File**: `tui/src/core/pty_mux/vt_100_ansi_parser/operations/line_ops.rs`
- Lines 105, 153: Now using `check_inclusive_range_bounds` for scroll region checks

## Remaining Work

### Phase 1: Core Improvements to Bounds Checking Methods

#### Task 1.1: Rename `check_bounds_range` to `check_viewport_bounds`
**File**: `tui/src/core/units/bounds_check/length_and_index_markers.rs`

1. Rename the method to clarify its purpose:
```rust
// Old name
fn check_bounds_range(&self, start_pos: Self, width_or_height: L) -> ArrayAccessBoundsStatus

// New name
fn check_viewport_bounds(&self, start_pos: Self, width_or_height: L) -> ArrayAccessBoundsStatus
```

2. Update documentation to clarify exclusive upper bound:
```rust
/// Checks if this index is within a viewport/window range [start, start+size).
/// The upper bound is EXCLUSIVE, making this suitable for viewport and window bounds.
///
/// # Examples
/// - Viewport at row 5, height 10 = rows [5, 15) - row 15 is NOT included
/// - Window at col 10, width 20 = cols [10, 30) - col 30 is NOT included
```

3. Find and update all usages across the codebase (use grep/search for `check_bounds_range`)

#### Task 1.2: Add Semantic Aliases
**File**: `tui/src/core/units/bounds_check/length_and_index_markers.rs`

**Important**: Task 1.1 must be completed before Task 1.2, as the semantic aliases depend on `check_viewport_bounds` existing.

Add these helper methods to make code self-documenting:

**Note**: These are extension methods added directly to the concrete types, not to the IndexMarker trait itself.

```rust
impl RowIndex {
    /// Check if this row is visible in a viewport
    /// Uses exclusive upper bound: [viewport_start, viewport_start + viewport_height)
    #[inline]
    pub fn is_in_viewport(&self, viewport_start: RowIndex, viewport_height: RowHeight) -> bool {
        matches!(
            self.check_viewport_bounds(viewport_start, viewport_height),
            ArrayAccessBoundsStatus::Within
        )
    }

    /// Check if this row is within a scroll region
    /// Uses inclusive bounds: [top, bottom]
    #[inline]
    pub fn is_in_scroll_region(&self, top: RowIndex, bottom: RowIndex) -> bool {
        matches!(
            self.check_inclusive_range_bounds(top, bottom),
            ArrayAccessBoundsStatus::Within
        )
    }
}

impl ColIndex {
    /// Check if this column is visible in a viewport
    /// Uses exclusive upper bound: [viewport_start, viewport_start + viewport_width)
    #[inline]
    pub fn is_in_viewport(&self, viewport_start: ColIndex, viewport_width: ColWidth) -> bool {
        matches!(
            self.check_viewport_bounds(viewport_start, viewport_width),
            ArrayAccessBoundsStatus::Within
        )
    }

    /// Check if this column is within a selection range
    /// Uses inclusive bounds: [start, end]
    #[inline]
    pub fn is_in_selection_range(&self, start: ColIndex, end: ColIndex) -> bool {
        matches!(
            self.check_inclusive_range_bounds(start, end),
            ArrayAccessBoundsStatus::Within
        )
    }
}
```

### Phase 2: Editor Module Refactoring

#### Task 2.1: Update validate_scroll_on_resize.rs
**File**: `tui/src/tui/editor/editor_engine/validate_scroll_on_resize.rs`

**Location 1**: Lines 96-97 (Vertical viewport check)
```rust
// Current code:
let is_within = caret_scr_adj_row_index >= scr_ofs_row_index
    && caret_scr_adj_row_index <= (scr_ofs_row_index + vp_height);

// Refactor to:
use crate::core::units::bounds_check::{IndexMarker, ArrayAccessBoundsStatus};

let start_idx = scr_ofs_row_index;
let end_idx = scr_ofs_row_index + vp_height;
let is_within = matches!(
    caret_scr_adj_row_index.check_inclusive_range_bounds(start_idx, end_idx),
    ArrayAccessBoundsStatus::Within
);
```

**Note**: Using inclusive range bounds preserves the cursor-at-edge behavior while maintaining the viewport offset semantics.

**Location 2**: Lines 153-154 (Horizontal viewport check)
```rust
// Current code:
let is_within = if caret_scr_adj_col_index >= scr_ofs_col_index
    && caret_scr_adj_col_index < scr_ofs_col_index + viewport_width

// Refactor to:
let is_within = if caret_scr_adj_col_index.is_in_viewport(scr_ofs_col_index, viewport_width)
```

#### Task 2.2: Refactor selection_range.rs
**File**: `tui/src/tui/editor/editor_buffer/selection_range.rs`

This file has two methods to review:

**Method 1: `locate_scroll_offset_col`** (lines 148-157)

**Purpose**: Determines if a scroll offset position is before (Underflow) or after (Overflow) the selection range start. This is used to decide how to clip the selection when rendering with horizontal scrolling.

**Current implementation**:
```rust
pub fn locate_scroll_offset_col(
    &self,
    scroll_offset: ScrOfs,
) -> ScrollOffsetColLocationInRange {
    if self.start.col_index >= scroll_offset.col_index {
        ScrollOffsetColLocationInRange::Underflow
    } else {
        ScrollOffsetColLocationInRange::Overflow
    }
}
```

**Recommendation**: **Keep as-is**. This is a position comparison against a single point (selection start), not a range membership check, so our bounds checking utilities don't apply here.

**Method 2: `locate_column`** (lines 224-232) - **REQUIRES REFACTORING**

**Purpose**: Check if a column position is within a selection range [start, end) with EXCLUSIVE upper bound.

**Current implementation**:
```rust
pub fn locate_column(&self, caret: CaretScrAdj) -> CaretLocationInRange {
    if caret.col_index < self.start.col_index {
        CaretLocationInRange::Underflow
    } else if caret.col_index >= self.end.col_index {
        CaretLocationInRange::Overflow
    } else {
        CaretLocationInRange::Contained
    }
}
```

**Refactor to**:
```rust
use crate::core::units::bounds_check::{IndexMarker, ArrayAccessBoundsStatus};

pub fn locate_column(&self, caret: CaretScrAdj) -> CaretLocationInRange {
    let size = width(*(self.end.col_index - self.start.col_index));
    match caret.col_index.check_viewport_bounds(self.start.col_index, size) {
        ArrayAccessBoundsStatus::Underflowed => CaretLocationInRange::Underflow,
        ArrayAccessBoundsStatus::Overflowed => CaretLocationInRange::Overflow,
        ArrayAccessBoundsStatus::Within => CaretLocationInRange::Contained,
    }
}
```

**Note**: This refactoring is required for consistency with the rest of the codebase's type-safe bounds checking approach.

#### Task 2.3: Review scroll_editor_content.rs
**File**: `tui/src/tui/editor/editor_engine/scroll_editor_content.rs`

Search for viewport and scroll-related comparisons that could use the new semantic aliases.
Look for patterns like:
- Manual range checks that could use `is_in_viewport`
- Scroll boundary checks that might benefit from clearer semantics

### Phase 3: Offscreen Buffer Module Updates

#### Task 3.1: Document scroll region operations
**File**: `tui/src/tui/terminal_lib_backends/offscreen_buffer/vt_100_ansi_impl/impl_scroll_ops.rs`

Review lines 71 and 125 where manual comparisons are used:
```rust
// Line 71: Check if at bottom of scroll region
if current_row.underflows(scroll_bottom_boundary) {
    // This is correct - it means current_row < scroll_bottom_boundary
    // Add comment explaining the logic
}

// Line 125: Check if at top of scroll region
if scroll_top_boundary.underflows(current_row) {
    // This is correct - it means scroll_top_boundary < current_row
    // Add comment explaining the logic
}
```

#### Task 3.2: Document cursor clamping operations
**File**: `tui/src/tui/terminal_lib_backends/offscreen_buffer/vt_100_ansi_impl/impl_cursor_ops.rs`

Add documentation to clarify when to use `.clamp()` vs bounds checking:
```rust
// Around line 122
// Document that .clamp() is correct here because we're constraining
// the cursor position to stay within scroll region boundaries
let clamped_row = row.clamp(scroll_top_boundary, scroll_bottom_boundary);
```

### Phase 4: VT-100 Parser Module Simplification

#### Task 4.1: Apply semantic aliases
**File**: `tui/src/core/pty_mux/vt_100_ansi_parser/operations/line_ops.rs`

Update lines 105 and 153 to use semantic aliases:
```rust
// Current (already refactored to use check_inclusive_range_bounds):
match row_index.check_inclusive_range_bounds(scroll_top, scroll_bottom) {
    ArrayAccessBoundsStatus::Within => { /* ... */ }
    _ => { return; }
}

// Can simplify to:
if !row_index.is_in_scroll_region(scroll_top, scroll_bottom) {
    return;
}
// Continue with operation...
```

### Phase 5: Search for Additional Patterns

Search the entire codebase for patterns that could benefit from refactoring:

1. **Search patterns to find:**

   **Note**: These are conceptual patterns to illustrate what to look for. The actual regex syntax may need adjustment for your search tool (e.g., ripgrep may not support backreferences).

   ```bash
   # Inclusive range checks
   rg "(\w+)\s*>=\s*(\w+)\s*&&\s*\1\s*<=\s*(\w+)" --type rust

   # Exclusive range checks
   rg "(\w+)\s*>=\s*(\w+)\s*&&\s*\1\s*<\s*(\w+)" --type rust

   # Inverted range checks
   rg "(\w+)\s*<\s*(\w+)\s*\|\|\s*\1\s*>\s*(\w+)" --type rust
   ```

2. **Key areas to check:**
   - Selection range handling in editor modules
   - Additional viewport calculations
   - Any remaining scroll region checks
   - Window boundary checks in terminal modules

### Phase 6: Documentation Updates

#### Task 6.1: Update CLAUDE.md
**File**: `/home/nazmul/github/r3bl-open-core/CLAUDE.md`

Update the existing "Use strong type safety in the codebase for bounds checking" section to:

1. **Replace references to `check_bounds_range`** with `check_viewport_bounds`
2. **Add the new semantic aliases** to the existing pattern documentation
3. **Update the Pattern 3 examples** to show both the core methods and semantic aliases

**Specific changes needed:**
- Line ~13: Update "Pattern 3: Range Membership Checking" section
- Add documentation for the new semantic aliases
- Update all code examples that use `check_bounds_range`

**Add this new section:**
```markdown
### Bounds Checking Patterns Summary

| Pattern | Method | Range | Use Case |
|---------|--------|-------|----------|
| Array Access | `overflows()` | [0, length) | Array indexing |
| Cursor Position | `check_cursor_position_bounds()` | [0, length] | Cursor at end |
| Viewport | `check_viewport_bounds()` | [start, start+size) | Windows/viewports |
| Inclusive Range | `check_inclusive_range_bounds()` | [min, max] | Scroll regions, selections |

### Semantic Aliases for Better Code Readability

Use these self-documenting methods when the intent is clear:

```rust
// Viewport checking (exclusive upper bound)
if row.is_in_viewport(viewport_start, viewport_height) { /* visible */ }
if col.is_in_viewport(viewport_start, viewport_width) { /* visible */ }

// Scroll region checking (inclusive bounds)
if row.is_in_scroll_region(scroll_top, scroll_bottom) { /* in region */ }

// Selection range checking (inclusive bounds)
if col.is_in_selection_range(selection_start, selection_end) { /* selected */ }
```

**When to use semantic aliases vs core methods:**
- Use semantic aliases when the intent is clear and code readability is the priority
- Use core methods with pattern matching when you need to handle all bounds cases (Within/Underflowed/Overflowed)
```

#### Task 6.2: Update type-safe-bounds-check.md command file
**File**: `/home/nazmul/github/r3bl-open-core/.claude/commands/type-safe-bounds-check.md`

This command file needs comprehensive updates to reflect the new patterns:

1. **Update Pattern 3 description** to distinguish between 3a (viewport) and 3b (inclusive)
2. **Replace all instances of `check_bounds_range`** with `check_viewport_bounds`
3. **Add documentation for semantic aliases**
4. **Update examples** to show both approaches (core methods + semantic aliases)

**Specific sections to update:**
- Pattern descriptions at the top
- Code examples throughout the file
- Implementation guidance
- Add new section about semantic aliases with examples

**New content to add:**
```markdown
## Pattern 3a: Viewport Bounds Checking (Exclusive Upper Bound)
- **Method**: `check_viewport_bounds(start, size)`
- **Semantic Alias**: `is_in_viewport(start, size)`
- **Range**: [start, start+size) - exclusive upper bound
- **Use**: Checking if something is visible in a viewport/window

## Pattern 3b: Inclusive Range Checking
- **Method**: `check_inclusive_range_bounds(min, max)`
- **Semantic Alias**: `is_in_scroll_region(top, bottom)` or `is_in_selection_range(start, end)`
- **Range**: [min, max] - inclusive both bounds
- **Use**: VT-100 scroll regions, text selections

## Semantic Aliases
Prefer semantic aliases when intent is clear:
- `is_in_viewport()` - for viewport/window visibility
- `is_in_scroll_region()` - for VT-100 scroll regions
- `is_in_selection_range()` - for text selections
```

## Testing Requirements

After making changes:

1. **Run existing tests:**
   ```bash
   cargo test --package tui
   ```

2. **Verify VT-100 conformance:**
   ```bash
   cargo test test_line_ops
   cargo test test_scroll_ops
   cargo test validate_scroll
   ```

3. **Check for compilation issues:**
   ```bash
   cargo check
   cargo build
   ```

4. **Run clippy for code quality:**
   ```bash
   cargo clippy --all-targets
   ```

5. **Run specific editor tests:**
   ```bash
   cargo test --package tui editor
   ```

## Implementation Guidelines

### Choosing the Right Method

1. **Array/Buffer Access** → Use `overflows()` or `check_array_access_bounds()`
   - When: Accessing array elements where index must be < length
   - Example: `buffer[index]`

2. **Cursor Positioning** → Use `check_cursor_position_bounds()`
   - When: Cursor can be at the end position for insertion
   - Example: Text cursor at end of line

3. **Viewport/Window** → Use `check_viewport_bounds()` or `is_in_viewport()`
   - When: Checking visibility in a window with size
   - Example: Is row visible in current viewport?

4. **Inclusive Ranges** → Use `check_inclusive_range_bounds()` or semantic aliases
   - When: Both boundaries are included in the range
   - Example: VT-100 scroll regions, text selections

### Import Requirements

```rust
use crate::core::units::bounds_check::{IndexMarker, ArrayAccessBoundsStatus};
```

### Verification Steps

Before making any change:
1. **Identify current behavior**: Is the bound inclusive or exclusive?
2. **Run tests first**: `cargo nextest run` - ensure they pass
3. **Make the change**: Apply the refactoring
4. **Run tests again**: Verify nothing breaks
5. **If tests fail**: The refactoring changed behavior - fix the refactoring, not the test

### Edge Cases and Special Handling

#### Pattern: Offset-Based Range Checks
**Problem**: Checking if position is within a viewport that starts at a non-zero offset (like Task 2.1).

```rust
// Current code pattern:
let is_within = caret_row >= viewport_start && caret_row <= viewport_start + viewport_height;

// Solution: Calculate explicit boundaries
let start_idx = viewport_start;
let end_idx = viewport_start + viewport_height;
let is_within = matches!(
    caret_row.check_inclusive_range_bounds(start_idx, end_idx),
    ArrayAccessBoundsStatus::Within
);
```

#### Pattern: Mixed Comparison Operators
**Problem**: Handling ranges with different inclusivity on each end.

```rust
// Exclusive both ends: x > min && x < max
// Convert to: x >= (min + 1) && x <= (max - 1)
let adjusted_min = min + row(1);
let adjusted_max = max - row(1);
if x.check_inclusive_range_bounds(adjusted_min, adjusted_max) { /* ... */ }

// Exclusive start, inclusive end: x > min && x <= max
// Convert to: x >= (min + 1) && x <= max
let adjusted_min = min + row(1);
if x.check_inclusive_range_bounds(adjusted_min, max) { /* ... */ }
```

#### Pattern: Converting Between Representations
**Problem**: Converting between endpoint [start, end] and size-based [start, start+size) representations.

```rust
// From endpoints to size-based (for check_viewport_bounds):
let start = selection.start_col();
let size = width(*(selection.end_col() - selection.start_col()));
pos.check_viewport_bounds(start, size)

// From size-based to endpoints (for check_inclusive_range_bounds):
let start = viewport_start;
let end = viewport_start + viewport_size - 1; // Subtract 1 for inclusive
pos.check_inclusive_range_bounds(start, end)
```

#### Critical: Zero-Based vs One-Based Indexing
**Problem**: VT-100 operations use 1-based coordinates but our indices are 0-based.

```rust
// When working with VT-100 TermRow/TermCol:
// Always convert to 0-based before using our bounds checking methods
let zero_based_row = term_row.to_zero_based()?;
let scroll_region_0based = scroll_top.to_zero_based()?..=scroll_bottom.to_zero_based()?;

// Use converted values with our methods
zero_based_row.check_inclusive_range_bounds(region_start, region_end)
```

#### Common Pitfalls to Avoid

**1. Integer Overflow in Size Calculations**
```rust
// Bad: Could overflow
let size = width(end_index - start_index);

// Good: Check for valid range first
if end_index < start_index {
    return; // Invalid range
}
let size = width(*(end_index - start_index));
```

**2. Type Confusion Between Index and Length**
```rust
// Bad: Mixing index and length types
row_index.check_viewport_bounds(start_row, end_row); // Wrong: end_row is RowIndex, not RowHeight

// Good: Use correct types
row_index.check_viewport_bounds(start_row, row_height); // Correct: row_height is RowHeight
```

**3. Empty or Invalid Ranges**
```rust
// Always validate ranges before processing
if start > end {
    return; // Handle invalid range
}
if start == end {
    // Handle empty range case
}
```

#### Negated Range Checks
```rust
// Instead of: !(x >= min && x <= max)
// Use pattern matching for clarity:
match x.check_inclusive_range_bounds(min, max) {
    ArrayAccessBoundsStatus::Within => { /* in range */ }
    ArrayAccessBoundsStatus::Underflowed | ArrayAccessBoundsStatus::Overflowed => {
        // Not in range - handle both cases
    }
}

// Or use semantic aliases for simple cases:
if !x.is_in_scroll_region(min, max) {
    // Not in scroll region
}
```

#### Consistency Guidelines
**Always prefer type-safe methods** for all bounds checking:
- Improves code maintainability and readability
- Prevents off-by-one errors
- Ensures consistent patterns across the codebase
- Can profile later if performance becomes an issue

**Only keep manual checks** when:
- The pattern doesn't fit any of our standard methods (document why)
- Simple position comparisons against a single point (not ranges)

#### Testing Boundary Conditions
**Always test these edge cases:**
```rust
#[test]
fn test_boundary_conditions() {
    // Test at exact boundaries
    assert!(pos.is_in_range(min, max)); // pos == min
    assert!(pos.is_in_range(min, max)); // pos == max

    // Test just outside boundaries
    assert!(!pos.is_in_range(min, max)); // pos == min - 1
    assert!(!pos.is_in_range(min, max)); // pos == max + 1

    // Test empty and single-element ranges
    assert!(!pos.is_in_range(start, start)); // Empty range
    assert!(pos.is_in_range(start, start + 1)); // Single element
}
```

### Code Style Examples

**Using semantic aliases (preferred for readability):**
```rust
if row.is_in_viewport(viewport_start, viewport_height) {
    // Row is visible
}

if row.is_in_scroll_region(scroll_top, scroll_bottom) {
    // Row is within scroll region
}
```

**Using core methods with pattern matching:**
```rust
match col.check_inclusive_range_bounds(selection_start, selection_end) {
    ArrayAccessBoundsStatus::Within => { /* in selection */ }
    ArrayAccessBoundsStatus::Underflowed => { /* before selection */ }
    ArrayAccessBoundsStatus::Overflowed => { /* after selection */ }
}
```

## Success Criteria

- [ ] `check_bounds_range` renamed to `check_viewport_bounds` everywhere
- [ ] Semantic aliases added and used where they improve readability
- [ ] All identified manual comparisons replaced with type-safe methods
- [ ] Documentation updated to clarify the distinction between patterns
- [ ] All tests passing
- [ ] No new clippy warnings
- [ ] Code is more self-documenting and intent is clearer

## Priority Order

1. **High Priority**: Rename `check_bounds_range` → `check_viewport_bounds`
2. **High Priority**: Add semantic aliases
3. **Medium Priority**: Update existing code to use semantic aliases
4. **Medium Priority**: Fix validate_scroll_on_resize.rs
5. **Low Priority**: Search and fix additional patterns
6. **Low Priority**: Documentation updates

## Notes for Developer

- The distinction between inclusive and exclusive bounds is critical for correctness
- Semantic aliases make the code self-documenting - prefer them over raw methods
- When unsure about which pattern to use, check the semantics: Is the upper bound included or not?
- VT-100 operations traditionally use inclusive ranges (from line X to line Y, both included)
- Viewports use exclusive upper bounds (standard practice in windowing systems)
- Test thoroughly, especially edge cases at boundaries
- Consider adding debug assertions during refactoring to verify behavior doesn't change

## Detailed Analysis Summary

### Current State of Bounds Checking in Codebase

#### ✅ Already Using `check_inclusive_range_bounds` (Correct)
- **VT-100 ANSI Parser**: `tui/src/core/pty_mux/vt_100_ansi_parser/operations/line_ops.rs`
  - Lines 105, 153: Checking if cursor is within scroll region for insert/delete operations
  - This is correct usage for VT-100 scroll regions which use inclusive bounds

#### 🔄 Areas That Need `check_bounds_range` → `check_viewport_bounds` Rename
- **Editor Module**: Multiple files use `check_bounds_range` for viewport checking
  - These are semantically correct but the name is confusing
  - Need systematic rename + update all call sites

#### 🎯 High-Value Refactoring Opportunities

1. **Editor Selection Range Logic** (`editor_buffer/selection_range.rs:148`)
   - `locate_scroll_offset_col` uses manual comparisons
   - Could benefit from type-safe inclusive range checking

2. **Editor Viewport Validation** (`validate_scroll_on_resize.rs`)
   - Lines 96-97: Manual viewport bounds checking
   - Lines 153-154: Manual horizontal viewport checking
   - Prime candidates for semantic aliases

3. **Offscreen Buffer Scroll Operations** (`impl_scroll_ops.rs`)
   - Lines 71, 125: Manual boundary comparisons that are correct but could be clearer
   - Good candidates for adding explanatory comments

#### 📝 Documentation Improvements Needed
- Current `.clamp()` usage in cursor operations is correct but undocumented
- Need clear guidelines on when to use each bounds checking pattern
- Missing examples of semantic aliases

### Key Insight: Semantic Separation
The codebase has two fundamentally different range semantics:
- **VT-100/ANSI operations**: Use inclusive ranges `[min, max]` (both endpoints included)
- **Viewport/windowing operations**: Use exclusive upper bound `[start, start+size)` (end not included)

This distinction is critical for correctness and the semantic aliases make this intent explicit.

## Progress Tracking

### Phase 1: Core Improvements ⏳
- [ ] Task 1.1: Rename `check_bounds_range` to `check_viewport_bounds`
- [ ] Task 1.2: Add semantic aliases

### Phase 2: Editor Module 🔲
- [ ] Task 2.1: Update validate_scroll_on_resize.rs
- [ ] Task 2.2: Improve selection_range.rs
- [ ] Task 2.3: Review scroll_editor_content.rs

### Phase 3: Offscreen Buffer Module 🔲
- [ ] Task 3.1: Document scroll region operations
- [ ] Task 3.2: Document cursor clamping operations

### Phase 4: VT-100 Parser Module 🔲
- [ ] Task 4.1: Apply semantic aliases

### Phase 5: Search for Additional Patterns 🔲
- [ ] Search and document findings

### Phase 6: Documentation Updates 🔲
- [ ] Task 6.1: Update CLAUDE.md
- [ ] Task 6.2: Update type-safe-bounds-check.md

### Implementation Notes
<!-- Add discoveries, tricky cases, and patterns found during implementation -->

## References

- Bounds checking utilities: `tui/src/core/units/bounds_check/`
- VT-100 operations: `tui/src/core/pty_mux/vt_100_ansi_parser/operations/`
- Editor viewport: `tui/src/tui/editor/editor_engine/`
- Example of correct usage: `line_ops.rs` lines 105, 153