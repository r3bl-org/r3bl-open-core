// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::osc_pty_event::OscPtyEvent;
use crate::{LossyConvertToByte, Pc,
            core::ansi::constants::{OSC_DELIMITER, OSC_PROGRESS_START,
                                    OSC_TERMINATOR_ST}};

/// Streaming chunk accumulator and parser for [`OSC`] 9;4 progress sequences emitted by
/// child processes in a [`PTY`].
///
/// This is not the raw [`PTY`] read buffer, but a dedicated stream scanner that
/// accumulates [`OSC`] sequences as they are read from child [`PTY`] output. It handles
/// partial sequences that may be split across multiple read operations.
///
/// ## Architectural Context & Separation of Concerns
///
/// While the core [`OSC`] pipeline manages host terminal communication via [`OscSender`]
/// and [`vt_100_terminal_input_parser`], this struct is a specialized **outlier**: a
/// streaming chunk pre-filter plugged into the background [`PTY`] reader thread.
///
/// It is specifically dedicated to extracting out-of-band `OSC 9;4` progress reports
/// (such as `ConEmu` / Windows Terminal build status emitted by Cargo) for progress bars,
/// spinners, and telemetry.
///
/// Full virtual terminal emulation concerns (such as `OSC 8` hyperlinks and `OSC 0`,
/// `1`, `2` window titles) require 2D screen grid state and are handled by
/// [`OfsBufVT100`]. They are deliberately not the concern of this struct.
///
/// For the complete architecture, see the [bidirectional OSC pipeline][osc-pipeline] in
/// the module documentation.
///
/// ## Generating Sequences
///
/// To generate and emit outbound `OSC 9;4` progress sequences to the terminal, use
/// [`OscSender::send_set_progress`]. For outbound window titles and hyperlinks, see
/// [`OscSender::send_set_title_and_tab`] and [`OscSender::send_set_hyperlink`].
///
/// [`OfsBufVT100`]: crate::OfsBufVT100
/// [`OSC`]: crate::OscSequence
/// [`OscPtyEvent`]: crate::core::ansi::osc::OscPtyEvent
/// [`OscSender::send_set_hyperlink`]: crate::OscSender::send_set_hyperlink
/// [`OscSender::send_set_progress`]: crate::OscSender::send_set_progress
/// [`OscSender::send_set_title_and_tab`]: crate::OscSender::send_set_title_and_tab
/// [`OscSender`]: crate::OscSender
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
/// [`vt_100_terminal_input_parser`]: crate::core::ansi::vt_100_terminal_input_parser
/// [osc-pipeline]: mod@crate::core::ansi::osc#architecture--mental-model-bidirectional-osc-pipeline
#[derive(Debug)]
pub struct PtyOscProgressScanner {
    data: String,
}

impl Default for PtyOscProgressScanner {
    fn default() -> Self { Self::new() }
}

impl PtyOscProgressScanner {
    /// Creates a new empty [`OSC`] progress stream scanner.
    ///
    /// [`OSC`]: crate::osc_codes::OscSequence
    #[must_use]
    pub fn new() -> Self {
        Self {
            data: String::new(),
        }
    }

    /// Appends new bytes to the buffer and extracts any complete [`OSC`] sequences.
    ///
    /// # Arguments
    /// * `buffer` - Raw bytes read from the [`PTY`]
    /// * `n` - Number of valid bytes in the buffer
    ///
    /// # Returns
    /// A vector of parsed [`OscPtyEvent`] instances from any complete sequences found.
    ///
    /// [`OSC`]: crate::osc_codes::OscSequence
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    pub fn append_and_extract(&mut self, buffer: &[u8], n: usize) -> Vec<OscPtyEvent> {
        // Convert bytes to string and append to accumulated data.
        let text = String::from_utf8_lossy(&buffer[..n]);
        self.data.push_str(&text);

        let mut events = Vec::new();

        // Find and process all complete OSC sequences.
        while let Some(event) = self.try_extract_next_sequence() {
            events.push(event);
        }

        events
    }

    /// Extracts and parses the next complete [`OSC`] sequence from the buffer.
    ///
    /// Looks for sequences in the format: `ESC]9;4;{state};{progress}ESC\`
    ///
    /// # Returns
    /// * `Some(OscPtyEvent)` if a complete sequence was found and parsed.
    /// * `None` if no complete sequence is available.
    ///
    /// [`OSC`]: crate::osc_codes::OscSequence
    pub fn try_extract_next_sequence(&mut self) -> Option<OscPtyEvent> {
        // OSC sequence format "codes::OSC_PROGRESS_START {state};{progress}
        // codes::OSC_TERMINATOR_ST" Find start of OSC sequence.
        let start_idx = self.data.find(OSC_PROGRESS_START)?;
        let after_start_idx = start_idx + OSC_PROGRESS_START.len();

        // Find end of sequence.
        let end_idx = self.data[after_start_idx..].find(OSC_TERMINATOR_ST)?;
        let params_end_idx = after_start_idx + end_idx;
        let sequence_end_idx = params_end_idx + OSC_TERMINATOR_ST.len();

        // Extract parameters.
        let params = &self.data[after_start_idx..params_end_idx];

        // Parse the sequence.
        let event = self.try_parse_osc_params(params);

        // Remove processed portion from buffer (including everything up to sequence end).
        self.data.drain(0..sequence_end_idx);

        event
    }

    /// Parses [`OSC`] parameters into an [`OscPtyEvent`].
    ///
    /// # Arguments
    /// * `params` - The parameter string in format "{state};{progress}"
    ///
    /// # Returns
    /// * `Some(OscPtyEvent)` if parameters were valid.
    /// * `None` if parameters were malformed or state was unknown.
    ///
    /// [`OSC`]: crate::osc_codes::OscSequence
    #[must_use]
    pub fn try_parse_osc_params(&self, params: &str) -> Option<OscPtyEvent> {
        let parts: Vec<&str> = params.split(OSC_DELIMITER).collect();
        let [state_str, progress_str] = parts.as_slice() else {
            // Gracefully handle malformed sequences.
            return None;
        };

        let state = state_str.parse::<u8>().ok()?;
        let progress = progress_str.parse::<f64>().ok()?;

        match state {
            0 => Some(OscPtyEvent::ProgressCleared),
            1 => {
                // Clamp progress to valid u8 range (0-100).
                let clamped = progress.clamp(0.0, 100.0);
                let percentage = clamped.to_u8_lossy();
                let pc = Pc::try_from(percentage).ok()?;
                Some(OscPtyEvent::ProgressUpdate(pc))
            }
            2 => Some(OscPtyEvent::BuildError),
            3 => Some(OscPtyEvent::IndeterminateProgress),
            _ => None, // Gracefully ignore unknown states
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pc;

    fn progress_update(pct: u8) -> OscPtyEvent {
        OscPtyEvent::ProgressUpdate(pc!(pct).unwrap())
    }

    #[test]
    fn test_single_complete_sequence() {
        let mut buffer = PtyOscProgressScanner::new();

        // Test progress update (state 1)
        let input = b"\x1b]9;4;1;50\x1b\\";
        let events = buffer.append_and_extract(input, input.len());
        assert_eq!(events, vec![progress_update(50)]);

        // Test progress cleared (state 0)
        let input = b"\x1b]9;4;0;0\x1b\\";
        let events = buffer.append_and_extract(input, input.len());
        assert_eq!(events, vec![OscPtyEvent::ProgressCleared]);

        // Test build error (state 2)
        let input = b"\x1b]9;4;2;0\x1b\\";
        let events = buffer.append_and_extract(input, input.len());
        assert_eq!(events, vec![OscPtyEvent::BuildError]);

        // Test indeterminate progress (state 3)
        let input = b"\x1b]9;4;3;0\x1b\\";
        let events = buffer.append_and_extract(input, input.len());
        assert_eq!(events, vec![OscPtyEvent::IndeterminateProgress]);
    }

    #[test]
    fn test_multiple_sequences() {
        let mut buffer = PtyOscProgressScanner::new();

        // Multiple sequences in one buffer.
        let input = b"\x1b]9;4;1;25\x1b\\\x1b]9;4;1;50\x1b\\\x1b]9;4;0;0\x1b\\";
        let events = buffer.append_and_extract(input, input.len());
        assert_eq!(
            events,
            vec![
                progress_update(25),
                progress_update(50),
                OscPtyEvent::ProgressCleared
            ]
        );
    }

    #[test]
    fn test_sequences_with_text_between() {
        let mut buffer = PtyOscProgressScanner::new();

        // OSC sequences with regular text interleaved.
        let input =
            b"Building...\x1b]9;4;1;30\x1b\\Compiling crate...\x1b]9;4;1;60\x1b\\Done!";
        let events = buffer.append_and_extract(input, input.len());
        assert_eq!(events, vec![progress_update(30), progress_update(60)]);

        // Verify remaining text is preserved in buffer.
        assert!(buffer.data.contains("Done!"));
    }

    #[test]
    fn test_split_sequence_across_buffers() {
        let mut buffer = PtyOscProgressScanner::new();

        // First part of sequence.
        let input1 = b"\x1b]9;4;1;";
        let events1 = buffer.append_and_extract(input1, input1.len());
        assert_eq!(events1, vec![]); // No complete sequence yet

        // Second part of sequence.
        let input2 = b"75\x1b\\";
        let events2 = buffer.append_and_extract(input2, input2.len());
        assert_eq!(events2, vec![progress_update(75)]);
    }

    #[test]
    fn test_complex_split_scenarios() {
        let mut buffer = PtyOscProgressScanner::new();

        // Split at different points.
        let parts: [&[u8]; 4] = [b"\x1b]9", b";4;1;", b"42", b"\x1b\\"];

        // Feed parts one by one.
        assert_eq!(buffer.append_and_extract(parts[0], parts[0].len()), vec![]);
        assert_eq!(buffer.append_and_extract(parts[1], parts[1].len()), vec![]);
        assert_eq!(buffer.append_and_extract(parts[2], parts[2].len()), vec![]);
        assert_eq!(
            buffer.append_and_extract(parts[3], parts[3].len()),
            vec![progress_update(42)]
        );
    }

    #[test]
    fn test_invalid_sequences() {
        let mut buffer = PtyOscProgressScanner::new();

        // Missing progress value.
        let input = b"\x1b]9;4;1\x1b\\";
        let events = buffer.append_and_extract(input, input.len());
        assert_eq!(events, vec![]); // Should gracefully ignore

        // Non-numeric progress value.
        let input = b"\x1b]9;4;1;abc\x1b\\";
        let events = buffer.append_and_extract(input, input.len());
        assert_eq!(events, vec![]); // Should gracefully ignore

        // Unknown state value.
        let input = b"\x1b]9;4;99;50\x1b\\";
        let events = buffer.append_and_extract(input, input.len());
        assert_eq!(events, vec![]); // Should gracefully ignore
    }

    #[test]
    fn test_malformed_terminators() {
        let mut buffer = PtyOscProgressScanner::new();

        // Missing terminator - sequence should remain in buffer.
        let input = b"\x1b]9;4;1;50";
        let events = buffer.append_and_extract(input, input.len());
        assert_eq!(events, vec![]);
        assert!(buffer.data.contains("9;4;1;50")); // Data should still be in buffer

        // Now add terminator.
        let input2 = b"\x1b\\";
        let events2 = buffer.append_and_extract(input2, input2.len());
        assert_eq!(events2, vec![progress_update(50)]);
    }

    #[test]
    fn test_out_of_range_values() {
        let mut buffer = PtyOscProgressScanner::new();

        // Progress > 100 should be clamped to 100.
        let input = b"\x1b]9;4;1;150\x1b\\";
        let events = buffer.append_and_extract(input, input.len());
        assert_eq!(events, vec![progress_update(100)]);

        // Negative progress should be clamped to 0.
        let input = b"\x1b]9;4;1;-50\x1b\\";
        let events = buffer.append_and_extract(input, input.len());
        assert_eq!(events, vec![progress_update(0)]);
    }

    #[test]
    fn test_interleaved_incomplete_sequences() {
        let mut buffer = PtyOscProgressScanner::new();

        // Nested/interleaved starts (second start before first completes)
        // This creates an invalid sequence since the first one is missing its terminator.
        let input = b"\x1b]9;4;1;25\x1b]9;4;1;50\x1b\\";
        let events = buffer.append_and_extract(input, input.len());
        // The parser should gracefully handle this malformed input.
        // Since the first sequence is incomplete, nothing should be parsed.
        assert_eq!(events, vec![]);
    }

    #[test]
    fn test_buffer_with_unicode() {
        let mut buffer = PtyOscProgressScanner::new();

        // OSC sequences with Unicode text around them.
        let input = "🚀 Building...\x1b]9;4;1;50\x1b\\✨ Done!".as_bytes();
        let events = buffer.append_and_extract(input, input.len());
        assert_eq!(events, vec![progress_update(50)]);
        assert!(buffer.data.contains("✨ Done!"));
    }

    #[test]
    fn test_rapid_sequence_updates() {
        let mut buffer = PtyOscProgressScanner::new();

        // Simulate rapid progress updates.
        let mut all_events = Vec::new();
        for i in (0..=100).step_by(10) {
            let input = format!("\x1b]9;4;1;{i}\x1b\\");
            let events = buffer.append_and_extract(input.as_bytes(), input.len());
            all_events.extend(events);
        }

        assert_eq!(all_events.len(), 11); // 0, 10, 20, ..., 100
        assert_eq!(all_events[0], progress_update(0));
        assert_eq!(all_events[10], progress_update(100));
    }

    #[test]
    fn test_empty_buffer_operations() {
        let mut buffer = PtyOscProgressScanner::new();

        // Empty input
        let events = buffer.append_and_extract(b"", 0);
        assert_eq!(events, vec![]);

        // Just regular text, no OSC.
        let input = b"Just regular text";
        let events = buffer.append_and_extract(input, input.len());
        assert_eq!(events, vec![]);
        assert!(buffer.data.contains("Just regular text"));
    }

    #[test]
    fn test_partial_sequence_with_corruption() {
        let mut buffer = PtyOscProgressScanner::new();

        // Add partial sequence.
        let partial = b"\x1b]9;4;1;33";
        buffer.append_and_extract(partial, partial.len());
        assert!(buffer.data.contains("\x1b]9;4;1;33"));

        // Add unrelated text - this will corrupt the sequence.
        let text = b"some text";
        buffer.append_and_extract(text, text.len());

        // The buffer now contains: `\x1b]9;4;1;33some text`
        // This is not a valid OSC sequence due to the text in between.

        // Complete the sequence - but it's now invalid due to the text in between.
        let terminator = b"\x1b\\";
        let events = buffer.append_and_extract(terminator, terminator.len());

        // The parser finds `\x1b]9;4;` but `1;33some text` is not valid params
        // So it gracefully ignores the malformed sequence and extracts it.
        assert_eq!(events, vec![]);

        // After extraction attempt, buffer should be empty since the malformed
        // sequence was removed
        assert_eq!(buffer.data, "");
    }

    #[test]
    fn test_partial_sequence_clean() {
        let mut buffer = PtyOscProgressScanner::new();

        // Add partial sequence without corruption.
        let partial = b"\x1b]9;4;1;33";
        buffer.append_and_extract(partial, partial.len());

        // Complete the sequence properly.
        let terminator = b"\x1b\\";
        let events = buffer.append_and_extract(terminator, terminator.len());

        // Should parse correctly.
        assert_eq!(events, vec![progress_update(33)]);

        // Buffer should be empty after successful extraction.
        assert_eq!(buffer.data, "");
    }

    #[test]
    fn test_decimal_progress_values() {
        let mut buffer = PtyOscProgressScanner::new();

        // Test decimal values get truncated to integers.
        let input = b"\x1b]9;4;1;33.7\x1b\\";
        let events = buffer.append_and_extract(input, input.len());
        assert_eq!(events, vec![progress_update(33)]);

        let input = b"\x1b]9;4;1;99.9\x1b\\";
        let events = buffer.append_and_extract(input, input.len());
        assert_eq!(events, vec![progress_update(99)]);
    }
}
