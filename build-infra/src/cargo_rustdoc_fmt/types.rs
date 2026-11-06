// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// rustdoc-fmt: skip

//! Shared type definitions for `cargo-rustdoc-fmt`.
//!
//! Contains configuration options, execution policies, coordinate wrappers, and result
//! types shared across formatting passes. Concrete Syntax Tree (`CST`) node types and
//! their `impl` blocks live in [`super::cst`].

use r3bl_tui::{CRow, LengthOps, c_height};
use std::{fmt, ops::RangeInclusive};

/// Policy for whether an individual formatting pass is enabled or skipped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PassPolicy {
    /// Run this formatting pass.
    #[default]
    Enabled,
    /// Skip this formatting pass.
    Skipped,
}

impl From<bool> for PassPolicy {
    fn from(enabled: bool) -> Self {
        if enabled {
            Self::Enabled
        } else {
            Self::Skipped
        }
    }
}

/// Whether the formatter writes changes to disk or only checks if formatting is needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExecutionMode {
    /// Write formatted output back to the source files.
    #[default]
    ApplyChanges,
    /// Only check whether files would be modified; do not write to disk.
    CheckOnly,
}

impl From<bool> for ExecutionMode {
    fn from(check_only: bool) -> Self {
        if check_only {
            Self::CheckOnly
        } else {
            Self::ApplyChanges
        }
    }
}

/// Output verbosity level for the formatter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Verbosity {
    /// Standard concise output.
    #[default]
    Quiet,
    /// Detailed diagnostic output.
    Verbose,
}

impl From<bool> for Verbosity {
    fn from(verbose: bool) -> Self { if verbose { Self::Verbose } else { Self::Quiet } }
}

/// Configuration options for formatting operations.
#[derive(Debug, Clone, Default)]
pub struct FormatOptions {
    /// Format Markdown tables.
    pub format_tables: PassPolicy,

    /// Converts inline links to reference-style.
    pub convert_links: PassPolicy,

    /// Upgrades known terms to backticked+linked form with correct targets.
    pub link_terms: PassPolicy,

    /// Strip redundant `rust` language tags from code fence openings.
    pub normalize_fences: PassPolicy,

    /// 0-based inclusive line range filter for targeted formatting.
    pub line_range: Option<RangeInclusive<CRow>>,

    /// Whether to apply changes or only check formatting.
    pub mode: ExecutionMode,

    /// Output verbosity level.
    pub verbosity: Verbosity,
}

/// Indicates whether a `CST` formatting pass modified any nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatStatus {
    /// One or more nodes were modified by the formatting pass.
    Modified,
    /// No changes were needed; all nodes remained untouched.
    Unchanged,
}

/// A 1-indexed line number as displayed in a text editor or IDE gutter (`1..=N`).
///
/// Converted at the CLI / test boundary to a 0-based [`CRow`] storage coordinate via
/// [`EditorLineNum::to_c_row`] or [`EditorLineNum::to_c_row_range`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EditorLineNum(pub usize);

/// Constructs a 1-indexed [`EditorLineNum`].
#[must_use]
pub const fn editor_line_num(line: usize) -> EditorLineNum { EditorLineNum(line) }

impl EditorLineNum {
    /// Converts this 1-indexed editor line number to a 0-based [`CRow`] index via
    /// [`LengthOps::convert_to_index`].
    #[must_use]
    pub fn to_c_row(self) -> CRow { c_height(self.0).convert_to_index() }

    /// Converts a pair of 1-indexed [`EditorLineNum`] bounds into a 0-based inclusive
    /// [`RangeInclusive<CRow>`].
    #[must_use]
    pub fn to_c_row_range(
        start: EditorLineNum,
        end: EditorLineNum,
    ) -> RangeInclusive<CRow> {
        start.to_c_row()..=end.to_c_row()
    }
}

impl fmt::Display for EditorLineNum {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use r3bl_tui::c_row;

    #[test]
    fn test_domain_enum_from_bool_conversions() {
        assert_eq!(PassPolicy::from(true), PassPolicy::Enabled);
        assert_eq!(PassPolicy::from(false), PassPolicy::Skipped);

        assert_eq!(ExecutionMode::from(false), ExecutionMode::ApplyChanges);
        assert_eq!(ExecutionMode::from(true), ExecutionMode::CheckOnly);

        assert_eq!(Verbosity::from(false), Verbosity::Quiet);
        assert_eq!(Verbosity::from(true), Verbosity::Verbose);
    }

    #[test]
    fn test_editor_line_num_conversions_and_display() {
        let start = editor_line_num(10);
        let end = editor_line_num(20);
        assert_eq!(start.to_c_row(), c_row(9));
        assert_eq!(end.to_c_row(), c_row(19));
        assert_eq!(start.to_string(), "10");

        let range = EditorLineNum::to_c_row_range(start, end);
        assert_eq!(range, c_row(9)..=c_row(19));
    }
}
