---
name: fix-intradoc-links
description: Fix and create rustdoc intra-doc links for better IDE navigation. Convert backticked symbols to reference-style links, resolve broken links, and handle private types. Use when cargo doc shows link warnings or when improving documentation navigation.
---

# Rustdoc Intra-doc Links Best Practices

## When to Use

- Fixing rustdoc link warnings from `cargo doc --no-deps`
- Improving IDE navigation in documentation
- Converting backticked symbols to intra-doc links
- Converting inline links to reference-style links
- Handling links to private types or test functions
- Before creating commits with documentation changes
- When user says "fix doc links", "add intra-doc links", "fix link warnings", etc.

## Instructions

Follow these steps to create and fix rustdoc intra-doc links:

### Step 1: Convert Backticked Symbols to Links

If symbols are enclosed in backticks in rustdoc comments (`///` or `//!`), convert them to
reference-style intra-doc links for IDE navigation:

**Before:**
```rust
/// See `SomeType` for more details about the implementation.
```

**After:**
```rust
/// See [`SomeType`] for more details about the implementation.
///
/// [`SomeType`]: crate::path::to::SomeType
```

**Why:** This enables IDE "go to definition" from documentation tooltips.

### Step 2: Use Reference-Style Links

In rustdoc comments, always use reference-style links instead of inline links for better readability.

**✅ Good (reference-style with links at bottom):**
```rust
/// The module [`char_ops`] handles character operations.
///
/// Processing involves [`parse_input`] followed by [`transform`].
///
/// [`char_ops`]: crate::core::pty_mux::vt_100_ansi_parser::operations::char_ops
/// [`parse_input`]: crate::core::parse_input
/// [`transform`]: crate::core::transform
```

**❌ Bad (inline links clutter the prose):**
```rust
/// The module [`char_ops`](crate::core::pty_mux::vt_100_ansi_parser::operations::char_ops)
/// handles character operations.
///
/// Processing involves [`parse_input`](crate::core::parse_input) followed by
/// [`transform`](crate::core::transform).
```

**❌ Bad (links not at bottom):**
```rust
/// The module [`char_ops`] handles character operations.
/// [`char_ops`]: crate::core::pty_mux::vt_100_ansi_parser::operations::char_ops
///
/// Processing involves [`parse_input`] followed by [`transform`].
/// [`parse_input`]: crate::core::parse_input
/// [`transform`]: crate::core::transform
```

**Guidelines:**
- All reference-style link definitions go at the **bottom** of the comment block
- Verify links are correct with `cargo doc --no-deps`
- The `cargo rustdoc-fmt` tool can automate this conversion (see `write-documentation` skill)

### Step 3: Fix Broken Link Warnings

When `cargo doc --no-deps` shows missing link warnings, **DO NOT remove the links**.
Instead, add proper references.

**Error Example:**
```rust
/// See [`SomeType`] for more details.
// ⚠️ cargo doc warning: `SomeType` not found
```

**❌ Bad Fix (removing the link):**
```rust
/// See `SomeType` for more details.
```

**✅ Good Fix (adding the reference):**
```rust
/// See [`SomeType`] for more details.
///
/// [`SomeType`]: crate::path::to::SomeType
```

### Step 4: Handle Private Types

For private types that need to be linked in documentation, you must make them conditionally
visible using patterns from the `organize-modules` skill.

#### Make Module Conditionally Public

```rust
// In mod.rs - Make module public for docs and tests
#[cfg(any(test, doc))]
pub mod internal_module;
#[cfg(not(any(test, doc)))]
mod internal_module;

pub use internal_module::*;
```

**See the `organize-modules` skill for complete details on conditional visibility patterns.**

#### Link Using Explicit Prefixes

Then link using explicit prefixes to be clear about the symbol type:

- `mod@crate::path::to::modname` - For module links
- `struct@crate::path::to::StructName` - For struct links
- `enum@crate::path::to::EnumName` - For enum links
- `fn@crate::path::to::function_name` - For function links
- `trait@crate::path::to::TraitName` - For trait links
- `type@crate::path::to::TypeAlias` - For type alias links

**Example:**
```rust
/// See [`internal_module`] for implementation.
///
/// [`internal_module`]: mod@crate::tui::terminal_lib_backends::offscreen_buffer::internal_module
```

#### Alternative: Simple Links Through Public API

If types are re-exported through a clean public API, rustdoc can resolve simple links:

```rust
/// See [`Type`] for details.
///
/// [`Type`]: Type  // rustdoc resolves through public re-export
```

This is cleaner when dealing with well-organized module exports, but only works if the symbol
is publicly re-exported.

### Step 5: Method References

Always include parentheses `()` in method references in both the link text and the path:

**✅ Good:**
```rust
/// This calls [`reset_style()`] to clear attributes.
///
/// [`reset_style()`]: crate::OffscreenBuffer::reset_style
```

**❌ Bad (missing parentheses):**
```rust
/// This calls [`reset_style`] to clear attributes.
///
/// [`reset_style`]: crate::OffscreenBuffer::reset_style
```

**Why:** Parentheses make it visually clear this is a method, not a field or type.

### Step 6: Module References

Use the `mod@` prefix for module links to be explicit:

**✅ Good:**
```rust
/// See [`diff_chunks`] for implementation details.
///
/// [`diff_chunks`]: mod@crate::tui::terminal_lib_backends::offscreen_buffer::diff_chunks
```

**Why:** The `mod@` prefix prevents ambiguity if there's a type with the same name.

### Step 7: Linking to Test Functions

Test functions need special handling to be linkable in documentation.

#### Make Test Function Visible to Docs

Use `#[cfg_attr(not(doc), ...)]` to conditionally apply test attributes:

```rust
/// Documentation for this test function.
///
/// Run with: `cargo test my_test -- --ignored --nocapture`
#[cfg_attr(not(doc), tokio::test)]
#[cfg_attr(not(doc), ignore = "Manual test")]
pub async fn my_test() -> Result<()> {
    // test implementation
}
```

**How this works:**
- **In doc builds**: Test attributes are skipped → function is a regular `pub async fn` → rustdoc can see it
- **In test builds**: Test attributes are applied → test runner recognizes it as a test

#### Link to the Test Function

```rust
/// See [`my_test`] for ground truth validation.
///
/// [`my_test`]: crate::path::to::module::my_test
```

#### Requirements

For this to work:
- Function must be `pub`
- Module containing the function must be conditionally public: `#[cfg(any(test, doc))]`
- Use `#[cfg_attr(not(doc), ...)]` for test attributes

### Step 8: Verify All Links

After adding/fixing links, verify with cargo doc:

```bash
cargo doc --no-deps
```

**Ensure zero warnings** about unresolved links or broken paths.

## Common Link Patterns

### Struct Field

```rust
/// The [`cursor`] field tracks position.
///
/// [`cursor`]: crate::OffscreenBuffer::cursor
```

### Associated Type

```rust
/// Returns [`Self::Item`] on each iteration.
///
/// [`Self::Item`]: Iterator::Item
```

### Trait Method

```rust
/// Implements [`Display::fmt()`] for rendering.
///
/// [`Display::fmt()`]: std::fmt::Display::fmt
```

### Re-exported Symbol

```rust
/// Uses [`ColIndex`] for column positions.
///
/// [`ColIndex`]: crate::ColIndex  // Re-exported at crate root
```

## Transitive Visibility

**Important:** If a conditionally public module links to another module in its documentation,
that target module must also be conditionally public.

**Example:**

```rust
// mod.rs

#[cfg(any(test, doc))]
pub mod paint_impl;  // Links to diff_chunks in docs
#[cfg(not(any(test, doc)))]
mod paint_impl;

#[cfg(any(test, doc))]
pub mod diff_chunks;  // Must also be conditionally public!
#[cfg(not(any(test, doc)))]
mod diff_chunks;
```

See the `organize-modules` skill for more on conditional visibility.

## Reporting Results

After fixing intra-doc links:

- ✅ All links resolve → "Intra-doc links fixed and verified!"
- ⚠️ Some links still broken → Summarize which symbols couldn't be resolved
- 🔧 Made modules conditionally public → Report what visibility changes were needed
- 📝 Manual fixes needed → List what requires developer attention (e.g., ambiguous paths)

## Supporting Files in This Skill

This skill includes additional reference material:

- **`patterns.md`** - 15 detailed pattern examples covering every intra-doc link scenario: basic symbols, methods vs functions, module links, private types, test functions, struct fields, trait items, re-exported symbols, external crates, link placement, disambiguating with prefixes, `Self` vs absolute paths, generic types, section heading fragments, and more. Includes a complete checklist and quick reference table. **Read this when:**
  - Need specific examples for a linking scenario (see Pattern 1-15)
  - How to link to test functions → Pattern 5
  - Linking to private types with conditional visibility → Pattern 4
  - Module references with `mod@` prefix → Pattern 3
  - Re-exported symbols → Pattern 8
  - Disambiguating with prefixes (`mod@`, `struct@`, etc.) → Pattern 11
  - Linking to section headings with `#fragment` → Pattern 15
  - Complete checklist for perfect links → End of file

## Related Skills

- `write-documentation` - For overall doc writing and formatting (uses cargo rustdoc-fmt)
- `organize-modules` - For conditional visibility patterns (`#[cfg(any(test, doc))]`)
- `check-code-quality` - Includes doc verification step
- `run-clippy` - May suggest documentation improvements

## Related Commands

- `/fix-intradoc-links` - Explicitly invokes this skill
- `/docs` - Includes link fixing as part of documentation workflow

## Related Agents

- `code-formatter` - May invoke this skill when fixing doc warnings
- `clippy-runner` - May suggest adding intra-doc links
