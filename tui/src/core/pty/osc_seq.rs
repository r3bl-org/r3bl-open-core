/*
 *   Copyright (c) 2025 R3BL LLC
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

/// Represents the different types of OSC progress events that Cargo can emit.
#[derive(Debug, Clone, PartialEq)]
pub enum OscEvent {
    /// Set specific progress value 0-100% (OSC state 1).
    ProgressUpdate(u8),
    /// Clear/remove progress indicator (OSC state 0).
    ProgressCleared,
    /// Build error occurred (OSC state 2).
    BuildError,
    /// Indeterminate progress - build is running but no
    /// specific progress (OSC state 3).
    IndeterminateProgress,
}

/// `OSC 9;4` sequence constants wrapped in a dedicated module for clarity.
mod osc_codes {
    /// Sequence prefix: ESC ] 9 ; 4 ;
    pub const START: &str = "\x1b]9;4;";
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
