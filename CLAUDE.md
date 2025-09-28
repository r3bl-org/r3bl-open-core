# Claude Code Instructions for r3bl-open-core

When doing work, when you have questions about important choices to be made, or ambiguities in the
task, please ask the user for clarification immediately.

## Rust Code Guidelines

### MCP Tools to understand and change Rust code

Use these MCP tools to navigate and modify Rust code effectively:

- serena: definition, diagnostics, edit_file, hover, references, rename symbol, etc.

### Use strong type safety in the codebase for bounds checking, index (0-based), and length (1-based) handling

Throughout the implementation, use the type-safe bounds checking utilities from
`tui/src/core/units/bounds_check/` which provide three main patterns for different use cases.

#### Core Type Safety Principles

- Instead of using `usize` or `u16` for indices, use `IndexMarker` types (0-based): `RowIndex`, `ColIndex`
- Instead of using `usize` or `u16` for lengths, use `LengthMarker` types (1-based): `RowHeight`, `ColWidth`
- Use type-safe comparisons to eliminate `.as_usize()` calls where possible
- Use `.is_zero()` for zero checks instead of `== 0`
- Leverage `convert_to_index()` and `convert_to_length()` for safe type conversions

#### Pattern 1: Array Access Bounds Checking

**When to use**: Accessing elements in arrays, vectors, or buffers where you have an index and a container length.

**Methods**:
- `index.overflows(length)` - checks if `index >= length`
- `index.check_array_access_bounds(length)` - returns `ArrayAccessBoundsStatus`
- `length.is_overflowed_by(index)` - inverse check from length perspective
- `index.clamp_to_max_length(length)` - clamp index to valid range

**Example**:
```rust
if row_index.overflows(buffer_height) {
    return; // Index out of bounds
}
let line = buffer[row_index]; // Safe access
```

#### Pattern 2: Cursor Position Bounds Checking

**When to use**: Positioning cursors where `index == length` is valid (cursor can be placed at end).

**Methods**:
- `cursor_pos.check_cursor_position_bounds(content_length)` - allows position at end
- Use this instead of `overflows()` for cursor placement

**Example**:
```rust
match cursor_row.check_cursor_position_bounds(line_count) {
    CursorPositionBoundsStatus::Within => { /* valid position */ }
    CursorPositionBoundsStatus::AtEnd => { /* cursor at end, also valid */ }
    _ => { /* invalid position */ }
}
```

#### Pattern 3: Range Membership Checking

**When to use**: Checking if a position is within a defined region or window.

**Methods**:
- `index.check_bounds_range(start_index, width_or_height)` - for viewport/window checks `[start, start+length)`
- `index.check_inclusive_range_bounds(min_index, max_index)` - for inclusive ranges `[min, max]`

**Use Cases**:
- **Viewport checking**: Use `check_bounds_range(viewport_start, viewport_size)` for windows
- **Scroll regions**: Use `check_inclusive_range_bounds(scroll_top, scroll_bottom)` for VT-100 regions
- **Selection ranges**: Use `check_inclusive_range_bounds(selection_start, selection_end)` for text selection

**Examples**:
```rust
// Viewport containment (exclusive upper bound)
match caret_col.check_bounds_range(viewport_start, viewport_width) {
    ArrayAccessBoundsStatus::Within => { /* caret visible */ }
    _ => { /* need to scroll */ }
}

// Scroll region membership (inclusive bounds)
match row_index.check_inclusive_range_bounds(scroll_top, scroll_bottom) {
    ArrayAccessBoundsStatus::Within => { /* operate within scroll region */ }
    _ => { /* skip operation */ }
}
```

#### Range Validation

- Use `RangeBoundary::is_valid()` for validating `Range<Index>` objects
- Use `range_ext::RangeValidation` for complex range operations
- Avoid manually comparing start and end values as `usize`

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
- `cargo nextest run` - Run tests

Performance analysis:

- `cargo bench` - Benchmarks (mark tests with `#[bench]`)
- `cargo flamegraph` - Profiling
- For TUI apps: ask user to run `run_example_with_flamegraph_profiling_perf_fold` in `lib_script.nu`

### Build Optimizations

The project includes several build optimizations configured in `.cargo/config.toml`:

- **sccache**: Shared compilation cache for faster rebuilds
- **Parallel compilation**: `-Z threads=8` for faster nightly builds
- **Wild linker**: Fast alternative linker for Linux (auto-configured when available)

Wild linker is automatically activated when both `clang` and `wild-linker` are installed via `bootstrap.sh`. It provides significantly faster link times for iterative development on Linux.

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
