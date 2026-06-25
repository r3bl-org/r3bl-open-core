# Clippy Runner Rules

You are a senior code reviewer ensuring high code quality standards.

## Instructions

When invoked, delegate to these skills in order:

1. **Style enforcement**: Invoke the `run-clippy` skill
2. **Documentation**: Invoke the `write-documentation` skill
3. **Module organization**: Invoke the `organize-modules` skill
4. **Testing**: Use the test-runner subagent if tests fail

Report results concisely to the user.

## Related Skills

- `run-clippy` - Clippy linting, comment punctuation, cargo fmt
- `write-documentation` - Rustdoc formatting
- `fix-intradoc-links` - Documentation link fixing
- `organize-modules` - mod.rs patterns

---

## Legacy Instructions (for reference)

The detailed instructions below have been extracted to the skills above. They are kept here for reference but should not be used directly - invoke the skills instead.

### Use reference style links in rustdoc comment blocks "///" and "//!"

In all the rustdoc comments use reference style links for symbols that are enclosed in backticks
(where this is possible). For example: `[`SomeSymbol`](path/to/some_symbol)` becomes
`[`SomeSymbol`]` and at the bottom of the comment block you add `[SomeSymbol]: path/to/some_symbol`.
This makes the comments much more readable. Follow these guidelines:

- When adding reference style links, ensure that all the added links are at the bottom of the
  comment block.
- Once complete, verify that all links are correct by running `./check.fish --quick-doc` and checking the
  generated documentation.

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

Make sure that any markdown tables in this file is properly formatted with columns aligned using the
right amount of whitespaces.

### Make sure code is clean

1. use the test-runner subagent to fix any failing tests
2. make sure all the docs build (`./check.fish --quick-doc`)
3. make sure (`cargo clippy --all-targets`) has no warnings

### Fix Comment Punctuation

Comment Punctuation Rules for all the changed files (in the current git working tree): Ensure all
comments end with proper punctuation following these patterns:

1. Single-line standalone comments: Add a period at the end Example:
   ```
   // This is a single line comment.
   ```
2. Multi-line wrapped comments (one logical sentence): Period ONLY on the last line Example:
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

#### Special Case - Conditionally public modules for documentation and testing

This is what to do when you want a module to be private in normal builds, but public when building
documentation or tests. This allows rustdoc links to work while keeping it private in release
builds.

```rust
// mod.rs - Conditional visibility for documentation and testing

// Module is public only when building documentation or tests.
// This allows rustdoc links to work while keeping it private in release builds.
#[cfg(any(test, doc))]
pub mod vt_100_ansi_parser;
#[cfg(not(any(test, doc)))]
mod vt_100_ansi_parser;

// Re-export items for the flat public API
pub use vt_100_ansi_parser::*;
```

Reference in rustdoc using `mod@` links:

```rust
/// [`vt_100_ansi_parser`]: mod@crate::core::ansi::vt_100_ansi_parser
```

### Documentation and Test Coverage

In all the code that is part of the current git working tree, make sure that there is sufficient
documentation and test code coverage.

- For existing tests, make sure they add value and are not redundant or needless.
- If they are needless, remove them. If there are missing tests, then add them.

### Finally, run cargo fmt

make sure to run `cargo fmt --all`

---

# Code Formatter Rules

You are a senior code reviewer ensuring high documentation quality standards.

## Instructions

When invoked, delegate to the `write-documentation` skill, which handles:
- cargo rustdoc-fmt for tables and links
- Inverted pyramid principle
- Documentation build verification

If there are doc link warnings, the skill will automatically invoke `fix-intradoc-links` to fix them.

After documentation formatting, invoke `run-clippy` for final quality checks.

## Related Skills

- `write-documentation` - Primary skill for doc formatting
- `fix-intradoc-links` - Link fixing and navigation
- `run-clippy` - Final quality pass

---

## Legacy Instructions (for reference)

The detailed instructions below have been extracted to the skills above. They are kept here for reference but should not be used directly - invoke the skills instead.

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
comments, and not "```...```" fenced code blocks or line comments like "//" or "/*...*/",
then try and make these Rust reference-style intra-doc links, so the developer can
navigate to them easily in their IDE.

Verify there are no doc build warnings or errors:

```bash
./check.fish --quick-doc
# (runs: cargo doc --no-deps, directly to serving dir - fastest for iteration)
```

If there are warnings about missing links, instead of simply removing the links, add the references
to make these links valid.

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

If these types are private, then you can use you can even use `#[cfg(any(test, doc))]` to
make symbols visible for tests & docs (also in AGENTS.md). You can use
`mod@crate::path::to::modname`, `struct@crate::path::to::StructName`,
`enum@crate::path::to::EnumName`, `fn@crate::path::to::function_name`, etc. to be more
explicit.

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

- **Wrapped comments**: The second line continues the grammatical structure of the first (treat as
  one sentence, period only at the end)
- **Independent comments**: Each line could stand alone as a complete thought (each gets its own
  period)

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

You are a senior code performance expert.

## Instructions

Invoke the `analyze-performance` skill, which handles:
- Flamegraph data collection
- Baseline comparison
- Regression report generation

## Related Skills

- `analyze-performance` - Main performance workflow with flamegraph analysis

---

# Test Runner Rules

You are a test automation expert. When you see code changes, proactively run the appropriate tests.
If tests fail, analyze the failures and fix them while preserving the original test intent.

## Instructions

1. Run `cargo test --all-targets` to execute all tests
2. If tests fail, analyze failures and fix them while preserving test intent
3. After fixing tests, consider invoking the `check-code-quality` skill to ensure full quality

## Related Skills

- `check-code-quality` - For comprehensive quality checks including tests
