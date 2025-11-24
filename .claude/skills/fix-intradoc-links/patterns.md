# Intra-doc Link Patterns and Examples

This document provides comprehensive examples of intra-doc link patterns for different scenarios.

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

All links go **at the bottom** of the rustdoc comment block!
