// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Device Status Report ([`DSR`]) request and response sequence constants.
//!
//! These constants are used by [`DsrSequence`] for building [`DSR`] responses, and by
//! [`DsrRequestType`] for parsing incoming [`DSR`] requests.
//!
//! See [constants module design] for the three-tier architecture.
//!
//! [`DSR`]: crate::DsrSequence
//! [`DsrRequestType`]: crate::DsrRequestType
//! [`DsrSequence`]: crate::DsrSequence
//! [constants module design]: mod@crate::constants#design

use crate::define_ansi_const;

// DSR request sequences.

define_ansi_const!(@dsr_str : DSR_CURSOR_POSITION_REQUEST = ["6n"] =>
    "Cursor Position Request (DSR 6n)" : "Terminal asks host for cursor position. Host replies with `ESC [ row ; col R`."
);

define_ansi_const!(@dsr_str : DSR_STATUS_REQUEST = ["5n"] =>
    "Status Request (DSR 5n)" : "Terminal asks host for status. Host replies with `ESC [ 0 n` if OK."
);

// DSR response sequence components.

define_ansi_const!(@dsr_str : DSR_RESPONSE_START = [""] =>
    "DSR Response Start" : "Sequence start for DSR responses: `ESC [`."
);

/// Status OK Code ([`DSR`]): The `0` response code indicating terminal status is OK.
///
/// Value: `"0"`.
///
/// [`DSR`]: crate::DsrSequence
pub const DSR_STATUS_OK_CODE: &str = "0";

/// Status Response Terminator ([`DSR`]): Final byte `n` in status response sequences.
///
/// Value: `'n'` dec, `6E` hex.
///
/// Sequence: `CSI 0 n`.
///
/// [`CSI`]: crate::CsiSequence
/// [`DSR`]: crate::DsrSequence
pub const DSR_STATUS_RESPONSE_END: char = 'n';

/// Cursor Position Response Terminator ([`DSR`]): Final byte `R` in cursor position
/// responses.
///
/// Value: `'R'` dec, `52` hex.
///
/// Sequence: `CSI row ; col R`.
///
/// [`CSI`]: crate::CsiSequence
/// [`DSR`]: crate::DsrSequence
pub const DSR_CURSOR_POSITION_RESPONSE_END: char = 'R';

// Complete response sequences for testing.

define_ansi_const!(@dsr_str : DSR_STATUS_OK_FULL_RESPONSE = ["0n"] =>
    "Status OK Full Response (DSR 0n)" : "Complete status OK response: `ESC [ 0 n`."
);

define_ansi_const!(@dsr_str : DSR_STATUS_OK_RESPONSE_STR = ["0n"] =>
    "Status OK Response (DSR 0n)" : "Complete status OK response: `ESC [ 0 n`."
);
