// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! User-facing strings and messages.

pub const PROCESSING_FILE: &str = "Processing";
pub const FILE_MODIFIED: &str = "Modified";
pub const FILE_UNCHANGED: &str = "Unchanged";
pub const ERROR_PREFIX: &str = "Error";
pub const FORMATTING_ENTIRE_WORKSPACE: &str = "Formatting entire workspace...";
pub const CHECK_MODE_NEEDS_FORMATTING: &str =
    "Some files need formatting. Run without --check to format them.";
pub const ALL_PROPERLY_FORMATTED: &str = "All files are properly formatted!";

/// Format an error message for a specific file.
#[must_use] 
pub fn format_error(file: &str, error: &str) -> String {
    format!("{ERROR_PREFIX} in {file}: {error}")
}

/// Format a "file modified" message.
#[must_use] 
pub fn format_modified(file: &str) -> String {
    format!("{FILE_MODIFIED}: {file}")
}

/// Format a "file unchanged" message.
#[must_use] 
pub fn format_unchanged(file: &str) -> String {
    format!("{FILE_UNCHANGED}: {file}")
}

/// Format summary message.
#[must_use] 
pub fn format_summary(total: usize, modified: usize, errors: usize) -> String {
    format!(
        "Processed {total} files, {modified} modified, {errors} errors"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_error() {
        let msg = format_error("test.rs", "parsing failed");
        assert!(msg.contains("test.rs"));
        assert!(msg.contains("parsing failed"));
    }

    #[test]
    fn test_format_modified() {
        let msg = format_modified("src/lib.rs");
        assert!(msg.contains("Modified"));
        assert!(msg.contains("src/lib.rs"));
    }
}
