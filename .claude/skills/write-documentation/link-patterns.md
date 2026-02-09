# Intra-doc Link Patterns and Examples

This document provides comprehensive examples of intra-doc link patterns for different scenarios.

---

## Link Source Rubric

When deciding between local intra-doc links vs external URLs, follow this priority order:

| Priority | Source Type | Link Style | Example |
|----------|-------------|------------|---------|
| 1 | Code in this monorepo | Local path | `[`Foo`]: crate::module::Foo` |
| 2 | Dependency in Cargo.toml | Crate path | `[`mio`]: mio` |
| 3 | OS/CS/hardware terminology | External URL | `[`epoll`]: https://man7.org/...` |
| 4 | Pedagogical/domain terms | Wikipedia URL | `[design pattern]: https://en.wikipedia.org/...` |
| 5 | Non-dependency crates | docs.rs URL | `[`some_crate`]: https://docs.rs/some_crate` |

**Key principle:** If it's in your Cargo.toml, use local links. This enables:
- **Offline docs** — works without internet
- **Version-matched** — links to the exact version you depend on
- **Rustdoc-validated** — broken links caught at build time

### Examples

```rust
// ✅ Good: mio is a dependency, use crate path
//! [`mio`]: mio

// ❌ Bad: Using docs.rs for a dependency
//! [`mio`]: https://docs.rs/mio

// ✅ Good: OS concept (not a Rust crate), use external URL
//! [`epoll`]: https://man7.org/linux/man-pages/man7/epoll.7.html

// ✅ Good: Wikipedia for CS terminology
//! [Actor]: https://en.wikipedia.org/wiki/Actor_model
```

> **Note:** This rubric is also in `SKILL.md`. The redundancy is intentional—prioritizing reliable
> application over efficiency. SKILL.md content loads when the skill triggers (ensuring correct
> behavior during doc generation), while this file serves as detailed reference for auditing.

---

## Pattern 1: Basic Symbol Links

### ✅ Good: Reference-Style Links

```rust
/// This struct uses [`Position`] to track cursor location and [`Style`] for rendering.
///
/// The [`render()`] method updates the display.
///
/// [`Position`]: crate::Position
/// [`Style`]: crate::Style
/// [`render()`]: Self::render
```

### ❌ Bad: Inline Links

```rust
/// This struct uses [`Position`](crate::Position) to track cursor location
/// and [`Style`](crate::Style) for rendering.
///
/// The [`render()`](Self::render) method updates the display.
```

**Why bad:** Inline links clutter the prose and make it hard to read.

### ❌ Bad: No Links at All

```rust
/// This struct uses `Position` to track cursor location and `Style` for rendering.
///
/// The `render()` method updates the display.
```

**Why bad:** No IDE navigation, harder to explore the codebase.

---

## Pattern 2: Method vs Function Links

### Methods (Include Parentheses)

```rust
/// Call [`process()`] to handle input.
///
/// [`process()`]: Self::process
```

### Functions (Include Parentheses)

```rust
/// Use [`parse_input()`] for preprocessing.
///
/// [`parse_input()`]: crate::utils::parse_input
```

### Associated Functions (Include Parentheses)

```rust
/// Create with [`new()`] constructor.
///
/// [`new()`]: Self::new
```

**Rule:** Always include `()` for methods and functions to distinguish them from fields and types.

---

## Pattern 3: Module Links

### ✅ Good: Explicit mod@ Prefix

```rust
/// See [`parser`] module for details.
///
/// [`parser`]: mod@crate::core::parser
```

### ⚠️ Works but Ambiguous

```rust
/// See [`parser`] for details.
///
/// [`parser`]: crate::core::parser
```

**Why risky:** If there's a `struct Parser` or `fn parser()`, rustdoc might resolve incorrectly.

### ✅ Good: Full Path with mod@

```rust
/// Implementation details in [`vt_100_ansi_parser`].
///
/// [`vt_100_ansi_parser`]: mod@crate::tui::core::pty_mux::vt_100_ansi_parser
```

---

## Pattern 4: Links to Private Types (Conditional Visibility)

### Step 1: Make Module Conditionally Public

```rust
// mod.rs

// Make internal_module public only for docs and tests
#[cfg(any(test, doc))]
pub mod internal_module;
#[cfg(not(any(test, doc)))]
mod internal_module;

// Re-export items for public API
pub use internal_module::*;
```

### Step 2: Link with Explicit Prefix

```rust
/// See [`internal_module`] for implementation.
///
/// [`internal_module`]: mod@crate::internal_module
```

### Complete Example

```rust
// lib.rs or mod.rs
#[cfg(any(test, doc))]
pub mod vt_100_ansi_parser;
#[cfg(not(any(test, doc)))]
mod vt_100_ansi_parser;

pub use vt_100_ansi_parser::*;

/// Terminal state manager.
///
/// Uses [`vt_100_ansi_parser`] for escape sequence processing.
///
/// [`vt_100_ansi_parser`]: mod@crate::vt_100_ansi_parser
pub struct TerminalState {
    // ...
}
```

### Simpler Alternative: Just Make It `pub`

The conditional visibility pattern above is for **modules**. For **functions, structs, and other
items** inside a private module, there's a simpler approach: just make the item `pub`.

This is safe because of our **private modules with public re-exports** pattern. An item being
`pub` inside its containing module doesn't mean it escapes into the wild — `mod.rs` is the
gatekeeper. The `pub use` re-exports in `mod.rs` explicitly control which symbols are visible
to other modules, docs, and tests.

```rust
// rrt.rs — internal function, safe to be pub
/// Runs the worker's poll loop until it returns [`Continuation::Stop`].
pub fn run_worker_loop<W, E>(...) { ... }

// mod.rs — re-exports control actual visibility
pub use rrt::*;  // run_worker_loop is now linkable in docs
```

Now intra-doc links resolve:

```rust
//! Resources are cleaned up via [RAII] when [`run_worker_loop()`] returns.
//!
//! [`run_worker_loop()`]: run_worker_loop
```

**When to use which approach:**

| Situation | Approach |
| :-------- | :------- |
| Private **module** needs doc links | Conditional visibility (`#[cfg(any(test, doc))]`) |
| Private **item** inside a module | Just make it `pub` — `mod.rs` re-exports are the real gate |

---

## Pattern 5: Links to Test Functions

### Step 1: Make Test Function Visible to Docs

```rust
/// Ground truth validation for terminal parsing.
///
/// Run with: `cargo test test_terminal_parsing -- --ignored --nocapture`
#[cfg_attr(not(doc), tokio::test)]
#[cfg_attr(not(doc), ignore = "Manual test")]
pub async fn test_terminal_parsing() -> Result<()> {
    // test implementation
}
```

### Step 2: Module Must Be Conditionally Public

```rust
// mod.rs
#[cfg(any(test, doc))]
pub mod tests;
#[cfg(not(any(test, doc)))]
mod tests;
```

### Step 3: Link to Test Function

```rust
/// This function processes VT-100 sequences.
///
/// See [`test_terminal_parsing`] for comprehensive examples.
///
/// [`test_terminal_parsing`]: crate::tests::test_terminal_parsing
pub fn process_vt100(input: &str) -> Result<Action> {
    // ...
}
```

---

## Pattern 6: Struct Fields

### ✅ Good: Link to Field

```rust
/// Terminal buffer with cursor tracking.
///
/// The [`cursor`] field stores current position.
///
/// [`cursor`]: Self::cursor
pub struct Buffer {
    /// Current cursor position
    pub cursor: Position,
}
```

### ✅ Good: Link to Struct's Field from External Doc

```rust
/// Rendering uses [`Buffer::cursor`] for positioning.
///
/// [`Buffer::cursor`]: crate::Buffer::cursor
```

---

## Pattern 7: Trait Items

### Associated Types

```rust
/// Iterator over terminal lines.
///
/// [`Item`] is a single line of text.
///
/// [`Item`]: Self::Item
impl Iterator for LineIterator {
    type Item = String;
    // ...
}
```

### Trait Methods

```rust
/// Implements [`Display::fmt()`] for rendering.
///
/// [`Display::fmt()`]: std::fmt::Display::fmt
impl Display for TerminalState {
    // ...
}
```

---

## Pattern 8: Re-exported Symbols

When symbols are re-exported at the crate root:

### In the Original Module

```rust
// in core/units.rs
pub struct ColIndex(pub usize);
```

### In Crate Root (lib.rs)

```rust
pub use crate::core::units::ColIndex;
```

### Linking to Re-exported Symbol

```rust
/// Uses [`ColIndex`] for column tracking.
///
/// [`ColIndex`]: crate::ColIndex  // Link to re-export, not original location
```

**Why:** Users import from the re-export, so links should point there too.

---

## Pattern 9: External Crate Links

### Standard Library

```rust
/// Returns a [`Vec`] of results.
///
/// [`Vec`]: std::vec::Vec
```

Or even simpler (rustdoc auto-resolves):

```rust
/// Returns a [`Vec`] of results.
///
/// [`Vec`]: Vec
```

### External Crates

```rust
/// Uses [`serde::Serialize`] for serialization.
///
/// [`serde::Serialize`]: serde::Serialize
```

---

## Pattern 10: Link Placement (Always at Bottom)

### ✅ Good: All Links at Bottom

```rust
/// This module handles [`Input`] processing.
///
/// ## Overview
///
/// The [`Parser`] reads from [`Stream`] and produces [`Event`] instances.
///
/// ## Example
///
/// ```
/// let parser = Parser::new();
/// ```
///
/// ## See Also
///
/// Related: [`Output`] module
///
/// [`Input`]: crate::Input
/// [`Parser`]: crate::Parser
/// [`Stream`]: crate::Stream
/// [`Event`]: crate::Event
/// [`Output`]: mod@crate::Output
```

### ❌ Bad: Links Scattered Throughout

```rust
/// This module handles [`Input`] processing.
/// [`Input`]: crate::Input
///
/// ## Overview
///
/// The [`Parser`] reads from [`Stream`] and produces [`Event`] instances.
/// [`Parser`]: crate::Parser
/// [`Stream`]: crate::Stream
/// [`Event`]: crate::Event
```

---

## Pattern 11: Disambiguating with Prefixes

When names could be ambiguous:

### Without Prefix (Ambiguous)

```rust
/// Uses [`Parser`] for processing.
///
/// [`Parser`]: crate::parser  // Is this mod@parser or struct@Parser?
```

### With Prefix (Clear)

```rust
/// Uses [`Parser`] struct and [`parser`] module.
///
/// [`Parser`]: struct@crate::Parser
/// [`parser`]: mod@crate::parser
```

**Available prefixes:**
- `mod@` - Module
- `struct@` - Struct
- `enum@` - Enum
- `trait@` - Trait
- `fn@` - Function
- `type@` - Type alias
- `const@` - Constant
- `static@` - Static

---

## Pattern 12: Self vs Absolute Paths

### Using Self (For Items in Same Type)

```rust
impl TerminalState {
    /// Creates a new instance. See also [`reset()`].
    ///
    /// [`reset()`]: Self::reset
    pub fn new() -> Self { /* ... */ }

    /// Resets to initial state.
    pub fn reset(&mut self) { /* ... */ }
}
```

### Using Absolute Paths (For Items Elsewhere)

```rust
impl TerminalState {
    /// Processes input using [`parse_input()`].
    ///
    /// [`parse_input()`]: crate::utils::parse_input
    pub fn process(&mut self, input: &str) { /* ... */ }
}
```

---

## Pattern 13: Linking Between Modules

### From Module A to Module B

```rust
// in module_a.rs
/// Sends data to [`module_b`] for processing.
///
/// [`module_b`]: mod@crate::module_b
```

### Bidirectional Links

```rust
// in module_a.rs
/// Works with [`module_b`] for round-trip processing.
///
/// [`module_b`]: mod@crate::module_b

// in module_b.rs
/// Receives data from [`module_a`].
///
/// [`module_a`]: mod@crate::module_a
```

---

## Pattern 14: Generic Type Parameters

### Linking to Generic Types

```rust
/// A container for [`T`] values.
///
/// [`T`]: Self::T
pub struct Container<T> {
    items: Vec<T>,
}
```

### Linking to Trait Bounds

```rust
/// Processes items that implement [`Display`].
///
/// [`Display`]: std::fmt::Display
pub fn process<T: Display>(item: T) { /* ... */ }
```

---

## Pattern 15: Linking to Section Headings (Fragments)

You can link directly to `## Section Heading` in module documentation using fragment identifiers.

### Fragment Format

Section headings are converted to fragments:
- `## My Section Title` → `#my-section-title` (lowercase, spaces become hyphens)
- `## UTF-8 Encoding Explained` → `#utf-8-encoding-explained`

### ✅ Good: Full Crate Path with Fragment

```rust
/// See the [`utf8` encoding] section for bit pattern details.
///
/// [`utf8` encoding]: mod@crate::core::ansi::vt_100_terminal_input_parser::utf8#utf-8-encoding-explained
```

### ✅ Good: Simple super with Fragment (No Path Components)

```rust
/// See [parent module documentation] for the overview.
///
/// [parent module documentation]: mod@super#primary-consumer
```

### ❌ Bad: super:: Path with Fragment (Causes Recursion)

```rust
/// See the [`utf8` encoding] section for details.
///
/// [`utf8` encoding]: mod@super::utf8#utf-8-encoding-explained  // ⚠️ rustdoc recursion!
```

**Why bad:** Combining `mod@` + `super::path` (with `::`) + `#fragment` can cause rustdoc to enter an infinite loop during documentation generation.

### ❌ Bad: super:: Path without mod@ (Also Problematic)

```rust
/// See the [`utf8` encoding] section for details.
///
/// [`utf8` encoding]: super::utf8#utf-8-encoding-explained  // ⚠️ May also fail
```

### Rule Summary

| Pattern | Works? | Example |
|---------|--------|---------|
| `mod@super#fragment` | ✅ | `mod@super#testing-strategy` |
| `mod@crate::full::path#fragment` | ✅ | `mod@crate::core::parser#overview` |
| `mod@super::sibling#fragment` | ❌ | Causes rustdoc recursion |
| `super::sibling#fragment` | ❌ | May cause issues |

**Best Practice:** When linking to a section in another module, always use the full `crate::` path.

---

## Checklist for Perfect Intra-doc Links

When fixing links, ensure:

- [ ] All symbols in backticks are links: `` `Symbol` `` → `` [`Symbol`] ``
- [ ] All links use reference-style, not inline
- [ ] All link definitions are at the bottom of the comment block
- [ ] Methods and functions include parentheses: `fn()`
- [ ] Module links use `mod@` prefix when ambiguous
- [ ] Private types are in conditionally public modules
- [ ] Test functions use `#[cfg_attr(not(doc), test)]` pattern
- [ ] Absolute paths start with `crate::` or `std::`
- [ ] `cargo doc --no-deps` shows zero warnings
- [ ] Links enable IDE "go to definition"
- [ ] No trailing whitespace on blank doc lines (`/// ` → `///`)

---

## Troubleshooting: Mysterious Link Failures

When reference links fail despite correct syntax, check these hidden causes:

### Trailing Whitespace in Blank Doc Lines

**Symptom:** All reference links in a doc block fail with "unresolved link" warnings.

```rust
// ❌ BROKEN - trailing space on blank line breaks entire reference block
/// Some documentation.
///                        ← invisible trailing space here!
/// [`Foo`]: crate::Foo    ← this link fails
/// [`Bar`]: crate::Bar    ← this link also fails
```

```rust
// ✅ FIXED - no trailing whitespace
/// Some documentation.
///                        ← clean blank line (no trailing space)
/// [`Foo`]: crate::Foo    ← link works
/// [`Bar`]: crate::Bar    ← link works
```

**Why:** Rustdoc's reference link parser is whitespace-sensitive. A `/// ` line (with trailing
space) breaks the parsing of the entire reference definition block that follows.

**How to detect:** Run `grep -n '/// $' src/**/*.rs` to find lines ending with `/// ` (space).

**Editor tip:** Configure your editor to strip trailing whitespace on save, but be aware that
some doc comments intentionally use trailing whitespace for formatting (rare).

### Field/Method Name Collisions

When a field and method share the same name, rustdoc handles disambiguation automatically:

```rust
pub struct ThreadLiveness {
    /// The running state field.
    pub is_running: AtomicBool,  // field
}

impl ThreadLiveness {
    /// Check if running.
    pub fn is_running(&self) -> bool { ... }  // method
}
```

**Linking correctly:**
```rust
/// - [`is_running`]: The field
/// - [`is_running()`]: The method
///
/// [`is_running`]: Self::is_running      ← resolves to field
/// [`is_running()`]: Self::is_running()  ← resolves to method (note the () in target too)
```

This is **not** a cause of mysterious failures—rustdoc's disambiguation works well.

---

## Quick Reference

| Link To              | Pattern Example                                  |
|----------------------|--------------------------------------------------|
| Struct               | `` [`Foo`]: crate::Foo ``                        |
| Enum                 | `` [`Bar`]: crate::Bar ``                        |
| Trait                | `` [`Display`]: std::fmt::Display ``             |
| Function             | `` [`process()`]: crate::process ``              |
| Method               | `` [`run()`]: Self::run ``                       |
| Field                | `` [`field`]: Self::field ``                     |
| Module               | `` [`parser`]: mod@crate::parser ``              |
| Re-export            | `` [`Type`]: crate::Type ``                      |
| Test function        | `` [`test_foo`]: crate::tests::test_foo ``       |
| Private type (pub for doc) | `` [`Internal`]: struct@crate::internal::Internal `` |
| Section heading (fragment) | `` [`docs`]: mod@crate::path::module#section-name `` |

All links go **at the bottom** of the rustdoc comment block!
