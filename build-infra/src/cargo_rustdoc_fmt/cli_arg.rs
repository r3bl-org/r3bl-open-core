// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// XMARK: marker to skip rustdoc-fmt formatting for a file
// rustdoc-fmt: skip

//! Command-line argument parsing for cargo-rustdoc-fmt.

use crate::cargo_rustdoc_fmt::types::FormatOptions;
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

    /// Include lib.rs files (excluded by default since they serve as README templates)
    #[arg(long)]
    pub include_lib_rs: bool,

    /// Specific files or directories to format.
    /// If not provided, formats git-changed files (or entire workspace with
    /// --workspace).
    #[arg(value_name = "PATH")]
    pub paths: Vec<PathBuf>,
}

impl CLIArg {
    /// Converts CLI arguments to `FormatOptions`.
    #[must_use]
    pub fn to_format_options(&self) -> FormatOptions {
        FormatOptions {
            format_tables: !self.links_only && !self.terms_only,
            convert_links: !self.tables_only && !self.terms_only,
            link_terms: !self.tables_only && !self.links_only,
            check_only: self.check,
            verbose: self.verbose,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_defaults() {
        let cli = CLIArg::default();

        let opts = cli.to_format_options();
        assert!(opts.format_tables);
        assert!(opts.convert_links);
        assert!(opts.link_terms);
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
}
