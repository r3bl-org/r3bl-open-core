---
name: write-documentation
description: Write and format Rust documentation using inverted pyramid principle and cargo rustdoc-fmt for tables/formatting. Use when writing documentation comments, fixing rustdoc formatting issues, or before creating commits with doc changes.
---

# Writing Good Rust Documentation

## When to Use

- Writing new documentation comments for modules, traits, structs, or functions
- Improving existing documentation clarity
- Formatting markdown tables in rustdoc
- Fixing rustdoc build warnings or formatting issues
- Before creating commits with documentation changes
- When user says "write docs", "document this", "add documentation", "format docs", etc.

## Instructions

Follow these steps to write high-quality Rust documentation:

### Step 1: Apply the Inverted Pyramid Principle

Structure documentation with high-level concepts at the top, details below:

```
╲────────────╱
 ╲          ╱  High-level concepts - Module/trait/struct documentation
  ╲────────╱
   ╲      ╱  Mid-level details - Method group documentation
    ╲────╱
     ╲  ╱  Low-level specifics - Individual method documentation
      ╲╱
```

**Avoid making readers hunt through method docs for the big picture.**

#### Example Placement Guidelines

**Trait/Module level:**
- Place conceptual examples showing **why** and **when** to use the API
- Include complete workflows showing typical usage patterns
- Add visual diagrams (ASCII art) to illustrate concepts
- Document common mistakes and antipatterns
- These examples **teach the concept**

**Method level:**
- Place minimal syntax examples showing **how** to call the specific method
- Show exact types and parameters
- Keep it brief for IDE tooltips
- These examples serve as **quick reference**

**Graduated complexity:**
- Examples should match the abstraction level
- Comprehensive scenarios at trait/module level
- Simple syntax at method level

**Avoid duplication:**
- Don't repeat full examples between trait and method docs
- Reference the trait/module docs from methods when detailed examples already exist there

Example:
```rust
/// See the [module-level documentation](mod@crate::example) for complete usage examples.
pub fn some_method(&self) -> Result<()> { /* ... */ }
```

### Step 2: Use ASCII Diagrams and Code Examples

**ASCII Diagrams:**
Use wherever possible to illustrate concepts visually:

```rust
//! # State Machine
//!
//! ```text
//! ┌─────────┐
//! │  Start  │
//! └────┬────┘
//!      │
//!      ▼
//! ┌─────────┐      ┌─────────┐
//! │ Process │─────▶│  Done   │
//! └─────────┘      └─────────┘
//! ```
```

**Code Examples:**
Use extensively to demonstrate usage patterns. **All examples must compile or run.**

### Step 3: Handle Code Examples Properly

**Golden Rule:** Don't use `ignore` unless absolutely necessary.

#### For Regular Code (Not Macros)

**Option 1: Convert to compilable code**

```rust
/// Example that compiles and runs:
/// ```
/// use my_crate::add;
/// let result = add(2, 3);
/// assert_eq!(result, 5);
/// ```
```

**Option 2: Use `no_run` for examples that compile but shouldn't execute**

```rust
/// Example that compiles but doesn't run (e.g., requires external setup):
/// ```no_run
/// use my_crate::start_server;
/// start_server("0.0.0.0:8080")?;  // Would block
/// ```
```

**Option 3: Link to real code instead**

If you can't make it compile, remove the code block and link to actual implementation:

```rust
/// See [`example_usage`] for a working example.
///
/// [`example_usage`]: crate::tests::example_usage
```

Use `#[cfg(any(test, doc))]` to make private test functions visible to docs.

#### For Macros

Macro expansion issues often prevent doctests from working.

**Preferred: Link to real usage**

```rust
/// See [`test_example`] for actual usage of this macro.
///
/// [`test_example`]: crate::tests::test_example
```

**If showing syntax is essential: Use ignore with HTML comment**

```rust
//! <!-- It is ok to use ignore here, as this is a macro call -->
//! ```ignore
//! generate_pty_test! {
//!     test_fn: interactive_input_parsing,
//! }
//! ```
```

**This HTML comment documents that the `ignore` marker is intentional, not an oversight.**

### Step 4: Format with cargo rustdoc-fmt

Run the rustdoc formatter to fix markdown tables and convert inline links:

```bash
# Format specific file (most common during development)
cargo rustdoc-fmt path/to/file.rs

# Format all git-changed files
cargo rustdoc-fmt

# Format entire workspace
cargo rustdoc-fmt --workspace
```

**What it does:**
- Formats markdown tables with proper column alignment
- Converts `[foo](path)` inline links to reference-style `[foo]` with `[foo]: path` at bottom
- Preserves code examples and other content

**If cargo rustdoc-fmt is not installed:**

```bash
cd build-infra
cargo install --path . --force
```

### Step 5: Verify Documentation Builds

Run cargo doc to ensure no warnings or errors:

```bash
cargo doc --no-deps
```

**Common issues:**

1. **Broken intra-doc links** → Invoke the `fix-intradoc-links` skill
2. **Malformed markdown tables** → Run `cargo rustdoc-fmt` again
3. **Invalid code examples** → Fix or convert to `no_run` or remove
4. **Missing references** → Add reference-style link definitions

### Step 6: Review Documentation Quality

Before finalizing, check:

- [ ] High-level concepts at module/trait level (inverted pyramid)
- [ ] Simple syntax examples at method level
- [ ] ASCII diagrams where helpful
- [ ] All code examples compile (`cargo test --doc`)
- [ ] No broken links (`cargo doc --no-deps`)
- [ ] Markdown tables formatted correctly
- [ ] Reference-style links at bottom of comment blocks

## Reporting Results

After writing/formatting documentation:

- ✅ Docs build cleanly → "Documentation formatted and verified!"
- ⚠️ Build warnings → Summarize link errors or formatting issues
- 🔧 Auto-formatted → Report what `cargo rustdoc-fmt` fixed
- 📝 Manual fixes needed → List what requires attention

## Supporting Files in This Skill

This skill includes additional reference materials:

- **`rustdoc-formatting.md`** - Deep dive into cargo rustdoc-fmt: installation, usage modes (file/changed/workspace), troubleshooting, integration with workflow, and best practices. **Read this when:**
  - Installing or troubleshooting cargo rustdoc-fmt
  - Understanding the three usage modes (specific files, changed files, workspace)
  - cargo rustdoc-fmt command not found
  - Integrating with CI/CD pipelines
  - Understanding what the formatter does and doesn't touch

- **`examples.md`** - 5 complete, production-quality documentation examples demonstrating the inverted pyramid principle for modules, traits, structs, functions, and complex APIs. Each example shows trait/module/struct/function-level docs with ASCII diagrams, code examples, and pitfall guidance. **Read this when:**
  - Need to see inverted pyramid principle in action
  - Writing module-level documentation (Example 1)
  - Documenting a trait with implementation examples (Example 2)
  - Documenting a struct with internal architecture (Example 3)
  - Writing function documentation with argument/return/error docs (Example 4)
  - Documenting complex modules with graduated complexity (Example 5)

## Related Skills

- `fix-intradoc-links` - For fixing broken doc links and adding navigation
- `check-code-quality` - Includes doc verification step
- `organize-modules` - For understanding re-export chains and conditional visibility
- `run-clippy` - May suggest doc improvements

## Related Commands

- `/docs` - Explicitly invokes this skill
- `/fix-md-tables` - Focuses on table formatting (subset of this skill)

## Related Agents

- `code-formatter` - Agent that delegates to this skill
