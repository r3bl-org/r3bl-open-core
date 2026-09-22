// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// XMARK: marker to skip rustdoc-fmt formatting for a file
// rustdoc-fmt: skip

//! Command-line argument parsing for cargo-rustdoc-fmt.

use crate::cargo_rustdoc_fmt::types::{FormatOptions, LineRange};
use clap::Parser;
use std::path::PathBuf;

// cspell:words fences

/// Format markdown tables and links in Rust documentation comments.
#[derive(Debug, Default, Parser)]
#[command(
    name = "cargo-rustdoc-fmt",
    about = "Format markdown tables and links in Rust documentation comments",
    long_about = "A cargo subcommand to format markdown tables and convert inline links \
                  to reference-style links within rustdoc comments (/// and //!).\n\n\
                  By default (no args), formats git-changed files (staged/unstaged changes, \
                  or files from last commit if clean).\n\n\
                  Use --workspace to format entire workspace, or provide specific paths.\n\n\
                  PROTECTED CONTENT:\n\
                  - Files with `// rustdoc-fmt: skip` are skipped entirely\n\
                  - Files with #![rustfmt::skip] or #![cfg_attr(rustfmt, rustfmt_skip)] are skipped entirely\n\
                  - HTML tags are preserved (entire rustdoc block skipped)\n\
                  - Blockquotes (>) are preserved (entire rustdoc block skipped)\n\
                  - Code fence contents are generally protected by markdown parsers\n\
                  - For files with complex code fence examples, use rustfmt_skip",
    version
)]
#[allow(clippy::struct_excessive_bools)]
pub struct CLIArg {
    /// Check formatting without modifying files
    #[arg(long, short = 'c')]
    pub check: bool,

    /// Only format tables (skip link conversion)
    #[arg(long)]
    pub tables_only: bool,

    /// Only convert links (skip table formatting)
    #[arg(long)]
    pub links_only: bool,

    /// Only link known terms (skip table formatting and link conversion)
    #[arg(long)]
    pub terms_only: bool,

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

    /// Format only documentation within the specified line range (e.g. 10:20, 10..20,
    /// 42)
    #[arg(
        long = "lines",
        value_name = "RANGE",
        allow_hyphen_values = true,
        conflicts_with_all = ["links_only", "terms_only", "terms_file", "workspace"]
    )]
    pub lines: Option<LineRange>,

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
    /// - `--lines` is specified without an explicit target file path.
    /// - More than one target file path is provided with `--lines`.
    /// - The specified path does not exist, is a directory, or is not a `.rs` file.
    /// - Both `--lines` and `--workspace` are specified.
    pub fn validate(&self) -> miette::Result<()> {
        if self.lines.is_some() {
            if self.paths.is_empty() {
                return Err(miette::miette!(
                    "--lines requires an explicit target file path (e.g. 'cargo rustdoc-fmt --lines 10:20 src/lib.rs')"
                ));
            }
            if self.paths.len() > 1 {
                return Err(miette::miette!(
                    "--lines only supports formatting a single file at a time, but {} paths were provided",
                    self.paths.len()
                ));
            }
            let path = &self.paths[0];
            if !path.exists() {
                return Err(miette::miette!("File not found: '{}'", path.display()));
            }
            if path.is_dir() {
                return Err(miette::miette!(
                    "--lines requires a file, but directory was provided: '{}'",
                    path.display()
                ));
            }
            let is_rs = path
                .extension()
                .and_then(|s| s.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("rs"));
            if !is_rs {
                return Err(miette::miette!(
                    "--lines requires a Rust source file (.rs), but got: '{}'",
                    path.display()
                ));
            }
        }
        Ok(())
    }

    /// Converts CLI arguments to `FormatOptions`.
    #[must_use]
    pub fn to_format_options(&self) -> FormatOptions {
        if self.lines.is_some() {
            FormatOptions {
                format_tables: true,
                convert_links: false,
                link_terms: false,
                line_range: self.lines,
                check_only: self.check,
                verbose: self.verbose,
            }
        } else {
            FormatOptions {
                format_tables: !self.links_only && !self.terms_only,
                convert_links: !self.tables_only && !self.terms_only,
                link_terms: !self.tables_only && !self.links_only,
                line_range: None,
                check_only: self.check,
                verbose: self.verbose,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn verify_cli_schema() { CLIArg::command().debug_assert(); }

    #[test]
    fn test_cli_defaults() {
        let cli = CLIArg::default();

        let opts = cli.to_format_options();
        assert!(opts.format_tables);
        assert!(opts.convert_links);
        assert!(opts.link_terms);
        assert!(opts.line_range.is_none());
    }

    #[test]
    fn test_cli_tables_only() {
        let cli = CLIArg {
            tables_only: true,
            ..Default::default()
        };

        let opts = cli.to_format_options();
        assert!(opts.format_tables);
        assert!(!opts.convert_links);
        assert!(!opts.link_terms);
    }

    #[test]
    fn test_cli_terms_only() {
        let cli = CLIArg {
            terms_only: true,
            ..Default::default()
        };

        let opts = cli.to_format_options();
        assert!(!opts.format_tables);
        assert!(!opts.convert_links);
        assert!(opts.link_terms);
    }

    #[test]
    fn test_cli_lines_argument() {
        let cli = CLIArg {
            lines: Some(LineRange::new(10, 20)),
            ..Default::default()
        };

        let opts = cli.to_format_options();
        assert!(opts.format_tables);
        assert!(!opts.convert_links);
        assert!(!opts.link_terms);
        assert_eq!(opts.line_range, Some(LineRange::new(10, 20)));
    }

    #[test]
    fn test_cli_validate_lines() {
        // Missing paths
        let cli = CLIArg {
            lines: Some(LineRange::new(10, 20)),
            paths: vec![],
            ..Default::default()
        };
        assert!(cli.validate().is_err());

        // Multiple paths
        let cli = CLIArg {
            lines: Some(LineRange::new(10, 20)),
            paths: vec![PathBuf::from("a.rs"), PathBuf::from("b.rs")],
            ..Default::default()
        };
        assert!(cli.validate().is_err());

        // Nonexistent path
        let cli = CLIArg {
            lines: Some(LineRange::new(10, 20)),
            paths: vec![PathBuf::from("/nonexistent_file_xyz_123.rs")],
            ..Default::default()
        };
        assert!(cli.validate().is_err());
    }
}
