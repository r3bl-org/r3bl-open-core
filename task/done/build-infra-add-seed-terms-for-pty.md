# Plan: Add PTY/pty to Technical Term Dictionary

## Context

The `known_technical_term_link_dictionary.jsonc` has no entries for PTY/pty. The
`generate_pty_test.rs` file manually links `[`PTY`]` to `crate::core::pty`, but uses
uppercase display text for a lowercase module name - a mismatch.

**Goal:** Add `PTY` (external, Wikipedia) and `pty` (internal, `crate::core::pty`) to the
dictionary, and fix `generate_pty_test.rs` to use lowercase `pty` for the module link.

## Changes

### 1. Fix `generate_pty_test.rs` manual links (task a)

**File:** `tui/src/core/test_fixtures/pty_test_fixtures/generate_pty_test.rs`

- Change all `[`PTY`]` → `[`pty`]` in doc comment text (lines 6, 26, 29, 34, 43)
- Change reference definition `[`PTY`]: crate::core::pty` → `[`pty`]: crate::core::pty` (line 11)

### 2. Add entries to dictionary (task b)

**File:** `build-infra/src/cargo_rustdoc_fmt/known_technical_term_link_dictionary.jsonc`

Add to Tier 1 (internal types):
```json
"pty":  { "target": "crate::core::pty", "tier": "internal" },
```

Add to Tier 2 (external concepts):
```json
"PTY":  { "target": "https://en.wikipedia.org/wiki/Pseudoterminal", "tier": "external" },
```

### 3. Update seed term coverage input fixture

**File:** `build-infra/test_data/complete_file/input/sample_seed_term_coverage.rs`

- Add `pty` to the three Tier 1 sections (bare, backticked, already-linked)
- Add `PTY` to the three Tier 2 sections (bare, backticked, already-linked)

### 4. Update unit test

**File:** `build-infra/src/cargo_rustdoc_fmt/technical_term_dictionary.rs`

- Add `"pty"` to the Tier 1 term list in `test_all_seed_terms_present()`
- Add `"PTY"` to the Tier 2 term list in `test_all_seed_terms_present()`

### 5. Regenerate golden output and run tests

```bash
cargo test -p r3bl-build-infra --lib -- regen_golden_output_snapshot_files_for_input_files --ignored --nocapture
cargo test -p r3bl-build-infra --lib
```

### 6. Install updated binary and run `cargo rustdoc-fmt` on `generate_pty_test.rs`

After all tests pass, install the updated binary and run the formatter on the file that
inspired this task in the first place - to apply all the new PTY/pty auto-linking fixes:

```bash
cargo install --path build-infra --force
cargo rustdoc-fmt tui/src/core/test_fixtures/pty_test_fixtures/generate_pty_test.rs
```

## Word Boundary Safety

Verified the linker uses `is_ascii_alphanumeric() || b == b'_'` for word boundaries:
- `pty` won't match inside `pty_pair`, `PtyPair`, `pty_test` (underscore/alpha boundary)
- `PTY` won't match inside `ConPTY` (alpha boundary on `n`)

## File Summary

| File | Action |
|------|--------|
| `tui/.../generate_pty_test.rs` | Fix `[`PTY`]` → `[`pty`]`, then run `cargo rustdoc-fmt` |
| `build-infra/.../known_technical_term_link_dictionary.jsonc` | Add 2 entries |
| `build-infra/test_data/.../input/sample_seed_term_coverage.rs` | Add PTY+pty to all sections |
| `build-infra/.../technical_term_dictionary.rs` | Add to unit test assertions |
| `build-infra/test_data/.../expected_output/sample_seed_term_coverage.rs` | Regenerate via test |
