// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Mode setting operations for VT100/[`ANSI`] terminal emulation.
//!
//! This module implements mode operations that correspond to [`ANSI`] mode
//! sequences handled by the [`mode_ops`] module.
//! These include:
//!
//! - **SM h** (Set Mode) - [`set_requested_auto_wrap_mode`] (`AutoWrapState::Enabled`)
//! - **RM l** (Reset Mode) - [`set_requested_auto_wrap_mode`] (`AutoWrapState::Disabled`)
//!
//! All operations maintain VT100 compliance and handle proper mode state
//! management for terminal operations.
//!
//! This module implements the business logic for mode operations delegated from
//! the parser shim. The `impl_` prefix follows our naming convention for searchable
//! code organization. See the architecture documentation above
//! for the complete three-layer architecture.
//!
//! **Related Files:**
//!
//! [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
//! [`mode_ops`]: crate::vt_100_pty_output_parser::operations::vt_100_shim_mode_ops
//! [`set_requested_auto_wrap_mode`]: crate::OffscreenBuffer::set_requested_auto_wrap_mode

#[allow(clippy::wildcard_imports)]
use super::super::*;
use std::mem::swap;

impl OffscreenBuffer {
    /// Set auto wrap mode on.
    /// When enabled, text automatically wraps to the next line when it
    /// reaches the right margin.
    pub fn set_requested_auto_wrap_mode(&mut self, requested_state: AutoWrapState) {
        self.ansi_parser_support.auto_wrap_mode = requested_state;
    }

    /// Set the cursor visibility mode.
    /// Controls whether the terminal cursor is visible (DECTCEM ?25 mode).
    pub fn set_requested_cursor_visibility_mode(
        &mut self,
        requested_state: CursorVisibilityState,
    ) {
        self.ansi_parser_support.cursor_visibility = requested_state;
    }

    /// Toggle between the primary and alternate screen buffers.
    ///
    /// When switching to the alternate screen buffer:
    /// - Saves the primary cursor position.
    /// - Swaps the 2D grid buffers ([`self.buffer`] and
    ///   [`self.alt_screen_support.alt_buffer`]).
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
    /// [`self.alt_screen_support.alt_buffer`]: field@crate::AltScreenSupport::alt_buffer
    /// [`self.buffer`]: field@crate::OffscreenBuffer::buffer
    pub fn set_alt_screen_mode(&mut self, requested_screen_mode: RequestedScreenMode) {
        match (self.terminal_mode.alternate_screen, requested_screen_mode) {
            // Transition: Primary -> Alternate Screen
            (AlternateScreenState::Inactive, RequestedScreenMode::Alternate) => {
                // Save primary cursor.
                self.alt_screen_support.cursor_pos_primary = self.cursor_pos;

                // Swap the screen buffer grids.
                swap(&mut self.buffer, &mut self.alt_screen_support.alt_buffer);

                // Restore alternate cursor.
                self.cursor_pos = self.alt_screen_support.cursor_pos_alt;

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
                // Save alternate cursor.
                self.alt_screen_support.cursor_pos_alt = self.cursor_pos;

                // Swap back to primary buffer.
                swap(&mut self.buffer, &mut self.alt_screen_support.alt_buffer);

                // Restore primary cursor.
                self.cursor_pos = self.alt_screen_support.cursor_pos_primary;

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
    use crate::{Pos, RequestedScreenMode, col, height, new_style, row, width};

    fn create_test_buffer() -> OffscreenBuffer {
        let size = width(10) + height(6);
        OffscreenBuffer::new_empty(size)
    }

    #[test]
    fn test_set_auto_wrap_mode_enabled() {
        let mut buffer = create_test_buffer();

        // Initially should be enabled by default.
        assert_eq!(buffer.ansi_parser_support.auto_wrap_mode, AutoWrapState::Enabled);

        buffer.set_requested_auto_wrap_mode(AutoWrapState::Enabled);
        assert_eq!(buffer.ansi_parser_support.auto_wrap_mode, AutoWrapState::Enabled);
    }

    #[test]
    fn test_set_auto_wrap_mode_disabled() {
        let mut buffer = create_test_buffer();

        buffer.set_requested_auto_wrap_mode(AutoWrapState::Disabled);
        assert_eq!(buffer.ansi_parser_support.auto_wrap_mode, AutoWrapState::Disabled);
    }

    #[test]
    fn test_toggle_auto_wrap_mode() {
        let mut buffer = create_test_buffer();

        // Start enabled.
        buffer.set_requested_auto_wrap_mode(AutoWrapState::Enabled);
        assert_eq!(buffer.ansi_parser_support.auto_wrap_mode, AutoWrapState::Enabled);

        // Disable.
        buffer.set_requested_auto_wrap_mode(AutoWrapState::Disabled);
        assert_eq!(buffer.ansi_parser_support.auto_wrap_mode, AutoWrapState::Disabled);

        // Enable again.
        buffer.set_requested_auto_wrap_mode(AutoWrapState::Enabled);
        assert_eq!(buffer.ansi_parser_support.auto_wrap_mode, AutoWrapState::Enabled);
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
        buffer.ansi_parser_support.current_style = custom_style;

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
        // Saved primary cursor should be (2, 3).
        assert_eq!(
            buffer.alt_screen_support.cursor_pos_primary,
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
        // Saved alternate cursor should be (4, 5).
        assert_eq!(buffer.alt_screen_support.cursor_pos_alt, col(4) + row(5));
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
        buffer.ansi_parser_support.current_style = new_style;

        // Toggle to Alternate Screen again.
        buffer.set_alt_screen_mode(RequestedScreenMode::Alternate);
        assert_eq!(
            buffer.terminal_mode.alternate_screen,
            AlternateScreenState::Active
        );

        // Saved primary cursor should now be the new location (7, 8).
        assert_eq!(
            buffer.alt_screen_support.cursor_pos_primary,
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
