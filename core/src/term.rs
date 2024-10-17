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

use std::io::{self};

use crossterm::terminal::size;

use crate::{ch, size::Size};

pub const DEFAULT_WIDTH: usize = 80;

/// Get the terminal width. If there is a problem, return the default width.
pub fn get_terminal_width() -> usize {
    match get_size() {
        Ok(size) => ch!(@to_usize size.col_count),
        Err(_) => DEFAULT_WIDTH,
    }
}

/// Get the terminal size.
pub fn get_size() -> io::Result<Size> {
    let (columns, rows) = size()?;
    Ok(Size {
        col_count: columns.into(),
        row_count: rows.into(),
    })
}
