# cargo rustdoc-fmt Workflow

This document provides detailed guidance on using `cargo rustdoc-fmt`, the custom tool from `build-infra` that formats rustdoc comments.

## What is cargo rustdoc-fmt?

`cargo rustdoc-fmt` is a custom formatting tool specifically for Rust documentation comments (`///` and `//!`). It automates two tedious tasks:

1. **Markdown table formatting** - Aligns columns with proper whitespace
2. **Link conversion** - Converts inline links to reference-style links

## Installation

The tool is part of the `build-infra` crate:

```bash
cd build-infra
cargo install --path . --force
```

**Important:** After making code changes to `build-infra`, you MUST reinstall the binary:

```bash
cd build-infra
cargo install --path . --force
```

This updates the installed binary in `~/.cargo/bin`.

## Usage Modes

### Mode 1: Format Specific Files (Most Common)

Format one or more specific files during development:

```bash
cargo rustdoc-fmt path/to/file.rs
cargo rustdoc-fmt tui/src/core/units/mod.rs
cargo rustdoc-fmt tui/src/terminal_lib_backends/offscreen_buffer.rs another/file.rs
```

**When to use:**
- Working on specific modules
- Iterative documentation improvements
- Quick formatting of changed files

### Mode 2: Format Git-Changed Files (Default)

Format all files with uncommitted changes:

```bash
cargo rustdoc-fmt
```

**What it does:**
- Runs `git diff --name-only` to find changed files
- Filters for `*.rs` files
- Formats only the changed Rust files

**When to use:**
- Before committing changes
- After a documentation writing session
- As part of pre-commit quality checks

### Mode 3: Format Entire Workspace

Format all Rust files in the workspace:

```bash
cargo rustdoc-fmt --workspace
```

**When to use:**
- Major documentation refactoring
- Establishing consistent formatting across the codebase
- One-time cleanup of legacy docs

**Warning:** This can modify many files. Review changes carefully before committing.

## What It Formats

### Markdown Table Formatting

**Before:**
```rust
/// | Name | Type | Description |
/// |------|------|-------------|
/// | foo | `u32` | A number |
/// | bar | `String` | Some text |
```

**After (aligned columns):**
```rust
/// | Name | Type     | Description |
/// |------|----------|-------------|
/// | foo  | `u32`    | A number    |
/// | bar  | `String` | Some text   |
```

**Benefits:**
- Easier to read in source code
- Clearer column alignment
- Professional appearance

### Inline to Reference-Style Link Conversion

**Before:**
```rust
/// See [`SomeType`](crate::path::to::SomeType) for details.
/// Also check [`OtherType`](crate::other::OtherType).
```

**After:**
```rust
/// See [`SomeType`] for details.
/// Also check [`OtherType`].
///
/// [`SomeType`]: crate::path::to::SomeType
/// [`OtherType`]: crate::other::OtherType
```

**Benefits:**
- Cleaner prose in documentation
- Links grouped at bottom for easy maintenance
- Easier to read the narrative content
- IDE can still navigate to definitions

## What It Does NOT Format

`cargo rustdoc-fmt` only processes **rustdoc comments** (`///` and `//!`). It does NOT touch:

- Regular comments (`//` and `/* */`)
- Code blocks (` ```rust ... ``` `)
- Regular Rust code
- Non-rustdoc markdown files

For code formatting, use `cargo fmt --all`.

## Integration with Workflow

### Typical Development Workflow

1. **Write documentation:**
   ```rust
   /// This does something with [`Foo`](crate::Foo).
   ///
   /// | Param | Type | Desc |
   /// |---|---|---|
   /// | x | u32 | number |
   ```

2. **Format it:**
   ```bash
   cargo rustdoc-fmt path/to/current/file.rs
   ```

3. **Review the result:**
   ```rust
   /// This does something with [`Foo`].
   ///
   /// | Param | Type  | Desc   |
   /// |-------|-------|--------|
   /// | x     | u32   | number |
   ///
   /// [`Foo`]: crate::Foo
   ```

4. **Verify docs build:**
   ```bash
   cargo doc --no-deps
   ```

### Pre-Commit Checklist

Before committing documentation changes:

```bash
# 1. Format changed files
cargo rustdoc-fmt

# 2. Verify no doc build errors
cargo doc --no-deps

# 3. Run doctests
cargo test --doc

# 4. Format regular code
cargo fmt --all

# 5. Review changes
git diff
```

## Troubleshooting

### Issue: "cargo rustdoc-fmt: command not found"

**Solution:** Install the tool:

```bash
cd build-infra
cargo install --path . --force
```

Verify installation:
```bash
which cargo-rustdoc-fmt
# Should show: /home/username/.cargo/bin/cargo-rustdoc-fmt
```

---

### Issue: Changes to build-infra code not reflected

**Solution:** The installed binary is cached. Reinstall:

```bash
cd build-infra
cargo install --path . --force
```

---

### Issue: Formatting looks wrong after running

**Possible causes:**

1. **Table has inconsistent columns:**
   - Ensure all rows have the same number of columns
   - Check for missing `|` delimiters

2. **Code blocks interfering:**
   - The tool shouldn't touch code blocks
   - If it does, file a bug report

3. **Complex markdown:**
   - Nested tables or unusual markdown may not format perfectly
   - Consider simplifying the structure

---

### Issue: "No files to format"

**Causes:**
- No uncommitted changes (when running without arguments)
- Changed files are not `*.rs` files
- Git working directory is clean

**Solution:** Either:
- Make changes to Rust files
- Specify files explicitly: `cargo rustdoc-fmt path/to/file.rs`
- Use `--workspace` to format all files

## Best Practices

### 1. Format Incrementally

Don't wait until the end of a large documentation session:

```bash
# After documenting each module
cargo rustdoc-fmt tui/src/module_i_just_documented.rs
```

This keeps diffs small and reviewable.

### 2. Review Before Committing

Always review the changes:

```bash
cargo rustdoc-fmt
git diff
```

Ensure the formatter didn't inadvertently change semantics.

### 3. Use in Conjunction with fix-intradoc-links

The formatter converts inline links to reference-style, but doesn't create links.

If you have symbols in backticks that should be links:

```bash
# 1. Format first
cargo rustdoc-fmt

# 2. Then fix/add links
# (Invoke fix-intradoc-links skill)
```

### 4. Integrate with CI/CD

Consider adding a CI check to ensure docs are formatted:

```bash
# In CI script
cargo rustdoc-fmt --workspace
git diff --exit-code  # Fails if there are uncommitted changes
```

This enforces formatting standards across all contributors.

## Advanced Usage

### Combining with Other Tools

```bash
# Full doc quality workflow
cargo rustdoc-fmt                    # Format docs
cargo doc --no-deps                  # Verify builds
cargo clippy --all-targets           # Check code quality
cargo test --doc                     # Run doctests
cargo fmt --all                      # Format code
```

### Scripting

Create a script for common workflows:

```bash
#!/bin/bash
# doc-format.sh

echo "Formatting rustdoc comments..."
cargo rustdoc-fmt

echo "Verifying documentation builds..."
cargo doc --no-deps

echo "Running doctests..."
cargo test --doc

echo "✅ Documentation workflow complete!"
```

Make it executable:
```bash
chmod +x doc-format.sh
./doc-format.sh
```

## Summary

`cargo rustdoc-fmt` is your ally for maintaining beautiful, consistent Rust documentation:

- ✅ Formats markdown tables with aligned columns
- ✅ Converts inline links to reference-style
- ✅ Works on specific files, changed files, or entire workspace
- ✅ Integrates smoothly with cargo doc and other tools
- ✅ Installed from build-infra crate

Use it early and often for professional documentation!
