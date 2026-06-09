// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Tests for Device Attributes ([`DA`]) response generation.
//!
//! Terminal apps like fish, vim, and neovim send `CSI c` (DA1 - Primary Device
//! Attributes) to detect terminal capabilities. If the terminal emulator doesn't
//! respond, the app times out after ~10 seconds and falls back with reduced
//! features.
//!
//! [`DA`]: https://vt100.net/docs/vt100-ug/chapter3.html#T3-6

use super::super::test_fixtures_vt_100_ansi_conformance::
    create_test_offscreen_buffer_10r_by_10c;
use crate::tui::terminal_lib_backends::offscreen_buffer::test_fixtures_ofs_buf::
    assert_plain_text_at;

#[test]
fn test_da1_no_params_responds() {
    let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

    let (osc_events, dsr_responses, da_responses) =
        ofs_buf.apply_ansi_bytes("\x1b[c");

    assert_eq!(osc_events.len(), 0, "no OSC events expected");
    assert_eq!(dsr_responses.len(), 0, "no DSR responses expected");
    assert_eq!(da_responses.len(), 1, "expected exactly one DA response");
    assert_eq!(
        da_responses[0], "\x1b[?62;22c",
        "expected CSI ? 62 ; 22 c (VT220 + ANSI color)"
    );
}

#[test]
fn test_da1_param_zero_responds() {
    let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

    let (osc_events, dsr_responses, da_responses) =
        ofs_buf.apply_ansi_bytes("\x1b[0c");

    assert_eq!(osc_events.len(), 0, "no OSC events expected");
    assert_eq!(dsr_responses.len(), 0, "no DSR responses expected");
    assert_eq!(da_responses.len(), 1, "expected exactly one DA response");
    assert_eq!(
        da_responses[0], "\x1b[?62;22c",
        "expected CSI ? 62 ; 22 c"
    );
}

#[test]
fn test_da2_ignored() {
    let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

    let (osc_events, dsr_responses, da_responses) =
        ofs_buf.apply_ansi_bytes("\x1b[>c");

    assert_eq!(osc_events.len(), 0, "no OSC events expected");
    assert_eq!(dsr_responses.len(), 0, "no DSR responses expected");
    assert_eq!(
        da_responses.len(), 0,
        "DA2 should be ignored (intermediates present)"
    );
}

#[test]
fn test_da1_with_nonzero_param_ignored() {
    let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

    let (osc_events, dsr_responses, da_responses) =
        ofs_buf.apply_ansi_bytes("\x1b[1c");

    assert_eq!(osc_events.len(), 0, "no OSC events expected");
    assert_eq!(dsr_responses.len(), 0, "no DSR responses expected");
    assert_eq!(
        da_responses.len(), 0,
        "DA1 with param 1 should be ignored"
    );
}

#[test]
fn test_da1_no_param_does_not_affect_buffer() {
    let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

    let (osc_events, _dsr_responses, _da_responses) =
        ofs_buf.apply_ansi_bytes("\x1b[cafter");

    assert_eq!(osc_events.len(), 0, "no OSC events expected");
    assert_plain_text_at(&ofs_buf, 0, 0, "after");
}
