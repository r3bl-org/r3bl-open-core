// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// rustdoc-fmt: skip

//! Scanners for protected Markdown spans (code spans, links, images, URLs, and
//! multi-line span states) where technical terms must not be modified.

use crate::cargo_rustdoc_fmt::{constants::{AUTOLINK_CLOSE_BYTE, AUTOLINK_OPEN_BYTE,
                                           BACKTICK_BYTE, IMAGE_LINK_PREFIX_BYTE,
                                           LINK_LABEL_CLOSE_BYTE, LINK_LABEL_OPEN_BYTE,
                                           LINK_TARGET_CLOSE_BYTE,
                                           LINK_TARGET_OPEN_BYTE, URL_SCHEMES},
                               cst::find_backtick_span};

/// Scans a line for an unclosed backtick opening sequence.
///
/// Matches backtick pairs as [`find_inline_code_spans`] does. If an opening sequence
/// has no matching close on the same line, returns `Some(tick_count)`.
pub(super) fn find_unclosed_backtick_opening(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i] != BACKTICK_BYTE {
            i += 1;
            continue;
        }

        match find_backtick_span(line, i) {
            (_, Some(end)) => i = end,
            (tick_count, None) => return Some(tick_count),
        }
    }

    None
}

/// Checks if a line contains a closing backtick sequence of exactly `tick_count`
/// backticks.
pub(super) fn has_closing_backtick_sequence(line: &str, tick_count: usize) -> bool {
    let bytes = line.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i] != BACKTICK_BYTE {
            i += 1;
            continue;
        }

        let start = i;
        while i < len && bytes[i] == BACKTICK_BYTE {
            i += 1;
        }

        if i - start == tick_count {
            return true;
        }
    }

    false
}

/// Tracks multi-line bracketed Markdown links (`[text spanning\n multiple lines]`)
/// across lines.
///
/// Given the `pending_bracket_depth` entering `line`, returns
/// `(new_pending_bracket_depth, leading_link_end)`, where `leading_link_end` is the
/// byte offset on `line` where an open multi-line link from a previous line closes
/// (or `line.len()` if it remains open across the entire `line`, or `0` if
/// `pending_bracket_depth` was `0`).
pub(super) fn advance_multiline_bracket_state(
    line: &str,
    mut depth: usize,
) -> (usize, usize) {
    let code_spans = find_inline_code_spans(line);
    let bytes = line.as_bytes();
    let len = bytes.len();
    let mut leading_link_end = if depth > 0 { len } else { 0 };
    let mut i = 0;

    while i < len {
        if code_spans.iter().any(|&(start, end)| i >= start && i < end) {
            i += 1;
            continue;
        }
        match bytes[i] {
            LINK_LABEL_OPEN_BYTE => {
                // Ignore single backticked [`Term`] when opening at depth 0 (unless image
                // ![`Term`]).
                let is_image = i > 0 && bytes[i - 1] == IMAGE_LINK_PREFIX_BYTE;
                let is_single_backticked_term = code_spans.iter().any(|&(s, e)| {
                    s == i + 1 && bytes.get(e) == Some(&LINK_LABEL_CLOSE_BYTE)
                });
                if depth > 0 || is_image || !is_single_backticked_term {
                    depth += 1;
                }
            }
            LINK_LABEL_CLOSE_BYTE if depth > 0 => {
                depth -= 1;
                if depth == 0 && leading_link_end == len {
                    let mut end = i + 1;
                    if bytes.get(end) == Some(&LINK_TARGET_OPEN_BYTE) {
                        let mut paren_depth = 0;
                        while end < len {
                            match bytes[end] {
                                LINK_TARGET_OPEN_BYTE => paren_depth += 1,
                                LINK_TARGET_CLOSE_BYTE => {
                                    paren_depth -= 1;
                                    if paren_depth == 0 {
                                        end += 1;
                                        break;
                                    }
                                }
                                _ => {}
                            }
                            end += 1;
                        }
                    } else if bytes.get(end) == Some(&LINK_LABEL_OPEN_BYTE) {
                        end += 1;
                        while end < len && bytes[end] != LINK_LABEL_CLOSE_BYTE {
                            end += 1;
                        }
                        if end < len {
                            end += 1;
                        }
                    }
                    leading_link_end = end;
                }
            }
            _ => {}
        }
        i += 1;
    }

    (depth, leading_link_end)
}

/// Finds all Markdown link text and URL byte ranges in a line.
///
/// Locates:
/// - The full span of `[text](url)`, `![alt](url)`, `[text][ref]`, `![alt][ref]`, and
///   `[text]` patterns
/// - Unclosed `[text...` openings at the end of a line (start of a multi-line link)
/// - Inline/autolink URLs (`http://...`, `https://...`, `<http://...>`, `<https://...>`)
pub(super) fn find_md_link_ranges(line: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let code_spans = find_inline_code_spans(line);
    let bytes = line.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i] != LINK_LABEL_OPEN_BYTE
            || code_spans.iter().any(|&(start, end)| i >= start && i < end)
        {
            i += 1;
            continue;
        }

        let bracket_start = i;
        let is_image =
            bracket_start > 0 && bytes[bracket_start - 1] == IMAGE_LINK_PREFIX_BYTE;

        // Find matching closing bracket, accounting for nested brackets.
        let mut depth = 0;
        let mut close = None;
        let mut j = i;
        while j < len {
            if code_spans.iter().any(|&(start, end)| j >= start && j < end) {
                j += 1;
                continue;
            }
            match bytes[j] {
                LINK_LABEL_OPEN_BYTE => depth += 1,
                LINK_LABEL_CLOSE_BYTE => {
                    depth -= 1;
                    if depth == 0 {
                        close = Some(j);
                        break;
                    }
                }
                _ => {}
            }
            j += 1;
        }

        let Some(close_pos) = close else {
            // Unclosed bracket at end of line (start of a multi-line link).
            if bracket_start + 1 < len
                && (bracket_start == 0
                    || bytes[bracket_start - 1] != LINK_LABEL_CLOSE_BYTE)
            {
                ranges.push((bracket_start, len));
            }
            break;
        };

        let is_single_backticked_term = code_spans
            .iter()
            .any(|&(s, e)| s == bracket_start + 1 && e == close_pos);

        // Check what follows the closing bracket: (url), [ref], or end-of-context
        // (shortcut reference link).
        let after = close_pos + 1;
        if after < len && bytes[after] == LINK_TARGET_OPEN_BYTE {
            // Inline link [text](url) or image ![alt](url) - include the (url) target.
            let mut paren_depth = 0;
            let mut target_end = after;
            while target_end < len {
                match bytes[target_end] {
                    LINK_TARGET_OPEN_BYTE => paren_depth += 1,
                    LINK_TARGET_CLOSE_BYTE => {
                        paren_depth -= 1;
                        if paren_depth == 0 {
                            target_end += 1;
                            break;
                        }
                    }
                    _ => {}
                }
                target_end += 1;
            }
            ranges.push((bracket_start, target_end));
            i = target_end;
            continue;
        } else if after < len && bytes[after] == LINK_LABEL_OPEN_BYTE {
            // Full reference link [text][ref] or image ![alt][ref] - include [ref].
            let mut ref_end = after + 1;
            while ref_end < len && bytes[ref_end] != LINK_LABEL_CLOSE_BYTE {
                ref_end += 1;
            }
            if ref_end < len {
                ref_end += 1;
            }
            ranges.push((bracket_start, ref_end));
            i = ref_end;
            continue;
        } else if bracket_start + 1 < close_pos
            && (!is_single_backticked_term || is_image)
            && (bracket_start == 0 || bytes[bracket_start - 1] != LINK_LABEL_CLOSE_BYTE)
        {
            // Shortcut reference link [text], image ![alt], or compound [`VT-100` PTY
            // output parser] - the bracket text itself is the ref def name. We exclude:
            // - [`Term`] (non-image where entire bracket content is a single code span) -
            //   term references that should remain modifiable.
            // - [ref] immediately after ] - ref label of a full reference link
            //   [text][ref], not a standalone shortcut link.
            ranges.push((bracket_start, close_pos + 1));
        }

        i = close_pos + 1;
    }

    find_inline_url_ranges(line, &code_spans, &mut ranges);
    ranges
}

/// Appends byte ranges of inline/autolink `http://` and `https://` URLs outside of code
/// spans to `ranges`.
fn find_inline_url_ranges(
    line: &str,
    code_spans: &[(usize, usize)],
    ranges: &mut Vec<(usize, usize)>,
) {
    let bytes = line.as_bytes();
    let len = bytes.len();
    for scheme in URL_SCHEMES {
        let mut search_from = 0;
        while let Some(pos) = line[search_from..].find(scheme) {
            let url_pos = search_from + pos;
            if code_spans
                .iter()
                .any(|&(start, end)| url_pos >= start && url_pos < end)
            {
                search_from = url_pos + scheme.len();
                continue;
            }
            let start = if url_pos > 0 && bytes[url_pos - 1] == AUTOLINK_OPEN_BYTE {
                url_pos - 1
            } else {
                url_pos
            };
            let mut end = url_pos + scheme.len();
            while end < len
                && !bytes[end].is_ascii_whitespace()
                && bytes[end] != AUTOLINK_CLOSE_BYTE
                && bytes[end] != LINK_TARGET_CLOSE_BYTE
                && bytes[end] != LINK_LABEL_CLOSE_BYTE
            {
                end += 1;
            }
            if start < url_pos && end < len && bytes[end] == AUTOLINK_CLOSE_BYTE {
                end += 1;
            }
            ranges.push((start, end));
            search_from = end;
        }
    }
}

/// Finds all inline code span byte ranges in a line.
///
/// Follows the [`CommonMark`] spec: a code span opens with a sequence of N backticks and
/// closes at the next sequence of exactly N backticks. Returns ranges as `(start, end)`
/// where `start` is the position of the first opening backtick and `end` is one past the
/// last closing backtick.
///
/// [`CommonMark`]: https://spec.commonmark.org/0.30/#code-spans
pub(super) fn find_inline_code_spans(line: &str) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    let bytes = line.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i] != BACKTICK_BYTE {
            i += 1;
            continue;
        }

        match find_backtick_span(line, i) {
            (_, Some(end)) => {
                spans.push((i, end));
                i = end;
            }
            (tick_count, None) => i += tick_count,
        }
    }

    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_inline_code_spans_single() {
        let spans = find_inline_code_spans("The `foo bar` end");
        assert_eq!(spans, vec![(4, 13)]);
    }

    #[test]
    fn test_find_inline_code_spans_multiple() {
        let spans = find_inline_code_spans("`a` and `b`");
        assert_eq!(spans, vec![(0, 3), (8, 11)]);
    }

    #[test]
    fn test_find_inline_code_spans_double_backtick() {
        let spans = find_inline_code_spans("The ``foo `bar` baz`` end");
        assert_eq!(spans, vec![(4, 21)]);
    }

    #[test]
    fn test_find_inline_code_spans_none() {
        let spans = find_inline_code_spans("plain text with no backticks");
        assert!(spans.is_empty());
    }

    #[test]
    fn test_find_md_link_ranges() {
        let ranges = find_md_link_ranges("See [CSI info](url) here.");
        assert_eq!(ranges, vec![(4, 19)]);
    }

    #[test]
    fn test_find_md_link_ranges_ref_style() {
        let ranges = find_md_link_ranges("See [CSI info][ref] here.");
        assert_eq!(ranges, vec![(4, 19)]);
    }

    #[test]
    fn test_find_md_link_ranges_none() {
        let ranges = find_md_link_ranges("Plain text without links.");
        assert!(ranges.is_empty());
    }
}
