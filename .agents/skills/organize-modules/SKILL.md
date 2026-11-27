---
name: organize-modules
description:
    Apply private modules with public re-exports (barrel export) pattern for clean API
    design. Covers 3-tier visibility (`Folder-local`, `Crate-wide`, `Public`),
    progressive-disclosure `// Attach.` and `// Re-export.` coordinator headers,
    `#[macro_use]` propagation, `--document-private-items` doc visibility, and
    single-arm `#[cfg(any(test, doc))]` for test-only modules. Use when creating
    modules, organizing mod.rs files, or before creating commits.
---

# Module Organization Best Practices

## When to Use

- Creating new Rust modules
- Refactoring module structure
- Organizing mod.rs files
- Reviewing code that exposes internal structure
- Structuring internal module visibility for docs and tests
- Before creating commits with module changes
- When user says "organize modules", "refactor modules", "fix module structure", etc.

## Instructions

Follow these patterns for clean, maintainable module organization:

### Step 1: Apply the Recommended Pattern

**Prefer private modules with public re-exports** (also known as the [barrel export]
pattern) as the default pattern.

This provides a clean API while maintaining flexibility to refactor internal structure.

[barrel export]:
    https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Statements/export#re-exporting_aggregating

```rust
// mod.rs - Module coordinator.

// Attach.
mod constants;
mod types;
mod helpers;

// Re-export.
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

#![rustfmt::skip]

// Attach.
mod constants;
mod types;
mod helpers;

// Re-export.
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

### Step 3: 3-Tier Visibility & Progressive-Disclosure Coordinator Headers

The workspace uses a 3-tier module visibility vocabulary:

| Declaration | Tier | Scope & Rules |
| :--- | :--- | :--- |
| `mod foo;` / `pub(super) mod foo;` | **Folder-local (Intra-module)** | Default minimum scope. Visible to parent `mod.rs` and folder siblings/descendants (and still documented via `--document-private-items`). |
| `pub(crate) mod foo; // <reason>.` | **Crate-wide (Intra-crate)** | Cross-folder crate-internal path access (cousin folders, ancestor-to-grandchild `mod@crate::...` rustdoc links, or out-of-folder tests). Always include a 1-line inline reason comment. |
| `pub mod foo;` | **Public (Inter-crate)** | Intentional public API namespace (domain hubs or explicit non-globbed namespaces). External crates can import paths through it. |

#### Coordinator `mod.rs` Section Ordering (Progressive Disclosure)

Structure module attachment and re-export blocks in `mod.rs` top-to-bottom as follows:

1. **Top `#[macro_use]` block (only when needed):** Always place upward-propagating
   macro modules first under:
   `// Attach. Upward #[macro_use] propagation (must precede modules below that invoke their macros).`
   **Never split a `#[macro_use]` block by visibility**, because textual macro expansion
   depends on declaration order (add a 1-line inline reason comment to any
   `pub(crate) mod` inside this block).
2. **Non-`#[macro_use]` module attachments (Progressive Disclosure):**
   - **Single-tier coordinator** (all remaining attachments share one tier, e.g., all
     `mod` or all `pub mod`): use a single concise `// Attach.` header.
   - **Mixed-tier coordinator** (2+ visibility tiers present): split wide-to-narrow with
     explicit tier headers (include only tiers present in that file):
     - `// Attach. Public (Inter-crate).`
     - `// Attach. Crate-wide (Intra-crate).`
     - `// Attach. Folder-local (Intra-module).`
3. **Re-exports:** Always introduce barrel exports with `// Re-export.`.
4. **Test and doc-only modules (when present):** Keep `#[cfg(test)]` and single-arm
   `#[cfg(any(test, doc))]` test/fixture modules separated cleanly from production
   attachments (preserving a 1-line inline reason comment on any `pub(crate) mod`).

#### Documenting Private and Crate-Wide Items (`--document-private-items`)

The workspace enables `--document-private-items` across all documentation workflows:

- `.cargo/config.toml` (`[build] rustdocflags`) for local `cargo doc` and IDE runs.
- `check_docs.fish` (`RUSTDOCFLAGS` in `run_cargo_doc`) for `./check.fish --quick-doc`,
  `--doc`, `--watch-doc`, and `--full`.
- `[package.metadata.docs.rs]` (`rustdoc-args`) in member `Cargo.toml` files (`tui`,
  `cmdr`, `build-infra`, `rust-analyzer-mcp-server`, `analytics_schema`).
- `[workspace.lints.rustdoc] private_intra_doc_links = "allow"` in root `Cargo.toml`,
  allowing public items to link to `pub(crate)` and private internal items.

Because `rustdoc` already documents private and `pub(crate)` items that exist in the AST,
do **not** use dual-arm `#[cfg(any(test, doc))]` / negated-`cfg` fallback shims on
production modules. Instead, default to **Folder-local** (`mod`) and widen to
**Crate-wide** (`pub(crate) mod`) only as needed:

- **`mod foo;` (Folder-local / Intra-module, default minimum scope)**: Use for all
  internal modules unless cross-folder paths require widening. Because
  `--document-private-items` generates HTML pages for private modules and items, **all
  four** of these work with plain `mod foo;`:
    1. HTML page generation for `foo` and every private item inside it.
    2. **Parent-to-child** links and imports (`parent/mod.rs` linking to `mod@foo` or
       `foo::Item`).
    3. **Sibling-to-sibling** links and imports (`parent/sibling_a.rs` accessing
       `mod@super::sibling_b` or `mod@crate::parent::sibling_b`, because children can see
       private items of their ancestors).
    4. **Descendant-to-ancestor-sibling** access (`parent/a/inner.rs` accessing
       `crate::parent::b`).
- **`pub(crate) mod foo;` (Crate-wide / Intra-crate, only when cross-folder paths fail)**:
  Upgrade from `mod foo;` to `pub(crate) mod foo;` **only** when either of these two
  cross-folder cases needs path access (flagged by `cargo check --workspace --all-targets`
  or `./check.fish --quick-doc`), and document the reason with a 1-line inline comment:
    1. **Cousin access across sibling folders**: `crate::folder_x::...` referenced from
       `folder_y/` in Rust code, tests, or `mod@crate::folder_x::sub` rustdoc links.
    2. **Ancestor-to-grandchild access**: `lib.rs` or `grandparent/mod.rs` architecture
       tables linking down into `mod@crate::grandparent::parent::child` (in Rust, children
       see parents' private `mod`, whereas parents cannot see children's private `mod`).

Historical note: before `--document-private-items` was enabled workspace-wide, even
parent-to-child and sibling-to-sibling doc links forced `#[cfg(any(test, doc))] pub mod`
because `rustdoc` omitted HTML pages for private modules. Now **Folder-local** `mod` is
always the default, and **Crate-wide** `pub(crate) mod` acts as an explicit signal that
code, tests, or docs outside the folder depend on that module path.

```rust
// mod.rs: Mixed-tier coordinator using Progressive Disclosure headers.

// Attach. Crate-wide (Intra-crate).
pub(crate) mod internal_parser; // Cross-folder rustdoc module links and integration tests.

// Attach. Folder-local (Intra-module).
mod helpers;

// Re-export.
pub use internal_parser::*;
```

Both `mod` and `pub(crate) mod` remain hidden from the crate's public API and never
trigger `ambiguous_glob_reexports` in `pub use *;` barrel exports, while staying
documented under `--document-private-items`.

#### Why `rustdoc` Still Requires `pub(crate) mod` for Cross-Folder Links

Even with `--document-private-items` and `private_intra_doc_links = "allow"`, `rustdoc`
still enforces `rustc`'s module privacy rules when resolving a path from the linking
module's scope (`--document-private-items` controls HTML page generation and
`private_intra_doc_links` controls the public-to-private lint, not path resolution):

```rust
pub mod a {
    mod secret {}            // ❌ `mod@crate::a::secret` from `b` fails (`no item named secret in module a`)
    pub(crate) mod shared {} // ✅ `mod@crate::a::shared` from `b` resolves; still hidden outside crate
}

/// Links to [`a::shared`].
///
/// [`a::shared`]: mod@crate::a::shared
pub mod b {}
```

#### Single-Arm `#[cfg(any(test, doc))]` for Test-Only Modules

Reserve single-arm `#[cfg(any(test, doc))]` (and platform variants such as
`#[cfg(any(all(unix, doc), test))]` or `#[cfg(any(doc, all(target_os = "linux", test)))]`
on `pub(crate) mod ...`) exclusively for **test-only modules** and documentation-only test
stubs. Running `cargo doc` with `--document-private-items` documents private items present
in the AST, but it does **not** enable `cfg(test)`. Single-arm gating keeps test modules
in the AST for `cargo doc` and `cargo test` while excluding them from production and
release builds:

```rust
// Test-only modules and documentation-only test stubs:
// compiled in `cargo test` and present in the AST for `cargo doc`,
// while excluded from production and release builds.
#[cfg(any(test, doc))]
pub(crate) mod unit_tests;

#[cfg(any(test, doc))]
pub(crate) mod integration_tests_docs;
```

#### Integration Tests Module

For modules that contain [`PTY`] integration tests (typically under
`<module>_integration_tests/`), follow these rules:

1.  **Dedicated Files**: Each complex test gets its own file.
2.  **Module Documentation**: Use `//!` at the top of the file to document the test's
    intent.
3.  **Run Instructions**: Always include a "Run with:" section at the top of the file (see
    `write-documentation` skill for details).
4.  **Single-Arm Test Gate**: Parent modules declare test-only modules with single-arm
    `#[cfg(any(test, doc))] pub(crate) mod ...;` (plus platform gates when needed) so
    rustdoc can resolve links without exposing test modules in the public API or shipping
    test code in release builds.

```rust
// <module>_integration_tests/mod.rs

#[cfg(any(test, doc))]
pub(crate) mod pty_feature_test;
```

[`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal

#### Platform-Specific Modules with Cross-Platform Docs

Separate platform-specific **production code** from platform-specific **test-only
modules**:

1. **Platform-specific production code** (such as `direct_to_ansi::input`): Gate directly
   with the target platform `cfg` and `pub(crate) mod` (with zero dual-arm negated-`cfg`
   fallbacks). Non-target platforms exclude the module across all builds:

```rust
// direct_to_ansi/mod.rs: Mixed-tier Linux-only production module.

// Attach. Crate-wide (Intra-crate).
pub(crate) mod output; // Cross-folder rustdoc links.

#[cfg(target_os = "linux")]
pub(crate) mod input; // Linux-only input poller types and rustdoc links.

// Attach. Folder-local (Intra-module).
mod debug;

// Re-export.
pub use debug::*;
pub use output::*;
#[cfg(target_os = "linux")]
pub use input::*;
```

2. **Platform-specific test-only modules (pure Rust, cross-platform docs)**: When a
   test-only module compiles on all platforms and should have docs generated everywhere
   while tests run only on Linux, use single-arm `any(doc, ...)`:

```rust
// ❌ Broken: Docs will not generate on macOS.
#[cfg(all(target_os = "linux", any(test, doc)))]
pub(crate) mod linux_only_tests;

// ✅ Fixed: Docs generate on all platforms; tests run only on Linux.
#[cfg(any(doc, all(target_os = "linux", test)))]
pub(crate) mod linux_only_tests;
```

**Key insight:** `rustdoc` runs the Rust compiler internally. When you write
`#[cfg(all(target_os = "linux", any(test, doc)))]`, the `target_os = "linux"` check still
excludes macOS and Windows **even during doc builds**. The `doc` cfg flag does not
override other conditions: it is just another flag you can check.

**The fix:** Use `any(doc, ...)` to make `doc` an alternative path:

- `any(doc, all(target_os = "linux", test))` means: "docs on any platform OR tests on
  Linux"
- `all(target_os = "linux", any(test, doc))` means: "Linux AND (tests OR docs)" (still
  requires Linux)

#### ⚠️ Unix Dependency Caveat

The `cfg(any(doc, ...))` pattern above assumes the module's code **compiles on all
platforms**. When a test-only module uses Unix-only APIs (e.g., `mio::unix::SourceFd`,
`signal_hook`, `std::os::fd::AsRawFd`), restrict doc builds to Unix using a single-arm
gate:

```rust
// Test-only module uses Unix-only APIs: restrict doc builds to Unix platforms.
#[cfg(any(all(unix, doc), all(target_os = "linux", test)))]
pub(crate) mod integration_tests;
```

For Unix-only production modules (such as `raw_mode_unix`), gate directly with
`#[cfg(unix)] pub(crate) mod raw_mode_unix;` without any `doc` or `test` arms.

**Three-tier hierarchy for test-only modules:**

| Module dependencies | Pattern                                        | Docs: Linux | Docs: macOS | Docs: Windows |
| :------------------ | :--------------------------------------------- | :---------- | :---------- | :------------ |
| Platform-agnostic   | `cfg(any(doc, ...))`                           | ✅          | ✅          | ✅            |
| Unix APIs           | `cfg(any(all(unix, doc), ...))`                | ✅          | ✅          | excluded      |
| Linux-only APIs     | `cfg(any(all(target_os = "linux", doc), ...))` | ✅          | excluded    | excluded      |

**Rule of thumb:** Match your `doc` cfg guard to your dependency's `cfg` guard in
`Cargo.toml`.

#### ⚠️ `[dev-dependencies]` Caveat (Two-Layer Split)

Cargo does **not** have a `[doc-dependencies]` table and **never links
`[dev-dependencies]`** (such as `mockito`, `serial_test`, `pretty_assertions`,
`test_case`) during `cargo doc` (`cfg(doc)` = `true`, `cfg(test)` = `false`).

When a test module parent is gated with `#[cfg(any(test, doc))]` so its documentation
pages and intra-doc links build in `cargo doc`, use a **Two-Layer Split**:

1. **Documentation & pure-`[dependencies]` layer (inherits `#[cfg(any(test, doc))]`)**:
    - Module shells (`pub(crate) mod integration_tests;`, `pub(crate) mod my_test;`,
      `pub(crate) mod test_fixtures;`) and their `//!` module-level rustdocs.
    - Fixture submodules that only depend on `std` or `[dependencies]` (e.g.,
      `pub(crate) mod constants;`, `pub(crate) mod jsonc_golden;`): do **not** add
      redundant `#[cfg(test)]` to them, so rustdocs can link directly to their symbols
      without leaking test constants into production modules.
2. **Test-execution & `[dev-dependencies]` layer (`#[cfg(test)]`)**:
    - Executable test blocks (`#[cfg(test)] mod tests { ... }`) and helper
      functions/submodules that import `[dev-dependencies]` (e.g.,
      `#[cfg(test)] pub(crate) mod http_server;` using `mockito`).
    - Always add a brief comment above `#[cfg(test)]` fixture submodules explaining _why_
      they are narrower than their `#[cfg(any(test, doc))]` parent (because
      `[dev-dependencies]` are not linked during `cargo doc`).
    - In `//!` rustdocs compiled under `cfg(doc)`, link to `[dev-dependencies]` crates via
      `https://docs.rs/<crate>` rather than crate-path intra-doc links.

```rust
// integration_tests/test_fixtures/mod.rs
// (parent `integration_tests` is already `#[cfg(any(test, doc))]`)

// Already non-production via parent `#[cfg(any(test, doc))]`.
// Kept visible under `cfg(doc)` so rustdocs can link to fixture constants/builders.
pub(crate) mod constants;
pub(crate) mod jsonc_golden;

// Additionally gated behind `#[cfg(test)]` because `mockito` is a `[dev-dependency]` and
// Cargo does not link `[dev-dependencies]` during `cargo doc` (`cfg(doc)`).
#[cfg(test)]
pub(crate) mod http_server;

pub use constants::*;
#[cfg(test)]
pub use http_server::*;
pub use jsonc_golden::*;
```

**Apply at nested levels**: Nested `#[cfg(...)]` attributes combine with logical AND, so
child test modules inside a `#[cfg(any(all(unix, doc), all(target_os = "linux", test)))]`
parent inherit that gate automatically (`pub(crate) mod pty_input_test;`). Production
modules in the `direct_to_ansi` style (such as
`#[cfg(target_os = "linux")] pub(crate) mod input;`) keep their production submodules
unconditional (`pub(crate) mod mio_poller;`) and gate only test or doc stub submodules
with single-arm `#[cfg(any(test, doc))]` (never re-exporting test modules into production
APIs with `pub use`).

### Step 4: Handle Transitive Crate-Internal Path Access

**Important:** If documentation in one module links to another module across the crate by
path (for example, `mod@crate::parent::diff_chunks`), both the target module and every
intermediate ancestor along that path must be at least `pub(crate) mod`.

```rust
// mod.rs (single-tier Crate-wide coordinator)

// Attach.
pub(crate) mod paint_impl; // Linked via crate::ofs_buf::paint_impl.
pub(crate) mod diff_chunks; // Also pub(crate) mod so external modules can link to it.

// Re-export.
pub use paint_impl::*;
pub use diff_chunks::*;
```

**Why:** Even with `--document-private-items`, Rust visibility rules govern path
resolution across module boundaries. A plain `mod diff_chunks;` is only accessible from
its parent module and descendants, whereas `pub(crate) mod diff_chunks;` makes the path
resolvable anywhere inside the crate without exposing it in the public API.

### Step 5: Reference in Rustdoc

When linking to crate-internal (`pub(crate)`) or private modules in documentation, use the
`mod@` prefix:

```rust
/// See [`internal_parser`] for implementation details.
///
/// [`internal_parser`]: mod@crate::internal_parser
```

See the `write-documentation` skill for complete details on rustdoc links.

### Step 6: Multi-Level Barrel Exports (Never Combine `pub mod` with `pub use *;`)

When rustdoc generates documentation, the search index includes every public item at its
**shortest public path**, while rustdoc only builds HTML pages at the canonical definition
path. Combining `pub mod intermediate;` with `pub use intermediate::*;` leaks internal
submodules into the public API and creates mismatched or redundant search paths that
previously motivated `#[doc(inline)]` shims.

**The root-cause fix:** Never declare barrel-exported implementation submodules as
`pub mod`:

- Keep barrel-exported submodules as **Folder-local** (`mod`) or **Crate-wide**
  (`pub(crate) mod` with a 1-line inline reason comment) and flatten consumers to import
  from the public facade (`use crate::Item;` or `use my_crate::Item;`).
- Reserve **Public (Inter-crate)** `pub mod` strictly for intentional API domain hubs and
  explicit non-globbed namespaces (never glob-re-exported into a parent).
- Do **not** add `#[doc(inline)]` re-export shims: with `--document-private-items`, every
  `mod` and `pub(crate) mod` module is already documented and searchable at its canonical
  path.

| Coordinator pattern | Action | Why |
| :--- | :--- | :--- |
| Barrel export (`pub use child::*;`) | Use `mod child;` or `pub(crate) mod child; // <reason>.` | Keeps internal paths out of public API; avoids broken search paths without `#[doc(inline)]` |
| Intentional namespace (`mouse_sgr`, domain hub) | Use `pub mod ns;` **without** `pub use ns::*;` | Canonical path is the only public path; zero search-index ambiguity |

```rust
// core/ansi/mod.rs

// Attach. Crate-wide (Intra-crate).
pub(crate) mod constants; // Cross-folder rustdoc module links (#design, mouse).

// Attach. Folder-local (Intra-module).
mod color;

// Re-export.
pub use color::*;
pub use constants::*;
```

### Step 7: Macro Module Organization

For modules that define `#[macro_export]` macros consumed within the same crate, use
`#[macro_use]` on the module declaration chain and order macro-defining modules BEFORE
sibling consumer modules. Put them in their own top block under
`// Attach. Upward #[macro_use] propagation (must precede modules below that invoke their macros).`
and keep them in textual macro-dependency order (never split a `#[macro_use]` block by
visibility). Do **NOT** import them with `use crate::macro_name;` or call them with
`crate::macro_name!()` / `$crate::macro_name!()` inside macro bodies, as this triggers the
`macro_expanded_macro_exports_accessed_by_absolute_paths` future-incompatibility lint
([rust-lang/rust#52234](https://github.com/rust-lang/rust/issues/52234)):

```rust
// mod.rs - Two-block coordinator split preserves textual macro scope.

// Attach. Upward #[macro_use] propagation (must precede modules below that invoke their macros).
#[macro_use]
mod my_domain_macros; // Must precede consumer_a/consumer_b even if visibilities mix.

// Attach.
pub mod consumer_a; // Uses my_macro!() directly without `use crate::my_macro;`.

// Re-export.
pub use consumer_a::*;

// consumer_a.rs
fn example() {
    my_macro!();
}

mod inner {
    // Textual macro scope propagates into child `mod` blocks automatically.
    fn helper() { my_macro!(); }
}
```

**Why:** In Rust, `#[macro_export]` makes a macro available to external crates (and
doctests) at the crate root (`use r3bl_tui::my_macro;`). Within the defining crate itself,
resolving `#[macro_export]` macros through `crate::` or `$crate::` triggers the
future-incompatibility lint when any `#[macro_export]` macro invocation is involved. Using
`#[macro_use]` up the module hierarchy avoids absolute crate-root resolution internally
while preserving `#[macro_export]` for external consumers.

**Cross-calls inside `macro_rules!` bodies:** Inline the helper type/method expansion
using `$crate::TypeName` (type and method paths do not trigger the lint) rather than
calling `$crate::helper_macro!()`.

**Minimal `#[macro_use]` Lifting (Lowest Common Ancestor):** Keep the macro module private
(`#[macro_use] mod ansi_const_macros;`) and propagate `#[macro_use]` up through parent
`mod.rs` files **only** as far as needed to reach the lowest common ancestor of all
internal callers. If every caller lives inside the same submodule (for example,
`constants/csi.rs` and `constants/esc.rs` calling `define_ansi_const!` inside
`constants/`), put `#[macro_use] mod ansi_const_macros;` in `constants/mod.rs` and do
**not** lift `#[macro_use]` onto `pub mod constants;` or higher ancestors.

**Keep `#[macro_export]` on Shared and Documented Macros (Option A):** Annotate all shared
and documented macros (even subsystem macros like `define_ansi_const!`) with
`#[macro_export]` so `cargo doc` generates `macro.<name>.html` from `///` docs, bare
[`my_macro!`] links resolve cleanly without `private_intra_doc_links`, and `no_run`
doctests compile. Only single-file boilerplate macros used solely inside their own `.rs`
file omit `#[macro_export]`.

### Step 8: File and Module Naming Conventions

Follow these conventions when naming files within modules:

#### 1. Inception Collision Avoidance (`<name>_struct.rs` / `<name>_enum.rs`)

When organizing code into subfolders, avoid creating a file whose base name is identical
to its parent directory. This avoids "inception" redundancy and naming confusion in paths:

```text
// ❌ Collision / Inception Anti-Pattern:
chunk_framer/
├── mod.rs
├── chunk_framer.rs       // Collision with parent directory name!
└── circuit_breaker.rs

// ✅ Disambiguated Pattern:
chunk_framer/
├── mod.rs
├── chunk_framer_struct.rs // Houses `pub struct ChunkFramer`
└── circuit_breaker/
    ├── mod.rs
    └── circuit_breaker_struct.rs
```

**Rules:**

- **Primary Struct**: When the colliding file houses the primary `struct` of that module,
  suffix it with `_struct.rs` (e.g., `accumulator_struct.rs`, `chunk_framer_struct.rs`,
  `circuit_breaker_struct.rs`, `engine_struct.rs`).
- **Primary Enum**: When the colliding file houses the primary `enum` of that module,
  suffix it with `_enum.rs` (e.g., `parser_enum.rs`).
- **Non-colliding files**: Files whose names differ from the parent directory keep their
  natural descriptive names **without** suffixes (e.g., `unparsed_buffer_action.rs`,
  `constants.rs`). Do NOT add `_struct` or `_enum` to non-colliding files.

#### 2. Standalone Functions Entry Point (`<xyz>_entry_point.rs`)

When a module has a file whose sole responsibility is providing top-level function entry
points (such as pure functions, pipeline decoders, or event dispatchers) without primary
structs or enums, name it `<xyz>_entry_point.rs`:

- `chunk_decoder_entry_point.rs` (houses `try_decode_input_event()`)
- `input_parser_entry_point.rs` (houses `try_parse_input_event()`)

#### 3. Public API Surface (`<xyz>_public_api.rs`)

When a module explicitly segregates its external, user-facing API surface from internal
implementation details, name the public facade `<xyz>_public_api.rs` (optionally paired
with `<xyz>_internal_api.rs`):

- `ansi_parser_public_api.rs`
- `engine_public_api.rs` (paired with `engine_internal_api.rs`)
- `scoped_mutex_public_api.rs`
- `log_public_api.rs`
- `input_device_public_api.rs`

#### 4. Macro Module Files (`<domain>_macros.rs`)

Never name a file generically `macros.rs`. Always use a domain-specific name (such as
`ansi_const_macros.rs`, `canvas_index_and_length_impl_macros.rs`, `crossterm_macros.rs`,
`editor_macros.rs`, `layout_macros.rs`, or `render_macros.rs`) so that:

- Module filenames remain unique across the crate without colliding under wildcard
  `pub use *;` barrel re-exports if any macro module is ever made visible in tests or
  docs.
- Single-file macro folders (such as a lone `mod.rs` paired with `macros.rs`) stay
  flattened into `<domain>.rs` (for example, `tui/src/core/decl_macros.rs`).

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
// No conflicts with other `constants` modules in the crate.

// Attach.
mod constants; // Folder-local (Intra-module): name hidden outside parent.

// Re-export.
pub use constants::*; // Items public.

// Elsewhere in the crate.
mod constants; // No conflict! This is in a different scope.
```

**⚠️ Warning: Re-export Collisions with `pub mod`**

While private module names don't collide, their **public contents** (including public
submodules) will collide if they share the same name and are pulled into the same parent
namespace via glob re-exports (`pub use module::*;`).

If multiple modules in the same hierarchy have submodules with identical names (e.g.,
several `pub mod integration_tests;`), glob re-exporting them into a single parent module
will cause **ambiguous re-export** errors, because the module names leak into the public
API.

**How to solve test module collisions:**

1. **The Default:** Keep test modules **Folder-local** (`#[cfg(test)] mod unit_tests;`).
   `cargo test` finds them automatically, and `--document-private-items` documents
   non-test-gated modules without exposing module names in the public API.
2. **The Rustdoc Exception:** If you *must* link to a test module across folders in your
   documentation using intra-doc links, keep it **Crate-wide** behind single-arm
   `#[cfg(any(test, doc))]` (`#[cfg(any(test, doc))] pub(crate) mod ...; // <reason>.`)
   under `--document-private-items` instead of `pub mod`. Always use a uniquely prefixed
   name (e.g., `pub(crate) mod vt_100_parser_integration_tests;`) to avoid collisions in
   internal or doc paths.

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

**Why:** The separation is meaningful to users. They WANT to know if they're using
frontend or backend APIs.

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
separate files (large file). Comment banners are informal and don't provide the same
benefits as actual module boundaries (scoped imports, clear boundaries, IDE navigation).

## Complete Examples

### Example 1: Simple Module Organization (Single-Tier `// Attach.`)

```rust
// src/terminal/mod.rs

#![rustfmt::skip]

// Attach.
mod position;
mod size;
mod style;
mod cursor;
mod buffer;

// Re-export.
pub use position::*;
pub use size::*;
pub use style::*;
pub use cursor::*;
pub use buffer::*;
```

Usage:

```rust
use terminal::{Buffer, Cursor, Position, Size, Style};
// Not: use terminal::position::Position;
```

### Example 2: Mixed `Crate-wide` and `Folder-local` Modules

```rust
// src/parser/mod.rs

// Attach. Crate-wide (Intra-crate).
pub(crate) mod vt_100; // Cross-folder rustdoc module link (renderer).

// Attach. Folder-local (Intra-module).
mod escape_sequences;

// Re-export.
pub use vt_100::*;
pub use escape_sequences::*;
```

Now both intra-folder sibling links (`mod`) and cross-folder cousin/ancestor links
(`pub(crate) mod`) resolve:

```rust
// src/parser/mod.rs or src/parser/vt_100.rs (parent or sibling can link to plain `mod`):
/// [`escape_sequences`]: mod@crate::parser::escape_sequences

// src/renderer/mod.rs (cousin folder needs `pub(crate) mod vt_100;` on target):
/// [`vt_100`]: mod@crate::parser::vt_100
```

### Example 3: Mixed `Public` and `Folder-local` Modules

```rust
// src/rendering/mod.rs

// Attach. Public (Inter-crate).
pub mod backends; // Different backend implementations (not glob-re-exported).
pub mod widgets;  // UI widgets (not glob-re-exported).

// Attach. Folder-local (Intra-module).
mod buffer;
mod diff_engine;

// Re-export.
pub use buffer::RenderBuffer; // This one is public; diff_engine stays internal.
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

### ❌ Mistake 2: Blocking Transitive Crate-Internal Path Access

```rust
// mod.rs: Bad!
pub(crate) mod a;  // External docs link to crate::parent::b

mod b;             // Parent-only private! Cross-module crate paths cannot see it.
```

**Problem:** Modules outside `parent` cannot resolve `mod@crate::parent::b` when `b` is
plain `mod b;`.

**Fix:**

```rust
pub(crate) mod a;
pub(crate) mod b;  // Also pub(crate) mod for crate-wide path resolution.
```

### ❌ Mistake 3: Using `#[cfg(any(test, doc))]` Shims on Production Modules

```rust
// mod.rs: Bad! Production module gated behind test/doc cfg (or paired with a negated-cfg fallback).
#[cfg(any(test, doc))]
pub mod utils;
```

**Problem:** Dual-arm `#[cfg(any(test, doc))]` / negated-`cfg` shims are obsolete because
`--document-private-items` documents private and `pub(crate)` modules directly, and a
single-arm `#[cfg(any(test, doc))]` gate on a production module omits it from release
builds.

**Fix:**

- Use `mod utils;` by default for parent-scoped and sibling-scoped production modules
  whose items are re-exported via `pub use`.
- Use `pub(crate) mod utils;` only when ancestor or cousin modules outside the parent
  folder need path access in code, tests, or `mod@crate::...` rustdoc links.
- Reserve single-arm `#[cfg(any(test, doc))] pub(crate) mod ...;` strictly for **test-only
  modules**.

### ❌ Mistake 4: Using `pub(crate) mod` Everywhere Instead of Default `mod`

```rust
// src/resilient_reactor_thread/mod.rs: Over-widened!
pub(crate) mod rrt_worker; // Only referenced by parent/siblings inside this folder.
```

**Problem:** Marking every submodule **Crate-wide** (`pub(crate) mod`) hides which
modules actually have cross-folder path dependencies from cousin modules, tests, or
architecture tables.

**Fix:** Default to **Folder-local** (`mod`) everywhere, and keep **Crate-wide**
(`pub(crate) mod`) with a 1-line inline reason comment only where
`cargo check --workspace --all-targets` or `./check.fish --quick-doc` requires
cross-folder path resolution:

```rust
// src/resilient_reactor_thread/mod.rs: Tight minimum scope!
mod rrt_worker; // Folder-local: parent-to-child and sibling links still resolve under --document-private-items.
```

## Reporting Results

After organizing modules:

- ✅ Organized successfully → "Module structure organized with private modules and public
  re-exports!"
- 🔧 Updated internal visibility → Report which production modules use `mod` vs
  `pub(crate) mod`, and which test-only modules use single-arm `#[cfg(any(test, doc))]`
- 📝 Manual review needed → List modules that may need public exposure for API reasons

## Supporting Files in This Skill

This skill includes additional reference material:

- **`examples.md`**: Complete, working examples of module organization for different
  scenarios (simple library with internal structure, `pub(crate) mod` visibility for
  documentation, large crate with domain separation, single-arm `#[cfg(any(test, doc))]`
  test-only modules, gradual refactoring strategy, avoiding naming conflicts,
  platform-specific docs, macro exports, and file/module naming conventions). Each example
  shows full file structure and implementation. **Read this when:**
    - Simple library module organization → Example 1
    - Need `pub(crate) mod` visibility for docs and cross-module links → Example 2
    - Large crate with multiple domains (graphics/audio/physics) → Example 3
    - Test utilities excluded from production builds (`#[cfg(any(test, doc))]`) → Example
      4
    - Refactoring from public modules to private + re-exports → Example 5
    - Avoiding module naming conflicts → Example 6
    - Cross-platform docs for platform-specific code → Example 7
    - Multi-level barrel exports without `#[doc(inline)]` → Example 8
    - Macro module exports (`#[macro_export]`) and file/module naming conventions
      (`_struct.rs`, `_entry_point.rs`, `_public_api.rs`) → Example 9 & Example 10
    - Decision tree for when to use which pattern → Quick Decision Tree

## Related Skills

- `write-documentation`: Documenting module organization and fixing intra-doc links (uses
  `pub(crate) mod` and `--document-private-items` for linking internal types)
- `run-clippy`: Ensures `mod.rs` follows module organization patterns

## Related Commands

No dedicated command, but used by:

- `/clippy`: Checks module organization as part of code quality
- `/fix-intradoc-links`: Uses `pub(crate) mod` and `mod@` intra-doc link patterns from
  this skill

## Related Agents

- `clippy-runner`: Invokes this skill to enforce module patterns
