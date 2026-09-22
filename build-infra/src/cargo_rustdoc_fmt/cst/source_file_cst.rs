// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// rustdoc-fmt: skip

//! Top-level source file Concrete Syntax Tree ([`SourceFileCst`]), [`FileChunk`],
//! [`LineEndings`], and [`TrailingNewline`].

use super::{DocBlockCst, DocNode, detect_doc_comment, format_table_data};
use crate::cargo_rustdoc_fmt::{constants::{CONTENT_SPACE_CHAR, CRLF_LINE_ENDING,
                                           LF_LINE_ENDING, LF_LINE_ENDING_CHAR},
                               types::FormatStatus};
use r3bl_tui::{CRow, c_row};
use std::ops::RangeInclusive;

/// Represents the entire source file partitioned into code and doc comment `CST`
/// instances.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFileCst {
    pub chunks: Vec<FileChunk>,
    pub line_endings: LineEndings,
    pub trailing_newline: TrailingNewline,
}

impl SourceFileCst {
    /// Parses Rust source code into a lossless Concrete Syntax Tree (`CST`).
    #[must_use]
    pub fn parse(source: &str) -> Self {
        if source.is_empty() {
            return Self {
                chunks: Vec::new(),
                line_endings: LineEndings::Unix,
                trailing_newline: TrailingNewline::Absent,
            };
        }

        let line_endings = LineEndings::detect(source);
        let trailing_newline = TrailingNewline::detect(source);
        let lines: Vec<&str> = source.lines().collect();

        let mut chunks = Vec::new();
        let mut line_idx = 0;

        while line_idx < lines.len() {
            let line = lines[line_idx];

            if let Some(doc_line) = detect_doc_comment(line) {
                // Collect contiguous doc comment lines with the same comment type,
                // marker, and indentation.
                let block_start_line = c_row(line_idx);
                let mut raw_lines = Vec::new();

                while line_idx < lines.len() {
                    let cur_line = lines[line_idx];
                    if let Some(cur_doc_line) = detect_doc_comment(cur_line) {
                        if cur_doc_line == doc_line {
                            raw_lines.push(cur_line.to_string());
                            line_idx += 1;
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }

                let block_end_line = c_row(line_idx - 1);
                let span = block_start_line..=block_end_line;
                let doc_block = DocBlockCst::parse(
                    raw_lines,
                    doc_line.comment_type,
                    doc_line.indent,
                    doc_line.marker,
                    span,
                );
                chunks.push(FileChunk::DocBlock(doc_block));
            } else {
                // Collect contiguous non-doc lines (Rust code, blank lines, regular
                // comments).
                let code_start_line = c_row(line_idx);
                let mut code_lines = Vec::new();

                while line_idx < lines.len() {
                    let cur_line = lines[line_idx];
                    if detect_doc_comment(cur_line).is_some() {
                        break;
                    }
                    code_lines.push(cur_line.to_string());
                    line_idx += 1;
                }

                let code_end_line = c_row(line_idx - 1);
                let span = code_start_line..=code_end_line;
                chunks.push(FileChunk::Code {
                    span,
                    lines: code_lines,
                });
            }
        }

        Self {
            chunks,
            line_endings,
            trailing_newline,
        }
    }

    /// Reconstructs the source file from the `CST`.
    ///
    /// Unmodified nodes output their original `raw_lines` verbatim, ensuring 100%
    /// byte-for-byte preservation for untouched code and documentation.
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn reconstruct(&self) -> String {
        let mut output = String::new();
        let mut first_line = true;
        let newline = self.line_endings.as_str();

        for chunk in &self.chunks {
            match chunk {
                FileChunk::Code { lines, .. } => {
                    for line in lines {
                        if !first_line {
                            output.push_str(newline);
                        }
                        output.push_str(line);
                        first_line = false;
                    }
                }
                FileChunk::DocBlock(block) => {
                    let block_status = block.status();
                    let marker = block.comment_type.marker();

                    for node in &block.nodes {
                        match node {
                            DocNode::Paragraph {
                                lines,
                                raw_lines,
                                status,
                                ..
                            } => {
                                if *status == FormatStatus::Modified {
                                    for line in lines {
                                        if !first_line {
                                            output.push_str(newline);
                                        }
                                        output.push_str(&block.indent);
                                        output.push_str(marker);
                                        if !line.is_empty() {
                                            output.push(CONTENT_SPACE_CHAR);
                                            output.push_str(line);
                                        }
                                        first_line = false;
                                    }
                                } else {
                                    for raw_line in raw_lines {
                                        if !first_line {
                                            output.push_str(newline);
                                        }
                                        output.push_str(raw_line);
                                        first_line = false;
                                    }
                                }
                            }
                            DocNode::Table {
                                table,
                                content_indent,
                                raw_lines,
                                status,
                                ..
                            } => {
                                if *status == FormatStatus::Modified {
                                    let formatted_rows = format_table_data(table);
                                    for row in formatted_rows {
                                        if !first_line {
                                            output.push_str(newline);
                                        }
                                        output.push_str(&block.indent);
                                        output.push_str(marker);
                                        output.push(CONTENT_SPACE_CHAR);
                                        output.push_str(content_indent);
                                        output.push_str(&row);
                                        first_line = false;
                                    }
                                } else {
                                    for raw_line in raw_lines {
                                        if !first_line {
                                            output.push_str(newline);
                                        }
                                        output.push_str(raw_line);
                                        first_line = false;
                                    }
                                }
                            }
                            DocNode::ReferenceDefinitions {
                                definitions,
                                raw_lines,
                                status,
                                ..
                            } => {
                                if *status == FormatStatus::Modified {
                                    for ref_def in definitions {
                                        let rendered = ref_def.render();
                                        for def_line in rendered.lines() {
                                            if !first_line {
                                                output.push_str(newline);
                                            }
                                            output.push_str(&block.indent);
                                            output.push_str(marker);
                                            if !def_line.is_empty() {
                                                output.push(CONTENT_SPACE_CHAR);
                                                output.push_str(def_line);
                                            }
                                            first_line = false;
                                        }
                                    }
                                } else {
                                    for raw_line in raw_lines {
                                        if !first_line {
                                            output.push_str(newline);
                                        }
                                        output.push_str(raw_line);
                                        first_line = false;
                                    }
                                }
                            }
                            DocNode::CodeFence { lines, .. } => {
                                for raw_line in lines {
                                    if !first_line {
                                        output.push_str(newline);
                                    }
                                    output.push_str(raw_line);
                                    first_line = false;
                                }
                            }
                            DocNode::BlankLine { raw_line, .. } => {
                                if !first_line {
                                    output.push_str(newline);
                                }
                                if block_status == FormatStatus::Modified {
                                    output.push_str(&block.indent);
                                    output.push_str(marker);
                                } else {
                                    output.push_str(raw_line);
                                }
                                first_line = false;
                            }
                        }
                    }
                }
            }
        }

        if self.trailing_newline == TrailingNewline::Present && !output.is_empty() {
            output.push_str(newline);
        }

        output
    }
}

/// A chunk of a source file: either Rust code or a contiguous doc comment block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileChunk {
    /// Rust code, regular comments, blank lines, attributes.
    Code {
        span: RangeInclusive<CRow>,
        lines: Vec<String>,
    },
    /// A contiguous block of doc comments (`///` or `//!`).
    DocBlock(DocBlockCst),
}

impl FileChunk {
    /// Returns the 0-based inclusive source line span for this file chunk.
    #[must_use]
    pub fn span(&self) -> RangeInclusive<CRow> {
        match self {
            Self::Code { span, .. } => span.clone(),
            Self::DocBlock(block) => block.span.clone(),
        }
    }
}

/// Line ending style detected in a source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEndings {
    /// Windows CRLF (`\r\n`).
    Windows,
    /// Unix LF (`\n`).
    Unix,
}

impl LineEndings {
    /// Detects the line ending style used in `source`.
    #[must_use]
    pub fn detect(source: &str) -> Self {
        if source.contains(CRLF_LINE_ENDING) {
            Self::Windows
        } else {
            Self::Unix
        }
    }

    /// Returns the string slice representation (`"\r\n"` or `"\n"`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Windows => CRLF_LINE_ENDING,
            Self::Unix => LF_LINE_ENDING,
        }
    }
}

/// Whether the source file ends with a trailing newline at EOF.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrailingNewline {
    /// File ends with a trailing newline (`\n` or `\r\n`).
    Present,
    /// File does not end with a trailing newline.
    Absent,
}

impl TrailingNewline {
    /// Detects whether `source` ends with a trailing newline.
    #[must_use]
    pub fn detect(source: &str) -> Self {
        if source.ends_with(LF_LINE_ENDING_CHAR) {
            Self::Present
        } else {
            Self::Absent
        }
    }
}
