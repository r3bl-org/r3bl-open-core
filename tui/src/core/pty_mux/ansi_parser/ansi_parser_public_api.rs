// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Public API for ANSI/VT sequence processing.
//!
//! There are three categories of escape sequences: **CSI**, **OSC**, and direct **ESC**.
//! These are the fundamental commands a terminal uses to display and control text. They
//! differ primarily in their structure, purpose, and the range of commands they offer.
//!
//! ## 1. CSI Sequences (Control Sequence Introducer)
//!
//! CSI sequences, which begin with `ESC [`, are the most common and versatile type of
//! escape sequence. They are used for a wide variety of terminal operations, mainly
//! related to **cursor movement**, **text formatting**, and **screen manipulation**. The
//! structure `ESC [ param ; param letter` makes them highly flexible. The parameters are
//! typically numbers that modify the command, and the final letter determines the
//! specific action. For example:
//!
//! * `ESC[31m` changes the text color to red.
//! * `ESC[1;2H` moves the cursor to row 1, column 2.
//! * `ESC[2J` clears the entire screen.
//!
//! `vte` spends most of its time parsing these sequences because they are responsible for
//! the majority of what you see on a terminal screen.
//!
//! ## 2. OSC Sequences (Operating System Command)
//!
//! OSC sequences, which start with `ESC ]`, are used for non-display commands that
//! interact with the terminal emulator itself or the operating system. They are typically
//! used for tasks that don't involve drawing characters on the screen. The structure is
//! `ESC ] number ; text ST`, where `ST` is the string terminator (either `ESC \` or
//! `BEL`—the bell character). For example:
//!
//! * `ESC]0;new_titleST` sets the window title of the terminal.
//! * `ESC]2;new_icon_nameST` changes the icon name.
//!
//! These commands are often used by programs to provide user feedback beyond the standard
//! text output, such as setting the title of a shell session to reflect the current
//! working directory.
//!
//! ## 3. Direct ESC Sequences (Single-Character Commands)
//!
//! Direct escape sequences are simpler, single-character commands that start with `ESC`
//! and are followed by a single character. They predate CSI and OSC sequences and are
//! generally used for more fundamental or legacy terminal functions. Unlike CSI and OSC,
//! they don't have a parameter-based structure, making them less flexible but very fast
//! to parse. Examples include:
//!
//! * `ESC 7` saves the current cursor position and attributes.
//! * `ESC 8` restores the cursor position and attributes.
//! * `ESC c` performs a hard reset of the terminal to its initial state.
//!
//! The simplicity of these commands means they are often used for quick, common tasks.
//! They are also used to switch character sets (e.g., from ASCII to a graphics set), a
//! feature that's less common in modern applications but still important for
//! compatibility.

use super::ansi_parser_perform_impl;
use crate::{OffscreenBuffer, Pos, TuiStyle, TuiStyleAttribs, core::osc::OscEvent};

/// Processes ANSI sequences from [`PTY output`] and updates [`OffscreenBuffer`].
///
/// [`PTY output`]: crate::Process::process_pty_output_and_update_buffer
#[derive(Debug)]
pub struct AnsiToBufferProcessor<'a> {
    pub ofs_buf: &'a mut OffscreenBuffer,
    pub cursor_pos: Pos,
    pub current_style: Option<TuiStyle>,
    // SGR state tracking with type-safe pattern using shared TuiStyleAttribs.
    pub attribs: TuiStyleAttribs,
    pub fg_color: Option<crate::TuiColor>,
    pub bg_color: Option<crate::TuiColor>,
    /// Pending OSC events to be retrieved after processing.
    pub pending_osc_events: Vec<OscEvent>,
}

/// Public API to process ANSI/VT sequences and apply them to an [`OffscreenBuffer`].
impl OffscreenBuffer {
    /// Process & apply ANSI/VT sequences directly to this buffer.
    ///
    /// # Arguments
    ///
    /// * `bytes` - The byte sequence containing ANSI/VT escape sequences to process
    ///
    /// # Returns
    ///
    /// A vector of [`OSC events`] that were detected during processing (e.g., title
    /// changes, hyperlinks). Returns an empty vector if no [`OSC events`] were
    /// detected.
    ///
    /// # Example
    ///
    /// ```
    /// use r3bl_tui::{OffscreenBuffer, Size, height, width, SgrCode, ANSIBasicColor};
    ///
    /// let mut buffer = OffscreenBuffer::new_with_capacity_initialized(height(10) + width(10));
    /// let red_text = format!("Hello{a}Red Text{b}",
    ///     a = SgrCode::ForegroundBasic(ANSIBasicColor::DarkRed),
    ///     b = SgrCode::Reset);
    /// let events = buffer.apply_ansi_bytes(red_text);
    /// ```
    ///
    /// # Processing details
    ///
    /// The processor is designed to be transient/stateless - created fresh for each
    /// batch of bytes to process. This is intentional because:
    ///
    /// - Style attributes (`bold`, `fg_color`, etc.) are SGR (Select Graphic Rendition)
    ///   attributes that apply to characters being written. These styles get baked into
    ///   the [`PixelChar`] objects in the buffer. Once a character is written with its
    ///   style, the style state in the processor (the `bold`, `italic`, `fg_color`,
    ///   `bg_color` fields, etc.) is no longer needed.
    /// - Cursor position is working state that gets copied to the buffer at the end of
    ///   processing.
    /// - All persistent state lives in the [`OffscreenBuffer`], not the processor.
    /// - The [`VTE Parser`] (which must maintain state across reads for split sequences)
    ///   is kept separately in the [`Process`] struct.
    ///
    /// [`VTE parser`]: vte::Parser
    /// [`Process`]: crate::pty_mux::Process
    /// [`PixelChar`]: crate::PixelChar
    /// [`OSC events`]: crate::core::osc::OscEvent
    #[must_use]
    pub fn apply_ansi_bytes(&mut self, bytes: impl AsRef<[u8]>) -> Vec<OscEvent> {
        let mut processor = ansi_parser_perform_impl::new(self);

        ansi_parser_perform_impl::process_bytes(
            &mut processor,
            &mut vte::Parser::new(),
            bytes.as_ref(),
        );

        let events = processor.pending_osc_events.clone();

        // The buffer's cursor position will be updated automatically on drop.
        drop(processor);

        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ANSIBasicColor, SgrCode, TuiColor, ansi_parser::csi_codes, height,
                offscreen_buffer::test_fixtures_offscreen_buffer::*, width};

    /// Create a test `OffscreenBuffer` with 10x10 dimensions (9 content rows + 1 status
    /// bar).
    fn create_test_offscreen_buffer() -> OffscreenBuffer {
        OffscreenBuffer::new_with_capacity_initialized(height(10) + width(10))
    }

    #[test]
    #[allow(clippy::items_after_statements)]
    fn test_public_api_plain_text() {
        let mut buffer = create_test_offscreen_buffer();

        const TEXT: &str = "Hello";

        // Test that the public API processes text correctly.
        let events = buffer.apply_ansi_bytes(TEXT);

        // Should not produce any OSC events for SGR sequences.
        assert_eq!(events.len(), 0);

        // Verify "Hello" is in the buffer.
        assert_plain_text_at(&buffer, 0, 0, TEXT);
    }

    #[test]
    #[allow(clippy::items_after_statements)]
    fn test_public_api_with_colors() {
        let mut buffer = create_test_offscreen_buffer();

        const TEXT: &str = "Red Text";

        // Test processing with ANSI color codes.
        let events = buffer.apply_ansi_bytes(format!(
            "{fg_color}{text}{reset}",
            fg_color = SgrCode::ForegroundBasic(ANSIBasicColor::DarkRed),
            text = TEXT,
            reset = SgrCode::Reset
        ));

        // Should not produce any OSC events for SGR sequences.
        assert_eq!(events.len(), 0);

        // Verify the text with proper styling.
        for (col, expected_char) in TEXT.chars().enumerate() {
            assert_styled_char_at(
                &buffer,
                0,
                col,
                expected_char,
                |style_from_buffer| {
                    matches!(
                        style_from_buffer.color_fg,
                        Some(TuiColor::Basic(ANSIBasicColor::DarkRed))
                    )
                },
                "red foreground",
            );
        }
    }

    #[test]
    fn test_public_api_cursor_movement() {
        let mut buffer = create_test_offscreen_buffer();

        // Test cursor movement sequences.
        let events = buffer.apply_ansi_bytes(format!(
            "A{fwd_2}B{up_1}D",
            fwd_2 = csi_codes::CsiSequence::CursorForward(2), // move right 2.
            up_1 = csi_codes::CsiSequence::CursorUp(1),       // move up 1.
        ));

        // Should not produce any OSC events.
        assert_eq!(events.len(), 0);

        // Verify characters are at expected positions.
        assert_plain_char_at(&buffer, 0, 0, 'A');
        assert_plain_char_at(&buffer, 0, 3, 'B'); // After moving right 2.

        // IMPORTANT: Verify the cursor position is correct after all movements.
        // After writing 'A' at (0,0), cursor moves to (0,1).
        // Then CursorForward(2) moves it to (0,3).
        // Then writing 'B' at (0,3) moves cursor to (0,4).
        // Then CursorUp(1) from row 0 stays at row 0 (can't go negative).
        // Then writing 'D' at (0,4) moves cursor to (0,5).
        assert_eq!(
            buffer.my_pos.row_index.as_usize(),
            0,
            "cursor should be at row 0"
        );
        assert_eq!(
            buffer.my_pos.col_index.as_usize(),
            5,
            "cursor should be at column 5 after writing 'D'"
        );

        // Also verify 'D' was written at the expected position.
        assert_plain_char_at(&buffer, 0, 4, 'D');
    }

    #[test]
    fn test_public_api_csi_position_change() {
        use crate::ansi_parser::csi_codes::CsiSequence;

        let mut buffer = create_test_offscreen_buffer();

        // Test through the public API using proper code builders.
        let events = buffer.apply_ansi_bytes(format!(
            "Start{move_to_2_3}Mid{move_to_1_1}Home{move_to_8_8}End",
            move_to_2_3 = CsiSequence::CursorPosition { row: 2, col: 3 }, /* Move to row 2, col 3. */
            move_to_1_1 = CsiSequence::CursorPosition { row: 1, col: 1 }, /* Move to home (1,1). */
            move_to_8_8 = CsiSequence::CursorPosition { row: 8, col: 8 }, /* Move to row 8, col 8. */
        ));

        // Should not produce any OSC events.
        assert_eq!(events.len(), 0);

        // Verify text is at expected positions.
        // "Start" is written at (0,0), then cursor moves to (1,2) and writes "Mid".
        // Then cursor moves to (0,0) and writes "Home" (overwrites "Start").
        // Then cursor moves to (7,7) and writes "End".
        assert_plain_text_at(&buffer, 0, 0, "Home");
        // The 't' from "Start" that wasn't overwritten.
        assert_plain_char_at(&buffer, 0, 4, 't');
        assert_plain_text_at(&buffer, 1, 2, "Mid");
        assert_plain_text_at(&buffer, 7, 7, "End");

        // Final cursor position should be after "End".
        // No status bar in this test buffer, all 10 rows are available.
        // After writing "End" at (7,7), cursor is at (7,10) which wraps to (8,0).
        // since column 10 is beyond the buffer width of 10 (columns 0-9).
        assert_eq!(buffer.my_pos.row_index.as_usize(), 8);
        assert_eq!(buffer.my_pos.col_index.as_usize(), 0); // Wrapped to next line.
    }
}
