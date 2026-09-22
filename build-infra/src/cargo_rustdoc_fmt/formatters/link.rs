// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// rustdoc-fmt: skip
#![rustfmt::skip]

//! Convert inline Markdown links to reference-style links.

use crate::cargo_rustdoc_fmt::{
    constants::{
        ANCHOR_LINK_PREFIX, BACKTICK_BYTE, BLOCKQUOTE_PREFIX, IMAGE_LINK_PREFIX_BYTE,
        LF_LINE_ENDING, LF_LINE_ENDING_CHAR, LINK_LABEL_CLOSE, LINK_LABEL_CLOSE_BYTE,
        LINK_LABEL_OPEN, LINK_LABEL_OPEN_BYTE, LINK_REF_COLON_PREFIX,
        LINK_TARGET_CLOSE, LINK_TARGET_OPEN, SINGLE_SPACE_STR,
    },
    cst::{DocBlockCst, DocNode, LinkReference, find_backtick_span},
    types::FormatStatus,
};
use r3bl_tui::{CRow, RangeBoundsExt, RangeOverlapResult};
use rustc_hash::FxHashSet;
use std::{collections::BTreeMap, fmt::Write as _, ops::RangeInclusive};

/// Result of parsing an inline Markdown link `[link_text](url)` from the start of a
/// string slice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ParsedInlineLink<'a> {
    link_text: &'a str,
    url: &'a str,
    consumed_len: usize,
}

/// Attempts to parse an inline Markdown link `[link_text](url)` at the start of `input`.
///
/// - Inside `[link_text]`: skips over balanced backtick code spans via
///   [`find_backtick_span`] so brackets inside backticks (such as
///   `` [`SmallString<[u8; 64]>`](url) ``) are allowed, while unbackticked nested `[`
///   characters are rejected to prevent cross-line matching across escape sequences.
/// - Inside `(url)`: supports arbitrary balanced parentheses (such as
///   `EditorBuffer::get_lines()`) and rejects whitespace.
fn try_parse_inline_link(input: &str) -> Option<ParsedInlineLink<'_>> {
    let after_open_bracket = input.strip_prefix(LINK_LABEL_OPEN)?;
    let (link_text, after_close_bracket) = split_link_label(after_open_bracket)?;
    let after_open_paren = after_close_bracket.strip_prefix(LINK_TARGET_OPEN)?;
    let (url, remaining) = split_link_url(after_open_paren)?;

    Some(ParsedInlineLink {
        link_text,
        url,
        consumed_len: input.len() - remaining.len(),
    })
}

/// Scans `input` (immediately after an opening `[`) up to the matching `]`, skipping
/// over balanced backtick code spans and rejecting unbackticked nested `[` or empty
/// labels.
///
/// Returns `Some((link_text, remaining_after_close_bracket))`.
fn split_link_label(input: &str) -> Option<(&str, &str)> {
    let bytes = input.as_bytes();
    let mut idx = 0;
    while idx < bytes.len() {
        match bytes[idx] {
            BACKTICK_BYTE => {
                let (tick_count, close_end) = find_backtick_span(input, idx);
                idx = close_end.unwrap_or(idx + tick_count);
            }
            LINK_LABEL_OPEN_BYTE => return None,
            LINK_LABEL_CLOSE_BYTE => {
                let (link_text, rest) = input.split_at(idx);
                let remaining = rest.strip_prefix(LINK_LABEL_CLOSE)?;
                return (!link_text.is_empty()).then_some((link_text, remaining));
            }
            _ => idx += 1,
        }
    }
    None
}

/// Scans `input` (immediately after an opening `(`) up to the balanced closing `)`,
/// allowing nested parentheses (such as `EditorBuffer::get_lines()`) and rejecting
/// whitespace.
///
/// Returns `Some((url, remaining_after_close_paren))`.
fn split_link_url(input: &str) -> Option<(&str, &str)> {
    let mut paren_depth: usize = 0;
    for (idx, ch) in input.char_indices() {
        match ch {
            ch if ch.is_ascii_whitespace() => return None,
            LINK_TARGET_OPEN => paren_depth += 1,
            LINK_TARGET_CLOSE if paren_depth == 0 => {
                let (url, rest) = input.split_at(idx);
                let remaining = rest.strip_prefix(LINK_TARGET_CLOSE)?;
                return Some((url, remaining));
            }
            LINK_TARGET_CLOSE => paren_depth -= 1,
            _ => {}
        }
    }
    None
}

/// Converts inline Markdown links to reference-style links in a snippet of text.
///
/// Inline backtick spans are preserved untouched.
/// Returns `(converted_text, extracted_links)` where `extracted_links` is
/// `Vec<(link_text, url)>`.
fn convert_inline_links(text: &str) -> (String, Vec<(String, String)>) {
    if text.is_empty() {
        return (String::new(), Vec::new());
    }

    let mut link_info: Vec<(String, String)> = Vec::new();
    let mut seen_urls: FxHashSet<String> = FxHashSet::default();
    let converted = convert_inline_links_segment(text, &mut seen_urls, &mut link_info);
    (converted, link_info)
}

fn convert_inline_links_segment(
    text: &str,
    seen_urls: &mut FxHashSet<String>,
    link_info: &mut Vec<(String, String)>,
) -> String {
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut result = String::with_capacity(len);
    let mut segment_has_links = false;
    let mut byte_idx = 0;

    while byte_idx < len {
        match bytes[byte_idx] {
            BACKTICK_BYTE => {
                let (tick_count, close_end) = find_backtick_span(text, byte_idx);
                let end = close_end.unwrap_or(byte_idx + tick_count);
                result.push_str(&text[byte_idx..end]);
                byte_idx = end;
            }
            LINK_LABEL_OPEN_BYTE => {
                let is_image = byte_idx > 0 && bytes[byte_idx - 1] == IMAGE_LINK_PREFIX_BYTE;
                if let Some(parsed) = try_parse_inline_link(&text[byte_idx..]) {
                    let end = byte_idx + parsed.consumed_len;
                    if is_image || parsed.url.starts_with(ANCHOR_LINK_PREFIX) {
                        result.push_str(&text[byte_idx..end]);
                    } else {
                        segment_has_links = true;
                        let _ = write!(
                            result,
                            "{LINK_LABEL_OPEN}{}{LINK_LABEL_CLOSE}",
                            parsed.link_text
                        );
                        if seen_urls.insert(parsed.url.to_string()) {
                            link_info.push((
                                normalize_multiline_link_label(parsed.link_text),
                                parsed.url.to_string(),
                            ));
                        }
                    }
                    byte_idx = end;
                } else {
                    result.push(LINK_LABEL_OPEN);
                    byte_idx += 1;
                }
            }
            _ => {
                let start = byte_idx;
                byte_idx += 1;
                while byte_idx < len
                    && bytes[byte_idx] != BACKTICK_BYTE
                    && bytes[byte_idx] != LINK_LABEL_OPEN_BYTE
                {
                    byte_idx += 1;
                }
                result.push_str(&text[start..byte_idx]);
            }
        }
    }

    if !segment_has_links {
        return text.to_string();
    }

    result
}

/// Normalizes a multi-line link text into a single-line reference definition label.
///
/// When an inline link spans multiple lines in a paragraph or blockquote, e.g.:
/// ```text
/// /// > see the [coordinates module
/// /// > documentation](crate::coordinates).
/// ```
/// the in-place shortcut link in the paragraph remains wrapped across lines, while the
/// reference definition label at the bottom of the doc block must be a single line
/// (`[coordinates module documentation]: crate::coordinates`) with continuation
/// indentation and blockquote (`>`) prefixes stripped.
fn normalize_multiline_link_label(label: &str) -> String {
    if !label.contains(LF_LINE_ENDING_CHAR) {
        return label.to_string();
    }
    label
        .lines()
        .enumerate()
        .map(|(idx, line)| {
            let trimmed = line.trim();
            if idx > 0 {
                trimmed.trim_start_matches(BLOCKQUOTE_PREFIX).trim()
            } else {
                trimmed
            }
        })
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join(SINGLE_SPACE_STR)
}

/// Converts inline links in a [`DocBlockCst`] across [`DocNode::Paragraph`] and
/// [`DocNode::Table`] nodes, and aggregates [`DocNode::ReferenceDefinitions`] at the
/// bottom of the block.
///
/// Collects all newly converted links and existing reference definitions,
/// deduplicates and sorts them alphabetically, and places them in a
/// [`DocNode::ReferenceDefinitions`] at the bottom of the block.
///
/// Code fences are structurally isolated in their own `CST` nodes and never modified.
/// Returns [`FormatStatus::Modified`] if the block was modified, or
/// [`FormatStatus::Unchanged`] otherwise.
pub fn convert_links_in_doc_block(
    block: &mut DocBlockCst,
    line_range: Option<&RangeInclusive<CRow>>,
) -> FormatStatus {
    let mut modified = false;
    let mut new_refs: Vec<(String, String)> = Vec::new();

    for node in &mut block.nodes {
        match node {
            DocNode::Paragraph {
                lines,
                raw_lines: _,
                status: p_status,
                span,
            } => {
                if let Some(range) = line_range
                    && span.overlaps(range) == RangeOverlapResult::Disjoint
                {
                    continue;
                }

                let para_text = lines.join(LF_LINE_ENDING);
                let (converted, links) = convert_inline_links(&para_text);
                if !links.is_empty() {
                    new_refs.extend(links);
                    *lines = converted.lines().map(String::from).collect();
                    *p_status = FormatStatus::Modified;
                    modified = true;
                }
            }
            DocNode::Table {
                table,
                status: t_status,
                span,
                ..
            } => {
                if let Some(range) = line_range
                    && span.overlaps(range) == RangeOverlapResult::Disjoint
                {
                    continue;
                }

                for cell in &mut table.headers {
                    let (converted, links) = convert_inline_links(cell);
                    if !links.is_empty() {
                        new_refs.extend(links);
                        *cell = converted;
                        *t_status = FormatStatus::Modified;
                        modified = true;
                    }
                }
                for row in &mut table.rows {
                    for cell in row {
                        let (converted, links) = convert_inline_links(cell);
                        if !links.is_empty() {
                            new_refs.extend(links);
                            *cell = converted;
                            *t_status = FormatStatus::Modified;
                            modified = true;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // Collect pre-existing reference definitions from the block.
    let mut existing_refs = Vec::new();
    let mut ref_node_count = 0;
    for node in &block.nodes {
        if let DocNode::ReferenceDefinitions { definitions, .. } = node {
            ref_node_count += 1;
            for def in definitions {
                existing_refs.push(def.clone());
            }
        }
    }

    // Check if existing reference definitions need aggregation or sorting even when
    // no new inline links were converted (in full-block mode).
    let refs_need_aggregation =
        existing_refs_need_aggregation(block, &existing_refs, ref_node_count, line_range);

    if !new_refs.is_empty() || (ref_node_count > 0 && modified) || refs_need_aggregation {
        aggregate_and_append_references(block, existing_refs, new_refs);
        modified = true;
    }

    if modified {
        FormatStatus::Modified
    } else {
        FormatStatus::Unchanged
    }
}

/// Checks whether existing [`DocNode::ReferenceDefinitions`] in `block` need to be
/// aggregated, repositioned to the bottom of the block, or sorted alphabetically.
fn existing_refs_need_aggregation(
    block: &DocBlockCst,
    existing_refs: &[LinkReference],
    ref_node_count: usize,
    line_range: Option<&RangeInclusive<CRow>>,
) -> bool {
    line_range.is_none()
        && ref_node_count > 0
        && (!block.has_canonical_reference_layout(ref_node_count, !existing_refs.is_empty())
            || !(existing_refs.is_sorted_by(|left, right| left.label <= right.label)
                || existing_refs.is_sorted_by(|left, right| {
                    format!("{LINK_LABEL_OPEN}{}{LINK_LABEL_CLOSE}{LINK_REF_COLON_PREFIX} {}", left.label, left.target).to_lowercase()
                        <= format!("{LINK_LABEL_OPEN}{}{LINK_LABEL_CLOSE}{LINK_REF_COLON_PREFIX} {}", right.label, right.target).to_lowercase()
                })))
}

/// Deduplicates, sorts, and places all reference definitions into a single
/// [`DocNode::ReferenceDefinitions`] node at the bottom of `block`.
fn aggregate_and_append_references(
    block: &mut DocBlockCst,
    existing_refs: Vec<LinkReference>,
    new_refs: Vec<(String, String)>,
) {
    let mut ref_map: BTreeMap<String, LinkReference> = BTreeMap::new();

    for def in existing_refs {
        ref_map.insert(def.label.clone(), def);
    }

    for (label, target) in new_refs {
        ref_map.entry(label.clone()).or_insert(LinkReference {
            label,
            target,
            title: None,
            raw_content: None,
        });
    }

    let sorted_defs: Vec<LinkReference> = ref_map.into_values().collect();
    block.set_reference_definitions(sorted_defs);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cargo_rustdoc_fmt::{
        constants::{CONTENT_SPACE_CHAR, OUTER_DOC_MARKER},
        cst::{FileChunk, SourceFileCst, extract_comment_content},
    };

    /// Test helper: wraps raw Markdown lines in `/// ` doc comments, runs
    /// [`convert_links_in_doc_block`] on the parsed [`DocBlockCst`], and strips the
    /// `/// ` prefix from the reconstructed output.
    fn convert_links(text: &str) -> String {
        let raw_doc: String = text
            .lines()
            .map(|line| {
                if line.is_empty() {
                    OUTER_DOC_MARKER.to_string()
                } else {
                    format!("{OUTER_DOC_MARKER}{CONTENT_SPACE_CHAR}{line}")
                }
            })
            .collect::<Vec<_>>()
            .join(LF_LINE_ENDING);
        let mut cst = SourceFileCst::parse(&raw_doc);
        for chunk in &mut cst.chunks {
            if let FileChunk::DocBlock(block) = chunk {
                convert_links_in_doc_block(block, None);
            }
        }
        cst.reconstruct()
            .lines()
            .map(|line| extract_comment_content(line, OUTER_DOC_MARKER))
            .collect::<Vec<_>>()
            .join(LF_LINE_ENDING)
    }

    #[test]
    fn test_single_link_conversion() {
        let input = "See [docs](https://example.com) here.";
        let output = convert_links(input);
        assert!(output.contains("[docs]"));
        assert!(output.contains("[docs]: https://example.com"));
        assert!(!output.contains("[docs]["));
    }

    #[test]
    fn test_multiple_links() {
        let input = "See [docs](https://example.com) and [Rust](https://rust-lang.org).";
        let output = convert_links(input);
        assert!(output.contains("[docs]"));
        assert!(output.contains("[Rust]"));
        assert!(output.contains("[docs]: https://example.com"));
        assert!(output.contains("[Rust]: https://rust-lang.org"));
    }

    #[test]
    fn test_duplicate_urls() {
        let input =
            "See [docs](https://example.com) and [more docs](https://example.com).";
        let output = convert_links(input);
        // First link text becomes the reference
        assert!(output.contains("[docs]"));
        assert!(output.contains("[more docs]"));
        // Should only have one reference definition (using first link text)
        assert_eq!(output.matches("[docs]: https://example.com").count(), 1);
        assert_eq!(output.matches("https://example.com").count(), 1);
    }

    #[test]
    fn test_no_links() {
        assert_eq!(convert_inline_links(""), (String::new(), Vec::new()));
        let input = "This text has no links.";
        let output = convert_links(input);
        assert_eq!(output, input);
    }

    #[test]
    fn test_malformed_inline_links_rejected() {
        let input = "Unbackticked [outer [inner]](https://example.com), empty [](https://example.com), whitespace [label](https://example.com/a b), unclosed [label](https://example.com, unclosed bracket [no_close";
        let (converted, links) = convert_inline_links(input);
        assert!(links.is_empty());
        assert_eq!(converted, input);
        assert_eq!(try_parse_inline_link("not_a_bracket"), None);
    }

    #[test]
    fn test_convert_links_in_doc_block_table_headers_and_cells() {
        let source = "/// | [HeaderLink](https://h.example.com) | Col |\n/// |---|---|\n/// | [CellLink](https://c.example.com) | val |\nfn demo() {}\n";
        let mut cst = SourceFileCst::parse(source);
        if let FileChunk::DocBlock(ref mut block) = cst.chunks[0] {
            assert_eq!(
                convert_links_in_doc_block(block, None),
                FormatStatus::Modified
            );
        }
        let reconstructed = cst.reconstruct();
        assert!(reconstructed.contains("| [HeaderLink] | Col |"));
        assert!(reconstructed.contains("| [CellLink]   | val |"));
        assert!(reconstructed.contains("/// [CellLink]: https://c.example.com"));
        assert!(reconstructed.contains("/// [HeaderLink]: https://h.example.com"));
    }

    #[test]
    fn test_aggregate_scattered_references() {
        let input = "Bla bla [link].\n[link]: crate::Link\n\nBla bla [otherlink].\n[otherlink]: crate::Otherlink";
        let output = convert_links(input);

        // Check that content is preserved
        assert!(output.contains("Bla bla [link]."));
        assert!(output.contains("Bla bla [otherlink]."));

        // Check that references are at the bottom
        let lines: Vec<&str> = output.lines().collect();
        assert!(lines.len() >= 2);
        assert!(lines[lines.len() - 2].contains("[link]: crate::Link"));
        assert!(lines[lines.len() - 1].contains("[otherlink]: crate::Otherlink"));
    }

    #[test]
    fn test_aggregate_alphabetical_sorting() {
        let input = "Text [zebra] and [alpha].\n[zebra]: crate::Z\n[alpha]: crate::A";
        let output = convert_links(input);

        // References should be sorted alphabetically
        let lines: Vec<&str> = output.lines().collect();
        let ref_section = lines
            .iter()
            .skip_while(|line| !line.contains("[alpha]:"))
            .collect::<Vec<_>>();
        assert!(ref_section.len() >= 2);
        assert!(ref_section[0].contains("[alpha]: crate::A"));
        assert!(ref_section[1].contains("[zebra]: crate::Z"));
    }

    #[test]
    fn test_aggregate_blank_line_before_references() {
        let input = "Content [link].\n[link]: target";
        let output = convert_links(input);

        // Should have blank line before references for visual separation
        assert!(output.contains("Content [link].\n\n[link]: target"));
    }

    #[test]
    fn test_aggregate_with_content_after_refs() {
        // Simulates the file where refs are in middle and content + more refs are after.
        let input = r"Some content.

[ref1]: target1
[ref2]: target2

More content here.

[ref3]: target3";
        let output = convert_links(input);

        // All refs should be at the bottom, sorted alphabetically.
        let lines: Vec<&str> = output.lines().collect();
        let last_three: Vec<&str> = lines.iter().rev().take(3).copied().collect();
        assert!(last_three.iter().all(|line| line.starts_with('[')));

        // Content should be before refs (with some blank lines from removed refs).
        assert!(output.contains("Some content."));
        assert!(output.contains("More content here."));
    }

    #[test]
    fn test_aggregate_with_code_fence_and_cst() {
        let input = r#"/// Content before.
///
/// ```rust,ignore
/// // Code here: [code_not_link](https://example.com)
/// fn example() {}
/// ```
///
/// ## References
///
/// - [mio issue #1377] - "Polling"
///
/// [ref1]: target1
/// [ref2]: target2
///
/// # Entry Point
///
/// More content: [regular_link](https://example.com/regular).
///
/// [ref3]: target3"#;

        let mut cst = SourceFileCst::parse(input);

        if let FileChunk::DocBlock(ref mut block) = cst.chunks[0] {
            let status = convert_links_in_doc_block(block, None);
            assert_eq!(status, FormatStatus::Modified);
        }

        let reconstructed = cst.reconstruct();
        // Code fence must NOT have its link converted.
        assert!(reconstructed.contains("[code_not_link](https://example.com)"));
        // Regular link must have been converted.
        assert!(reconstructed.contains("[regular_link]"));
        // All references must be at the bottom, sorted.
        assert!(reconstructed.contains("[ref1]: target1"));
        assert!(reconstructed.contains("[ref2]: target2"));
        assert!(reconstructed.contains("[ref3]: target3"));
        assert!(reconstructed.contains("[regular_link]: https://example.com/regular"));

        // Also verify convert_links preserves links and ref-like lines in code fences.
        let raw_block = "Before [real](https://example.com/real).\n```rust\n// [code_link](https://example.com/code)\n[not_ref]: u32\n```";
        let direct = convert_links(raw_block);
        assert!(direct.contains("[code_link](https://example.com/code)"));
        assert!(direct.contains("\n[not_ref]: u32\n```"));
        assert!(direct.contains("[real]: https://example.com/real"));
    }

    #[test]
    fn test_multiline_content_preserved() {
        // Content that spans multiple lines (including numbered lists and indentation)
        // should be preserved exactly.
        let input = r"Some text with a [link](https://example.com) that
continues on the next line without breaking.

1. **Item one** — description with a [link](https://example.com/one)
   that continues on the next line.

2. **Item two** — another description.";
        let output = convert_links(input);

        // Line structure should be preserved
        assert!(output.contains("that\ncontinues"), "Newlines in text should be preserved");
        assert!(output.contains("1. **Item one**"), "Numbered list should be preserved");
        assert!(output.contains("2. **Item two**"), "Numbered list should be preserved");
        assert!(output.contains("   that continues"), "Indentation should be preserved");
    }

    #[test]
    fn test_no_cross_line_matching_with_bracket_escape_sequences() {
        // Regression test: `[200~` in escape sequence text must not match a `]`
        // many lines later, causing content duplication. `split_link_label` skips
        // balanced backtick spans and rejects unbackticked nested `[` characters.
        let input = concat!(
            "- **How it works**: Terminal sends pasted text wrapped in escape\n",
            "  sequences (`ESC[200~` text `ESC[201~`)\n",
            "- **Characteristics**: Text arrives as a single chunk\n",
            "\n",
            "Note: Bracketed paste must be enabled via\n",
            "[`EnableBracketedPaste`](crate::PaintRenderOpImplCrossterm::raw_mode_enter) in raw\n",
            "mode.",
        );
        let output = convert_links(input);

        // The inline link should be converted to reference-style
        assert!(
            output.contains("[`EnableBracketedPaste`]"),
            "Inline link should be converted to reference-style"
        );
        assert!(
            output.contains("[`EnableBracketedPaste`]: crate::PaintRenderOpImplCrossterm::raw_mode_enter"),
            "Reference definition should be appended"
        );

        // The escape sequence line must NOT be duplicated or mangled
        assert!(
            output.contains("sequences (`ESC[200~` text `ESC[201~`)"),
            "Escape sequence line should remain intact"
        );

        // Content should appear exactly once (no duplication)
        assert_eq!(
            output.matches("Characteristics").count(),
            1,
            "Content must not be duplicated"
        );
    }

    #[test]
    fn test_multiline_reference_definition_preserved() {
        let input = "See [Registered buffers] for details.\n\n[Registered buffers]:\n    https://manpages.debian.org/io_uring_register_buffers.3";
        let output = convert_links(input);

        // Multi-line reference should be preserved as two lines.
        assert!(
            output.contains("[Registered buffers]:\n    https://manpages.debian.org/io_uring_register_buffers.3"),
            "Multi-line reference should be preserved as two lines"
        );
        // URL should not be orphaned as content.
        assert_eq!(
            output.matches("https://manpages.debian.org/io_uring_register_buffers.3").count(),
            1,
            "URL should appear exactly once"
        );
    }

    #[test]
    fn test_mixed_single_and_multiline_references() {
        let input = "Text [a] and [b].\n\n[a]: https://example.com/a\n[b]:\n    https://example.com/b";
        let output = convert_links(input);

        assert!(output.contains("[a]: https://example.com/a"));
        assert!(output.contains("[b]:\n    https://example.com/b"));
    }

    #[test]
    fn test_inline_link_inside_backticks_not_converted() {
        // Regression test: `[link](http://r3bl.com)` inside backticks must NOT be
        // converted to a reference-style link. The backtick span is inline code.
        let input = "eg: `**bold**`, `*italic*`, `[link](http://r3bl.com)`, etc.";
        let output = convert_links(input);

        assert_eq!(
            output, input,
            "Inline link inside backticks should not be modified"
        );
    }

    #[test]
    fn test_mixed_backtick_and_real_inline_links() {
        // Backtick-wrapped links should be preserved, real inline links converted.
        let input = "See `[example](http://example.com)` and [docs](https://docs.rs).";
        let output = convert_links(input);

        // Backtick-wrapped link should be untouched.
        assert!(
            output.contains("`[example](http://example.com)`"),
            "Backtick-wrapped link should be preserved"
        );
        // Real inline link should be converted to reference-style.
        assert!(
            output.contains("[docs]"),
            "Real inline link should be converted"
        );
        assert!(
            output.contains("[docs]: https://docs.rs"),
            "Reference definition should be appended"
        );
    }

    #[test]
    fn test_url_with_balanced_parentheses() {
        // Regression test: URLs like `EditorBuffer::get_lines()` contain balanced
        // parentheses. `split_link_url` tracks parenthesis depth so the full URL is
        // captured rather than truncating at the first `)`.
        let input = "See [`editor_buffer.get_lines()`](EditorBuffer::get_lines()).";
        let output = convert_links(input);

        // Should convert to reference-style with full URL.
        assert!(
            output.contains("[`editor_buffer.get_lines()`]"),
            "Link text should be preserved as reference"
        );
        assert!(
            output.contains("[`editor_buffer.get_lines()`]: EditorBuffer::get_lines()"),
            "Reference should have full URL including parentheses"
        );
        // The trailing period should remain after the reference link.
        assert!(
            output.contains("[`editor_buffer.get_lines()`]."),
            "Trailing period should remain after the reference link"
        );
    }

    #[test]
    fn test_backtick_ref_definition_not_extracted() {
        // A reference-like pattern inside backticks is not on its own line starting
        // with `[`, so `try_parse_reference_def` won't match it.
        let input = "Example: `[name]: url` is how you write references.";
        let output = convert_links(input);

        assert_eq!(
            output, input,
            "Reference-like pattern inside backticks should not be extracted"
        );
    }

    #[test]
    fn test_aggregate_nested_bracket_refs_no_spurious_blank_line() {
        // Regression test: reference definitions with nested brackets (e.g.,
        // `[u8; 64]` inside the link name) must not produce a spurious blank
        // line when aggregated with normal references.
        let input = "\
[`SmallString<[u8; 64]>`]: `smallstr::SmallString`
[`SmallString<[u8; 256]>`]: `smallstr::SmallString`
[`Display::fmt`]: Display::fmt
[`String`]: std::string::String";
        let output = convert_links(input);

        // All four should be recognized as references and sorted together
        // with no blank lines between them.
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 4, "Should have exactly 4 lines (no blank line)");
        // Sorted order: Display::fmt, SmallString 256, SmallString 64, String
        assert!(lines[0].contains("[`Display::fmt`]"));
        assert!(lines[1].contains("[`SmallString<[u8; 256]>`]"));
        assert!(lines[2].contains("[`SmallString<[u8; 64]>`]"));
        assert!(lines[3].contains("[`String`]"));
    }

    #[test]
    fn test_aggregate_nested_bracket_refs_with_content() {
        // Simulates the real-world scenario from fast_stringify.rs where
        // content precedes reference definitions containing nested brackets.
        let input = "\
This type alias allows us to easily experiment with different string-like data
structures in the future without impacting the rest of the codebase.

[`SmallString<[u8; 64]>`]: `smallstr::SmallString`
[`SmallString<[u8; 256]>`]: `smallstr::SmallString`
[`Display::fmt`]: Display::fmt
[`String`]: std::string::String";
        let output = convert_links(input);

        // Content should be preserved.
        assert!(output.contains("This type alias"));

        // References should be at the bottom with no spurious blank line.
        let lines: Vec<&str> = output.lines().collect();
        let ref_lines: Vec<&&str> = lines.iter().filter(|line| line.starts_with('[') ).collect();
        assert_eq!(ref_lines.len(), 4, "Should have exactly 4 reference lines");

        // No blank lines within the reference block.
        let first_ref = lines.iter().position(|line| line.starts_with('[')).unwrap();
        let ref_block = &lines[first_ref..];
        assert!(
            !ref_block.iter().any(|line| line.trim().is_empty()),
            "No blank lines should appear within the reference block"
        );
    }

    #[test]
    fn test_anchor_links_not_converted() {
        // Anchor links like [text](#section) should be left as inline links,
        // not converted to reference-style.
        let input = "- [Introduction](#introduction)\n- [Features](#features)";
        let output = convert_links(input);

        assert_eq!(
            output, input,
            "Anchor links should not be converted to reference-style"
        );
    }

    #[test]
    fn test_anchor_links_mixed_with_regular_links() {
        // Anchor links should be preserved while regular links are converted.
        let input = "See [Introduction](#intro) and [docs](https://docs.rs).";
        let output = convert_links(input);

        assert!(
            output.contains("[Introduction](#intro)"),
            "Anchor link should be preserved as inline"
        );
        assert!(
            output.contains("[docs]"),
            "Regular link should be converted to reference-style"
        );
        assert!(
            output.contains("[docs]: https://docs.rs"),
            "Reference definition should be appended"
        );
    }

    #[test]
    fn test_image_links_not_converted() {
        // Image links ![alt](url) should be left unchanged.
        let input = "![screenshot](https://example.com/img.png)";
        let output = convert_links(input);

        assert_eq!(
            output, input,
            "Image links should not be converted to reference-style"
        );
    }

    #[test]
    fn test_image_links_mixed_with_regular_links() {
        // Image links should be preserved while regular links are converted.
        let input =
            "See ![logo](https://example.com/logo.png) and [docs](https://docs.rs).";
        let output = convert_links(input);

        assert!(
            output.contains("![logo](https://example.com/logo.png)"),
            "Image link should be preserved"
        );
        assert!(
            output.contains("[docs]"),
            "Regular link should be converted"
        );
        assert!(
            output.contains("[docs]: https://docs.rs"),
            "Reference definition should be appended"
        );
    }

    #[test]
    fn test_multi_backtick_span_does_not_corrupt_subsequent_backticked_links() {
        let input = "- Bold (`**text**`), italic (`_text_`), inline code (`` `code` ``)\n\
                     - Images (`![alt](url)`), links (`[text](url)`)";
        let output = convert_links(input);
        assert_eq!(
            output, input,
            "Multi-backtick span containing a single backtick must not unbalance subsequent backtick spans"
        );
    }

    #[test]
    fn test_multiline_blockquote_inline_link_normalized_ref_def() {
        let input = "> operations and bounds checking, see the [coordinates module\n\
                     > documentation](crate::coordinates#common-workflows).";
        let output = convert_links(input);
        assert!(
            output.contains("[coordinates module\n> documentation]."),
            "In-place blockquote link wrapping should be preserved: {output}"
        );
        assert!(
            output.contains("[coordinates module documentation]: crate::coordinates#common-workflows"),
            "Reference definition label should be normalized to a single line without '>': {output}"
        );
    }

    #[test]
    fn test_multiline_indented_inline_link_normalized_ref_def() {
        let input = concat!(
            "  - Fast (over 100x faster than [Python-based\n",
            "    alternatives](https://github.com/edc/bass))."
        );
        let output = convert_links(input);
        assert!(
            output.contains("[Python-based\n    alternatives])."),
            "In-place list link wrapping should be preserved: {output}"
        );
        assert!(
            output.contains("[Python-based alternatives]: https://github.com/edc/bass"),
            "Reference definition label should be normalized to a single line: {output}"
        );
    }
}

