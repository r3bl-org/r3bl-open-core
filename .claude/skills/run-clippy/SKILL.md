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
cargo clippy --all-targets
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

### Step 3: Verify Module Organization

Review all `mod.rs` files in the git working tree and ensure they follow the patterns from the `organize-modules` skill:

**Check for:**
- ✅ Private modules with public re-exports (preferred pattern)
- ✅ Conditional visibility for docs/tests where needed: `#[cfg(any(test, doc))]`
- ✅ Rustfmt skip directive if manual formatting is needed
- ✅ Flat public API (avoid exposing internal structure)

If module organization doesn't follow patterns, invoke the `organize-modules` skill for guidance.

### Step 4: Verify Documentation Quality

If working with rustdoc comments (`///` or `//!`):

1. **Reference-style links**: Ensure backticked symbols use reference-style intra-doc links
2. **Link placement**: All reference-style links at bottom of comment block
3. **Table formatting**: Markdown tables properly aligned

If there are issues, invoke the `fix-intradoc-links` skill.

### Step 5: Run Tests if Needed

If clippy fixes modify behavior or you're unsure about changes:

```bash
cargo test --all-targets
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
- `fix-intradoc-links` - For rustdoc link formatting
- `write-documentation` - For comprehensive doc formatting

## Related Commands

- `/clippy` - Explicitly invokes this skill
- `/fix-comments` - Focuses on comment punctuation (subset of this skill)

## Related Agents

- `clippy-runner` - Agent that delegates to this skill
