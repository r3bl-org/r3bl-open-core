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
- **Technical Precision**: Use standard, precise terminology (e.g., Parameter vs. Argument) to ensure the reader's mental model matches the implementation exactly. See the [Terminology Precision] guide.

[Terminology Precision]: ../write-documentation/terminology-precision.md

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

### 5. Modern Rust Patterns: ADT Const Params

Use Enums with Const Generics (Algebraic Data Type Const Params) to control behavior without runtime overhead or boilerplate. This pattern is enabled by the [`adt_const_params`](tui/src/lib.rs) feature flag.

**When to Apply:**
- Proactively apply this pattern when writing **new code** or **refactoring existing code** that requires choosing between a closed set of behaviors or strategies at compile-time.

**Guidelines:**
- **Zero-Cost Behavior**: Prefer `const POLICY: MyEnum` over runtime fields. This allows the compiler to prune dead code and branches at compile-time (monomorphization). 
  Example: [`ScopedMutex`](tui/src/core/common/scoped_mutex/scoped_mutex_public_api.rs).
- **Reduce Boilerplate**: Prefer `const` Enums over the Trait-based Strategy pattern. This centralizes logic and eliminates the need for multiple marker structs and trait implementations.
- **Type-Level Identity**: Use this pattern when you want different behaviors to result in different types, enabling compile-time enforcement of safety rules.

## Supporting Files

- `patterns.md` - Detailed patterns with good/bad examples

## Related Skills

- `check-bounds-safety` - Type-safe Index/Length patterns (exemplar of principle #3)
- `organize-modules` - Module organization for encapsulation (supports principle #1)
- `write_documentation` - Inverted pyramid documentation (supports principle #2)
- `concurrency-safety` - Thread safety, Chain of Custody, and Loud Lock Releases

