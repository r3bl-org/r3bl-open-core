// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Tests for OSC (Operating System Command) sequences.

use crate::{core::osc::OscEvent,
            offscreen_buffer::test_fixtures_offscreen_buffer::*};
use super::tests_parse_common::create_test_offscreen_buffer_10r_by_10c;
use crate::ansi_parser::ansi_parser_perform_impl::{new, process_bytes};
use crate::core::osc::osc_codes::{OSC0_SET_TITLE_AND_TAB, OSC2_SET_TITLE, OSC8_START, BELL_TERMINATOR};

#[test]
fn test_osc_title_sequences() {
    let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();
    let mut parser = vte::Parser::new();

    {
        let mut processor = new(&mut ofs_buf);

        // Test OSC 0 (set title and icon)
        let sequence = format!("{}Test Title{}", OSC0_SET_TITLE_AND_TAB, BELL_TERMINATOR);
        process_bytes(&mut processor, &mut parser, sequence);

        // Test OSC 2 (set title only)
        let sequence = format!("{}Window Title{}", OSC2_SET_TITLE, BELL_TERMINATOR);
        process_bytes(&mut processor, &mut parser, sequence);

        // Verify OSC events were captured
        assert_eq!(processor.pending_osc_events.len(), 2);

        match &processor.pending_osc_events[0] {
            OscEvent::SetTitleAndTab(title) => {
                assert_eq!(title, "Test Title");
            }
            _ => panic!("Expected SetTitleAndTab event"),
        }

        match &processor.pending_osc_events[1] {
            OscEvent::SetTitleAndTab(title) => {
                assert_eq!(title, "Window Title");
            }
            _ => panic!("Expected SetTitleAndTab event"),
        }
    }
}

#[test]
fn test_osc_hyperlink() {
    let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();
    let mut parser = vte::Parser::new();

    {
        let mut processor = new(&mut ofs_buf);

        // Test OSC 8 hyperlink
        let sequence = format!("{}https://example.com{}Link Text{}{}", 
            OSC8_START, BELL_TERMINATOR, OSC8_START, BELL_TERMINATOR);
        process_bytes(&mut processor, &mut parser, sequence);

        // Verify hyperlink event was captured
        assert_eq!(processor.pending_osc_events.len(), 2);

        match &processor.pending_osc_events[0] {
            OscEvent::Hyperlink { uri, text: _ } => {
                assert_eq!(uri, "https://example.com");
            }
            _ => panic!("Expected Hyperlink event"),
        }
    }

    // Verify text was written
    assert_plain_text_at(&ofs_buf, 0, 0, "Link Text");
}