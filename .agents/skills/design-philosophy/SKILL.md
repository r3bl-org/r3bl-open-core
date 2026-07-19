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
- **Clean Imports**: Use `use` statements at the top of files rather than inline absolute paths (e.g. `crate::Type`) to reduce visual noise and cognitive clutter in function bodies.
- **No Magic Numbers/Strings**: Extract domain-specific numbers and strings (like ANSI mode integers or escape sequences) into named constants (e.g., in `tui/src/core/ansi/constants/`). Do not use magic numbers directly in business logic or pattern matches, as this requires readers to memorize their meaning.
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
- **Eliminate Boolean Soup / Boolean Blindness**: Avoid raw `bool` flags, function parameters, or return values that obscure intent (e.g. `fn process(flag: bool)` or `fn is_default() -> bool`). Replace boolean soup with self-documenting domain enums (e.g. `SyntaxHighlightPipeline::R3BLMarkdown` instead of `is_file_extension_default() -> bool`, or `HasSelection::Yes` / `No`). Domain enums make call sites self-documenting and leverage Rust's `match` for exhaustive compiler checking.
- **Avoid Boolean Blindness in Mutator Returns (`Option<T>` / `Option<()>`)**: Mutator methods taking `&mut self` that delete or replace text cannot return borrowed slices (`Option<&str>`) or metadata offsets (`DocSeg`) because the borrow checker forbids returning a `&str` borrowing from `self` across a `&mut self` mutation, and deleted bytes no longer exist in memory. Returning a text payload from a `&mut self` mutator unavoidably requires an owned heap allocation (`Option<String>`). Return a payload `Option<T>` (e.g. `Option<LineMetadata>` for zero-allocation metrics or `Option<String>` for text) ONLY when callers genuinely consume it. If no text payload is consumed by callers, do not incur speculative allocations. Return `Option<()>` (`Some(())` for success, `None` for failure). `Option<()>` achieves the exact same zero-allocation CPU-register performance as `bool` while eliminating boolean blindness and supporting `?` operator chaining.
- **Constructor Conventions (`Default` over No-Arg `new()`)**: If a type requires no arguments for construction, derive or implement `Default` (`#[derive(Default)]`) and do **NOT** create a redundant `pub fn new() -> Self` method. Use parameterized constructors (`with_capacity(...)` or `new(arg: Type)`) only when arguments are required.
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

### 5. High-Fidelity Error Handling

Treat errors as a user interface (UI) for developers. Public-facing errors must provide actionable information.

**Guidelines:**

- **Standardize on `miette`**: All custom error types (enums/structs) must derive `miette::Diagnostic` in addition to `thiserror::Error`.
- **Actionable Metadata**: Use `#[diagnostic(help(...))]` to provide hints on how to resolve the error.
- **Searchable Codes**: Use `#[diagnostic(code(...))]` to provide unique identifiers for errors, facilitating documentation and search.
- **Preserve Context**: Avoid "lossy" error conversions (like turning a rich `miette::Report` into a plain string). Use transparent delegation or dedicated variants to preserve the full error chain.

### 6. Modern Rust Patterns: ADT Const Params

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

