---
name: check-bounds-safety
description: Apply type-safe bounds checking patterns using Index/Length types instead of usize. Use when working with arrays, buffers, cursors, viewports, or any code that handles indices and lengths.
---

# Type-Safe Bounds Checking

## When to Use

- Working with array access or buffer operations
- Implementing cursor positioning logic (text editors, terminal emulators)
- Handling viewport rendering and scrolling
- Dealing with 0-based indices vs 1-based lengths
- Validating range boundaries
- Converting VT-100 ranges to Rust ranges
- Before creating commits with bounds-sensitive code
- When user asks about "bounds checking", "type safety", "off-by-one errors", etc.

## The Problem

Raw `usize` values are ambiguous and error-prone:

```rust
// ❌ Bad - What is `x`? Index or length?
let x = 10_usize;
if x < length {  // Off-by-one error waiting to happen
    buffer[x]
}

// Is this an index (0-based) or a length (1-based)?
// The type system can't help us!
```

## The Solution

Use type-safe wrappers from `tui/src/core/units/bounds_check/`:

```rust
// ✅ Good - Types make it clear
use r3bl_tui::{idx, len, ArrayBoundsCheck};

let index = idx(10);      // Clearly an index (0-based)
let length = len(100);    // Clearly a length (1-based)

if index.overflows(length) {
    // Safely caught! Can't accidentally compare incompatible types
}
```

## Core Principles

Follow these principles when working with indices and lengths:

1. **Use Index types (0-based)** instead of `usize`
   - `RowIndex`, `ColIndex`, `Index`
   - Construct with `row()`, `col()`, `idx()`

2. **Use Length types (1-based)** instead of `usize`
   - `RowHeight`, `ColWidth`, `Length`
   - Construct with `height()`, `width()`, `len()`

3. **Type-safe comparisons**
   - Cannot compare `RowIndex` with `ColWidth` (compile error!)
   - Prevents category errors like "is row 5 < width 10?"

4. **Use `.is_zero()` for zero checks**
   - Instead of `== 0`
   - More idiomatic with newtype wrappers

## Common Imports

```rust
use std::ops::Range;
use r3bl_tui::{
    // Traits
    ArrayBoundsCheck, CursorBoundsCheck, ViewportBoundsCheck,
    RangeBoundsExt, RangeConvertExt, IndexOps, LengthOps,

    // Status enums
    ArrayOverflowResult, CursorPositionBoundsStatus,
    RangeValidityStatus, RangeBoundsResult,

    // Type constructors
    col, row, width, height, idx, len,

    // Terminal delta types (relative cursor movement)
    TermRowDelta, TermColDelta, term_row_delta, term_col_delta,
};
```

## Quick Pattern Reference

| Use Case                | Trait                 | Key Method                                   | When to Use                                                 |
| ----------------------- | --------------------- | -------------------------------------------- | ----------------------------------------------------------- |
| **Array access**        | `ArrayBoundsCheck`    | `index.overflows(length)`                    | Validating `buffer[index]` access (`index < length`)        |
| **Cursor positioning**  | `CursorBoundsCheck`   | `length.check_cursor_position_bounds(pos)`   | Text editing where cursor can be at end (`index <= length`) |
| **Viewport visibility** | `ViewportBoundsCheck` | `index.check_viewport_bounds(start, size)`   | Rendering optimization (is content on-screen?)              |
| **Range validation**    | `RangeBoundsExt`      | `range.check_range_is_valid_for_length(len)` | Iterator bounds, algorithm parameters                       |
| **Range membership**    | `RangeBoundsExt`      | `range.check_index_is_within(index)`         | VT-100 scroll regions, text selections                      |
| **Range conversion**    | `RangeConvertExt`     | `inclusive_range.to_exclusive()`             | Converting VT-100 ranges for Rust iteration                 |
| **Relative movement**   | `TermRowDelta`/`TermColDelta` | `delta.as_nonzero_u16()`               | ANSI cursor movement preventing CSI zero bug                |

## Detailed Examples

### Example 1: Array Bounds Checking

**Use `ArrayBoundsCheck` when validating buffer access.**

```rust
use r3bl_tui::{idx, len, ArrayBoundsCheck, ArrayOverflowResult};

let buffer_length = len(100);
let index = idx(50);

match index.overflows(buffer_length) {
    ArrayOverflowResult::Within => {
        // Safe to access: buffer[50]
        let value = buffer[index.value()];
    }
    ArrayOverflowResult::Overflows => {
        // Out of bounds! Handle error
        eprintln!("Index {} overflows buffer length {}", index, buffer_length);
    }
}
```

**Mathematical law:**
- For valid access: `0 <= index < length`
- Or equivalently: `index < length` (since Index is always >= 0)

### Example 2: Cursor Position Bounds

**Use `CursorBoundsCheck` for text cursor positioning.**

```rust
use r3bl_tui::{idx, len, CursorBoundsCheck, CursorPositionBoundsStatus};

let text_length = len(10);  // Text has 10 characters
let cursor = idx(10);        // Cursor at position 10 (after last char)

match text_length.check_cursor_position_bounds(cursor) {
    CursorPositionBoundsStatus::Within => {
        // Valid! Cursor CAN be at position 10 (after char 9)
        // User can insert text here
    }
    CursorPositionBoundsStatus::Overflows => {
        // Invalid cursor position
    }
}
```

**Mathematical law:**
- For valid cursor: `0 <= position <= length`
- **Note:** Cursor CAN be at `length` (after the last character)

**Key difference from array access:**
- Array access: `index < length` (strict inequality)
- Cursor position: `index <= length` (includes equality)

### Example 3: Viewport Visibility Check

**Use `ViewportBoundsCheck` to optimize rendering.**

```rust
use r3bl_tui::{idx, len, ViewportBoundsCheck};

let line_index = idx(150);      // Line 150 in document
let viewport_start = idx(100);  // Viewport starts at line 100
let viewport_size = len(50);    // Viewport shows 50 lines

if line_index.check_viewport_bounds(viewport_start, viewport_size) {
    // Line 150 is visible (100 <= 150 < 150)
    // Render this line
} else {
    // Line is off-screen, skip rendering
}
```

**Mathematical law:**
- Visible if: `viewport_start <= index < viewport_start + viewport_size`

### Example 4: Range Validation

**Use `RangeBoundsExt` to validate range boundaries.**

```rust
use r3bl_tui::{len, RangeBoundsExt, RangeValidityStatus};

let buffer_length = len(100);
let range = 10..50;  // Want to process elements 10-49

match range.check_range_is_valid_for_length(buffer_length) {
    RangeValidityStatus::Valid => {
        // Range is valid for this buffer
        for i in range {
            process(buffer[i]);
        }
    }
    RangeValidityStatus::Invalid(reason) => {
        eprintln!("Invalid range: {}", reason);
    }
}
```

### Example 5: Range Membership

**Use `RangeBoundsExt` to check if index is within a range.**

```rust
use r3bl_tui::{idx, RangeBoundsExt};

// VT-100 scroll region: lines 5-15
let scroll_region = 5..=15;  // Inclusive range
let cursor_row = idx(10);

if scroll_region.check_index_is_within(cursor_row) {
    // Cursor is within scroll region
    // Apply scroll behavior
} else {
    // Cursor outside scroll region
}
```

### Example 6: Range Conversion

**Use `RangeConvertExt` to convert inclusive to exclusive ranges.**

```rust
use r3bl_tui::RangeConvertExt;

// VT-100 uses inclusive ranges: 1..=10 means lines 1 through 10
let vt100_range = 1..=10;

// Rust iterators use exclusive ranges: 1..11
let rust_range = vt100_range.to_exclusive();

// Now can use in Rust iteration
for line in rust_range {
    process_line(line);
}
```

### Example 7: Terminal Cursor Movement (CSI Zero Guard)

**Use `TermRowDelta`/`TermColDelta` for relative cursor movement in ANSI sequences.**

The CSI zero problem: ANSI cursor movement commands interpret parameter 0 as 1:
- `CSI 0 A` (`CursorUp` with n=0) moves cursor **1 row up**, not 0
- `CSI 0 C` (`CursorForward` with n=0) moves cursor **1 column right**, not 0

```rust
use r3bl_tui::{term_row_delta, term_col_delta, CsiSequence, width};
use std::io::Write;

// Calculate cursor movement from position on 80-column terminal.
let position: u16 = 240;  // 240 chars from start
let term_width: u16 = 80;

// Calculate row and column deltas.
let rows_down = term_row_delta(position / term_width);  // 3 rows
let cols_right = term_col_delta(position % term_width); // 0 cols

// Use as_nonzero_u16() to safely emit - prevents CSI zero bug!
if let Some(n) = rows_down.as_nonzero_u16() {
    term.write_all(CsiSequence::CursorDown(n).to_string().as_bytes())?;
}
if let Some(n) = cols_right.as_nonzero_u16() {
    // This branch is NOT taken for position 240 (cols = 0)
    // Without this guard, CursorForward(0) would move 1 column right!
    term.write_all(CsiSequence::CursorForward(n).to_string().as_bytes())?;
}
```

**Converting from size types:**

```rust
use r3bl_tui::{width, height, TermColDelta, TermRowDelta};

let col_width = width(40);
let delta: TermColDelta = col_width.into();  // Uses From<ColWidth>

let row_height = height(10);
let delta: TermRowDelta = row_height.into(); // Uses From<RowHeight>
```

**Mathematical law:**
- For safe emission: Only emit if `delta != 0`
- `as_nonzero_u16()` returns `None` for zero, `Some(n)` otherwise

**Key difference from absolute positioning:**
- `TermRow`/`TermCol`: 1-based absolute coordinates (for `CursorPosition`)
- `TermRowDelta`/`TermColDelta`: Relative movement amounts (for `CursorUp/Down/Forward/Backward`)

## Decision Trees

See the accompanying `decision-trees.md` file for flowcharts showing which trait to use for
each scenario.

## Detailed Reference

For comprehensive documentation, decision trees, and more examples, see:

[`tui/src/core/units/bounds_check/mod.rs`](tui/src/core/units/bounds_check/mod.rs)

This module contains:
- Complete API documentation
- Mathematical laws for each trait
- Visual decision trees
- Edge case handling
- Performance notes

## Common Mistakes

### ❌ Mistake 1: Using raw usize

```rust
// Bad - ambiguous types
let index: usize = 10;
let length: usize = 100;
if index < length {  // Works, but no type safety
    // ...
}
```

**Fix:**
```rust
// Good - clear types
let index = idx(10);
let length = len(100);
if !index.overflows(length) {  // Type-safe!
    // ...
}
```

### ❌ Mistake 2: Array bounds used for cursor

```rust
// Bad - cursor can be at end!
let cursor = idx(10);
let text_length = len(10);
if cursor.overflows(text_length) {  // Wrong! Cursor at end is valid
    return Err("Invalid cursor");
}
```

**Fix:**
```rust
// Good - cursor bounds check
if text_length.check_cursor_position_bounds(cursor) == CursorPositionBoundsStatus::Overflows {
    return Err("Invalid cursor");
}
```

### ❌ Mistake 3: Comparing incompatible types

```rust
// Bad - this won't compile (good!)
let row = row(5);
let width = width(10);
if row < width {  // Compile error! Can't compare RowIndex with ColWidth
    // ...
}
```

This is actually GOOD - the type system prevents nonsensical comparisons!

### ❌ Mistake 4: Emitting CSI zero for cursor movement

```rust
// Bad - CSI 0 C moves 1 column right, not 0!
let cols = position % term_width;  // Could be 0!
let seq = CsiSequence::CursorForward(cols);
term.write_all(seq.to_string().as_bytes())?;  // Bug when cols = 0
```

**Fix:**
```rust
// Good - use delta types with zero guard
let cols_right = term_col_delta(position % term_width);
if let Some(n) = cols_right.as_nonzero_u16() {
    term.write_all(CsiSequence::CursorForward(n).to_string().as_bytes())?;
}
// No sequence emitted when cols = 0 (correct behavior)
```

## Reporting Results

When applying bounds checking:

- ✅ All bounds checked with types → "Bounds safety verified with type-safe checks!"
- 🔧 Converted raw usize to Index/Length types → Report conversions made
- 📝 Added bounds checks → List where checks were added

## Supporting Files in This Skill

This skill includes additional reference material:

- **`decision-trees.md`** - Visual decision trees and flowcharts for choosing the right bounds checking approach: main decision tree (which trait?), array vs cursor bounds comparison, index vs length visual diagrams, viewport visibility flowchart, range validation flowchart, comparison table, edge case reference, and quick reference card. **Read this when:**
  - Not sure which trait to use → Main decision tree
  - Array access vs cursor positioning confusion → Visual comparison diagrams
  - Viewport visibility logic → Viewport flowchart
  - Range validation → Range validation flowchart
  - Edge cases (empty arrays, cursor at end, zero-sized viewport) → Edge cases section
  - Quick lookup of which method for which scenario → Comparison table

## Related Skills

- `check-code-quality` - Includes testing bounds-checking code
- `write-documentation` - For documenting bounds-checking logic

## Related Commands

No dedicated command, but used throughout the codebase for safe index/length handling.

## Additional Resources

- Main implementation: `tui/src/core/units/bounds_check/mod.rs`
- Type definitions: `tui/src/core/units/`
- Examples in tests: `tui/src/core/units/bounds_check/tests/`
- Terminal delta types: `tui/src/core/coordinates/vt_100_ansi_coords/term_row_delta.rs` and `term_col_delta.rs`
