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
    /// let mut ofs_buf = OffscreenBuffer::new_empty(height(10) + width(10));
    /// let red_text = format!("Hello{a}Red Text{b}",
    ///     a = SgrCode::ForegroundBasic(ANSIBasicColor::DarkRed),
    ///     b = SgrCode::Reset);
    /// let events = ofs_buf.apply_ansi_bytes(red_text);
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
    use crate::{ANSIBasicColor, SgrCode, TuiColor,
                ansi_parser::{ansi_parser_perform_impl_tests::tests_parse_common::create_test_offscreen_buffer_10r_by_10c,
                              csi_codes::{self, CsiSequence}},
                offscreen_buffer::test_fixtures_offscreen_buffer::*};

    #[test]
    #[allow(clippy::items_after_statements)]
    fn test_public_api_plain_text() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        const TEXT: &str = "Hello";

        // Test that the public API processes text correctly.
        let events = ofs_buf.apply_ansi_bytes(TEXT);

        // Should not produce any OSC events for SGR sequences.
        assert_eq!(events.len(), 0);

        // Verify "Hello" is in the buffer.
        assert_plain_text_at(&ofs_buf, 0, 0, TEXT);
    }

    #[test]
    #[allow(clippy::items_after_statements)]
    fn test_public_api_with_colors() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        const TEXT: &str = "Red Text";

        // Test processing with ANSI color codes.
        let events = ofs_buf.apply_ansi_bytes(format!(
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
                &ofs_buf,
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
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // Note: OffscreenBuffer uses 0-based index, and terminal (CSI, ESC seq, etc) uses
        // 1-based index.

        // Buffer layout after cursor movements:
        //
        // Column:  0   1   2   3   4   5   6   7   8   9
        //         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
        // Row 0:  │ A │   │   │ B │ D │   │   │   │   │   │
        //         └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘
        //
        // Sequence breakdown:
        // 1. Write 'A' at (0,0) → cursor moves to (0,1)
        // 2. CursorForward(2) → cursor moves to (0,3)
        // 3. Write 'B' at (0,3) → cursor moves to (0,4)
        // 4. CursorUp(1) → cursor stays at (0,4) (can't go up from row 0)
        // 5. Write 'D' at (0,4) → cursor moves to (0,5)

        // Test cursor movement sequences.
        let events = ofs_buf.apply_ansi_bytes(format!(
            "A{right_2}B{up_1}D",
            right_2 = csi_codes::CsiSequence::CursorForward(2), // move right 2.
            up_1 = csi_codes::CsiSequence::CursorUp(1),         // move up 1.
        ));

        // Should not produce any OSC events.
        assert_eq!(events.len(), 0);

        assert_eq!(
            ofs_buf.my_pos.row_index.as_usize(),
            0,
            "cursor should be at row 0"
        );
        assert_eq!(
            ofs_buf.my_pos.col_index.as_usize(),
            5,
            "cursor should be at column 5 after writing 'D'"
        );

        // Verify characters at specific positions instead of continuous string.
        assert_plain_char_at(&ofs_buf, 0, 0, 'A');
        assert_empty_at(&ofs_buf, 0, 1); // Empty space
        assert_empty_at(&ofs_buf, 0, 2); // Empty space
        assert_plain_text_at(&ofs_buf, 0, 3, "BD");
    }

    #[test]
    fn test_public_api_csi_position_change() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // Note: OffscreenBuffer uses 0-based index, and terminal (CSI, ESC seq, etc) uses
        // 1-based index.

        // Buffer layout after cursor position changes:
        //
        // Column:  0   1   2   3   4   5   6   7   8   9
        //         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
        // Row 0:  │ H │ o │ m │ e │ t │   │   │   │   │   │
        //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤
        // Row 1:  │   │   │ M │ i │ d │   │   │   │   │   │
        //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤
        // Row 2:  │   │   │   │   │   │   │   │   │   │   │
        //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤
        //         │ … │ … │ … │ … │ … │ … │ … │ … │ … │ … │
        //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤
        // Row 7:  │   │   │   │   │   │   │   │ E │ n │ d │
        //         ├───┼───┼───┼───┼───┼───┼───┼───┼───┼───┤
        // Row 8:  │ ← cursor ends here (8,0) after wrapping
        //         └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘
        //
        // Sequence: "Start" → move(1,2) → "Mid" → move(0,0) → "Home" → move(7,7) → "End"

        let events = ofs_buf.apply_ansi_bytes(format!(
            "Start{move_to_2_3}Mid{move_to_1_1}Home{move_to_8_8}End",
            move_to_2_3 = CsiSequence::CursorPosition { row: 2, col: 3 },
            move_to_1_1 = CsiSequence::CursorPosition { row: 1, col: 1 },
            move_to_8_8 = CsiSequence::CursorPosition { row: 8, col: 8 },
        ));

        assert_eq!(events.len(), 0);

        // Verify layout matches diagram.
        assert_plain_text_at(&ofs_buf, 0, 0, "Homet");
        assert_plain_text_at(&ofs_buf, 1, 2, "Mid");
        assert_plain_text_at(&ofs_buf, 7, 7, "End");

        // Cursor wraps from (7,10) to (8,0).
        assert_eq!(ofs_buf.my_pos.row_index.as_usize(), 8);
        assert_eq!(ofs_buf.my_pos.col_index.as_usize(), 0);
    }
}
