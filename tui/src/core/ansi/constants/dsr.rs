// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Device Status Report (DSR) request and response sequence constants.
//!
//! These constants are used by [`DsrSequence`] for building DSR responses, and by
//! [`DsrRequestType`] for parsing incoming DSR requests.
//!
//! [`DsrRequestType`]: crate::DsrRequestType
//! [`DsrSequence`]: crate::DsrSequence

// DSR request sequences.

/// DSR cursor position request: `ESC [ 6 n` (0x1B in hex).
///
/// The terminal sends this to ask the host "where is the cursor?" The host
/// replies with `ESC [ row ; col R`. [`ConPTY`] sends this during session
/// startup and blocks all child stdout forwarding until it receives a response.
///
/// [`ConPTY`]: https://learn.microsoft.com/en-us/windows/console/creating-a-pseudoconsole-session
pub const DSR_CURSOR_POSITION_REQUEST: &str = "\x1b[6n";

/// DSR status request: `ESC [ 5 n` (0x1B in hex).
///
/// The terminal sends this to ask the host "are you OK?" The host replies with
/// [`DSR_STATUS_OK_FULL_RESPONSE`] (`ESC [ 0 n`) to confirm it is operating
/// normally.
pub const DSR_STATUS_REQUEST: &str = "\x1b[5n";

// DSR response sequence components.

/// CSI sequence start for DSR responses: ESC [
pub const DSR_RESPONSE_START: &str = "\x1b[";

/// Status OK code: 0
pub const DSR_STATUS_OK_CODE: &str = "0";

/// Status response terminator: n
pub const DSR_STATUS_RESPONSE_END: char = 'n';

/// Cursor position response terminator: R
pub const DSR_CURSOR_POSITION_RESPONSE_END: char = 'R';

// Complete response sequences for testing.

/// Complete status OK response: `ESC [ 0 n`
pub const DSR_STATUS_OK_FULL_RESPONSE: &str = "\x1b[0n";
