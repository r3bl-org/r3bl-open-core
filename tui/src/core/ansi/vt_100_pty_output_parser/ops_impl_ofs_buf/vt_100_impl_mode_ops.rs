// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Mode setting operations for [`VT-100`]/[`ANSI`] terminal emulation.
//!
//! This module implements mode operations that correspond to [`ANSI`] mode sequences
//! handled by the [`mode_ops`] module. These include:
//!
//! - `SM h` (Set Mode) - [`set_requested_auto_wrap_mode`] ([`AutoWrapState::Enabled`])
//! - `RM l` (Reset Mode) - [`set_requested_auto_wrap_mode`] ([`AutoWrapState::Disabled`])
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
    pub fn set_requested_auto_wrap_mode(&mut self, requested_state: AutoWrapState) {
        self.parser_global_state.auto_wrap_mode = requested_state;
    }

    /// Set the cursor visibility mode.
    ///
    /// Controls whether the terminal cursor is visible ([`DECTCEM`] `?25` mode).
    ///
    /// [`DECTCEM`]: https://en.wikipedia.org/wiki/ANSI_escape_code#Set_terminal_mode
    pub fn set_requested_cursor_visibility_mode(
        &mut self,
        requested_state: CursorVisibilityState,
    ) {
        self.parser_global_state.cursor_visibility = requested_state;
    }

    pub fn set_focus_events_mode(&mut self, enabled: bool) {
        self.terminal_mode.focus_events = enabled;
    }

    /// Handle alternate screen buffer transitions.
    ///
    /// When switching to the alternate screen buffer:
    /// - Saves the primary cursor position.
    /// - Swaps the 2D grid buffers.
    /// - Sets the active cursor position to the saved alternate cursor position.
    /// - Clears the alternate screen buffer with cells carrying the active style to be
    ///   [`BCE`] (Background Color Erase) compliant.
    /// - Updates the terminal mode to [`AlternateScreenState::Active`].
    ///
    /// When switching back to the primary screen buffer:
    /// - Saves the alternate cursor position.
    /// - Swaps the 2D grid buffers back.
    /// - Restores the primary cursor position.
    /// - Updates the terminal mode to [`AlternateScreenState::Inactive`].
    ///
    /// [`AlternateScreenState::Active`]: crate::AlternateScreenState::Active
    /// [`AlternateScreenState::Inactive`]: crate::AlternateScreenState::Inactive
    /// [`BCE`]: https://invisible-island.net/xterm/xterm.faq.html#what_is_bce
    /// [`self.buffer`]: field@crate::OfsBufVT100::ofs_buf
    /// [`self.hidden_screen_state.hidden_buffer`]: field@crate::HiddenScreenState::hidden_buffer
    pub fn set_alt_screen_mode(&mut self, requested_screen_mode: RequestedScreenMode) {
        match (self.terminal_mode.alternate_screen, requested_screen_mode) {
            // Transition: Primary -> Alternate Screen
            (AlternateScreenState::Inactive, RequestedScreenMode::Alternate) => {
                // Swap the screen buffer grids and their respective cursor positions.
                swap(
                    &mut self.ofs_buf.buffer,
                    &mut self.hidden_screen_state.hidden_buffer,
                );
                swap(
                    &mut self.ofs_buf.cursor_pos,
                    &mut self.hidden_screen_state.hidden_cursor_pos,
                );

                // Update mode status.
                self.terminal_mode.alternate_screen = AlternateScreenState::Active;

                // Clear the alternate screen buffer using BCE-compliant active style.
                let empty_char = self.create_empty_pixel_char();
                for line in self.buffer.iter_mut() {
                    for pixel_char in line.iter_mut() {
                        *pixel_char = empty_char;
                    }
                }
            }

            // Transition: Alternate -> Primary Screen
            (AlternateScreenState::Active, RequestedScreenMode::Primary) => {
                // Swap the screen buffer grids and their respective cursor positions.
                swap(
                    &mut self.ofs_buf.buffer,
                    &mut self.hidden_screen_state.hidden_buffer,
                );
                swap(
                    &mut self.ofs_buf.cursor_pos,
                    &mut self.hidden_screen_state.hidden_cursor_pos,
                );

                // Update mode status.
                self.terminal_mode.alternate_screen = AlternateScreenState::Inactive;
            }

            // No-op: requested mode is already the active mode (e.g. Active -> Alternate)
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests_mode_ops {
    use super::*;
    use crate::{OfsBufVT100, Pos, RequestedScreenMode, col, height, new_style, row,
                width};

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
            AutoWrapState::Enabled
        );

        buffer.set_requested_auto_wrap_mode(AutoWrapState::Enabled);
        assert_eq!(
            buffer.parser_global_state.auto_wrap_mode,
            AutoWrapState::Enabled
        );
    }

    #[test]
    fn test_set_auto_wrap_mode_disabled() {
        let mut buffer = create_test_buffer();

        buffer.set_requested_auto_wrap_mode(AutoWrapState::Disabled);
        assert_eq!(
            buffer.parser_global_state.auto_wrap_mode,
            AutoWrapState::Disabled
        );
    }

    #[test]
    fn test_toggle_auto_wrap_mode() {
        let mut buffer = create_test_buffer();

        // Start enabled.
        buffer.set_requested_auto_wrap_mode(AutoWrapState::Enabled);
        assert_eq!(
            buffer.parser_global_state.auto_wrap_mode,
            AutoWrapState::Enabled
        );

        // Disable.
        buffer.set_requested_auto_wrap_mode(AutoWrapState::Disabled);
        assert_eq!(
            buffer.parser_global_state.auto_wrap_mode,
            AutoWrapState::Disabled
        );

        // Enable again.
        buffer.set_requested_auto_wrap_mode(AutoWrapState::Enabled);
        assert_eq!(
            buffer.parser_global_state.auto_wrap_mode,
            AutoWrapState::Enabled
        );
    }

    #[test]
    fn test_alt_screen_buffer_toggle_scenario_1() {
        let mut buffer = create_test_buffer();

        // Initially should be Inactive.
        assert_eq!(
            buffer.terminal_mode.alternate_screen,
            AlternateScreenState::Inactive
        );
        assert_eq!(buffer.cursor_pos, Pos::default());

        // Set a styled current_style to verify BCE clearing.
        let custom_style = new_style!(bold);
        buffer.parser_global_state.current_style = custom_style;

        // Move primary cursor.
        buffer.cursor_pos = col(2) + row(3);

        // Toggle to Alternate Screen.
        buffer.set_alt_screen_mode(RequestedScreenMode::Alternate);
        assert_eq!(
            buffer.terminal_mode.alternate_screen,
            AlternateScreenState::Active
        );

        // Cursor pos should be reset to default/alt state (0, 0).
        assert_eq!(buffer.cursor_pos, Pos::default());
        // Saved hidden (primary) cursor should be (2, 3).
        assert_eq!(
            buffer.hidden_screen_state.hidden_cursor_pos,
            col(2) + row(3)
        );

        // Alternate screen should be cleared using custom_style (BCE).
        let expected_empty_char = buffer.create_empty_pixel_char();
        for line in buffer.buffer.iter() {
            for pixel_char in line.iter() {
                assert_eq!(pixel_char, &expected_empty_char);
            }
        }
    }

    #[test]
    fn test_alt_screen_buffer_toggle_scenario_2() {
        let mut buffer = create_test_buffer();

        // Setup: Primary -> Alternate
        buffer.cursor_pos = col(2) + row(3);
        buffer.set_alt_screen_mode(RequestedScreenMode::Alternate);

        // Move alt cursor.
        buffer.cursor_pos = col(4) + row(5);

        // Toggle back to Primary.
        buffer.set_alt_screen_mode(RequestedScreenMode::Primary);
        assert_eq!(
            buffer.terminal_mode.alternate_screen,
            AlternateScreenState::Inactive
        );

        // Cursor pos should restore to (2, 3).
        assert_eq!(buffer.cursor_pos, col(2) + row(3));
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
        buffer.cursor_pos = col(2) + row(3);
        buffer.set_alt_screen_mode(RequestedScreenMode::Alternate);
        buffer.cursor_pos = col(4) + row(5);
        buffer.set_alt_screen_mode(RequestedScreenMode::Primary);

        // --- SECOND CYCLE: Primary -> Alternate ---

        // Move primary cursor to a new location.
        buffer.cursor_pos = col(7) + row(8);

        // Change the active style to verify the second BCE clear.
        let new_style = new_style!(italic);
        buffer.parser_global_state.current_style = new_style;

        // Toggle to Alternate Screen again.
        buffer.set_alt_screen_mode(RequestedScreenMode::Alternate);
        assert_eq!(
            buffer.terminal_mode.alternate_screen,
            AlternateScreenState::Active
        );

        // Saved hidden (primary) cursor should now be the new location (7, 8).
        assert_eq!(
            buffer.hidden_screen_state.hidden_cursor_pos,
            col(7) + row(8)
        );

        // Cursor pos should restore to where we left it in the Alt screen (4, 5).
        assert_eq!(buffer.cursor_pos, col(4) + row(5));

        // Alternate screen should be cleared AGAIN, using the new italic style (BCE).
        let expected_empty_char_italic = buffer.create_empty_pixel_char();
        for line in buffer.buffer.iter() {
            for pixel_char in line.iter() {
                assert_eq!(pixel_char, &expected_empty_char_italic);
            }
        }
    }
}
