# Use strong type safety in the codebase for bounds checking, index (0-based), and length (1-based) handling

This task involves refactoring two major subsystems to use type-safe index and length types instead of raw `usize`.

Throughout the implementation, use the type-safe bounds checking utilities from
`tui/src/core/units/bounds_check.rs` which have 2 main patterns: "array access bounds checking, and
"cursor positioning bounds checking". Make sure to use the `cursor_bounds.rs` utilities for cursor
positioning related tasks, and `array_bounds.rs` utilities for array access related tasks.

For both patterns:

- Instead of using `usize` or `u16` for indices, try using `IndexMarker` which is 0-based
- Instead of using `usize` or `u16` for lengths, try using `LengthMarker` which is 1-based
- Implement Range Validation with RangeBoundary
- Array Access Bounds Checking with check_array_access_bounds() and overflows()
- Cursor Position Bounds Checking with check_cursor_position_bounds()
- Type-safe comparisons to eliminate .as_usize() calls
- Use .is_zero() for zero checks instead of == 0

For "array access bounds checking" pattern:

- Use `IndexMarker::overflows()` instead of raw `<` or `>` comparisons between 0/1-based values
- Use `LengthMarker::is_overflowed_by()` for inverse checks, and `IndexMarker::is_overflowed_by()`
  similarly
- Use `LengthMarker::clamp_to()` for clamping operations
- Leverage `convert_to_index()` and `convert_to_length()` for type conversions
- Use `clamp_to()` to ensure indices and lengths stay within valid bounds and `remaining_from()` to
  compute available space
- Use `range_ext::RangeValidation` for validating ranges instead of manually comparing start and end
  values as `usize`

For "cursor positioning bounds checking" pattern:

- Use `check_cursor_position_bounds` instead of `overflows` since line_index == line_count() which
  is valid for insertions (can insert at the end), while still preventing indices beyond that point

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
|--||--|
| `buffer[index]` access | Array Access | Reading/writing needs valid element |
| `buffer.insert(pos, item)` | Cursor Position | Can insert at end (position == length) |
| `line.grapheme_at(col)` | Array Access | Retrieving existing grapheme |
| `cursor.move_to(pos)` | Cursor Position | Cursor can be after last char |
| `range.start` | Array Access | Range start must point to valid element |
| `range.end` (exclusive) | Cursor Position | Exclusive end can equal length |



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

**Preferred: Function-level rustdoc documentation**
```rust
/// Processes items in the buffer.
///
/// # Implementation Note: Intentional Use of Raw `usize`
///
/// Uses `.as_usize()` for stdlib interfacing:
/// - `Vec::get()` and indexing require `usize` parameters
/// - Type-safe bounds checking performed via `overflows()` before usage
/// - Display formatting for user-visible coordinates (1-indexed conversion)
fn process_items(&self) {
    let item = &self.items[index.as_usize()];
    let line_display = line_index.as_usize() + 1; // 1-indexed for display
}
```

**Alternative: Inline comments (when function docs not appropriate)**
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
- Debug/Display formatting: `write!(f, "count: {}", count.as_usize())`
- 1-indexed display conversion: `line_number = index.as_usize() + 1`

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

**Preferred: Function-level rustdoc**
```rust
/// Processes buffer elements safely.
///
/// # Implementation Note: Intentional Use of Raw `usize`
///
/// Uses `.as_usize()` for stdlib interfacing:
/// - `Vec` indexing requires `usize` parameters
/// - String slicing requires `usize` bounds
/// - Bounds checking performed via `idx.overflows(length)` before usage
fn process_elements(&self, idx: Index, start: Index, end: Index) {
    let element = &self.buffer[idx.as_usize()];
    let slice = &content[start.as_usize()..end.as_usize()];
}
```

**Alternative: Inline comments**
```rust
// ✅ When function docs not appropriate
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

## Implementation Notes

### Patterns Discovered
- **Type Conversion**: `Length` → `RowHeight` uses `.into()`, not `.convert_to_length()`
- **Bounds Checking**: `IndexMarker` trait must be imported for `overflows()` method
- **Gap Buffer Integration**: Existing type-safe methods reduce refactoring complexity
- **Documentation Style**: Function-level rustdoc preferred over inline comments
- **🚨 CRITICAL: Bounds Checking Semantics**:
  - Array access: `index.overflows(length)` checks `index >= length`
  - Cursor position: `index.check_cursor_position_bounds(length)` uses `index > length` for Beyond
  - Cursor at `index == length` is VALID for insertion operations!
- **Function Inlining**: Single-use helper functions should be inlined for clarity
- **Range Extensions**: Use `ByteIndexRangeExt::to_usize_range()` for stdlib range operations

### Common Issues and Solutions
- **Missing trait imports**: Add `IndexMarker` import for bounds checking methods
- **Type mismatches**: Use `.into()` for compatible type conversions
- **Unused imports**: Clean up after refactoring (e.g., remove `Length` when using direct methods)
- **🚨 Semantic bugs**: Don't replace `>` with `overflows()` - they have different semantics!
- **Single-use functions**: Consider inlining helper functions that obscure the main logic
- **Doctest failures**: Import extension traits (`ByteIndexRangeExt`) for range operations in examples

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
