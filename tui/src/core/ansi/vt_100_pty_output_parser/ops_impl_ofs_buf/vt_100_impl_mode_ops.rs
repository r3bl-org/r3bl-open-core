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
//! [`set_requested_auto_wrap_mode`]: OfsBufVT100::set_requested_auto_wrap_mode
//! [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html

#[allow(clippy::wildcard_imports)]
use super::super::*;
use crate::ActiveScreenBuffer;
#[cfg(test)]
use crate::core::coordinates::bounds_check::cursor_bounds_check::CursorBoundsCheck;
#[cfg(test)]
use crate::{AutoWrapMode, OfsBufVT100};

impl OfsBufVT100 {
    /// Set auto wrap mode on.
    ///
    /// When enabled, text automatically wraps to the next line when it reaches the right
    /// margin.
    pub fn set_requested_auto_wrap_mode(&mut self, requested_state: AutoWrapMode) {
        self.get_parser_global_state_mut().auto_wrap_mode = requested_state;
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
        self.get_parser_global_state_mut().cursor_visibility = requested_state;
    }

    /// Set the mouse tracking mode (Enabled/Disabled).
    ///
    /// Controls whether the terminal captures and reports mouse events (e.g. click,
    /// scroll).
    pub fn set_requested_mouse_tracking_mode(&mut self, state: MouseTrackingMode) {
        self.get_terminal_mode_mut().mouse_tracking_mode = state;
    }

    /// Set the mouse tracking format ([`X10`] vs Sgr).
    ///
    /// [`X10`]: https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h2-Mouse-Tracking
    pub fn set_mouse_tracking_format(&mut self, format: MouseTrackingFormat) {
        self.get_terminal_mode_mut().mouse_tracking_format = format;
    }

    /// Toggle between the primary and alternate screen buffers.
    ///
    /// When switching to the alternate screen buffer:
    /// - Saves the primary cursor position.
    /// - Swaps the 2D grid buffers (primary and alternate).
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
    /// [`ActiveScreenBuffer::Alternate`]: ActiveScreenBuffer::Alternate
    /// [`ActiveScreenBuffer::Primary`]: ActiveScreenBuffer::Primary
    /// [`BCE`]: https://invisible-island.net/xterm/xterm.faq.html#what_is_bce
    pub fn set_alt_screen_mode(&mut self, requested_screen_mode: RequestedScreenMode) {
        match (
            self.get_terminal_mode_mut().active_screen_buffer,
            requested_screen_mode,
        ) {
            // Transition: Primary -> Alternate Screen.
            (ActiveScreenBuffer::Primary, RequestedScreenMode::Alternate) => {
                // Update mode status.
                self.get_terminal_mode_mut().active_screen_buffer =
                    ActiveScreenBuffer::Alternate;

                // Alternate screen must be cleared when entered, as it doesn't preserve
                // state from previous alternate sessions.
                let empty_char = self.create_empty_pixel_char();
                self.get_alternate_buffer_mut().clear_with(empty_char);
            }

            // Transition: Alternate -> Primary Screen.
            (ActiveScreenBuffer::Alternate, RequestedScreenMode::Primary) => {
                // Update mode status.
                self.get_terminal_mode_mut().active_screen_buffer =
                    ActiveScreenBuffer::Primary;
            }

            // No-op: requested mode is already the active mode (e.g. Active -> Alternate)
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests_mode_ops {
    use super::*;
    use crate::{OfsBufVT100, RangeExt, RequestedScreenMode, VPPos, new_style, vp_col,
                vp_height, vp_row, vp_width};

    fn create_test_buffer() -> OfsBufVT100 {
        let size = vp_width(10) + vp_height(6);
        OfsBufVT100::new_empty(size)
    }

    #[test]
    fn test_set_auto_wrap_mode_enabled() {
        let mut buffer = create_test_buffer();

        // Initially should be enabled by default.
        assert_eq!(
            buffer.get_parser_global_state_mut().auto_wrap_mode,
            AutoWrapMode::Enabled
        );

        buffer.set_requested_auto_wrap_mode(AutoWrapMode::Enabled);
        assert_eq!(
            buffer.get_parser_global_state_mut().auto_wrap_mode,
            AutoWrapMode::Enabled
        );
    }

    #[test]
    fn test_set_auto_wrap_mode_disabled() {
        let mut buffer = create_test_buffer();

        buffer.set_requested_auto_wrap_mode(AutoWrapMode::Disabled);
        assert_eq!(
            buffer.get_parser_global_state_mut().auto_wrap_mode,
            AutoWrapMode::Disabled
        );
    }

    #[test]
    fn test_toggle_auto_wrap_mode() {
        let mut buffer = create_test_buffer();

        // Start enabled.
        buffer.set_requested_auto_wrap_mode(AutoWrapMode::Enabled);
        assert_eq!(
            buffer.get_parser_global_state_mut().auto_wrap_mode,
            AutoWrapMode::Enabled
        );

        // Disable.
        buffer.set_requested_auto_wrap_mode(AutoWrapMode::Disabled);
        assert_eq!(
            buffer.get_parser_global_state_mut().auto_wrap_mode,
            AutoWrapMode::Disabled
        );

        // Enable again.
        buffer.set_requested_auto_wrap_mode(AutoWrapMode::Enabled);
        assert_eq!(
            buffer.get_parser_global_state_mut().auto_wrap_mode,
            AutoWrapMode::Enabled
        );
    }

    #[test]
    fn test_alt_screen_buffer_toggle_scenario_1() {
        let mut buffer = create_test_buffer();

        // Initially should be Inactive.
        assert_eq!(
            buffer.get_terminal_mode_mut().active_screen_buffer,
            ActiveScreenBuffer::Primary
        );
        assert_eq!(buffer.get_cursor_pos(), VPPos::default());

        // Set a styled current_style to verify BCE clearing.
        let custom_style = new_style!(bold);
        buffer.get_parser_global_state_mut().current_style = custom_style;

        // Move primary cursor.
        buffer.set_cursor_pos(vp_col(2) + vp_row(3));

        // Toggle to Alternate Screen.
        buffer.set_alt_screen_mode(RequestedScreenMode::Alternate);
        assert_eq!(
            buffer.get_terminal_mode_mut().active_screen_buffer,
            ActiveScreenBuffer::Alternate
        );

        // Cursor pos should be reset to default/alt state (0, 0).
        assert_eq!(
            buffer.get_active_screen_buffer().get_cursor_pos(),
            VPPos::default()
        );
        // Saved hidden (primary) cursor should be (2, 3).
        assert_eq!(
            buffer.get_primary_buffer().get_cursor_pos(),
            vp_col(2) + vp_row(3)
        );

        // Alternate screen should be cleared using custom_style (BCE).
        let expected_empty_char = buffer.create_empty_pixel_char();
        let end = buffer
            .get_active_screen_buffer()
            .get_viewport()
            .get_height()
            .eol_cursor_position();
        let row_range = vp_row(0)..end;
        for row_idx in row_range.as_index_iter() {
            let line = buffer
                .get_active_screen_buffer()
                .get_row(row_idx)
                .expect("conversion error");
            for pixel_char in line {
                assert_eq!(pixel_char, &expected_empty_char);
            }
        }
    }

    #[test]
    fn test_alt_screen_buffer_toggle_scenario_2() {
        let mut buffer = create_test_buffer();

        // Setup: Primary -> Alternate
        buffer
            .get_active_screen_buffer_mut()
            .set_cursor_pos(vp_col(2) + vp_row(3));
        buffer.set_alt_screen_mode(RequestedScreenMode::Alternate);

        // Move alt cursor.
        buffer
            .get_active_screen_buffer_mut()
            .set_cursor_pos(vp_col(4) + vp_row(5));

        // Toggle back to Primary.
        buffer.set_alt_screen_mode(RequestedScreenMode::Primary);
        assert_eq!(
            buffer.get_terminal_mode_mut().active_screen_buffer,
            ActiveScreenBuffer::Primary
        );

        // Cursor pos should restore to (2, 3).
        assert_eq!(
            buffer.get_active_screen_buffer().get_cursor_pos(),
            vp_col(2) + vp_row(3)
        );
        // Saved hidden (alternate) cursor should be (4, 5).
        assert_eq!(
            buffer.get_alternate_buffer_mut().get_cursor_pos(),
            vp_col(4) + vp_row(5)
        );
    }

    #[test]
    fn test_alt_screen_buffer_toggle_scenario_3() {
        let mut buffer = create_test_buffer();

        // Setup: Primary -> Alternate -> Primary
        buffer
            .get_active_screen_buffer_mut()
            .set_cursor_pos(vp_col(2) + vp_row(3));
        buffer.set_alt_screen_mode(RequestedScreenMode::Alternate);
        buffer
            .get_active_screen_buffer_mut()
            .set_cursor_pos(vp_col(4) + vp_row(5));
        buffer.set_alt_screen_mode(RequestedScreenMode::Primary);

        // --- SECOND CYCLE: Primary -> Alternate ---

        // Move primary cursor to a new location.
        buffer
            .get_active_screen_buffer_mut()
            .set_cursor_pos(vp_col(7) + vp_row(8));

        // Change the active style to verify the second BCE clear.
        let new_style = new_style!(italic);
        buffer.get_parser_global_state_mut().current_style = new_style;

        // Toggle to Alternate Screen again.
        buffer.set_alt_screen_mode(RequestedScreenMode::Alternate);
        assert_eq!(
            buffer.get_terminal_mode_mut().active_screen_buffer,
            ActiveScreenBuffer::Alternate
        );

        // Saved hidden (primary) cursor should now be the new location (7, 8).
        assert_eq!(
            buffer.get_primary_buffer().get_cursor_pos(),
            vp_col(7) + vp_row(8)
        );

        // Cursor pos should restore to where we left it in the Alt screen (4, 5).
        assert_eq!(
            buffer.get_active_screen_buffer().get_cursor_pos(),
            vp_col(4) + vp_row(5)
        );

        // Alternate screen should be cleared AGAIN, using the new italic style (BCE).
        let expected_empty_char_italic = buffer.create_empty_pixel_char();
        let end = buffer
            .get_active_screen_buffer()
            .get_viewport()
            .get_height()
            .eol_cursor_position();
        let row_range = vp_row(0)..end;
        for row_idx in row_range.as_index_iter() {
            let line = buffer
                .get_active_screen_buffer()
                .get_row(row_idx)
                .expect("conversion error");
            for pixel_char in line {
                assert_eq!(pixel_char, &expected_empty_char_italic);
            }
        }
    }
}
