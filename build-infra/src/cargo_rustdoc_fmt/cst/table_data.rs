// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// rustdoc-fmt: skip

//! Markdown table domain types ([`TableData`], [`ColumnAlignment`], [`TableStartStatus`],
//! [`SeparatorRowStatus`]) and table row parsing/formatting helpers.

use super::extract_comment_content;
use crate::cargo_rustdoc_fmt::constants::{MIN_TABLE_COL_WIDTH, SINGLE_SPACE_STR,
                                          TABLE_ALIGN_COLON, TABLE_CELL_DELIMITER,
                                          TABLE_CELL_JOINER, TABLE_ROW_PREFIX,
                                          TABLE_ROW_SUFFIX, TABLE_SEPARATOR_DASH,
                                          TABLE_SEPARATOR_DASH_STR};
use r3bl_tui::{ArrayBoundsCheck, ArrayOverflowResult, CCol, CWidth, c_col, c_width};
use unicode_width::UnicodeWidthStr;

/// Column alignment in a Markdown table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnAlignment {
    Left,
    Right,
    Center,
    None,
}

/// Parsed Markdown table structure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableData {
    pub headers: Vec<String>,
    pub alignments: Vec<ColumnAlignment>,
    pub separator_widths: Vec<CWidth>,
    pub rows: Vec<Vec<String>>,
}

/// Whether a line inside a doc comment block starts a valid `GFM` Markdown table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableStartStatus {
    /// The line and its immediate successor form a valid table header and separator row.
    StartsTable,
    /// The line does not start a valid table.
    NotTable,
}

/// Whether a table row is a valid `GFM` separator row (e.g., `| :--- | ---: |`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeparatorRowStatus {
    /// Valid `GFM` separator row containing dashes and optional alignment colons.
    Valid,
    /// Not a valid separator row.
    Invalid,
}

/// Checks whether lines starting at index `start_idx` form a valid Markdown table.
#[must_use]
pub fn is_table_start(
    raw_lines: &[String],
    start_idx: usize,
    marker: &str,
) -> TableStartStatus {
    // A valid GFM table requires at least two lines: a header row at `start_idx` and a
    // separator row immediately following at `start_idx + 1`. If `start_idx` is the last
    // line of the doc block, a table cannot start here.
    if start_idx + 1 >= raw_lines.len() {
        return TableStartStatus::NotTable;
    }

    let header_line = extract_comment_content(&raw_lines[start_idx], marker).trim();
    let separator_line =
        extract_comment_content(&raw_lines[start_idx + 1], marker).trim();

    if !header_line.starts_with(TABLE_CELL_DELIMITER)
        || !header_line.ends_with(TABLE_CELL_DELIMITER)
    {
        return TableStartStatus::NotTable;
    }

    if !separator_line.starts_with(TABLE_CELL_DELIMITER)
        || !separator_line.ends_with(TABLE_CELL_DELIMITER)
    {
        return TableStartStatus::NotTable;
    }

    let is_valid_sep = is_separator_row(separator_line) == SeparatorRowStatus::Valid;
    let matching_columns =
        parse_table_row(header_line).len() == parse_table_row(separator_line).len();

    if is_valid_sep && matching_columns {
        TableStartStatus::StartsTable
    } else {
        TableStartStatus::NotTable
    }
}

/// Checks if a row is a valid `GFM` separator row: cells contain only `-`, `:`, and
/// whitespace with at least one `-`.
#[must_use]
pub fn is_separator_row(line: &str) -> SeparatorRowStatus {
    let cells = parse_table_row(line);
    if cells.is_empty() {
        return SeparatorRowStatus::Invalid;
    }

    for cell in &cells {
        let trimmed = cell.trim();
        if trimmed.is_empty() {
            return SeparatorRowStatus::Invalid;
        }
        let has_dash = trimmed.contains(TABLE_SEPARATOR_DASH);
        let valid_chars = trimmed.chars().all(|ch| {
            ch == TABLE_SEPARATOR_DASH || ch == TABLE_ALIGN_COLON || ch.is_whitespace()
        });
        if !has_dash || !valid_chars {
            return SeparatorRowStatus::Invalid;
        }
    }

    SeparatorRowStatus::Valid
}

/// Parses a pipe-delimited table row into cells.
#[must_use]
pub fn parse_table_row(line: &str) -> Vec<String> {
    let trimmed = line.trim();
    let inner = if let Some(stripped) = trimmed.strip_prefix(TABLE_CELL_DELIMITER) {
        stripped
            .strip_suffix(TABLE_CELL_DELIMITER)
            .unwrap_or(stripped)
    } else {
        trimmed
    };

    inner
        .split(TABLE_CELL_DELIMITER)
        .map(|cell| cell.trim().to_string())
        .collect()
}

/// Parses raw table lines into [`TableData`].
#[must_use]
pub fn parse_table_lines(lines: &[String]) -> TableData {
    if lines.len() < 2 {
        return TableData {
            headers: Vec::new(),
            alignments: Vec::new(),
            separator_widths: Vec::new(),
            rows: Vec::new(),
        };
    }

    let headers = parse_table_row(&lines[0]);
    let sep_cells = parse_table_row(&lines[1]);
    let separator_widths: Vec<CWidth> = sep_cells
        .iter()
        .map(|cell| c_width(UnicodeWidthStr::width(cell.trim())))
        .collect();
    let alignments: Vec<ColumnAlignment> = sep_cells
        .iter()
        .map(|cell| {
            let trimmed = cell.trim();
            let starts = trimmed.starts_with(TABLE_ALIGN_COLON);
            let ends = trimmed.ends_with(TABLE_ALIGN_COLON);
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
        separator_widths,
        rows,
    }
}

/// Formats [`TableData`] into aligned Markdown table rows.
#[must_use]
pub fn format_table_data(table: &TableData) -> Vec<String> {
    if table.headers.is_empty() {
        return Vec::new();
    }

    let col_count = c_width(table.headers.len());
    let min_col_width = c_width(MIN_TABLE_COL_WIDTH);
    let mut widths: Vec<CWidth> = (0..col_count.as_usize())
        .map(|col_offset| {
            let col_idx: CCol = c_col(col_offset);
            table
                .separator_widths
                .get(col_idx.as_usize())
                .copied()
                .unwrap_or(min_col_width)
                .max(min_col_width)
        })
        .collect();

    // Calculate maximum width for each column based on headers and rows.
    for (col_offset, header_text) in table.headers.iter().enumerate() {
        let col_idx = c_col(col_offset);
        if col_idx.overflows(col_count) == ArrayOverflowResult::Within {
            widths[col_idx.as_usize()] = widths[col_idx.as_usize()]
                .max(c_width(UnicodeWidthStr::width(header_text.as_str())));
        }
    }

    for row in &table.rows {
        for (col_offset, cell) in row.iter().enumerate() {
            let col_idx = c_col(col_offset);
            if col_idx.overflows(col_count) == ArrayOverflowResult::Within {
                widths[col_idx.as_usize()] = widths[col_idx.as_usize()]
                    .max(c_width(UnicodeWidthStr::width(cell.as_str())));
            }
        }
    }

    let mut result = Vec::new();

    // 1. Header row.
    let header_cells: Vec<String> = table
        .headers
        .iter()
        .enumerate()
        .map(|(col_offset, header_text)| {
            let col_idx = c_col(col_offset);
            let width = widths[col_idx.as_usize()];
            let cell_width = c_width(UnicodeWidthStr::width(header_text.as_str()));
            let padding = (width - cell_width).as_usize();
            format!("{header_text}{}", SINGLE_SPACE_STR.repeat(padding))
        })
        .collect();
    result.push(format!(
        "{TABLE_ROW_PREFIX}{}{TABLE_ROW_SUFFIX}",
        header_cells.join(TABLE_CELL_JOINER)
    ));

    // 2. Separator row.
    let sep_cells: Vec<String> = widths
        .iter()
        .enumerate()
        .map(|(col_offset, &col_width)| {
            let col_idx = c_col(col_offset);
            let align = table
                .alignments
                .get(col_idx.as_usize())
                .copied()
                .unwrap_or(ColumnAlignment::None);
            format_separator_cell(col_width, align, min_col_width)
        })
        .collect();
    result.push(format!(
        "{TABLE_ROW_PREFIX}{}{TABLE_ROW_SUFFIX}",
        sep_cells.join(TABLE_CELL_JOINER)
    ));

    // 3. Data rows.
    for row in &table.rows {
        let row_cells: Vec<String> = (0..col_count.as_usize())
            .map(|col_offset| {
                let col_idx = c_col(col_offset);
                let cell = row
                    .get(col_idx.as_usize())
                    .map_or("", |cell_str| cell_str.as_str());
                let width = widths[col_idx.as_usize()];
                let cell_width = c_width(UnicodeWidthStr::width(cell));
                let padding = (width - cell_width).as_usize();
                let align = table
                    .alignments
                    .get(col_idx.as_usize())
                    .copied()
                    .unwrap_or(ColumnAlignment::None);
                match align {
                    ColumnAlignment::Right => {
                        format!("{}{cell}", SINGLE_SPACE_STR.repeat(padding))
                    }
                    _ => format!("{cell}{}", SINGLE_SPACE_STR.repeat(padding)),
                }
            })
            .collect();
        result.push(format!(
            "{TABLE_ROW_PREFIX}{}{TABLE_ROW_SUFFIX}",
            row_cells.join(TABLE_CELL_JOINER)
        ));
    }

    result
}

/// Formats a single `GFM` separator cell (`---`, `:---`, `---:`, or `:---:`) for a column
/// of `col_width`.
fn format_separator_cell(
    col_width: CWidth,
    align: ColumnAlignment,
    min_col_width: CWidth,
) -> String {
    match align {
        ColumnAlignment::Center => {
            let dashes = TABLE_SEPARATOR_DASH_STR
                .repeat((col_width - c_width(2)).max(c_width(1)).as_usize());
            format!("{TABLE_ALIGN_COLON}{dashes}{TABLE_ALIGN_COLON}")
        }
        ColumnAlignment::Left => {
            let dashes = TABLE_SEPARATOR_DASH_STR
                .repeat((col_width - c_width(1)).max(c_width(2)).as_usize());
            format!("{TABLE_ALIGN_COLON}{dashes}")
        }
        ColumnAlignment::Right => {
            let dashes = TABLE_SEPARATOR_DASH_STR
                .repeat((col_width - c_width(1)).max(c_width(2)).as_usize());
            format!("{dashes}{TABLE_ALIGN_COLON}")
        }
        ColumnAlignment::None => {
            TABLE_SEPARATOR_DASH_STR.repeat(col_width.max(min_col_width).as_usize())
        }
    }
}
