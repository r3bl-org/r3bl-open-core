---
name: design-philosophy
description: Core design principles for the codebase - cognitive load, progressive disclosure, type safety, abstraction worth. Use when designing APIs, modules, or data structures.
---

# Design Philosophy Skill

Apply these principles when writing or reviewing code.

## When to Use

- Proactively when designing new APIs, modules, or data structures
- When refactoring existing code
- When reviewing code for maintainability

## Core Principles

### 1. Minimize Cognitive Load

Code should be easy to understand without loading too much into working memory.

**Guidelines:**

- Good Separation of Concerns (SoC) means fewer "things" to keep in mind
- Each module/function should have a single, clear responsibility
- Limit the number of concepts a reader must hold simultaneously

### 2. Progressive Disclosure

Reveal complexity only when needed.

**Guidelines:**

- Public APIs should be minimal and intuitive
- Advanced features should be discoverable but not in-your-face
- Documentation follows inverted pyramid: high-level first, details later
- Module structure should guide users from simple to advanced

### 3. Make Illegal States Unrepresentable

Use the type system to prevent bugs at compile time.

**Guidelines:**

- Prefer newtypes over primitives (e.g., `Index` instead of `usize`)
- Design enums and structs so invalid combinations cannot be constructed
- Move validation from runtime to compile time where possible
- See `check-bounds-safety` skill for exemplary patterns

### 4. Abstractions Must Earn Their Keep

An abstraction should reduce cognitive load, not add to it.

**Guidelines:**

- If understanding the abstraction requires more effort than the concrete code, don't abstract
- Good abstractions match mental models developers already have
- Three similar lines of code is often better than a premature abstraction
- Abstractions should hide complexity, not just move it

## Supporting Files

- `patterns.md` - Detailed patterns with good/bad examples

## Related Skills

- `check-bounds-safety` - Type-safe Index/Length patterns (exemplar of principle #3)
- `organize-modules` - Module organization for encapsulation (supports principle #1)
- `write-documentation` - Inverted pyramid documentation (supports principle #2)
