// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Tests for Device Status Report (DSR) response generation.

use super::super::test_fixtures_vt_100_ansi_conformance::{create_test_offscreen_buffer_10r_by_10c, nz};
use crate::{
    DsrRequestFromPtyEvent, DsrRequestType, col,
            core::ansi::{
                constants::dsr::DSR_STATUS_OK_FULL_RESPONSE,
                vt_100_ansi_parser::{CsiSequence,
                                     vt_100_ansi_conformance_tests::test_sequence_builders::dsr_builders::dsr_cursor_position_response},
            },
            row, term_col, term_row};

#[test]
fn test_dsr_status_report() {
    let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

    // Send CSI 5n (status report request)
    let dsr_request = format!(
        "{}",
        CsiSequence::DeviceStatusReport(DsrRequestType::RequestStatus)
    );
    let (osc_events, dsr_responses) = ofs_buf.apply_ansi_bytes(&dsr_request);

    // Should not produce OSC events.
    assert_eq!(osc_events.len(), 0, "no OSC events expected");

    // Should produce exactly one DSR response.
    assert_eq!(dsr_responses.len(), 1, "expected one DSR response");

    // Check the response is a status report.
    assert_eq!(
        dsr_responses[0],
        DsrRequestFromPtyEvent::TerminalStatus,
        "expected status report response"
    );

    // Verify the response bytes are correct.
    let response_bytes = dsr_responses[0].to_string().into_bytes();
    assert_eq!(
        response_bytes,
        DSR_STATUS_OK_FULL_RESPONSE.as_bytes(),
        "expected ESC[0n response"
    );
}

#[test]
fn test_dsr_cursor_position_report() {
    let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

    // Move cursor to position (3, 5) - 0-based internally
    ofs_buf.cursor_pos = row(3) + col(5);

    // Send CSI 6n (cursor position report request)
    let dsr_request = format!(
        "{}",
        CsiSequence::DeviceStatusReport(DsrRequestType::RequestCursorPosition)
    );
    let (osc_events, dsr_responses) = ofs_buf.apply_ansi_bytes(&dsr_request);

    // Should not produce OSC events.
    assert_eq!(osc_events.len(), 0, "no OSC events expected");

    // Should produce exactly one DSR response.
    assert_eq!(dsr_responses.len(), 1, "expected one DSR response");

    // Check the response is a cursor position report with correct 1-based position.
    assert_eq!(
        dsr_responses[0],
        DsrRequestFromPtyEvent::CursorPosition {
            row: term_row(nz(4)),
            col: term_col(nz(6))
        },
        "expected cursor position report at (4, 6) in 1-based coordinates"
    );

    // Verify the response bytes are correct.
    let response_bytes = dsr_responses[0].to_string().into_bytes();
    let expected_bytes = dsr_cursor_position_response(term_row(nz(4)), term_col(nz(6)));
    assert_eq!(
        response_bytes,
        expected_bytes.as_bytes(),
        "expected ESC[4;6R response"
    );
}

#[test]
fn test_dsr_cursor_position_at_origin() {
    let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

    // Cursor starts at (0, 0) - 0-based internally
    assert_eq!(ofs_buf.cursor_pos, row(0) + col(0));

    // Send CSI 6n (cursor position report request)
    let dsr_request = format!(
        "{}",
        CsiSequence::DeviceStatusReport(DsrRequestType::RequestCursorPosition)
    );
    let (_osc_events, dsr_responses) = ofs_buf.apply_ansi_bytes(&dsr_request);

    // Should produce cursor position report at (1, 1) in 1-based coordinates
    assert_eq!(
        dsr_responses[0],
        DsrRequestFromPtyEvent::CursorPosition {
            row: term_row(nz(1)),
            col: term_col(nz(1))
        },
        "expected cursor position report at (1, 1) for origin"
    );

    // Verify the response bytes.
    let response_bytes = dsr_responses[0].to_string().into_bytes();
    let expected_bytes = dsr_cursor_position_response(term_row(nz(1)), term_col(nz(1)));
    assert_eq!(
        response_bytes,
        expected_bytes.as_bytes(),
        "expected ESC[1;1R response"
    );
}

#[test]
fn test_dsr_unknown_request() {
    let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

    // Send CSI 99n (unknown DSR request)
    let dsr_request = format!(
        "{}",
        CsiSequence::DeviceStatusReport(DsrRequestType::Other(99))
    );
    let (osc_events, dsr_responses) = ofs_buf.apply_ansi_bytes(&dsr_request);

    // Should not produce any events for unknown DSR requests.
    assert_eq!(osc_events.len(), 0, "no OSC events expected");
    assert_eq!(
        dsr_responses.len(),
        0,
        "no DSR responses expected for unknown request"
    );
}

#[test]
fn test_multiple_dsr_requests() {
    let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

    // Move cursor to (2, 3)
    ofs_buf.cursor_pos = row(2) + col(3);

    // Send multiple DSR requests in one sequence.
    let dsr_requests = format!(
        "{}{}{}",
        CsiSequence::DeviceStatusReport(DsrRequestType::RequestStatus), /* Status report */
        CsiSequence::DeviceStatusReport(DsrRequestType::RequestCursorPosition), /* Cursor position */
        CsiSequence::DeviceStatusReport(DsrRequestType::Other(99)) // Unknown
    );
    let (_osc_events, dsr_responses) = ofs_buf.apply_ansi_bytes(&dsr_requests);

    // Should produce exactly two DSR responses (status and cursor position)
    assert_eq!(dsr_responses.len(), 2, "expected two DSR responses");

    // Check the responses.
    assert_eq!(dsr_responses[0], DsrRequestFromPtyEvent::TerminalStatus);
    assert_eq!(
        dsr_responses[1],
        DsrRequestFromPtyEvent::CursorPosition {
            row: term_row(nz(3)),
            col: term_col(nz(4))
        }
    );
}

#[test]
fn test_dsr_events_are_cleared_after_processing() {
    let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

    // First DSR request.
    let dsr_request = format!(
        "{}",
        CsiSequence::DeviceStatusReport(DsrRequestType::RequestStatus)
    );
    let (_, dsr_responses1) = ofs_buf.apply_ansi_bytes(&dsr_request);
    assert_eq!(dsr_responses1.len(), 1, "expected one DSR response");

    // Second call without any DSR request.
    let plain_text = "Hello";
    let (_, dsr_responses2) = ofs_buf.apply_ansi_bytes(plain_text);
    assert_eq!(
        dsr_responses2.len(),
        0,
        "DSR responses should be cleared after each apply_ansi_bytes call"
    );

    // Third DSR request.
    let dsr_request = format!(
        "{}",
        CsiSequence::DeviceStatusReport(DsrRequestType::RequestCursorPosition)
    );
    let (_, dsr_responses3) = ofs_buf.apply_ansi_bytes(&dsr_request);
    assert_eq!(
        dsr_responses3.len(),
        1,
        "should get new DSR response, not accumulate old ones"
    );
}
