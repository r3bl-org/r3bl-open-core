---
name: run-clippy
description: Run clippy linting, enforce comment punctuation rules, format code with cargo fmt, and verify module organization patterns. Use after code changes and before creating commits.
---

# Clippy Fixes and Code Style Enforcement

## When to Use

- After making code changes
- Fixing clippy warnings
- Ensuring comment punctuation consistency
- Enforcing module organization patterns
- Before creating commits
- When user says "run clippy", "fix style", "format code", "check lints", etc.

## Instructions

Run these steps in order to enforce code quality and style standards:

### Step 1: Run Clippy

Run clippy to catch linting issues:

```bash
./check.fish --clippy
# (runs: cargo clippy --all-targets)
```

**Review and fix all warnings.** For auto-fixable issues:

```bash
cargo clippy --all-targets --fix --allow-dirty
```

**Common clippy categories:**
- **Correctness**: Potential bugs and logic errors (must fix)
- **Performance**: Inefficient code patterns (should fix)
- **Style**: Idiomatic Rust patterns (should fix)
- **Pedantic**: Opinionated style (optional, review case-by-case)
- **Complexity**: Overly complex code (refactor if excessive)

### Step 2: Enforce Comment Punctuation

Apply these punctuation rules to **all comments** (not rustdoc `///` or `//!`, but regular `//` comments) in the git working tree:

#### Rule 1: Single-line Standalone Comments

Add a period at the end:

```rust
// This is a single line comment.
```

#### Rule 2: Multi-line Wrapped Comments (One Logical Sentence)

Period ONLY on the last line:

```rust
// This is a long line that wraps
// to the next line.
```

#### Rule 3: Multiple Independent Single-line Comments

Each gets its own period:

```rust
// First independent thought.
// Second independent thought.
```

#### How to Identify Wrapped vs Independent

**Wrapped comments:**
- The second line continues the grammatical structure of the first
- Reads as one sentence if combined

**Independent comments:**
- Each line could stand alone as a complete thought
- Separate sentences with distinct subjects

### Step 3: Enforce Clean Imports (No Inline Absolute Paths)

Scan the changed files for any inline absolute paths (e.g., `crate::Type`, `crate::function!`) and replace them with proper `use` statements at the top of the file.

**✅ Good:**
```rust
use crate::{Size, Pos};

pub fn render(size: Size) -> Pos { ... }
```

**❌ Bad:**
```rust
pub fn render(size: crate::Size) -> crate::Pos { ... }
```

*Note: Intra-doc links like `/// [`Type`]: crate::Type` are exempt from this rule and SHOULD use `crate::` paths.*

### Step 4: Verify Module Organization

Review all `mod.rs` files in the git working tree and ensure they follow the patterns from the `organize-modules` skill:

**Check for:**
- ✅ Private modules with public re-exports (preferred pattern)
- ✅ Conditional visibility for docs/tests where needed: `#[cfg(any(test, doc))]`
- ✅ Rustfmt skip directive if manual formatting is needed
- ✅ Flat public API (avoid exposing internal structure)

If module organization doesn't follow patterns, invoke the `organize-modules` skill for guidance.

### Step 5: Verify Documentation Quality

If working with rustdoc comments (`///` or `//!`):

1. **Reference-style links**: Ensure backticked symbols use reference-style intra-doc links
2. **Link placement**: All reference-style links at bottom of comment block
3. **Table formatting**: Markdown tables properly aligned

If there are issues, invoke the `write-documentation` skill (or use `/fix-intradoc-links` command).

### Step 5: Run Tests if Needed

If clippy fixes modify behavior or you're unsure about changes:

```bash
./check.fish --test
# (runs: cargo test --all-targets)
```

Use the Task tool with `subagent_type='test-runner'` if tests fail.

### Step 6: Final Code Formatting

Run cargo fmt to ensure consistent formatting:

```bash
cargo fmt --all
```

This applies:
- Consistent indentation (4 spaces)
- Line length limits (100 chars default)
- Spacing around operators and braces
- Import organization

### Step 7: Enforce Clean Imports

Ensure that code does not use absolute inline paths like `crate::Type` or `crate::CONSTANT` inside function signatures or bodies. They must be cleanly imported via `use` statements at the top of the file.

### Step 8: Handling Unwraps and Panics

When addressing `clippy::unwrap_used` violations or manually reviewing code that can mathematically never fail:
- **Avoid `.expect("...")`**. While `.expect()` provides a panic message at runtime, we prefer to keep string literals out of the logic flow for a cleaner aesthetic.
- **Use `#[allow(clippy::unwrap_used, reason = "...")]`** instead of `.expect()`. The reason should explicitly state the mathematical or logical proof for why the `unwrap()` is safe.
- **Fallbacks**: If the `unwrap()` is not mathematically proven safe and is truly fallible (e.g., file I/O), rewrite it using `if let Ok()` or propagate the error instead of panicking.

## Reporting Results

After completing all steps, report concisely:

- ✅ All checks passed → "Code style and linting checks passed!"
- ⚠️ Clippy warnings → Summarize warning categories and counts
- 🔧 Auto-fixed → Report what was automatically corrected
- 📝 Manual fixes needed → List what requires developer attention

## Supporting Files in This Skill

This skill includes additional reference material:

- **`patterns.md`** - Comprehensive examples of code style patterns including comment punctuation rules (20+ examples), clippy lint categories with fixes, cargo fmt formatting rules, and module organization quick checks. **Read this when:**
  - Need examples of proper comment punctuation (wrapped vs independent)
  - Understanding clippy lint categories (correctness, performance, style)
  - See good vs bad examples for specific lints
  - Quick module organization checks
  - Understanding cargo fmt rules (indentation, line length, imports)

## Related Skills

- `check-code-quality` - Includes clippy as part of full quality checks
- `organize-modules` - For module organization patterns
- `write-documentation` - For rustdoc link formatting and comprehensive doc formatting

## Related Commands

- `/clippy` - Explicitly invokes this skill
- `/fix-comments` - Focuses on comment punctuation (subset of this skill)

## Related Agents

- `clippy-runner` - Agent that delegates to this skill
