// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Tests for Device Attributes ([`DA`]) response generation.
//!
//! [`DA`]: crate::DaSequence

use crate::PtyResponseEvent;
use crate::core::ansi::constants::{DA1_REQUEST_NO_PARAM, DA1_REQUEST_PARAM_0, DA2_REQUEST};
use super::super::test_fixtures_vt_100_ansi_conformance::create_test_ofs_buf_10r_by_10c;

#[test]
fn test_conformance_da1_request_no_params() {
    let mut ofs_buf_vt_100 = create_test_ofs_buf_10r_by_10c();

    let (_, responses) = ofs_buf_vt_100.apply_ansi_bytes(DA1_REQUEST_NO_PARAM);
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], PtyResponseEvent::PrimaryDeviceAttributes);
}

#[test]
fn test_conformance_da1_request_param_0() {
    let mut ofs_buf_vt_100 = create_test_ofs_buf_10r_by_10c();

    let (_, responses) = ofs_buf_vt_100.apply_ansi_bytes(DA1_REQUEST_PARAM_0);
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0], PtyResponseEvent::PrimaryDeviceAttributes);
}

#[test]
fn test_conformance_da2_request_ignored() {
    let mut ofs_buf_vt_100 = create_test_ofs_buf_10r_by_10c();

    let (_, responses) = ofs_buf_vt_100.apply_ansi_bytes(DA2_REQUEST);
    assert_eq!(responses.len(), 0);
}
