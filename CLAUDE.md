# Claude Code Instructions for r3bl-open-core

When doing work, when you have questions about important choices to be made, or ambiguities in the
task, please ask the user for clarification immediately.

Take your time to make changes. Slow and steady wins the race. It is better to be slow and steady
and careful and considerate than to be fast and careless.

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

- **Trait/Module level**: Place conceptual examples showing _why_ and _when_ to use the API,
  complete workflows, visual diagrams, common mistakes, and antipatterns. These examples teach the
  concept.
- **Method level**: Place minimal syntax examples showing _how_ to call the specific method with
  exact types and parameters. These serve as quick reference for IDE tooltips.
- **Graduated complexity**: Examples should match the abstraction level - comprehensive scenarios at
  trait level, simple syntax at method level.
- **Avoid duplication**: Don't repeat full examples between trait and method docs. Reference the
  trait docs from methods when detailed examples already exist there.

Where it is possible use ASCII diagrams to illustrate concepts. Use code examples extensively to
demonstrate usage patterns.

### Module Organization Pattern

When organizing Rust modules, prefer **private modules with public re-exports** as the default
pattern. This provides a clean API while maintaining flexibility to refactor internal structure.

#### The Recommended Pattern

```rust
// mod.rs - Module coordinator

// Private modules (hide internal structure)
mod constants;
mod types;
mod helpers;

// Public re-exports (expose stable API)
pub use constants::*;
pub use types::*;
pub use helpers::*;
```

#### Benefits

1. **Clean, Flat API** - Users import directly without unnecessary nesting:

   ```rust
   // Good - flat, ergonomic
   use my_module::MyType;
   use my_module::CONSTANT;

   // Bad - exposes internal structure
   use my_module::types::MyType;
   use my_module::constants::CONSTANT;
   ```

2. **Refactoring Freedom** - Internal reorganization doesn't break external code:

   ```rust
   // Can move items between files freely
   // External API stays: use my_module::Item;
   ```

3. **Avoid Naming Conflicts** - Private module names don't pollute the namespace:

   ```rust
   // No conflicts with other `constants` modules in the crate
   mod constants;  // Private - name hidden
   pub use constants::*;  // Items public
   ```

4. **Encapsulation** - Module structure is an implementation detail, not part of the API

#### When to Use This Pattern

**✅ Use private modules + public re-exports when:**

- Module structure is an implementation detail
- You want a flat, ergonomic API surface
- Avoiding potential name collisions
- Working with small to medium-sized modules with clear responsibilities

**Example from `csi_codes`:**

```rust
// tui/src/core/pty_mux/vt_100_ansi_parser/protocols/csi_codes/mod.rs

// Private modules - can refactor without breaking changes
mod constants;
mod margin;
mod params;
mod private_mode;
mod sequence;

// Public re-exports - stable API
pub use constants::*;
pub use margin::*;
pub use params::*;
pub use private_mode::*;
pub use sequence::*;

// Usage: clean imports
use csi_codes::CsiSequence;
use csi_codes::CSI_START;
```

#### When NOT to Use This Pattern

**❌ Keep modules public when:**

1. **Module structure IS the API** - Different domains should be explicit:

   ```rust
   pub mod frontend;  // Frontend-specific APIs
   pub mod backend;   // Backend-specific APIs
   ```

2. **Large feature domains** - When namespacing provides clarity:

   ```rust
   pub mod graphics;   // 100+ graphics-related items
   pub mod audio;      // 100+ audio-related items
   // Users: use engine::graphics::Renderer;
   ```

3. **Optional/conditional features** - Make feature boundaries explicit:
   ```rust
   #[cfg(feature = "async")]
   pub mod async_api;  // Keep separate for clarity
   ```

#### Special Cases

**Test utilities** can be public but conditional:

```rust
#[cfg(any(test, doc))]
pub mod test_helpers;  // Public for testing/docs, not in release builds
```

**Hybrid approach** (used by standard library):

```rust
pub mod collections {
    // Internal structure hidden
    mod vec;
    mod hashmap;

    // Items re-exported
    pub use vec::Vec;
    pub use hashmap::HashMap;
}
```

### Use strong type safety in the codebase for bounds checking, index (0-based), and length (1-based) handling

Use the type-safe bounds checking utilities from `tui/src/core/units/bounds_check/` which provide
comprehensive protection against off-by-one errors. See
[`bounds_check/mod.rs`](tui/src/core/units/bounds_check/mod.rs) for detailed documentation, quick
start guide, and examples.

#### Core Principles

- Use index types (0-based) instead of `usize`: `RowIndex`, `ColIndex`, `Index`
- Use length types (1-based) instead of `usize`: `RowHeight`, `ColWidth`, `Length`
- Use type-safe comparisons to prevent comparing incompatible types (e.g., `RowIndex` vs `ColWidth`)
- Use `.is_zero()` for zero checks instead of `== 0`

#### Common Imports

```rust
use std::ops::Range;
use r3bl_tui::{
    // Traits
    ArrayBoundsCheck, CursorBoundsCheck, ViewportBoundsCheck,
    RangeBoundsExt, RangeConvertExt, IndexOps, LengthOps,
    // Status enums
    ArrayOverflowResult, CursorPositionBoundsStatus, RangeValidityStatus, RangeBoundsResult,
    // Type constructors
    col, row, width, height, idx, len,
};
```

#### Quick Pattern Reference

| Use Case                | Trait                 | Key Method                                   | When to Use                                                 |
| ----------------------- | --------------------- | -------------------------------------------- | ----------------------------------------------------------- |
| **Array access**        | `ArrayBoundsCheck`    | `index.overflows(length)`                    | Validating `buffer[index]` access (`index < length`)        |
| **Cursor positioning**  | `CursorBoundsCheck`   | `length.check_cursor_position_bounds(pos)`   | Text editing where cursor can be at end (`index <= length`) |
| **Viewport visibility** | `ViewportBoundsCheck` | `index.check_viewport_bounds(start, size)`   | Rendering optimization (is content on-screen?)              |
| **Range validation**    | `RangeBoundsExt`      | `range.check_range_is_valid_for_length(len)` | Iterator bounds, algorithm parameters                       |
| **Range membership**    | `RangeBoundsExt`      | `range.check_index_is_within(index)`         | VT-100 scroll regions, text selections                      |
| **Range conversion**    | `RangeConvertExt`     | `inclusive_range.to_exclusive()`             | Converting VT-100 ranges for Rust iteration                 |

**For comprehensive details:**

- Quick start guide and examples: [`bounds_check/mod.rs`](tui/src/core/units/bounds_check/mod.rs)
- Decision tree for choosing the right trait
- Trait hierarchy and type system architecture
- Common mistakes to avoid
- All method documentation

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
- `cargo test --all-targets` - Run tests (does not run doctests)
- `cargo test --doc` - Run doctests

Performance analysis:

- `cargo bench` - Benchmarks (mark tests with `#[bench]`)
- `cargo flamegraph` - Profiling (requires flamegraph crate)
- For TUI apps: `./run.fish run-examples-flamegraph-fold --benchmark` - Automated flamegraph
  profiling
  - 8-second continuous workload with 999Hz sampling
  - Scripted input (pangrams, cursor movements) for consistent results
  - Generates `tui/flamegraph-benchmark.perf-folded` file for analysis
  - Use `--benchmark` flag for reproducible performance comparisons

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
