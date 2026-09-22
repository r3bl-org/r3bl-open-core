// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// rustdoc-fmt: skip

//! User-facing strings and messages.

pub const FILE_MODIFIED: &str = "Modified";
pub const CHECK_MODE_NEEDS_FORMATTING: &str =
    "Some files need formatting. Run without --check to format them.";

/// Formats a "file modified" message.
#[must_use]
pub fn format_modified(file: &str) -> String { format!("{FILE_MODIFIED}: {file}") }

/// Formats summary message.
#[must_use]
pub fn format_summary(total: usize, modified: usize, errors: usize) -> String {
    format!("Processed {total} files, {modified} modified, {errors} errors")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_modified() {
        let msg = format_modified("src/lib.rs");
        assert!(msg.contains("Modified"));
        assert!(msg.contains("src/lib.rs"));
    }
}
