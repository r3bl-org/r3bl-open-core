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

use std::io;

use crossterm::terminal::*;
use is_terminal::IsTerminal;
use r3bl_rs_utils_core::*;

/// Get the terminal size.
pub fn get_size() -> io::Result<Size> {
    let (columns, rows) = size()?;
    Ok(Size {
        col_count: columns.into(),
        row_count: rows.into(),
    })
}

#[derive(Debug)]
pub enum StdinIsPipedResult {
    StdinIsPiped,
    StdinIsNotPiped,
}

#[derive(Debug)]
pub enum StdoutIsPipedResult {
    StdoutIsPiped,
    StdoutIsNotPiped,
}

/// If you run `echo "test" | cargo run` the following will return true.
pub fn is_stdin_piped() -> StdinIsPipedResult {
    if !std::io::stdin().is_terminal() {
        StdinIsPipedResult::StdinIsPiped
    } else {
        StdinIsPipedResult::StdinIsNotPiped
    }
}

/// If you run `cargo run | grep foo` the following will return true.
pub fn is_stdout_piped() -> StdoutIsPipedResult {
    if !std::io::stdout().is_terminal() {
        StdoutIsPipedResult::StdoutIsPiped
    } else {
        StdoutIsPipedResult::StdoutIsNotPiped
    }
}

#[derive(Debug)]
pub enum IsTTYResult {
    IsTTY,
    IsNotTTY,
}

/// If you run `cargo run` the following will return true.
pub fn is_tty() -> IsTTYResult {
    match std::io::stdin().is_terminal()
        && std::io::stdout().is_terminal()
        && std::io::stderr().is_terminal()
    {
        true => IsTTYResult::IsTTY,
        false => IsTTYResult::IsNotTTY,
    }
}
