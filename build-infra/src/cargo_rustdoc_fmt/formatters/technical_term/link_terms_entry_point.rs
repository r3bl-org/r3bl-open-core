// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// rustdoc-fmt: skip

//! High-level CST block orchestrator and Paragraph/Table node walkers for technical
//! term linking.

use super::{TechnicalTermDictionary,
            markdown_spans::{advance_multiline_bracket_state,
                             find_unclosed_backtick_opening,
                             has_closing_backtick_sequence},
            ref_defs::{build_final_ref_defs, collect_existing_ref_defs,
                       should_rewrite_block_ref_defs},
            upgrade_term::{contains_standalone_linked_term, upgrade_term_in_line}};
use crate::cargo_rustdoc_fmt::{constants::{TOC_CLOSE_MARKER, TOC_OPEN_MARKER},
                               cst::{DocBlockCst, DocNode, TableData},
                               types::FormatStatus};
use r3bl_tui::{CRow, RangeBoundsExt, RangeOverlapResult};
use std::{collections::BTreeSet, ops::RangeInclusive};

/// Tracks whether line scanning is currently inside a table-of-contents block
/// (`<!-- TOC -->` ... `<!-- /TOC -->`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TocBlockState {
    Inside,
    Outside,
}

/// Upgrades known terms in a [`DocBlockCst`] across [`DocNode::Paragraph`] and
/// [`DocNode::Table`] nodes, and updates [`DocNode::ReferenceDefinitions`] at the bottom
/// of the block.
///
/// Code fences are structurally isolated in [`DocNode::CodeFence`] and never modified.
/// Returns [`FormatStatus::Modified`] if any node or reference definition was modified,
/// or [`FormatStatus::Unchanged`] otherwise.
pub fn link_terms_in_doc_block(
    block: &mut DocBlockCst,
    registry: &TechnicalTermDictionary,
    line_range: Option<&RangeInclusive<CRow>>,
) -> FormatStatus {
    if block.nodes.is_empty()
        || registry.is_empty()
        || line_range.is_some_and(|range| {
            block.span.overlaps(range) == RangeOverlapResult::Disjoint
        })
    {
        return FormatStatus::Unchanged;
    }

    let terms = registry.terms_longest_first();
    let existing_refs = collect_existing_ref_defs(&block.nodes);
    let mut linked_terms = prescan_linked_terms(&block.nodes, &terms);

    let mut modified = false;
    let mut toc_state = TocBlockState::Outside;

    for node in &mut block.nodes {
        match node {
            DocNode::Paragraph {
                span,
                lines,
                status: p_status,
                ..
            } => {
                let in_range = line_range.is_none_or(|range| {
                    span.overlaps(range) == RangeOverlapResult::Overlaps
                });
                if upgrade_terms_in_paragraph(
                    lines,
                    in_range,
                    &mut toc_state,
                    &terms,
                    &mut linked_terms,
                ) == FormatStatus::Modified
                {
                    *p_status = FormatStatus::Modified;
                    modified = true;
                }
            }
            DocNode::Table {
                span,
                table,
                status: t_status,
                ..
            } => {
                let in_range = line_range.is_none_or(|range| {
                    span.overlaps(range) == RangeOverlapResult::Overlaps
                });
                if toc_state == TocBlockState::Outside
                    && in_range
                    && upgrade_terms_in_table(table, &terms, &mut linked_terms)
                        == FormatStatus::Modified
                {
                    *t_status = FormatStatus::Modified;
                    modified = true;
                }
            }
            _ => {}
        }
    }

    if line_range.is_some() && !modified {
        return FormatStatus::Unchanged;
    }

    let final_defs = build_final_ref_defs(&linked_terms, registry, &existing_refs);
    if should_rewrite_block_ref_defs(block, &existing_refs, &final_defs, modified) {
        block.set_reference_definitions(final_defs);
        modified = true;
    }

    if modified {
        FormatStatus::Modified
    } else {
        FormatStatus::Unchanged
    }
}

/// Pre-scans all content nodes ([`DocNode::Paragraph`] and [`DocNode::Table`]) for
/// already-linked terms so existing linked terms outside a targeted `line_range` still
/// retain their reference definitions.
fn prescan_linked_terms(nodes: &[DocNode], terms: &[&str]) -> BTreeSet<String> {
    let mut linked_terms = BTreeSet::new();

    for node in nodes {
        match node {
            DocNode::Paragraph { lines, .. } => {
                for line in lines {
                    for &term in terms {
                        if contains_standalone_linked_term(line, term) {
                            linked_terms.insert(term.to_string());
                        }
                    }
                }
            }
            DocNode::Table { table, .. } => {
                for cell in table.headers.iter().chain(table.rows.iter().flatten()) {
                    for &term in terms {
                        if contains_standalone_linked_term(cell, term) {
                            linked_terms.insert(term.to_string());
                        }
                    }
                }
            }
            _ => {}
        }
    }

    linked_terms
}

/// Upgrades technical terms across lines in a [`DocNode::Paragraph`], tracking TOC block
/// state, multi-line backtick spans, and multi-line bracketed links.
fn upgrade_terms_in_paragraph(
    lines: &mut Vec<String>,
    in_range: bool,
    toc_state: &mut TocBlockState,
    terms: &[&str],
    linked_terms: &mut BTreeSet<String>,
) -> FormatStatus {
    let mut new_lines: Vec<String> = Vec::with_capacity(lines.len());
    let mut para_changed = false;
    let mut pending_backtick_close: usize = 0;
    let mut pending_bracket_depth: usize = 0;

    for line in lines.iter() {
        if line.contains(TOC_OPEN_MARKER) && !line.contains(TOC_CLOSE_MARKER) {
            *toc_state = TocBlockState::Inside;
            new_lines.push(line.clone());
            continue;
        }
        if line.contains(TOC_CLOSE_MARKER) {
            *toc_state = TocBlockState::Outside;
            new_lines.push(line.clone());
            continue;
        }
        if *toc_state == TocBlockState::Inside || !in_range {
            new_lines.push(line.clone());
            continue;
        }
        if pending_backtick_close > 0 {
            if has_closing_backtick_sequence(line, pending_backtick_close) {
                pending_backtick_close = 0;
            }
            new_lines.push(line.clone());
            continue;
        }
        if let Some(tick_count) = find_unclosed_backtick_opening(line) {
            pending_backtick_close = tick_count;
            new_lines.push(line.clone());
            continue;
        }

        let (next_bracket_depth, leading_link_end) =
            advance_multiline_bracket_state(line, pending_bracket_depth);
        pending_bracket_depth = next_bracket_depth;

        if leading_link_end == line.len() && leading_link_end > 0 {
            new_lines.push(line.clone());
            continue;
        }

        let prefix = &line[..leading_link_end];
        let suffix = &line[leading_link_end..];
        let modified_suffix = upgrade_all_terms_in_text(suffix, terms, linked_terms);

        let mut modified_line =
            String::with_capacity(prefix.len() + modified_suffix.len());
        modified_line.push_str(prefix);
        modified_line.push_str(&modified_suffix);

        if &modified_line != line {
            para_changed = true;
        }
        new_lines.push(modified_line);
    }

    if para_changed {
        *lines = new_lines;
        FormatStatus::Modified
    } else {
        FormatStatus::Unchanged
    }
}

/// Upgrades technical terms across headers and data cells in a [`TableData`].
fn upgrade_terms_in_table(
    table: &mut TableData,
    terms: &[&str],
    linked_terms: &mut BTreeSet<String>,
) -> FormatStatus {
    let mut table_modified = false;

    for cell in table
        .headers
        .iter_mut()
        .chain(table.rows.iter_mut().flatten())
    {
        let modified_cell = upgrade_all_terms_in_text(cell, terms, linked_terms);
        if &modified_cell != cell {
            *cell = modified_cell;
            table_modified = true;
        }
    }

    if table_modified {
        FormatStatus::Modified
    } else {
        FormatStatus::Unchanged
    }
}

/// Upgrades all known terms in `text` longest-first and records any standalone linked
/// terms into `linked_terms`.
fn upgrade_all_terms_in_text(
    text: &str,
    terms: &[&str],
    linked_terms: &mut BTreeSet<String>,
) -> String {
    let mut modified_text = text.to_string();
    for &term in terms {
        modified_text = upgrade_term_in_line(&modified_text, term);
        if contains_standalone_linked_term(&modified_text, term) {
            linked_terms.insert(term.to_string());
        }
    }
    modified_text
}

#[cfg(test)]
pub(crate) fn link_known_terms_for_test(
    text: &str,
    registry: &TechnicalTermDictionary,
) -> String {
    use crate::cargo_rustdoc_fmt::{constants::{CONTENT_SPACE_CHAR, LF_LINE_ENDING,
                                               OUTER_DOC_MARKER},
                                   cst::{CommentType, FileChunk, SourceFileCst,
                                         extract_comment_content}};

    if text.is_empty() {
        let mut empty_block = DocBlockCst {
            span: CRow::from(0)..=CRow::from(0),
            comment_type: CommentType::Outer,
            indent: String::new(),
            nodes: Vec::new(),
        };
        assert_eq!(
            link_terms_in_doc_block(&mut empty_block, registry, None),
            FormatStatus::Unchanged
        );
        return String::new();
    }

    let doc_source = text
        .lines()
        .map(|l| {
            if l.is_empty() {
                OUTER_DOC_MARKER.to_string()
            } else {
                format!("{OUTER_DOC_MARKER}{CONTENT_SPACE_CHAR}{l}")
            }
        })
        .collect::<Vec<_>>()
        .join(LF_LINE_ENDING);
    let mut cst = SourceFileCst::parse(&doc_source);
    for chunk in &mut cst.chunks {
        if let FileChunk::DocBlock(block) = chunk {
            link_terms_in_doc_block(block, registry, None);
        }
    }
    cst.reconstruct()
        .lines()
        .map(|l| extract_comment_content(l, OUTER_DOC_MARKER))
        .collect::<Vec<_>>()
        .join(LF_LINE_ENDING)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cargo_rustdoc_fmt::cst::{FileChunk, SourceFileCst};

    fn test_registry() -> TechnicalTermDictionary {
        TechnicalTermDictionary::from_seed(None).unwrap()
    }

    #[test]
    fn test_empty_input() {
        let registry = test_registry();
        let result = link_known_terms_for_test("", &registry);
        assert_eq!(result, "");
    }

    #[test]
    fn test_no_known_terms_unchanged() {
        let registry = test_registry();
        let input = "This has no known terms at all.";
        let result = link_known_terms_for_test(input, &registry);
        assert_eq!(result, input);
    }

    #[test]
    fn test_heading_also_linked() {
        let registry = test_registry();
        let input = "# CSI Sequences\n\nParsing CSI here.";
        let result = link_known_terms_for_test(input, &registry);
        assert!(
            result.starts_with("# [`CSI`]"),
            "Heading should be linked: {result}"
        );
        assert!(
            result.contains("Parsing [`CSI`]"),
            "Body should be linked: {result}"
        );
    }

    #[test]
    fn test_code_fence_not_modified() {
        let registry = test_registry();
        let input = "```\nCSI sequence inside code\n```";
        let result = link_known_terms_for_test(input, &registry);
        assert!(
            result.contains("```\nCSI sequence inside code\n```"),
            "Code fence should be preserved: {result}"
        );
    }

    #[test]
    fn test_toc_block_not_modified() {
        let registry = test_registry();
        let input = concat!(
            "<!-- TOC -->\n",
            "\n",
            "- [CSI Sequences](#csi)\n",
            "- [SGR Codes](#sgr)\n",
            "\n",
            "<!-- /TOC -->\n",
            "\n",
            "Uses CSI sequences.",
        );
        let result = link_known_terms_for_test(input, &registry);

        assert!(
            result.contains("- [CSI Sequences](#csi)"),
            "TOC content should not be modified: {result}"
        );
        assert!(
            result.contains("- [SGR Codes](#sgr)"),
            "TOC content should not be modified: {result}"
        );
        assert!(
            result.contains("Uses [`CSI`]"),
            "Content outside TOC should be linked: {result}"
        );
    }

    #[test]
    fn test_multiline_code_span_not_modified() {
        let registry = test_registry();
        let input = "Run with: `cargo test -p r3bl_tui --lib CSI --\n--nocapture`";
        let result = link_known_terms_for_test(input, &registry);
        assert!(
            !result.contains("[`CSI`]"),
            "Term inside multi-line code span should not be linked: {result}"
        );
        assert_eq!(result, input, "Multi-line code span should be unchanged");
    }

    #[test]
    fn test_multiline_code_span_three_lines() {
        let registry = test_registry();
        let input = "Run: `cargo test\n-p CSI\n--nocapture`";
        let result = link_known_terms_for_test(input, &registry);
        assert!(
            !result.contains("[`CSI`]"),
            "Term on middle line of multi-line code span should not be linked: {result}"
        );
        assert_eq!(result, input, "Multi-line code span should be unchanged");
    }

    #[test]
    fn test_multiline_code_span_subsequent_linking_works() {
        let registry = test_registry();
        let input = "Run: `cargo test\n--nocapture`\nUses CSI sequences.";
        let result = link_known_terms_for_test(input, &registry);
        assert!(
            result.contains("Uses [`CSI`]"),
            "Term after multi-line code span should be linked: {result}"
        );
    }

    #[test]
    fn test_multiline_code_span_double_backtick() {
        let registry = test_registry();
        let input = "Run: ``cargo test CSI\n--nocapture``";
        let result = link_known_terms_for_test(input, &registry);
        assert!(
            !result.contains("[`CSI`]"),
            "Term inside double-backtick multi-line span should not be linked: {result}"
        );
        assert_eq!(
            result, input,
            "Double-backtick multi-line span should be unchanged"
        );
    }

    #[test]
    fn test_multiline_bracketed_link_not_modified() {
        let registry = test_registry();
        let input = concat!(
            "See our [Unix\n",
            "implementation's termios section] for details on [termios].\n",
            "\n",
            "[Unix implementation's termios section]: crate::raw_mode_unix#termios\n",
            "[termios]: https://manpages.debian.org/termios.3",
        );
        let result = link_known_terms_for_test(input, &registry);
        assert!(
            result.contains("See our [Unix\nimplementation's termios section]"),
            "Multi-line bracketed link text should not have term linkified: {result}"
        );
        assert!(
            result.contains("for details on [`termios`]."),
            "Standalone [termios] shortcut link should be upgraded to [`termios`]: {result}"
        );
    }

    #[test]
    fn test_table_headers_and_cells_terms_upgraded() {
        let registry = test_registry();
        let input = concat!(
            "| CSI Header | Description |\n",
            "| --- | --- |\n",
            "| [`SGR`] | Uses ANSI codes |\n",
            "\n",
            "[`SGR`]: crate::SgrCode",
        );
        let result = link_known_terms_for_test(input, &registry);
        assert!(
            result.contains("| [`CSI`] Header | Description         |"),
            "Table header term should be upgraded: {result}"
        );
        assert!(
            result.contains("| [`SGR`]        | Uses [`ANSI`] codes |"),
            "Table cell term should be upgraded: {result}"
        );
        assert!(
            result.contains("[`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code")
                && result.contains("[`CSI`]: crate::CsiSequence")
                && result.contains("[`SGR`]: crate::SgrCode"),
            "All table term ref defs should be present: {result}"
        );
    }

    #[test]
    fn test_line_range_filtering_in_doc_block() {
        let registry = test_registry();
        let source = concat!(
            "/// Uses [`SGR`] already.\n",
            "///\n",
            "/// Parses CSI sequences.\n",
            "///\n",
            "/// Uses ANSI codes.\n",
            "///\n",
            "/// [`SGR`]: crate::SgrCode",
        );
        let mut cst = SourceFileCst::parse(source);
        let Some(FileChunk::DocBlock(block)) = cst.chunks.first_mut() else {
            panic!("Expected DocBlock");
        };

        // 1. Disjoint line_range returns Unchanged.
        let disjoint = CRow::from(50)..=CRow::from(60);
        assert_eq!(
            link_terms_in_doc_block(block, &registry, Some(&disjoint)),
            FormatStatus::Unchanged
        );

        // 2. Overlapping line_range with no new term upgrades returns Unchanged.
        let first_para_only = CRow::from(0)..=CRow::from(0);
        assert_eq!(
            link_terms_in_doc_block(block, &registry, Some(&first_para_only)),
            FormatStatus::Unchanged
        );

        // 3. Overlapping line_range covering only second paragraph upgrades CSI,
        // leaves ANSI in third paragraph untouched, and retains pre-existing SGR ref def.
        let second_para_only = CRow::from(2)..=CRow::from(2);
        assert_eq!(
            link_terms_in_doc_block(block, &registry, Some(&second_para_only)),
            FormatStatus::Modified
        );
        let output = cst.reconstruct();
        assert!(
            output.contains("/// Parses [`CSI`] sequences."),
            "Second paragraph in range should be upgraded: {output}"
        );
        assert!(
            output.contains("/// Uses ANSI codes."),
            "Third paragraph outside range should not be upgraded: {output}"
        );
        assert!(
            output.contains("/// [`CSI`]: crate::CsiSequence")
                && output.contains("/// [`SGR`]: crate::SgrCode"),
            "Both newly linked and pre-existing linked terms should have ref defs: {output}"
        );
    }
}
