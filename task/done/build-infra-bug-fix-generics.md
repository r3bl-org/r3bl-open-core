# Fix rustdoc-fmt Content Protector and Inline Link Regex Bugs

## Summary

Running `cargo rustdoc-fmt` on files with generic type parameters in reference link
definitions (like `` [`Vec<u8>`]: std::vec::Vec ``) caused empty lines, wrong reference
ordering, and content duplication. Two bugs were fixed:

## Bug A: ContentProtector False Positives

**Root cause:** `HTML_TAG_REGEX` (`<[^>]+>`) in `content_protector.rs` matched generic
type params like `<u8>`, `<F::Waker>` inside backtick code spans in reference definition
lines. The entire line got "protected" (replaced with placeholder), hiding it from the
link converter. Unprotected references got sorted and appended at the bottom, while
protected ones stayed in place - creating gaps and wrong ordering.

**Fix:** Added `BACKTICK_SPAN_REGEX` to strip backtick-delimited spans from a copy of
the line before testing against `HTML_TAG_REGEX`. If the stripped version doesn't match,
the line is not protected.

**Files:** `content_protector.rs`, `sample_resilient_reactor.rs` expected output

## Bug B: Inline Link Regex Cross-Line Matching

**Root cause:** `INLINE_LINK_REGEX` used `[^\]]+` for link text, which matched any
character except `]` - including `[` and newlines. This allowed `[200~` in escape
sequence text (`` `ESC[200~` ``) to chain through other `[` characters across many lines
until reaching a distant `]`, creating a pathological match that duplicated content.

**Fix:** Changed the character class from `[^\]]+` to `[^\[\]]+`, excluding both `[` and
`]` from the link text capture. This prevents cross-line chaining while preserving
legitimate multi-line inline links (whose text never contains `[`).

**File:** `link_converter.rs`

## Files Modified

| File | Change |
|:-----|:-------|
| `build-infra/src/cargo_rustdoc_fmt/content_protector.rs` | Added `BACKTICK_SPAN_REGEX`, strip backtick spans before HTML check, 4 unit tests |
| `build-infra/src/cargo_rustdoc_fmt/link_converter.rs` | Changed `[^\]]+` to `[^\[\]]+` in `INLINE_LINK_REGEX`, 1 regression test |
| `build-infra/src/cargo_rustdoc_fmt/validation_tests/complete_file_tests.rs` | Added 3 e2e tests for all 3 failing files |
| `build-infra/.../expected_output/sample_resilient_reactor.rs` | Updated expected reference sorting |

## New Test Fixtures (3 input/output pairs)

| Fixture | Test | Bug |
|:--------|:-----|:----|
| `sample_rrt.rs` | `test_rrt_generic_type_refs_sorted_correctly` | Bug A: `<F::Waker>`, `<u8>` in backticks |
| `sample_rrt_mod.rs` | `test_rrt_mod_reference_sorting` | Bug A: reference sorting across large doc block |
| `sample_input_event.rs` | `test_input_event_no_cross_line_duplication` | Bug B: `[200~` cross-line regex match |

## Affected Source Files

- `tui/src/core/resilient_reactor_thread/rrt.rs` - references with generics now sorted correctly
- `tui/src/core/resilient_reactor_thread/mod.rs` - references sorted alphabetically, no gaps
- `tui/src/core/terminal_io/input_event.rs` - inline link converted, no content duplication
