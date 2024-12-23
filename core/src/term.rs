/*
 *   Copyright (c) 2023 R3BL LLC
 *   All rights reserved.
 *
 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
 */

use crossterm::terminal::size;
use miette::IntoDiagnostic;

use crate::{ChUnit, ch, size::Size};

pub const DEFAULT_WIDTH: u16 = 80;

/// Get the terminal width. If there is a problem, return the default width.
pub fn get_terminal_width() -> ChUnit {
    match get_size() {
        Ok(size) => size.col_count,
        Err(_) => ch(DEFAULT_WIDTH),
    }
}

/// Get the terminal size.
pub fn get_size() -> miette::Result<Size> {
    let (columns, rows) = size().into_diagnostic()?;
    Ok(Size {
        col_count: columns.into(),
        row_count: rows.into(),
    })
}
