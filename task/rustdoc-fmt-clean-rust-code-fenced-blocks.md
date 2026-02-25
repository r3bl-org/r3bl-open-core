# Plan: Add Fence Normalizer to `cargo rustdoc-fmt`

## Context

Rustdoc defaults to Rust for unlabeled code fences, making ` ```rust` redundant. The codebase
has ~160 occurrences across ~50 source files. We need:

1. A new `cargo rustdoc-fmt` transformation to strip redundant `rust` from fence openings
2. Updated test fixtures to match the new behavior
3. A codebase-wide run of the formatter to apply the fix

## Transformation Rules

- ` ```rust` → ` ``` ` (standalone)
- ` ```rust,<attr>` → ` ```<attr>` (e.g., `rust,ignore` → `ignore`)
- Non-rust fences unchanged (` ```text`, ` ```python`, bare ` ``` `)

## Steps

### 1. Create `fence_normalizer.rs`

**New file**: `build-infra/src/cargo_rustdoc_fmt/fence_normalizer.rs`

- Pure text-in/text-out function: `pub fn normalize_fences(text: &str) -> String`
- `#[must_use]`, `LazyLock<Regex>` — matches existing module conventions
- Regex: `^```rust(,.+)?$` — anchored, no false positives on e.g. ` ```rustic`
- Line-by-line processing: only fence-opening lines are modified
- Unit tests covering: standalone, comma-separated attrs, non-rust fences, empty input,
  no-false-positive on `rustic`

### 2. Register module in `mod.rs`

**File**: `build-infra/src/cargo_rustdoc_fmt/mod.rs`

- Add `pub mod fence_normalizer;` (after `extractor`)
- Add `pub use fence_normalizer::*;` (in re-export block)

### 3. Add `normalize_fences` to `FormatOptions`

**File**: `build-infra/src/cargo_rustdoc_fmt/types.rs`

- New field: `pub normalize_fences: bool` — defaults to `true`
- Update `Default` impl and `test_format_options_default` assertion

### 4. Add `--fences-only` CLI flag

**File**: `build-infra/src/cargo_rustdoc_fmt/cli_arg.rs`

- New `CLIArg` field: `pub fences_only: bool` with `#[arg(long)]`
- Mutual exclusion in `to_format_options()`:
  ```
  format_tables:    !links_only && !fences_only
  convert_links:    !tables_only && !fences_only
  normalize_fences: !tables_only && !links_only
  ```
- Update existing test constructors to include `fences_only: false`
- Add `test_cli_fences_only` test
- Update `long_about` help text to mention fence normalization

### 5. Wire into `processor.rs` pipeline

**File**: `build-infra/src/cargo_rustdoc_fmt/processor.rs`

- Add `fence_normalizer` to the import
- Insert fence normalization as **first** transformation (before table formatting),
  because `ContentProtector` hides the fence-opening line inside a placeholder:
  ```
  1. Fence normalization  ← NEW (must run before ContentProtector)
  2. Table formatting
  3. Link conversion (with ContentProtector)
  ```

### 6. Update 2 golden test expected outputs

These are intentional changes per `test_data/CLAUDE.md` — part of the test change.

**Input files stay unchanged** (they represent the "before" state).

| File (expected_output/) | Line | Before | After |
|:---|:---:|:---|:---|
| `sample_input_event.rs` | 122 | `/// ```rust` | `/// ``` ` |
| `sample_code_fence_comma.rs` | 31 | `//! ```rust,ignore` | `//! ```ignore` |

### 7. Update test assertion in `complete_file_tests.rs`

**File**: `build-infra/src/cargo_rustdoc_fmt/validation_tests/complete_file_tests.rs`

- Line 558: Change `formatted.contains("```rust,ignore")` → `formatted.contains("```ignore")`
- Update assertion message and test doc comment
- Add 4 `normalize_fences` fields to manually-constructed `FormatOptions` at lines 160, 190, 410, 619

### 8. Add new test fixture pair for fence normalization

- **Input**: `test_data/complete_file/input/sample_fence_normalizer.rs`
  — Synthetic file with ` ```rust`, ` ```rust,ignore`, ` ```rust,no_run`, ` ```text`, bare ` ``` `
- **Expected output**: `test_data/complete_file/expected_output/sample_fence_normalizer.rs`
  — Same file with `rust` stripped from fence openings
- **Test function** in `complete_file_tests.rs`: `test_fence_normalizer_strips_redundant_rust()`

### 9. Build, test, install

```bash
./check.fish --check
./check.fish --build
./check.fish --test
./check.fish --clippy
cargo install --path build-infra --force
```

### 10. Run formatter across codebase

`--workspace` processes ALL `.rs` files in the workspace (not just git-dirty ones).

```bash
cargo rustdoc-fmt --workspace --fences-only
cargo rustdoc-fmt --workspace --fences-only --check   # verify idempotency
```

### 11. Final verification

```bash
./check.fish --full
```

## Files Modified (summary)

| File | Change |
|:---|:---|
| `build-infra/src/cargo_rustdoc_fmt/fence_normalizer.rs` | **New** — transformation module |
| `build-infra/src/cargo_rustdoc_fmt/mod.rs` | Add module + re-export |
| `build-infra/src/cargo_rustdoc_fmt/types.rs` | Add `normalize_fences` field |
| `build-infra/src/cargo_rustdoc_fmt/cli_arg.rs` | Add `--fences-only` flag |
| `build-infra/src/cargo_rustdoc_fmt/processor.rs` | Wire normalizer into pipeline |
| `.../expected_output/sample_input_event.rs` | Update golden output |
| `.../expected_output/sample_code_fence_comma.rs` | Update golden output |
| `.../complete_file_tests.rs` | Update assertions, add new test |
| `.../input/sample_fence_normalizer.rs` | **New** — test fixture |
| `.../expected_output/sample_fence_normalizer.rs` | **New** — test fixture |
| ~50 source files in `tui/src/` | Mechanical: ` ```rust` → ` ``` ` |
