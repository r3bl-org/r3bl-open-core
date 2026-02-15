# Complete Module Organization Examples

This document provides full, working examples of module organization patterns for different scenarios.

---

## Example 1: Simple Library with Internal Structure

**Project:** A terminal color library

### File Structure

```
color_lib/
├── src/
│   ├── lib.rs
│   ├── colors/
│   │   ├── mod.rs        ← Module coordinator
│   │   ├── rgb.rs
│   │   ├── ansi.rs
│   │   └── named.rs
│   └── utils/
│       ├── mod.rs
│       └── convert.rs
```

### Implementation

**src/lib.rs:**
```rust
//! A library for working with terminal colors.

pub mod colors;  // Public module (API namespace)

// Internal utilities - private
mod utils;
```

**src/colors/mod.rs:**
```rust
//! Color representations and conversions.

// Skip rustfmt to preserve organization
#![cfg_attr(rustfmt, rustfmt_skip)]

// Private submodules (internal structure)
mod rgb;
mod ansi;
mod named;

// Public re-exports (flat API)
pub use rgb::RgbColor;
pub use ansi::AnsiColor;
pub use named::NamedColor;

// Re-export utils for this module's use, but keep private to external users
pub(crate) use crate::utils::*;
```

**src/colors/rgb.rs:**
```rust
/// An RGB color representation.
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl RgbColor {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}
```

**src/colors/ansi.rs:**
```rust
/// ANSI color codes (0-255).
pub struct AnsiColor(pub u8);
```

**src/colors/named.rs:**
```rust
/// Named terminal colors.
pub enum NamedColor {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
}
```

### Usage (External User Perspective)

```rust
// Clean, flat API
use color_lib::colors::{RgbColor, AnsiColor, NamedColor};

let rgb = RgbColor::new(255, 0, 0);
let ansi = AnsiColor(196);
let named = NamedColor::Red;

// Cannot access internal modules
// use color_lib::colors::rgb::RgbColor;  ❌ Compile error! `rgb` is private
```

---

## Example 2: Conditional Visibility for Documentation

**Project:** A parser with internal modules that need rustdoc links

### File Structure

```
parser/
├── src/
│   ├── lib.rs
│   └── parser/
│       ├── mod.rs
│       ├── vt_100.rs       ← Private but needs doc links
│       ├── escape.rs       ← Private but needs doc links
│       └── state.rs
```

### Implementation

**src/parser/mod.rs:**
```rust
//! VT-100 escape sequence parser.
//!
//! Uses [`vt_100`] for parsing and [`escape`] for sequence handling.
//!
//! [`vt_100`]: mod@crate::parser::vt_100
//! [`escape`]: mod@crate::parser::escape

// Conditional visibility for docs and tests
#[cfg(any(test, doc))]
pub mod vt_100;
#[cfg(not(any(test, doc)))]
mod vt_100;

#[cfg(any(test, doc))]
pub mod escape;
#[cfg(not(any(test, doc)))]
mod escape;

// Regular private module
mod state;

// Public API - flat re-exports
pub use vt_100::*;
pub use escape::*;
pub use state::*;
```

**src/parser/vt_100.rs:**
```rust
/// VT-100 parser implementation.
///
/// See [`escape`] module for escape sequence details.
///
/// [`escape`]: mod@crate::parser::escape
pub struct Vt100Parser {
    // Implementation...
}
```

### Documentation Build Result

```bash
$ cargo doc --no-deps
   Documenting parser v0.1.0
    Finished dev [unoptimized + debuginfo] target(s)
```

✅ All rustdoc links resolve! The `#[cfg(any(test, doc))]` makes modules visible to rustdoc.

### Production Build Result

```bash
$ cargo build --release
   Compiling parser v0.1.0
    Finished release [optimized] target(s)
```

✅ Modules are private in production! The `#[cfg(not(any(test, doc)))]` keeps them internal.

---

## Example 3: Large Crate with Domain Separation

**Project:** A game engine with distinct domains

### File Structure

```
game_engine/
├── src/
│   ├── lib.rs
│   ├── graphics/
│   │   ├── mod.rs
│   │   ├── renderer.rs
│   │   ├── shader.rs
│   │   └── texture.rs
│   ├── audio/
│   │   ├── mod.rs
│   │   ├── mixer.rs
│   │   └── sound.rs
│   └── physics/
│       ├── mod.rs
│       ├── rigidbody.rs
│       └── collision.rs
```

### Implementation

**src/lib.rs:**
```rust
//! A game engine with graphics, audio, and physics.

// Public modules - domain separation is part of the API
pub mod graphics;
pub mod audio;
pub mod physics;
```

**src/graphics/mod.rs:**
```rust
//! Graphics rendering subsystem.

// Private internal modules
mod renderer;
mod shader;
mod texture;

// Public re-exports
pub use renderer::Renderer;
pub use shader::Shader;
pub use texture::Texture;
```

**src/audio/mod.rs:**
```rust
//! Audio mixing subsystem.

// Private internal modules
mod mixer;
mod sound;

// Public re-exports
pub use mixer::Mixer;
pub use sound::Sound;
```

**src/physics/mod.rs:**
```rust
//! Physics simulation subsystem.

// Private internal modules
mod rigidbody;
mod collision;

// Public re-exports
pub use rigidbody::RigidBody;
pub use collision::CollisionShape;
```

### Usage

```rust
// Namespaced API - domains are explicit
use game_engine::graphics::{Renderer, Shader, Texture};
use game_engine::audio::{Mixer, Sound};
use game_engine::physics::{RigidBody, CollisionShape};

// Users benefit from seeing the domain separation
let renderer = graphics::Renderer::new();
let mixer = audio::Mixer::new();
let body = physics::RigidBody::new();
```

**Why keep modules public here?**
- The domain separation (graphics vs audio vs physics) is meaningful to users
- Each namespace has 20+ items, so grouping aids discovery
- Feature flags might enable/disable entire domains

---

## Example 4: Test-Only Module Visibility

**Project:** A library with extensive test utilities

### File Structure

```
my_lib/
├── src/
│   ├── lib.rs
│   ├── core/
│   │   ├── mod.rs
│   │   └── processor.rs
│   └── test_utils/
│       ├── mod.rs
│       ├── fixtures.rs
│       └── assertions.rs
```

### Implementation

**src/lib.rs:**
```rust
//! Core library functionality.

pub mod core;

// Test utilities - visible only in test builds
#[cfg(any(test, doc))]
pub mod test_utils;
```

**src/test_utils/mod.rs:**
```rust
//! Test utilities for validating library behavior.
//!
//! **Note:** This module is only available in test builds.

mod fixtures;
mod assertions;

pub use fixtures::*;
pub use assertions::*;
```

**src/core/processor.rs:**
```rust
#[cfg(test)]
mod tests {
    use crate::test_utils::*;  // Can access test_utils in test builds

    #[test]
    fn test_processor() {
        let fixture = create_test_fixture();
        // Test with fixture...
    }
}
```

### Build Behavior

**Normal build:**
```bash
$ cargo build
   Compiling my_lib v0.1.0
```
`test_utils` not included ✅ (smaller binary)

**Test build:**
```bash
$ cargo test
   Compiling my_lib v0.1.0
   Running unittests src/lib.rs
```
`test_utils` available ✅ (tests can use fixtures)

**Doc build:**
```bash
$ cargo doc
   Documenting my_lib v0.1.0
```
`test_utils` visible ✅ (can link to test utilities in docs)

---

## Example 5: Gradual Refactoring Strategy

**Scenario:** Refactoring a crate from public modules to private + re-exports

### Before (Version 1.0 - Public Modules)

```rust
// src/lib.rs
pub mod constants;
pub mod types;
pub mod functions;
```

**Problem:** Internal structure is exposed. If we want to reorganize, it breaks compatibility.

### After (Version 2.0 - Private + Re-exports)

```rust
// src/lib.rs

// Private modules (internal structure)
mod constants;
mod types;
mod functions;

// Public re-exports (stable API)
pub use constants::*;
pub use types::*;
pub use functions::*;
```

**Benefit:** Now we can refactor internal structure without breaking users!

### Version 2.1 - Internal Refactoring

```rust
// src/lib.rs

// Renamed modules internally
mod config;      // Was: constants
mod data_types;  // Was: types
mod utilities;   // Was: functions

// Same public API!
pub use config::*;
pub use data_types::*;
pub use utilities::*;
```

**User code unchanged:**
```rust
use my_lib::{CONSTANT_X, TypeY, function_z};
// Still works! Internal refactoring is invisible.
```

---

## Example 6: Avoiding Module Naming Conflicts

**Scenario:** Multiple modules might want the same internal module name

### Problem Without Private Modules

```rust
// src/frontend/mod.rs
pub mod utils;  // Naming conflict!

// src/backend/mod.rs
pub mod utils;  // Naming conflict!
```

Users see:
```rust
use my_crate::frontend::utils;  // Which utils?
use my_crate::backend::utils;   // Confusing!
```

### Solution With Private Modules

```rust
// src/frontend/mod.rs
mod utils;  // Private - name doesn't escape
pub use utils::*;

// src/backend/mod.rs
mod utils;  // Private - name doesn't escape
pub use utils::*;
```

Users see:
```rust
use my_crate::frontend::FrontendHelper;  // Clear!
use my_crate::backend::BackendHelper;    // Clear!
```

No naming conflict! Each `utils` module is private to its parent.

---

## Example 7: Cross-Platform Docs for Platform-Specific Code

**Scenario:** Linux-only code that should have documentation generated on all platforms (macOS, Windows)

### The Problem

You have a module that only works on Linux (e.g., uses `epoll`), but developers on macOS want to
read the documentation locally.

```rust
// ❌ Broken: Docs won't generate on macOS!
#[cfg(all(target_os = "linux", any(test, doc)))]
pub mod input;
```

The issue: `#[cfg(all(target_os = "linux", any(test, doc)))]` requires **both** Linux AND (test or
doc). On macOS, even during doc builds, `target_os = "linux"` is false, so the whole condition is
false.

### The Solution

Use `any(doc, ...)` to make documentation an **alternative path**, not an additional requirement:

```rust
// ✅ Fixed: Docs generate on all platforms (if module code is platform-agnostic)
#[cfg(any(doc, all(target_os = "linux", test)))]
pub mod input;
#[cfg(all(target_os = "linux", not(any(test, doc))))]
mod input;

// Re-export also needs the doc condition
#[cfg(any(target_os = "linux", doc))]
pub use input::*;
```

### File Structure

```
terminal_backend/
├── src/
│   ├── lib.rs
│   └── direct_to_ansi/
│       ├── mod.rs
│       ├── output.rs        ← Cross-platform
│       └── input/           ← Linux-only (uses epoll)
│           ├── mod.rs
│           ├── mio_poller.rs
│           └── integration_tests/
│               ├── mod.rs
│               └── pty_input_test.rs
```

### Implementation

**src/direct_to_ansi/mod.rs:**
```rust
//! DirectToAnsi backend for terminal I/O.
//!
//! - **Output**: Cross-platform (pure ANSI generation)
//! - **Input**: Linux-only (uses mio/epoll for stdin polling)
//!
//! See [`input`] module for Linux-specific input handling.
//!
//! [`input`]: mod@crate::direct_to_ansi::input

// Output is cross-platform
#[cfg(any(test, doc))]
pub mod output;
#[cfg(not(any(test, doc)))]
mod output;

// Input is Linux-only, but docs should build on all platforms
// Doc builds are allowed on all platforms so documentation can be read anywhere.
#[cfg(any(doc, all(target_os = "linux", test)))]
pub mod input;
#[cfg(all(target_os = "linux", not(any(test, doc))))]
mod input;

// Re-exports
pub use output::*;
#[cfg(any(target_os = "linux", doc))]
pub use input::*;
```

**src/direct_to_ansi/input/mod.rs:**
```rust
//! Linux input handling using mio/epoll.
//!
//! This module is **Linux-only** at runtime but documentation is generated
//! on all platforms.

// Submodules also use the cross-platform doc pattern
#[cfg(any(doc, all(target_os = "linux", test)))]
pub mod mio_poller;
#[cfg(all(target_os = "linux", not(any(test, doc))))]
mod mio_poller;

#[cfg(any(doc, all(target_os = "linux", test)))]
pub mod integration_tests;

pub use mio_poller::*;
```

### Build Results

**macOS doc build:**
```bash
$ cargo doc --no-deps
   Documenting terminal_backend v0.1.0
    Finished dev [unoptimized + debuginfo] target(s)
   Generated target/doc/terminal_backend/index.html
```
✅ Docs generate! Links to `input` module resolve correctly.

**macOS regular build:**
```bash
$ cargo build
   Compiling terminal_backend v0.1.0
    Finished dev [unoptimized + debuginfo] target(s)
```
✅ Input module excluded (Linux-only code not compiled).

**Linux test build:**
```bash
$ cargo test
   Compiling terminal_backend v0.1.0
   Running unittests
```
✅ Input module included and tests run.

### Key Insight

The `doc` cfg flag doesn't override other conditions—it's just another flag you can check. Use
`any()` to make it an **alternative path**:

| Pattern | Meaning | Docs on macOS? |
|:--------|:--------|:---------------|
| `all(target_os = "linux", any(test, doc))` | Linux AND (test OR doc) | ❌ No |
| `any(doc, all(target_os = "linux", test))` | doc OR (Linux AND test) | ✅ Yes |

### When the Module Uses Unix-Only APIs

The `cfg(any(doc, ...))` pattern assumes the module code compiles on **all** platforms. When the
module uses Unix-only APIs (e.g., `mio::unix::SourceFd`, `signal_hook`, `std::os::fd::AsRawFd`),
you must restrict doc builds to Unix platforms where the dependencies exist:

```rust
// Module uses Unix-only APIs — dependencies in Cargo.toml are cfg(unix).
// Doc builds are restricted to Unix (macOS/Linux); Windows excludes this module.
#[cfg(any(all(unix, doc), all(target_os = "linux", test)))]
pub mod input;
#[cfg(all(target_os = "linux", not(any(test, doc))))]
mod input;

// Re-export also needs the unix-gated doc condition
#[cfg(any(target_os = "linux", all(unix, doc)))]
pub use input::*;
```

**Three-tier platform hierarchy:**

| Module dependencies | Pattern | Docs: Linux | Docs: macOS | Docs: Windows |
| :------------------ | :------ | :---------- | :---------- | :------------ |
| Platform-agnostic (pure Rust) | `cfg(any(doc, ...))` | ✅ | ✅ | ✅ |
| Unix APIs (`mio::unix`, `signal_hook`) | `cfg(any(all(unix, doc), ...))` | ✅ | ✅ | excluded |
| Linux-only APIs | `cfg(any(all(target_os = "linux", doc), ...))` | ✅ | excluded | excluded |

**Rule of thumb:** Match your `doc` cfg guard to your dependency's `cfg` guard in `Cargo.toml`.

---

## Summary Checklist

When organizing modules, verify:

- [ ] Private modules for internal structure: `mod foo;`
- [ ] Public re-exports for stable API: `pub use foo::*;`
- [ ] Conditional visibility for doc links: `#[cfg(any(test, doc))] pub mod ...`
- [ ] Rustfmt skip for deliberate organization (if needed): `#![cfg_attr(rustfmt, rustfmt_skip)]`
- [ ] Transitive visibility for linked modules
- [ ] Public modules ONLY when namespacing is part of the API
- [ ] Documentation compiles: `./check.fish --quick-doc`
- [ ] Tests can access internal modules (if needed via `pub(crate)` or conditional visibility)

---

## Quick Decision Tree

```
Should this module be public?
│
├─ Is the namespace meaningful to users?
│  (e.g., graphics vs audio vs physics)
│  │
│  ├─ YES → Keep module public
│  │         pub mod graphics;
│  │
│  └─ NO → Continue...
│
├─ Does it have 100+ items that need grouping?
│  │
│  ├─ YES → Keep module public
│  │         pub mod large_feature;
│  │
│  └─ NO → Continue...
│
├─ Is it a feature flag boundary?
│  │
│  ├─ YES → Keep module public
│  │         #[cfg(feature = "async")]
│  │         pub mod async_api;
│  │
│  └─ NO → Use private module + re-export
│           mod internal;
│           pub use internal::*;
│
└─ Do docs need to link to this module?
   │
   ├─ YES → Use conditional visibility
   │         #[cfg(any(test, doc))]
   │         pub mod internal;
   │
   └─ NO → Keep fully private
             mod internal;
             pub use internal::*;
```

Use this decision tree when organizing any Rust module!
