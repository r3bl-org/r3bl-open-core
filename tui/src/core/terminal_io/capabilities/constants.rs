// Copyright (c) 2023-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

/// If there is a problem access the terminal width this is the default width to use.
pub const DEFAULT_WIDTH: u16 = 80;

/// Error message displayed when the terminal's [`stdin`] is not interactive.
///
/// [`stdin`]: std::io::stdin
pub const ERR_MSG_STDIN_NOT_INTERACTIVE: &str =
    "The terminal is not interactive (stdin).";

/// Error message displayed when the terminal's [`stdout`] is not interactive.
///
/// [`stdout`]: std::io::stdout
pub const ERR_MSG_STDOUT_NOT_INTERACTIVE: &str =
    "The terminal is not interactive (stdout).";

/// Error message displayed when both [`stdin`] and [`stdout`] are not interactive.
///
/// [`stdin`]: std::io::stdin
/// [`stdout`]: std::io::stdout
pub const ERR_MSG_BOTH_NOT_INTERACTIVE: &str =
    "The terminal is not interactive (both stdin and stdout).";
