---
name: organize-modules
description: Apply private modules with public re-exports pattern for clean API design. Includes conditional visibility for docs and tests. Use when creating modules, organizing mod.rs files, or before creating commits.
---

# Module Organization Best Practices

## When to Use

- Creating new Rust modules
- Refactoring module structure
- Organizing mod.rs files
- Reviewing code that exposes internal structure
- Making private types visible to documentation
- Before creating commits with module changes
- When user says "organize modules", "refactor modules", "fix module structure", etc.

## Instructions

Follow these patterns for clean, maintainable module organization:

### Step 1: Apply the Recommended Pattern

**Prefer private modules with public re-exports** as the default pattern.

This provides a clean API while maintaining flexibility to refactor internal structure.

```rust
// mod.rs - Module coordinator

// Private modules (hide internal structure)
mod constants;
mod types;
mod helpers;

// Public re-exports (expose stable API)
pub use constants::*;
pub use types::*;
pub use helpers::*;
```

**What this achieves:**
- Clean, flat API for users
- Internal structure is hidden and can be refactored freely
- No namespace pollution from module names

### Step 2: Control Rustfmt Behavior (When Needed)

For `mod.rs` files with deliberate manual alignment, prevent rustfmt from reformatting:

```rust
// mod.rs

// Skip rustfmt for rest of file.
// https://stackoverflow.com/a/75910283/2085356
#![cfg_attr(rustfmt, rustfmt_skip)]

// Private modules
mod constants;
mod types;
mod helpers;

// Public re-exports
pub use constants::*;
pub use types::*;
pub use helpers::*;
```

**When to use rustfmt skip:**
- Large `mod.rs` files with many exports
- Deliberately structured code alignment for clarity
- Manual grouping of related items (e.g., test fixtures)
- Files where organization conveys semantic meaning

**When NOT to use:**
- Small, simple mod.rs files
- When automatic formatting is preferred

### Step 3: Apply Conditional Visibility for Docs and Tests

When you need a module to be:
- **Private in production builds** (encapsulation)
- **Public for documentation** (rustdoc links work)
- **Public for tests** (test code can access internals)

Use conditional compilation:

```rust
// mod.rs - Conditional visibility

#[cfg(any(test, doc))]
pub mod internal_parser;
#[cfg(not(any(test, doc)))]
mod internal_parser;

// Re-export items for the flat public API
pub use internal_parser::*;
```

**How this works:**
- In doc builds: Module is public → rustdoc can see and link to it
- In test builds: Module is public → tests can access internals
- In production builds: Module is private → internal implementation detail

**This pattern is frequently used with the `fix-intradoc-links` skill when fixing documentation
links to private types.**

#### When to Omit the Fallback Branch

You can skip the `#[cfg(not(any(test, doc)))]` fallback when the module has **no code to compile
in production**:

```rust
// ✅ OK to skip fallback - documentation-only module (no actual code)
#[cfg(any(test, doc))]
pub mod integration_tests_docs;

// ✅ OK to skip fallback - all submodules are #[cfg(test)] anyway
#[cfg(any(test, doc))]
pub mod integration_tests;
```

**Keep the fallback** when the module contains code that must compile in production (even if
private):

```rust
// ✅ Need fallback - module has actual code used in production
#[cfg(any(test, doc))]
pub mod internal_parser;
#[cfg(not(any(test, doc)))]
mod internal_parser;

pub use internal_parser::*;  // Re-exports need the module to exist!
```

#### Platform-Specific Test Modules

For test modules that are **both test-only AND platform-specific**, combine the conditions:

```rust
// ✅ Linux-only test module, visible in docs and tests on Linux
#[cfg(all(any(test, doc), target_os = "linux"))]
pub mod backend_compat_tests;
```

**When you see this warning:**
> "unresolved link to `crate::path::test_module`"
>
> And the module is `#[cfg(test)]` or `#[cfg(all(test, target_os = "..."))]`

**Fix by adding conditional doc visibility:**

```rust
// Before (links won't resolve in docs):
#[cfg(test)]
mod my_test_module;

// After (links resolve in docs):
#[cfg(any(test, doc))]
pub mod my_test_module;

// Or for platform-specific:
#[cfg(all(any(test, doc), target_os = "linux"))]
pub mod my_linux_test_module;
```

**Apply at all levels** — If the test module is nested, both parent and child need the visibility
change.

### Step 4: Handle Transitive Visibility

**Important:** If a conditionally public module links to another module in its documentation,
that target module must also be conditionally public.

```rust
// mod.rs

#[cfg(any(test, doc))]
pub mod paint_impl;  // Contains docs that link to diff_chunks
#[cfg(not(any(test, doc)))]
mod paint_impl;

#[cfg(any(test, doc))]
pub mod diff_chunks;  // Must also be conditionally public!
#[cfg(not(any(test, doc)))]
mod diff_chunks;

// Re-export for public API
pub use paint_impl::*;
pub use diff_chunks::*;
```

**Why:** Rustdoc needs to resolve all links in documentation. If `paint_impl` docs link to
`diff_chunks`, rustdoc must be able to see `diff_chunks`.

### Step 5: Reference in Rustdoc

When linking to conditionally public modules in documentation, use the `mod@` prefix:

```rust
/// See [`internal_parser`] for implementation details.
///
/// [`internal_parser`]: mod@crate::internal_parser
```

See the `fix-intradoc-links` skill for complete details on rustdoc links.

## Benefits of This Pattern

### 1. Clean, Flat API

Users import directly without unnecessary nesting:

**✅ Good (flat, ergonomic):**
```rust
use my_module::MyType;
use my_module::CONSTANT;
```

**❌ Bad (exposes internal structure):**
```rust
use my_module::types::MyType;
use my_module::constants::CONSTANT;
```

### 2. Refactoring Freedom

Internal reorganization doesn't break external code:

```rust
// You can move items between files freely
// External API stays: use my_module::Item;

// Before:
// mod.rs: pub use types::Item;
// types.rs: pub struct Item;

// After refactoring:
// mod.rs: pub use helpers::Item;  // Changed!
// helpers.rs: pub struct Item;    // Moved!

// External code unaffected:
// use my_module::Item;  // Still works!
```

### 3. Avoid Naming Conflicts

Private module names don't pollute the namespace:

```rust
// No conflicts with other `constants` modules in the crate
mod constants;  // Private - name hidden
pub use constants::*;  // Items public

// Elsewhere in the crate
mod constants;  // No conflict! This is in a different scope
```

### 4. Encapsulation

Module structure is an implementation detail, not part of the API:

```rust
// Internal structure can change without breaking compatibility
// v1.0: mod types; pub use types::*;
// v1.1: mod models; pub use models::*;  // Renamed module
// Users don't care!
```

## Decision Trees

### When to Use Private Modules + Public Re-exports

**✅ Use this pattern when:**

- Module structure is an implementation detail
- You want a flat, ergonomic API surface
- Avoiding potential name collisions across the crate
- Working with small to medium-sized modules with clear responsibilities
- Building a library with a stable public API

**Example scenarios:**
- Utility modules with helpers, types, constants
- Internal parser implementation
- Data structure implementations

### When NOT to Use This Pattern

**❌ Keep modules public when:**

#### 1. Module Structure IS the API

Different domains should be explicit:

```rust
pub mod frontend;  // Frontend-specific APIs
pub mod backend;   // Backend-specific APIs

// Users: use my_crate::frontend::Component;
// Users: use my_crate::backend::Database;
```

**Why:** The separation is meaningful to users. They WANT to know if they're using frontend or
backend APIs.

#### 2. Large Feature Domains

When namespacing provides clarity for 100+ items:

```rust
pub mod graphics;   // 100+ graphics-related items
pub mod audio;      // 100+ audio-related items
pub mod physics;    // 100+ physics-related items

// Users: use engine::graphics::Renderer;
// Users: use engine::audio::Mixer;
```

**Why:** Flat re-export of 300+ items would be overwhelming. Namespacing aids discovery.

#### 3. Optional/Conditional Features

Make feature boundaries explicit:

```rust
#[cfg(feature = "async")]
pub mod async_api;  // Keep separate for clarity

#[cfg(feature = "serde")]
pub mod serialization;

// Users: use my_crate::async_api::Client;
```

**Why:** Users need to know which features enable which APIs.

### Inner Modules vs. Separate Files

When organizing code into logical groups, choose between **inner modules** (same file) and
**separate files** based on file size and complexity.

#### Inner Modules (Same File)

**✅ Use inner modules when:**

- File is small-to-medium (under ~300 lines total)
- Groups are logically related and benefit from proximity
- Comment banners (`// ======`) are being used to separate sections
- Each group is relatively small (~20-50 lines)

```rust
// ansi_sequence_generator.rs - Inner module pattern

pub struct AnsiSequenceGenerator;

mod cursor_movement {
    use super::*;
    impl AnsiSequenceGenerator {
        pub fn cursor_position(...) -> String { ... }
        pub fn cursor_to_column(...) -> String { ... }
    }
}

mod screen_clearing {
    use super::*;
    impl AnsiSequenceGenerator {
        pub fn clear_screen() -> String { ... }
        pub fn clear_current_line() -> String { ... }
    }
}

mod color_ops {
    use super::*;
    impl AnsiSequenceGenerator {
        pub fn fg_color(...) -> String { ... }
        pub fn bg_color(...) -> String { ... }
    }
}
```

**Benefits:**
- Single-file cohesion - everything related stays together
- Easier navigation - no jumping between files
- Clear grouping - `mod` keyword is more formal than comment banners
- Scoped imports - each inner mod can import only what it needs

#### Separate Files

**✅ Use separate files when:**

- Individual groups exceed ~100 lines each
- Groups have distinct dependencies (different imports)
- File would exceed ~500 lines total
- Groups are conceptually independent (could be tested separately)

```
generator/
├── mod.rs                    # Re-exports + struct definition
├── cursor_movement.rs        # impl AnsiSequenceGenerator { cursor_* }
├── screen_clearing.rs        # impl AnsiSequenceGenerator { clear_* }
├── color_ops.rs              # impl AnsiSequenceGenerator { colors }
└── terminal_modes.rs         # impl AnsiSequenceGenerator { modes }
```

#### Code Smell: Comment Banners

If you find yourself writing comment banners like this:

```rust
impl MyStruct {
    // ==================== Group A ====================
    fn method_a1() { ... }
    fn method_a2() { ... }

    // ==================== Group B ====================
    fn method_b1() { ... }
    fn method_b2() { ... }
}
```

**This is a signal to formalize the grouping** using either inner modules (small file) or
separate files (large file). Comment banners are informal and don't provide the same benefits
as actual module boundaries (scoped imports, clear boundaries, IDE navigation).

## Complete Examples

### Example 1: Simple Module Organization

```rust
// src/terminal/mod.rs

// Skip rustfmt for deliberate organization
#![cfg_attr(rustfmt, rustfmt_skip)]

// Core types
mod position;
mod size;
mod style;

// State management
mod cursor;
mod buffer;

// Public API - flat exports
pub use position::*;
pub use size::*;
pub use style::*;
pub use cursor::*;
pub use buffer::*;
```

Usage:
```rust
use terminal::{Position, Size, Style, Cursor, Buffer};
// Not: use terminal::position::Position;
```

### Example 2: Conditional Visibility for Docs

```rust
// src/parser/mod.rs

// Make internal modules public for docs and tests
#[cfg(any(test, doc))]
pub mod vt_100;
#[cfg(not(any(test, doc)))]
mod vt_100;

#[cfg(any(test, doc))]
pub mod escape_sequences;
#[cfg(not(any(test, doc)))]
mod escape_sequences;

// Public API
pub use vt_100::*;
pub use escape_sequences::*;
```

Now rustdoc can link to these modules:
```rust
/// Uses [`vt_100`] for parsing.
///
/// [`vt_100`]: mod@crate::parser::vt_100
```

### Example 3: Mixed Public and Private Modules

```rust
// src/rendering/mod.rs

// Public modules (API namespacing)
pub mod backends;     // Different backend implementations
pub mod widgets;      // UI widgets

// Private modules (internal implementation)
mod buffer;
mod diff_engine;

// Selective re-exports
pub use buffer::RenderBuffer;  // This one is public
// diff_engine stays internal
```

Usage:
```rust
use rendering::backends::Crossterm;
use rendering::widgets::Button;
use rendering::RenderBuffer;  // Flat re-export
// Cannot use: rendering::diff_engine  // Private!
```

## Common Mistakes

### ❌ Mistake 1: Everything Public

```rust
// mod.rs - Bad!
pub mod constants;
pub mod types;
pub mod helpers;
```

**Problem:** Exposes internal structure, hard to refactor later.

### ❌ Mistake 2: Forgetting Transitive Visibility

```rust
// mod.rs - Bad!
#[cfg(any(test, doc))]
pub mod a;  // Docs link to module b

mod b;  // Private! Rustdoc can't see it
```

**Problem:** Rustdoc can't resolve links from `a` to `b`.

**Fix:**
```rust
#[cfg(any(test, doc))]
pub mod a;

#[cfg(any(test, doc))]
pub mod b;  // Also conditionally public
#[cfg(not(any(test, doc)))]
mod b;
```

### ❌ Mistake 3: Using Conditional Visibility Everywhere

```rust
// mod.rs - Overkill!
#[cfg(any(test, doc))]
pub mod utils;
#[cfg(not(any(test, doc)))]
mod utils;
```

**Problem:** Only use conditional visibility when:
- Linking to the module in rustdoc, OR
- Accessing the module from test code

**Simple case:** If module items are re-exported and you don't need to link to the module itself,
just use private modules.

## Reporting Results

After organizing modules:

- ✅ Organized successfully → "Module structure organized with private modules and public re-exports!"
- 🔧 Made conditionally public → Report which modules got conditional visibility
- 📝 Manual review needed → List modules that may need public exposure for API reasons

## Supporting Files in This Skill

This skill includes additional reference material:

- **`examples.md`** - 6 complete, working examples of module organization for different scenarios: simple library with internal structure, conditional visibility for documentation, large crate with domain separation, test-only module visibility, gradual refactoring strategy, and avoiding naming conflicts. Each example shows full file structure and implementation. **Read this when:**
  - Simple library module organization → Example 1
  - Need conditional visibility for docs/tests → Example 2
  - Large crate with multiple domains (graphics/audio/physics) → Example 3
  - Test utilities that should only exist in test builds → Example 4
  - Refactoring from public modules to private + re-exports → Example 5
  - Avoiding module naming conflicts → Example 6
  - Decision tree for when to use which pattern → End of file

## Related Skills

- `write-documentation` - For documenting module organization
- `fix-intradoc-links` - Uses conditional visibility for linking private types
- `run-clippy` - Ensures mod.rs follows patterns

## Related Commands

No dedicated command, but used by:
- `/clippy` - Checks module organization as part of code quality

## Related Agents

- `clippy-runner` - Invokes this skill to enforce module patterns
