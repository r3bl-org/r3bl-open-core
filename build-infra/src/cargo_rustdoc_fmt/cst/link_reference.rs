// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// rustdoc-fmt: skip

//! Reference link definitions ([`LinkReference`], [`ParsedReferenceDef`]) and shared
//! slice-scanning helpers ([`split_reference_def_label_and_rest`],
//! [`find_backtick_span`]).

use super::extract_comment_content;
use crate::cargo_rustdoc_fmt::constants::{BACKTICK_BYTE, LF_LINE_ENDING,
                                          LINK_LABEL_CLOSE, LINK_LABEL_OPEN,
                                          LINK_REF_COLON_PREFIX, LINK_TITLE_QUOTE};

/// Parsed reference link definition: `[label]: target "title"`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkReference {
    pub label: String,
    pub target: String,
    pub title: Option<String>,
    /// Multi-line content (without `/// ` or `//! ` marker) when `rustfmt` split a long
    /// reference definition across two lines (`[label]:\n    target`). Preserved
    /// verbatim during reconstruction when the target is unchanged.
    pub raw_content: Option<String>,
}

impl LinkReference {
    /// Renders this reference definition into its Markdown definition string (without
    /// comment prefix).
    #[must_use]
    pub fn render(&self) -> String {
        if let Some(raw_content) = &self.raw_content {
            raw_content.clone()
        } else if let Some(title) = &self.title {
            format!(
                "{LINK_LABEL_OPEN}{}{LINK_LABEL_CLOSE}{LINK_REF_COLON_PREFIX} {} {LINK_TITLE_QUOTE}{title}{LINK_TITLE_QUOTE}",
                self.label, self.target
            )
        } else {
            format!(
                "{LINK_LABEL_OPEN}{}{LINK_LABEL_CLOSE}{LINK_REF_COLON_PREFIX} {}",
                self.label, self.target
            )
        }
    }
}

/// Result of parsing a single-line or multi-line reference link definition from a slice
/// of raw doc comment lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedReferenceDef {
    pub definition: LinkReference,
    pub lines_consumed: usize,
}

/// Splits a reference definition line `[label]: rest` into `(label, rest_trimmed)`.
///
/// Balances nested `[` and `]` brackets inside `label` (e.g.,
/// `` [`SmallString<[u8; 64]>`]: target `` or `` [`[`CSI`]` spec]: target ``).
/// Returns `None` if `line` does not start with a non-empty `[label]` followed by `:`.
#[must_use]
pub fn split_reference_def_label_and_rest(line: &str) -> Option<(&str, &str)> {
    let trimmed = line.trim();
    if !trimmed.starts_with(LINK_LABEL_OPEN) {
        return None;
    }

    let mut depth = 0usize;
    let mut close_bracket = None;
    for (byte_idx, ch) in trimmed.char_indices() {
        match ch {
            LINK_LABEL_OPEN => depth += 1,
            LINK_LABEL_CLOSE => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    close_bracket = Some(byte_idx);
                    break;
                }
            }
            _ => {}
        }
    }

    let close_bracket = close_bracket?;
    if close_bracket <= 1 {
        return None;
    }

    let label = &trimmed[1..close_bracket];
    let after_bracket = trimmed[close_bracket + 1..].trim_start();
    let after_colon = after_bracket.strip_prefix(LINK_REF_COLON_PREFIX)?.trim();

    Some((label, after_colon))
}

/// Attempts to parse a reference link definition starting at `raw_lines[index]`.
///
/// Supports:
/// 1. Single-line definitions (`[label]: target` or `[label]: target "title"`), including
///    nested brackets in `label` (e.g., `` [`SmallString<[u8; 64]>`]: target ``).
/// 2. Multi-line definitions split across two lines by `rustfmt` (`[label]:` on one line
///    followed by the indented `target` on the next line).
///
/// Returns `Some(`[`ParsedReferenceDef`]`)` if matched.
#[must_use]
pub fn try_parse_reference_def(
    raw_lines: &[String],
    index: usize,
    marker: &str,
) -> Option<ParsedReferenceDef> {
    if index >= raw_lines.len() {
        return None;
    }

    let content = extract_comment_content(&raw_lines[index], marker);
    let trimmed = content.trim();
    let (label, after_colon) = split_reference_def_label_and_rest(trimmed)?;

    if !after_colon.is_empty() {
        // Case 1: Single-line reference definition (`[label]: target` or `[label]: target
        // "title"`).
        let (target, title) = if let Some((target_part, rest)) =
            after_colon.split_once(char::is_whitespace)
        {
            let rest_trimmed = rest.trim();
            if rest_trimmed.starts_with(LINK_TITLE_QUOTE)
                && rest_trimmed.ends_with(LINK_TITLE_QUOTE)
                && rest_trimmed.len() >= 2
            {
                (
                    target_part.to_string(),
                    Some(rest_trimmed[1..rest_trimmed.len() - 1].to_string()),
                )
            } else {
                // Not a valid reference definition (e.g., prose after `[label]: some
                // sentence`).
                return None;
            }
        } else {
            (after_colon.to_string(), None)
        };

        return Some(ParsedReferenceDef {
            definition: LinkReference {
                label: label.to_string(),
                target,
                title,
                raw_content: None,
            },
            lines_consumed: 1,
        });
    }

    // Case 2: Multi-line reference definition (`[label]:` on this line, target on next
    // line).
    if index + 1 < raw_lines.len() {
        let next_content = extract_comment_content(&raw_lines[index + 1], marker);
        let next_trimmed = next_content.trim();
        if !next_trimmed.is_empty()
            && !next_trimmed.starts_with(LINK_LABEL_OPEN)
            && !next_trimmed.contains(char::is_whitespace)
        {
            let combined = format!("{trimmed}{LF_LINE_ENDING}{next_content}");
            return Some(ParsedReferenceDef {
                definition: LinkReference {
                    label: label.to_string(),
                    target: next_trimmed.to_string(),
                    title: None,
                    raw_content: Some(combined),
                },
                lines_consumed: 2,
            });
        }
    }

    None
}

/// Scans an inline backtick code span opening at `open_start` in `text` following the
/// [`CommonMark`] N-backtick rule.
///
/// Given `open_start` (where `text.as_bytes()[open_start] == BACKTICK_BYTE`), counts the
/// run of `tick_count` opening backticks and searches forward for the next run of
/// *exactly* `tick_count` backticks.
///
/// Returns `(tick_count, matching_close_end)` where `matching_close_end` is `Some(end)`
/// (one byte past the closing backtick sequence) if a matching close was found, or `None`
/// if the opening backtick sequence is unclosed in `text`.
///
/// [`CommonMark`]: https://spec.commonmark.org/0.30/#code-spans
#[must_use]
pub fn find_backtick_span(text: &str, open_start: usize) -> (usize, Option<usize>) {
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut idx = open_start;

    while idx < len && bytes[idx] == BACKTICK_BYTE {
        idx += 1;
    }
    let tick_count = idx - open_start;
    if tick_count == 0 {
        return (0, None);
    }

    let mut scan_idx = idx;
    while scan_idx < len {
        if bytes[scan_idx] != BACKTICK_BYTE {
            scan_idx += 1;
            continue;
        }
        let close_start = scan_idx;
        while scan_idx < len && bytes[scan_idx] == BACKTICK_BYTE {
            scan_idx += 1;
        }
        if scan_idx - close_start == tick_count {
            return (tick_count, Some(scan_idx));
        }
    }

    (tick_count, None)
}
