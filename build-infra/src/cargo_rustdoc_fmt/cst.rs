// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// rustdoc-fmt: skip

//! Lossless Concrete Syntax Tree (CST) parser and reassembler for Rust source files.
//!
//! Partitions Rust source code into `FileChunk::Code` and `FileChunk::DocBlock`.
//! Inside each doc comment block, classifies lines into typed `DocNode`s (`Paragraph`,
//! `CodeFence`, `Table`, `ReferenceDefinitions`, `BlankLine`), each tracking its exact
//! 1-indexed `SourceSpan`.

use crate::cargo_rustdoc_fmt::types::{ColumnAlignment, CommentType, DocBlockCst,
                                      DocNode, FileChunk, LinkReference, SourceFileCst,
                                      SourceSpan, TableData};
use regex::Regex;
use std::{path::PathBuf, sync::LazyLock};
use unicode_width::UnicodeWidthStr;

static REFERENCE_DEF_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^\[([^\[\]]+)\]:\s*(\S+)(?:\s+"([^"]*)")?\s*$"#)
        .expect("Invalid reference definition regex")
});

impl SourceFileCst {
    /// Parses Rust source code into a lossless Concrete Syntax Tree.
    #[must_use]
    pub fn parse(source: &str) -> Self { Self::parse_with_path(source, None) }

    /// Parses Rust source code with an optional file path.
    #[must_use]
    pub fn parse_with_path(source: &str, path: Option<PathBuf>) -> Self {
        if source.is_empty() {
            return Self {
                path,
                chunks: Vec::new(),
                newline: "\n",
                has_trailing_newline: false,
            };
        }

        let newline = if source.contains("\r\n") {
            "\r\n"
        } else {
            "\n"
        };
        let has_trailing_newline = source.ends_with('\n');
        let lines: Vec<&str> = source.lines().collect();

        let mut chunks = Vec::new();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i];

            if let Some((comment_type, indent, marker)) = detect_doc_comment(line) {
                // Collect contiguous doc comment lines with the same comment type and
                // marker
                let block_start_line = i + 1; // 1-indexed
                let mut raw_lines = Vec::new();

                while i < lines.len() {
                    let cur_line = lines[i];
                    if let Some((cur_type, cur_indent, cur_marker)) =
                        detect_doc_comment(cur_line)
                    {
                        if cur_type == comment_type
                            && cur_marker == marker
                            && cur_indent == indent
                        {
                            raw_lines.push(cur_line.to_string());
                            i += 1;
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }

                let block_end_line = i; // 1-indexed, inclusive
                let span = SourceSpan::new(block_start_line, block_end_line);
                let doc_block =
                    DocBlockCst::parse(raw_lines, comment_type, indent, marker, span);
                chunks.push(FileChunk::DocBlock(doc_block));
            } else {
                // Collect contiguous non-doc lines (Rust code, blank lines, regular
                // comments)
                let code_start_line = i + 1;
                let mut code_lines = Vec::new();

                while i < lines.len() {
                    let cur_line = lines[i];
                    if detect_doc_comment(cur_line).is_some() {
                        break;
                    }
                    code_lines.push(cur_line.to_string());
                    i += 1;
                }

                let code_end_line = i;
                let span = SourceSpan::new(code_start_line, code_end_line);
                chunks.push(FileChunk::Code {
                    span,
                    lines: code_lines,
                });
            }
        }

        Self {
            path,
            chunks,
            newline,
            has_trailing_newline,
        }
    }

    /// Reconstructs the source file from the CST.
    ///
    /// Unmodified nodes output their original `raw_lines` verbatim, ensuring 100%
    /// byte-for-byte preservation for untouched code and documentation.
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn reconstruct(&self) -> String {
        let mut output = String::new();
        let mut first_line = true;

        for chunk in &self.chunks {
            match chunk {
                FileChunk::Code { lines, .. } => {
                    for line in lines {
                        if !first_line {
                            output.push_str(self.newline);
                        }
                        output.push_str(line);
                        first_line = false;
                    }
                }
                FileChunk::DocBlock(block) => {
                    for node in &block.nodes {
                        match node {
                            DocNode::Paragraph {
                                lines,
                                raw_lines,
                                modified,
                                ..
                            } => {
                                if *modified {
                                    for line in lines {
                                        if !first_line {
                                            output.push_str(self.newline);
                                        }
                                        output.push_str(&block.indent);
                                        output.push_str(marker_for_type(
                                            block.comment_type,
                                        ));
                                        if !line.is_empty() {
                                            output.push(' ');
                                            output.push_str(line);
                                        }
                                        first_line = false;
                                    }
                                } else {
                                    for raw in raw_lines {
                                        if !first_line {
                                            output.push_str(self.newline);
                                        }
                                        output.push_str(raw);
                                        first_line = false;
                                    }
                                }
                            }
                            DocNode::Table {
                                table,
                                content_indent,
                                raw_lines,
                                modified,
                                ..
                            } => {
                                if *modified {
                                    let formatted_rows = format_table_data(table);
                                    for row in formatted_rows {
                                        if !first_line {
                                            output.push_str(self.newline);
                                        }
                                        output.push_str(&block.indent);
                                        output.push_str(marker_for_type(
                                            block.comment_type,
                                        ));
                                        output.push(' ');
                                        output.push_str(content_indent);
                                        output.push_str(&row);
                                        first_line = false;
                                    }
                                } else {
                                    for raw in raw_lines {
                                        if !first_line {
                                            output.push_str(self.newline);
                                        }
                                        output.push_str(raw);
                                        first_line = false;
                                    }
                                }
                            }
                            DocNode::ReferenceDefinitions {
                                definitions,
                                raw_lines,
                                modified,
                                ..
                            } => {
                                if *modified {
                                    for def in definitions {
                                        if !first_line {
                                            output.push_str(self.newline);
                                        }
                                        output.push_str(&block.indent);
                                        output.push_str(marker_for_type(
                                            block.comment_type,
                                        ));
                                        output.push(' ');
                                        output.push('[');
                                        output.push_str(&def.label);
                                        output.push_str("]: ");
                                        output.push_str(&def.target);
                                        if let Some(title) = &def.title {
                                            output.push_str(" \"");
                                            output.push_str(title);
                                            output.push('"');
                                        }
                                        first_line = false;
                                    }
                                } else {
                                    for raw in raw_lines {
                                        if !first_line {
                                            output.push_str(self.newline);
                                        }
                                        output.push_str(raw);
                                        first_line = false;
                                    }
                                }
                            }
                            DocNode::CodeFence { lines, .. } => {
                                for raw in lines {
                                    if !first_line {
                                        output.push_str(self.newline);
                                    }
                                    output.push_str(raw);
                                    first_line = false;
                                }
                            }
                            DocNode::BlankLine { raw_line, .. } => {
                                if !first_line {
                                    output.push_str(self.newline);
                                }
                                output.push_str(raw_line);
                                first_line = false;
                            }
                        }
                    }
                }
            }
        }

        if self.has_trailing_newline && !output.is_empty() {
            output.push_str(self.newline);
        }

        output
    }
}

impl DocBlockCst {
    /// Parses the raw lines of a doc comment block into CST nodes.
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn parse(
        raw_lines: Vec<String>,
        comment_type: CommentType,
        indent: String,
        marker: &'static str,
        span: SourceSpan,
    ) -> Self {
        let mut nodes = Vec::new();
        let mut j = 0;

        while j < raw_lines.len() {
            let cur_line = &raw_lines[j];
            let cur_line_num = span.start_line + j;
            let content = extract_comment_content(cur_line, marker);

            // 1. Blank line (empty doc comment, e.g. "///" or "/// ")
            if content.trim().is_empty() {
                nodes.push(DocNode::BlankLine {
                    span: SourceSpan::new(cur_line_num, cur_line_num),
                    raw_line: cur_line.clone(),
                });
                j += 1;
                continue;
            }

            // 2. Code fence: starts with ```
            if content.trim_start().starts_with("```") {
                let fence_start = cur_line_num;
                let trimmed = content.trim_start();
                let language = trimmed
                    .strip_prefix("```")
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty());

                let mut fence_lines = vec![cur_line.clone()];
                j += 1;

                while j < raw_lines.len() {
                    let fence_line = &raw_lines[j];
                    fence_lines.push(fence_line.clone());
                    let inner_content = extract_comment_content(fence_line, marker);
                    if inner_content.trim() == "```" {
                        j += 1;
                        break;
                    }
                    j += 1;
                }

                let fence_end = span.start_line + j - 1;
                nodes.push(DocNode::CodeFence {
                    span: SourceSpan::new(fence_start, fence_end),
                    language,
                    lines: fence_lines,
                });
                continue;
            }

            // 3. Table: starts with | and next line is a valid separator row
            if is_table_start(&raw_lines, j, marker) {
                let table_start = cur_line_num;
                let first_content = extract_comment_content(&raw_lines[j], marker);
                let content_indent = &first_content
                    [..first_content.len() - first_content.trim_start().len()];

                let mut table_raw_lines = Vec::new();
                let mut table_content_lines = Vec::new();

                while j < raw_lines.len() {
                    let table_line = &raw_lines[j];
                    let line_content = extract_comment_content(table_line, marker);
                    let trimmed = line_content.trim();
                    if trimmed.starts_with('|') && trimmed.ends_with('|') {
                        table_raw_lines.push(table_line.clone());
                        table_content_lines.push(trimmed.to_string());
                        j += 1;
                    } else {
                        break;
                    }
                }

                let table_end = span.start_line + j - 1;
                let table_data = parse_table_lines(&table_content_lines);

                nodes.push(DocNode::Table {
                    span: SourceSpan::new(table_start, table_end),
                    content_indent: content_indent.to_string(),
                    table: table_data,
                    raw_lines: table_raw_lines,
                    modified: false,
                });
                continue;
            }

            // 4. Reference definitions: [label]: target
            if REFERENCE_DEF_REGEX.is_match(content.trim()) {
                let ref_start = cur_line_num;
                let mut ref_raw_lines = Vec::new();
                let mut ref_defs = Vec::new();

                while j < raw_lines.len() {
                    let ref_line = &raw_lines[j];
                    let line_content = extract_comment_content(ref_line, marker);
                    if let Some(caps) = REFERENCE_DEF_REGEX.captures(line_content.trim())
                    {
                        ref_raw_lines.push(ref_line.clone());
                        let label = caps.get(1).map_or("", |m| m.as_str()).to_string();
                        let target = caps.get(2).map_or("", |m| m.as_str()).to_string();
                        let title = caps.get(3).map(|m| m.as_str().to_string());
                        ref_defs.push(LinkReference {
                            label,
                            target,
                            title,
                        });
                        j += 1;
                    } else {
                        break;
                    }
                }

                let ref_end = span.start_line + j - 1;
                nodes.push(DocNode::ReferenceDefinitions {
                    span: SourceSpan::new(ref_start, ref_end),
                    definitions: ref_defs,
                    raw_lines: ref_raw_lines,
                    modified: false,
                });
                continue;
            }

            // 5. Paragraph: general prose, headings, list items
            let para_start = cur_line_num;
            let mut para_raw_lines = Vec::new();
            let mut para_content_lines = Vec::new();

            while j < raw_lines.len() {
                let p_line = &raw_lines[j];
                let p_content = extract_comment_content(p_line, marker);

                // Stop if we hit blank line, code fence, table start, or reference
                // definition
                if p_content.trim().is_empty()
                    || p_content.trim_start().starts_with("```")
                    || is_table_start(&raw_lines, j, marker)
                    || REFERENCE_DEF_REGEX.is_match(p_content.trim())
                {
                    break;
                }

                para_raw_lines.push(p_line.clone());
                para_content_lines.push(p_content.to_string());
                j += 1;
            }

            let para_end = span.start_line + j - 1;
            nodes.push(DocNode::Paragraph {
                span: SourceSpan::new(para_start, para_end),
                lines: para_content_lines,
                raw_lines: para_raw_lines,
                modified: false,
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

/// Detects if a line is a rustdoc comment (`///` or `//!`).
#[must_use]
pub fn detect_doc_comment(line: &str) -> Option<(CommentType, String, &'static str)> {
    let trimmed = line.trim_start();
    let indent = line[..line.len() - trimmed.len()].to_string();

    if trimmed.starts_with("//!") {
        Some((CommentType::Inner, indent, "//!"))
    } else if trimmed.starts_with("///") && !trimmed.starts_with("////") {
        Some((CommentType::Outer, indent, "///"))
    } else {
        None
    }
}

/// Extracts comment content, removing the marker and one optional leading space.
#[must_use]
pub fn extract_comment_content<'a>(line: &'a str, marker: &str) -> &'a str {
    let trimmed = line.trim_start();
    if let Some(after_marker) = trimmed.strip_prefix(marker) {
        if let Some(stripped) = after_marker.strip_prefix(' ') {
            stripped
        } else {
            after_marker
        }
    } else {
        line
    }
}

/// Checks whether lines starting at index `start` form a valid markdown table.
fn is_table_start(raw_lines: &[String], start: usize, marker: &str) -> bool {
    if start + 1 >= raw_lines.len() {
        return false;
    }

    let line0 = extract_comment_content(&raw_lines[start], marker).trim();
    let line1 = extract_comment_content(&raw_lines[start + 1], marker).trim();

    if !line0.starts_with('|') || !line0.ends_with('|') {
        return false;
    }

    if !line1.starts_with('|') || !line1.ends_with('|') {
        return false;
    }

    is_separator_row(line1)
        && parse_table_row(line0).len() == parse_table_row(line1).len()
}

/// Checks if a row is a valid GFM separator row: cells contain only `-`, `:`, and
/// whitespace with at least one `-`.
#[must_use]
pub fn is_separator_row(line: &str) -> bool {
    let cells = parse_table_row(line);
    if cells.is_empty() {
        return false;
    }

    for cell in &cells {
        let trimmed = cell.trim();
        if trimmed.is_empty() {
            return false;
        }
        let has_dash = trimmed.contains('-');
        let valid_chars = trimmed
            .chars()
            .all(|c| c == '-' || c == ':' || c.is_whitespace());
        if !has_dash || !valid_chars {
            return false;
        }
    }

    true
}

/// Parses a pipe-delimited table row into cells.
#[must_use]
pub fn parse_table_row(line: &str) -> Vec<String> {
    let trimmed = line.trim();
    let inner = if let Some(stripped) = trimmed.strip_prefix('|') {
        stripped.strip_suffix('|').unwrap_or(stripped)
    } else {
        trimmed
    };

    inner
        .split('|')
        .map(|cell| cell.trim().to_string())
        .collect()
}

/// Parses raw table lines into `TableData`.
#[must_use]
pub fn parse_table_lines(lines: &[String]) -> TableData {
    if lines.len() < 2 {
        return TableData {
            headers: Vec::new(),
            alignments: Vec::new(),
            rows: Vec::new(),
        };
    }

    let headers = parse_table_row(&lines[0]);
    let sep_cells = parse_table_row(&lines[1]);
    let alignments: Vec<ColumnAlignment> = sep_cells
        .iter()
        .map(|c| {
            let trimmed = c.trim();
            let starts = trimmed.starts_with(':');
            let ends = trimmed.ends_with(':');
            match (starts, ends) {
                (true, true) => ColumnAlignment::Center,
                (true, false) => ColumnAlignment::Left,
                (false, true) => ColumnAlignment::Right,
                (false, false) => ColumnAlignment::None,
            }
        })
        .collect();

    let mut rows = Vec::new();
    for line in &lines[2..] {
        rows.push(parse_table_row(line));
    }

    TableData {
        headers,
        alignments,
        rows,
    }
}

/// Formats `TableData` into aligned markdown table rows.
#[must_use]
pub fn format_table_data(table: &TableData) -> Vec<String> {
    if table.headers.is_empty() {
        return Vec::new();
    }

    let col_count = table.headers.len();
    let mut widths = vec![3; col_count]; // Minimum width of 3 for `---`

    // Calculate maximum width for each column based on headers and rows
    for (i, h) in table.headers.iter().enumerate() {
        if i < col_count {
            widths[i] = widths[i].max(UnicodeWidthStr::width(h.as_str()));
        }
    }

    for row in &table.rows {
        for (i, cell) in row.iter().enumerate() {
            if i < col_count {
                widths[i] = widths[i].max(UnicodeWidthStr::width(cell.as_str()));
            }
        }
    }

    let mut result = Vec::new();

    // 1. Header row
    let header_cells: Vec<String> = table
        .headers
        .iter()
        .enumerate()
        .map(|(i, h)| {
            let width = widths[i];
            let cell_w = UnicodeWidthStr::width(h.as_str());
            let pad = width.saturating_sub(cell_w);
            format!("{h}{}", " ".repeat(pad))
        })
        .collect();
    result.push(format!("| {} |", header_cells.join(" | ")));

    // 2. Separator row
    let sep_cells: Vec<String> = widths
        .iter()
        .enumerate()
        .map(|(i, &w)| {
            let align = table
                .alignments
                .get(i)
                .copied()
                .unwrap_or(ColumnAlignment::None);
            match align {
                ColumnAlignment::Center => {
                    let dashes = "-".repeat(w.saturating_sub(2).max(1));
                    format!(":{dashes}:")
                }
                ColumnAlignment::Left => {
                    let dashes = "-".repeat(w.saturating_sub(1).max(2));
                    format!(":{dashes}")
                }
                ColumnAlignment::Right => {
                    let dashes = "-".repeat(w.saturating_sub(1).max(2));
                    format!("{dashes}:")
                }
                ColumnAlignment::None => "-".repeat(w.max(3)),
            }
        })
        .collect();
    result.push(format!("| {} |", sep_cells.join(" | ")));

    // 3. Data rows
    for row in &table.rows {
        let row_cells: Vec<String> = (0..col_count)
            .map(|i| {
                let cell = row.get(i).map_or("", |s| s.as_str());
                let width = widths[i];
                let cell_w = UnicodeWidthStr::width(cell);
                let pad = width.saturating_sub(cell_w);
                let align = table
                    .alignments
                    .get(i)
                    .copied()
                    .unwrap_or(ColumnAlignment::None);
                match align {
                    ColumnAlignment::Right => format!("{}{cell}", " ".repeat(pad)),
                    _ => format!("{cell}{}", " ".repeat(pad)),
                }
            })
            .collect();
        result.push(format!("| {} |", row_cells.join(" | ")));
    }

    result
}

const fn marker_for_type(comment_type: CommentType) -> &'static str {
    match comment_type {
        CommentType::Inner => "//!",
        CommentType::Outer => "///",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lossless_roundtrip_pure_code() {
        let source = "fn main() {\n    println!(\"Hello\");\n}\n";
        let cst = SourceFileCst::parse(source);
        assert_eq!(cst.reconstruct(), source);
    }

    #[test]
    fn test_lossless_roundtrip_outer_doc() {
        let source = "/// Outer doc\n/// Second line\nfn foo() {}\n";
        let cst = SourceFileCst::parse(source);
        assert_eq!(cst.reconstruct(), source);
    }

    #[test]
    fn test_lossless_roundtrip_inner_doc() {
        let source = "//! Module doc\n//! Second line\n\nfn bar() {}\n";
        let cst = SourceFileCst::parse(source);
        assert_eq!(cst.reconstruct(), source);
    }

    #[test]
    fn test_lossless_roundtrip_crlf() {
        let source = "/// Doc with CRLF\r\n/// Second line\r\nfn crlf() {}\r\n";
        let cst = SourceFileCst::parse(source);
        assert_eq!(cst.reconstruct(), source);
    }

    #[test]
    fn test_lossless_roundtrip_no_trailing_newline() {
        let source = "/// Doc without newline\nfn eof() {}";
        let cst = SourceFileCst::parse(source);
        assert_eq!(cst.reconstruct(), source);
    }

    #[test]
    fn test_lossless_roundtrip_table_and_fences() {
        let source = "/// Documentation\n///\n/// ```rust\n/// let x = 1;\n/// ```\n///\n/// | Col A | Col B |\n/// |:------|------:|\n/// | 1     | 2     |\n///\n/// [label]: url\nfn test() {}\n";
        let cst = SourceFileCst::parse(source);
        assert_eq!(cst.reconstruct(), source);

        // Verify CST nodes in doc block
        if let FileChunk::DocBlock(block) = &cst.chunks[0] {
            assert_eq!(block.nodes.len(), 7);
            assert!(matches!(block.nodes[0], DocNode::Paragraph { .. }));
            assert!(matches!(block.nodes[1], DocNode::BlankLine { .. }));
            assert!(matches!(block.nodes[2], DocNode::CodeFence { .. }));
            assert!(matches!(block.nodes[3], DocNode::BlankLine { .. }));
            assert!(matches!(block.nodes[4], DocNode::Table { .. }));
            assert!(matches!(block.nodes[5], DocNode::BlankLine { .. }));
            assert!(matches!(
                block.nodes[6],
                DocNode::ReferenceDefinitions { .. }
            ));
        } else {
            panic!("Expected DocBlock chunk");
        }
    }

    #[test]
    fn test_source_spans_accuracy() {
        let source = "fn line1() {}\n/// Line 2\n/// Line 3\nfn line4() {}\n";
        let cst = SourceFileCst::parse(source);

        assert_eq!(cst.chunks.len(), 3);
        assert_eq!(cst.chunks[0].span(), SourceSpan::new(1, 1));
        assert_eq!(cst.chunks[1].span(), SourceSpan::new(2, 3));
        assert_eq!(cst.chunks[2].span(), SourceSpan::new(4, 4));
    }
}
