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
//!
//! ## Evolution and Overlap Between ESC and CSI
//!
//! There is significant functional overlap between ESC and CSI sequences, largely due to
//! the evolutionary history of terminal control:
//!
//! **ESC sequences came first**: They were the original, simple terminal control codes
//! used in early terminals like the VT100. Each ESC sequence does one specific thing
//! without parameters. For example, `ESC D` moves the cursor down exactly one line.
//!
//! **CSI sequences evolved later**: As terminals became more sophisticated, the need for
//! parameterized control became apparent. CSI sequences (ESC[) were introduced to provide
//! the same functionality with much greater flexibility. For example, `ESC[5B` moves the
//! cursor down 5 lines, and `ESC[31m` sets the foreground color to red.
//!
//! **Why both exist**: Modern terminals support both for backward compatibility. Many
//! operations can be performed using either approach:
//!
//! | Operation | ESC Sequence | CSI Sequence | Notes |
//! |-----------|-------------|--------------|-------|
//! | Save cursor | `ESC 7` | `ESC[s` | Both work identically |
//! | Restore cursor | `ESC 8` | `ESC[u` | Both work identically |
//! | Move cursor down 1 line | `ESC D` | `ESC[1B` | CSI version can take parameters |
//! | Move cursor up 1 line | `ESC M` | `ESC[1A` | CSI version can take parameters |
//!
//! This overlap is demonstrated in the test suite: the cursor operations tests contain
//! both `test_csi_save_restore_cursor` and `test_esc_save_restore_cursor`, showing both
//! approaches work identically.
//!
//! **Modern practice**: New applications typically use CSI sequences for their
//! flexibility, while ESC sequences remain for compatibility and simple operations that
//! don't need parameters.

use crate::{OffscreenBuffer, core::osc::OscEvent};

/// Terminal state context for ANSI sequence processing.
///
/// This processor is created by [`OffscreenBuffer::apply_ansi_bytes`] and passed to the
/// VTE parser implementation. It provides direct access to persistent terminal state
/// stored in the buffer's [`OffscreenBuffer::ansi_parser_support`] field. All state is
/// stored directly in the buffer and persisted between processor instances.
#[derive(Debug)]
pub struct AnsiToBufferProcessor<'a> {
    /// Target buffer receiving processed terminal output and storing all persistent
    /// terminal state. Characters are written at the current cursor position, and the
    /// buffer's viewport and scrolling are managed automatically as content flows
    /// beyond boundaries.
    pub ofs_buf: &'a mut OffscreenBuffer,
}

impl<'a> AnsiToBufferProcessor<'a> {
    /// Create a new processor for the given `ofs_buf`.
    ///
    /// This creates a processor instance that provides direct access to persistent
    /// terminal state stored in the buffer's `ansi_parser_support` field.
    /// All terminal state is maintained in the buffer and persists between processor
    /// instances.
    pub fn new(ofs_buf: &'a mut OffscreenBuffer) -> Self { Self { ofs_buf } }

    /// Handle the core parsing loop where each byte is fed to the [`VTE parser`], which
    /// in turn calls methods on the processor (via the [`Perform`] trait).
    ///
    /// [`VTE parser`]: vte::Parser
    /// [`Perform`]: vte::Perform
    pub fn process_bytes(&mut self, bytes: impl AsRef<[u8]>) {
        let mut parser = vte::Parser::new();
        for &byte in bytes.as_ref() {
            parser.advance(self, byte);
        }
    }
}

/// Public API to process ANSI/VT sequences and apply them to an [`OffscreenBuffer`].
impl OffscreenBuffer {
    /// Process & apply ANSI/VT sequences directly to this buffer.
    ///
    /// ## Data Flow:
    ///
    /// ```text
    /// 1. Child process (e.g., vim) sends ESC 7 to save cursor
    ///                             ↓
    /// 2. AnsiToBufferProcessor::esc_dispatch() handles ESC 7
    ///                             ↓
    /// 3. Saves current cursor_pos to buffer.my_pos_for_esc_save_and_restore
    ///                             ↓
    /// 4. Later, child sends ESC 8 to restore cursor
    ///                             ↓
    /// 5. AnsiToBufferProcessor::esc_dispatch() handles ESC 8
    ///                             ↓
    /// 6. Restores cursor_pos from buffer.my_pos_for_esc_save_and_restore
    /// ```
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
    /// The processor is designed to be a transient manipulator that works directly on the
    /// buffer's state. It's created fresh for each batch of bytes to process:
    ///
    /// - Style attributes (`bold`, `fg_color`, etc.) are SGR (Select Graphic Rendition)
    ///   attributes that apply to characters being written. These styles get baked into
    ///   the [`PixelChar`] objects in the buffer and stored in the buffer's
    ///   `ansi_parser_support` field for persistence.
    /// - Cursor position is read from and written directly to `buffer.my_pos` during
    ///   processing - no copying or synchronization is needed.
    /// - All persistent state lives in the [`OffscreenBuffer`], accessed directly by the
    ///   processor through mutable references.
    /// - The [`VTE Parser`] (which must maintain state across reads for split sequences)
    ///   is kept separately in the [`Process`] struct.
    ///
    /// [`VTE parser`]: vte::Parser
    /// [`Process`]: crate::pty_mux::Process
    /// [`PixelChar`]: crate::PixelChar
    /// [`OSC events`]: crate::core::osc::OscEvent
    #[must_use]
    pub fn apply_ansi_bytes(&mut self, bytes: impl AsRef<[u8]>) -> Vec<OscEvent> {
        let mut processor = AnsiToBufferProcessor::new(self);
        processor.process_bytes(bytes.as_ref());
        processor
            .ofs_buf
            .ansi_parser_support
            .pending_osc_events
            .clone()
    }
}

#[cfg(test)]
mod tests {
    use crate::{ANSIBasicColor, SgrCode, TuiColor,
                ansi_parser::{ansi_parser_perform_impl_tests::tests_fixtures::create_test_offscreen_buffer_10r_by_10c,
                              csi_codes::{self, csi_seq_cursor_pos},
                              term_units::{term_col, term_row}},
                col,
                offscreen_buffer::test_fixtures_offscreen_buffer::*,
                row};

    #[test]
    #[allow(clippy::items_after_statements)]
    fn test_public_api_plain_text() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        const TEXT: &str = "Hello";

        // Note: OffscreenBuffer uses 0-based index, and terminal (CSI, ESC seq, etc) uses
        // 1-based index.
        //
        // Buffer layout with plain text:
        //
        // Column:   0   1   2   3   4   5   6   7   8   9
        //         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
        // Row 0:  │ H │ e │ l │ l │ o │ ␩ │   │   │   │   │
        //         └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘
        //                               ╰─ cursor ends here

        // Test that the public API processes text correctly.
        let events = ofs_buf.apply_ansi_bytes(TEXT);

        // Should not produce any OSC events for SGR sequences.
        assert_eq!(events.len(), 0, "no OSC events expected");

        // Verify "Hello" is in the buffer.
        assert_plain_text_at(&ofs_buf, 0, 0, TEXT);

        // Verify cursor position is updated correctly.
        assert_eq!(
            ofs_buf.my_pos,
            row(0) + col(TEXT.len()),
            "cursor should be at end of text"
        );
    }

    #[test]
    #[allow(clippy::items_after_statements)]
    fn test_public_api_with_colors() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        const TEXT: &str = "Red Text";

        // Note: OffscreenBuffer uses 0-based index, and terminal (CSI, ESC seq, etc) uses
        // 1-based index.
        //
        // Buffer layout with colored text:
        //
        // Column:   0   1   2   3   4   5   6   7   8   9
        //         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
        // Row 0:  │ R │ e │ d │   │ T │ e │ x │ t │ ␩ │   │
        //         └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘
        //          ╰─────────────────────────────╯  ╰─ cursor ends here
        //           All chars have red foreground
        //
        // Sequence: ESC[31m + "Red Text" + ESC[0m

        // Test processing with ANSI color codes.
        let events = ofs_buf.apply_ansi_bytes(format!(
            "{red_fg}{text}{reset}",
            red_fg = SgrCode::ForegroundBasic(ANSIBasicColor::Red),
            text = TEXT,
            reset = SgrCode::Reset
        ));

        // Should not produce any OSC events for SGR sequences.
        assert_eq!(events.len(), 0, "no OSC events expected");

        // Verify the text with proper styling.
        for (col, expected_char) in TEXT.chars().enumerate() {
            assert_styled_char_at(
                &ofs_buf,
                0,
                col,
                expected_char,
                |style_from_buffer| {
                    style_from_buffer.color_fg
                        == Some(TuiColor::Basic(ANSIBasicColor::Red))
                },
                "red foreground",
            );
        }

        // Verify cursor position is updated correctly.
        assert_eq!(
            ofs_buf.my_pos,
            row(0) + col(TEXT.len()),
            "cursor should be at end of text"
        );
    }

    #[test]
    fn test_public_api_cursor_movement() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // Note: OffscreenBuffer uses 0-based index, and terminal (CSI, ESC seq, etc) uses
        // 1-based index.
        //
        // Buffer layout after cursor movements:
        //
        // Column:   0   1   2   3   4   5   6   7   8   9
        //         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
        // Row 0:  │ A │   │   │ B │ D │ ␩ │   │   │   │   │
        //         └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘
        //                               ╰─── cursor ends after writing 'D'
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
            right_2 = csi_codes::CsiSequence::CursorForward(2),
            up_1 = csi_codes::CsiSequence::CursorUp(1),
        ));

        // Should not produce any OSC events.
        assert_eq!(events.len(), 0, "no OSC events expected");

        // Verify cursor position after all operations.
        assert_eq!(
            ofs_buf.my_pos,
            row(0) + col(5),
            "cursor should be at (0,5) after writing 'D'"
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
        //
        // Buffer layout after cursor position changes:
        //
        // Column:   0   1   2   3   4   5   6   7   8   9
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
        // Row 8:  │ ␩ │   │   │   │   │   │   │   │   │   │ ← cursor ends here (8,0)
        //         └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘   after wrapping
        //
        // Sequence: "Start" → move(2,3) → "Mid" → move(1,1) → "Home" → move(8,8) → "End"

        let events = ofs_buf.apply_ansi_bytes(format!(
            "Start{move_to_r2_c3}Mid{move_to_r1_c1}Home{move_to_r8_c8}End",
            move_to_r2_c3 = csi_seq_cursor_pos(term_row(2) + term_col(3)),
            move_to_r1_c1 = csi_seq_cursor_pos(term_row(1) + term_col(1)),
            move_to_r8_c8 = csi_seq_cursor_pos(term_row(8) + term_col(8)),
        ));

        assert_eq!(events.len(), 0, "no OSC events expected");

        // Verify layout matches diagram.
        // cspell:disable-next-line
        assert_plain_text_at(&ofs_buf, 0, 0, "Homet");
        assert_plain_text_at(&ofs_buf, 1, 2, "Mid");
        assert_plain_text_at(&ofs_buf, 7, 7, "End");

        // Cursor wraps from (7,10) to (8,0).
        assert_eq!(
            ofs_buf.my_pos,
            row(8) + col(0),
            "cursor should be at (8,0) wrapping after 'End'"
        );
    }
}
