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

use std::io::{self, IsTerminal as _};

use crossterm::terminal::size;
use r3bl_core::{ch, size::Size};

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
/// More info: <https://unix.stackexchange.com/questions/597083/how-does-piping-affect-stdin>
pub fn is_stdin_piped() -> StdinIsPipedResult {
    if !std::io::stdin().is_terminal() {
        StdinIsPipedResult::StdinIsPiped
    } else {
        StdinIsPipedResult::StdinIsNotPiped
    }
}

/// If you run `cargo run | grep foo` the following will return true.
/// More info: <https://unix.stackexchange.com/questions/597083/how-does-piping-affect-stdin>
pub fn is_stdout_piped() -> StdoutIsPipedResult {
    use std::io::IsTerminal as _;
    if !std::io::stdout().is_terminal() {
        StdoutIsPipedResult::StdoutIsPiped
    } else {
        StdoutIsPipedResult::StdoutIsNotPiped
    }
}

#[derive(Debug)]
pub enum TTYResult {
    IsInteractive,
    IsNotInteractive,
}

/// Returns [TTYResult::IsInteractive] if stdin, stdout, and stderr are *all* fully
/// interactive.
///
/// There are situations where some can be interactive and others not, such as when piping
/// is active.
pub fn is_fully_interactive_terminal() -> TTYResult {
    use crossterm::tty::IsTty;
    let is_tty: bool = std::io::stdin().is_tty();
    match is_tty {
        true => TTYResult::IsInteractive,
        false => TTYResult::IsNotInteractive,
    }
}

/// Returns [TTYResult::IsNotInteractive] if stdin, stdout, and stderr are *all* fully
/// uninteractive. This happens when `cargo test` runs.
///
/// There are situations where some can be interactive and others not, such as when piping
/// is active.
pub fn is_fully_uninteractive_terminal() -> TTYResult {
    use crossterm::tty::IsTty;
    let stdin_is_tty: bool = std::io::stdin().is_tty();
    let stdout_is_tty: bool = std::io::stdout().is_tty();
    let stderr_is_tty: bool = std::io::stderr().is_tty();
    match !stdin_is_tty && !stdout_is_tty && !stderr_is_tty {
        true => TTYResult::IsNotInteractive,
        false => TTYResult::IsInteractive,
    }
}
