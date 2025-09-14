// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Tests for OSC (Operating System Command) sequences.

use super::tests_fixtures::*;
use crate::{core::osc::{OscEvent, osc_codes::OscSequence},
            offscreen_buffer::ofs_buf_test_fixtures::*};

#[test]
fn test_osc_title_sequences() {
    let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

    // Test OSC 0 (set title and icon)
    let sequence1 = OscSequence::SetTitleAndIcon("Test Title".to_string()).to_string();
    let (osc_events1, dsr_responses1) = ofs_buf.apply_ansi_bytes(sequence1);

    // Should get one OSC event.
    assert_eq!(osc_events1.len(), 1);
    assert_eq!(dsr_responses1.len(), 0);

    match &osc_events1[0] {
        OscEvent::SetTitleAndTab(title) => {
            assert_eq!(title, "Test Title");
        }
        _ => panic!("Expected SetTitleAndTab event"),
    }

    // Test OSC 2 (set title only)
    let sequence2 = OscSequence::SetTitle("Window Title".to_string()).to_string();
    let (osc_events2, dsr_responses2) = ofs_buf.apply_ansi_bytes(sequence2);

    // Should get one new OSC event (not accumulated)
    assert_eq!(osc_events2.len(), 1);
    assert_eq!(dsr_responses2.len(), 0);

    match &osc_events2[0] {
        OscEvent::SetTitleAndTab(title) => {
            assert_eq!(title, "Window Title");
        }
        _ => panic!("Expected SetTitleAndTab event"),
    }
}

#[test]
fn test_osc_hyperlink() {
    let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

    // Test OSC 8 hyperlink.
    let hyperlink_start = OscSequence::HyperlinkStart {
        uri: "https://example.com".to_string(),
        id: None,
    };
    let hyperlink_end = OscSequence::HyperlinkEnd;
    let sequence = format!("{hyperlink_start}Link Text{hyperlink_end}");
    let (osc_events, dsr_responses) = ofs_buf.apply_ansi_bytes(sequence);

    // Should get two OSC events (start and end hyperlink)
    assert_eq!(osc_events.len(), 2);
    assert_eq!(dsr_responses.len(), 0);

    match &osc_events[0] {
        OscEvent::Hyperlink { uri, text: _ } => {
            assert_eq!(uri, "https://example.com");
        }
        _ => panic!("Expected Hyperlink event"),
    }

    // Verify text was written.
    assert_plain_text_at(&ofs_buf, 0, 0, "Link Text");

    // Verify events are drained on next call.
    let (osc_events2, dsr_responses2) = ofs_buf.apply_ansi_bytes("more text");
    assert_eq!(osc_events2.len(), 0, "OSC events should be drained");
    assert_eq!(dsr_responses2.len(), 0, "DSR responses should be empty");
}
