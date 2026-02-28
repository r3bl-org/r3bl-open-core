# cargo-rustdoc-fmt: remove --include-lib-rs, fix shortcut ref links, seed-only registry

## Part 1: Remove `--include-lib-rs` [DONE]

Deleted the flag and filtering logic. TOC blocks are now protected content, and the
offending TOC in `readline_async/mod.rs` was removed.

## Part 2: Fix term linker clobbering shortcut reference links [DONE]

Updated `find_markdown_link_ranges()` to protect standalone `[text]` (shortcut reference
links). Excludes `` [`Term`] `` (starts with backtick) and `[ref]` immediately after `]`
(ref label of full reference link). Reverted corrupted line in `readline_async/mod.rs`.

## Part 3: Drop workspace scanning, make seed file the single source of truth

The workspace scan (`TechnicalTermDictionary::build()`) augments the seed file by scanning
all `.rs` files for reference-style link definitions. This causes:

- **Corruption propagation**: a corrupted ref def in one file spreads to all others
- **Non-determinism**: formatting output depends on what other files exist in the workspace
- **Golden file mismatch**: tests use seed-only, binary uses workspace scan (different terms)
- **Conflict resolution ambiguity**: same term with different targets in different files

**Changes:**

- **`build-infra/src/cargo_rustdoc_fmt/technical_term_dictionary.rs`**: Remove the
  `build()` method (workspace scanner). Keep only `from_seed()`.
- **`build-infra/src/bin/cargo-rustdoc-fmt.rs`**: Replace `TechnicalTermDictionary::build()`
  call with `TechnicalTermDictionary::from_seed()`. Remove the workspace_root lookup that
  was only needed for the scanner.
- **`build-infra/src/cargo_rustdoc_fmt/cli_arg.rs`**: Remove `--terms-file` if it was only
  needed to override the workspace scan (evaluate).
- **Tests**: Any tests that call `build()` should use `from_seed()` instead.
- **Seed file**: Audit `known_technical_term_link_dictionary.jsonc` to ensure it has all
  needed terms (check what the workspace scanner was finding that the seed doesn't have).

## Investigation note: multi-line inline code span detection

**Decision: keep it.** ~10 tui/ files have genuine multi-line inline code spans from rustfmt
wrapping. The code is cheap (~75 lines, no allocations).

## Verification

1. `cargo test -p r3bl-build-infra --lib` - all tests pass
2. `cargo install --path build-infra --force` - install updated binary
3. Run `cargo rustdoc-fmt` on `tui/src/readline_async/mod.rs` - verify no clobbering
4. `cargo rustdoc-fmt --workspace --verbose` - verify lib.rs files are now processed
5. `./check.fish --clippy` - no warnings
