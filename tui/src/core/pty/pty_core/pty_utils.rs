// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Utility functions for [PTY] operations.
//!
//! [PTY]: https://en.wikipedia.org/wiki/Pseudoterminal

/// Converts [`portable_pty::ExitStatus`] to [`std::process::ExitStatus`].
///
/// - Handles Unix wait status format encoding and Windows exit codes
/// - Clamps large exit codes to `255` to prevent overflow on Unix systems
/// - On success: uses explicit success status (exit code `0`)
/// - On failure: encodes exit code in Unix wait status format with bounds checking
///
/// [PTY]: https://en.wikipedia.org/wiki/Pseudoterminal
/// [`portable_pty::ExitStatus`]: portable_pty::ExitStatus
/// [`std::process::ExitStatus`]: std::process::ExitStatus
#[must_use]
pub fn pty_to_std_exit_status(
    status: portable_pty::ExitStatus,
) -> std::process::ExitStatus {
    #[cfg(unix)]
    use std::os::unix::process::ExitStatusExt;
    #[cfg(windows)]
    use std::os::windows::process::ExitStatusExt;

    if status.success() {
        // Success case: use explicit success status
        #[cfg(unix)]
        return std::process::ExitStatus::from_raw(0);
        #[cfg(windows)]
        return std::process::ExitStatus::from_raw(0);
    }
    // Failure case: encode exit code properly
    let code = status.exit_code();

    // Ensure we don't overflow when shifting for Unix wait status format.
    let wait_status = if code <= 255 {
        #[allow(clippy::cast_possible_wrap)]
        let code_i32 = code as i32;
        #[cfg(unix)]
        {
            code_i32 << 8
        }
        #[cfg(windows)]
        {
            code_i32
        }
    } else {
        // If exit code is too large, clamp to 255 and encode.
        #[cfg(unix)]
        {
            255_i32 << 8
        }
        #[cfg(windows)]
        {
            255_i32
        }
    };

    #[cfg(unix)]
    return std::process::ExitStatus::from_raw(wait_status);
    #[cfg(windows)]
    return std::process::ExitStatus::from_raw(wait_status as u32);
}
