# Claude Code Instructions for r3bl-open-core

Ask for clarification immediately on important choices or ambiguities. Take your time with
changes—slow, steady, and careful work beats fast and careless.

## Crate-Specific Instructions

Some crates have additional instructions in their own `CLAUDE.md` files:

- **build-infra/**: Provides CLI tools (binaries). **After making code changes, you MUST run
  `cargo install --path build-infra --force`** to update the installed binaries in
  `~/.cargo/bin`. See `build-infra/CLAUDE.md` for details.

When working on a specific crate, always check for a local `CLAUDE.md` file in that crate's
directory for additional workflow requirements.

## Rust Code Guidelines

### MCP Tools to understand and change Rust code

Use these MCP tools to navigate and modify Rust code effectively:

- serena: definition, diagnostics, edit_file, hover, references, rename symbol, etc.

### How to write documentation comments

Use the "inverted pyramid" principle: high-level concepts at module/trait/struct level,
implementation details at method level. Avoid making readers hunt through method docs for the big
picture.

```
╲────────────╱
 ╲          ╱  High-level concepts - Module/trait/struct documentation
  ╲────────╱
   ╲      ╱  Mid-level details - Method group documentation
    ╲────╱
     ╲  ╱  Low-level specifics - Individual method documentation
      ╲╱
```

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

If you are including an example (rustdoc test with code) then make sure that it can either compile
or run. Don't use ignore. If you can't make it compile or run, then don't include the example.

#### Handling `ignore` in Rustdoc Code Examples

When you encounter `\`\`\`rust,ignore` or `\`\`\`ignore` in code, follow this strategy to improve
documentation quality:

**For regular code (not macros):**

1. **Convert to compilable code**: Rewrite the example to actually compile or run:
   - Use `\`\`\`rust` for examples that compile and run
   - Use `\`\`\`no_run` for examples that only need to compile (e.g., code that requires external
     setup or would block execution)

2. **If conversion isn't possible**: Remove the code block and replace it with an
   [intra-doc link](mod@crate::path::to::example) pointing to actual implementation code in the
   codebase (either test or production). Optionally use `#[cfg(any(test, doc))]` to make private
   types accessible to documentation.

**For macros:**

Macro expansion issues often prevent doctests from working. In these cases:

1. **Link to real usage**: Link to actual code (test or production) that invokes the macro using
   intra-doc links.

2. **If showing macro syntax is essential**: Use the `\`\`\`ignore` format **only with this
   HTML comment** to document why it's ignored:

   ```rust
   //! <!-- It is ok to use ignore here, as this is a macro call -->
   //! ```ignore
   //! generate_pty_test! {
   //!     test_fn: interactive_input_parsing,
   //! }
   //! ```
   ```

This approach ensures readers understand that the `ignore` marker is intentional, not an oversight.

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

#### Controlling Rustfmt Behavior in Module Files

When organizing imports and exports in `mod.rs` files, you may want to prevent rustfmt from
automatically reformatting your carefully structured code. Use this directive at the top of the file
(after copyright and module-level documentation):

```rust
// Skip rustfmt for rest of file.
// https://stackoverflow.com/a/75910283/2085356
#![cfg_attr(rustfmt, rustfmt_skip)]
```

**Why use this?**

- Preserve manual alignment of public exports for readability
- Control grouping of related items (e.g., keeping test fixtures together)
- Prevent reformatting that obscures logical organization
- Maintain consistent structure across similar modules

**When to use:**

- Large `mod.rs` files with many exports
- When you have deliberately structured code alignment for documentation clarity
- Files where the organization conveys semantic meaning

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

#### Special Case - Conditionally public modules for documentation and testing

When you want a module to be private in normal builds, but public when building documentation or
tests, use conditional compilation. This allows rustdoc links to work while keeping it private in
release builds.

```rust
// mod.rs - Conditional visibility for documentation and testing

#[cfg(any(test, doc))]
pub mod vt_100_ansi_parser;
#[cfg(not(any(test, doc)))]
mod vt_100_ansi_parser;

// Re-export items for the flat public API
pub use vt_100_ansi_parser::*;
```

**Transitive Visibility:** If a conditionally public module links to another module in its
documentation, that target module must also be conditionally public:

```rust
#[cfg(any(test, doc))]
pub mod paint_impl;  // Links to diff_chunks in docs

#[cfg(any(test, doc))]
pub mod diff_chunks;  // Must also be conditionally public!
#[cfg(not(any(test, doc)))]
mod diff_chunks;
```

#### Rustdoc Intra-doc Links Best Practices

When writing rustdoc links in modules with private submodules and public re-exports:

**Method References:**

- Always include parentheses `()` in both inline mentions and reference-style link definitions
- Use `crate::` paths pointing to the public struct, not the submodule implementation

```rust
//! This calls [`reset_style()`] to clear attributes.
//!
//! [`reset_style()`]: crate::OffscreenBuffer::reset_style
```

**Module References:**

- Use the `mod@` prefix with full module path

```rust
//! See [`diff_chunks`] for implementation details.
//!
//! [`diff_chunks`]: mod@crate::tui::terminal_lib_backends::offscreen_buffer::diff_chunks
```

### Use strong type safety in the codebase for bounds checking, index (0-based), and length (1-based) handling

Use the type-safe bounds checking utilities from `tui/src/core/units/bounds_check/` which provide
comprehensive protection against off-by-one errors.

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

See [`bounds_check/mod.rs`](tui/src/core/units/bounds_check/mod.rs) for detailed documentation,
decision trees, and examples.

### Testing Interactive Terminal Applications

For testing interactive terminal applications, use (they are both installed):

- `tmux`
- `screen`

### Rust Code Quality

After completing significant code changes, run this checklist in order:

**Essential Quality Checks:**

1. `cargo check` - Fast typecheck
2. `cargo build` - Compile production code
3. `cargo rustdoc-fmt` - Format rustdoc comments (markdown tables, section headers, code blocks)
4. `cargo doc --no-deps` - Generate docs and verify no warnings
5. `cargo clippy --all-targets` - Discover linting issues (use `--fix --allow-dirty` to auto-fix)
6. `cargo test --no-run` - Compile test code
7. `cargo test --all-targets` - Run all tests (does not run doctests)
8. `cargo test --doc` - Run doctests

**Optional Performance Analysis:**

Run only when optimizing performance-critical code:

- `cargo bench` - Benchmarks (mark tests with `#[bench]`)
- `cargo flamegraph` - Profiling (requires flamegraph crate)
- `./run.fish run-examples-flamegraph-fold --benchmark` - TUI app profiling (8s workload, 999Hz
  sampling, generates `tui/flamegraph-benchmark.perf-folded`)

### Build Optimizations

The project includes several build optimizations configured in `.cargo/config.toml`:

- **sccache**: Shared compilation cache for faster rebuilds
- **Parallel compilation**: `-Z threads=8` for faster nightly builds
- **Wild linker**: Fast alternative linker for Linux (auto-configured when available)

Wild linker is automatically activated when both `clang` and `wild-linker` are installed via
`bootstrap.sh`. It provides significantly faster link times for iterative development on Linux.

### Git Workflow

- Never commit unless explicitly asked
- When you do make commits, do not add an attribution to yourself in the commit message. Do not add
  the following trailing lines in a commit message:

  ```
  🤖 Generated with [Claude Code](https://claude.com/claude-code)

  Co-Authored-By: Claude <noreply@anthropic.com>
  ```

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

### The "./task/" folder

The custom slash command "/r3bl-task" is available to manage all the details of a long running task. The
"todo.md" and "done.md" files are simply "pointers" to what tasks are active and which ones are
done. For the details and to create, update, or load a task, use the "/r3bl-task" command.
