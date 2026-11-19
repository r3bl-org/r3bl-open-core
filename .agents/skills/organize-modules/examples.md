# Complete Module Organization Examples

This document provides full, working examples of module organization patterns for
different scenarios.

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

#![rustfmt::skip]

// Attach.
mod rgb;
mod ansi;
mod named;

// Re-export.
pub use rgb::RgbColor;
pub use ansi::AnsiColor;
pub use named::NamedColor;

// Re-export utils for this module's use, but keep private to external users.
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

## Example 2: Minimum-Scope Internal Visibility (`Folder-local` vs `Crate-wide`)

**Project:** A parser where `vt_100` is linked from a cousin module
(`src/renderer/mod.rs`) and `src/lib.rs`, while `escape` is only linked inside `parser/`
(from `parser/mod.rs` and sibling `parser/vt_100.rs`)

### File Structure

```
parser/
├── src/
│   ├── lib.rs                 ← Ancestor linking to `crate::parser::vt_100`
│   ├── renderer/mod.rs        ← Cousin linking to `crate::parser::vt_100`
│   └── parser/
│       ├── mod.rs
│       ├── vt_100.rs          ← Crate-wide `pub(crate) mod` (required for cousin/ancestor links)
│       ├── escape.rs          ← Folder-local `mod` (parent/sibling links resolve directly)
│       └── state.rs           ← Folder-local `mod` (internal state)
```

### Implementation

**src/parser/mod.rs:**

```rust
//! VT-100 escape sequence parser.
//!
//! Uses [`vt_100`] for parsing and [`escape`] for sequence handling.
//!
//! [`escape`]: mod@crate::parser::escape
//! [`vt_100`]: mod@crate::parser::vt_100

// Attach. Crate-wide (Intra-crate).
pub(crate) mod vt_100; // Cross-folder rustdoc module links (lib.rs, renderer).

// Attach. Folder-local (Intra-module).
mod escape;
mod state;

// Re-export.
pub use vt_100::*;
pub use escape::*;
pub use state::*;
```

**src/parser/vt_100.rs (sibling can link to plain `mod escape`):**

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

**src/renderer/mod.rs (cousin folder needs `pub(crate) mod vt_100` on target):**

```rust
//! Consumes [`vt_100`] output.
//!
//! [`vt_100`]: mod@crate::parser::vt_100
```

### Documentation Build Result

```bash
cargo doc --no-deps
# Documenting parser v0.1.0
# Finished `dev` profile [unoptimized + debuginfo] target(s)
```

✅ All rustdoc links resolve! Parent and sibling links to `escape` work with plain `mod`,
while cousin and ancestor links to `vt_100` resolve via `pub(crate) mod vt_100;`, with
zero dual-arm `#[cfg(any(test, doc))]` shims.

### Production Build Result

```bash
cargo build --release
# Compiling parser v0.1.0
# Finished `release` profile [optimized] target(s)
```

✅ Modules stay out of the external public API! External crates see only the flat
`pub use` re-exports, while intra-crate paths and docs resolve cleanly via
`pub(crate) mod`.

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

// Attach.
pub mod graphics; // Public (Inter-crate): domain separation is part of the API.
pub mod audio;
pub mod physics;
```

**src/graphics/mod.rs:**

```rust
//! Graphics rendering subsystem.

// Attach.
mod renderer;
mod shader;
mod texture;

// Re-export.
pub use renderer::Renderer;
pub use shader::Shader;
pub use texture::Texture;
```

**src/audio/mod.rs:**

```rust
//! Audio mixing subsystem.

// Attach.
mod mixer;
mod sound;

// Re-export.
pub use mixer::Mixer;
pub use sound::Sound;
```

**src/physics/mod.rs:**

```rust
//! Physics simulation subsystem.

// Attach.
mod rigidbody;
mod collision;

// Re-export.
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

// Attach.
pub mod core;

// Re-export.
// (none: `core` is a Public domain hub)

// Single-arm gate keeps test-only code in the AST for `cargo doc` and `cargo test`,
// while excluding it from production and release binaries.
#[cfg(any(test, doc))]
pub(crate) mod test_utils; // Shared test fixtures accessed by crate unit tests.
```

**src/test_utils/mod.rs:**

```rust
//! Test utilities for validating library behavior.
//!
//! **Note:** This module is compiled only in test and documentation builds.

// Attach.
mod fixtures;
mod assertions;

// Re-export.
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

**Normal and release builds:**

```bash
cargo build --release
# Compiling my_lib v0.1.0
```

`test_utils` excluded ✅ (no test code shipped in release binaries)

**Test build:**

```bash
cargo test
# Compiling my_lib v0.1.0
# Running unittests src/lib.rs
```

`test_utils` compiled ✅ (tests across the crate can use fixtures via `pub(crate)`)

**Doc build:**

```bash
cargo doc
# Documenting my_lib v0.1.0
```

`test_utils` kept in the AST ✅ (`--document-private-items` documents private AST items
but does not enable `cfg(test)`, so single-arm `#[cfg(any(test, doc))]` keeps `test_utils`
in the AST for `cargo doc` while excluding it from release binaries)

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

**Problem:** Internal structure is exposed. If we want to reorganize, it breaks
compatibility.

### After (Version 2.0 - Private + Re-exports)

```rust
// src/lib.rs

// Attach.
mod constants;
mod types;
mod functions;

// Re-export.
pub use constants::*;
pub use types::*;
pub use functions::*;
```

**Benefit:** Now we can refactor internal structure without breaking users!

### Version 2.1 - Internal Refactoring

```rust
// src/lib.rs

// Attach (renamed modules internally).
mod config;     // Was: constants.
mod data_types; // Was: types.
mod utilities;  // Was: functions.

// Re-export (same public API).
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
// Attach.
mod utils; // Folder-local: name doesn't escape.
// Re-export.
pub use utils::*;

// src/backend/mod.rs
// Attach.
mod utils; // Folder-local: name doesn't escape.
// Re-export.
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

**Scenario:** Separating Linux-only production code (`direct_to_ansi::input`) from
platform-specific test-only modules whose docs should build across platforms

### Part 1: Platform-Specific Production Modules (`direct_to_ansi` Pattern)

In `direct_to_ansi`, `output` is cross-platform while `input` depends on Linux
`mio`/`epoll` APIs. Because `--document-private-items` is enabled workspace-wide, neither
module uses dual-arm `#[cfg(any(test, doc))]` / negated-`cfg` shims:

- `output` uses unconditional `pub(crate) mod output;`.
- `input` uses `#[cfg(target_os = "linux")] pub(crate) mod input;` (excluded on macOS and
  Windows across all builds, documented on Linux and the Linux `docs.rs` target).
- Test-only modules and documentation stubs inside `input` use single-arm
  `#[cfg(any(test, doc))]`.

### File Structure

```
terminal_backend/
├── src/
│   ├── lib.rs
│   └── direct_to_ansi/
│       ├── mod.rs
│       ├── debug.rs                 ← Folder-local (`mod`)
│       ├── output/                  ← Crate-wide cross-platform (`pub(crate) mod`)
│       └── input/                   ← Crate-wide Linux-only (`#[cfg(target_os = "linux")] pub(crate) mod`)
│           ├── mod.rs
│           ├── mio_poller/
│           └── integration_tests_stub/
│               └── mod.rs           ← Crate-wide doc stub (`#[cfg(any(test, doc))] pub(crate) mod`)
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
//! [`input`]: mod@crate::terminal_lib_backends::direct_to_ansi::input

// Attach. Crate-wide (Intra-crate).
pub(crate) mod output; // Cross-folder rustdoc links (terminal_lib_backends, direct_to_ansi).
// Input handling is Linux-only because macOS kqueue does not support PTY/tty polling.
// Non-Linux platforms (macOS/Windows) exclude this module across all builds.
#[cfg(target_os = "linux")]
pub(crate) mod input; // Cross-folder input poller types and rustdoc links.

// Attach. Folder-local (Intra-module).
mod debug;

// Re-export.
pub use debug::*;
pub use output::*;
#[cfg(target_os = "linux")]
pub use input::*;
```

**src/direct_to_ansi/input/mod.rs:**

```rust
//! Linux input handling using mio/epoll.

// Attach. Crate-wide (Intra-crate).
pub(crate) mod mio_poller; // Cross-folder poller types and rustdoc links.
pub(crate) mod channel_types; // Cross-folder PollerEvent imports and rustdoc links.

// Attach. Folder-local (Intra-module).
mod input_device_impl;
mod input_device_public_api;

// Re-export.
pub use input_device_impl::*;
pub use input_device_public_api::*;
pub use mio_poller::*;

// Documentation-only module pointing to actual PTY tests location.
#[cfg(any(test, doc))]
pub(crate) mod integration_tests_stub; // Cross-folder rustdoc module links (direct_to_ansi, output tests).
```

### Part 2: Cross-Platform Docs for Platform-Specific Test-Only Modules

When a **test-only module** should run tests only on Linux (or Unix) while generating docs
on other platforms, avoid requiring `target_os = "linux"` for `doc`:

```rust
// ❌ Broken: Docs will not generate on macOS or Windows!
#[cfg(all(target_os = "linux", any(test, doc)))]
pub(crate) mod linux_only_tests;

// ✅ Fixed (pure Rust test module): Docs generate on all platforms; tests run on Linux.
#[cfg(any(doc, all(target_os = "linux", test)))]
pub(crate) mod linux_only_tests;
```

### Key Insight

The `doc` cfg flag does not override other conditions: it is just another flag you can
check. Use `any()` to make it an **alternative path** on test-only modules:

| Pattern                                    | Meaning                 | Docs on macOS? |
| :----------------------------------------- | :---------------------- | :------------- |
| `all(target_os = "linux", any(test, doc))` | Linux AND (test OR doc) | ❌ No          |
| `any(doc, all(target_os = "linux", test))` | doc OR (Linux AND test) | ✅ Yes         |

### When a Test Module Uses Unix-Only APIs

The `cfg(any(doc, ...))` pattern assumes the module code compiles on **all** platforms.
When a test-only module uses Unix-only APIs (e.g., `mio::unix::SourceFd`, `signal_hook`,
`std::os::fd::AsRawFd`), restrict doc builds to Unix platforms where the dependencies
exist:

```rust
// Single-arm gate for a test-only module using Unix-only APIs:
// docs build on Unix (macOS/Linux); tests run on Linux; Windows excludes this module.
#[cfg(any(all(unix, doc), all(target_os = "linux", test)))]
pub(crate) mod integration_tests;
```

**Three-tier platform hierarchy for test-only modules:**

| Module dependencies                    | Pattern                                        | Docs: Linux | Docs: macOS | Docs: Windows |
| :------------------------------------- | :--------------------------------------------- | :---------- | :---------- | :------------ |
| Platform-agnostic (pure Rust)          | `cfg(any(doc, ...))`                           | ✅          | ✅          | ✅            |
| Unix APIs (`mio::unix`, `signal_hook`) | `cfg(any(all(unix, doc), ...))`                | ✅          | ✅          | excluded      |
| Linux-only APIs                        | `cfg(any(all(target_os = "linux", doc), ...))` | ✅          | excluded    | excluded      |

**Rule of thumb:** Match your `doc` cfg guard to your dependency's `cfg` guard in
`Cargo.toml`.

---

## Summary Checklist

When organizing modules, verify:

- [ ] **Folder-local (Intra-module)** default for parent/sibling scope: `mod foo;`
- [ ] **Crate-wide (Intra-crate)** only when cousin folders, out-of-folder tests, or
      cross-folder `mod@crate::...` docs need path access, with a 1-line inline reason
      comment: `pub(crate) mod foo; // <reason>.`
- [ ] **Public (Inter-crate)** (`pub mod`) ONLY for intentional domain hubs or explicit
      non-globbed API namespaces (never combine with `pub use foo::*;` or `#[doc(inline)]`)
- [ ] Top `#[macro_use]` block (when needed) placed first under
      `// Attach. Upward #[macro_use] propagation (must precede modules below that invoke their macros).`
      in textual macro-dependency order (never split by visibility)
- [ ] Progressive-disclosure headers: single `// Attach.` for single-tier coordinators, or
      wide-to-narrow `// Attach. Public (Inter-crate).`,
      `// Attach. Crate-wide (Intra-crate).`, `// Attach. Folder-local (Intra-module).`
      when tiers mix
- [ ] Barrel exports introduced with `// Re-export.` (`pub use foo::*;`)
- [ ] Single-arm `#[cfg(any(test, doc))]` (on `pub(crate) mod ... // <reason>.`) ONLY for
      test-only modules and doc-only test stubs (zero dual-arm negated-`cfg` fallbacks)
- [ ] Rustfmt skip for deliberate organization (if needed): `#![rustfmt::skip]`
- [ ] Transitive `pub(crate) mod` visibility along cross-module rustdoc paths
- [ ] Documentation compiles cleanly: `./check.fish --quick-doc`

---

## Quick Decision Tree

```text
What visibility tier should this module have?
│
├─ Is it a test-only module (unit/integration tests or test fixtures)?
│  │
│  ├─ YES → Gate with single-arm cfg so cargo doc & cargo test see it,
│  │         while release builds exclude it:
│  │         #[cfg(any(test, doc))]
│  │         pub(crate) mod my_module_tests; // <1-line reason>.
│  │
│  └─ NO (production module) → Continue...
│
├─ Is the namespace meaningful to external users (domain hub or non-globbed leaf API)?
│  (e.g., graphics vs audio vs physics, mouse_sgr, 100+ items, or feature flag boundary)
│  │
│  ├─ YES → Public (Inter-crate): never glob-re-exported
│  │         pub mod graphics;
│  │
│  └─ NO → Keep out of external public API; re-export items under `// Re-export.` with `pub use`:
│           │
│           ├─ Do ancestor modules (grandparent/`lib.rs`), cousin folders, or out-of-folder
│           │  tests need path access in Rust code or `mod@crate::...` rustdoc links?
│           │  │
│           │  ├─ YES → Crate-wide (Intra-crate) with mandatory 1-line inline reason:
│           │  │         pub(crate) mod internal; // <1-line reason>.
│           │  │         pub use internal::*;
│           │  │
│           │  └─ NO (only parent `mod.rs` or siblings in this folder reference it)
│           │      → Folder-local (Intra-module): documented automatically via `--document-private-items`:
│           │        mod internal;
│           │        pub use internal::*;
```

Use this decision tree when organizing any Rust module!

---

## Example 8: Multi-Level Barrel Exports Without `#[doc(inline)]` (Root-Cause Fix)

**Project:** A library with deeply nested constants (`core::ansi::constants::{csi, dsr}`)
whose items are flattened into `core::ansi` (`pub use constants::*;`).

### Problem with Combining `pub mod` + `pub use *;`

Declaring `pub mod constants;` while simultaneously doing `pub use constants::*;` in
`core/ansi/mod.rs` causes rustdoc search to pick `core::ansi::csi` as the shortest public
path while building HTML at `core::ansi::constants::csi`, leaking internal structure and
tempting `#[doc(inline)]` shims.

### Root-Cause Fix

Keep barrel-exported submodules out of the public API (`pub(crate) mod` or `mod`) under
`--document-private-items`, and flatten consumers to import `crate::Item` directly:

**tui/src/core/ansi/mod.rs:**

```rust
//! ANSI module re-exports.

#![rustfmt::skip]

// Attach. Crate-wide (Intra-crate).
pub(crate) mod constants; // Cross-folder rustdoc module links (#design, mouse).

// Attach. Folder-local (Intra-module).
mod color;

// Re-export.
pub use color::*;
pub use constants::*;
```

**tui/src/core/ansi/constants/mod.rs:**

```rust
//! Well-documented constants hub.
//!
//! | Module | Description |
//! | :--- | :--- |
//! | [`csi`] | CSI sequences |
//! | [`dsr`] | DSR sequences |

#![rustfmt::skip]

// Attach. Upward #[macro_use] propagation (must precede modules below that invoke their macros).
#[macro_use]
mod ansi_const_macros;

// Attach. Crate-wide (Intra-crate).
pub(crate) mod mouse; // Cross-folder sequence generators and rustdoc links.

// Attach. Folder-local (Intra-module).
mod csi;
mod dsr;

// Re-export.
pub use csi::*;
pub use dsr::*;
pub use mouse::*;
```

### Result

1. Items are exposed cleanly at `r3bl_tui::{...}` without exposing `ansi::constants::csi`
   in the public API.
2. Under `--document-private-items`, internal modules and `mod@crate::...` rustdoc links
   resolve directly at their canonical paths with zero `#[doc(inline)]` workarounds.

---

## Example 9: Module That Defines `#[macro_export]` Macros

**Scenario:** A module defines `#[macro_export]` macros that are used both throughout the
crate and by downstream library consumers.

### The Anti-Pattern (Don't Do This)

```rust
// mod.rs - ❌ Siblings must use `use crate::my_macro;`, triggering future-incompat lint.
pub mod macros;
pub mod consumer_a;
pub mod consumer_b;
```

```rust
// consumer_a.rs - ❌ Triggers macro_expanded_macro_exports_accessed_by_absolute_paths.
use crate::my_macro;

fn example() {
    my_macro!();
}
```

### The Correct Pattern

Keep `#[macro_export]` on the macro (for `rustdoc`, doctests, and clickable links), keep
the macro module private, lift `#[macro_use]` **only** up to the lowest common ancestor of
its internal callers, and use the two-block `// Attach.` coordinator split (never split
the `#[macro_use]` block by visibility):

```rust
// parent/mod.rs - ✅ Textual propagation stops here if all callers are inside `parent`.

// Attach. Upward #[macro_use] propagation (must precede modules below that invoke their macros).
#[macro_use]
mod my_domain_macros;

// Attach.
pub mod consumer_a;
pub mod consumer_b;
```

```rust
// parent/my_domain_macros.rs
/// Macro-level documentation renders at `macro.my_macro.html`.
#[macro_export]
macro_rules! my_macro { () => {}; }
```

```rust
// parent/consumer_a.rs - ✅ No `use crate::my_macro;` needed; works in inner mods too.
fn example() {
    my_macro!();
}

mod inner_helper {
    fn helper() { my_macro!(); }
}
```

---

## Example 10: File and Module Naming Conventions

**Scenario:** A parser module with submodules, an entry point function, a primary struct
that avoids collision with its directory name, a public API facade, and auxiliary
non-colliding types.

### File Structure

```text
input_parser/
├── mod.rs                       # Coordinator and barrel exports
├── input_parser_entry_point.rs  # Standalone pure function entry point
├── chunk_framer/
│   ├── mod.rs                   # Coordinator
│   ├── chunk_framer_struct.rs   # Primary struct (avoids collision with chunk_framer/)
│   └── accumulator/
│       ├── mod.rs               # Coordinator
│       ├── accumulator_struct.rs # Primary struct (avoids collision with accumulator/)
│       ├── constants.rs         # Non-colliding auxiliary file
│       └── unparsed_buffer_action.rs # Non-colliding enum (keeps descriptive name)
└── parser_engine/
    ├── mod.rs
    ├── engine_struct.rs         # Primary struct (avoids collision with parser_engine/)
    ├── engine_public_api.rs     # User-facing public API methods
    └── engine_internal_api.rs   # Internal implementation methods
```

### Key Takeaways

1. **Inception collision avoidance**: `chunk_framer_struct.rs` inside `chunk_framer/`,
   `accumulator_struct.rs` inside `accumulator/`, and `engine_struct.rs` inside
   `parser_engine/`.
2. **Non-colliding files keep natural names**: `unparsed_buffer_action.rs` is not named
   `accumulator.rs`, so it does not collide and does not take `_enum.rs` or `_struct.rs`.
3. **Pure function entry points**: `input_parser_entry_point.rs` provides standalone
   parsing functions without primary data types.
4. **Public API facade**: `engine_public_api.rs` provides public methods for the struct,
   keeping internal methods in `engine_internal_api.rs`.
