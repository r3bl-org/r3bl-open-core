// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Helper functions for creating OSC 8 hyperlink sequences.

use std::path::Path;

use super::osc_codes::OscSequence;

/// Creates an OSC 8 hyperlink sequence.
///
/// # Arguments
/// * `uri` - The URI/URL to link to (e.g., `<https://example.com>`, `<file:///path/to/file>`)
/// * `text` - The display text for the hyperlink
///
/// # Returns
/// A string containing the complete OSC 8 hyperlink sequence
///
/// # Example
/// ```
/// use r3bl_tui::core::osc::osc_hyperlink::format_hyperlink;
/// let link = format_hyperlink("https://example.com", "Example");
/// assert_eq!(link, "\u{1b}]8;;https://example.com\u{7}Example\u{1b}]8;;\u{7}");
/// ```
#[must_use]
pub fn format_hyperlink(uri: &str, text: &str) -> String {
    let start = OscSequence::HyperlinkStart {
        uri: uri.to_string(),
        id: None,
    };
    let end = OscSequence::HyperlinkEnd;
    format!("{start}{text}{end}")
}

/// Creates an OSC 8 hyperlink for a file path.
///
/// This function converts a file path to a proper file:// URI and creates
/// a clickable hyperlink that will open the file in the default application
/// when clicked in a terminal that supports OSC 8.
///
/// # Arguments
/// * `path` - The file path to create a hyperlink for
///
/// # Returns
/// A string containing the OSC 8 hyperlink sequence for the file
///
/// # Example
/// ```
/// use r3bl_tui::core::osc::osc_hyperlink::format_file_hyperlink;
/// use std::path::Path;
/// let path = Path::new("/home/user/document.txt");
/// let link = format_file_hyperlink(path);
/// // Result will be a clickable link showing the path
/// ```
#[must_use]
pub fn format_file_hyperlink(path: &Path) -> String {
    let display_text = path.display().to_string();

    // Convert path to file:// URI
    let uri = if path.is_absolute() {
        format!("file://{}", path.display())
    } else {
        // For relative paths, convert to absolute first.
        match std::env::current_dir().map(|cwd| cwd.join(path)) {
            Ok(abs_path) => format!("file://{}", abs_path.display()),
            Err(_) => format!("file://{}", path.display()), // Fallback
        }
    };

    // URL encode special characters in the URI.
    let encoded_uri = uri
        .chars()
        .map(|c| match c {
            ' ' => "%20".to_string(),
            '#' => "%23".to_string(),
            '?' => "%3F".to_string(),
            '&' => "%26".to_string(),
            '%' => "%25".to_string(),
            _ => c.to_string(),
        })
        .collect::<String>();

    format_hyperlink(&encoded_uri, &display_text)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{format_file_hyperlink, format_hyperlink};

    #[test]
    fn test_format_hyperlink_basic() {
        let result = format_hyperlink("https://example.com", "Example Link");
        let expected = "\x1b]8;;https://example.com\x07Example Link\x1b]8;;\x07";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_format_hyperlink_empty() {
        let result = format_hyperlink("", "Empty URI");
        let expected = "\x1b]8;;\x07Empty URI\x1b]8;;\x07";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_format_hyperlink_special_chars() {
        let result =
            format_hyperlink("https://example.com/path?q=test&v=1", "Complex URL");
        let expected =
            "\x1b]8;;https://example.com/path?q=test&v=1\x07Complex URL\x1b]8;;\x07";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_format_file_hyperlink_absolute_path() {
        let path = Path::new("/home/user/document.txt");
        let result = format_file_hyperlink(path);

        // Should contain file:// URI and display text
        assert!(result.contains("file:///home/user/document.txt"));
        assert!(result.contains("/home/user/document.txt"));
        assert!(result.starts_with("\x1b]8;;"));
        assert!(result.ends_with("\x1b]8;;\x07"));
    }

    #[test]
    fn test_format_file_hyperlink_with_spaces() {
        let path = Path::new("/home/user/my document.txt");
        let result = format_file_hyperlink(path);

        // URI should have URL-encoded spaces.
        assert!(result.contains("file:///home/user/my%20document.txt"));
        // Display text should keep original spaces.
        assert!(result.contains("/home/user/my document.txt"));
    }

    #[test]
    fn test_format_file_hyperlink_with_special_chars() {
        let path = Path::new("/home/user/file#with&special%chars?.txt");
        let result = format_file_hyperlink(path);

        // URI should have URL-encoded special characters.
        assert!(
            result.contains("file:///home/user/file%23with%26special%25chars%3F.txt")
        );
        // Display text should keep original characters.
        assert!(result.contains("/home/user/file#with&special%chars?.txt"));
    }

    #[test]
    fn test_format_file_hyperlink_relative_path() {
        let path = Path::new("./relative/path.txt");
        let result = format_file_hyperlink(path);

        // Should contain file:// URI (will be converted to absolute)
        assert!(result.starts_with("\x1b]8;;file://"));
        // Display text should show the original relative path.
        assert!(result.contains("./relative/path.txt"));
    }

    #[test]
    fn test_url_encoding_coverage() {
        let test_cases = [
            (" ", "%20"),
            ("#", "%23"),
            ("?", "%3F"),
            ("&", "%26"),
            ("%", "%25"),
        ];

        for (input_char, expected_encoding) in test_cases {
            let path_str = format!("/home/user/file{input_char}test.txt");
            let path = Path::new(&path_str);
            let result = format_file_hyperlink(path);

            assert!(
                result.contains(expected_encoding),
                "Failed to encode '{input_char}' as '{expected_encoding}' in result: {result}"
            );
        }
    }
}
