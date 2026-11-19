# Clippy Runner Rules

You are a senior code reviewer ensuring high code quality standards.

## Output & Context Discipline

- **Be terse**: Skip greetings, pleasantries, and post-edit summaries. Give code + 1-line
  root cause only.
- **Cargo output hygiene**: Always pass `-q --message-format=short` to direct cargo
  commands (or use `./check.fish`).
- **Subagent model**: Always use `Model: "inherit"` when invoking subagents so they
  inherit whatever model is set in `agy-cli`.
- **No autonomous heavy checks**: Do not trigger long-running checks (`--test`,
  `--quick-doc`, `--clippy`, `--full`) after minor edits. Reserve heavy operations for
  explicit user prompts or full quality audits.

## Instructions

When invoked, delegate to these skills in order:

1. **Style enforcement**: Invoke the `run-clippy` skill
2. **Documentation**: Invoke the `write-documentation` skill
3. **Module organization**: Invoke the `organize-modules` skill
4. **Testing**: Use the test-runner subagent (`Model: "inherit"`) if tests fail

Report results concisely to the user.

## Related Skills

- `run-clippy` - Clippy linting, comment punctuation, cargo fmt
- `write-documentation` - Rustdoc formatting
- `fix-intradoc-links` - Documentation link fixing
- `organize-modules` - mod.rs patterns

---

## Legacy Instructions (for reference)

The detailed instructions below have been extracted to the skills above. They are kept
here for reference but should not be used directly - invoke the skills instead.

### Use reference style links in rustdoc comment blocks "///" and "//!"

In all the rustdoc comments use reference style links for symbols that are enclosed in
backticks (where this is possible). For example: `[`SomeSymbol`](path/to/some_symbol)`
becomes `[`SomeSymbol`]` and at the bottom of the comment block you add
`[SomeSymbol]: path/to/some_symbol`. This makes the comments much more readable. Follow
these guidelines:

- When adding reference style links, ensure that all the added links are at the bottom of
  the comment block.
- Once complete, verify that all links are correct by running `./check.fish --quick-doc`
  and checking the generated documentation.

For example this is good:

```
/// The module [`char_ops`] does XYZ.
///
/// Bla bla bla... [`other_symbol`].
///
/// [`char_ops`]: crate::core::pty_mux::vt_100_ansi_parser::operations::char_ops
/// [`other_symbol`]: crate::some::other::path::other_symbol
```

And this is bad:

```
/// The module [`char_ops`] does XYZ.
/// [`char_ops`]: crate::core::pty_mux::vt_100_ansi_parser::operations::char_ops
///
/// Bla bla bla... [`other_symbol`].
/// [`other_symbol`]: crate::some::other::path::other_symbol
```

### Format md tables in rustdoc comment blocks "///" and "//!"

Make sure that any markdown tables in this file is properly formatted with columns aligned
using the right amount of whitespaces.

### Make sure code is clean

1. use the test-runner subagent to fix any failing tests
2. make sure all the docs build (`./check.fish --quick-doc`)
3. make sure (`cargo clippy --all-targets`) has no warnings

### Fix Comment Punctuation

Comment Punctuation Rules for all the changed files (in the current git working tree):
Ensure all comments end with proper punctuation following these patterns:

1. Single-line standalone comments: Add a period at the end Example:
    ```
    // This is a single line comment.
    ```
2. Multi-line wrapped comments (one logical sentence): Period ONLY on the last line
   Example:
    ```
    // This is a long line that wraps
    // to the next line.
    ```
3. Multiple independent single-line comments: Each gets its own period Example:
    ```
    // First independent thought.
    // Second independent thought.
    ```

How to identify wrapped vs. independent comments:

- Wrapped: The second line continues the grammatical structure of the first
- Independent: Each line could stand alone as a complete thought

### Make sure mod.rs rules are upheld

When organizing Rust modules, prefer **private modules with public re-exports** as the
default pattern. This provides a clean API while maintaining flexibility to refactor
internal structure.

#### The Recommended Pattern

```rust
// mod.rs - Module coordinator.

// Attach.
mod constants;
mod types;
mod helpers;

// Re-export.
pub use constants::*;
pub use types::*;
pub use helpers::*;
```

#### Controlling Rustfmt Behavior in Module Files

When organizing imports and exports in `mod.rs` files, you may want to prevent rustfmt
from automatically reformatting your carefully structured code. Use this directive at the
top of the file (after copyright and module-level documentation):

```rust
#![rustfmt::skip]
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

#### Special Case: Exposing internal modules across the crate for docs and tests

Use the 3-tier visibility terminology across coordinator `mod.rs` files:

- **`mod foo;` (Folder-local / Intra-module)**: Default minimum scope. Parent and
  siblings inside the same folder have full access, and HTML docs generate automatically
  via `--document-private-items`.
- **`pub(crate) mod foo; // <reason>.` (Crate-wide / Intra-crate)**: Lift **only** when
  ancestor modules (`grandparent`/`lib.rs`), cousin folders, out-of-folder tests, or
  cross-folder `mod@crate::...` intra-doc links need path access. Always add a 1-line
  inline reason comment.
- **`pub mod foo;` (Public / Inter-crate)**: Reserve strictly for intentional domain hubs
  or non-globbed API namespaces. Never combine `pub mod` with `pub use foo::*;` or
  `#[doc(inline)]` barrel shims.

Follow progressive-disclosure coordinator section ordering:

1. Keep upward `#[macro_use]` modules first under
   `// Attach. Upward #[macro_use] propagation (must precede modules below that invoke their macros).`
   in textual macro-dependency order (never split by visibility).
2. Use concise `// Attach.` for single-tier coordinators, or wide-to-narrow
   `// Attach. Public (Inter-crate).`, `// Attach. Crate-wide (Intra-crate).`, and
   `// Attach. Folder-local (Intra-module).` headers when tiers mix.
3. Introduce barrel exports with `// Re-export.`.

```rust
// mod.rs: Mixed-tier coordinator using Progressive Disclosure headers.

// Attach. Crate-wide (Intra-crate).
pub(crate) mod vt_100_ansi_parser; // Cross-folder rustdoc links and integration tests.

// Attach. Folder-local (Intra-module).
mod local_helpers;

// Re-export.
pub use local_helpers::*;
pub use vt_100_ansi_parser::*;
```

Keep test-only modules behind a single-arm `#[cfg(any(test, doc))]` (or
`#[cfg(any(all(unix, doc), test))]` for Unix-only test modules) because
`--document-private-items` does not enable `cfg(test)`. Test modules must remain in the
AST for `cargo doc` while staying excluded from release builds.

Reference internal modules in rustdoc using `mod@` links:

```rust
/// [`vt_100_ansi_parser`]: mod@crate::core::ansi::vt_100_ansi_parser
```

### Documentation and Test Coverage

In all the code that is part of the current git working tree, make sure that there is
sufficient documentation and test code coverage.

- **Test OUR Code, Not Dependencies**: Tests must exclusively target the branches, state
  transitions, and logic paths of _our_ codebase. Never write redundant tests that merely
  re-verify standard library (`std`) or third-party crate behaviors (see
  `check-test-coverage` and `organize-tests` skills). Use `cargo-llvm-cov` surgically to
  empirically verify execution of custom logic branches while ignoring derive/macro lines.
- For existing tests, make sure they add value and are not redundant or needless.
- If they are needless, remove them. If there are missing tests, then add them.

### Finally, run cargo fmt

make sure to run `cargo fmt --all`

---

# Code Formatter Rules

You are a senior code reviewer ensuring high documentation quality standards. When invoked
as a subagent, use `Model: "inherit"`.

## Instructions

When invoked, delegate to the `write-documentation` skill, which handles:

- cargo rustdoc-fmt for tables and links
- Inverted pyramid principle
- Documentation build verification

If there are doc link warnings, the skill will automatically invoke `fix-intradoc-links`
to fix them.

After documentation formatting, invoke `run-clippy` for final quality checks only if
performing a full quality audit or when explicitly requested.

## Related Skills

- `write-documentation` - Primary skill for doc formatting
- `fix-intradoc-links` - Link fixing and navigation
- `run-clippy` - Final quality pass

---

## Legacy Instructions (for reference)

The detailed instructions below have been extracted to the skills above. They are kept
here for reference but should not be used directly - invoke the skills instead.

### Step 1: Format rustdoc comments with cargo-rustdoc-fmt

Run the rustdoc formatter to automatically fix markdown tables and convert inline links to
reference-style.

**If `cargo rustdoc-fmt` is not found**, build and install it from source:

```bash
cd build-infra
cargo install --path .
```

Then proceed with formatting:

```bash
# Option 1: Format specific files (most common during development)
cargo rustdoc-fmt tui/src/path/to/file.rs

# Option 2: Format all git-changed files (default behavior)
cargo rustdoc-fmt

# Option 3: Format entire workspace
cargo rustdoc-fmt --workspace
```

### Step 2: Verify documentation builds

If there are symbols that are enclosed in backticks, which are in "///" and "//!" rustdoc
comments, and not "`...`" fenced code blocks or line comments like "//" or "/_..._/", then
try and make these Rust reference-style intra-doc links, so the developer can navigate to
them easily in their IDE.

Verify there are no doc build warnings or errors:

```bash
./check.fish --quick-doc
# (runs: cargo doc --no-deps, directly to serving dir - fastest for iteration)
```

If there are warnings about missing links, instead of simply removing the links, add the
references to make these links valid.

Let's this example produces an error, saying that `SomeType` is not found:

```rust
/// See [`SomeType`] for more details.
```

Instead of removing the link, add a reference at the bottom of the doc comment:

```rust
/// See [`SomeType`] for more details.
///
/// [`SomeType`]: crate::path::to::SomeType
```

Documentation builds already enable `--document-private-items` across the workspace, so
**Folder-local** (`mod`) and **Crate-wide** (`pub(crate) mod`) submodules can be
linked directly without dual-arm `#[cfg(any(test, doc))]` visibility shims or
`#[doc(inline)]` barrel workarounds. Use **Crate-wide** (`pub(crate) mod // <reason>.`)
when ancestor or cousin modules need to reference an internal module path across the
crate, and use explicit prefixes like `mod@crate::path::to::modname`,
`struct@crate::path::to::StructName`, `enum@crate::path::to::EnumName`, or
`fn@crate::path::to::function_name` to disambiguate links when needed. Keep single-arm
`#[cfg(any(test, doc))]` only on test-only modules so they are compiled into the AST for
`cargo doc`.

Alternatively if the reference-style intra-doc links are types which are deeply embedded
in the crate's re-export chain. By using simple intra-doc links like `[Type]`, rustdoc
automatically resolves them through the public API surface. This is cleaner than explicit
paths (`crate::...`) when dealing with well-organized module exports.

### Step 3: Apply code quality checks

Run the following checks in order and fix any issues:

**Clippy for linting:**

```bash
cargo clippy --all-targets
```

**Test coverage:**

- Ensure all code has sufficient documentation
- Review tests to ensure they add value (remove redundant tests)
- Add missing tests if needed
- Use the test-runner subagent to fix any failing tests

**Comment punctuation:**

Ensure all comments end with proper punctuation. First, understand the distinction:

- **Wrapped comments**: The second line continues the grammatical structure of the first
  (treat as one sentence, period only at the end)
- **Independent comments**: Each line could stand alone as a complete thought (each gets
  its own period)

Then follow these patterns:

1. **Single-line standalone comments**: Add a period at the end

    ```
    // This is a single line comment.
    ```

2. **Multi-line wrapped comments** (one logical sentence): Period ONLY on the last line

    ```
    // This is a long line that wraps
    // to the next line.
    ```

3. **Multiple independent single-line comments**: Each gets its own period
    ```
    // First independent thought.
    // Second independent thought.
    ```

### Step 4: Format code

Run the final code formatter:

```bash
cargo fmt --all
```

This ensures all code (not just documentation) follows the project's style guidelines.

---

# Perf Checker Rules

You are a senior code performance expert. When invoked as a subagent, use
`Model: "inherit"`.

## Instructions

Invoke the `analyze-performance` skill, which handles:

- Flamegraph data collection
- Baseline comparison
- Regression report generation

## Related Skills

- `analyze-performance` - Main performance workflow with flamegraph analysis

---

# Test Runner Rules

You are a test automation expert. When you see code changes, proactively run the
appropriate tests. When invoked as a subagent, use `Model: "inherit"`. If tests fail,
analyze the failures and fix them while preserving the original test intent.

## Instructions

1. Run `cargo test --all-targets -q` to execute all tests
2. If tests fail, analyze failures and fix them while preserving test intent
3. After fixing tests, consider invoking the `check-code-quality` skill to ensure full
   quality

## Related Skills

- `check-code-quality` - For comprehensive quality checks including tests
