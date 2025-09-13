// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Tests for performer lifecycle - creation, initialization, and direct state mutation.

use vte::Perform;

use super::tests_fixtures::*;
use crate::{AnsiToOfsBufPerformer, Pos, TuiStyle, col, row};

#[test]
fn test_performer_creation() {
    let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();
    {
        let performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);
        assert_eq!(performer.ofs_buf.my_pos, Pos::default());
        assert_eq!(performer.ofs_buf.ansi_parser_support.current_style, TuiStyle::default());
        assert!(
            performer
                .ofs_buf
                .ansi_parser_support
                .pending_osc_events
                .is_empty()
        );
    }

    assert_eq!(
        ofs_buf.my_pos,
        Pos::default(),
        "Buffer position should remain unchanged on performer creation"
    );
}

/// Tests that validate state persistence in the buffer across multiple performer
/// instances. The performer works directly with mutable references to the buffer, so all
/// changes are immediately visible and persistent in the buffer.
#[test]
fn test_state_persists_in_buffer_across_performer_lifecycles() {
    let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

    // First performer session
    {
        let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);
        performer.ofs_buf.my_pos = row(1) + col(5);
        performer.print('A'); // Cursor advances to (r:1,c:6)
        assert_eq!(
            performer.ofs_buf.my_pos,
            row(1) + col(6),
            "Cursor should advance after printing a character"
        );
    } // End of first performer scope - changes are already in buffer

    assert_eq!(
        ofs_buf.my_pos,
        row(1) + col(6),
        "Buffer position should persist after performer goes out of scope"
    );

    // Second performer session - should start with buffer's current position
    {
        let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);
        // New performer should initialize with buffer's current cursor position
        assert_eq!(
            performer.ofs_buf.my_pos,
            row(1) + col(6),
            "New performer should start with buffer's current position"
        );

        performer.ofs_buf.my_pos = row(4) + col(2);
        performer.print('B'); // Cursor advances to (r:4,c:3)
        assert_eq!(
            performer.ofs_buf.my_pos,
            row(4) + col(3),
            "Cursor should advance after printing a character"
        );
    } // End of second performer scope - changes are already in buffer

    assert_eq!(
        ofs_buf.my_pos,
        row(4) + col(3),
        "Buffer position should persist after second performer goes out of scope"
    );

    // Third performer session - should start with buffer's current position
    {
        let performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);
        assert_eq!(
            performer.ofs_buf.my_pos,
            row(4) + col(3),
            "Third performer should start with previous session's final position"
        );
    } // End of third performer scope - no changes made

    assert_eq!(
        ofs_buf.my_pos,
        row(4) + col(3),
        "Buffer position should persist across performer lifecycles"
    );
}
