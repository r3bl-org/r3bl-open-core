// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Terminal output operations using crossterm and `r3bl_tui`'s color system.
//!
//! This module provides high-level functions for terminal operations like cursor
//! movement, screen clearing, and formatting. Instead of hardcoded escape sequences,
//! it uses crossterm's proper command system and `r3bl_tui`'s styled text with TUI colors
//! for better maintainability and cross-platform compatibility.

use crossterm::{ExecutableCommand,
                cursor::MoveTo,
                style::{ResetColor, SetBackgroundColor, SetForegroundColor},
                terminal::{Clear, ClearType}};

use crate::{ASText, TuiColor, lock_output_device_as_mut, terminal_io::OutputDevice};

/// Clears the screen and positions cursor at home (top-left).
///
/// Uses crossterm's Clear and `MoveTo` commands for proper cross-platform behavior.
pub fn clear_screen_and_home_cursor(output_device: &OutputDevice) {
    let out = lock_output_device_as_mut!(output_device);
    let _unused = out.execute(Clear(ClearType::All));
    let _unused = out.execute(MoveTo(0, 0));
    let _unused = out.flush(); // Immediate effect needed for screen clearing
}

/// Moves cursor to specific position (1-indexed like terminal coordinates).
///
/// # Arguments
/// * `row` - Row position (1-indexed, where 1 is top)
/// * `col` - Column position (1-indexed, where 1 is left)
pub fn move_cursor_to(output_device: &OutputDevice, row: u16, col: u16) {
    let out = lock_output_device_as_mut!(output_device);
    // Convert to 0-indexed for crossterm.
    let _unused = out.execute(MoveTo(col.saturating_sub(1), row.saturating_sub(1)));
}

/// Sets foreground and background colors for the terminal.
///
/// # Arguments
/// * `fg` - Foreground color
/// * `bg` - Background color
pub fn set_colors(output_device: &OutputDevice, fg: TuiColor, bg: TuiColor) {
    let out = lock_output_device_as_mut!(output_device);
    let _unused = out.execute(SetBackgroundColor(bg.into()));
    let _unused = out.execute(SetForegroundColor(fg.into()));
}

/// Resets all formatting to default.
pub fn reset_formatting(output_device: &OutputDevice) {
    let out = lock_output_device_as_mut!(output_device);
    let _unused = out.execute(ResetColor);
}

/// Clears the current line.
pub fn clear_current_line(output_device: &OutputDevice) {
    let out = lock_output_device_as_mut!(output_device);
    let _unused = out.execute(Clear(ClearType::CurrentLine));
}

/// Flush the output device to ensure all commands are written immediately.
/// Only call this when immediate output is required.
pub fn flush_output(output_device: &OutputDevice) {
    let out = lock_output_device_as_mut!(output_device);
    let _unused = out.flush();
}

/// Write text with the current styling.
pub fn write_text(output_device: &OutputDevice, text: &str) {
    let out = lock_output_device_as_mut!(output_device);
    let _unused = write!(out, "{text}");
}

/// Write styled text using `r3bl_tui`'s `AnsiStyledText` system.
pub fn write_styled_text(output_device: &OutputDevice, styled_text: &ASText) {
    let out = lock_output_device_as_mut!(output_device);
    let _unused = write!(out, "{styled_text}");
}
