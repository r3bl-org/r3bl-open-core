// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Trait and implementations for rendering content to ANSI escape sequences.
//! See [`RenderToAnsi`] for details.

use super::PixelCharRenderer;
use crate::{CRLF_BYTES, OffscreenBuffer, SGR_RESET_BYTES};

/// Trait for rendering content to ANSI escape sequences.
///
/// This trait defines a unified interface for converting various buffer types to ANSI
/// escape sequence byte arrays. This enables both the full TUI and lightweight
/// [`choose()`] rendering paths to converge on a single ANSI generation mechanism.
///
/// ## Architecture
///
/// ```text
/// OffscreenBuffer (full TUI)
///        │
///        ├─────────────────┐
///        │                 │
///        ▼                 ▼
/// RenderToAnsi   Alternative Buffer Types
///        │                 │
///        └────────┬────────┘
///                 ▼
///        PixelCharRenderer
///                 │
///                 ▼
///          ANSI Escape Sequences
/// ```
///
/// The trait is intentionally simple to minimize coupling between rendering abstractions.
/// Unified interface for rendering content to ANSI escape sequences.
///
/// This trait defines a contract for any buffer-like type to render itself as
/// ANSI-encoded bytes. Both full TUI (via [`OffscreenBuffer`]) and lightweight modes (via
/// alternative buffer types) can implement this to provide consistent ANSI generation.
///
/// ## Design Principles
///
/// 1. **Backend Agnostic**: The trait doesn't care about I/O backend (crossterm, direct
///    ANSI, etc.)
/// 2. **Simple Contract**: Single method with clear semantics
/// 3. **Reusable**: Multiple implementations possible for different buffer types
/// 4. **Future-Ready**: Designed to work with both current crossterm backend and future
///    direct ANSI backend
///
/// [`choose()`]: crate::choose
/// [`OffscreenBuffer`]: crate::OffscreenBuffer
pub trait RenderToAnsi {
    /// Render this buffer to ANSI escape sequence bytes.
    ///
    /// This method converts the buffer's contents to a byte vector containing ANSI escape
    /// sequences and character data. The output is ready to be written directly to a
    /// terminal device.
    ///
    /// # Returns
    ///
    /// A vector of bytes containing:
    /// - ANSI escape sequences for styling (color, bold, italic, etc.)
    /// - Character data (UTF-8 encoded)
    /// - Line separators (`\r\n` or equivalent)
    ///
    /// # Algorithm
    ///
    /// For each line in the buffer:
    /// - Create a new [`PixelCharRenderer`]
    /// - Call [`render_line()`] on each line's pixels
    /// - Collect all line outputs into a single vector
    /// - Join lines with `\r\n` separators
    /// - Apply final reset if any styling was emitted
    ///
    /// The smart style diffing in [`PixelCharRenderer`] ensures minimal ANSI output.
    ///
    /// [`PixelCharRenderer`]: crate::tui::terminal_lib_backends::direct_to_ansi::pixel_char_renderer::PixelCharRenderer
    /// [`render_line()`]: crate::tui::terminal_lib_backends::direct_to_ansi::pixel_char_renderer::PixelCharRenderer::render_line
    fn render_to_ansi(&self) -> Vec<u8>;
}

impl RenderToAnsi for OffscreenBuffer {
    fn render_to_ansi(&self) -> Vec<u8> {
        let mut output = Vec::new();
        let mut renderer = PixelCharRenderer::new();

        // Iterate through each line in the buffer
        for (row_idx, line) in self.buffer.iter().enumerate() {
            // Add line separator for all lines except the first
            if row_idx > 0 {
                output.extend_from_slice(CRLF_BYTES);
            }

            // Render this line's pixels to ANSI bytes
            let ansi_line = renderer.render_line(&line.pixel_chars);
            output.extend_from_slice(ansi_line);
        }

        // Emit reset at the end if any styling was active
        // This ensures the terminal returns to default state
        if !output.is_empty() {
            output.extend_from_slice(SGR_RESET_BYTES);
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PixelChar, TuiStyle, height, width};

    #[test]
    fn test_render_to_ansi_empty_buffer() {
        let buffer = OffscreenBuffer::new_empty(height(2) + width(3));
        let ansi = buffer.render_to_ansi();

        // Empty buffer (all spacers) should still produce output (spaces + line separator
        // + spaces) The pattern is: spaces, \r\n, spaces, \x1b[0m
        assert!(!ansi.is_empty());
        // Should contain spaces (from Spacer pixels)
        assert!(ansi.contains(&b' '));
        // Should contain line separator
        assert!(ansi.windows(2).any(|w| w == b"\r\n"));
        // Should end with reset
        assert!(ansi.ends_with(b"\x1b[0m"));
    }

    #[test]
    fn test_render_to_ansi_single_line() {
        let mut buffer = OffscreenBuffer::new_empty(height(1) + width(5));

        // Add some plain text
        if let Some(first_line) = buffer.buffer.first_mut() {
            first_line.pixel_chars.clear();
            first_line.pixel_chars.push(PixelChar::PlainText {
                display_char: 'H',
                style: TuiStyle::default(),
            });
            first_line.pixel_chars.push(PixelChar::PlainText {
                display_char: 'i',
                style: TuiStyle::default(),
            });
        }

        let ansi = buffer.render_to_ansi();

        // Should contain 'H' and 'i'
        assert!(ansi.contains(&b'H'));
        assert!(ansi.contains(&b'i'));
        // Should end with reset
        assert!(ansi.ends_with(b"\x1b[0m"));
    }

    #[test]
    fn test_render_to_ansi_multi_line() {
        let mut buffer = OffscreenBuffer::new_empty(height(2) + width(3));

        // Populate two lines
        if buffer.buffer.len() >= 2 {
            // First line
            if let Some(first_line) = buffer.buffer.get_mut(0) {
                first_line.pixel_chars.clear();
                first_line.pixel_chars.push(PixelChar::PlainText {
                    display_char: 'A',
                    style: TuiStyle::default(),
                });
            }

            // Second line
            if let Some(second_line) = buffer.buffer.get_mut(1) {
                second_line.pixel_chars.clear();
                second_line.pixel_chars.push(PixelChar::PlainText {
                    display_char: 'B',
                    style: TuiStyle::default(),
                });
            }
        }

        let ansi = buffer.render_to_ansi();

        // Should contain both 'A' and 'B'
        assert!(ansi.contains(&b'A'));
        assert!(ansi.contains(&b'B'));
        // Should have line separator between lines
        assert!(ansi.windows(2).any(|w| w == b"\r\n"));
        // Should end with reset
        assert!(ansi.ends_with(b"\x1b[0m"));
    }

    #[test]
    fn test_render_to_ansi_with_spacers() {
        let mut buffer = OffscreenBuffer::new_empty(height(1) + width(5));

        if let Some(first_line) = buffer.buffer.first_mut() {
            first_line.pixel_chars.clear();
            first_line.pixel_chars.push(PixelChar::PlainText {
                display_char: 'X',
                style: TuiStyle::default(),
            });
            first_line.pixel_chars.push(PixelChar::Spacer);
            first_line.pixel_chars.push(PixelChar::PlainText {
                display_char: 'Y',
                style: TuiStyle::default(),
            });
        }

        let ansi = buffer.render_to_ansi();

        // Should contain X, space, and Y
        let expected = b"X Y";
        assert!(ansi.windows(3).any(|w| w == expected));
    }

    #[test]
    fn test_render_to_ansi_with_void() {
        let mut buffer = OffscreenBuffer::new_empty(height(1) + width(5));

        if let Some(first_line) = buffer.buffer.first_mut() {
            first_line.pixel_chars.clear();
            first_line.pixel_chars.push(PixelChar::PlainText {
                display_char: 'Z',
                style: TuiStyle::default(),
            });
            first_line.pixel_chars.push(PixelChar::Void);
            first_line.pixel_chars.push(PixelChar::PlainText {
                display_char: 'W',
                style: TuiStyle::default(),
            });
        }

        let ansi = buffer.render_to_ansi();

        // Should contain Z and W (Void should be skipped)
        assert!(ansi.contains(&b'Z'));
        assert!(ansi.contains(&b'W'));
    }
}
