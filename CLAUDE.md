# Claude Code Instructions for r3bl-open-core

When doing work, when you have questions about important choices to be made, or ambiguities in the
task, please ask the user for clarification immediately.

## Rust Code Guidelines

### MCP Tools to understand and change Rust code

Use these MCP tools to navigate and modify Rust code effectively:

- serena: definition, diagnostics, edit_file, hover, references, rename symbol, etc.

### How to write documentation comments

Documentation architecture should follow the "inverted pyramid" principle: high-level concepts at
the module, trait, struct, enum level, and implementation details at the method level. When a trait
has only one primary method, most documentation belongs at the trait level to avoid readers having
to hunt through method docs for the big picture.

**Example Placement Guidelines:**
- **Trait/Module level**: Place conceptual examples showing *why* and *when* to use the API, complete
  workflows, visual diagrams, common mistakes, and antipatterns. These examples teach the concept.
- **Method level**: Place minimal syntax examples showing *how* to call the specific method with exact
  types and parameters. These serve as quick reference for IDE tooltips.
- **Graduated complexity**: Examples should match the abstraction level - comprehensive scenarios at
  trait level, simple syntax at method level.
- **Avoid duplication**: Don't repeat full examples between trait and method docs. Reference the trait
  docs from methods when detailed examples already exist there.

Where it is possible use ASCII diagrams to illustrate concepts. Use code examples extensively to
demonstrate usage patterns.

### Use strong type safety in the codebase for bounds checking, index (0-based), and length (1-based) handling

Throughout the implementation, use the type-safe bounds checking utilities from
`tui/src/core/units/bounds_check/` which provide foundational traits for core operations and
semantic traits for specific use case validation.

#### Core Type Safety Principles

- Instead of using `usize` or `u16` for indices, use index types (0-based): `RowIndex`, `ColIndex`
- Instead of using `usize` or `u16` for lengths, use length types (1-based): `RowHeight`, `ColWidth`
- Use type-safe comparisons to eliminate `.as_usize()` calls where possible
- Use `.is_zero()` for zero checks instead of `== 0`
- Leverage `convert_to_index()` and `convert_to_length()` for safe type conversions

#### Common Imports for Bounds Checking

When using bounds checking in your code, you'll typically need these imports:

```rust
use std::ops::Range;
use r3bl_tui::{
    // Core traits
    ArrayBoundsCheck, IndexOps, LengthOps, ViewportBoundsCheck, RangeBoundsExt,
    RangeConvertExt, CursorBoundsCheck,
    // Status enums
    ArrayAccessBoundsStatus, CursorPositionBoundsStatus, RangeValidityStatus,
    // Type constructors
    col, row, width, height, idx, len,
    // Index types
    ColIndex, RowIndex, Index,
    // Length types
    ColWidth, RowHeight, Length,
};
```

#### Pattern 1: Array Access Bounds Checking

**When to use**: Accessing elements in arrays, vectors, or buffers where you have an index and a
container length.

**Methods** (from `ArrayBoundsCheck` trait):

- `index.overflows(length)` - checks if `index >= length`
- `index.underflows(min_bound)` - checks if `index < min_bound`
- `index.check_array_access_bounds(length)` - returns `ArrayAccessBoundsStatus`
- `index.check_within_bounds(min, max)` - returns `RangeBoundsResult`
- `length.is_overflowed_by(index)` - same check from length perspective (LengthOps)
- `index.clamp_to_max_length(length)` - clamp index to valid range (IndexOps)
- `index.clamp_to_min_index(min_bound)` - clamp index to minimum bound (IndexOps)

**Example**:

```rust
if row_index.overflows(buffer_height) {
    return; // Index out of bounds
}
let line = buffer[row_index.as_usize()]; // Safe access
```

#### Pattern 2: Cursor Position Bounds Checking

**When to use**: Positioning cursors where `index == length` is valid (cursor can be placed at end).

**Methods**:

- `content_length.check_cursor_position_bounds(cursor_pos)` - allows position at end
- `content_length.eol_cursor_position()` - get cursor position at end-of-line
- `content_length.is_valid_cursor_position(pos)` - check if cursor position is valid
- `content_length.clamp_cursor_position(pos)` - clamp cursor to valid bounds
- Use these instead of `overflows()` for cursor placement

**Example**:

```rust
match line_count.check_cursor_position_bounds(cursor_row) {
    CursorPositionBoundsStatus::Within => { /* valid position */ }
    CursorPositionBoundsStatus::AtEnd => { /* cursor at end, also valid */ }
    _ => { /* invalid position */ }
}
```

#### Pattern 3: Viewport Visibility Checking

**When to use**: Determining what content is visible in rendering windows with exclusive upper
bounds.

**Methods**:

- `index.check_viewport_bounds(viewport_start, viewport_size)` - returns three-state result for
  viewport/window checks with exclusive upper bound `[start, start+size)`
  - Returns `RangeBoundsResult::Underflowed`, `Within`, or `Overflowed`
  - For boolean checks, compare with `RangeBoundsResult::Within`

**Use Cases**:

- **Viewport checking**: Use `check_viewport_bounds(viewport_start, viewport_size)` for windows
- **Rendering optimization**: Skip processing for off-screen elements
- **Scroll calculations**: Determining what content needs to be rendered

**Examples**:

```rust
// Viewport containment (exclusive upper bound)
match caret_col.check_viewport_bounds(viewport_start, viewport_width) {
    RangeBoundsResult::Underflowed => { /* scroll right */ }
    RangeBoundsResult::Within => { /* caret visible */ }
    RangeBoundsResult::Overflowed => { /* scroll left */ }
}

// Simple viewport visibility check
if content_index.check_viewport_bounds(viewport_start, viewport_size) == RangeBoundsResult::Within {
    render_content(content_index);
}
```

#### Pattern 4: Range Membership & Validation

**When to use**: Validating range objects and checking if indices are within ranges.

This pattern covers TWO distinct operations:

**A. Range Structure Validation** - Check if range object is well-formed:

**Methods**:

- `range.check_range_is_valid_for_length(buffer_length)` - check if `Range<Index>` or
  `RangeInclusive<Index>` is structurally valid, returns `RangeValidityStatus`
- `range.clamp_range_to(buffer_length)` - ensure range fits within content bounds

**Use Cases**:

- Iterator bounds validation
- Algorithm parameter checking
- Range operations on buffers

**Example**:

```rust
use std::ops::Range;
use r3bl_tui::{col, width, RangeBoundsExt, RangeValidityStatus};

let range: Range<ColIndex> = col(2)..col(8);
let buffer_length = width(10);

if range.check_range_is_valid_for_length(buffer_length) == RangeValidityStatus::Valid {
    for i in range { /* safe iteration */ }
}

// Or handle specific validation failures:
match range.check_range_is_valid_for_length(buffer_length) {
    RangeValidityStatus::Valid => { /* safe to use range */ }
    RangeValidityStatus::Inverted => { /* start > end */ }
    RangeValidityStatus::StartOutOfBounds => { /* start >= buffer_length */ }
    RangeValidityStatus::EndOutOfBounds => { /* end out of bounds */ }
}
```

**B. Range Membership Checking** - Check if index is within range:

**Methods**:

- `range.check_index_is_within(index)` - check if index is within range bounds (works for both
  `Range<Index>` and `RangeInclusive<Index>`)

**Use Cases**:

- VT-100 scroll region checking (inclusive bounds `[min, max]`)
- Text selection highlighting (inclusive bounds)
- Array iteration bounds (exclusive bounds `[min, max)`)
- Any range membership test with detailed status

**Examples**:

```rust
use r3bl_tui::{row, col, RangeBoundsExt, RangeBoundsResult};

// VT-100 scroll region checking (inclusive: both endpoints valid)
let scroll_region = row(2)..=row(5);
if scroll_region.check_index_is_within(row_index) == RangeBoundsResult::Within {
    perform_line_operation(row_index);
}

// Exclusive range checking for array iteration
let range = col(3)..col(8);  // [3, 8) - exclusive upper bound
match range.check_index_is_within(col_index) {
    RangeBoundsResult::Within => process_column(col_index),
    RangeBoundsResult::Underflowed => handle_too_low(),
    RangeBoundsResult::Overflowed => handle_too_high(),
}

// Three-state checking for detailed error handling
if scroll_region.check_index_is_within(row_index) != RangeBoundsResult::Within {
    return Ok(()); // Skip operation - cursor outside scroll region
}
```

**Alternative**: For simple boolean checks, you can still use standard library's `.contains()`:

```rust
// Simple boolean check (no detailed status)
if (scroll_top..=scroll_bottom).contains(&row_index) {
    perform_line_operation(row_index);
}
```

**C. Range Clamping** - Clamp index to stay within inclusive range bounds:

**Methods**:

- `index.clamp_to_range(range)` - clamp index to `RangeInclusive<Index>` bounds

**Use Cases**:

- VT-100 scroll region clamping (cursor positioning within scroll bounds)
- Text selection boundary enforcement
- Widget/viewport boundary constraints

**Examples**:

```rust
use std::ops::RangeInclusive;
use r3bl_tui::{IndexOps, row};

// VT-100 scroll region clamping
let scroll_region: RangeInclusive<_> = row(2)..=row(5);
let cursor_row = row(8);
let clamped = cursor_row.clamp_to_range(scroll_region);
assert_eq!(clamped, row(5)); // Clamped to bottom of scroll region

// Text selection boundary enforcement
let selection = col(10)..=col(20);
let insert_col = col(25);
let clamped_col = insert_col.clamp_to_range(selection);
assert_eq!(clamped_col, col(20)); // Clamped to end of selection
```

**D. Range Type Conversion** - Convert between inclusive and exclusive ranges:

**Methods**:

- `inclusive_range.to_exclusive()` - convert `RangeInclusive<Index>` → `Range<Index>`

**Use Cases**:

- VT-100 scroll regions (stored as inclusive) → Rust iteration (needs exclusive)
- Any case where you have inclusive bounds but need exclusive range semantics
- Eliminating manual `+1` arithmetic when converting range types

**Why This Matters**: VT-100 terminal operations use inclusive ranges where both endpoints are valid
row positions (e.g., scroll region `2..=5` means rows 2,3,4,5 are all in the region). However,
Rust's slice operations and iteration use exclusive ranges where the end is NOT included (e.g.,
`2..6` processes indices 2,3,4,5). The `to_exclusive()` method provides explicit, type-safe
conversion between these two semantics.

**Example**:

```rust
use r3bl_tui::{row, len, RangeConvertExt};

// VT-100 scroll region: rows 2,3,4,5 (inclusive - both endpoints valid)
let scroll_region = row(2)..=row(5);

// Convert for Rust iteration (exclusive - end not included)
let iter_range = scroll_region.to_exclusive();  // row(2)..row(6)

// Use with buffer operations
buffer.shift_lines_up(iter_range, len(1));
```

**Before** (manual conversion - error-prone):

```rust
let scroll_region = self.get_scroll_range_inclusive();
self.shift_lines_up(
    {
        let start = *scroll_region.start();
        let end = *scroll_region.end() + 1;  // Manual +1 - easy to forget!
        start..end
    },
    len(1),
)
```

**After** (type-safe conversion - explicit intent):

```rust
let scroll_region = self.get_scroll_range_inclusive();
self.shift_lines_up(scroll_region.to_exclusive(), len(1))
```

#### Trait Organization and Use Case Mapping

The bounds checking system is organized into foundational traits and semantic traits. Use this guide
to quickly find the right trait for your task:

##### Trait Hierarchy

Both `IndexOps` and `LengthOps` build on top of `NumericValue` as their super-trait:

```text
Trait Hierarchy:

                   NumericValue
                  (super-trait)
                       │
                       │ Provides: as_usize(), as_u16(), is_zero()
                       │ Purpose: Generic numeric conversions
                       │
          ┌────────────┴────────────┐
          │                         │
      IndexOps                  LengthOps
     (0-based)                  (1-based)
          │                         │
  Adds: overflows(),        Adds: is_overflowed_by(),
        underflows(),             remaining_from(),
        clamp_to_*(),             convert_to_index(),
        convert_to_length()       clamp_to_max()
```

- **`NumericValue`** - The foundational trait providing numeric conversions that enable all
  higher-level operations. Use this directly when writing generic bounds checking functions.

- **`IndexOps`** - Extends `NumericValue` with 0-based position semantics and bounds checking
  operations specific to array indexing.

- **`LengthOps`** - Extends `NumericValue` with 1-based size semantics and space calculation
  operations specific to container sizes.

This hierarchy enables both generic operations (via `NumericValue`) and specialized, type-safe
operations (via `IndexOps` and `LengthOps`).

##### Foundational Traits (Core Operations)

| Trait        | Module             | Key Question                           |
| ------------ | ------------------ | -------------------------------------- |
| NumericValue | `numeric_value.rs` | "How do I convert to usize/u16?"       |
| IndexOps     | `index_ops.rs`     | "How do indices relate to each other?" |
| Result enums | `result_enums.rs`  | "What status types are available?"     |
| LengthOps    | `length_ops.rs`    | "What can I do with a length value?"   |

##### Semantic Traits (Use Case Validation)

| Pattern | Trait               | Module                      | Key Question                                |
| ------- | ------------------- | --------------------------- | ------------------------------------------- |
| 1       | ArrayBoundsCheck    | `array_bounds_check.rs`     | "Can I safely access array index?"          |
| 2       | CursorBoundsCheck   | `cursor_bounds_check.rs`    | "Can a cursor be placed at position N?"     |
| 3       | ViewportBoundsCheck | `viewport_bounds_check.rs`  | "Is this content visible in my viewport?"   |
| 4       | RangeBoundsExt      | `range_bounds_check_ext.rs` | "Is this range valid for iteration?"        |
|         | RangeConvertExt     | `range_convert_ext.rs`      | "Convert inclusive → exclusive range types" |

##### When to Use Foundational Traits Directly

**📐 Space calculations & text wrapping** → Use `LengthOps` trait

```rust
let remaining = line_width.remaining_from(cursor_col);
if text_length.convert_to_index().overflows(remaining) {
    /* wrap to next line */
}
```

**🔧 Writing generic bounds functions** → Use `NumericValue` trait

```rust
fn safe_access<I, L>(index: I, length: L) -> bool
where I: NumericValue, L: NumericValue {
    index.as_usize() < length.as_usize()
}
```

**🎛️ Check cursor position (EOL detection)** → Use `CursorPositionBoundsStatus`

```rust
match content.check_cursor_position_bounds(cursor) {
    CursorPositionBoundsStatus::AtEnd => /* cursor after last char */,
    CursorPositionBoundsStatus::Beyond => /* show error to user */,
    _ => /* other cases */,
}
```

#### Additional Methods

**Cursor Positioning**:

- `length.eol_cursor_position()` - get cursor position at end-of-line (after last character)
- `length.is_valid_cursor_position(pos)` - check if cursor position is valid
- `length.clamp_cursor_position(pos)` - clamp cursor to valid bounds

**Index Clamping**:

- `index.clamp_to_range(range)` - clamp index to `RangeInclusive<Index>` bounds for scroll regions
  and selections
- `index.clamp_to_max_length(length)` - clamp index to upper bound expressed as length
- `index.clamp_to_min_index(min_bound)` - clamp index to lower bound expressed as index

**Length Operations**:

- `length.is_overflowed_by(index)` - same check from length perspective
- `length.remaining_from(index)` - calculate remaining space from position
- `length.clamp_to_max(max_length)` - clamp length to maximum bounds

**Advanced Usage**:

- Avoid manually comparing start and end values as `usize`
- Use semantic aliases for domain-specific operations (scroll regions, selections)
- Prefer type-safe methods over raw numeric comparisons

# Testing interactive terminal applications

For testing interactive terminal applications, use (they are both installed):

- `tmux`
- `screen`

### Rust Code Quality

After completing tasks, run:

- `cargo check` - Fast typecheck
- `cargo build` - Compile production code
- `cargo test --no-run` - Compile test code
- `cargo clippy --all-targets` / `cargo clippy --fix --allow-dirty` - Discover lints
- `cargo doc --no-deps` - Generate docs
- `cargo nextest run` - Run tests (does not run doctests)
- `cargo test --doc` - Run doctests

Performance analysis:

- `cargo bench` - Benchmarks (mark tests with `#[bench]`)
- `cargo flamegraph` - Profiling
- For TUI apps: ask user to run `run_example_with_flamegraph_profiling_perf_fold` in `lib_script.nu`

### Build Optimizations

The project includes several build optimizations configured in `.cargo/config.toml`:

- **sccache**: Shared compilation cache for faster rebuilds
- **Parallel compilation**: `-Z threads=8` for faster nightly builds
- **Wild linker**: Fast alternative linker for Linux (auto-configured when available)

Wild linker is automatically activated when both `clang` and `wild-linker` are installed via
`bootstrap.sh`. It provides significantly faster link times for iterative development on Linux.

### Git Workflow

- Never commit unless explicitly asked

## Task Tracking System

Two-file system: active work in `todo.md`, completed work in `done.md`.

### todo.md - Active Work

- Check at session start for current state
- Latest changes at top
- Mark completed tasks with `[x]`
- Keep partial sections with mixed completion states
- Maintain task hierarchy with 2-space indentation

### done.md - Archive

- Latest changes at top for historical reference
- Move complete sections only (all subtasks `[x]`)
- Never move individual tasks - preserve context
- Example: Move "fix md parser" only after all 200+ subtasks complete

### Task Format

- `[x]` completed, `[ ]` pending
- Group under descriptive headers
- Include GitHub issue links
- Add technical notes for complex tasks
