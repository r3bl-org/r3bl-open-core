# Claude Code Instructions for r3bl-open-core

When doing work, when you have questions about important choices to be made, or ambiguities in the
task, please ask the user for clarification immediately.

## Rust Code Guidelines

### MCP Tools to understand and change Rust code

Use these MCP tools to navigate and modify Rust code effectively:

- serena: definition, diagnostics, edit_file, hover, references, rename symbol, etc.

Use these tools to lookup documentation and APIs:

- context7: Documentation lookup for Rust crates and APIs, and all other APIs as well

### Use strong type safety in the codebase for bounds checking, index (0-based), and length (1-based) handling

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
