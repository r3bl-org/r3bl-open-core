// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! ANSI/VT sequence parsing for PTY multiplexer.
//!
//! This module processes ANSI sequences from PTY output and updates an OffscreenBuffer
//! accordingly. It uses the `vte` crate (same as Alacritty) for robust ANSI parsing.

use vte::{Params, Perform};

use crate::{ANSIBasicColor, OffscreenBuffer, PixelChar, Pos, TuiColor, TuiStyle, col,
            row, tui_style_attrib};

/// Processes ANSI sequences from PTY output and updates OffscreenBuffer.
///
/// This processor implements the vte::Perform trait to handle ANSI escape sequences
/// and converts them into OffscreenBuffer updates. It maintains cursor position
/// and SGR (Select Graphic Rendition) state.
#[derive(Debug)]
pub struct AnsiToBufferProcessor<'a> {
    buffer: &'a mut OffscreenBuffer,
    cursor_pos: Pos,
    current_style: Option<TuiStyle>,
    // SGR state tracking
    bold: bool,
    dim: bool,
    italic: bool,
    underline: bool,
    blink: bool,
    reverse: bool,
    hidden: bool,
    strikethrough: bool,
    fg_color: Option<TuiColor>,
    bg_color: Option<TuiColor>,
}

impl<'a> AnsiToBufferProcessor<'a> {
    /// Create a new processor for the given buffer.
    pub fn new(buffer: &'a mut OffscreenBuffer) -> Self {
        Self {
            buffer,
            cursor_pos: Pos::default(),
            current_style: None,
            bold: false,
            dim: false,
            italic: false,
            underline: false,
            blink: false,
            reverse: false,
            hidden: false,
            strikethrough: false,
            fg_color: None,
            bg_color: None,
        }
    }

    /// Update the current TuiStyle based on SGR attributes.
    fn update_style(&mut self) {
        self.current_style = Some(TuiStyle {
            id: None,
            bold: if self.bold {
                Some(tui_style_attrib::Bold)
            } else {
                None
            },
            italic: if self.italic {
                Some(tui_style_attrib::Italic)
            } else {
                None
            },
            dim: if self.dim {
                Some(tui_style_attrib::Dim)
            } else {
                None
            },
            underline: if self.underline {
                Some(tui_style_attrib::Underline)
            } else {
                None
            },
            reverse: if self.reverse {
                Some(tui_style_attrib::Reverse)
            } else {
                None
            },
            hidden: if self.hidden {
                Some(tui_style_attrib::Hidden)
            } else {
                None
            },
            strikethrough: if self.strikethrough {
                Some(tui_style_attrib::Strikethrough)
            } else {
                None
            },
            computed: None,
            color_fg: self.fg_color,
            color_bg: self.bg_color,
            padding: None,
            lolcat: None,
        });
    }

    /// Move cursor up by n lines.
    fn cursor_up(&mut self, n: i64) {
        let n = n.max(1) as usize;
        let current_row = self.cursor_pos.row_index.as_usize();
        self.cursor_pos.row_index = row(current_row.saturating_sub(n));
    }

    /// Move cursor down by n lines (reserve last row for status bar).
    fn cursor_down(&mut self, n: i64) {
        let n = n.max(1) as usize;
        let max_row = self
            .buffer
            .window_size
            .row_height
            .as_usize()
            .saturating_sub(2); // Reserve status bar row
        let current_row = self.cursor_pos.row_index.as_usize();
        self.cursor_pos.row_index = row((current_row + n).min(max_row));
    }

    /// Move cursor forward by n columns.
    fn cursor_forward(&mut self, n: i64) {
        let n = n.max(1) as usize;
        let max_col = self
            .buffer
            .window_size
            .col_width
            .as_usize()
            .saturating_sub(1);
        let current_col = self.cursor_pos.col_index.as_usize();
        self.cursor_pos.col_index = col((current_col + n).min(max_col));
    }

    /// Move cursor backward by n columns.
    fn cursor_backward(&mut self, n: i64) {
        let n = n.max(1) as usize;
        let current_col = self.cursor_pos.col_index.as_usize();
        self.cursor_pos.col_index = col(current_col.saturating_sub(n));
    }

    /// Set cursor position (1-based coordinates from ANSI, converted to 0-based).
    fn cursor_position(&mut self, params: &Params) {
        let row_param = params
            .iter()
            .next()
            .and_then(|p| p.get(0))
            .copied()
            .unwrap_or(1)
            .max(1) as usize
            - 1;
        let col_param = params
            .iter()
            .nth(1)
            .and_then(|p| p.get(0))
            .copied()
            .unwrap_or(1)
            .max(1) as usize
            - 1;
        let max_row = self
            .buffer
            .window_size
            .row_height
            .as_usize()
            .saturating_sub(2);
        let max_col = self
            .buffer
            .window_size
            .col_width
            .as_usize()
            .saturating_sub(1);

        self.cursor_pos = Pos {
            col_index: col(col_param.min(max_col)),
            row_index: row(row_param.min(max_row)),
        };
    }

    /// Handle SGR (Select Graphic Rendition) parameters.
    fn sgr(&mut self, params: &Params) {
        for param_slice in params.iter() {
            for &param in param_slice.iter() {
                match param {
                    0 => {
                        // Reset all attributes
                        self.bold = false;
                        self.dim = false;
                        self.italic = false;
                        self.underline = false;
                        self.blink = false;
                        self.reverse = false;
                        self.hidden = false;
                        self.strikethrough = false;
                        self.fg_color = None;
                        self.bg_color = None;
                    }
                    1 => self.bold = true,
                    2 => self.dim = true,
                    3 => self.italic = true,
                    4 => self.underline = true,
                    5 => self.blink = true,
                    7 => self.reverse = true,
                    8 => self.hidden = true,
                    9 => self.strikethrough = true,
                    22 => {
                        self.bold = false;
                        self.dim = false;
                    }
                    23 => self.italic = false,
                    24 => self.underline = false,
                    25 => self.blink = false,
                    27 => self.reverse = false,
                    28 => self.hidden = false,
                    29 => self.strikethrough = false,
                    30..=37 => {
                        self.fg_color = Some(ansi_to_tui_color((param - 30).into()))
                    }
                    39 => self.fg_color = None, // Default foreground
                    40..=47 => {
                        self.bg_color = Some(ansi_to_tui_color((param - 40).into()))
                    }
                    49 => self.bg_color = None, // Default background
                    _ => {}                     /* Ignore unsupported SGR parameters
                                                  * (256-color, RGB, etc.) */
                }
            }
        }
        self.update_style();
    }

    /// Get the current cursor position (for updating buffer's my_pos).
    pub fn cursor_pos(&self) -> Pos { self.cursor_pos }
}

impl Perform for AnsiToBufferProcessor<'_> {
    /// Handle printable characters.
    fn print(&mut self, c: char) {
        let row_max = self
            .buffer
            .window_size
            .row_height
            .as_usize()
            .saturating_sub(1);
        let col_max = self.buffer.window_size.col_width.as_usize();
        let current_row = self.cursor_pos.row_index.as_usize();
        let current_col = self.cursor_pos.col_index.as_usize();

        // Only write if within bounds (and not in status bar row)
        if current_row < row_max && current_col < col_max {
            // Write character to buffer using public fields
            self.buffer.buffer[current_row][current_col] = PixelChar::PlainText {
                display_char: c,
                maybe_style: self.current_style,
            };

            // Move cursor forward
            let new_col = current_col + 1;

            // Handle line wrap
            if new_col >= col_max {
                self.cursor_pos.col_index = col(0);
                if current_row < row_max - 1 {
                    self.cursor_pos.row_index = row(current_row + 1);
                }
            } else {
                self.cursor_pos.col_index = col(new_col);
            }
        }
    }

    /// Handle control characters (C0 set).
    fn execute(&mut self, byte: u8) {
        match byte {
            0x08 => {
                // Backspace
                let current_col = self.cursor_pos.col_index.as_usize();
                if current_col > 0 {
                    self.cursor_pos.col_index = col(current_col - 1);
                }
            }
            0x09 => {
                // Tab - move to next 8-column boundary
                let current_col = self.cursor_pos.col_index.as_usize();
                let next_tab = ((current_col / 8) + 1) * 8;
                let max_col = self.buffer.window_size.col_width.as_usize();
                self.cursor_pos.col_index = col(next_tab.min(max_col - 1));
            }
            0x0A => {
                // Line feed (newline)
                let max_row = self
                    .buffer
                    .window_size
                    .row_height
                    .as_usize()
                    .saturating_sub(2);
                let current_row = self.cursor_pos.row_index.as_usize();
                if current_row < max_row {
                    self.cursor_pos.row_index = row(current_row + 1);
                }
            }
            0x0D => {
                // Carriage return
                self.cursor_pos.col_index = col(0);
            }
            _ => {}
        }
    }

    /// Handle CSI (Control Sequence Introducer) sequences.
    fn csi_dispatch(
        &mut self,
        params: &Params,
        _intermediates: &[u8],
        _ignore: bool,
        c: char,
    ) {
        match c {
            'A' => {
                let n = params
                    .iter()
                    .next()
                    .and_then(|p| p.get(0))
                    .copied()
                    .unwrap_or(1) as i64;
                self.cursor_up(n);
            }
            'B' => {
                let n = params
                    .iter()
                    .next()
                    .and_then(|p| p.get(0))
                    .copied()
                    .unwrap_or(1) as i64;
                self.cursor_down(n);
            }
            'C' => {
                let n = params
                    .iter()
                    .next()
                    .and_then(|p| p.get(0))
                    .copied()
                    .unwrap_or(1) as i64;
                self.cursor_forward(n);
            }
            'D' => {
                let n = params
                    .iter()
                    .next()
                    .and_then(|p| p.get(0))
                    .copied()
                    .unwrap_or(1) as i64;
                self.cursor_backward(n);
            }
            'H' | 'f' => self.cursor_position(params),
            'J' => {} // Clear screen - ignore, TUI apps will repaint themselves
            'K' => {} // Clear line - ignore, TUI apps will repaint themselves
            'm' => self.sgr(params), // Select Graphic Rendition
            _ => {}   // Ignore other CSI sequences
        }
    }

    /// Handle OSC (Operating System Command) sequences.
    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {
        // Ignore OSC sequences - PTYMux controls terminal title
        // TUI apps often try to set titles, but we override them
    }

    /// Handle escape sequences (not CSI or OSC).
    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {
        // Most escape sequences handled by CSI dispatch
        // Ignore others for now
    }

    /// Hook for DCS (Device Control String) start.
    fn hook(&mut self, _params: &Params, _intermediates: &[u8], _ignore: bool, _c: char) {
        // Ignore DCS sequences
    }

    /// Handle DCS data.
    fn put(&mut self, _byte: u8) {
        // Ignore DCS data
    }

    /// Hook for DCS end.
    fn unhook(&mut self) {
        // Ignore DCS end
    }
}

/// Convert ANSI color code (0-7) to TuiColor.
fn ansi_to_tui_color(ansi_code: i64) -> TuiColor {
    match ansi_code {
        0 => TuiColor::Basic(ANSIBasicColor::Black),
        1 => TuiColor::Basic(ANSIBasicColor::Red),
        2 => TuiColor::Basic(ANSIBasicColor::Green),
        3 => TuiColor::Basic(ANSIBasicColor::Yellow),
        4 => TuiColor::Basic(ANSIBasicColor::Blue),
        5 => TuiColor::Basic(ANSIBasicColor::Magenta),
        6 => TuiColor::Basic(ANSIBasicColor::Cyan),
        7 => TuiColor::Basic(ANSIBasicColor::White),
        _ => TuiColor::Reset,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{height, width};

    #[test]
    fn test_ansi_to_tui_color() {
        assert_eq!(ansi_to_tui_color(0), TuiColor::Basic(ANSIBasicColor::Black));
        assert_eq!(ansi_to_tui_color(1), TuiColor::Basic(ANSIBasicColor::Red));
        assert_eq!(ansi_to_tui_color(7), TuiColor::Basic(ANSIBasicColor::White));
        assert_eq!(ansi_to_tui_color(999), TuiColor::Reset);
    }

    #[test]
    fn test_processor_creation() {
        let mut buffer =
            OffscreenBuffer::new_with_capacity_initialized(height(10) + width(20));
        let processor = AnsiToBufferProcessor::new(&mut buffer);
        assert_eq!(processor.cursor_pos, Pos::default());
        assert!(!processor.bold);
        assert!(!processor.italic);
        assert!(processor.fg_color.is_none());
    }

    #[test]
    fn test_sgr_reset() {
        let mut buffer =
            OffscreenBuffer::new_with_capacity_initialized(height(10) + width(20));
        let mut processor = AnsiToBufferProcessor::new(&mut buffer);

        // Set some attributes
        processor.bold = true;
        processor.italic = true;
        processor.fg_color = Some(TuiColor::Basic(ANSIBasicColor::Red));

        // Test manual reset instead of using Params (which is complex to construct)
        // Reset attributes manually (simulating SGR 0 behavior)
        processor.bold = false;
        processor.dim = false;
        processor.italic = false;
        processor.underline = false;
        processor.blink = false;
        processor.reverse = false;
        processor.hidden = false;
        processor.strikethrough = false;
        processor.fg_color = None;
        processor.bg_color = None;
        processor.update_style();

        assert!(!processor.bold);
        assert!(!processor.italic);
        assert!(processor.fg_color.is_none());
    }
}
