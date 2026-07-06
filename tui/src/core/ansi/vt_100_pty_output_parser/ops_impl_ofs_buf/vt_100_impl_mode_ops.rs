// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Mode setting operations for [`VT-100`]/[`ANSI`] terminal emulation.
//!
//! This module implements mode operations that correspond to [`ANSI`] mode sequences
//! handled by the [`mode_ops`] module. These include:
//!
//! - `SM h` (Set Mode) - [`set_requested_auto_wrap_mode`] ([`AutoWrapMode::Enabled`])
//! - `RM l` (Reset Mode) - [`set_requested_auto_wrap_mode`] ([`AutoWrapMode::Disabled`])
//!
//! All operations maintain [`VT-100`] compliance and handle proper mode state management
//! for terminal operations.
//!
//! This module implements the business logic for mode operations delegated from the
//! parser shim. The `impl_` prefix follows our naming convention for searchable code
//! organization. See the architecture documentation above for the complete three-layer
//! architecture.
//!
//! [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
//! [`mode_ops`]: crate::core::ansi::vt_100_pty_output_parser::ops::vt_100_shim_mode_ops
//! [`set_requested_auto_wrap_mode`]: crate::OfsBufVT100::set_requested_auto_wrap_mode
//! [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html

#[allow(clippy::wildcard_imports)]
use super::super::*;
use std::mem::swap;

impl OfsBufVT100 {
    /// Set auto wrap mode on.
    ///
    /// When enabled, text automatically wraps to the next line when it reaches the right
    /// margin.
    pub fn set_requested_auto_wrap_mode(&mut self, requested_state: AutoWrapMode) {
        self.parser_global_state.auto_wrap_mode = requested_state;
    }

    /// Set the cursor visibility mode.
    ///
    /// Controls whether the terminal cursor is visible ([`DECTCEM`] `?25` mode).
    ///
    /// [`DECTCEM`]: https://en.wikipedia.org/wiki/ANSI_escape_code#Set_terminal_mode
    pub fn set_requested_cursor_visibility_mode(
        &mut self,
        requested_state: CursorVisibilityMode,
    ) {
        self.parser_global_state.cursor_visibility = requested_state;
    }

    /// Set the mouse tracking mode (Enabled/Disabled).
    ///
    /// Controls whether the terminal captures and reports mouse events (e.g. click,
    /// scroll).
    pub fn set_requested_mouse_tracking_mode(&mut self, state: MouseTrackingMode) {
        self.terminal_mode.mouse_tracking_mode = state;
    }

    /// Set the mouse tracking format ([`X10`] vs Sgr).
    ///
    /// [`X10`]: https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h2-Mouse-Tracking
    pub fn set_mouse_tracking_format(&mut self, format: MouseTrackingFormat) {
        self.terminal_mode.mouse_tracking_format = format;
    }

    /// Toggle between the primary and alternate screen buffers.
    ///
    /// When switching to the alternate screen buffer:
    /// - Saves the primary cursor position.
    /// - Swaps the 2D grid buffers ([`self.buffer`] and
    ///   [`self.hidden_screen_state.hidden_buffer`]).
    /// - Sets the active cursor position to the saved alternate cursor position.
    /// - Clears the alternate screen buffer with cells carrying the active style to be
    ///   [`BCE`] (Background Color Erase) compliant.
    /// - Updates the terminal mode to [`ActiveScreenBuffer::Alternate`].
    ///
    /// When switching back to the primary screen buffer:
    /// - Saves the alternate cursor position.
    /// - Swaps the 2D grid buffers back.
    /// - Restores the primary cursor position.
    /// - Updates the terminal mode to [`ActiveScreenBuffer::Primary`].
    ///
    /// [`ActiveScreenBuffer::Alternate`]: crate::ActiveScreenBuffer::Alternate
    /// [`ActiveScreenBuffer::Primary`]: crate::ActiveScreenBuffer::Primary
    /// [`BCE`]: https://invisible-island.net/xterm/xterm.faq.html#what_is_bce
    /// [`self.buffer`]: field@crate::OfsBufVT100::ofs_buf
    /// [`self.hidden_screen_state.hidden_buffer`]: field@crate::HiddenScreenState::hidden_buffer
    pub fn set_alt_screen_mode(&mut self, requested_screen_mode: RequestedScreenMode) {
        match (
            self.terminal_mode.active_screen_buffer,
            requested_screen_mode,
        ) {
            // Transition: Primary -> Alternate Screen.
            (ActiveScreenBuffer::Primary, RequestedScreenMode::Alternate) => {
                // Swap the screen buffer grids and their respective cursor positions.
                swap(
                    &mut *self.ofs_buf,
                    &mut self.hidden_screen_state.hidden_buffer,
                );
                let current_cursor = self.ofs_buf.get_cursor_pos();
                self.ofs_buf
                    .set_cursor_pos(self.hidden_screen_state.hidden_cursor_pos);
                self.hidden_screen_state.hidden_cursor_pos = current_cursor;

                // Alternate screen must be cleared when entered, as it doesn't
                // preserve state from previous alternate sessions.
                let empty_char = self.create_empty_pixel_char();
                self.ofs_buf.clear_with(empty_char);

                // Update mode status.
                self.terminal_mode.active_screen_buffer = ActiveScreenBuffer::Alternate;
            }

            // Transition: Alternate -> Primary Screen.
            (ActiveScreenBuffer::Alternate, RequestedScreenMode::Primary) => {
                // Restore the primary buffer and cursor position.
                swap(
                    &mut *self.ofs_buf,
                    &mut self.hidden_screen_state.hidden_buffer,
                );
                let current_cursor = self.ofs_buf.get_cursor_pos();
                self.ofs_buf
                    .set_cursor_pos(self.hidden_screen_state.hidden_cursor_pos);
                self.hidden_screen_state.hidden_cursor_pos = current_cursor;

                // Update mode status.
                self.terminal_mode.active_screen_buffer = ActiveScreenBuffer::Primary;
            }

            // No-op: requested mode is already the active mode (e.g. Active -> Alternate)
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests_mode_ops {
    use super::*;
    use crate::{OfsBufVT100, RequestedScreenMode, col, height, new_style, row, width};

    fn create_test_buffer() -> OfsBufVT100 {
        let size = width(10) + height(6);
        OfsBufVT100::new_empty(size)
    }

    #[test]
    fn test_set_auto_wrap_mode_enabled() {
        let mut buffer = create_test_buffer();

        // Initially should be enabled by default.
        assert_eq!(
            buffer.parser_global_state.auto_wrap_mode,
            AutoWrapMode::Enabled
        );

        buffer.set_requested_auto_wrap_mode(AutoWrapMode::Enabled);
        assert_eq!(
            buffer.parser_global_state.auto_wrap_mode,
            AutoWrapMode::Enabled
        );
    }

    #[test]
    fn test_set_auto_wrap_mode_disabled() {
        let mut buffer = create_test_buffer();

        buffer.set_requested_auto_wrap_mode(AutoWrapMode::Disabled);
        assert_eq!(
            buffer.parser_global_state.auto_wrap_mode,
            AutoWrapMode::Disabled
        );
    }

    #[test]
    fn test_toggle_auto_wrap_mode() {
        let mut buffer = create_test_buffer();

        // Start enabled.
        buffer.set_requested_auto_wrap_mode(AutoWrapMode::Enabled);
        assert_eq!(
            buffer.parser_global_state.auto_wrap_mode,
            AutoWrapMode::Enabled
        );

        // Disable.
        buffer.set_requested_auto_wrap_mode(AutoWrapMode::Disabled);
        assert_eq!(
            buffer.parser_global_state.auto_wrap_mode,
            AutoWrapMode::Disabled
        );

        // Enable again.
        buffer.set_requested_auto_wrap_mode(AutoWrapMode::Enabled);
        assert_eq!(
            buffer.parser_global_state.auto_wrap_mode,
            AutoWrapMode::Enabled
        );
    }

    #[test]
    fn test_alt_screen_buffer_toggle_scenario_1() {
        let mut buffer = create_test_buffer();

        // Initially should be Inactive.
        assert_eq!(
            buffer.terminal_mode.active_screen_buffer,
            ActiveScreenBuffer::Primary
        );
        assert_eq!(buffer.get_cursor_pos(), crate::Pos::default());

        // Set a styled current_style to verify BCE clearing.
        let custom_style = new_style!(bold);
        buffer.parser_global_state.current_style = custom_style;

        // Move primary cursor.
        buffer.set_cursor_pos(col(2) + row(3));

        // Toggle to Alternate Screen.
        buffer.set_alt_screen_mode(RequestedScreenMode::Alternate);
        assert_eq!(
            buffer.terminal_mode.active_screen_buffer,
            ActiveScreenBuffer::Alternate
        );

        // Cursor pos should be reset to default/alt state (0, 0).
        assert_eq!(buffer.get_cursor_pos(), crate::Pos::default());
        // Saved hidden (primary) cursor should be (2, 3).
        assert_eq!(
            buffer.hidden_screen_state.hidden_cursor_pos,
            col(2) + row(3)
        );

        // Alternate screen should be cleared using custom_style (BCE).
        let expected_empty_char = buffer.create_empty_pixel_char();
        let height = buffer.get_height().as_usize();
        for line in (0..height).map(|i| buffer.get_row(i).unwrap()) {
            for pixel_char in line {
                assert_eq!(pixel_char, &expected_empty_char);
            }
        }
    }

    #[test]
    fn test_alt_screen_buffer_toggle_scenario_2() {
        let mut buffer = create_test_buffer();

        // Setup: Primary -> Alternate
        buffer.set_cursor_pos(col(2) + row(3));
        buffer.set_alt_screen_mode(RequestedScreenMode::Alternate);

        // Move alt cursor.
        buffer.set_cursor_pos(col(4) + row(5));

        // Toggle back to Primary.
        buffer.set_alt_screen_mode(RequestedScreenMode::Primary);
        assert_eq!(
            buffer.terminal_mode.active_screen_buffer,
            ActiveScreenBuffer::Primary
        );

        // Cursor pos should restore to (2, 3).
        assert_eq!(buffer.get_cursor_pos(), col(2) + row(3));
        // Saved hidden (alternate) cursor should be (4, 5).
        assert_eq!(
            buffer.hidden_screen_state.hidden_cursor_pos,
            col(4) + row(5)
        );
    }

    #[test]
    fn test_alt_screen_buffer_toggle_scenario_3() {
        let mut buffer = create_test_buffer();

        // Setup: Primary -> Alternate -> Primary
        buffer.set_cursor_pos(col(2) + row(3));
        buffer.set_alt_screen_mode(RequestedScreenMode::Alternate);
        buffer.set_cursor_pos(col(4) + row(5));
        buffer.set_alt_screen_mode(RequestedScreenMode::Primary);

        // --- SECOND CYCLE: Primary -> Alternate ---

        // Move primary cursor to a new location.
        buffer.set_cursor_pos(col(7) + row(8));

        // Change the active style to verify the second BCE clear.
        let new_style = new_style!(italic);
        buffer.parser_global_state.current_style = new_style;

        // Toggle to Alternate Screen again.
        buffer.set_alt_screen_mode(RequestedScreenMode::Alternate);
        assert_eq!(
            buffer.terminal_mode.active_screen_buffer,
            ActiveScreenBuffer::Alternate
        );

        // Saved hidden (primary) cursor should now be the new location (7, 8).
        assert_eq!(
            buffer.hidden_screen_state.hidden_cursor_pos,
            col(7) + row(8)
        );

        // Cursor pos should restore to where we left it in the Alt screen (4, 5).
        assert_eq!(buffer.get_cursor_pos(), col(4) + row(5));

        // Alternate screen should be cleared AGAIN, using the new italic style (BCE).
        let expected_empty_char_italic = buffer.create_empty_pixel_char();
        let height = buffer.get_height().as_usize();
        for line in (0..height).map(|i| buffer.get_row(i).unwrap()) {
            for pixel_char in line {
                assert_eq!(pixel_char, &expected_empty_char_italic);
            }
        }
    }
}
