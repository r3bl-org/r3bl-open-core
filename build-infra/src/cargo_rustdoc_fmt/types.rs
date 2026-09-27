// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// rustdoc-fmt: skip

//! Type definitions and CST nodes for rustdoc formatting.

use std::{fmt, num::IntErrorKind, path::PathBuf, str::FromStr};

/// Configuration options for formatting operations.
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct FormatOptions {
    /// Format markdown tables
    pub format_tables: bool,
    /// Converts inline links to reference-style
    pub convert_links: bool,
    /// Upgrades known terms to backticked+linked form with correct targets
    pub link_terms: bool,
    /// Line range filter for targeted formatting
    pub line_range: Option<LineRange>,
    /// Only check formatting, don't modify files
    pub check_only: bool,
    /// Print verbose output
    pub verbose: bool,
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            format_tables: true,
            convert_links: true,
            link_terms: true,
            line_range: None,
            check_only: false,
            verbose: false,
        }
    }
}

/// Result of processing a single file.
#[derive(Debug)]
pub struct ProcessingResult {
    /// Path to the processed file
    pub file_path: PathBuf,
    /// Whether the file was modified
    pub modified: bool,
    /// Any errors encountered
    pub errors: Vec<String>,
}

impl ProcessingResult {
    /// Creates a new processing result.
    #[must_use]
    pub fn new(file_path: PathBuf) -> Self {
        Self {
            file_path,
            modified: false,
            errors: Vec::new(),
        }
    }

    /// Marks this result as modified.
    pub fn mark_modified(&mut self) { self.modified = true; }

    /// Adds an error to this result.
    pub fn add_error(&mut self, error: String) { self.errors.push(error); }
}

/// Type of rustdoc comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentType {
    /// Inner doc comment: `//!`
    Inner,
    /// Outer doc comment: `///`
    Outer,
}

/// A block of rustdoc comments extracted from source code.
/// Maintained for backwards compatibility while migrating to CST.
#[derive(Debug, Clone)]
pub struct RustdocBlock {
    /// Type of comment (`///` or `//!`)
    pub comment_type: CommentType,
    /// Starting line number (0-indexed)
    pub start_line: usize,
    /// Ending line number (0-indexed, inclusive)
    pub end_line: usize,
    /// Content lines (without comment markers or indentation)
    pub lines: Vec<String>,
    /// Original indentation to preserve
    pub indentation: String,
}

/// Result type for formatter operations.
pub type FormatterResult<T> = miette::Result<T>;

// ---------------------------------------------------------------------------------
// CST Types & Source Spans
// ---------------------------------------------------------------------------------

/// A 1-indexed inclusive source line span: `[start_line, end_line]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceSpan {
    pub start_line: usize,
    pub end_line: usize,
}

impl SourceSpan {
    /// Creates a new 1-indexed inclusive source span.
    #[must_use]
    pub const fn new(start_line: usize, end_line: usize) -> Self {
        Self {
            start_line,
            end_line,
        }
    }

    /// Checks if this span overlaps with a `LineRange`.
    #[must_use]
    pub fn overlaps(&self, range: &LineRange) -> bool {
        self.start_line <= range.end && self.end_line >= range.start
    }

    /// Returns the number of lines covered by this span.
    #[must_use]
    pub const fn line_count(&self) -> usize {
        self.end_line.saturating_sub(self.start_line) + 1
    }
}

/// Error encountered when parsing a `LineRange` from CLI input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineRangeError {
    EmptyInput,
    NegativeLineNumber,
    ZeroLineNumber,
    StartGreaterThanEnd { start: usize, end: usize },
    OpenEndedRange,
    MultipleDelimiters,
    InvalidNumber(String),
    IntegerOverflow,
}

impl fmt::Display for LineRangeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyInput => write!(f, "Line range cannot be empty"),
            Self::NegativeLineNumber => {
                write!(f, "Line numbers must be positive integers")
            }
            Self::ZeroLineNumber => {
                write!(f, "Line numbers are 1-indexed; line 0 is invalid")
            }
            Self::StartGreaterThanEnd { start, end } => {
                write!(
                    f,
                    "Start line ({start}) cannot be greater than end line ({end})"
                )
            }
            Self::OpenEndedRange => {
                write!(
                    f,
                    "Open-ended line ranges are not supported; specify both start and end"
                )
            }
            Self::MultipleDelimiters => {
                write!(
                    f,
                    "Multiple range delimiters found; expected exactly one delimiter (e.g. '10:20', '10..20')"
                )
            }
            Self::InvalidNumber(s) => write!(f, "Invalid line number: '{s}'"),
            Self::IntegerOverflow => {
                write!(f, "Line number exceeds maximum supported integer value")
            }
        }
    }
}

impl std::error::Error for LineRangeError {}

/// A 1-indexed inclusive line range requested for formatting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineRange {
    pub start: usize,
    pub end: usize,
}

impl LineRange {
    /// Creates a new 1-indexed inclusive line range.
    #[must_use]
    pub const fn new(start: usize, end: usize) -> Self { Self { start, end } }

    /// Returns the 0-indexed start line number.
    #[must_use]
    pub const fn start_0_indexed(&self) -> usize { self.start.saturating_sub(1) }

    /// Returns the 0-indexed end line number.
    #[must_use]
    pub const fn end_0_indexed(&self) -> usize { self.end.saturating_sub(1) }

    /// Checks if this range overlaps with a 0-indexed `[start, end]` range.
    #[must_use]
    pub const fn overlaps_0_indexed(&self, start: usize, end: usize) -> bool {
        self.start_0_indexed() <= end && self.end_0_indexed() >= start
    }

    /// Checks if this range overlaps with a 1-indexed `[start, end]` range.
    #[must_use]
    pub const fn overlaps_1_indexed(&self, start: usize, end: usize) -> bool {
        self.start <= end && self.end >= start
    }

    /// Checks if this range overlaps with a `SourceSpan`.
    #[must_use]
    pub fn overlaps_span(&self, span: &SourceSpan) -> bool {
        self.overlaps_1_indexed(span.start_line, span.end_line)
    }
}

impl FromStr for LineRange {
    type Err = LineRangeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err(LineRangeError::EmptyInput);
        }

        // Check for leading negative sign before delimiter check
        if trimmed.starts_with('-') {
            return Err(LineRangeError::NegativeLineNumber);
        }

        // Delimiter precedence: "..=", "..", ":", "-"
        let delimiters = ["..=", "..", ":", "-"];
        let mut detected_delim = None;

        for delim in &delimiters {
            if trimmed.contains(delim) {
                detected_delim = Some(*delim);
                break;
            }
        }

        let (lhs, rhs) = if let Some(delim) = detected_delim {
            if trimmed.matches(delim).count() > 1 {
                return Err(LineRangeError::MultipleDelimiters);
            }
            let mut parts = trimmed.splitn(2, delim);
            let left = parts.next().unwrap_or("");
            let right = parts.next().unwrap_or("");
            (left.trim(), right.trim())
        } else {
            // Single line
            (trimmed, trimmed)
        };

        if lhs.is_empty() || rhs.is_empty() {
            return Err(LineRangeError::OpenEndedRange);
        }

        if lhs.starts_with('-') || rhs.starts_with('-') {
            return Err(LineRangeError::NegativeLineNumber);
        }

        let start = lhs.parse::<usize>().map_err(|e| match e.kind() {
            IntErrorKind::PosOverflow => LineRangeError::IntegerOverflow,
            _ => LineRangeError::InvalidNumber(lhs.to_string()),
        })?;

        let end = rhs.parse::<usize>().map_err(|e| match e.kind() {
            IntErrorKind::PosOverflow => LineRangeError::IntegerOverflow,
            _ => LineRangeError::InvalidNumber(rhs.to_string()),
        })?;

        if start == 0 || end == 0 {
            return Err(LineRangeError::ZeroLineNumber);
        }

        if start > end {
            return Err(LineRangeError::StartGreaterThanEnd { start, end });
        }

        Ok(LineRange { start, end })
    }
}

/// Column alignment in a markdown table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnAlignment {
    Left,
    Right,
    Center,
    None,
}

/// Parsed markdown table structure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableData {
    pub headers: Vec<String>,
    pub alignments: Vec<ColumnAlignment>,
    pub rows: Vec<Vec<String>>,
}

/// Parsed reference link definition: `[label]: target "title"`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkReference {
    pub label: String,
    pub target: String,
    pub title: Option<String>,
}

/// Concrete Syntax Tree node inside a doc comment block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocNode {
    /// Prose: paragraphs, bullet/numbered lists, headings, blockquotes.
    Paragraph {
        span: SourceSpan,
        lines: Vec<String>,
        raw_lines: Vec<String>,
        modified: bool,
    },
    /// Code fence: ```` ```[lang] ... ``` ````
    CodeFence {
        span: SourceSpan,
        language: Option<String>,
        lines: Vec<String>,
    },
    /// Markdown table
    Table {
        span: SourceSpan,
        content_indent: String,
        table: TableData,
        raw_lines: Vec<String>,
        modified: bool,
    },
    /// Reference link definitions at bottom of doc block: `[label]: target`
    ReferenceDefinitions {
        span: SourceSpan,
        definitions: Vec<LinkReference>,
        raw_lines: Vec<String>,
        modified: bool,
    },
    /// Blank doc comment line (`///` or `//!` with no text)
    BlankLine { span: SourceSpan, raw_line: String },
}

impl DocNode {
    /// Returns the source line span for this node.
    #[must_use]
    pub fn span(&self) -> SourceSpan {
        match self {
            Self::Paragraph { span, .. }
            | Self::CodeFence { span, .. }
            | Self::Table { span, .. }
            | Self::ReferenceDefinitions { span, .. }
            | Self::BlankLine { span, .. } => *span,
        }
    }

    /// Returns whether this node was modified by a formatting pass.
    #[must_use]
    pub fn is_modified(&self) -> bool {
        match self {
            Self::Paragraph { modified, .. }
            | Self::Table { modified, .. }
            | Self::ReferenceDefinitions { modified, .. } => *modified,
            Self::CodeFence { .. } | Self::BlankLine { .. } => false,
        }
    }

    /// Returns the original raw lines for this node.
    #[must_use]
    pub fn raw_lines(&self) -> &[String] {
        match self {
            Self::Paragraph { raw_lines, .. }
            | Self::Table { raw_lines, .. }
            | Self::ReferenceDefinitions { raw_lines, .. } => raw_lines,
            Self::CodeFence { lines, .. } => lines,
            Self::BlankLine { raw_line, .. } => std::slice::from_ref(raw_line),
        }
    }
}

/// A contiguous block of doc comments (`///` or `//!`) parsed into CST nodes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocBlockCst {
    pub span: SourceSpan,
    pub comment_type: CommentType,
    pub indent: String,
    pub nodes: Vec<DocNode>,
}

/// A chunk of a source file: either Rust code or a contiguous doc comment block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileChunk {
    /// Rust code, regular comments, blank lines, attributes.
    Code {
        span: SourceSpan,
        lines: Vec<String>,
    },
    /// A contiguous block of doc comments (`///` or `//!`).
    DocBlock(DocBlockCst),
}

impl FileChunk {
    /// Returns the source line span for this file chunk.
    #[must_use]
    pub fn span(&self) -> SourceSpan {
        match self {
            Self::Code { span, .. } => *span,
            Self::DocBlock(block) => block.span,
        }
    }
}

/// Represents the entire source file partitioned into code and doc comment CSTs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFileCst {
    pub path: Option<PathBuf>,
    pub chunks: Vec<FileChunk>,
    pub newline: &'static str,
    pub has_trailing_newline: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_options_default() {
        let opts = FormatOptions::default();
        assert!(opts.format_tables);
        assert!(opts.convert_links);
        assert!(opts.link_terms);
        assert!(opts.line_range.is_none());
        assert!(!opts.check_only);
        assert!(!opts.verbose);
    }

    #[test]
    fn test_processing_result() {
        let mut result = ProcessingResult::new(PathBuf::from("test.rs"));
        assert!(!result.modified);
        assert!(result.errors.is_empty());

        result.mark_modified();
        assert!(result.modified);

        result.add_error("test error".to_string());
        assert_eq!(result.errors.len(), 1);
    }

    #[test]
    fn test_line_range_parsing_valid() {
        // Colon syntax
        let r: LineRange = "10:20".parse().unwrap();
        assert_eq!(r, LineRange::new(10, 20));

        // Hyphen syntax
        let r: LineRange = "10-20".parse().unwrap();
        assert_eq!(r, LineRange::new(10, 20));

        // Rust range syntax
        let r: LineRange = "10..20".parse().unwrap();
        assert_eq!(r, LineRange::new(10, 20));
        let r: LineRange = "10..=20".parse().unwrap();
        assert_eq!(r, LineRange::new(10, 20));

        // Single line
        let r: LineRange = "42".parse().unwrap();
        assert_eq!(r, LineRange::new(42, 42));

        // Whitespace handling
        let r: LineRange = " 10 : 20 ".parse().unwrap();
        assert_eq!(r, LineRange::new(10, 20));
        let r: LineRange = " 10 ..= 20 ".parse().unwrap();
        assert_eq!(r, LineRange::new(10, 20));
    }

    #[test]
    fn test_line_range_parsing_errors() {
        // Empty
        assert_eq!(
            "".parse::<LineRange>().unwrap_err(),
            LineRangeError::EmptyInput
        );
        assert_eq!(
            "   ".parse::<LineRange>().unwrap_err(),
            LineRangeError::EmptyInput
        );

        // Negative numbers
        assert_eq!(
            "-5".parse::<LineRange>().unwrap_err(),
            LineRangeError::NegativeLineNumber
        );
        assert_eq!(
            "-10:20".parse::<LineRange>().unwrap_err(),
            LineRangeError::NegativeLineNumber
        );
        assert_eq!(
            "10:-20".parse::<LineRange>().unwrap_err(),
            LineRangeError::NegativeLineNumber
        );

        // Zero
        assert_eq!(
            "0".parse::<LineRange>().unwrap_err(),
            LineRangeError::ZeroLineNumber
        );
        assert_eq!(
            "0:10".parse::<LineRange>().unwrap_err(),
            LineRangeError::ZeroLineNumber
        );
        assert_eq!(
            "10:0".parse::<LineRange>().unwrap_err(),
            LineRangeError::ZeroLineNumber
        );

        // Start > End
        assert_eq!(
            "20:10".parse::<LineRange>().unwrap_err(),
            LineRangeError::StartGreaterThanEnd { start: 20, end: 10 }
        );

        // Open ended
        assert_eq!(
            "10:".parse::<LineRange>().unwrap_err(),
            LineRangeError::OpenEndedRange
        );
        assert_eq!(
            ":20".parse::<LineRange>().unwrap_err(),
            LineRangeError::OpenEndedRange
        );
        assert_eq!(
            "10..".parse::<LineRange>().unwrap_err(),
            LineRangeError::OpenEndedRange
        );
        assert_eq!(
            "..20".parse::<LineRange>().unwrap_err(),
            LineRangeError::OpenEndedRange
        );

        // Multiple delimiters
        assert_eq!(
            "10:20:30".parse::<LineRange>().unwrap_err(),
            LineRangeError::MultipleDelimiters
        );
        assert_eq!(
            "10..20..30".parse::<LineRange>().unwrap_err(),
            LineRangeError::MultipleDelimiters
        );
        assert_eq!(
            "10--20".parse::<LineRange>().unwrap_err(),
            LineRangeError::MultipleDelimiters
        );

        // Non-numeric
        assert!(matches!(
            "abc".parse::<LineRange>().unwrap_err(),
            LineRangeError::InvalidNumber(_)
        ));
        assert!(matches!(
            "10:abc".parse::<LineRange>().unwrap_err(),
            LineRangeError::InvalidNumber(_)
        ));
    }

    #[test]
    fn test_line_range_overlap() {
        let r = LineRange::new(10, 20);

        // 1-indexed overlaps
        assert!(r.overlaps_1_indexed(5, 10));
        assert!(r.overlaps_1_indexed(10, 20));
        assert!(r.overlaps_1_indexed(15, 25));
        assert!(r.overlaps_1_indexed(5, 25));
        assert!(!r.overlaps_1_indexed(1, 9));
        assert!(!r.overlaps_1_indexed(21, 30));

        // 0-indexed overlaps (lines 10..=20 are 0-indexed 9..=19)
        assert!(r.overlaps_0_indexed(4, 9));
        assert!(r.overlaps_0_indexed(9, 19));
        assert!(r.overlaps_0_indexed(14, 24));
        assert!(!r.overlaps_0_indexed(0, 8));
        assert!(!r.overlaps_0_indexed(20, 29));

        // SourceSpan overlap
        let span = SourceSpan::new(15, 25);
        assert!(r.overlaps_span(&span));
        let disjoint_span = SourceSpan::new(1, 5);
        assert!(!r.overlaps_span(&disjoint_span));
    }
}
