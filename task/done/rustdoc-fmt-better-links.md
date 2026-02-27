# Plan: Auto-Link and Backtick Known Terms in `cargo rustdoc-fmt`

<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

- [Overview](#overview)
  - [Problem](#problem)
  - [Solution](#solution)
  - [Two-Tier Linking Convention](#two-tier-linking-convention)
    - [Tier 1: Terms with internal types (link to crate path)](#tier-1-terms-with-internal-types-link-to-crate-path)
    - [Tier 2: Terms without internal types (link to external URL)](#tier-2-terms-without-internal-types-link-to-external-url)
  - [Design Decisions](#design-decisions)
    - [Resolved decisions](#resolved-decisions)
    - [Two-phase approach](#two-phase-approach)
    - [What "upgrade" means](#what-upgrade-means)
    - [What the linker does NOT do](#what-the-linker-does-not-do)
    - [Seed file structure](#seed-file-structure)
    - [Integration with existing pipeline](#integration-with-existing-pipeline)
- [Implementation Plan](#implementation-plan)
  - [Step 0: Create `known_terms.jsonc` seed file](#step-0-create-known_termsjsonc-seed-file)
  - [Step 1: Create `term_registry.rs` - the registry builder](#step-1-create-term_registryrs---the-registry-builder)
    - [Key types](#key-types)
    - [Step 1.0: Implement JSONC seed file loading](#step-10-implement-jsonc-seed-file-loading)
    - [Step 1.1: Implement workspace scanner](#step-11-implement-workspace-scanner)
    - [Step 1.2: Implement merge logic](#step-12-implement-merge-logic)
  - [Step 2: Create `term_linker.rs` - the doc block fixer](#step-2-create-term_linkerrs---the-doc-block-fixer)
    - [Key function](#key-function)
    - [Step 2.0: Implement term detection](#step-20-implement-term-detection)
    - [Step 2.1: Implement term upgrading](#step-21-implement-term-upgrading)
    - [Step 2.2: Implement target definition management](#step-22-implement-target-definition-management)
    - [Step 2.3: Handle edge cases](#step-23-handle-edge-cases)
  - [Step 3: Add `link_terms` to `FormatOptions`](#step-3-add-link_terms-to-formatoptions)
  - [Step 4: Add `--terms-only` and `--terms-file` CLI flags](#step-4-add---terms-only-and---terms-file-cli-flags)
  - [Step 5: Wire into `processor.rs` pipeline](#step-5-wire-into-processorrs-pipeline)
    - [Step 5.0: Build `TermRegistry` once at startup](#step-50-build-termregistry-once-at-startup)
    - [Step 5.1: Apply term linking per doc block](#step-51-apply-term-linking-per-doc-block)
  - [Step 6: Register modules in `mod.rs`](#step-6-register-modules-in-modrs)
  - [Step 7: Add dependencies](#step-7-add-dependencies)
  - [Step 8: Write tests](#step-8-write-tests)
    - [Step 8.0: Unit tests for `term_registry.rs`](#step-80-unit-tests-for-term_registryrs)
    - [Step 8.1: Unit tests for `term_linker.rs`](#step-81-unit-tests-for-term_linkerrs)
    - [Step 8.2: Integration test fixtures](#step-82-integration-test-fixtures)
    - [Step 8.3: Test in `complete_file_tests.rs`](#step-83-test-in-complete_file_testsrs)
  - [Step 9: Build, test, install](#step-9-build-test-install)
  - [Step 10: Run across codebase](#step-10-run-across-codebase)
  - [Step 11: Add external spec links to internal types](#step-11-add-external-spec-links-to-internal-types)
  - [Step 12: Final verification](#step-12-final-verification)
- [Files Modified (summary)](#files-modified-summary)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

# Overview

## Problem

Across the workspace (especially `tui/src/core/ansi/` and its submodules), many rustdoc comments
reference technical terms like `CSI`, `DSR`, `ESC`, `SGR`, `ANSI`, `ASCII`, `VT-100`, etc. These
terms should be:

1. Backticked (`` `CSI` ``)
2. Linked (``[`CSI`]``)
3. Have a correct link target definition at the bottom of the doc block
   (``[`CSI`]: crate::CsiSequence`` or ``[`ANSI`]: https://en.wikipedia.org/wiki/...``)

Currently, this is done manually - which is tedious, error-prone, and inconsistent. The same term
may be fully linked in one file but plain text in another. Common problems:

| Problem                                                | Example                                              |
| :----------------------------------------------------- | :--------------------------------------------------- |
| Term is plain text, missing backticks and link         | `CSI sequence`                                       |
| Term is backticked but not linked                      | `` `CSI` sequence ``                                 |
| Term is backticked and linked but no target definition | ``[`CSI`] sequence`` (no ``[`CSI`]: url`` at bottom) |
| Term has wrong or outdated link target                 | ``[`CSI`]: https://wrong-url.com``                   |

## Solution

Add a new transformation to `cargo rustdoc-fmt` that uses a **known-terms registry** (a hashmap of
term -> link target) to automatically:

1. **Scan** all `.rs` files in the workspace to discover existing backticked+linked terms and their
   targets (building the registry from real usage)
2. **Fix** doc blocks by adding missing backticks, links, and link target definitions for known
   terms

This approach is:

- **Data-driven**: The registry is built from actual codebase usage, not hardcoded
- **Incremental**: New terms are discovered as they're added to any file
- **Consistent**: The same term always gets the same link target across all files
- **Safe**: Only adds links for terms already established in the codebase

## Two-Tier Linking Convention

A key design decision: terms that have **internal Rust types** link to the internal type (intra-doc
link), while terms that are **pure external concepts** link to external URLs. This creates a useful
navigation chain: doc comment -> internal type -> external spec.

### Tier 1: Terms with internal types (link to crate path)

These terms have corresponding Rust types in the codebase. The term links to the internal type, and
the internal type's own rustdoc contains the external spec link.

| Term  | Link Target (intra-doc) | Internal Type        | Defined In                                                           |
| :---- | :---------------------- | :------------------- | :------------------------------------------------------------------- |
| `CSI` | `crate::CsiSequence`    | `CsiSequence` (enum) | `core/ansi/vt_100_pty_output_parser/protocols/csi_codes/sequence.rs` |
| `SGR` | `crate::SgrCode`        | `SgrCode` (enum)     | `core/ansi/generator/sgr_code.rs`                                    |
| `ESC` | `crate::EscSequence`    | `EscSequence` (enum) | `core/ansi/generator/esc_sequence.rs`                                |
| `DSR` | `crate::DsrSequence`    | `DsrSequence` (enum) | `core/ansi/generator/dsr_sequence.rs`                                |
| `OSC` | `crate::OscSequence`    | `OscSequence` (enum) | `core/osc/osc_codes.rs`                                              |

**Example in a doc comment:**

```rust
/// Parses [`CSI`] sequences from the input stream.
///
/// [`CSI`]: crate::CsiSequence
```

**Example on the internal type itself** (added separately, not by this tool):

```rust
/// Control Sequence Introducer - the most common ANSI escape sequence type.
///
/// See the [`CSI specification`] for the full standard.
///
/// [`CSI specification`]: https://en.wikipedia.org/wiki/ANSI_escape_code#CSI_(Control_Sequence_Introducer)_sequences
pub enum CsiSequence { ... }
```

### Tier 2: Terms without internal types (link to external URL)

These terms are pure external concepts with no corresponding Rust type. They link directly to an
external URL.

| Term                   | Link Target (external URL)                                                         |
| :--------------------- | :--------------------------------------------------------------------------------- |
| `ANSI`                 | https://en.wikipedia.org/wiki/ANSI_escape_code                                     |
| `ASCII`                | https://en.wikipedia.org/wiki/ASCII                                                |
| `UTF-8`                | https://en.wikipedia.org/wiki/UTF-8                                                |
| `DCS`                  | https://vt100.net/docs/vt510-rm/chapter4.html#S4.3.4                               |
| `VT-100`               | https://vt100.net/docs/vt100-ug/chapter3.html                                      |
| `VT-100 spec`          | https://vt100.net/docs/vt100-ug/chapter3.html                                      |
| `VT-100 specification` | https://vt100.net/docs/vt100-ug/chapter3.html                                      |
| `VT-220`               | https://en.wikipedia.org/wiki/VT220                                                |
| `DECSTBM`              | https://vt100.net/docs/vt510-rm/DECSTBM.html                                       |
| `DEC`                  | https://en.wikipedia.org/wiki/Digital_Equipment_Corporation                        |
| `xterm`                | https://en.wikipedia.org/wiki/Xterm                                                |
| `Alacritty`            | https://alacritty.org/                                                             |
| `Kitty`                | https://sw.kovidgoyal.net/kitty/                                                   |
| `RXVT`                 | https://en.wikipedia.org/wiki/Rxvt                                                 |
| `rxvt-unicode`         | https://en.wikipedia.org/wiki/Rxvt-unicode                                         |
| `urxvt`                | https://en.wikipedia.org/wiki/Rxvt-unicode                                         |
| `ConPTY`               | https://learn.microsoft.com/en-us/windows/console/creating-a-pseudoconsole-session |
| `gnome-terminal`       | https://en.wikipedia.org/wiki/GNOME_Terminal                                       |
| `ReGIS`                | https://en.wikipedia.org/wiki/ReGIS                                                |
| `Sixel`                | https://en.wikipedia.org/wiki/Sixel                                                |
| `X10`                  | https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h2-Mouse-Tracking          |
| `ITU-T Rec. T.416`     | https://www.itu.int/rec/T-REC-T.416-199303-I                                       |
| `vte`                  | https://docs.rs/vte                                                                |

## Design Decisions

### Resolved decisions

1. **All-occurrences linking**: Every occurrence of a known term in a doc block gets linked (not
   just the first). This keeps each doc block self-contained - a reader can click any occurrence.

2. **Auto-backtick plain text terms**: Yes, the tool automatically upgrades plain text `CSI` to
   ``[`CSI`]``. In this codebase, all the seed terms are unambiguous technical terms (no false
   positive risk for terms like `CSI`, `SGR`, `ANSI` in a terminal emulator codebase). The seed file
   is curated - only terms that are safe to auto-backtick go in it.

3. **Seed file format**: JSON5 via the `json5` crate (v1.3, serde-native). Supports `//` and `/* */`
   comments, trailing commas. Well-maintained (226 stars, last release Dec 2025).

4. **Seed file is canonical**: The seed file always wins over discovered link targets. The workspace
   scan only contributes terms that the seed file doesn't already define. No need for per-file
   authority overrides - if a URL needs to change, update the seed file. The seed file is in git, so
   changes are tracked and reviewable.

5. **Seed file baked into binary**: The seed file is embedded at compile time via `include_str!`.
   This means no runtime file-not-found errors, no path resolution, and the binary is fully
   self-contained. An optional `--terms-file <path>` CLI flag allows overriding with an external
   file for testing new terms without recompiling. Changes to the embedded seed file require
   `cargo install --path build-infra --force` (the standard workflow).

### Two-phase approach

1. **Phase 1 - Registry builder**: Load the seed file, then scan the workspace for additional
   ``[`Term`]: url`` reference definitions. Build a single `HashMap<String, String>` (term -> link
   target). Seed entries take priority over discovered entries.

2. **Phase 2 - Term linker**: For each doc block, find occurrences of known terms and upgrade them.
   Add missing link target definitions at the bottom of the block.

### What "upgrade" means

The linker applies these fixes to **every** occurrence of a known term:

| Current state                   | Action                                                       |
| :------------------------------ | :----------------------------------------------------------- |
| Plain `CSI`                     | Wrap in backtick+link: ``[`CSI`]`` and add target definition |
| `` `CSI` `` (backticked only)   | Wrap in link: ``[`CSI`]`` and add target definition          |
| ``[`CSI`]`` (linked, no target) | Add target definition: ``[`CSI`]: <target>``                 |
| ``[`CSI`]: wrong-target``       | Replace with canonical target from registry                  |

### What the linker does NOT do

- Does not touch existing intra-doc links (links containing `crate::`, `Self::`, `std::`, etc.)
- Does not touch content inside code fences or inline code spans
- Does not add links inside headings (headings should stay clean)
- Does not create new terms - only links terms that already exist in the registry
- Does not modify terms that appear as substrings of longer words (whole-word match only)
- Does not add external spec links to internal types (that's a separate manual step)

### Seed file structure

The seed file uses JSONC format with a `"tier"` field to distinguish internal vs external link
targets.

Proposed location: `build-infra/src/cargo_rustdoc_fmt/known_terms.jsonc`

```jsonc
{
  // Known terms registry - canonical link targets for technical terms in rustdoc comments.
  // The tool also discovers terms by scanning existing link targets in the workspace.
  //
  // "tier": "internal" = links to a crate type (intra-doc link)
  // "tier": "external" = links to an external URL

  // Tier 1: Terms with internal Rust types
  "CSI": { "target": "crate::CsiSequence", "tier": "internal" },
  "SGR": { "target": "crate::SgrCode", "tier": "internal" },
  "ESC": { "target": "crate::EscSequence", "tier": "internal" },
  "DSR": { "target": "crate::DsrSequence", "tier": "internal" },
  "OSC": { "target": "crate::OscSequence", "tier": "internal" },

  // Tier 2: Pure external concepts (no internal type)
  "ANSI": { "target": "https://en.wikipedia.org/wiki/ANSI_escape_code", "tier": "external" },
  "ASCII": { "target": "https://en.wikipedia.org/wiki/ASCII", "tier": "external" },
  "UTF-8": { "target": "https://en.wikipedia.org/wiki/UTF-8", "tier": "external" },
  "DCS": { "target": "https://vt100.net/docs/vt510-rm/chapter4.html#S4.3.4", "tier": "external" },
  "VT-100": { "target": "https://vt100.net/docs/vt100-ug/chapter3.html", "tier": "external" },
  "VT-100 spec": { "target": "https://vt100.net/docs/vt100-ug/chapter3.html", "tier": "external" },
  "VT-100 specification": {
    "target": "https://vt100.net/docs/vt100-ug/chapter3.html",
    "tier": "external",
  },
  "VT-220": { "target": "https://en.wikipedia.org/wiki/VT220", "tier": "external" },
  "DECSTBM": { "target": "https://vt100.net/docs/vt510-rm/DECSTBM.html", "tier": "external" },
  "DEC": {
    "target": "https://en.wikipedia.org/wiki/Digital_Equipment_Corporation",
    "tier": "external",
  },
  "xterm": { "target": "https://en.wikipedia.org/wiki/Xterm", "tier": "external" },
  "Alacritty": { "target": "https://alacritty.org/", "tier": "external" },
  "Kitty": { "target": "https://sw.kovidgoyal.net/kitty/", "tier": "external" },
  "RXVT": { "target": "https://en.wikipedia.org/wiki/Rxvt", "tier": "external" },
  "rxvt-unicode": { "target": "https://en.wikipedia.org/wiki/Rxvt-unicode", "tier": "external" },
  "urxvt": { "target": "https://en.wikipedia.org/wiki/Rxvt-unicode", "tier": "external" },
  "ConPTY": {
    "target": "https://learn.microsoft.com/en-us/windows/console/creating-a-pseudoconsole-session",
    "tier": "external",
  },
  "gnome-terminal": {
    "target": "https://en.wikipedia.org/wiki/GNOME_Terminal",
    "tier": "external",
  },
  "ReGIS": { "target": "https://en.wikipedia.org/wiki/ReGIS", "tier": "external" },
  "Sixel": { "target": "https://en.wikipedia.org/wiki/Sixel", "tier": "external" },
  "X10": {
    "target": "https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h2-Mouse-Tracking",
    "tier": "external",
  },
  "ITU-T Rec. T.416": {
    "target": "https://www.itu.int/rec/T-REC-T.416-199303-I",
    "tier": "external",
  },
  "vte": { "target": "https://docs.rs/vte", "tier": "external" },
}
```

### Integration with existing pipeline

The term linker runs **after** the existing link converter (which handles inline -> reference
conversion), because it needs reference-style links to exist before it can check for missing
targets.

```
Pipeline order:
1. Fence normalization (existing)
2. Table formatting (existing)
3. Link conversion: inline -> reference (existing)
4. Term linker: add backticks, links, and targets for known terms (NEW)
```

# Implementation Plan

## Step 0: Create `known_terms.jsonc` seed file

**New file**: `build-infra/src/cargo_rustdoc_fmt/known_terms.jsonc`

Populate with the terms from the "Two-Tier Linking Convention" section above. JSONC format - JSON
with `//` comments.

## Step 1: Create `term_registry.rs` - the registry builder

**New file**: `build-infra/src/cargo_rustdoc_fmt/term_registry.rs`

This module:

1. Parses `known_terms.jsonc` (embedded via `include_str!`) to load seed terms
2. Scans workspace `.rs` files to discover additional ``[`Term`]: target`` reference definitions
3. Merges seed + discovered terms into a single registry
4. Seed file is authoritative (overrides discovered targets if they differ)

### Key types

```rust
/// Whether a term links to an internal Rust type or an external URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TermTier {
    /// Links to a crate type via intra-doc link (e.g., `crate::CsiSequence`).
    Internal,
    /// Links to an external URL (e.g., `https://en.wikipedia.org/wiki/...`).
    External,
}

/// A known term entry with its canonical link target and tier.
#[derive(Debug, Clone)]
pub struct TermEntry {
    /// The link target (crate path or URL).
    pub target: String,
    /// Whether this is an internal or external link.
    pub tier: TermTier,
}

/// Registry of known technical terms and their canonical link targets.
pub struct TermRegistry {
    /// Map from term text (e.g., "CSI") to its entry.
    terms: HashMap<String, TermEntry>,
}

impl TermRegistry {
    /// Builds the registry from the seed file (embedded or override) and workspace scan.
    pub fn build(workspace_root: &Path, terms_file: Option<&Path>) -> FormatterResult<Self>;

    /// Looks up the entry for a term.
    pub fn get(&self, term: &str) -> Option<&TermEntry>;

    /// Returns all known terms, sorted longest-first (for overlapping match priority).
    pub fn terms_longest_first(&self) -> Vec<(&str, &TermEntry)>;
}
```

### Step 1.0: Implement JSONC seed file loading

- Default: Use `include_str!("known_terms.jsonc")` to embed the seed file at compile time
- Override: If `--terms-file <path>` is provided, read from that file instead at runtime
- Strip `//` comments from lines before parsing as JSON
- Deserialize into `HashMap<String, TermEntry>` using `serde_json`
- No new dependency needed (`serde_json` is likely already in the workspace; if not, add it)

### Step 1.1: Implement workspace scanner

- Walk all `.rs` files under the workspace root
- For each file, extract rustdoc blocks (reuse `extractor.rs`)
- Within each block, find reference definitions matching: ``[`Term`]: target``
- Use regex: ``\[`([^`]+)`\]:\s+(\S+)``
- Classify discovered targets:
  - Contains `::` -> `TermTier::Internal` (intra-doc link, skip - don't override seed)
  - Starts with `http://` or `https://` -> `TermTier::External`
- Collect into `HashMap<String, TermEntry>` (first target wins for duplicates)

### Step 1.2: Implement merge logic

- Start with seed terms (authoritative)
- Add discovered terms that are NOT already in the seed
- In verbose mode, log when a discovered target differs from the seed

## Step 2: Create `term_linker.rs` - the doc block fixer

**New file**: `build-infra/src/cargo_rustdoc_fmt/term_linker.rs`

This module takes a doc block (as text) and the `TermRegistry`, and:

1. Finds plain-text or backticked-only occurrences of known terms
2. Upgrades ALL occurrences to backticked+linked form
3. Adds missing link target definitions at the bottom of the block
4. Fixes incorrect link target URLs/paths

### Key function

```rust
/// Upgrades known terms in a doc block to backticked+linked form.
///
/// Links ALL occurrences of each known term (not just the first).
/// Returns the modified text, or the original if no changes were needed.
#[must_use]
pub fn link_known_terms(text: &str, registry: &TermRegistry) -> String;
```

### Step 2.0: Implement term detection

For each known term (processed longest-first to handle overlapping terms like `VT-100 specification`
before `VT-100`), build a regex that matches:

- Whole-word only (word boundaries or punctuation boundaries)
- Three states to detect:
  1. Plain text: `CSI` (not inside backticks, links, code fences, or headings)
  2. Backticked only: `` `CSI` `` (not inside link brackets)
  3. Linked: ``[`CSI`]`` (check if target definition exists and is correct)

Use `ContentProtector` to shield code fences and HTML before scanning.

### Step 2.1: Implement term upgrading

For each detected term occurrence (ALL occurrences, not just first):

- Plain `CSI` -> ``[`CSI`]``
- `` `CSI` `` -> ``[`CSI`]``
- ``[`CSI`]`` (already linked) -> keep as-is

Track which terms need target definitions added or corrected.

### Step 2.2: Implement target definition management

After processing all lines in the block:

- Collect all terms that were linked (new or existing)
- Check which already have correct target definitions at the bottom
- Add missing definitions: ``[`CSI`]: crate::CsiSequence`` or ``[`ANSI`]: https://...``
- Fix incorrect definitions (target doesn't match registry)
- Sort definitions alphabetically (consistent with existing `link_converter.rs` behavior)

### Step 2.3: Handle edge cases

- **All occurrences linked**: Every occurrence of a known term gets linked, not just the first.
- **Terms inside headings**: Skip (headings should stay clean)
- **Terms inside code fences**: Skip (protected by `ContentProtector`)
- **Terms inside inline code spans**: Skip (e.g., `` `CSI [ 38 ; 5 ; n m` `` - the `CSI` here is
  part of a code example, not a standalone linkable term). Only match when the entire inline code
  span is the term itself (`` `CSI` `` matches, `` `CSI sequence` `` does not).
- **Overlapping terms**: Process longest terms first (e.g., `VT-100 specification` before `VT-100`)
- **Case sensitivity**: Exact match only (`CSI` does not match `csi`)
- **Existing intra-doc links**: Do not touch ``[`CsiSequence`]`` - these are already proper
  intra-doc links to Rust types. Only match terms from the registry.

## Step 3: Add `link_terms` to `FormatOptions`

**File**: `build-infra/src/cargo_rustdoc_fmt/types.rs`

- New field: `pub link_terms: bool` - defaults to `true`
- Update `Default` impl and `test_format_options_default` assertion

## Step 4: Add `--terms-only` and `--terms-file` CLI flags

**File**: `build-infra/src/cargo_rustdoc_fmt/cli_arg.rs`

- New `CLIArg` field: `pub terms_only: bool` with `#[arg(long)]`
- When `--terms-only` is set, only term linking runs (no tables, no inline link conversion)
- New `CLIArg` field: `pub terms_file: Option<PathBuf>` with `#[arg(long)]`
- When `--terms-file <path>` is set, load the seed from the given JSONC file instead of the embedded
  one. Useful for testing new terms without recompiling.
- Update mutual exclusion logic in `to_format_options()`
- Update `long_about` help text

## Step 5: Wire into `processor.rs` pipeline

**File**: `build-infra/src/cargo_rustdoc_fmt/processor.rs`

Insert term linking as the **last** transformation in the pipeline:

```
1. Fence normalization (existing)
2. Table formatting (existing)
3. Link conversion: inline -> reference (existing)
4. Term linker: backtick + link + target for known terms (NEW)
```

The term linker runs last because:

- It needs inline links to already be converted to reference-style
- It needs to check existing reference definitions (which may have been aggregated by step 3)

### Step 5.0: Build `TermRegistry` once at startup

The registry should be built **once** when the binary starts (not per-file). Pass it through the
processing pipeline.

- Modify `FileProcessor::new()` or the top-level processing function to accept `&TermRegistry`
- Build the registry in `main()` or the CLI entry point

### Step 5.1: Apply term linking per doc block

After existing transformations, if `options.link_terms` is true:

```rust
if options.link_terms {
    block_text = link_known_terms(&block_text, registry);
}
```

## Step 6: Register modules in `mod.rs`

**File**: `build-infra/src/cargo_rustdoc_fmt/mod.rs`

- Add `pub mod term_registry;`
- Add `pub mod term_linker;`
- Add corresponding `pub use` re-exports

## Step 7: Add dependencies

**File**: `build-infra/Cargo.toml`

- Add `serde_json = "1"` to `[dependencies]` (if not already present)
- `serde` with `derive` feature (if not already present)

## Step 8: Write tests

### Step 8.0: Unit tests for `term_registry.rs`

- Test seed file parsing (JSONC comment stripping + JSON parsing)
- Test workspace scanning with mock files
- Test merge logic (seed overrides discovered)
- Test that existing intra-doc links are excluded from discovery
- Test `terms_longest_first()` ordering

### Step 8.1: Unit tests for `term_linker.rs`

- Plain text term -> backticked+linked
- Backticked term -> linked
- Linked term with missing target -> target added
- Linked term with wrong target -> target corrected
- Term inside code fence -> not touched
- Term inside inline code span (longer than just the term) -> not touched
- Term inside heading -> not touched
- Multiple occurrences of same term -> all linked
- Overlapping terms (longer match first)
- Tier 1 term -> intra-doc link target added
- Tier 2 term -> external URL target added
- No known terms in block -> unchanged

### Step 8.2: Integration test fixtures

Create test fixture pairs in `validation_tests/test_data/complete_file/`:

- `input/sample_term_linker.rs` - file with various term states (plain, backticked,
  linked-no-target, linked-wrong-target, tier 1 and tier 2 terms)
- `expected_output/sample_term_linker.rs` - fully linked version with correct targets

### Step 8.3: Test in `complete_file_tests.rs`

- Add `test_term_linker_upgrades_known_terms()` test function

## Step 9: Build, test, install

```bash
./check.fish --check
./check.fish --build
./check.fish --test
./check.fish --clippy
cargo install --path build-infra --force
```

## Step 10: Run across codebase

```bash
cargo rustdoc-fmt --workspace --terms-only
cargo rustdoc-fmt --workspace --terms-only --check  # verify idempotency
```

## Step 11: Add external spec links to internal types

This is a **manual** follow-up step (not automated by the tool). For each Tier 1 type, ensure its
own rustdoc contains a link to the external specification:

| Type          | Add to its rustdoc                                                                                                    |
| :------------ | :-------------------------------------------------------------------------------------------------------------------- |
| `CsiSequence` | ``[`CSI specification`]: https://en.wikipedia.org/wiki/ANSI_escape_code#CSI_(Control_Sequence_Introducer)_sequences`` |
| `SgrCode`     | ``[`SGR specification`]: https://en.wikipedia.org/wiki/ANSI_escape_code#SGR_(Select_Graphic_Rendition)_parameters``   |
| `EscSequence` | ``[`ESC specification`]: https://en.wikipedia.org/wiki/Escape_character#ASCII_escape_character``                      |
| `DsrSequence` | ``[`DSR specification`]: https://en.wikipedia.org/wiki/ANSI_escape_code#DSR_(Device_Status_Report)``                  |
| `OscSequence` | ``[`OSC specification`]: https://en.wikipedia.org/wiki/ANSI_escape_code#OSC_(Operating_System_Command)_sequences``    |

## Step 12: Final verification

```bash
./check.fish --full
```

# Files Modified (summary)

| File                                                                | Change                                       |
| :------------------------------------------------------------------ | :------------------------------------------- |
| `build-infra/src/cargo_rustdoc_fmt/known_terms.jsonc`               | **New** - seed file with known terms (JSONC) |
| `build-infra/src/cargo_rustdoc_fmt/term_registry.rs`                | **New** - registry builder module            |
| `build-infra/src/cargo_rustdoc_fmt/term_linker.rs`                  | **New** - doc block term linker module       |
| `build-infra/src/cargo_rustdoc_fmt/mod.rs`                          | Add modules + re-exports                     |
| `build-infra/src/cargo_rustdoc_fmt/types.rs`                        | Add `link_terms` field                       |
| `build-infra/src/cargo_rustdoc_fmt/cli_arg.rs`                      | Add `--terms-only` and `--terms-file` flags  |
| `build-infra/src/cargo_rustdoc_fmt/processor.rs`                    | Wire term linker into pipeline               |
| `build-infra/src/bin/cargo-rustdoc-fmt.rs`                          | Build registry at startup                    |
| `build-infra/Cargo.toml`                                            | Add `serde_json` dependency (if needed)      |
| `.../validation_tests/complete_file_tests.rs`                       | Add test function                            |
| `.../test_data/complete_file/input/sample_term_linker.rs`           | **New** - test fixture                       |
| `.../test_data/complete_file/expected_output/sample_term_linker.rs` | **New** - test fixture                       |
| `tui/src/core/ansi/.../CsiSequence` etc.                            | Manual: add spec links to Tier 1 types       |
