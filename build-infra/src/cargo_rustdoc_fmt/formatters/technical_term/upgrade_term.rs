// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// rustdoc-fmt: skip

//! Upgrades plain or backticked occurrences of a technical term to linked form
//! in a single line of Markdown text.

use super::markdown_spans::{find_inline_code_spans, find_md_link_ranges};
use crate::cargo_rustdoc_fmt::constants::{BACKTICK_BYTE, BACKTICK_CHAR,
                                          IMAGE_LINK_PREFIX_BYTE, LINK_LABEL_CLOSE,
                                          LINK_LABEL_CLOSE_BYTE, LINK_LABEL_OPEN,
                                          LINK_LABEL_OPEN_BYTE, LINK_REF_COLON_BYTE,
                                          LINK_TARGET_CLOSE, LINK_TARGET_OPEN_BYTE,
                                          RUST_PATH_SEPARATOR, UNDERSCORE_BYTE};

/// Returns the linked form of a term for use in doc comment text.
///
/// - Plain term `"CSI"` → `` [`CSI`] ``
/// - Compound term `` "[`CSI` spec]" `` → `[`[`CSI`]` spec]`
///
/// [`CSI` spec]: https://en.wikipedia.org/wiki/ANSI_escape_code#CSI
/// [`CSI`]: crate::CsiSequence
pub(super) fn linked_form(term: &str) -> String {
    if term.contains(BACKTICK_CHAR) {
        format!("{LINK_LABEL_OPEN}{term}{LINK_LABEL_CLOSE}")
    } else {
        format!("{LINK_LABEL_OPEN}{BACKTICK_CHAR}{term}{BACKTICK_CHAR}{LINK_LABEL_CLOSE}")
    }
}

/// Checks whether `line` contains a standalone reference-style link for `term`
/// (e.g., `` [`Term`] `` or `[{compound_term}]`) that is not an image (`![...]`),
/// not an inline/full-reference link (`[...](...)` or `[...][...]`), and not
/// inside an enclosing multi-backtick code span (e.g., ``` `` [`Term`] `` ```).
pub(super) fn contains_standalone_linked_term(line: &str, term: &str) -> bool {
    let pattern = linked_form(term);
    if !line.contains(&pattern) {
        return false;
    }
    let code_spans = find_inline_code_spans(line);
    let bytes = line.as_bytes();
    let mut search_start = 0;
    while let Some(pos) = line[search_start..].find(&pattern) {
        let abs_pos = search_start + pos;
        let end_pos = abs_pos + pattern.len();
        let is_image =
            abs_pos > 0 && bytes.get(abs_pos - 1) == Some(&IMAGE_LINK_PREFIX_BYTE);
        let has_inline_or_ref_suffix = match bytes.get(end_pos).copied() {
            Some(LINK_TARGET_OPEN_BYTE) => {
                line[end_pos + 1..].contains(LINK_TARGET_CLOSE)
            }
            Some(LINK_LABEL_OPEN_BYTE) => line[end_pos + 1..].contains(LINK_LABEL_CLOSE),
            _ => false,
        };
        let inside_enclosing_code_span =
            code_spans.iter().any(|&(s, e)| abs_pos > s && end_pos < e);
        if !is_image && !has_inline_or_ref_suffix && !inside_enclosing_code_span {
            return true;
        }
        search_start = end_pos;
    }
    false
}

/// Upgrades a single term occurrence in a line.
///
/// For **plain terms** (no backticks), handles four cases:
/// 1. Already linked: `` [`Term`] `` - leave as-is
/// 2. Plain shortcut link: `[Term]` - upgrade to `` [`Term`] ``
/// 3. Backticked only: `` `Term` `` - upgrade to `` [`Term`] ``
/// 4. Plain text: `Term` - upgrade to `` [`Term`] ``
///
/// For **compound terms** (contain backticks, e.g., `` [`CSI` spec] ``):
/// 1. Already linked: `[`[`CSI`]` spec]` - leave as-is
/// 2. Not linked: `` [`CSI` spec] `` - upgrade to `[`[`CSI`]` spec]`
///
/// [`CSI` spec]: https://en.wikipedia.org/wiki/ANSI_escape_code#CSI
/// [`CSI`]: crate::CsiSequence
pub(super) fn upgrade_term_in_line(line: &str, term: &str) -> String {
    // Fast path: if the line does not contain the term substring at all, skip
    // allocations.
    if !line.contains(term) {
        return line.to_string();
    }

    if term.contains(BACKTICK_CHAR) {
        return upgrade_compound_term_in_line(line, term);
    }

    let plain_bracket_pattern = format!("{LINK_LABEL_OPEN}{term}{LINK_LABEL_CLOSE}");
    let backtick_pattern = format!("{BACKTICK_CHAR}{term}{BACKTICK_CHAR}");
    let linked_pattern = linked_form(term);

    // Step 0: Upgrade standalone plain shortcut link [Term] -> [`Term`].
    let line =
        upgrade_plain_bracket_term_in_line(line, &plain_bracket_pattern, &linked_pattern);

    // Step 1: Upgrade backticked-only `Term` -> [`Term`].
    let line = upgrade_backticked_term_in_line(&line, &backtick_pattern, &linked_pattern);

    // Step 2: Upgrade plain text Term -> [`Term`].
    upgrade_plain_word_term_in_line(&line, term, &linked_pattern)
}

/// Upgrades standalone plain shortcut link `[Term]` to `` [`Term`] ``.
///
/// Leaves `[Term](url)`, `[Term][ref]`, `[Term]: target`, `![Term]`, and `][Term]`
/// untouched.
fn upgrade_plain_bracket_term_in_line(
    line: &str,
    plain_bracket_pattern: &str,
    linked_pattern: &str,
) -> String {
    if !line.contains(plain_bracket_pattern) {
        return line.to_string();
    }
    let code_spans = find_inline_code_spans(line);
    let bytes = line.as_bytes();
    let mut step0 = String::with_capacity(line.len() + 4);
    let mut search_start = 0;
    while let Some(pos) = line[search_start..].find(plain_bracket_pattern) {
        let abs_pos = search_start + pos;
        let end_pos = abs_pos + plain_bracket_pattern.len();
        let preceded_by_bang_or_bracket = abs_pos > 0
            && matches!(
                bytes.get(abs_pos - 1).copied(),
                Some(IMAGE_LINK_PREFIX_BYTE | LINK_LABEL_CLOSE_BYTE)
            );
        let followed_by_link_target = matches!(
            bytes.get(end_pos).copied(),
            Some(LINK_TARGET_OPEN_BYTE | LINK_LABEL_OPEN_BYTE | LINK_REF_COLON_BYTE)
        );
        let inside_code_span = code_spans
            .iter()
            .any(|&(s, e)| abs_pos >= s && end_pos <= e);

        if preceded_by_bang_or_bracket || followed_by_link_target || inside_code_span {
            step0.push_str(&line[search_start..end_pos]);
        } else {
            step0.push_str(&line[search_start..abs_pos]);
            step0.push_str(linked_pattern);
        }
        search_start = end_pos;
    }
    step0.push_str(&line[search_start..]);
    step0
}

/// Upgrades backticked-only `` `Term` `` to `` [`Term`] `` when not part of an existing
/// link or enclosed in a multi-backtick span.
fn upgrade_backticked_term_in_line(
    line: &str,
    backtick_pattern: &str,
    linked_pattern: &str,
) -> String {
    let code_spans = find_inline_code_spans(line);
    let link_ranges = find_md_link_ranges(line);

    let mut result = String::new();
    let mut search_start = 0;
    let bytes = line.as_bytes();
    while let Some(pos) = line[search_start..].find(backtick_pattern) {
        let abs_pos = search_start + pos;
        let end_pos = abs_pos + backtick_pattern.len();

        let is_exact_code_span = code_spans.contains(&(abs_pos, end_pos));
        let already_linked =
            abs_pos > 0 && bytes.get(abs_pos - 1) == Some(&LINK_LABEL_OPEN_BYTE);

        let inside_link = link_ranges
            .iter()
            .any(|&(start, end)| abs_pos >= start && end_pos <= end);

        if !is_exact_code_span || already_linked || inside_link {
            result.push_str(&line[search_start..end_pos]);
        } else {
            result.push_str(&line[search_start..abs_pos]);
            result.push_str(linked_pattern);
        }
        search_start = end_pos;
    }
    result.push_str(&line[search_start..]);
    result
}

/// Upgrades plain text `Term` to `` [`Term`] `` when appearing as a whole word outside of
/// code spans, existing links, or qualified Rust paths (`tokio::...`).
fn upgrade_plain_word_term_in_line(
    line: &str,
    term: &str,
    linked_pattern: &str,
) -> String {
    let code_spans = find_inline_code_spans(line);
    let link_ranges = find_md_link_ranges(line);
    let mut final_result = String::new();
    let mut search_start = 0;
    let line_bytes = line.as_bytes();

    while let Some(pos) = line[search_start..].find(term) {
        let abs_pos = search_start + pos;
        let end_pos = abs_pos + term.len();

        // Whole-word check: not part of a longer identifier or qualified path.
        // Treats `::` as a word continuation so `tokio::mpsc`, `vte::Parser`,
        // and `tokio::io::stdin()` are not split by linkifying a path segment.
        let prev_is_qualified_path = line[..abs_pos].ends_with(RUST_PATH_SEPARATOR);
        let prev_is_word = (abs_pos > 0
            && line_bytes.get(abs_pos - 1).is_some_and(|&b| {
                b.is_ascii_alphanumeric() || b == UNDERSCORE_BYTE || b == BACKTICK_BYTE
            }))
            || prev_is_qualified_path;
        let next_is_qualified_path = line[end_pos..].starts_with(RUST_PATH_SEPARATOR);
        let next_is_word = line_bytes.get(end_pos).is_some_and(|&b| {
            b.is_ascii_alphanumeric() || b == UNDERSCORE_BYTE || b == BACKTICK_BYTE
        }) || next_is_qualified_path;

        let is_inside_code_span = code_spans
            .iter()
            .any(|&(span_start, span_end)| abs_pos > span_start && end_pos < span_end);

        let is_inside_link = link_ranges
            .iter()
            .any(|&(start, end)| abs_pos >= start && end_pos <= end);

        let is_part_of_word = prev_is_word || next_is_word;

        if is_inside_code_span || is_part_of_word || is_inside_link {
            final_result.push_str(&line[search_start..end_pos]);
        } else {
            final_result.push_str(&line[search_start..abs_pos]);
            final_result.push_str(linked_pattern);
        }
        search_start = end_pos;
    }
    final_result.push_str(&line[search_start..]);

    final_result
}

/// Upgrades a compound term (one that contains backticks) in a line.
///
/// For a term like `` [`CSI` spec] ``, finds that exact text and wraps it
/// with brackets: `[`[`CSI`]` spec]`. Skips occurrences already inside brackets,
/// enclosing multi-backtick code spans, or existing Markdown links.
///
/// [`CSI` spec]: https://en.wikipedia.org/wiki/ANSI_escape_code#CSI
/// [`CSI`]: crate::CsiSequence
fn upgrade_compound_term_in_line(line: &str, term: &str) -> String {
    let linked_pattern = format!("{LINK_LABEL_OPEN}{term}{LINK_LABEL_CLOSE}");
    let code_spans = find_inline_code_spans(line);
    let link_ranges = find_md_link_ranges(line);

    let mut result = String::new();
    let mut search_start = 0;
    let bytes = line.as_bytes();

    while let Some(pos) = line[search_start..].find(term) {
        let abs_pos = search_start + pos;
        let end_pos = abs_pos + term.len();

        let already_linked =
            abs_pos > 0 && bytes.get(abs_pos - 1) == Some(&LINK_LABEL_OPEN_BYTE);
        let inside_enclosing_code_span =
            code_spans.iter().any(|&(s, e)| abs_pos > s && end_pos < e);
        let inside_link = link_ranges
            .iter()
            .any(|&(s, e)| abs_pos >= s && end_pos <= e);

        if already_linked || inside_enclosing_code_span || inside_link {
            result.push_str(&line[search_start..end_pos]);
        } else {
            result.push_str(&line[search_start..abs_pos]);
            result.push_str(&linked_pattern);
        }
        search_start = end_pos;
    }
    result.push_str(&line[search_start..]);

    result
}

#[cfg(test)]
mod tests {
    use crate::cargo_rustdoc_fmt::formatters::{TechnicalTermDictionary,
                                               link_known_terms_for_test};

    fn test_registry() -> TechnicalTermDictionary {
        TechnicalTermDictionary::from_seed(None).unwrap()
    }

    #[test]
    fn test_plain_text_upgraded_to_linked() {
        let registry = test_registry();
        let input = "Parses CSI sequences from the input.";
        let result = link_known_terms_for_test(input, &registry);
        assert!(result.contains("[`CSI`]"), "Expected [`CSI`] in: {result}");
        assert!(
            result.contains("[`CSI`]: crate::CsiSequence"),
            "Expected ref def in: {result}"
        );
    }

    #[test]
    fn test_backticked_upgraded_to_linked() {
        let registry = test_registry();
        let input = "Parses `CSI` sequences from the input.";
        let result = link_known_terms_for_test(input, &registry);
        assert!(result.contains("[`CSI`]"), "Expected [`CSI`] in: {result}");
        assert!(
            !result.contains("``CSI``"),
            "Should not double-backtick: {result}"
        );
    }

    #[test]
    fn test_already_linked_preserved() {
        let registry = test_registry();
        let input = "Parses [`CSI`] sequences.\n\n[`CSI`]: crate::CsiSequence";
        let result = link_known_terms_for_test(input, &registry);
        let content_links =
            result.matches("[`CSI`]").count() - result.matches("[`CSI`]: ").count();
        assert_eq!(content_links, 1, "Expected 1 content link in: {result}");
    }

    #[test]
    fn test_multiple_occurrences_all_linked() {
        let registry = test_registry();
        let input = "The CSI sequence and another CSI usage.";
        let result = link_known_terms_for_test(input, &registry);
        let content_links =
            result.matches("[`CSI`]").count() - result.matches("[`CSI`]: ").count();
        assert_eq!(content_links, 2, "Expected 2 content links in: {result}");
    }

    #[test]
    fn test_tier2_external_link() {
        let registry = test_registry();
        let input = "Uses ANSI escape codes.";
        let result = link_known_terms_for_test(input, &registry);
        assert!(
            result.contains("[`ANSI`]"),
            "Expected [`ANSI`] in: {result}"
        );
        assert!(
            result.contains("[`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code"),
            "Expected external URL ref def in: {result}"
        );
    }

    #[test]
    fn test_overlapping_terms_longest_first() {
        let registry = test_registry();
        let input = "See the `VT-100` spec for details.";
        let result = link_known_terms_for_test(input, &registry);
        assert!(
            result.contains("[`VT-100` spec]"),
            "Expected longest match in: {result}"
        );
    }

    #[test]
    fn test_inline_code_span_not_modified() {
        let registry = test_registry();
        let input = "The `CSI [ 38 ; 5 ; n m` sequence sets color.";
        let result = link_known_terms_for_test(input, &registry);
        assert!(
            result.contains("`CSI [ 38 ; 5 ; n m`"),
            "Inline code span should be preserved: {result}"
        );
    }

    #[test]
    fn test_term_deep_inside_code_span_not_modified() {
        let registry = test_registry();
        let input = "Run with: `cargo test -p CSI --lib foo -- --nocapture`";
        let result = link_known_terms_for_test(input, &registry);
        assert!(
            result.contains("`cargo test -p CSI --lib foo -- --nocapture`"),
            "Term deep inside code span should be preserved: {result}"
        );
        assert!(
            !result.contains("[`CSI`]"),
            "Should not linkify term inside code span: {result}"
        );
    }

    #[test]
    fn test_term_inside_md_link_text_not_modified() {
        let registry = test_registry();
        let input = "See [Linux TTY and CSI codes](https://example.com) for details.";
        let result = link_known_terms_for_test(input, &registry);
        assert!(
            result.contains("[Linux TTY and CSI codes](https://example.com)"),
            "Term inside link text should not be modified: {result}"
        );
    }

    #[test]
    fn test_term_inside_ref_link_text_not_modified() {
        let registry = test_registry();
        let input = "See [CSI overview][ref] for details.";
        let result = link_known_terms_for_test(input, &registry);
        assert!(
            result.contains("[CSI overview][ref]"),
            "Term inside reference link text should not be modified: {result}"
        );
    }

    #[test]
    fn test_term_inside_shortcut_ref_link_not_modified() {
        let registry = test_registry();
        let input = concat!(
            "- [Linux CSI and async Rust - Article on developerlife.com]\n",
            "\n",
            "[Linux CSI and async Rust - Article on developerlife.com]: https://example.com",
        );
        let result = link_known_terms_for_test(input, &registry);
        assert!(
            result
                .contains("- [Linux CSI and async Rust - Article on developerlife.com]"),
            "Term inside shortcut reference link should not be modified: {result}"
        );
    }

    #[test]
    fn test_qualified_path_not_split() {
        let registry = test_registry();
        let input = "Reads from tokio::io::stdin() in a loop.";
        let result = link_known_terms_for_test(input, &registry);
        assert!(
            result.contains("tokio::io::stdin()"),
            "Qualified path should not be split: {result}"
        );
        assert!(
            !result.contains("[`tokio`]"),
            "Should not linkify prefix of qualified path: {result}"
        );
    }

    #[test]
    fn test_standalone_term_still_linked() {
        let registry = test_registry();
        let input = "Uses tokio for async runtime.";
        let result = link_known_terms_for_test(input, &registry);
        assert!(
            result.contains("[`tokio`]"),
            "Standalone term should be linked: {result}"
        );
    }

    #[test]
    fn test_term_followed_by_single_colon_still_linked() {
        let registry = test_registry();
        let input = "When first byte is not ESC:";
        let result = link_known_terms_for_test(input, &registry);
        assert!(
            result.contains("[`ESC`]"),
            "Term followed by single colon should be linked: {result}"
        );
    }

    #[test]
    fn test_term_inside_autolink_url_not_modified() {
        let registry = test_registry();
        let input = "- <https://unix.stackexchange.com/questions/597083/how-does-piping-affect-stdin>";
        let result = link_known_terms_for_test(input, &registry);
        assert_eq!(
            result, input,
            "Term inside autolink URL should not be linkified: {result}"
        );
    }

    #[test]
    fn test_compound_shortcut_link_starting_with_backtick_not_corrupted() {
        let registry = test_registry();
        let input = concat!(
            "Controls debug logging for the [`VT-100` PTY output parser].\n",
            "\n",
            "[`VT-100` PTY output parser]: crate::core::ansi::vt_100_pty_output_parser",
        );
        let result = link_known_terms_for_test(input, &registry);
        assert_eq!(
            result, input,
            "Compound shortcut link starting with backtick must not have interior terms linkified: {result}"
        );
    }

    #[test]
    fn test_image_alt_text_and_url_with_terms_not_corrupted() {
        let registry = test_registry();
        let input = "Diagram: ![termios and stdin diagram](https://example.com/stdin.png) and ![`CSI`](https://example.com/csi.png).";
        let result = link_known_terms_for_test(input, &registry);
        assert_eq!(
            result, input,
            "Image alt text and URL containing known terms must not be linkified or emit spurious ref defs: {result}"
        );
    }

    #[test]
    fn test_anchor_link_and_full_ref_link_with_terms_not_corrupted() {
        let registry = test_registry();
        let input = "See [the stdin section](#stdin-handling) and [custom label][stdin] for details.";
        let result = link_known_terms_for_test(input, &registry);
        assert_eq!(
            result, input,
            "Inline anchor links and full reference links containing known terms must not be corrupted: {result}"
        );
    }

    #[test]
    fn test_double_backtick_enclosing_backticked_term_not_corrupted() {
        let registry = test_registry();
        let input = "In Markdown, write `` `CSI` `` or `` [`CSI`] `` or `` `CSI` spec `` as code examples.";
        let result = link_known_terms_for_test(input, &registry);
        assert_eq!(
            result, input,
            "Terms inside double-backtick code spans must not be modified or emit spurious ref defs: {result}"
        );
    }
}
