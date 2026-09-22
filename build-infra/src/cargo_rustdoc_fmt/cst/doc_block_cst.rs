// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// rustdoc-fmt: skip

//! Doc comment block Concrete Syntax Tree ([`DocBlockCst`]), [`DocNode`],
//! [`CommentType`], and doc comment line detection helpers ([`DocCommentLine`]).

use super::{LinkReference, TableData, TableStartStatus, is_table_start,
            parse_table_lines, try_parse_reference_def};
use crate::cargo_rustdoc_fmt::{constants::{CODE_FENCE_DELIMITER, CONTENT_SPACE_CHAR,
                                           FOUR_SLASH_COMMENT_PREFIX, INNER_DOC_MARKER,
                                           OUTER_DOC_MARKER, TABLE_CELL_DELIMITER},
                               types::FormatStatus};
use r3bl_tui::CRow;
use std::ops::RangeInclusive;

/// Type of rustdoc comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentType {
    /// Inner doc comment: `//!`
    Inner,
    /// Outer doc comment: `///`
    Outer,
}

impl CommentType {
    /// Returns the comment marker prefix (`"//!"` or `"///"`) for this comment type.
    #[must_use]
    pub const fn marker(self) -> &'static str {
        match self {
            Self::Inner => INNER_DOC_MARKER,
            Self::Outer => OUTER_DOC_MARKER,
        }
    }
}

/// A contiguous block of doc comments (`///` or `//!`) parsed into `CST` nodes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocBlockCst {
    pub span: RangeInclusive<CRow>,
    pub comment_type: CommentType,
    pub indent: String,
    pub nodes: Vec<DocNode>,
}

impl DocBlockCst {
    /// Returns [`FormatStatus::Modified`] if any node in this doc block has been
    /// modified, or [`FormatStatus::Unchanged`] otherwise.
    #[must_use]
    pub fn status(&self) -> FormatStatus {
        if self
            .nodes
            .iter()
            .any(|node| node.status() == FormatStatus::Modified)
        {
            FormatStatus::Modified
        } else {
            FormatStatus::Unchanged
        }
    }

    /// Checks whether [`DocNode::ReferenceDefinitions`] in this block are in canonical
    /// layout:
    /// - When `has_final_defs` is `false`: `ref_node_count == 0`.
    /// - When `has_final_defs` is `true`: a single [`DocNode::ReferenceDefinitions`] node
    ///   at the end of `self.nodes`, either as the only node (`len == 1`) or preceded by
    ///   content and exactly one [`DocNode::BlankLine`] (`len >= 3`).
    #[must_use]
    pub fn has_canonical_reference_layout(
        &self,
        ref_node_count: usize,
        has_final_defs: bool,
    ) -> bool {
        if !has_final_defs {
            return ref_node_count == 0;
        }
        ref_node_count == 1
            && matches!(
                self.nodes.last(),
                Some(DocNode::ReferenceDefinitions { .. })
            )
            && (self.nodes.len() == 1
                || (self.nodes.len() >= 3
                    && matches!(
                        self.nodes.get(self.nodes.len() - 2),
                        Some(DocNode::BlankLine { .. })
                    )
                    && !matches!(
                        self.nodes.get(self.nodes.len() - 3),
                        Some(DocNode::BlankLine { .. })
                    )))
    }

    /// Replaces all [`DocNode::ReferenceDefinitions`] in this block with a single
    /// canonical [`DocNode::ReferenceDefinitions`] node at the bottom (preceded by a
    /// single [`DocNode::BlankLine`] if the block has preceding content nodes).
    pub fn set_reference_definitions(&mut self, definitions: Vec<LinkReference>) {
        let mut new_nodes = Vec::new();
        for node in self.nodes.drain(..) {
            if !matches!(node, DocNode::ReferenceDefinitions { .. }) {
                new_nodes.push(node);
            }
        }

        while let Some(DocNode::BlankLine { .. }) = new_nodes.last() {
            new_nodes.pop();
        }

        if !definitions.is_empty() {
            let marker = self.comment_type.marker();
            if !new_nodes.is_empty() {
                new_nodes.push(DocNode::BlankLine {
                    span: self.span.clone(),
                    raw_line: format!("{}{}", self.indent, marker),
                });
            }
            new_nodes.push(DocNode::ReferenceDefinitions {
                span: self.span.clone(),
                definitions,
                raw_lines: Vec::new(),
                status: FormatStatus::Modified,
            });
        }

        self.nodes = new_nodes;
    }

    /// Parses the raw lines of a doc comment block into `CST` nodes.
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn parse(
        raw_lines: Vec<String>,
        comment_type: CommentType,
        indent: String,
        marker: &'static str,
        span: RangeInclusive<CRow>,
    ) -> Self {
        let mut nodes = Vec::new();
        let mut block_line_idx = 0;
        let start_row = *span.start();

        while block_line_idx < raw_lines.len() {
            let cur_line = &raw_lines[block_line_idx];
            let cur_line_num = start_row + block_line_idx;
            let content = extract_comment_content(cur_line, marker);

            // 1. Blank line (empty doc comment, e.g. "///" or "/// ").
            if content.trim().is_empty() {
                nodes.push(DocNode::BlankLine {
                    span: cur_line_num..=cur_line_num,
                    raw_line: cur_line.clone(),
                });
                block_line_idx += 1;
                continue;
            }

            // 2. Code fence: starts with ```.
            if content.trim_start().starts_with(CODE_FENCE_DELIMITER) {
                let fence_start = cur_line_num;
                let mut fence_lines = vec![cur_line.clone()];
                block_line_idx += 1;

                while block_line_idx < raw_lines.len() {
                    let fence_line = &raw_lines[block_line_idx];
                    fence_lines.push(fence_line.clone());
                    let inner_content = extract_comment_content(fence_line, marker);
                    if inner_content.trim() == CODE_FENCE_DELIMITER {
                        block_line_idx += 1;
                        break;
                    }
                    block_line_idx += 1;
                }

                let fence_end = start_row + block_line_idx - 1;
                nodes.push(DocNode::CodeFence {
                    span: fence_start..=fence_end,
                    lines: fence_lines,
                    status: FormatStatus::Unchanged,
                });
                continue;
            }

            // 3. Table: starts with | and next line is a valid separator row.
            if is_table_start(&raw_lines, block_line_idx, marker)
                == TableStartStatus::StartsTable
            {
                let table_start = cur_line_num;
                let first_content =
                    extract_comment_content(&raw_lines[block_line_idx], marker);
                let content_indent = &first_content
                    [..first_content.len() - first_content.trim_start().len()];

                let mut table_raw_lines = Vec::new();
                let mut table_content_lines = Vec::new();

                while block_line_idx < raw_lines.len() {
                    let table_line = &raw_lines[block_line_idx];
                    let line_content = extract_comment_content(table_line, marker);
                    let trimmed = line_content.trim();
                    if trimmed.starts_with(TABLE_CELL_DELIMITER)
                        && trimmed.ends_with(TABLE_CELL_DELIMITER)
                    {
                        table_raw_lines.push(table_line.clone());
                        table_content_lines.push(trimmed.to_string());
                        block_line_idx += 1;
                    } else {
                        break;
                    }
                }

                let table_end = start_row + block_line_idx - 1;
                let table_data = parse_table_lines(&table_content_lines);

                nodes.push(DocNode::Table {
                    span: table_start..=table_end,
                    content_indent: content_indent.to_string(),
                    table: table_data,
                    raw_lines: table_raw_lines,
                    status: FormatStatus::Unchanged,
                });
                continue;
            }

            // 4. Reference definitions: [label]: target (single-line or multi-line).
            if try_parse_reference_def(&raw_lines, block_line_idx, marker).is_some() {
                let ref_start = cur_line_num;
                let mut ref_raw_lines = Vec::new();
                let mut ref_defs = Vec::new();

                while block_line_idx < raw_lines.len() {
                    if let Some(parsed_ref) =
                        try_parse_reference_def(&raw_lines, block_line_idx, marker)
                    {
                        for offset_idx in 0..parsed_ref.lines_consumed {
                            ref_raw_lines
                                .push(raw_lines[block_line_idx + offset_idx].clone());
                        }
                        ref_defs.push(parsed_ref.definition);
                        block_line_idx += parsed_ref.lines_consumed;
                    } else {
                        break;
                    }
                }

                let ref_end = start_row + block_line_idx - 1;
                nodes.push(DocNode::ReferenceDefinitions {
                    span: ref_start..=ref_end,
                    definitions: ref_defs,
                    raw_lines: ref_raw_lines,
                    status: FormatStatus::Unchanged,
                });
                continue;
            }

            // 5. Paragraph: general prose, headings, list items.
            let para_start = cur_line_num;
            let mut para_raw_lines = Vec::new();
            let mut para_content_lines = Vec::new();

            while block_line_idx < raw_lines.len() {
                let para_line = &raw_lines[block_line_idx];
                let para_content = extract_comment_content(para_line, marker);

                // Stop if we hit blank line, code fence, table start, or reference
                // definition.
                if para_content.trim().is_empty()
                    || para_content.trim_start().starts_with(CODE_FENCE_DELIMITER)
                    || is_table_start(&raw_lines, block_line_idx, marker)
                        == TableStartStatus::StartsTable
                    || try_parse_reference_def(&raw_lines, block_line_idx, marker)
                        .is_some()
                {
                    break;
                }

                para_raw_lines.push(para_line.clone());
                para_content_lines.push(para_content.to_string());
                block_line_idx += 1;
            }

            let para_end = start_row + block_line_idx - 1;
            nodes.push(DocNode::Paragraph {
                span: para_start..=para_end,
                lines: para_content_lines,
                raw_lines: para_raw_lines,
                status: FormatStatus::Unchanged,
            });
        }

        Self {
            span,
            comment_type,
            indent,
            nodes,
        }
    }
}

/// Concrete Syntax Tree (`CST`) node inside a doc comment block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocNode {
    /// Prose: paragraphs, bullet/numbered lists, headings, blockquotes.
    Paragraph {
        span: RangeInclusive<CRow>,
        lines: Vec<String>,
        raw_lines: Vec<String>,
        status: FormatStatus,
    },
    /// Code fence: ```` ```[lang] ... ``` ````
    CodeFence {
        span: RangeInclusive<CRow>,
        lines: Vec<String>,
        status: FormatStatus,
    },
    /// Markdown table.
    Table {
        span: RangeInclusive<CRow>,
        content_indent: String,
        table: TableData,
        raw_lines: Vec<String>,
        status: FormatStatus,
    },
    /// Reference link definitions at bottom of doc block: `[label]: target`.
    ReferenceDefinitions {
        span: RangeInclusive<CRow>,
        definitions: Vec<LinkReference>,
        raw_lines: Vec<String>,
        status: FormatStatus,
    },
    /// Blank doc comment line (`///` or `//!` with no text).
    BlankLine {
        span: RangeInclusive<CRow>,
        raw_line: String,
    },
}

impl DocNode {
    /// Returns the 0-based inclusive source line span for this node.
    #[must_use]
    pub fn span(&self) -> RangeInclusive<CRow> {
        match self {
            Self::Paragraph { span, .. }
            | Self::CodeFence { span, .. }
            | Self::Table { span, .. }
            | Self::ReferenceDefinitions { span, .. }
            | Self::BlankLine { span, .. } => span.clone(),
        }
    }

    /// Returns the [`FormatStatus`] for this node.
    #[must_use]
    pub fn status(&self) -> FormatStatus {
        match self {
            Self::Paragraph { status, .. }
            | Self::CodeFence { status, .. }
            | Self::Table { status, .. }
            | Self::ReferenceDefinitions { status, .. } => *status,
            Self::BlankLine { .. } => FormatStatus::Unchanged,
        }
    }
}

/// Metadata extracted from a single rustdoc comment line (`///` or `//!`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocCommentLine {
    pub comment_type: CommentType,
    pub indent: String,
    pub marker: &'static str,
}

/// Detects if a line is a rustdoc comment (`///` or `//!`).
#[must_use]
pub fn detect_doc_comment(line: &str) -> Option<DocCommentLine> {
    let trimmed = line.trim_start();
    let indent = line[..line.len() - trimmed.len()].to_string();

    if trimmed.starts_with(INNER_DOC_MARKER) {
        Some(DocCommentLine {
            comment_type: CommentType::Inner,
            indent,
            marker: INNER_DOC_MARKER,
        })
    } else if trimmed.starts_with(OUTER_DOC_MARKER)
        && !trimmed.starts_with(FOUR_SLASH_COMMENT_PREFIX)
    {
        Some(DocCommentLine {
            comment_type: CommentType::Outer,
            indent,
            marker: OUTER_DOC_MARKER,
        })
    } else {
        None
    }
}

/// Extracts comment content, removing the marker and one optional leading space.
#[must_use]
pub fn extract_comment_content<'a>(line: &'a str, marker: &str) -> &'a str {
    let trimmed = line.trim_start();
    if let Some(after_marker) = trimmed.strip_prefix(marker) {
        if let Some(stripped) = after_marker.strip_prefix(CONTENT_SPACE_CHAR) {
            stripped
        } else {
            after_marker
        }
    } else {
        line
    }
}
