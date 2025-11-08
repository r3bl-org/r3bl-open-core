// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Command-line argument parsing for cargo-rustdoc-fmt.

use crate::cargo_rustdoc_fmt::types::FormatOptions;
use clap::Parser;
use std::path::PathBuf;

/// Format markdown tables and links in Rust documentation comments.
#[derive(Debug, Parser)]
#[command(
    name = "cargo-rustdoc-fmt",
    about = "Format markdown tables and links in Rust documentation comments",
    long_about = "A cargo subcommand to format markdown tables and convert inline links \
                  to reference-style links within rustdoc comments (/// and //!).\n\n\
                  By default (no args), formats git-changed files (staged/unstaged changes, \
                  or files from last commit if clean).\n\n\
                  Use --workspace to format entire workspace, or provide specific paths.",
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

    /// Verbose output
    #[arg(long, short = 'v')]
    pub verbose: bool,

    /// Format entire workspace instead of git-changed files
    #[arg(long, short = 'w')]
    pub workspace: bool,

    /// Specific files or directories to format.
    /// If not provided, formats git-changed files (or entire workspace with --workspace).
    #[arg(value_name = "PATH")]
    pub paths: Vec<PathBuf>,
}

impl CLIArg {
    /// Convert CLI arguments to `FormatOptions`.
    #[must_use] 
    pub fn to_format_options(&self) -> FormatOptions {
        FormatOptions {
            format_tables: !self.links_only,
            convert_links: !self.tables_only,
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
        let cli = CLIArg {
            check: false,
            tables_only: false,
            links_only: false,
            verbose: false,
            workspace: false,
            paths: Vec::new(),
        };

        let opts = cli.to_format_options();
        assert!(opts.format_tables);
        assert!(opts.convert_links);
    }

    #[test]
    fn test_cli_tables_only() {
        let cli = CLIArg {
            check: false,
            tables_only: true,
            links_only: false,
            verbose: false,
            workspace: false,
            paths: Vec::new(),
        };

        let opts = cli.to_format_options();
        assert!(opts.format_tables);
        assert!(!opts.convert_links);
    }
}
