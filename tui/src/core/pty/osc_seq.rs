// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! OSC (Operating System Command) sequence parsing for terminal progress indicators.
//!
//! Parses OSC 9;4 sequences used by Cargo and other build tools to communicate
//! progress information. Supports four progress states: progress updates (0-100%),
//! progress cleared, build errors, and indeterminate progress.
//!
//! The [`OscBuffer`] handles partial sequences split across buffer reads and
//! gracefully ignores malformed input.

/// Represents the different types of OSC events that can be handled.
#[derive(Debug, Clone, PartialEq)]
pub enum OscEvent {
    /// Set specific progress value 0-100% (OSC 9;4 state 1).
    ProgressUpdate(u8),
    /// Clear/remove progress indicator (OSC 9;4 state 0).
    ProgressCleared,
    /// Build error occurred (OSC 9;4 state 2).
    BuildError,
    /// Indeterminate progress - build is running but no
    /// specific progress (OSC 9;4 state 3).
    IndeterminateProgress,
    /// Hyperlink (OSC 8) with URI and display text.
    Hyperlink { uri: String, text: String },
}

/// OSC sequence constants wrapped in a dedicated module for clarity.
mod osc_codes {
    /// OSC 9;4 sequence prefix: ESC ] 9 ; 4 ;
    pub const START: &str = "\x1b]9;4;";
    /// OSC 8 hyperlink sequence prefix: ESC ] 8 ; ;
    pub const OSC8_START: &str = "\x1b]8;;";
    /// Sequence terminator: ESC \\ (String Terminator)
    pub const END: &str = "\x1b\\";
    /// Parameter delimiter within OSC sequences
    pub const DELIMITER: char = ';';
}

/// Buffer for accumulating and parsing OSC (Operating System Command) sequences.
///
/// This is not the raw PTY read buffer, but a dedicated buffer that accumulates OSC
/// sequences as they are read from the PTY output. It handles partial sequences that may
/// be split across multiple read operations.
#[derive(Debug)]
pub struct OscBuffer {
    data: String,
}

impl Default for OscBuffer {
    fn default() -> Self { Self::new() }
}

/// Helper functions for creating OSC 8 hyperlink sequences.
pub mod hyperlink {
    use std::path::Path;

    use super::osc_codes;

    /// Creates an OSC 8 hyperlink sequence.
    ///
    /// # Arguments
    /// * `uri` - The URI/URL to link to (e.g., "<https://example.com>", "<file:///path/to/file>")
    /// * `text` - The display text for the hyperlink
    ///
    /// # Returns
    /// A string containing the complete OSC 8 hyperlink sequence
    ///
    /// # Example
    /// ```
    /// use r3bl_tui::core::pty::osc_seq::hyperlink::format_hyperlink;
    /// let link = format_hyperlink("https://example.com", "Example");
    /// assert_eq!(link, "\u{1b}]8;;https://example.com\u{1b}\\Example\u{1b}]8;;\u{1b}\\");
    /// ```
    #[must_use]
    pub fn format_hyperlink(uri: &str, text: &str) -> String {
        format!(
            "{}{}{}{}{}{}{}",
            osc_codes::OSC8_START,
            uri,
            osc_codes::END,
            text,
            osc_codes::OSC8_START,
            "", // Empty URI to close the hyperlink
            osc_codes::END
        )
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
    /// use r3bl_tui::core::pty::osc_seq::hyperlink::format_file_hyperlink;
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
            // For relative paths, convert to absolute first
            match std::env::current_dir().map(|cwd| cwd.join(path)) {
                Ok(abs_path) => format!("file://{}", abs_path.display()),
                Err(_) => format!("file://{}", path.display()), // Fallback
            }
        };

        // URL encode special characters in the URI
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
}

impl OscBuffer {
    /// Creates a new empty OSC buffer.
    #[must_use]
    pub fn new() -> Self {
        Self {
            data: String::new(),
        }
    }

    /// Appends new bytes to the buffer and extracts any complete OSC sequences.
    ///
    /// # Arguments
    /// * `buffer` - Raw bytes read from the PTY
    /// * `n` - Number of valid bytes in the buffer
    ///
    /// # Returns
    /// A vector of parsed [`OscEvent`]s from any complete sequences found.
    pub fn append_and_extract(&mut self, buffer: &[u8], n: usize) -> Vec<OscEvent> {
        // Convert bytes to string and append to accumulated data.
        let text = String::from_utf8_lossy(&buffer[..n]);
        self.data.push_str(&text);

        let mut events = Vec::new();

        // Find and process all complete OSC sequences.
        while let Some(event) = self.extract_next_sequence() {
            events.push(event);
        }

        events
    }

    /// Extracts and parses the next complete OSC sequence from the buffer.
    ///
    /// Looks for sequences in the format: `ESC]9;4;{state};{progress}ESC\`
    ///
    /// # Returns
    /// * `Some(OscEvent)` if a complete sequence was found and parsed.
    /// * `None` if no complete sequence is available.
    pub fn extract_next_sequence(&mut self) -> Option<OscEvent> {
        // OSC sequence format "osc::START {state};{progress} osc::END"
        // Find start of OSC sequence.
        let start_idx = self.data.find(osc_codes::START)?;
        let after_start_idx = start_idx + osc_codes::START.len();

        // Find end of sequence.
        let end_idx = self.data[after_start_idx..].find(osc_codes::END)?;
        let params_end_idx = after_start_idx + end_idx;
        let sequence_end_idx = params_end_idx + osc_codes::END.len();

        // Extract parameters.
        let params = &self.data[after_start_idx..params_end_idx];

        // Parse the sequence.
        let event = self.parse_osc_params(params);

        // Remove processed portion from buffer (including everything up to sequence end).
        self.data.drain(0..sequence_end_idx);

        event
    }

    /// Parses OSC parameters into an `OscEvent`.
    ///
    /// # Arguments
    /// * `params` - The parameter string in format "{state};{progress}"
    ///
    /// # Returns
    /// * `Some(OscEvent)` if parameters were valid.
    /// * `None` if parameters were malformed or state was unknown.
    #[must_use]
    pub fn parse_osc_params(&self, params: &str) -> Option<OscEvent> {
        let parts: Vec<&str> = params.split(osc_codes::DELIMITER).collect();
        if parts.len() != 2 {
            // Gracefully handle malformed sequences.
            return None;
        }

        let state = parts[0].parse::<u8>().ok()?;
        let progress = parts[1].parse::<f64>().ok()?;

        match state {
            0 => Some(OscEvent::ProgressCleared),
            1 => {
                // Clamp progress to valid u8 range (0-100).
                let clamped = progress.clamp(0.0, 100.0);
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let percentage = clamped as u8;
                Some(OscEvent::ProgressUpdate(percentage))
            }
            2 => Some(OscEvent::BuildError),
            3 => Some(OscEvent::IndeterminateProgress),
            _ => None, // Gracefully ignore unknown states
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_complete_sequence() {
        let mut buffer = OscBuffer::new();

        // Test progress update (state 1)
        let input = b"\x1b]9;4;1;50\x1b\\";
        let events = buffer.append_and_extract(input, input.len());
        assert_eq!(events, vec![OscEvent::ProgressUpdate(50)]);

        // Test progress cleared (state 0)
        let input = b"\x1b]9;4;0;0\x1b\\";
        let events = buffer.append_and_extract(input, input.len());
        assert_eq!(events, vec![OscEvent::ProgressCleared]);

        // Test build error (state 2)
        let input = b"\x1b]9;4;2;0\x1b\\";
        let events = buffer.append_and_extract(input, input.len());
        assert_eq!(events, vec![OscEvent::BuildError]);

        // Test indeterminate progress (state 3)
        let input = b"\x1b]9;4;3;0\x1b\\";
        let events = buffer.append_and_extract(input, input.len());
        assert_eq!(events, vec![OscEvent::IndeterminateProgress]);
    }

    #[test]
    fn test_multiple_sequences() {
        let mut buffer = OscBuffer::new();

        // Multiple sequences in one buffer
        let input = b"\x1b]9;4;1;25\x1b\\\x1b]9;4;1;50\x1b\\\x1b]9;4;0;0\x1b\\";
        let events = buffer.append_and_extract(input, input.len());
        assert_eq!(
            events,
            vec![
                OscEvent::ProgressUpdate(25),
                OscEvent::ProgressUpdate(50),
                OscEvent::ProgressCleared
            ]
        );
    }

    #[test]
    fn test_sequences_with_text_between() {
        let mut buffer = OscBuffer::new();

        // OSC sequences with regular text interleaved
        let input =
            b"Building...\x1b]9;4;1;30\x1b\\Compiling crate...\x1b]9;4;1;60\x1b\\Done!";
        let events = buffer.append_and_extract(input, input.len());
        assert_eq!(
            events,
            vec![OscEvent::ProgressUpdate(30), OscEvent::ProgressUpdate(60)]
        );

        // Verify remaining text is preserved in buffer
        assert!(buffer.data.contains("Done!"));
    }

    #[test]
    fn test_split_sequence_across_buffers() {
        let mut buffer = OscBuffer::new();

        // First part of sequence
        let input1 = b"\x1b]9;4;1;";
        let events1 = buffer.append_and_extract(input1, input1.len());
        assert_eq!(events1, vec![]); // No complete sequence yet

        // Second part of sequence
        let input2 = b"75\x1b\\";
        let events2 = buffer.append_and_extract(input2, input2.len());
        assert_eq!(events2, vec![OscEvent::ProgressUpdate(75)]);
    }

    #[test]
    fn test_complex_split_scenarios() {
        let mut buffer = OscBuffer::new();

        // Split at different points
        let parts: [&[u8]; 4] = [b"\x1b]9", b";4;1;", b"42", b"\x1b\\"];

        // Feed parts one by one
        assert_eq!(buffer.append_and_extract(parts[0], parts[0].len()), vec![]);
        assert_eq!(buffer.append_and_extract(parts[1], parts[1].len()), vec![]);
        assert_eq!(buffer.append_and_extract(parts[2], parts[2].len()), vec![]);
        assert_eq!(
            buffer.append_and_extract(parts[3], parts[3].len()),
            vec![OscEvent::ProgressUpdate(42)]
        );
    }

    #[test]
    fn test_invalid_sequences() {
        let mut buffer = OscBuffer::new();

        // Missing progress value
        let input = b"\x1b]9;4;1\x1b\\";
        let events = buffer.append_and_extract(input, input.len());
        assert_eq!(events, vec![]); // Should gracefully ignore

        // Non-numeric progress value
        let input = b"\x1b]9;4;1;abc\x1b\\";
        let events = buffer.append_and_extract(input, input.len());
        assert_eq!(events, vec![]); // Should gracefully ignore

        // Unknown state value
        let input = b"\x1b]9;4;99;50\x1b\\";
        let events = buffer.append_and_extract(input, input.len());
        assert_eq!(events, vec![]); // Should gracefully ignore
    }

    #[test]
    fn test_malformed_terminators() {
        let mut buffer = OscBuffer::new();

        // Missing terminator - sequence should remain in buffer
        let input = b"\x1b]9;4;1;50";
        let events = buffer.append_and_extract(input, input.len());
        assert_eq!(events, vec![]);
        assert!(buffer.data.contains("9;4;1;50")); // Data should still be in buffer

        // Now add terminator
        let input2 = b"\x1b\\";
        let events2 = buffer.append_and_extract(input2, input2.len());
        assert_eq!(events2, vec![OscEvent::ProgressUpdate(50)]);
    }

    #[test]
    fn test_out_of_range_values() {
        let mut buffer = OscBuffer::new();

        // Progress > 100 should be clamped to 100
        let input = b"\x1b]9;4;1;150\x1b\\";
        let events = buffer.append_and_extract(input, input.len());
        assert_eq!(events, vec![OscEvent::ProgressUpdate(100)]);

        // Negative progress should be clamped to 0
        let input = b"\x1b]9;4;1;-50\x1b\\";
        let events = buffer.append_and_extract(input, input.len());
        assert_eq!(events, vec![OscEvent::ProgressUpdate(0)]);
    }

    #[test]
    fn test_interleaved_incomplete_sequences() {
        let mut buffer = OscBuffer::new();

        // Nested/interleaved starts (second start before first completes)
        // This creates an invalid sequence since the first one is missing its terminator
        let input = b"\x1b]9;4;1;25\x1b]9;4;1;50\x1b\\";
        let events = buffer.append_and_extract(input, input.len());
        // The parser should gracefully handle this malformed input
        // Since the first sequence is incomplete, nothing should be parsed
        assert_eq!(events, vec![]);
    }

    #[test]
    fn test_buffer_with_unicode() {
        let mut buffer = OscBuffer::new();

        // OSC sequences with Unicode text around them
        let input = "🚀 Building...\x1b]9;4;1;50\x1b\\✨ Done!".as_bytes();
        let events = buffer.append_and_extract(input, input.len());
        assert_eq!(events, vec![OscEvent::ProgressUpdate(50)]);
        assert!(buffer.data.contains("✨ Done!"));
    }

    #[test]
    fn test_rapid_sequence_updates() {
        let mut buffer = OscBuffer::new();

        // Simulate rapid progress updates
        let mut all_events = Vec::new();
        for i in (0..=100).step_by(10) {
            let input = format!("\x1b]9;4;1;{i}\x1b\\");
            let events = buffer.append_and_extract(input.as_bytes(), input.len());
            all_events.extend(events);
        }

        assert_eq!(all_events.len(), 11); // 0, 10, 20, ..., 100
        assert_eq!(all_events[0], OscEvent::ProgressUpdate(0));
        assert_eq!(all_events[10], OscEvent::ProgressUpdate(100));
    }

    #[test]
    fn test_empty_buffer_operations() {
        let mut buffer = OscBuffer::new();

        // Empty input
        let events = buffer.append_and_extract(b"", 0);
        assert_eq!(events, vec![]);

        // Just regular text, no OSC
        let input = b"Just regular text";
        let events = buffer.append_and_extract(input, input.len());
        assert_eq!(events, vec![]);
        assert!(buffer.data.contains("Just regular text"));
    }

    #[test]
    fn test_partial_sequence_with_corruption() {
        let mut buffer = OscBuffer::new();

        // Add partial sequence
        let partial = b"\x1b]9;4;1;33";
        buffer.append_and_extract(partial, partial.len());
        assert!(buffer.data.contains("\x1b]9;4;1;33"));

        // Add unrelated text - this will corrupt the sequence
        let text = b"some text";
        buffer.append_and_extract(text, text.len());

        // The buffer now contains: "\x1b]9;4;1;33some text"
        // This is not a valid OSC sequence due to the text in between

        // Complete the sequence - but it's now invalid due to the text in between
        let terminator = b"\x1b\\";
        let events = buffer.append_and_extract(terminator, terminator.len());

        // The parser finds "\x1b]9;4;" but "1;33some text" is not valid params
        // So it gracefully ignores the malformed sequence and extracts it
        assert_eq!(events, vec![]);

        // After extraction attempt, buffer should be empty since the malformed
        // sequence was removed
        assert_eq!(buffer.data, "");
    }

    #[test]
    fn test_partial_sequence_clean() {
        let mut buffer = OscBuffer::new();

        // Add partial sequence without corruption
        let partial = b"\x1b]9;4;1;33";
        buffer.append_and_extract(partial, partial.len());

        // Complete the sequence properly
        let terminator = b"\x1b\\";
        let events = buffer.append_and_extract(terminator, terminator.len());

        // Should parse correctly
        assert_eq!(events, vec![OscEvent::ProgressUpdate(33)]);

        // Buffer should be empty after successful extraction
        assert_eq!(buffer.data, "");
    }

    #[test]
    fn test_decimal_progress_values() {
        let mut buffer = OscBuffer::new();

        // Test decimal values get truncated to integers
        let input = b"\x1b]9;4;1;33.7\x1b\\";
        let events = buffer.append_and_extract(input, input.len());
        assert_eq!(events, vec![OscEvent::ProgressUpdate(33)]);

        let input = b"\x1b]9;4;1;99.9\x1b\\";
        let events = buffer.append_and_extract(input, input.len());
        assert_eq!(events, vec![OscEvent::ProgressUpdate(99)]);
    }

    mod hyperlink_tests {
        use std::path::Path;

        use super::hyperlink::{format_file_hyperlink, format_hyperlink};

        #[test]
        fn test_format_hyperlink_basic() {
            let result = format_hyperlink("https://example.com", "Example Link");
            let expected = "\x1b]8;;https://example.com\x1b\\Example Link\x1b]8;;\x1b\\";
            assert_eq!(result, expected);
        }

        #[test]
        fn test_format_hyperlink_empty() {
            let result = format_hyperlink("", "Empty URI");
            let expected = "\x1b]8;;\x1b\\Empty URI\x1b]8;;\x1b\\";
            assert_eq!(result, expected);
        }

        #[test]
        fn test_format_hyperlink_special_chars() {
            let result =
                format_hyperlink("https://example.com/path?q=test&v=1", "Complex URL");
            let expected = "\x1b]8;;https://example.com/path?q=test&v=1\x1b\\Complex URL\x1b]8;;\x1b\\";
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
            assert!(result.ends_with("\x1b]8;;\x1b\\"));
        }

        #[test]
        fn test_format_file_hyperlink_with_spaces() {
            let path = Path::new("/home/user/my document.txt");
            let result = format_file_hyperlink(path);

            // URI should have URL-encoded spaces
            assert!(result.contains("file:///home/user/my%20document.txt"));
            // Display text should keep original spaces
            assert!(result.contains("/home/user/my document.txt"));
        }

        #[test]
        fn test_format_file_hyperlink_with_special_chars() {
            let path = Path::new("/home/user/file#with&special%chars?.txt");
            let result = format_file_hyperlink(path);

            // URI should have URL-encoded special characters
            assert!(
                result.contains("file:///home/user/file%23with%26special%25chars%3F.txt")
            );
            // Display text should keep original characters
            assert!(result.contains("/home/user/file#with&special%chars?.txt"));
        }

        #[test]
        fn test_format_file_hyperlink_relative_path() {
            let path = Path::new("./relative/path.txt");
            let result = format_file_hyperlink(path);

            // Should contain file:// URI (will be converted to absolute)
            assert!(result.starts_with("\x1b]8;;file://"));
            // Display text should show the original relative path
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
                let path_str = format!("/home/user/file{}test.txt", input_char);
                let path = Path::new(&path_str);
                let result = format_file_hyperlink(path);

                assert!(
                    result.contains(expected_encoding),
                    "Failed to encode '{}' as '{}' in result: {}",
                    input_char,
                    expected_encoding,
                    result
                );
            }
        }
    }
}
