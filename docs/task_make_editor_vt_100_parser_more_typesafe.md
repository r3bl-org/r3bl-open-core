# Task: Refactor Editor and VT100 Parser to Use Type-Safe Indices and Lengths

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

---

## Table of Contents
1. [Overview](#overview)
2. [Background and Motivation](#background-and-motivation)
3. [Understanding the Type System](#understanding-the-type-system)
4. [The Two Bounds Checking Patterns](#the-two-bounds-checking-patterns)
5. [Refactoring Guide](#refactoring-guide)
6. [File-by-File Breakdown](#file-by-file-breakdown)
7. [Common Transformation Patterns](#common-transformation-patterns)
8. [Testing Strategy](#testing-strategy)
9. [Progress Tracking](#progress-tracking)

---

## Overview

This task involves refactoring two major subsystems to use type-safe index and length types instead of raw `usize`:

1. **Editor Module** (`tui/src/tui/editor/`) - ~32 `as_usize()` calls (gap buffer already complete)
2. **VT100 Parser Module** (`tui/src/core/pty_mux/vt_100_ansi_parser/`) - ~18 `as_usize()` calls

**Goal**: Replace raw `usize` comparisons, bounds checking, and arithmetic with type-safe operations using the `bounds_check` module utilities.

**Timeline Estimate**: 1-2 weeks for a developer familiar with the codebase

---

## Background and Motivation

### Why This Refactoring?

Previous code used raw `usize` for indices and lengths, leading to:
- **Type confusion**: Mixing 0-based indices with 1-based lengths
- **Off-by-one errors**: Easy to accidentally compare index with length incorrectly
- **Unclear semantics**: Hard to tell if a value is an index, length, offset, or byte position
- **No compile-time safety**: Can accidentally compare row indices with column lengths

### What Has Already Been Done

Recent commits (c55a3025, 989c8691, 20639fd1) refactored:
- Core grapheme handling (`tui/src/core/graphemes/`)
- Gap buffer implementation (`tui/src/tui/editor/zero_copy_gap_buffer/`) ✅ **COMPLETE**
- Core units (`tui/src/core/units/`)

These provide excellent examples of the refactoring patterns to follow.

---

## Understanding the Type System

### Core Traits

#### `UnitCompare` Trait
Base trait providing numeric conversions:
```rust
pub trait UnitCompare: From<usize> + From<u16> {
    fn as_usize(&self) -> usize;
    fn as_u16(&self) -> u16;
    fn is_zero(&self) -> bool;
}
```

#### `IndexMarker` Trait (0-based)
Identifies position types - represents **where** something is:
```rust
pub trait IndexMarker: UnitCompare {
    type LengthType: LengthMarker<IndexType = Self>;

    // Key methods:
    fn convert_to_length(&self) -> Self::LengthType;
    fn overflows(&self, length: impl Into<Self::LengthType>) -> bool;
    fn underflows(&self, min_bound: impl Into<Self>) -> bool;
    fn clamp_to_max_length(&self, max_length: Self::LengthType) -> Self;
    fn clamp_to_min_index(&self, min_bound: impl Into<Self>) -> Self;
}
```

#### `LengthMarker` Trait (1-based)
Identifies size/count types - represents **how many** of something:
```rust
pub trait LengthMarker: UnitCompare {
    type IndexType: IndexMarker<LengthType = Self>;

    // Key methods:
    fn convert_to_index(&self) -> Self::IndexType;
    fn is_overflowed_by(&self, index: impl Into<Self::IndexType>) -> bool;
    fn remaining_from(&self, index: impl Into<Self::IndexType>) -> Length;
    fn clamp_to_max(&self, max_length: impl Into<Self>) -> Self;
}
```

### Type Relationships

```
0-based (IndexMarker)          1-based (LengthMarker)
━━━━━━━━━━━━━━━━━━━━          ━━━━━━━━━━━━━━━━━━━━━
Index          ←→              Length
RowIndex       ←→              RowHeight
ColIndex       ←→              ColWidth
ByteIndex      ←→              ByteLength
SegIndex       ←→              SegLength
```

**Bidirectional Constraint**: Each index type has exactly ONE corresponding length type.

### Visual Example: 0-based vs 1-based

```text
          ╭────── length=5 (1-based) ──────╮
Content:  │ h │ e │ l │ l │ o │
          └───┴───┴───┴───┴───┘
Index:      0   1   2   3   4   5 (out of bounds)
          ╰─ valid indices ──╯   ↑
                           (length-1)

• Index 0-4: Valid content access (index < length)
• Index 5: Invalid for access (index >= length)
• Position 5: Valid for cursor placement (index == length OK for insertions)
```

---

## The Two Bounds Checking Patterns

### Pattern 1: Array Access Bounds Checking

**Use when**: Accessing array/buffer elements (read/write operations)

**Rule**: Index must be `< length` (strictly less than)

```rust
use r3bl_tui::{IndexMarker, idx, len};

let index = idx(5);
let length = len(10);

// Method 1: Boolean check - from index perspective
if index.overflows(length) {
    println!("Can't access - out of bounds");
}

// Method 2: Boolean check - from length perspective
if length.is_overflowed_by(index) {
    println!("Can't access - out of bounds");
}

// Method 3: Pattern matching for detailed status
match index.check_array_access_bounds(length) {
    ArrayAccessBoundsStatus::Within => { /* safe to access */ }
    ArrayAccessBoundsStatus::Overflowed => { /* index >= length */ }
    ArrayAccessBoundsStatus::Underflowed => { /* index < min */ }
}
```

### Pattern 2: Cursor Position Bounds Checking

**Use when**: Positioning cursors, insertions, or operations that can occur "after" content

**Rule**: Index can be `<= length` (equal is valid for end position)

```rust
use r3bl_tui::{BoundsCheck, CursorPositionBoundsStatus, idx, len};

let cursor_pos = idx(5);
let content_length = len(5);

match cursor_pos.check_cursor_position_bounds(content_length) {
    CursorPositionBoundsStatus::AtStart => { /* cursor at position 0 */ }
    CursorPositionBoundsStatus::Within => { /* 0 < cursor < length */ }
    CursorPositionBoundsStatus::AtEnd => { /* cursor == length (valid!) */ }
    CursorPositionBoundsStatus::Beyond => { /* cursor > length (invalid) */ }
}
```

**Key Difference**:
```text
Array Access:     0 ≤ index < length     (5 < 5 = false, out of bounds)
Cursor Position:  0 ≤ index ≤ length     (5 ≤ 5 = true, valid for insertion)
```

### When to Use Which Pattern

| Operation | Pattern | Reason |
|-----------|---------|--------|
| `buffer[index]` access | Array Access | Reading/writing needs valid element |
| `buffer.insert(pos, item)` | Cursor Position | Can insert at end (position == length) |
| `line.grapheme_at(col)` | Array Access | Retrieving existing grapheme |
| `cursor.move_to(pos)` | Cursor Position | Cursor can be after last char |
| `range.start` | Array Access | Range start must point to valid element |
| `range.end` (exclusive) | Cursor Position | Exclusive end can equal length |

---

## Refactoring Guide

### Step-by-Step Process

#### Step 1: Identify Index and Length Variables

Look for variables that represent:
- **Indices** (0-based): `row_idx`, `col_idx`, `line_index`, `char_pos`, `byte_idx`
- **Lengths** (1-based): `line_count`, `width`, `height`, `capacity`, `size`
- **Counts**: Usually lengths (e.g., `grapheme_count`)

#### Step 2: Change Function Signatures

**Before:**
```rust
fn get_line(&self, line_idx: usize) -> Option<&str> {
    self.lines.get(line_idx)
}

fn line_count(&self) -> usize {
    self.lines.len()
}
```

**After:**
```rust
fn get_line(&self, arg_line_idx: impl Into<RowIndex>) -> Option<&str> {
    let line_idx: RowIndex = arg_line_idx.into();
    self.lines.get(line_idx.as_usize())
}

fn line_count(&self) -> Length {
    len(self.lines.len())
}
```

**Note the pattern**:
- Accept `impl Into<T>` for flexibility
- Convert immediately to concrete type
- Use descriptive `arg_` prefix for parameters

#### Step 3: Replace Comparisons

**Before:**
```rust
if index >= length {
    return Err("Index out of bounds");
}

if index < min_index {
    return Err("Index underflow");
}

let safe_index = index.min(max_length - 1);
```

**After:**
```rust
if index.overflows(length) {
    return Err("Index out of bounds");
}

if index.underflows(min_index) {
    return Err("Index underflow");
}

let safe_index = index.clamp_to_max_length(max_length);
```

#### Step 4: Replace Arithmetic

**Before:**
```rust
let remaining = length - index;
let max_valid_index = length - 1;
let one_based = zero_based + 1;
```

**After:**
```rust
let remaining = length.remaining_from(index);
let max_valid_index = length.convert_to_index();
let one_based = zero_based.convert_to_length();
```

#### Step 5: Mark Intentional `usize` Usage

When `.as_usize()` is **legitimately** needed (stdlib interfacing):

```rust
/// # Implementation Note: Intentional Use of Raw `usize`
///
/// Uses `.as_usize()` for vec indexing because:
/// - Rust's `Vec::get()` requires `usize` indices
/// - Type-safe bounds checking already performed above via `overflows()`
/// - Direct indexing is safe after bounds validation
let item = &self.items[index.as_usize()];
```

**Common legitimate cases**:
- Vec/array indexing: `vec[idx.as_usize()]`
- String slicing: `&s[start.as_usize()..end.as_usize()]`
- Stdlib min/max: `.min(len.as_usize())`
- Allocations: `Vec::with_capacity(len.as_usize())`

---

## File-by-File Breakdown

### Phase 1: Editor Buffer Module

#### Priority 1: Core Editor Buffer Files

##### 1. `editor_buffer/buffer_struct.rs`
**Current issues**: ~6 `as_usize()` calls
**Focus areas**:
- `EditorBuffer` struct methods that return indices/lengths
- `line_count()`, `get_line()` type methods
- Buffer capacity and sizing operations

##### 2. `editor_buffer/caret_locate.rs`
**Current issues**: ~3 `as_usize()` calls
**Focus areas**:
- Caret positioning logic
- Conversion between buffer positions and screen positions

##### 3. `editor_buffer/history.rs`
**Current issues**: ~4 `as_usize()` calls
**Focus areas**:
- Undo/redo history operations
- Buffer versioning and indexing

#### Priority 2: Editor Buffer Support Files

##### 4. `editor_buffer/clipboard_service.rs`
**Focus areas**: Clipboard operations with selections

##### 5. `editor_buffer/clipboard_support.rs`
**Focus areas**: Clipboard trait implementations

##### 6. `editor_buffer/selection_list.rs`
**Focus areas**: Selection handling and list operations

##### 7. `editor_buffer/selection_range.rs`
**Focus areas**: Selection range calculations

##### 8. `editor_buffer/selection_support.rs`
**Focus areas**: Selection trait implementations

##### 9. `editor_buffer/sizing.rs`
**Focus areas**: Buffer sizing calculations

##### 10. `editor_buffer/render_cache.rs`
**Focus areas**: Rendering cache with coordinates

### Phase 2: Editor Engine Module

#### Priority 1: Core Engine Files

##### 1. `editor_engine/content_mut.rs`
**Current issues**: ~21 `as_usize()` calls (highest priority)
**Focus areas**:
- Content mutation operations
- Insert/delete operations using type-safe indices

##### 2. `editor_engine/validate_buffer_mut.rs`
**Current issues**: ~6 `as_usize()` calls
**Focus areas**:
- Buffer validation logic
- Bounds checking during mutations

##### 3. `editor_engine/engine_public_api.rs`
**Current issues**: ~1 `as_usize()` call
**Focus areas**:
- Public API surface with type-safe parameters

#### Priority 2: Engine Support Files

##### 4. `editor_engine/engine_struct.rs`
**Focus areas**: Core engine struct and initialization

##### 5. `editor_engine/engine_internal_api.rs`
**Focus areas**: Internal API methods

##### 6. `editor_engine/caret_mut.rs`
**Focus areas**: Caret mutation operations

##### 7. `editor_engine/select_mode.rs`
**Focus areas**: Selection mode operations

##### 8. `editor_engine/scroll_editor_content.rs`
**Focus areas**: Scrolling calculations

##### 9. `editor_engine/validate_scroll_on_resize.rs`
**Focus areas**: Scroll validation during resize

##### 10. `editor_engine/editor_macros.rs`
**Focus areas**: Utility macros for type-safe operations

### Phase 3: Editor Component Module

##### 1. `editor_component/editor_component_struct.rs`
**Focus areas**: Component struct definition

##### 2. `editor_component/editor_component_traits.rs`
**Focus areas**: Component trait implementations

##### 3. `editor_component/editor_event.rs`
**Focus areas**: Event handling with coordinates

### Phase 4: VT100 Parser Module

#### Priority 1: Terminal Units and Core Operations

##### 1. `term_units.rs`
**Current issues**: ~4 `as_usize()` calls
**Special consideration**: Uses `TermRow`/`TermCol` (1-based terminal coordinates)

**Key insight**: Terminal coordinates are ALREADY type-safe with 1-based semantics:
```rust
pub struct TermRow(pub u16);  // 1-based
pub struct TermCol(pub u16);  // 1-based

impl TermRow {
    pub fn from_zero_based(row: Row) -> Self { Self(row.as_u16() + 1) }
    pub fn to_zero_based(self) -> Option<Row> { ... }
}
```

**Refactoring focus**:
- Verify conversions between `TermRow`/`Row` and `TermCol`/`Col`
- Ensure bounds checking when converting to 0-based indices

##### 2. `operations/cursor_ops.rs`
**Focus**: Cursor movement operations
- When converting to buffer coordinates, use type-safe indices
- Validate cursor positions with `check_cursor_position_bounds`

##### 3. `operations/scroll_ops.rs`
**Focus**: Scrolling operations
- Scroll regions use `TermRow` for top/bottom margins
- Ensure scroll amount calculations use type-safe arithmetic

##### 4. `operations/line_ops.rs`
**Focus**: Line manipulation
- Line insertions/deletions use `TermRow`
- Buffer line counts should be `Length` not `usize`

##### 5. `operations/char_ops.rs`
**Focus**: Character operations
- Column positions use `TermCol`/`ColIndex`
- Wide character handling

##### 6. `operations/control_ops.rs`
**Focus**: Control character handling
- Cursor movements
- Tab stops (use `ColIndex` for tab positions)

#### Priority 2: Remaining Operations

##### 7-12. Other operation files:
- `sgr_ops.rs` - SGR (styling) operations
- `osc_ops.rs` - OSC (operating system command) operations
- `dsr_ops.rs` - DSR (device status report) operations
- `mode_ops.rs` - Mode setting operations
- `margin_ops.rs` - Margin operations
- `terminal_ops.rs` - Terminal control operations

#### Priority 3: Parser Core

##### 13. `perform.rs`
**Focus**:
- Main ANSI parsing logic
- Parameter validation
- Coordinate extraction from CSI sequences

##### 14. `protocols/csi_codes.rs`
**Current issues**: ~3 `as_usize()` calls
**Focus**:
- CSI sequence parameter handling
- Coordinate parsing

##### 15. `ansi_parser_public_api.rs`
**Focus**: Public API surface
- Ensure public methods use type-safe parameters

#### Priority 4: Tests

Update tests after implementation changes. Focus on:
- Updating test fixtures with type-safe constructors
- Verifying bounds checking behavior
- Testing edge cases (empty buffers, boundary conditions)

---

## Common Transformation Patterns

### Pattern 1: Simple Index/Length Variables

```rust
// ❌ Before
let line_idx: usize = 5;
let line_count: usize = buffer.len();

// ✅ After
let line_idx: RowIndex = row(5);
let line_count: Length = buffer.line_count();
```

### Pattern 2: Function Parameters

```rust
// ❌ Before
fn get_line(&self, line_idx: usize) -> Option<&str> { ... }

// ✅ After
fn get_line(&self, arg_line_idx: impl Into<RowIndex>) -> Option<&str> {
    let line_idx: RowIndex = arg_line_idx.into();
    // ...
}
```

**Why `impl Into<T>`?** Allows callers to pass:
- Concrete type: `buffer.get_line(row(5))`
- Compatible types: `buffer.get_line(cursor.row)`
- Raw values: `buffer.get_line(5)` (via `From<usize>` impl)

### Pattern 3: Bounds Checking

```rust
// ❌ Before
if index >= length {
    return None;
}

if index < start || index > end {
    return None;
}

// ✅ After
if index.overflows(length) {
    return None;
}

match index.check_bounds_range(start, end_length) {
    ArrayAccessBoundsStatus::Within => { /* proceed */ }
    _ => return None,
}
```

### Pattern 4: Index Arithmetic

```rust
// ❌ Before
let next_line = current_line + 1;
let prev_line = if current_line > 0 { current_line - 1 } else { 0 };
let last_valid_index = length - 1;

// ✅ After
let next_line = current_line + row(1);
let prev_line = current_line.saturating_sub(row(1));
let last_valid_index = length.convert_to_index();
```

### Pattern 5: Min/Max Operations

```rust
// ❌ Before
let safe_index = index.min(max_length - 1);
let clamped_start = start.max(0);

// ✅ After
let safe_index = index.clamp_to_max_length(max_length);
let clamped_start = start.clamp_to_min_index(idx(0));
```

### Pattern 6: Length Calculations

```rust
// ❌ Before
let remaining = total_length - current_position;
let chars_to_end = (length - position) as usize;

// ✅ After
let remaining = total_length.remaining_from(current_position);
let chars_to_end = length.remaining_from(position);
```

### Pattern 7: Conversions Between Index and Length

```rust
// ❌ Before
let one_based_count = zero_based_index + 1;
let zero_based_index = one_based_count - 1;

// ✅ After
let one_based_count = zero_based_index.convert_to_length();
let zero_based_index = one_based_count.convert_to_index();
```

### Pattern 8: Range Operations

```rust
// ❌ Before
let range = start_idx..end_idx;
if range.start >= range.end {
    return Err("Invalid range");
}

// ✅ After
use r3bl_tui::RangeBoundary;

let range = start_idx..end_idx;
if !range.is_valid(buffer_length) {
    return Err("Invalid range");
}
```

### Pattern 9: Loop Iteration

```rust
// ❌ Before
for i in 0..line_count {
    let line = buffer.get_line(i).unwrap();
    // ...
}

// ✅ After
for i in 0..line_count.as_usize() {
    let line = buffer.get_line(row(i)).unwrap();
    // ...
}

// ✅ Even better - iterate directly
for line_idx in (0..line_count.as_usize()).map(row) {
    let line = buffer.get_line(line_idx).unwrap();
    // ...
}
```

### Pattern 10: Intentional `usize` with Documentation

```rust
// ✅ After (when legitimately needed)
/// # Implementation Note: Intentional Use of Raw `usize`
///
/// Uses `.as_usize()` for:
/// 1. Vec indexing - `self.buffer[idx.as_usize()]`
/// 2. String slicing - `&s[start.as_usize()..end.as_usize()]`
///
/// Bounds checking performed above via `idx.overflows(length)`.
let element = &self.buffer[idx.as_usize()];
let slice = &content[start.as_usize()..end.as_usize()];
```

---

## Testing Strategy

### Per-File Testing Checklist

After refactoring each file:

1. **Compile Check**: `cargo check`
   - Verify no type errors
   - Check for missing trait imports

2. **Clippy Check**: `cargo clippy --all-targets`
   - Look for new warnings about comparisons
   - Check for inefficient type conversions
   - Verify no `.as_usize()` calls that should be type-safe

3. **Unit Tests**: `cargo nextest run`
   - Run tests for the module
   - Pay special attention to:
     - Boundary conditions (empty buffers, single element)
     - Off-by-one scenarios
     - Cursor positioning at end of content

4. **Integration Tests**: Run full test suite
   - Editor tests with various content sizes
   - VT100 parser conformance tests

### Specific Test Scenarios

#### For Editor Module:
```rust
#[test]
fn test_cursor_at_end_of_line() {
    let mut buffer = EditorBuffer::new();
    // Test scenario setup...

    let line_length = len(10);
    let cursor_at_end = col(10); // Equal to length

    // Should be valid for cursor placement
    assert_eq!(
        cursor_at_end.check_cursor_position_bounds(line_length),
        CursorPositionBoundsStatus::AtEnd
    );
}

#[test]
fn test_index_overflow() {
    let line_length = len(5);
    let invalid_index = col(5); // Equal to length

    // Should overflow for array access
    assert!(invalid_index.overflows(line_length));
}
```

#### For VT100 Parser:
```rust
#[test]
fn test_terminal_to_buffer_conversion() {
    let term_row = term_row(5); // 1-based
    let buffer_row = term_row.to_zero_based().unwrap(); // 0-based

    assert_eq!(buffer_row, row(4));
}

#[test]
fn test_cursor_position_bounds() {
    let buffer_height = len(24);
    let term_row = term_row(24);
    let buffer_row = term_row.to_zero_based().unwrap();

    // Should be valid (index 23 < length 24)
    assert!(!buffer_row.overflows(buffer_height));
}
```

### Manual Testing

1. **Editor Testing**:
   ```bash
   cargo run --example editor
   ```
   - Test cursor movement to end of lines
   - Test insertion at end of buffer
   - Test deletion at boundaries
   - Test undo/redo with boundary cases

2. **Terminal Emulator Testing**:
   ```bash
   cargo run --example pty_mux
   ```
   - Run programs with cursor movement (vim, emacs)
   - Test scrolling at screen edges
   - Test wide character handling
   - Verify terminal resizing

---

## Progress Tracking

### Tracking Spreadsheet

Update this section as you complete files:

```markdown
# Refactoring Progress

## Editor Module

### Phase 1: Core Editor Buffer
- [ ] editor_buffer/buffer_struct.rs (6 calls)
- [ ] editor_buffer/caret_locate.rs (3 calls)
- [ ] editor_buffer/history.rs (4 calls)

### Phase 2: Editor Buffer Support
- [ ] editor_buffer/clipboard_service.rs
- [ ] editor_buffer/clipboard_support.rs
- [ ] editor_buffer/selection_list.rs
- [ ] editor_buffer/selection_range.rs
- [ ] editor_buffer/selection_support.rs
- [ ] editor_buffer/sizing.rs
- [ ] editor_buffer/render_cache.rs

### Phase 3: Editor Engine
- [ ] editor_engine/content_mut.rs (21 calls)
- [ ] editor_engine/validate_buffer_mut.rs (6 calls)
- [ ] editor_engine/engine_public_api.rs (1 call)
- [ ] editor_engine/engine_struct.rs
- [ ] editor_engine/engine_internal_api.rs
- [ ] editor_engine/caret_mut.rs
- [ ] editor_engine/select_mode.rs
- [ ] editor_engine/scroll_editor_content.rs
- [ ] editor_engine/validate_scroll_on_resize.rs
- [ ] editor_engine/editor_macros.rs

### Phase 4: Editor Component
- [ ] editor_component/editor_component_struct.rs
- [ ] editor_component/editor_component_traits.rs
- [ ] editor_component/editor_event.rs

## VT100 Parser Module

### Phase 1: Core
- [ ] term_units.rs (4 calls)
- [ ] operations/cursor_ops.rs
- [ ] operations/scroll_ops.rs
- [ ] operations/line_ops.rs
- [ ] operations/char_ops.rs
- [ ] operations/control_ops.rs

### Phase 2: Remaining Operations
- [ ] operations/sgr_ops.rs
- [ ] operations/osc_ops.rs
- [ ] operations/dsr_ops.rs
- [ ] operations/mode_ops.rs
- [ ] operations/margin_ops.rs
- [ ] operations/terminal_ops.rs

### Phase 3: Parser Core
- [ ] perform.rs
- [ ] protocols/csi_codes.rs (3 calls)
- [ ] ansi_parser_public_api.rs

### Phase 4: Tests
- [ ] Update all test files

## Testing Milestones
- [ ] All cargo check passes
- [ ] All cargo clippy passes
- [ ] All unit tests pass
- [ ] Manual editor testing complete
- [ ] Manual terminal emulator testing complete
```

### Implementation Notes Section

**Add your discoveries and notes here as you work:**

```markdown
## Implementation Notes

### Completed Files

#### [Date] - filename.rs
- **Changes made**: Brief description
- **Tricky cases**: Any complex transformations
- **Tests**: Any test updates or fixes needed
- **Notes**: Patterns discovered, gotchas, etc.

### Patterns Discovered
- Add any new patterns you find here

### Common Issues and Solutions
- Document problems you encounter and how you solved them
```

### Daily Commit Strategy

Commit after completing each file or logical group:

```bash
git add tui/src/tui/editor/editor_buffer/buffer_struct.rs
git commit -m "[tui/editor] Refactor buffer_struct to use type-safe indices

- Replace usize with RowIndex/Length
- Add bounds checking with overflows()
- Document intentional as_usize() usage
- Tests passing"
```

---

## Common Pitfalls and Solutions

### Pitfall 1: Confusing Array Access vs Cursor Position

**Problem**: Using `overflows()` when cursor semantics are needed

```rust
// ❌ Wrong - prevents insertion at end
if cursor_pos.overflows(line_length) {
    return Err("Can't move cursor");
}

// ✅ Correct - allows cursor at end
match cursor_pos.check_cursor_position_bounds(line_length) {
    CursorPositionBoundsStatus::Beyond => return Err("Can't move cursor"),
    _ => { /* valid position */ }
}
```

### Pitfall 2: Forgetting to Convert in Loops

**Problem**: Using type-safe length in range but not converting loop variable

```rust
// ❌ Wrong - type mismatch
let line_count = buffer.line_count(); // Returns Length
for i in 0..line_count.as_usize() {
    buffer.get_line(i); // Error: expects RowIndex, got usize
}

// ✅ Correct - convert loop variable
for i in 0..line_count.as_usize() {
    buffer.get_line(row(i)); // OK
}
```

### Pitfall 3: Unnecessary Type Conversions

**Problem**: Converting to usize for comparisons

```rust
// ❌ Wrong - defeats type safety
if index.as_usize() >= length.as_usize() {
    // ...
}

// ✅ Correct - use type-safe comparison
if index.overflows(length) {
    // ...
}
```

### Pitfall 4: Mismatched Index/Length Types

**Problem**: Comparing row indices with column lengths

```rust
// ❌ Won't compile (good!)
let row_idx: RowIndex = row(5);
let col_width: ColWidth = width(10);
if row_idx.overflows(col_width) { // Error: type mismatch
    // ...
}

// ✅ Correct - compare compatible types
let row_idx: RowIndex = row(5);
let row_height: RowHeight = height(10);
if row_idx.overflows(row_height) { // OK
    // ...
}
```

### Pitfall 5: Off-by-One in Conversions

**Problem**: Manually adding/subtracting for 0-based ↔ 1-based conversion

```rust
// ❌ Wrong - error-prone
let length_val = index.as_usize() + 1;
let index_val = length.as_usize() - 1;

// ✅ Correct - use conversion methods
let length_val = index.convert_to_length();
let index_val = length.convert_to_index();
```

### Pitfall 6: Not Documenting Intentional `usize`

**Problem**: Leaving `.as_usize()` calls without explanation

```rust
// ❌ Wrong - unclear why usize is needed
let item = &self.buffer[idx.as_usize()];

// ✅ Correct - documented
/// # Implementation Note: Intentional Use of Raw `usize`
///
/// Vec indexing requires usize. Bounds check performed above.
let item = &self.buffer[idx.as_usize()];
```

---

## Useful Commands

### Search for Remaining `usize` Usage

```bash
# Find function signatures with usize
rg "fn.*\(.*usize" --type rust tui/src/tui/editor/

# Find struct fields with usize
rg "^\s+\w+:\s*usize" --type rust tui/src/tui/editor/

# Find variables with usize type annotation
rg "let.*:\s*usize" --type rust tui/src/tui/editor/

# Count as_usize() calls per file
rg "as_usize\(\)" --type rust tui/src/tui/editor/ --count
```

### Run Focused Tests

```bash
# Test specific module
cargo nextest run -p r3bl_tui --test editor_tests

# Test specific function
cargo nextest run -p r3bl_tui test_cursor_bounds

# Run with output
cargo nextest run -p r3bl_tui --nocapture
```

### Clippy for Bounds Checking

```bash
# Check for comparison issues
cargo clippy --all-targets -- -W clippy::comparison_chain

# Check for casting issues
cargo clippy --all-targets -- -W clippy::cast_possible_truncation
```

---

## Questions and Support

If you encounter issues:

1. **Type Confusion**: Check if you're mixing row/column types or index/length types
2. **Bounds Checking**: Verify if you need array access or cursor position semantics
3. **Conversion Errors**: Use `.convert_to_index()` / `.convert_to_length()` instead of arithmetic
4. **Test Failures**: Check boundary conditions - especially empty buffers and end positions

Refer to:
- `tui/src/core/units/bounds_check/` - Full bounds checking implementation
- Recent commits c55a3025, 989c8691, 20639fd1 - Refactoring examples
- `CLAUDE.md` - Project coding guidelines

---

## Success Criteria

- [ ] All `usize` comparisons replaced with type-safe operations
- [ ] All bounds checking uses `overflows()`, `check_cursor_position_bounds()`, etc.
- [ ] All clamping uses `clamp_to_max_length()`, `clamp_to_min_index()`, etc.
- [ ] All remaining `.as_usize()` calls documented with implementation notes
- [ ] `cargo check` passes
- [ ] `cargo clippy --all-targets` passes with no new warnings
- [ ] `cargo nextest run` passes all tests
- [ ] Manual testing of editor shows no regressions
- [ ] Manual testing of terminal emulator shows no regressions
- [ ] **All tests remain unchanged** - behavior preserved exactly

**Estimated Timeline**: 1-2 weeks for a developer familiar with the codebase

Good luck! 🚀