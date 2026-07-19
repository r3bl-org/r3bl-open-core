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

## Exceptions

- **Declarative Macros (`macro_rules!`)**: Do NOT apply this refactor inside declarative macros, and NEVER touch or remove the `$crate::` prefix. Modifying `$crate::` will result in a syntax error because it turns a built-in macro keyword into an undeclared macro variable.
- Macro invocations often require absolute paths (e.g., `crate::key_press!`). This rule
  primarily targets structs, traits, and enums (e.g., `crate::ModifierKeysMask`).
- Sometimes in generated code or highly isolated scopes, a `use crate::{...}` block
  directly inside a function or `if` block is preferred over a file-level import to
  minimize scope pollution.

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
