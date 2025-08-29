// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Tests for processor lifecycle - creation, initialization, and direct state mutation.

use vte::Perform;

use super::tests_fixtures::*;
use crate::{AnsiToBufferProcessor, Pos, TuiStyleAttribs, col, row};

#[test]
fn test_processor_creation() {
    let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();
    {
        let processor = AnsiToBufferProcessor::new(&mut ofs_buf);
        assert_eq!(processor.ofs_buf.my_pos, Pos::default());
        assert_eq!(
            processor.ofs_buf.ansi_parser_support.attribs,
            TuiStyleAttribs::default(),
            "New processor should have default style attributes"
        );
        assert_eq!(processor.ofs_buf.ansi_parser_support.current_style, None);
        assert_eq!(processor.ofs_buf.ansi_parser_support.fg_color, None);
        assert_eq!(processor.ofs_buf.ansi_parser_support.bg_color, None);
        assert!(
            processor
                .ofs_buf
                .ansi_parser_support
                .pending_osc_events
                .is_empty()
        );
    }

    assert_eq!(
        ofs_buf.my_pos,
        Pos::default(),
        "Buffer position should remain unchanged on processor creation"
    );
}

/// Tests that validate state persistence in the buffer across multiple processor
/// instances. The processor works directly with mutable references to the buffer, so all
/// changes are immediately visible and persistent in the buffer.
#[test]
fn test_state_persists_in_buffer_across_processor_lifecycles() {
    let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

    // First processor session
    {
        let mut processor = AnsiToBufferProcessor::new(&mut ofs_buf);
        processor.ofs_buf.my_pos = row(1) + col(5);
        processor.print('A'); // Cursor advances to (r:1,c:6)
        assert_eq!(
            processor.ofs_buf.my_pos,
            row(1) + col(6),
            "Cursor should advance after printing a character"
        );
    } // End of first processor scope - changes are already in buffer

    assert_eq!(
        ofs_buf.my_pos,
        row(1) + col(6),
        "Buffer position should persist after processor goes out of scope"
    );

    // Second processor session - should start with buffer's current position
    {
        let mut processor = AnsiToBufferProcessor::new(&mut ofs_buf);
        // New processor should initialize with buffer's current cursor position
        assert_eq!(
            processor.ofs_buf.my_pos,
            row(1) + col(6),
            "New processor should start with buffer's current position"
        );

        processor.ofs_buf.my_pos = row(4) + col(2);
        processor.print('B'); // Cursor advances to (r:4,c:3)
        assert_eq!(
            processor.ofs_buf.my_pos,
            row(4) + col(3),
            "Cursor should advance after printing a character"
        );
    } // End of second processor scope - changes are already in buffer

    assert_eq!(
        ofs_buf.my_pos,
        row(4) + col(3),
        "Buffer position should persist after second processor goes out of scope"
    );

    // Third processor session - should start with buffer's current position
    {
        let processor = AnsiToBufferProcessor::new(&mut ofs_buf);
        assert_eq!(
            processor.ofs_buf.my_pos,
            row(4) + col(3),
            "Third processor should start with previous session's final position"
        );
    } // End of third processor scope - no changes made

    assert_eq!(
        ofs_buf.my_pos,
        row(4) + col(3),
        "Buffer position should persist across processor lifecycles"
    );
}
