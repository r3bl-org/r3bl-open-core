// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Tests for processor lifecycle - creation, initialization, and Drop trait behavior.

use vte::Perform;

use super::create_test_offscreen_buffer_10r_by_10c;
use crate::{Pos, AnsiToBufferProcessor, col, row};

#[test]
fn test_processor_creation() {
    let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();
    let processor = AnsiToBufferProcessor::new(&mut ofs_buf);
    assert_eq!(processor.cursor_pos, Pos::default());
    assert!(processor.attribs.bold.is_none());
    assert!(processor.attribs.italic.is_none());
    assert!(processor.fg_color.is_none());
}

/// Tests specifically for the Drop trait implementation of AnsiToBufferProcessor.
/// The Drop trait updates ofs_buf.my_pos with the final cursor position when the
/// processor is dropped.
pub mod drop_trait {
    use super::*;

    #[test]
    fn test_drop_updates_cursor_position() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // Initial buffer cursor position should be at origin
        assert_eq!(ofs_buf.my_pos, Pos::default());

        {
            let mut processor = AnsiToBufferProcessor::new(&mut ofs_buf);

            // Move cursor to a specific position
            processor.cursor_pos = row(3) + col(7);
            processor.print('D'); // Character advances cursor to (3, 8)

            // Verify processor cursor is at expected position
            assert_eq!(processor.cursor_pos, row(3) + col(8));
        } // processor drops here

        // After drop, buffer's my_pos should be updated to final processor cursor
        // position
        assert_eq!(
            ofs_buf.my_pos,
            row(3) + col(8),
            "Drop should update buffer cursor position to match processor's final position"
        );
    }

    #[test]
    fn test_multiple_processor_lifecycles() {
        let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

        // First processor session
        {
            let mut processor = AnsiToBufferProcessor::new(&mut ofs_buf);
            processor.cursor_pos = row(1) + col(5);
            processor.print('A'); // Cursor advances to (1, 6)
        } // First drop - should update ofs_buf.my_pos to (1, 6)

        assert_eq!(
            ofs_buf.my_pos,
            row(1) + col(6),
            "First drop should update buffer position"
        );

        // Second processor session - should start with buffer's current position
        {
            let mut processor = AnsiToBufferProcessor::new(&mut ofs_buf);
            // New processor should initialize with buffer's current cursor position
            assert_eq!(
                processor.cursor_pos,
                row(1) + col(6),
                "New processor should start with buffer's current position"
            );

            processor.cursor_pos = row(4) + col(2);
            processor.print('B'); // Cursor advances to (4, 3)
        } // Second drop - should update ofs_buf.my_pos to (4, 3)

        assert_eq!(
            ofs_buf.my_pos,
            row(4) + col(3),
            "Second drop should update buffer position"
        );

        // Third processor session
        {
            let processor = AnsiToBufferProcessor::new(&mut ofs_buf);
            assert_eq!(
                processor.cursor_pos,
                row(4) + col(3),
                "Third processor should start with previous session's final position"
            );
        } // Third drop - position unchanged, should remain (4, 3)

        assert_eq!(
            ofs_buf.my_pos,
            row(4) + col(3),
            "Buffer position should persist across processor lifecycles"
        );
    }

}
