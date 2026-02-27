# Fix rustdoc-fmt: Protect TOC blocks, anchor links, image links, and link text

## Context

`cargo rustdoc-fmt` mangles files with README-style content (TOCs, external links, images).
The `tui/src/readline_async/mod.rs` diff shows all four bugs in one file. We fix the root
causes and add that file as an e2e golden test.

## Changes

### Fix 1: Skip `#anchor` links in link converter

**File:** `build-infra/src/cargo_rustdoc_fmt/link_converter.rs`

In `convert_links()` (line 89), the `INLINE_LINK_REGEX.replace_all` converts ALL inline
links to reference-style. Add a check: if the captured URL (group 2) starts with `#`,
leave the link unchanged.

In the capture loop (line 77-86), also skip adding `#anchor` URLs to `link_info` so no
ref def is generated for them.

### Fix 2: Skip image links in link converter

**File:** `build-infra/src/cargo_rustdoc_fmt/link_converter.rs`

The `INLINE_LINK_REGEX` matches `![alt](url)` because the `!` is before the `[` and the
regex starts at `\[`. In `replace_all` (line 89), check if the character before the match
start is `!` - if so, leave unchanged. Same skip in the capture loop (line 77-86).

### Fix 3: Protect `<!-- TOC -->` ... `<!-- /TOC -->` blocks

Two places need protection:

**File:** `build-infra/src/cargo_rustdoc_fmt/content_protector.rs`

Currently, `<!-- TOC -->` and `<!-- /TOC -->` are each protected as individual single-line
HTML comments (lines 138-142), but the content between them is NOT protected. Change the
HTML comment handler: when a line contains `<!-- TOC` (opening marker), collect all lines
through `<!-- /TOC -->` (closing marker) as one protected block - similar to the multi-line
HTML comment pattern (lines 145-161). This protects the entire TOC from the link converter.

**File:** `build-infra/src/cargo_rustdoc_fmt/technical_term_linker.rs`

In `link_known_terms()`, add `inside_toc_block` state tracking alongside the existing
`inside_code_fence` flag (line 77). Toggle on `<!-- TOC` / `<!-- /TOC -->`. Skip term
processing for lines inside TOC blocks.

### Fix 4: Don't linkify terms inside existing markdown link text

**File:** `build-infra/src/cargo_rustdoc_fmt/technical_term_linker.rs`

Add `find_markdown_link_ranges(line: &str) -> Vec<(usize, usize)>` helper, similar to the
existing `find_inline_code_spans()`. It finds byte ranges of link text content in
`[text](url)` and `[text][ref]` patterns (the content between the brackets).

In `upgrade_term_in_line()`, compute link ranges alongside code spans. In Step 2 (plain
text upgrade), check if the term position falls inside a link range - if so, skip.
Also check in Step 1 (backtick upgrade) so `` `TTY` `` inside `[Linux `TTY` and...](url)`
isn't upgraded.

### Fix 5: Add e2e golden test using `readline_async/mod.rs`

**Files:**
- `build-infra/test_data/complete_file/input/sample_readline_async.rs` - copy of current
  `tui/src/readline_async/mod.rs`
- `build-infra/test_data/complete_file/expected_output/sample_readline_async.rs` - expected
  output after formatting
- `build-infra/src/cargo_rustdoc_fmt/validation_tests/complete_file_tests.rs` - new test
  function

Copy `tui/src/readline_async/mod.rs` as the input fixture. Run the formatter on it to
generate expected output (after fixes 1-4 are in place). Add a test function following the
pattern of `test_real_world_file_complete_formatting` (line 282).

## File summary

| File | Change |
|------|--------|
| `link_converter.rs` | Skip `#anchor` URLs and image links |
| `content_protector.rs` | Protect `<!-- TOC -->` ... `<!-- /TOC -->` as one block |
| `technical_term_linker.rs` | TOC block skipping + link text range protection |
| `complete_file_tests.rs` | New e2e golden test |
| `test_data/.../input/sample_readline_async.rs` | New test fixture (input) |
| `test_data/.../expected_output/sample_readline_async.rs` | New test fixture (expected) |

## Verification

1. `cargo test -p r3bl-build-infra --lib` - all unit + golden tests pass
2. `cargo install --path build-infra --force` - reinstall binary
3. Run `cargo rustdoc-fmt tui/src/readline_async/mod.rs` - verify no mangling
4. `./check.fish --full` - full suite passes, no doc warnings from our changes
