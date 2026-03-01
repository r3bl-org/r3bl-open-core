# Add More Terms to Technical Term Seed Dictionary

## Summary

Add 18 new terms to `known_technical_term_link_dictionary.jsonc`, update the seed coverage
test fixture, regenerate golden files, reinstall the binary, and apply `cargo rustdoc-fmt`
workspace-wide.

All external URLs verified (HTTP 200) and fragment anchors confirmed on 2026-03-01.

## New Terms (18 total)

### Internal - stdlib (3 terms)

| Term | Target | Tier |
|:-----|:-------|:-----|
| `stdout` | `std::io::Stdout` | internal |
| `stderr` | `std::io::Stderr` | internal |
| `stdin` | `std::io::Stdin` | internal |

### Internal - crate dependencies (3 terms)

| Term | Target | Tier | Notes |
|:-----|:-------|:-----|:------|
| `crossterm` | `crossterm` | internal | Dep in `tui/Cargo.toml` |
| `mio` | `mio` | internal | Dep in `tui/Cargo.toml` |
| `portable_pty` | `portable_pty` | internal | Dep in `tui/Cargo.toml` |

### External (12 terms)

| Term | Target | Tier |
|:-----|:-------|:-----|
| `EOF` | `https://en.wikipedia.org/wiki/End-of-file` | external |
| `SIGWINCH` | `https://en.wikipedia.org/wiki/Signal_(IPC)#SIGWINCH` | external |
| `SIGINT` | `https://en.wikipedia.org/wiki/Signal_(IPC)#SIGINT` | external |
| `SIGTERM` | `https://en.wikipedia.org/wiki/Signal_(IPC)#SIGTERM` | external |
| `grapheme` | `https://unicode.org/reports/tr29/` | external |
| `Unicode` | `https://en.wikipedia.org/wiki/Unicode` | external |
| `RGB` | `https://en.wikipedia.org/wiki/RGB_color_model` | external |
| `epoll` | `https://man7.org/linux/man-pages/man7/epoll.7.html` | external |
| `kqueue` | `https://en.wikipedia.org/wiki/Kqueue` | external |
| `Bracketed paste` | `https://en.wikipedia.org/wiki/Bracketed-paste` | external |
| `truecolor` | `https://en.wikipedia.org/wiki/Color_depth#True_color_(24-bit)` | external |
| `termios` | `https://en.wikipedia.org/wiki/Termios` | external |

## Impact Analysis

### File counts per term (in rustdoc comments)

| Term | Files |
|:-----|------:|
| `stdout` | 59 |
| `crossterm` | 57 |
| `Unicode` | 55 |
| `grapheme` | 46 |
| `stdin` | 36 |
| `RGB` | 26 |
| `portable_pty` | 23 |
| `stderr` | 21 |
| `Bracketed paste` | 19 |
| `truecolor` | 18 |
| `mio` | 12 |
| `termios` | 11 |
| `SIGWINCH` | 10 |
| `epoll` | 10 |
| `kqueue` | 9 |
| `SIGINT` | 6 |
| `SIGTERM` | 1 |
| `EOF` | 12 |

- **Total unique files with any of these terms**: ~223 (after excluding 10 `rustdoc-fmt: skip`
  files)
- **Note**: Not all occurrences will be modified - only bare or backticked-only terms get
  linkified. Already-linked terms and terms inside code fences are left unchanged.

### Risk areas

- **`grapheme`**: Appears heavily in grapheme module files. Verify these files don't have
  `rustdoc-fmt: skip` and that linkification looks natural in context.
- **`mio`**: Safe - only appears in the codebase as the Metal I/O crate name, no false positives.
- **`RGB`**: Safe - case-sensitive, only one meaning (Red Green Blue) in this codebase.
- **`portable_pty`**: Uses underscore form (Rust canonical), matching how the crate is referenced
  in code and intra-doc links.

## Implementation Steps

### Step 1: Add terms to the seed dictionary

**File**: `build-infra/src/cargo_rustdoc_fmt/known_technical_term_link_dictionary.jsonc`

Add the 18 new entries, organized into logical groups following the existing convention:

```jsonc
  // Tier 1c: Stdlib types
  "stdout":             { "target": "std::io::Stdout",                                            "tier": "internal" },
  "stderr":             { "target": "std::io::Stderr",                                            "tier": "internal" },
  "stdin":              { "target": "std::io::Stdin",                                             "tier": "internal" },

  // Tier 1d: Crate dependencies
  "crossterm":          { "target": "crossterm",                                                   "tier": "internal" },
  "mio":                { "target": "mio",                                                         "tier": "internal" },
  "portable_pty":       { "target": "portable_pty",                                                "tier": "internal" },

  // (Add to Tier 2 section)
  "EOF":                { "target": "https://en.wikipedia.org/wiki/End-of-file",                   "tier": "external" },
  "SIGWINCH":           { "target": "https://en.wikipedia.org/wiki/Signal_(IPC)#SIGWINCH",         "tier": "external" },
  "SIGINT":             { "target": "https://en.wikipedia.org/wiki/Signal_(IPC)#SIGINT",           "tier": "external" },
  "SIGTERM":            { "target": "https://en.wikipedia.org/wiki/Signal_(IPC)#SIGTERM",          "tier": "external" },
  "grapheme":           { "target": "https://unicode.org/reports/tr29/",                           "tier": "external" },
  "Unicode":            { "target": "https://en.wikipedia.org/wiki/Unicode",                       "tier": "external" },
  "RGB":                { "target": "https://en.wikipedia.org/wiki/RGB_color_model",               "tier": "external" },
  "epoll":              { "target": "https://man7.org/linux/man-pages/man7/epoll.7.html",          "tier": "external" },
  "kqueue":             { "target": "https://en.wikipedia.org/wiki/Kqueue",                        "tier": "external" },
  "Bracketed paste":    { "target": "https://en.wikipedia.org/wiki/Bracketed-paste",               "tier": "external" },
  "truecolor":          { "target": "https://en.wikipedia.org/wiki/Color_depth#True_color_(24-bit)", "tier": "external" },
  "termios":            { "target": "https://en.wikipedia.org/wiki/Termios",                       "tier": "external" },
```

### Step 2: Update the seed term coverage test fixture (input)

**File**: `build-infra/test_data/complete_file/input/sample_seed_term_coverage.rs`

Add the 18 new terms in **all three states** (bare, backticked, already-linked), following the
existing pattern. Add new sections for the new tiers:

- **Tier 1c section**: stdout, stderr, stdin (bare / backticked / already-linked)
- **Tier 1d section**: crossterm, mio, portable_pty (bare / backticked / already-linked)
- **Tier 2 additions**: EOF, SIGWINCH, SIGINT, SIGTERM, grapheme, Unicode, RGB, epoll, kqueue,
  Bracketed paste, truecolor, termios (bare / backticked / already-linked)
- **Edge cases**: Add qualified paths like `std::io::stdout()`, `crossterm::event::read()`,
  `mio::Poll::new()` that must NOT be split
- **Reference-style links** at the bottom for the already-linked variants

### Step 3: Update the seed term coverage unit test

**File**: `build-infra/src/cargo_rustdoc_fmt/technical_term_dictionary.rs`
**Function**: `test_all_seed_terms_present()`

Add the new terms to the assertion lists:

```rust
// Tier 1c: Stdlib types.
for term in ["stdout", "stderr", "stdin"] {
    assert!(registry.get(term).is_some(), "Missing tier 1c term: {term}");
}

// Tier 1d: Crate dependencies.
for term in ["crossterm", "mio", "portable_pty"] {
    assert!(registry.get(term).is_some(), "Missing tier 1d term: {term}");
}

// Add to tier 2 sample list:
// "EOF", "SIGWINCH", "SIGINT", "SIGTERM", "grapheme", "Unicode", "RGB",
// "epoll", "kqueue", "Bracketed paste", "truecolor", "termios"
```

### Step 4: Regenerate golden files

```bash
cd /home/nazmul/github/roc/build-infra
cargo test -p r3bl-build-infra --lib -- regen_golden_output_snapshot_files_for_input_files --ignored --nocapture
```

This regenerates:
- `build-infra/test_data/complete_file/expected_output/sample_seed_term_coverage.rs`

### Step 5: Run the full test suite

```bash
cd /home/nazmul/github/roc/build-infra
cargo test -p r3bl-build-infra --lib
```

All tests must pass, including:
- `test_all_seed_terms_present` - verifies dictionary has all expected terms
- `complete_file_tests` - verifies input/expected_output golden file pairs match
- All other existing tests - verifies no regressions

### Step 6: Reinstall the binary

```bash
cd /home/nazmul/github/roc/build-infra
cargo install --path . --force
```

### Step 7: Apply to the workspace (dry run first)

```bash
# Dry run - see what would change
cd /home/nazmul/github/roc
cargo rustdoc-fmt --check tui/

# Full application
cargo rustdoc-fmt tui/
```

### Step 8: Review the diff

```bash
cd /home/nazmul/github/roc
git diff --stat
git diff
```

Review for:
- False positives (terms matched where they shouldn't be)
- Broken qualified paths (e.g., `std::io::stdout()` incorrectly split)
- Terms inside code fences that were incorrectly linkified
- Natural-looking link text in context

### Step 9: Run workspace checks

```bash
cd /home/nazmul/github/roc
./check.fish --check   # Typecheck
./check.fish --doc     # Docs build (verifies intra-doc links resolve)
./check.fish --test    # Tests pass
```

The `--doc` step is critical - it will catch any broken intra-doc links (e.g., if `std::io::Stdout`
or `crossterm` or `mio` or `portable_pty` don't resolve from the `tui` crate context).

### Step 10: Commit

Two commits:
1. `[build-infra] Add 18 new terms to technical term dictionary` - dictionary, test fixture,
   golden files, unit test changes
2. `[tui] Apply cargo rustdoc-fmt with new term dictionary` - the workspace-wide auto-linking
   changes

## Verification Checklist

- [ ] All 18 terms present in the jsonc dictionary
- [ ] Seed coverage test fixture has all 18 terms in 3 states (bare, backticked, linked)
- [ ] `test_all_seed_terms_present` passes
- [ ] Golden file regeneration succeeds
- [ ] Full build-infra test suite passes
- [ ] Binary reinstalled
- [ ] `cargo rustdoc-fmt` applied to workspace
- [ ] No false positives in the diff
- [ ] `./check.fish --doc` passes (intra-doc links resolve)
- [ ] `./check.fish --test` passes
- [ ] Two clean commits created

## Design Decisions (resolved)

- **`stdin`/`stdout`/`stderr` target**: Using the type (`std::io::Stdin`) rather than the
  function (`std::io::stdin`). The type is more conventional for intra-doc link targets.
- **`crossterm`/`mio`/`portable_pty` tier**: Using `internal` tier since these are Cargo
  dependencies, enabling intra-doc resolution via `::crate_name` paths.
- **`portable_pty`**: Uses underscore form (Rust canonical), not `portable-pty` (hyphenated).
  The crate is `portable-pty` on crates.io but Rust code and intra-doc links use `portable_pty`.
- **Signal terms**: Including all three (`SIGWINCH`, `SIGINT`, `SIGTERM`) for completeness.
- **`EOF`**: All-caps, single meaning (End-of-file), already manually linked to the same
  Wikipedia URL in 8+ files. Auto-linking eliminates repetitive boilerplate.
- **`fd` rejected**: Too short (2 chars), appears in code fences, compound terms (`fd 0`,
  `stdin fd`), and ASCII diagrams. Best left as per-file manual links.
- **URL verification**: All 10 external base URLs return HTTP 200. All 4 fragment anchors
  (`#SIGWINCH`, `#SIGINT`, `#SIGTERM`, `#True_color_(24-bit)`) confirmed present on their
  respective pages.
