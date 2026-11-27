---
name: remove-crate-prefix
description: Enforce the "Clean Imports over Inline Absolute Paths" rule by removing inline crate:: prefixes and adding proper use statements.
---

# Remove Crate Prefix Skill

## Purpose

This skill enforces the "Clean Imports over Inline Absolute Paths (Mandatory)" rule
defined in the project's `AGENTS.md`. It automatically scans code for inline `crate::`
prefixes (e.g. `crate::Type`, `crate::Size`) and refactors the code to use clean imports
at the top of the file or scope.

## Rules

Do NOT write absolute inline paths like `crate::Type` or `crate::Size` inside function
signatures or bodies.

### ✅ Good:

```rust
use crate::{Size, Pos};

pub fn render(size: Size) -> Pos { ... }
```

### ❌ Bad:

```rust
pub fn render(size: crate::Size) -> crate::Pos { ... }
```

## Exceptions & Macro Rules

- **Declarative Macros (`macro_rules!`)**: Keep `$crate::` for non-macro items (types, traits, functions) inside `macro_rules!` bodies; removing `$crate::` causes a syntax error. However, do NOT call sibling `#[macro_export]` helper macros via `$crate::helper_macro!()`, because that triggers `macro_expanded_macro_exports_accessed_by_absolute_paths`.
- **Crate-local `#[macro_export]` macros**: Do NOT import crate-local macros with `use crate::my_macro;` or call them via `crate::my_macro!()`. Modules propagate macros via `#[macro_use]` in declaration order, so invoke them unqualified (e.g., `key_press!`, `throws!`, `ok!`) and strip `crate::` without adding a `use` import.
- **Scoped imports**: Sometimes in generated code or highly isolated scopes, a `use crate::{...}` block directly inside a function or `if` block is preferred over a file-level import to minimize scope pollution.

## Execution

When invoked:

1. **Determine Target Files:**
    - If specific files are provided as arguments, target those files.
    - If **no arguments** are supplied, find all modified staged and unstaged files (e.g.,
      via `git diff --name-only` and `git diff --cached --name-only`) and target all of
      them.
2. Scan the targeted file(s) for instances of inline `crate::` usage for types.
3. Group the required imports into a single `use crate::{...};` statement.
    - **For production code**: Place the `use` statement at or near the top of the file,
      joining existing file-level imports (even if the `crate::` usage is inside an inner
      module).
    - **For test code**: The `use` statement can be added to the closest inner test module
      where the other test imports are located.
4. Remove the `crate::` prefix from the inline usages.
5. Verify the changes compile successfully using `./check.fish --check`.
