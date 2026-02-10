// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

/// Normalizes a PTY output line by stripping ANSI escape sequences and
/// carriage returns.
///
/// ConPTY on Windows injects ANSI escape sequences (cursor movement, color
/// codes) and `\r\n` line endings into child output. This helper produces a
/// clean, platform-agnostic string for assertion comparisons.
///
/// # Steps
///
/// 1. Strip ANSI escape sequences via [`strip_ansi_escapes::strip_str`].
/// 2. Normalize `\r\n` → `\n` and remove stray `\r`.
/// 3. Trim leading/trailing whitespace.
pub fn normalize_pty_line(line: &str) -> String {
    let stripped = strip_ansi_escapes::strip_str(line);
    stripped
        .replace("\r\n", "\n")
        .replace('\r', "")
        .trim()
        .to_string()
}
