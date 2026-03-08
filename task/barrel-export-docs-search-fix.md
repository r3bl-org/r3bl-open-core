# Fix Rustdoc Search Navigation for Multi-Level Barrel Exports

## Problem

When rustdoc generates documentation, the search index includes all public items and
modules. For multi-level barrel exports (`pub mod intermediate; pub use intermediate::*;`),
the search index resolves the "shortest public path" for items. But rustdoc only generates
HTML pages at the **canonical definition path**, not at the flattened re-export path.

This means pressing "s" in rustdoc and searching for e.g. "csi" produces a link to
`core/ansi/csi/index.html` — which doesn't exist. The actual page is at
`core/ansi/constants/csi/index.html`.

## Root Cause

`pub use intermediate::*;` re-exports **items** (types, consts, functions) but does not
create rustdoc **module pages** at the re-export site. The glob flattens items upward
without aliasing the submodule.

## Fix Strategy

Use `#[doc(inline)]` to re-export submodules at the parent level, but **only when** the
intermediate module is a well-documented organizational hub (has module-level `//!` docs,
organization tables, etc.) AND its submodules are `pub mod`.

### Decision Criteria

| Intermediate module characteristics          | Action                       | Why                                                     |
| :------------------------------------------- | :--------------------------- | :------------------------------------------------------ |
| Public, has module docs, organization tables | Add `#[doc(inline)]`        | Submodules are discoverable via search; pages must exist |
| Private with `pub use *;` only (pure barrel) | No action needed             | Submodules aren't in the search index at all             |
| Public but no module docs (structural only)  | No action needed             | Not worth the doc noise; users won't search for these   |

### Implementation Pattern

In the parent module's `mod.rs`, add explicit `#[doc(inline)]` re-exports alongside the
existing glob re-export:

```rust
// Existing: keeps flat item access working
pub use constants::*;

// New: creates rustdoc pages at the parent path for searchability
#[doc(inline)]
pub use constants::csi;
#[doc(inline)]
pub use constants::dsr;
// ... etc.
```

This creates pages at **both** paths (`ansi/csi/index.html` AND
`ansi/constants/csi/index.html`), so rustdoc search links work regardless of which
path the search index resolves.

## Part 1: Update organize-modules Skill

Add a new section to `.claude/skills/organize-modules/SKILL.md` covering:

- [ ] New section: "Multi-Level Barrel Exports and Rustdoc Search" after Step 5
  - Explain the search index vs HTML page generation discrepancy
  - The decision criteria table (documented hub → inline, pure barrel → skip)
  - Example showing `#[doc(inline)]` usage
  - Warning: only apply when intermediate has substantial module docs
- [ ] Add Example 8 to `examples.md`: "Multi-Level Barrel Export with Doc Inline"
  - Show before (broken search) and after (working search)
  - Include the conditional visibility variant

## Part 2: Codebase Audit — Modules Requiring `#[doc(inline)]`

### Needs fix (well-documented organizational hubs)

#### 1. `core::ansi` re-exporting `constants` submodules

- **File:** `tui/src/core/ansi/mod.rs` (line 276: `pub use constants::*;`)
- **Intermediate:** `constants/mod.rs` — 99 lines of module docs, organization table
- **Leaf modules to inline:** `csi`, `dsr`, `esc`, `generic`, `input_sequences`,
  `mouse`, `raw_mode`, `sgr`, `utf8`
- **Action:** Add 9 `#[doc(inline)] pub use constants::<leaf>;` lines

#### 2. `core::ansi::constants` is itself well-documented

The parent `ansi/mod.rs` already has `pub mod constants;` so `constants` itself shows
up correctly. The issue is its children (`csi`, etc.) not showing up under `ansi::`.

#### 3. `core::coordinates` re-exporting submodules

- **File:** `tui/src/core/coordinates/mod.rs` (lines 252-257: glob re-exports)
- **Intermediate docs:** 207 lines — 6-domain architecture diagram, type-safety rationale
- **Leaf modules to inline:** `buffer_coords`, `bounds_check`, `byte`, `primitives`,
  `percent_spec`, `vt_100_ansi_coords`
- **Action:** Add 6 `#[doc(inline)] pub use <leaf>;` lines
- **Note:** Some leaves (`bounds_check` with 1154 doc lines, `buffer_coords`) are
  themselves organizational hubs — check if their children need inlining too

#### 4. `core::pty` re-exporting submodules

- **File:** `tui/src/core/pty/mod.rs` (lines 336-338: glob re-exports)
- **Intermediate docs:** 323 lines — "Developer's Journey" narrative, 3-layer stack
- **Leaf modules to inline:** `pty_engine`, `pty_mux`, `pty_session`
- **Action:** Add 3 `#[doc(inline)] pub use <leaf>;` lines

#### 5. `core::graphemes` re-exporting submodules

- **File:** `tui/src/core/graphemes/mod.rs` (lines 261-264: glob re-exports)
- **Intermediate docs:** 250 lines — Unicode/grapheme explanations, index type tables
- **Leaf modules to inline:** `gc_string`, `traits`, `unicode_segment`, `word_boundaries`
- **Action:** Add 4 `#[doc(inline)] pub use <leaf>;` lines

#### 6. `tui::md_parser` re-exporting submodules

- **File:** `tui/src/tui/md_parser/mod.rs` (lines 251-258: glob re-exports)
- **Intermediate docs:** 228 lines — parsing diagrams, naming conventions, flowcharts
- **Leaf modules to inline:** `block`, `convert_to_plain_text`, `extended`, `fragment`,
  `md_parser_constants`, `md_parser_types`, `parse_markdown`, `single_line`
- **Action:** Add 8 `#[doc(inline)] pub use <leaf>;` lines

### Does NOT need fix (pure barrel or minimal docs)

These use the barrel export pattern correctly — intermediate modules are private or have
no substantial docs, so submodules don't appear in search:

- `core::common/mod.rs` — structural grouping, no org tables
- `readline_async::readline_async_impl` — structural grouping
- `tui::editor::editor_engine` / `editor_buffer` — structural, internal
- `core::test_fixtures::pty_test_fixtures` — test-only
- `core::ansi::vt_100_pty_output_parser::protocols` — deep nesting, not a discovery point

### Needs investigation

- `tui::terminal_lib_backends::direct_to_ansi::input::mio_poller` — 359 doc lines but
  4 levels deep. The question is whether anyone searches for its submodules. Likely no
  action needed due to nesting depth.
- `core::coordinates::bounds_check` — 1154 doc lines, itself has submodules. Check if
  this is a second-level case that also needs `#[doc(inline)]`.

## Part 3: Verification

After implementation, verify by:

1. Build docs: `./check.fish --quick-doc`
2. Open rustdoc in browser
3. Press "s" and search for "csi", "bounds_check", "pty_session", etc.
4. Confirm search result links navigate to valid pages
5. Confirm both paths work (e.g., `ansi/csi/` and `ansi/constants/csi/`)
