// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// XMARK: marker to skip rustdoc-fmt formatting for a file
// rustdoc-fmt: skip

//! Command-line argument parsing for cargo-rustdoc-fmt.

use crate::cargo_rustdoc_fmt::{constants::{DEFAULT_SKIP_FOLDER_MATCH,
                                           NEGATIVE_NUMBER_PREFIX, RANGE_DELIMITERS,
                                           RUST_FILE_EXTENSION},
                               types::{EditorLineNum, ExecutionMode, FormatOptions,
                                       PassPolicy, Verbosity, editor_line_num}};
use clap::Parser;
use r3bl_tui::CRow;
use std::{num::IntErrorKind, ops::RangeInclusive, path::PathBuf};

/// Format Markdown tables, links, technical terms, and code fences in Rust
/// documentation comments.
#[derive(Debug, Default, Parser)]
#[command(
    name = "cargo-rustdoc-fmt",
    about = "Format markdown tables, links, technical terms, and code fences in Rust documentation comments",
    long_about = "A cargo subcommand to format markdown tables, convert inline links \
                  to reference-style links, auto-link known technical terms, and strip \
                  redundant `rust` code fence tags within rustdoc comments (/// and //!).\n\n\
                  By default (no args), formats git-changed files (staged/unstaged changes, \
                  or files from last commit if clean).\n\n\
                  Use --workspace to format entire workspace, or provide specific paths.\n\n\
                  PROTECTED CONTENT:\n\
                  - Files with `// rustdoc-fmt: skip` are skipped entirely (unless `--lines-force` is used)\n\
                  - Paths matching `--skip-folder-matches` (default: `test_data`) are skipped\n\
                  - Non-doc Rust code is preserved 100% byte-for-byte via the lossless CST\n\
                  - Code fence block bodies (```) and inline backtick spans are isolated and never modified",
    version
)]
#[allow(clippy::struct_excessive_bools)]
pub struct CLIArg {
    /// Check formatting without modifying files
    #[arg(long, short = 'c')]
    pub check: bool,

    /// Only format tables (skip link conversion, term linking, and code fence
    /// normalization)
    #[arg(long)]
    pub tables_only: bool,

    /// Only convert links (skip table formatting, term linking, and code fence
    /// normalization)
    #[arg(long)]
    pub links_only: bool,

    /// Only link known terms (skip table formatting, link conversion, and code fence
    /// normalization)
    #[arg(long)]
    pub terms_only: bool,

    /// Only strip redundant `rust` language tags from code fence openings (skip table
    /// formatting, link conversion, and term linking)
    #[arg(long)]
    pub fences_only: bool,

    /// Override the embedded known-terms seed file with a custom JSONC file
    #[arg(long, value_name = "PATH")]
    pub terms_file: Option<PathBuf>,

    /// Verbose output
    #[arg(long, short = 'v')]
    pub verbose: bool,

    /// Format entire workspace instead of git-changed files
    #[arg(long, short = 'w')]
    pub workspace: bool,

    /// Skip running cargo fmt on modified files
    #[arg(long)]
    pub skip_cargo_fmt: bool,

    /// Show which files would be processed without making changes
    #[arg(long, short = 'd')]
    pub dry_run: bool,

    /// Skip files whose path components contain any of these substrings
    /// (comma-separated). Pass "" to disable folder skipping.
    #[arg(
        long = "skip-folder-matches",
        value_name = "PATTERN",
        value_delimiter = ',',
        default_values_t = [DEFAULT_SKIP_FOLDER_MATCH.to_string()]
    )]
    pub skip_folder_matches: Vec<String>,

    /// Format only documentation within the specified line range (e.g. 10:20, 10..20,
    /// 42), bypassing file-level skip directives.
    #[arg(
        long = "lines-force",
        alias = "lines",
        value_name = "RANGE",
        value_parser = parse_line_range,
        allow_hyphen_values = true,
        conflicts_with_all = ["terms_only", "terms_file", "workspace", "skip_folder_matches"]
    )]
    pub lines_force: Option<RangeInclusive<CRow>>,

    /// Specific files or directories to format.
    /// If not provided, formats git-changed files (or entire workspace with
    /// --workspace).
    #[arg(value_name = "PATH")]
    pub paths: Vec<PathBuf>,
}

impl CLIArg {
    /// Validates CLI arguments for semantic consistency.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - `--lines-force` is specified without an explicit target file path.
    /// - More than one target file path is provided with `--lines-force`.
    /// - The specified path does not exist, is a directory, or is not a `.rs` file.
    pub fn validate(&self) -> miette::Result<()> {
        if self.lines_force.is_some() {
            if self.paths.is_empty() {
                return Err(miette::miette!(
                    "--lines-force requires an explicit target file path (e.g. 'cargo rustdoc-fmt --lines-force 10:20 src/lib.rs')"
                ));
            }
            if self.paths.len() > 1 {
                return Err(miette::miette!(
                    "--lines-force only supports formatting a single file at a time, but {} paths were provided",
                    self.paths.len()
                ));
            }
            let path = &self.paths[0];
            if !path.exists() {
                return Err(miette::miette!("File not found: '{}'", path.display()));
            }
            if path.is_dir() {
                return Err(miette::miette!(
                    "--lines-force requires a file, but directory was provided: '{}'",
                    path.display()
                ));
            }
            let is_rs = path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case(RUST_FILE_EXTENSION));
            if !is_rs {
                return Err(miette::miette!(
                    "--lines-force requires a Rust source file (.rs), but got: '{}'",
                    path.display()
                ));
            }
        }
        Ok(())
    }

    /// Converts CLI arguments to [`FormatOptions`].
    #[must_use]
    pub fn to_format_options(&self) -> FormatOptions {
        let format_tables =
            PassPolicy::from(!self.links_only && !self.terms_only && !self.fences_only);
        let convert_links =
            PassPolicy::from(!self.tables_only && !self.terms_only && !self.fences_only);
        let normalize_fences =
            PassPolicy::from(!self.tables_only && !self.links_only && !self.terms_only);
        let link_terms = if self.lines_force.is_some() {
            PassPolicy::Skipped
        } else {
            PassPolicy::from(!self.tables_only && !self.links_only && !self.fences_only)
        };

        FormatOptions {
            format_tables,
            convert_links,
            link_terms,
            normalize_fences,
            line_range: self.lines_force.clone(),
            mode: ExecutionMode::from(self.check),
            verbosity: Verbosity::from(self.verbose),
        }
    }
}

/// Parses a 1-indexed CLI line range specification (e.g. `"10:20"`, `"10..20"`,
/// `"10..=20"`, `"10-20"`, or `"42"`) into a 0-based inclusive [`RangeInclusive<CRow>`].
///
/// # Errors
///
/// Returns [`LineRangeError`] if the input is empty, contains zero or negative line
/// numbers, is open-ended, has multiple delimiters, or has `start > end`.
pub fn parse_line_range(range_str: &str) -> Result<RangeInclusive<CRow>, LineRangeError> {
    let trimmed = range_str.trim();
    if trimmed.is_empty() {
        return Err(LineRangeError::EmptyInput);
    }

    // Check for leading negative sign before delimiter check.
    if trimmed.starts_with(NEGATIVE_NUMBER_PREFIX) {
        return Err(LineRangeError::NegativeLineNumber);
    }

    let mut detected_delim = None;
    for delim in &RANGE_DELIMITERS {
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
        // Single line.
        (trimmed, trimmed)
    };

    if lhs.is_empty() || rhs.is_empty() {
        return Err(LineRangeError::OpenEndedRange);
    }

    if lhs.starts_with(NEGATIVE_NUMBER_PREFIX) || rhs.starts_with(NEGATIVE_NUMBER_PREFIX)
    {
        return Err(LineRangeError::NegativeLineNumber);
    }

    let start_raw = lhs.parse::<usize>().map_err(|err| match err.kind() {
        IntErrorKind::PosOverflow => LineRangeError::IntegerOverflow,
        _ => LineRangeError::InvalidNumber(lhs.to_string()),
    })?;

    let end_raw = rhs.parse::<usize>().map_err(|err| match err.kind() {
        IntErrorKind::PosOverflow => LineRangeError::IntegerOverflow,
        _ => LineRangeError::InvalidNumber(rhs.to_string()),
    })?;

    if start_raw == 0 || end_raw == 0 {
        return Err(LineRangeError::ZeroLineNumber);
    }

    let start = editor_line_num(start_raw);
    let end = editor_line_num(end_raw);

    if start > end {
        return Err(LineRangeError::StartGreaterThanEnd { start, end });
    }

    Ok(EditorLineNum::to_c_row_range(start, end))
}

/// Error encountered when parsing a 1-indexed line range from CLI input into a 0-based
/// [`RangeInclusive<CRow>`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, miette::Diagnostic)]
pub enum LineRangeError {
    #[error("Line range cannot be empty")]
    EmptyInput,

    #[error("Line numbers must be positive integers")]
    NegativeLineNumber,

    #[error("Line numbers are 1-indexed; line 0 is invalid")]
    ZeroLineNumber,

    #[error("Start line ({start}) cannot be greater than end line ({end})")]
    StartGreaterThanEnd {
        start: EditorLineNum,
        end: EditorLineNum,
    },

    #[error("Open-ended line ranges are not supported; specify both start and end")]
    OpenEndedRange,

    #[error(
        "Multiple range delimiters found; expected exactly one delimiter (e.g. '10:20', '10..20')"
    )]
    MultipleDelimiters,

    #[error("Invalid line number: '{0}'")]
    InvalidNumber(String),

    #[error("Line number exceeds maximum supported integer value")]
    IntegerOverflow,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;
    use r3bl_tui::c_row;
    use tempfile::TempDir;

    #[test]
    fn verify_cli_schema() { CLIArg::command().debug_assert(); }

    #[test]
    fn test_cli_defaults() {
        let cli = CLIArg::default();
        assert!(!cli.fences_only);

        let opts = cli.to_format_options();
        assert_eq!(opts.format_tables, PassPolicy::Enabled);
        assert_eq!(opts.convert_links, PassPolicy::Enabled);
        assert_eq!(opts.link_terms, PassPolicy::Enabled);
        assert_eq!(opts.normalize_fences, PassPolicy::Enabled);
        assert!(opts.line_range.is_none());
        assert_eq!(opts.mode, ExecutionMode::ApplyChanges);
        assert_eq!(opts.verbosity, Verbosity::Quiet);
    }

    #[test]
    fn test_cli_single_pass_flags() {
        let tables_only = CLIArg {
            tables_only: true,
            ..Default::default()
        }
        .to_format_options();
        assert_eq!(tables_only.format_tables, PassPolicy::Enabled);
        assert_eq!(tables_only.convert_links, PassPolicy::Skipped);
        assert_eq!(tables_only.link_terms, PassPolicy::Skipped);
        assert_eq!(tables_only.normalize_fences, PassPolicy::Skipped);

        let links_only = CLIArg {
            links_only: true,
            ..Default::default()
        }
        .to_format_options();
        assert_eq!(links_only.format_tables, PassPolicy::Skipped);
        assert_eq!(links_only.convert_links, PassPolicy::Enabled);
        assert_eq!(links_only.link_terms, PassPolicy::Skipped);
        assert_eq!(links_only.normalize_fences, PassPolicy::Skipped);

        let terms_only = CLIArg {
            terms_only: true,
            ..Default::default()
        }
        .to_format_options();
        assert_eq!(terms_only.format_tables, PassPolicy::Skipped);
        assert_eq!(terms_only.convert_links, PassPolicy::Skipped);
        assert_eq!(terms_only.link_terms, PassPolicy::Enabled);
        assert_eq!(terms_only.normalize_fences, PassPolicy::Skipped);

        let fences_only = CLIArg {
            fences_only: true,
            ..Default::default()
        }
        .to_format_options();
        assert_eq!(fences_only.format_tables, PassPolicy::Skipped);
        assert_eq!(fences_only.convert_links, PassPolicy::Skipped);
        assert_eq!(fences_only.link_terms, PassPolicy::Skipped);
        assert_eq!(fences_only.normalize_fences, PassPolicy::Enabled);
    }

    #[test]
    fn test_cli_lines_argument() {
        let range =
            EditorLineNum::to_c_row_range(editor_line_num(10), editor_line_num(20));
        let cli = CLIArg {
            lines_force: Some(range.clone()),
            ..Default::default()
        };

        let opts = cli.to_format_options();
        assert_eq!(opts.format_tables, PassPolicy::Enabled);
        assert_eq!(opts.convert_links, PassPolicy::Enabled);
        assert_eq!(opts.link_terms, PassPolicy::Skipped);
        assert_eq!(opts.normalize_fences, PassPolicy::Enabled);
        assert_eq!(opts.line_range, Some(range));
    }

    #[test]
    fn test_cli_validate_lines() {
        let range_10_20 =
            EditorLineNum::to_c_row_range(editor_line_num(10), editor_line_num(20));
        let range_1_1 =
            EditorLineNum::to_c_row_range(editor_line_num(1), editor_line_num(1));

        // Missing paths.
        let cli = CLIArg {
            lines_force: Some(range_10_20.clone()),
            paths: vec![],
            ..Default::default()
        };
        assert!(cli.validate().is_err());

        // Multiple paths.
        let cli = CLIArg {
            lines_force: Some(range_10_20.clone()),
            paths: vec![PathBuf::from("a.rs"), PathBuf::from("b.rs")],
            ..Default::default()
        };
        assert!(cli.validate().is_err());

        // Nonexistent path.
        let cli = CLIArg {
            lines_force: Some(range_10_20.clone()),
            paths: vec![PathBuf::from("/nonexistent_file_xyz_123.rs")],
            ..Default::default()
        };
        assert!(cli.validate().is_err());

        let temp_dir = TempDir::new().unwrap();

        // Directory path instead of a file.
        let cli = CLIArg {
            lines_force: Some(range_10_20),
            paths: vec![temp_dir.path().to_path_buf()],
            ..Default::default()
        };
        assert!(cli.validate().is_err());

        // Non-Rust file (.txt instead of .rs).
        let txt_file = temp_dir.path().join("notes.txt");
        std::fs::write(&txt_file, "hello").unwrap();
        let cli = CLIArg {
            lines_force: Some(range_1_1.clone()),
            paths: vec![txt_file],
            ..Default::default()
        };
        assert!(cli.validate().is_err());

        // Valid .rs file succeeds.
        let rs_file = temp_dir.path().join("valid.rs");
        std::fs::write(&rs_file, "fn main() {}\n").unwrap();
        let cli = CLIArg {
            lines_force: Some(range_1_1),
            paths: vec![rs_file],
            ..Default::default()
        };
        assert!(cli.validate().is_ok());
    }

    #[test]
    fn test_cli_skip_folder_matches() {
        // Default value when omitted is ["test_data"].
        let cli = CLIArg::try_parse_from(["cargo-rustdoc-fmt"]).unwrap();
        assert_eq!(cli.skip_folder_matches, vec!["test_data".to_string()]);

        // Custom comma-separated patterns.
        let cli = CLIArg::try_parse_from([
            "cargo-rustdoc-fmt",
            "--skip-folder-matches=testdata,snapshots",
        ])
        .unwrap();
        assert_eq!(
            cli.skip_folder_matches,
            vec!["testdata".to_string(), "snapshots".to_string()]
        );

        // Empty string disables folder skipping.
        let cli = CLIArg::try_parse_from(["cargo-rustdoc-fmt", "--skip-folder-matches="])
            .unwrap();
        assert_eq!(cli.skip_folder_matches, vec![String::new()]);

        // Conflicts with --lines-force.
        let result = CLIArg::try_parse_from([
            "cargo-rustdoc-fmt",
            "--lines-force",
            "1:10",
            "--skip-folder-matches=testdata",
            "src/lib.rs",
        ]);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_line_range_valid() {
        // Colon syntax (1-indexed CLI input "10:20" -> 0-based c_row(9)..=c_row(19)).
        let range = parse_line_range("10:20").unwrap();
        assert_eq!(
            range,
            EditorLineNum::to_c_row_range(editor_line_num(10), editor_line_num(20))
        );
        assert_eq!(range, c_row(9)..=c_row(19));

        // Hyphen syntax.
        let range = parse_line_range("10-20").unwrap();
        assert_eq!(
            range,
            EditorLineNum::to_c_row_range(editor_line_num(10), editor_line_num(20))
        );

        // Rust range syntax.
        let range = parse_line_range("10..20").unwrap();
        assert_eq!(
            range,
            EditorLineNum::to_c_row_range(editor_line_num(10), editor_line_num(20))
        );
        let range = parse_line_range("10..=20").unwrap();
        assert_eq!(
            range,
            EditorLineNum::to_c_row_range(editor_line_num(10), editor_line_num(20))
        );

        // Single line (1-indexed line 42 -> 0-based c_row(41)..=c_row(41)).
        let range = parse_line_range("42").unwrap();
        assert_eq!(
            range,
            EditorLineNum::to_c_row_range(editor_line_num(42), editor_line_num(42))
        );
        assert_eq!(range, c_row(41)..=c_row(41));

        // Whitespace handling.
        let range = parse_line_range(" 10 : 20 ").unwrap();
        assert_eq!(
            range,
            EditorLineNum::to_c_row_range(editor_line_num(10), editor_line_num(20))
        );
        let range = parse_line_range(" 10 ..= 20 ").unwrap();
        assert_eq!(
            range,
            EditorLineNum::to_c_row_range(editor_line_num(10), editor_line_num(20))
        );
    }

    #[test]
    fn test_parse_line_range_errors() {
        // Empty.
        assert_eq!(
            parse_line_range("").unwrap_err(),
            LineRangeError::EmptyInput
        );
        assert_eq!(
            parse_line_range("   ").unwrap_err(),
            LineRangeError::EmptyInput
        );

        // Negative numbers.
        assert_eq!(
            parse_line_range("-5").unwrap_err(),
            LineRangeError::NegativeLineNumber
        );
        assert_eq!(
            parse_line_range("-10:20").unwrap_err(),
            LineRangeError::NegativeLineNumber
        );
        assert_eq!(
            parse_line_range("10:-20").unwrap_err(),
            LineRangeError::NegativeLineNumber
        );

        // Zero.
        assert_eq!(
            parse_line_range("0").unwrap_err(),
            LineRangeError::ZeroLineNumber
        );
        assert_eq!(
            parse_line_range("0:10").unwrap_err(),
            LineRangeError::ZeroLineNumber
        );
        assert_eq!(
            parse_line_range("10:0").unwrap_err(),
            LineRangeError::ZeroLineNumber
        );

        // Start > End.
        assert_eq!(
            parse_line_range("20:10").unwrap_err(),
            LineRangeError::StartGreaterThanEnd {
                start: editor_line_num(20),
                end: editor_line_num(10),
            }
        );

        // Open ended.
        assert_eq!(
            parse_line_range("10:").unwrap_err(),
            LineRangeError::OpenEndedRange
        );
        assert_eq!(
            parse_line_range(":20").unwrap_err(),
            LineRangeError::OpenEndedRange
        );
        assert_eq!(
            parse_line_range("10..").unwrap_err(),
            LineRangeError::OpenEndedRange
        );
        assert_eq!(
            parse_line_range("..20").unwrap_err(),
            LineRangeError::OpenEndedRange
        );

        // Multiple delimiters.
        assert_eq!(
            parse_line_range("10:20:30").unwrap_err(),
            LineRangeError::MultipleDelimiters
        );
        assert_eq!(
            parse_line_range("10..20..30").unwrap_err(),
            LineRangeError::MultipleDelimiters
        );
        assert_eq!(
            parse_line_range("10--20").unwrap_err(),
            LineRangeError::MultipleDelimiters
        );

        // Non-numeric.
        assert!(matches!(
            parse_line_range("abc").unwrap_err(),
            LineRangeError::InvalidNumber(_)
        ));
        assert!(matches!(
            parse_line_range("10:abc").unwrap_err(),
            LineRangeError::InvalidNumber(_)
        ));

        // Integer overflow.
        assert_eq!(
            parse_line_range("999999999999999999999999999999").unwrap_err(),
            LineRangeError::IntegerOverflow
        );
        assert_eq!(
            parse_line_range("1:999999999999999999999999999999").unwrap_err(),
            LineRangeError::IntegerOverflow
        );
    }
}
