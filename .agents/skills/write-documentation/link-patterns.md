# Intra-doc Link Patterns and Examples

This document provides comprehensive examples of intra-doc link patterns for different
scenarios.

---

## Link Source Rubric

When deciding between local intra-doc links vs external URLs, follow this priority order:

| Priority | Source Type                                     | Link Style    | Example                                          |
| -------- | ----------------------------------------------- | ------------- | ------------------------------------------------ |
| 1        | Code in this monorepo                           | Local path    | `[`Foo`]: crate::module::Foo`                    |
| 2        | `[dependencies]` in Cargo.toml                  | Crate path    | `[`mio`]: mio`                                   |
| 3        | OS/CS/hardware terminology                      | External URL  | `[`epoll`]: https://manpages.debian.org/epoll.7` |
| 4        | Pedagogical/domain terms                        | Wikipedia URL | `[design pattern]: https://en.wikipedia.org/...` |
| 5        | Non-dependency or `[dev-dependencies]` crates\* | docs.rs URL   | `[`mockito`]: https://docs.rs/mockito`           |

\*Cargo does not link `[dev-dependencies]` during `cargo doc`, so crate-path intra-doc
links to `[dev-dependencies]` fail with broken-link warnings; use
`https://docs.rs/<crate>` instead.

**Key principle:** If it's in your Cargo.toml, use local links. This enables:

- **Offline docs** - works without internet
- **Version-matched** - links to the exact version you depend on
- **Rustdoc-validated** - broken links caught at build time

### Examples

```rust
// ✅ Good: mio is a dependency, use crate path
//! [`mio`]: mio

// ❌ Bad: Using docs.rs for a dependency
//! [`mio`]: https://docs.rs/mio

// ✅ Good: OS concept (not a Rust crate), use external URL
//! [`epoll`]: https://manpages.debian.org/epoll.7

// ✅ Good: Wikipedia for CS terminology
//! [Actor]: https://en.wikipedia.org/wiki/Actor_model
```

> [!IMPORTANT] **Link Checker Safety (`lychee`)**:
>
> - **Linux man pages**: Always use `https://manpages.debian.org/<name>.<sec>` (e.g.
>   `https://manpages.debian.org/epoll.7`). Never use `man7.org` (frequently times out or
>   drops connections for link checkers).
> - **GNU tools & utilities**: Always link to Wikipedia
>   (`https://en.wikipedia.org/wiki/...`), never `gnu.org` (drops automated link checker
>   traffic).

> **Note:** This rubric is also in `SKILL.md`. The redundancy is intentional -prioritizing
> reliable application over efficiency. SKILL.md content loads when the skill triggers
> (ensuring correct behavior during doc generation), while this file serves as detailed
> reference for auditing.

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

**Rule:** Always include `()` for methods and functions to distinguish them from fields
and types.

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

**Why risky:** If there's a `struct Parser` or `fn parser()`, rustdoc might resolve
incorrectly.

### ✅ Good: Full Path with mod@

```rust
/// Implementation details in [`vt_100_ansi_parser`].
///
/// [`vt_100_ansi_parser`]: mod@crate::tui::core::pty_mux::vt_100_ansi_parser
```

---

## Pattern 4: Links to Private Types and Crate-Internal Modules

Workspace doc builds enable `--document-private-items` across `.cargo/config.toml`
(`[build] rustdocflags`), `check_docs.fish` (`RUSTDOCFLAGS`), and crate `Cargo.toml`
(`[package.metadata.docs.rs]`), paired with
`[workspace.lints.rustdoc] private_intra_doc_links = "allow"` in the root `Cargo.toml`.

Because private items and modules are documented directly, **parent-to-child** links
(`mod.rs` to child), **sibling-to-sibling** links (same folder), and
**descendant-to-ancestor-sibling** links resolve with plain **`mod`** without dual-arm
`#[cfg(any(test, doc))]` shims. However, `--document-private-items` controls **page
generation** and `private_intra_doc_links = "allow"` only suppresses the public-to-private
warning; `rustdoc` still enforces `rustc`'s module privacy rules when resolving a path
from outside the module's folder:

```rust
pub mod a {
    mod secret {}            // ❌ `mod@crate::a::secret` from cousin `b` fails (`no item named secret in module a`)
    pub(crate) mod shared {} // ✅ `mod@crate::a::shared` from cousin `b` resolves; still hidden outside crate
}

/// Links to [`a::shared`].
///
/// [`a::shared`]: mod@crate::a::shared
pub mod b {}
```

Keep submodules as **Folder-local (Intra-module)** (`mod`) whenever only their parent
`mod.rs` or siblings in the same folder reference them. Upgrade to **Crate-wide
(Intra-crate)** (`pub(crate) mod`) with a 1-line inline reason comment **only** for the
two cases where `rustdoc` / `rustc` cross-folder privacy blocks path resolution (and
reserve **Public (Inter-crate)** `pub mod` strictly for domain hubs or non-globbed API
namespaces):

- **Cousin-to-cousin** links (`folder_y` linking to `mod@crate::folder_x::sub`).
- **Ancestor-to-grandchild** links (`lib.rs` or `grandparent/mod.rs` linking down into
  `mod@crate::grandparent::parent::child`, because parents cannot see children's private
  `mod`).

### Step 1: Use `Folder-local` (`mod`) or `Crate-wide` (`pub(crate) mod`)

```rust
// mod.rs (mixed-tier coordinator ordered wide-to-narrow)

// Attach. Crate-wide (Intra-crate).
pub(crate) mod internal_module; // Cross-folder rustdoc module link (other_domain).

// Attach. Folder-local (Intra-module).
mod debug;

// Re-export.
pub use debug::*;
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

// Attach.
pub(crate) mod vt_100_ansi_parser; // Cross-folder rustdoc module link (TerminalState).

// Re-export.
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

### Items Inside a Module: `pub(crate)` vs Barrel Re-Export

For **functions, structs, and other items** inside a module, keep visibility as tight as
possible:

- **Crate-internal only**: Keep items private (for links inside the same folder hierarchy)
  or mark them `pub(crate)` (for links from cousin/ancestor modules via `pub(crate) mod`).
  With `--document-private-items` and `private_intra_doc_links = "allow"`, intra-doc links
  resolve without widening visibility to `pub`.
- **Part of the public API**: Mark items `pub` inside a `mod` or `pub(crate) mod` and
  expose them through `pub use submodule::*;` in `mod.rs` (the barrel export gatekeeper),
  so un-re-exported `pub` items inside internal modules never leak into the crate's public
  API.

```rust
// rrt.rs: internal function, safe to keep pub(crate).
/// Runs the worker's poll loop until it returns [`Continuation::Stop`].
pub(crate) fn run_worker_loop<W, E>(...) { ... }
```

Intra-doc links resolve without re-exporting `run_worker_loop` in `mod.rs`:

```rust
//! Resources are cleaned up via [RAII] when [`run_worker_loop()`] returns.
//!
//! [`run_worker_loop()`]: run_worker_loop
//! [RAII]: https://en.wikipedia.org/wiki/Resource_acquisition_is_initialization
```

**When to use which approach:**

| Situation | Approach |
| :--- | :--- |
| Same-folder **module** (parent/sibling links only) | **Folder-local (Intra-module)**: `mod` (documented automatically) |
| Cross-folder **module** (cousin or ancestor-to-grandchild link) | **Crate-wide (Intra-crate)**: `pub(crate) mod // <reason>.` (no dual-arm shims) |
| Intentional API domain hub or explicit namespace | **Public (Inter-crate)**: `pub mod` (never glob-re-exported) |
| Internal **item** linked across modules | Private or `pub(crate)` (no need to make `pub` just for docs) |
| **Test-only** module linked in docs | Single-arm `#[cfg(any(test, doc))] pub(crate) mod ...; // <reason>.` |

---

## Pattern 5: Links to Test Functions

### Step 1: Make Test Function Visible to Docs

Use `#[cfg_attr(not(doc), test)]` (or `#[cfg_attr(not(doc), tokio::test)]` for async
tests) so `cargo doc` sees a regular function item while `cargo test` still executes it:

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

### Step 2: Include Test-Only Module in the Doc AST

Single-arm `#[cfg(any(test, doc))]` remains required on **test-only modules** because
`--document-private-items` does not enable `cfg(test)`. Without `doc` in the guard,
rustdoc omits the module from the AST and links to test functions fail; with
`pub(crate) mod`, sibling modules and docs across the crate can reference it while keeping
the module out of release binaries and the public API:

```rust
// mod.rs
#[cfg(any(test, doc))]
pub(crate) mod tests;
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

Use the `field@` disambiguator for compiler-checked links to struct fields. Without it,
rustdoc may resolve to a method of the same name or fail silently. With `field@`, the
compiler validates that the field exists - renames caught at build time.

### ✅ Good: field@ Disambiguator (Compiler-Checked)

```rust
/// Terminal buffer with cursor tracking.
///
/// The [`cursor`] field stores current position.
///
/// [`cursor`]: field@Self::cursor
pub struct Buffer {
    /// Current cursor position
    pub cursor: Position,
}
```

### ✅ Good: field@ from External Doc

```rust
/// Rendering uses [`Buffer::cursor`] for positioning.
///
/// [`Buffer::cursor`]: field@crate::Buffer::cursor
```

### ⚠️ Works but Not Compiler-Checked

```rust
/// The [`cursor`] field stores current position.
///
/// [`cursor`]: Self::cursor  // No field@ - may resolve to method if name collides
```

**Why risky:** Without `field@`, if a method `cursor()` is added later, the link silently
changes target. With `field@`, the intent is explicit and validated.

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

````rust
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
````

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
- `field@` - Struct field
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

### Using super:: Paths (For Co-located Items with Long crate:: Paths)

`super::` paths are valid in intra-doc links. Prefer `crate::` paths since they tend to be
shorter, but use `super::` when the `crate::` path would be excessively long and the
target symbol is co-located (in the same parent module).

```rust
// ✅ Good: crate:: is short and clear
/// [`ReadlineAsyncContext`]: crate::ReadlineAsyncContext

// ✅ Good: super:: avoids a very long crate:: path for a co-located symbol
/// [`apply_event_to_line_state_and_render()`]:
///     super::readline_internal::apply_event_to_line_state_and_render()

// ❌ Unnecessary: super:: when crate:: is just as short
/// [`Foo`]: super::Foo  // crate::Foo would be shorter
```

**When to use `super::`:**

- The `crate::` path has 3+ segments and the target is in a sibling or parent module
- The symbol is co-located with the file where the link is being added

**When to stick with `crate::`:**

- The path is short (1-2 segments after `crate::`)
- The symbol is re-exported at the crate root
- Clarity matters more than brevity

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

You can link directly to `## Section Heading` in module documentation using fragment
identifiers.

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

**Why bad:** Combining `mod@` + `super::path` (with `::`) + `#fragment` can cause rustdoc
to enter an infinite loop during documentation generation.

### ❌ Bad: super:: Path without mod@ (Also Problematic)

```rust
/// See the [`utf8` encoding] section for details.
///
/// [`utf8` encoding]: super::utf8#utf-8-encoding-explained  // ⚠️ May also fail
```

### Rule Summary

| Pattern                          | Works? | Example                            |
| -------------------------------- | ------ | ---------------------------------- |
| `mod@super#fragment`             | ✅     | `mod@super#testing-strategy`       |
| `mod@crate::full::path#fragment` | ✅     | `mod@crate::core::parser#overview` |
| `mod@super::sibling#fragment`    | ❌     | Causes rustdoc recursion           |
| `super::sibling#fragment`        | ❌     | May cause issues                   |

**Best Practice:** When linking to a section in another module, always use the full
`crate::` path.

### ⚠️ Rustfmt Comment-Wrapping Bug with Fragment Anchor Links

When `wrap_comments = true` is configured in `rustfmt.toml` (with a specific
`comment_width`, e.g., `90`), any doc comment line that exceeds the width limit will be
wrapped by `rustfmt`.

While `rustfmt` is smart enough to avoid wrapping standard HTTP/HTTPS URLs, **it does not
natively recognize Rust intra-doc paths (or relative paths) as URLs.**

If you have a reference definition pointing to an intra-doc path with a fragment (e.g.,
`#section-name`), and the total line length exceeds the comment width:

1. `rustfmt` will force-wrap the line, breaking the path right after the `#` character.
2. It then treats the subsequent line in the doc comment as part of the same paragraph,
   wrapping them together and completely corrupting the reference link block.

#### ❌ Bad: Long Relative/Absolute Path triggers `rustfmt` wrapping

```rust
/// [How this affects stdout as well]:
///     super::handler_stdin::consume_stdin_input_with_sender#how-this-affects-stdout-as-well
```

Because the second line (indented URL) is 93 characters (exceeding 90), `rustfmt` wraps
it, corrupting it into:

```rust
/// [How this affects stdout as well]:
///     super::handler_stdin::consume_stdin_input_with_sender#
/// how-this-affects-stdout-as-well [Why We Need Non-Blocking Read]:
```

#### ✅ Clever Workaround: Import the Symbol and Shorten the Link

To prevent `rustfmt` from wrapping, keep the line length under the `comment_width` limit.
The cleanest way to do this is to **import the target symbol into the current module** so
that you can reference it directly by name instead of using a long relative or absolute
path.

To avoid compiler warnings about unused imports, always:

1. **Separate** the doc-only import from the main, nested `use` group of the file.
2. Tag it with `#[allow(unused_imports)]` and add an explanatory comment.

3. **Import the symbol:**

    ```rust
    // Imported specifically for the intra-doc links in the struct documentation.
    #[allow(unused_imports)]
    use super::handler_stdin::consume_stdin_input_with_sender;
    ```

4. **Use the shorter direct reference:**
    ```rust
    /// [How this affects stdout as well]:
    ///     consume_stdin_input_with_sender#how-this-affects-stdout-as-well
    ```
    _Line length is now only 71 characters, safely below the 90-character wrapping
    threshold!_

### Heading Renames Break Fragment Links Silently

Rustdoc slugifies heading text into a heading anchor (e.g., `## My Section` becomes
`id="my-section"`). When you rename a heading, the slugified heading anchor changes - and
any reference-style link definitions targeting the old anchor will **silently break**.
Unlike symbol links (which `cargo doc` validates at build time), fragment links to heading
anchors are not validated by rustdoc. The link renders as clickable text but navigates
nowhere.

```rust
// Heading was: ## Understanding "Blocking I/O"
// Anchor was:  #understanding-blocking-io
//
// Heading renamed to: ## Example: Terminal Input Across Operating Systems
// Anchor is now:      #example-terminal-input-across-operating-systems

// ❌ Stale: Still points to old heading anchor (silently broken, no build error)
//! [blocks on I/O]: #understanding-blocking-io

// ✅ Fixed: Updated to match new heading anchor
//! [blocks on I/O]: #example-terminal-input-across-operating-systems
```

**After renaming any heading**, search the file for fragment links (`#old-anchor-text`)
and update them to match the new heading anchor. Anchor generation rules:

- Lowercase everything
- Replace spaces with hyphens
- Strip punctuation (quotes, colons, parentheses)
- Collapse consecutive hyphens

---

## Pattern 16: Linking to Trait Impl Sections

Rustdoc generates HTML anchors for each trait implementation on a type's page. You can
link to these using the `#impl-TraitName-for-TypeName` fragment.

### Anchor Format

The anchor follows the pattern: `#impl-{TraitName}-for-{TypeName}`

Examples:

- `impl Default for RestartPolicy` -> `#impl-Default-for-RestartPolicy`
- `impl Display for MyType` -> `#impl-Display-for-MyType`
- `impl From<String> for MyType` -> `#impl-From<String>-for-MyType`
- `impl From<&SharedWakerSlot<K>> for WakerSlotReader<K>` ->
  `#impl-From<%26Arc<Mutex<Option<K>>>>-for-WakerSlotReader<K>`

### URL-Encoding Rules

Only `&` needs URL-encoding (`%26`) in anchor fragments. Angle brackets `<>` can stay
as-is. Rustdoc expands type aliases to their underlying types in anchor IDs (e.g.,
`SharedWakerSlot<K>` becomes `Arc<Mutex<Option<K>>>`).

| Character    | Encode?     | Example                                              |
| :----------- | :---------- | :--------------------------------------------------- |
| `<` `>`      | No          | `From<String>` stays as-is                           |
| `&`          | Yes → `%26` | `&Arc` becomes `%26Arc`                              |
| Type aliases | Expanded    | `SharedWakerSlot<K>` becomes `Arc<Mutex<Option<K>>>` |

### Reference-Style (Preferred)

```rust
/// Override [`restart_policy()`] to customize behavior - if you
/// don't want to use the [default policy].
///
/// [default policy]: RestartPolicy#impl-Default-for-RestartPolicy
/// [`restart_policy()`]: Self::restart_policy
```

### Inline Style (Also Works)

```rust
/// The [default policy](RestartPolicy#impl-Default-for-RestartPolicy) allows 3
/// restarts with 100ms initial delay.
```

### Linking to a Method Within a Trait Impl

To link directly to a specific method (not just the impl block), use
`#method.method_name`:

```rust
/// Its [Drop implementation] calls
/// [`RRTWaker::wake_and_unblock_dedicated_thread()`].
///
/// [Drop implementation]: WakeOnDrop#method.drop
```

This is more precise than `#impl-Drop-for-WakeOnDrop<K>`, which anchors to the impl block
header. `#method.drop` scrolls directly to the `fn drop` method and its rustdoc comments.

**Three equivalent path forms** - all resolve to the same anchor on the type's page:

```rust
/// [`Drop`]: Self#method.drop              // Self resolves to current type
/// [`Drop`]: PtyTestWatchdog#method.drop   // Explicit type name
/// [`Drop`]: #method.drop                  // Same-page fragment (shortest)
```

| Form                   | When to use                                                      |
| :--------------------- | :--------------------------------------------------------------- |
| `Self#method.drop`     | In the type's own doc comment (idiomatic)                        |
| `TypeName#method.drop` | From another type's or module's doc comment                      |
| `#method.drop`         | Same-page shorthand (most concise, works on the type's own page) |

**When to use which:**

| Scenario                                                               | Anchor                 | Example                                        |
| :--------------------------------------------------------------------- | :--------------------- | :--------------------------------------------- |
| Linking to a specific method's docs                                    | `#method.method_name`  | `WakeOnDrop#method.drop`                       |
| Linking to the impl as a whole (e.g., default values, derive behavior) | `#impl-Trait-for-Type` | `RestartPolicy#impl-Default-for-RestartPolicy` |
| Trait has only one method and you want its docs                        | `#method.method_name`  | `TerminationGuard#method.drop`                 |
| Trait has multiple methods and you want the overview                   | `#impl-Trait-for-Type` | `MyType#impl-Iterator-for-MyType`              |

### Key Differences from Heading Anchors

| Aspect                        | Heading anchors (`#my-heading`)              | Impl anchors (`#impl-Trait-for-Type`)   |
| :---------------------------- | :------------------------------------------- | :-------------------------------------- |
| Source                        | `## My Heading` in `//!` docs                | Auto-generated by rustdoc HTML renderer |
| Validated at build time?      | No (silent breakage)                         | No (silent breakage)                    |
| Works with `mod@super::path`? | Yes (for module headings)                    | N/A (lives on type pages)               |
| Path syntax                   | `mod@super#heading` or `crate::path#heading` | `TypeName#impl-Trait-for-TypeName`      |

**Note:** These anchors are NOT validated by `cargo doc` - if you rename the type or
trait, the link breaks silently (same caveat as heading anchors).

---

## Pattern 17: Linking to Declarative Macros

Shared and documented `macro_rules!` macros carry `#[macro_export]` (so `rustdoc`
generates `macro.<name>.html`) and propagate internally via `#[macro_use]` from a private
`<domain>_macros` module. How you link to them depends on where the doc comment sits
relative to `#[macro_use]`:

### Inside Consumer Files / Below `#[macro_use]` (Bare Link)

For item docs (`///`) in sibling or descendant modules below `#[macro_use]`, write the
backticked macro link directly **without** a link reference definition:

```rust
/// Uses [`define_ansi_const!`] to build sequences at compile time.
pub const ESC_STR: &str = "\x1b";
```

**Why:** `#[macro_use]` puts `define_ansi_const!` into lexical scope, and
`#[macro_export]` makes its target public at crate root, so `rustdoc` resolves it
automatically. Do **not** add `crate::define_ansi_const` when `#[macro_use]` already
imports it.

### In Module Docs Above `#[macro_use]` (Explicit `crate::` Ref)

For module-level `//!` docs at the top of the coordinator `mod.rs` (before
`#[macro_use] mod <domain>_macros;`), add a crate-root link definition at the bottom of
the `//!` block:

```rust
//! Overview of [`define_ansi_const!`] sequences.
//!
//! [`define_ansi_const!`]: crate::define_ansi_const

#[macro_use]
mod ansi_const_macros;
```

**Why:** `//!` module docs sit textually above `#[macro_use] mod ...;`, so the macro is
not in textual scope at that point in the file. (Do **not** write ``[`crate::my_macro!`]``
in prose, and intra-doc link definitions never trigger
`macro_expanded_macro_exports_accessed_by_absolute_paths`.)

---

## Checklist for Perfect Intra-doc Links

When fixing links, ensure:

- [ ] All symbols in backticks are links: `` `Symbol` `` → ``[`Symbol`]``
- [ ] All links use reference-style, not inline
- [ ] All link definitions are at the bottom of the comment block
- [ ] Methods and functions include parentheses: `fn()`
- [ ] Field links use `field@` prefix for compiler-checked references
- [ ] Module links use `mod@` prefix when ambiguous
- [ ] Cross-module private targets use `pub(crate) mod` (no dual-arm
      `#[cfg(any(test, doc))]` production shims)
- [ ] Test functions use `#[cfg_attr(not(doc), test)]` pattern
- [ ] Absolute paths start with `crate::` or `std::`
- [ ] `./check.fish --quick-doc` shows zero warnings
- [ ] Links enable IDE "go to definition"
- [ ] No trailing whitespace on blank doc lines (`/// ` → `///`)

---

## Troubleshooting: Mysterious Link Failures

When reference links fail despite correct syntax, check these hidden causes:

### Angle Brackets in Link Reference Text

**Symptom:** All reference links after a certain link definition fail with "unresolved
link" warnings, but the links themselves look completely correct. `cargo doc` does not
warn about the actual root cause.

```rust
// ❌ BROKEN - angle brackets in the definition name scrambles the parser
/// [`Box<[T]>`]: std::boxed::Box  ← parser chokes here!
/// [`Foo`]: crate::Foo            ← silently ignored/broken
/// [`Bar`]: crate::Bar            ← silently ignored/broken
```

```rust
// ✅ FIXED - remove generic angle brackets from the link reference text
/// [`Box`]: std::boxed::Box       ← parser succeeds
/// [`Foo`]: crate::Foo            ← works
/// [`Bar`]: crate::Bar            ← works
```

**Why:** If you include generic angle brackets (`<`, `>`) inside the definition text (e.g.
`/// [`Box<[T]>`]: ...`), the rustdoc Markdown parser becomes scrambled. Not only does
that specific link fail, but rustdoc silently gives up on parsing **all subsequent
reference links** in that block, causing a cascade of confusing "unresolved link" warnings
that `cargo doc` blames on the other correctly-written links!

**How to detect:** Look for any `<` or `>` characters inside the square brackets of your
link definitions at the bottom of the comment block.

**Solution:** Remove the generic parameters from the link text (e.g., use ``[`Box`]``
instead of ``[`Box<[T]>`]``). If you must display the generics in the inline text, place
them outside the link brackets.

### ✅ Excellent Real-World Example

From `tui/src/core/common/flat_2d_array.rs`:

**Inline Text (Generics Outside Brackets):**

```rust
/// This array uses a [`Box`]`<[T]>` instead of a [`Vec`]`<T>`. This is a deliberate
/// design choice to make illegal states unrepresentable. A [`Vec`]`<T>` contains a
/// capacity field and allows methods like [`Vec::push`] or [`Vec::pop`] which could
/// silently change the length of the 1D buffer so it no longer perfectly matches `width *
/// height`, scrambling the 2D grid mapping.
///
/// By converting the [`Vec`]`<T>` into a [`Box`]`<[T]>` (a wide pointer) upon
/// initialization, we type-erase its ability to grow or shrink. [`Box`]`<[T]>` only
/// contains the wide pointer:
/// - the start address of the memory allocation, and the length,
/// - but not the capacity (which [`Vec`]`<T>` stores).
```

**Reference Definitions (Clean Link Names):**

```rust
/// [`Box`]: std::boxed::Box
/// [`Vec`]: std::vec::Vec
```

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

**Why:** Rustdoc's reference link parser is whitespace-sensitive. A `/// ` line (with
trailing space) breaks the parsing of the entire reference definition block that follows.

**How to detect:** Run `grep -n '/// $' src/**/*.rs` to find lines ending with `/// `
(space).

**Editor tip:** Configure your editor to strip trailing whitespace on save, but be aware
that some doc comments intentionally use trailing whitespace for formatting (rare).

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

**Linking correctly (prefer explicit disambiguators):**

```rust
/// - [`is_running`]: The field
/// - [`is_running()`]: The method
///
/// [`is_running`]: field@Self::is_running   ← explicit field target
/// [`is_running()`]: Self::is_running()     ← resolves to method (note the () in target too)
```

Using `field@` makes the intent explicit and ensures the link won't silently change target
if the field or method is renamed.

---

## Quick Reference

| Link To                             | Pattern Example                                    |
| ----------------------------------- | -------------------------------------------------- |
| Struct                              | ``[`Foo`]: crate::Foo``                            |
| Enum                                | ``[`Bar`]: crate::Bar``                            |
| Trait                               | ``[`Display`]: std::fmt::Display``                 |
| Function                            | ``[`process()`]: crate::process``                  |
| Method                              | ``[`run()`]: Self::run``                           |
| Field                               | ``[`field`]: field@Self::field``                   |
| Module                              | ``[`parser`]: mod@crate::parser``                  |
| Re-export                           | ``[`Type`]: crate::Type``                          |
| Test function                       | ``[`test_foo`]: crate::tests::test_foo``           |
| Private type (`pub(crate) mod`)     | ``[`Internal`]: struct@crate::internal::Internal`` |
| Section heading (fragment)          | ``[`docs`]: mod@crate::path::module#section-name`` |
| Trait impl section                  | `[default]: TypeName#impl-TraitName-for-TypeName`  |
| Macro (in `#[macro_use]` scope)     | ``[`my_macro!`]`` (no ref def needed)              |
| Macro (above `#[macro_use]`, `//!`) | ``[`my_macro!`]: crate::my_macro``                 |

All links go **at the bottom** of the rustdoc comment block!
