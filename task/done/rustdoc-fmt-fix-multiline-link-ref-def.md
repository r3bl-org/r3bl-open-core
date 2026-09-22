# Fix rustdoc-fmt deleting multi-line plain reference definitions

## Problem

`cargo rustdoc-fmt` silently deletes multi-line plain (non-backticked) reference
definitions like:

```rust
/// [Inclusive Naming Initiative - Tier 1 Terms]:
///     https://inclusivenaming.org/word-lists/tier-1/
```

These are valid markdown reference definitions that rustfmt may have split across two
lines to respect the line length limit.

## Root Cause

The bug is in `technical_term_linker.rs::parse_ref_def()`. This function is called from
`link_known_terms()` to parse existing reference definitions so they can be preserved
during term linking.

The function handles three cases:
1. Backticked single-line: `` [`Term`]: target `` (looks for `` `]: `` with space)
2. Backticked multi-line: `` [`Term`]:\n    target `` (looks for `` `]: `` without space)
3. **Plain single-line**: `[Term]: target` (looks for `]: ` with space)

**Missing case**: Plain multi-line `[Term]:\n    target` (where `]:\n` has no space).

When `split_off_ref_defs()` combines a two-line reference into a single string
`"[Term]:\n    url"`, `parse_ref_def()` fails to parse it (returns `None`). The
reference is then silently dropped at line 184-190 where only refs with
`parse_ref_def() == Some(...)` are kept.

The link converter (`link_converter.rs::separate_references()`) handles this correctly.
Only the term linker's `parse_ref_def()` is broken.

## Fix

In `technical_term_linker.rs::parse_ref_def()`, add a fourth case after the plain
single-line check:

```rust
// Multi-line plain ref def: [Term]:\n    target
if line.starts_with('[')
    && let Some(end) = line.find("]:")
{
    let term = line[1..end].to_string();
    let target = line[end + 2..].trim().to_string();
    return Some((term, target));
}
```

This mirrors the existing backticked multi-line handling (case 2).

## Files to Change

- [x] `build-infra/src/cargo_rustdoc_fmt/technical_term_linker.rs` - Add multi-line
  plain ref def parsing in `parse_ref_def()`
- [x] `build-infra/src/cargo_rustdoc_fmt/technical_term_linker.rs` - Add unit test
  `test_parse_ref_def_plain_multiline`
- [x] `build-infra/test_data/complete_file/input/sample_pty_types.rs` - Add e2e test
  fixture (copy of `tui/src/core/pty/pty_core/pty_types.rs`)
- [x] `build-infra/test_data/complete_file/expected_output/sample_pty_types.rs` -
  Regenerate golden file
- [x] `build-infra/src/cargo_rustdoc_fmt/validation_tests/complete_file_tests.rs` - Add
  e2e test `test_multiline_plain_ref_def_preserved`
- [x] `cargo install --path build-infra --force` - Install updated binary

## Verification

After the fix, running `cargo rustdoc-fmt --check tui/src/core/pty/pty_core/pty_types.rs`
should report `0 modified` (the multi-line reference is preserved).
