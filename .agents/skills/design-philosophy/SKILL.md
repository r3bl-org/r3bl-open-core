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
- **Constant Derivation Hierarchy (No Duplicate Literals)**: Never duplicate raw literals across byte, string, and length constants. Derive related constants from a single base definition:
  - **Atomic Primitives (Single byte/char)**: Define the `u8` constant first (`u8` to `char` via `ascii_byte_to_char` to `&str` via `ascii_byte_to_str`).
  - **Composed Sequences (Multi-byte strings)**: Define the `&str` constant first (`&str` via compile-time concatenation/macro to `&[u8]` via `.as_bytes()` to `usize` via `.len()`).
  See [Constant Design Patterns in `patterns.md`](patterns.md#constant-design-patterns).
- **Technical Precision**: Use standard, precise terminology (e.g., Parameter vs. Argument) to ensure the reader's mental model matches the implementation exactly. See the [Terminology Precision] guide.

[Terminology Precision]: ../write-documentation/terminology-precision.md

### 2. Progressive Disclosure

Reveal complexity only when needed.

**Guidelines:**

- Public APIs should be minimal and intuitive
- Advanced features should be discoverable but not in-your-face
- Documentation follows inverted pyramid: high-level first, details later
- Module structure should guide users from simple to advanced
- **Type-Centric Co-location (Associated Functions over Floating Functions)**: Prefer associated functions and methods on domain types over free-floating helper functions.
  - **Co-locate with Return Type**: When a function's primary job is constructing, parsing, or classifying a domain type `T` (e.g. `classify_csi_buffer -> CsiBufferKind`), make it an associated function on `T` (`CsiBufferKind::classify`).
  - **Co-locate with Subject Type**: When a function interprets, translates, or queries `T` (e.g. `parse_ss3_command(byte) -> Option<VT100KeyCodeIR>`), make it an associated method or function on `T` (`Ss3BufferKind::decode(self)` / `Ss3BufferKind::parse_command(byte)`).
  - **Keep Floating When Appropriate**: Reserve floating functions for high-level pipeline entry points / coordinators (e.g. `parse_keyboard_sequence` returning generic IR), multi-type algorithms, or functions returning external/standard library types (`u32`, `String`).

### 3. Make Illegal States Unrepresentable

Use the type system to prevent bugs at compile time.

**Guidelines:**

- Prefer newtypes over primitives (e.g., `Index` instead of `usize`)
- **Eliminate Boolean Soup / Boolean Blindness**: Never use raw `bool` flags, struct fields, function parameters, or return values in domain code (e.g. `fn process(flag: bool)` or `fn is_default() -> bool`). Replace boolean soup with self-documenting domain enums (e.g. `SyntaxHighlightPipeline::R3BLMarkdown` instead of `is_file_extension_default() -> bool`, `TrailingNewline::Present` / `Absent`, or `HasSelection::Yes` / `No`). Domain enums make call sites self-documenting and leverage Rust's `match` or `==` for explicit checking.
  - **No `.is_*() -> bool` Helpers on Domain Enums**: Do **NOT** add `pub fn is_modified(self) -> bool` or similar boolean conversion helpers onto domain enums. Callers must match on the enum or compare directly against the variant (e.g. `status == FormatStatus::Modified`).
  - **When `bool` Is Acceptable (`std` & Third-Party Compatibility Only)**: Raw `bool` is permitted **solely** when required for compatibility with `std`, `clippy`, or third-party crate contracts:
    1. `std` collection conventions: `pub fn is_empty(&self) -> bool` when paired with `pub fn len(&self)` (enforced by `clippy::len_without_is_empty`).
    2. `std` trait signatures and higher-order closures: implementing `PartialEq::eq` or passing predicate closures to `std` iterator/collection methods (`.filter(...)`, `.any(...)`, `.all(...)`, `.retain(...)`).
    3. Third-party derive macro contracts: struct fields required by derive macros such as `#[derive(clap::Parser)]` CLI switch flags (`#[arg(long)] pub verbose: bool`).
- **Avoid Boolean Blindness in Mutator Returns (`Option<T>` / `Option<()>` vs Domain Enums)**: Mutator methods taking `&mut self` that delete or replace text cannot return borrowed slices (`Option<&str>`) or metadata offsets (`DocSeg`) because the borrow checker forbids returning a `&str` borrowing from `self` across a `&mut self` mutation, and deleted bytes no longer exist in memory. Returning a text payload from a `&mut self` mutator unavoidably requires an owned heap allocation (`Option<String>`). Return a payload `Option<T>` (e.g. `Option<LineMetadata>` for zero-allocation metrics or `Option<String>` for text) ONLY when callers genuinely consume it. If no text payload is consumed by callers:
  - **Use `Option<()>` ONLY when callers chain with `?`**: When a mutator can fail (e.g. out-of-bounds index) and callers short-circuit on `None` using `?`, return `Option<()>` (`Some(())` for success, `None` for failure).
  - **Use a Domain Enum when callers do NOT chain with `?`**: Using `Option<()>` when callers do not short-circuit with `?` is an **anti-pattern** because it forces `.is_some()` checks (boolean blindness in disguise) and misrepresents a valid no-op state as failure (`None`). For sequential passes or status-reporting mutators, return an explicit domain enum (e.g. `FormatStatus::Modified` / `FormatStatus::Unchanged`).
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

### 7. Strict Encapsulation & Testing Hygiene

Maintain strict encapsulation boundaries in production code and do not leak internal state merely for the convenience of tests.

**Guidelines:**

- **No `pub` for tests**: Never make a field or method `pub` or `pub(crate)` if its only external caller is a test.
- **Test-Only Accessors**: Keep production fields strictly scoped (private or `pub(in crate::...)`). If tests need to inspect or manipulate internal state, provide explicit accessor methods annotated with `#[cfg(test)]` (e.g., `pub(crate) fn internal_state_for_testing(&self)`). This cleanly compiles the escape hatch out of the production binary.

## Supporting Files

- `patterns.md` - Detailed patterns with good/bad examples

## Related Skills

- `check-bounds-safety` - Type-safe Index/Length patterns (exemplar of principle #3)
- `organize-modules` - Module organization for encapsulation (supports principle #1)
- `write_documentation` - Inverted pyramid documentation (supports principle #2)
- `concurrency-safety` - Thread safety, Chain of Custody, and Loud Lock Releases

