// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! OS-level type aliases, constants, and utility functions for [`PTY`] operations.
//! - [`Controller`], [`Controlled`] - [`PTY`] halves
//! - [`ControlledChild`], [`ControlledChildTerminationHandle`] - Child process management
//! - [`ControllerReader`], [`ControllerWriter`] - [`PTY`] I/O streams
//! - [`PtyCommand`], [`PtyControlledChildExitStatus`] - Command execution and exit status
//! - [`pty_to_std_exit_status()`] - Convert [`PtyControlledChildExitStatus`] to
//!   [`std::process::ExitStatus`]
//!
//! [`pty_to_std_exit_status()`]: PtyControlledChildExitStatus::pty_to_std_exit_status
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal

use portable_pty::{ChildKiller, CommandBuilder, MasterPty, SlavePty};
use std::ops::Deref;

/// Buffer size for reading [`PTY`] output (4KB stack allocation).
///
/// This is used for the read buffer in [`PTY`] operations.
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
pub const READ_BUFFER_SIZE: usize = 4_096;

/// Type alias for the controlled half of a [`PTY`].
///
/// This represents the process-side of the [`PTY`] that the child process will use for
/// stdin/stdout/stderr.
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
pub type Controlled = Box<dyn SlavePty + Send>;

/// Type alias for the controller half of a [`PTY`].
///
/// This represents the controller half that the parent process uses to read from and
/// write to the child process.
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
pub type Controller = Box<dyn MasterPty + Send>;

/// Type alias for a spawned child process in a [`PTY`].
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
pub type ControlledChild = Box<dyn portable_pty::Child + Send + Sync>;

/// Type alias for the writer used in [`PTY`] operations.
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
pub type ControllerWriter = Box<dyn std::io::Write + Send>;

/// Type alias for the reader used in [`PTY`] operations.
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
pub type ControllerReader = Box<dyn std::io::Read + Send>;

/// Type alias for a controlled child termination handle.
pub type ControlledChildTerminationHandle = Box<dyn ChildKiller + Send + Sync>;

/// Type alias for a validated [`PTY`] command ready for execution.
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
pub type PtyCommand = CommandBuilder;

/// Type alias for the exit status of a controlled child process in a [`PTY`] session.
///
/// Wraps [`portable_pty::ExitStatus`] so that the rest of the codebase does not depend
/// on the [`portable_pty`] crate directly. All [`PTY`]-related exit statuses should use
/// this alias.
///
/// [`portable_pty`]: https://docs.rs/portable-pty
/// [`pty_to_std_exit_status()`]: PtyControlledChildExitStatus::pty_to_std_exit_status
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
#[derive(Debug, Clone)]
pub struct PtyControlledChildExitStatus {
    pub inner: portable_pty::ExitStatus,
}

impl Deref for PtyControlledChildExitStatus {
    type Target = portable_pty::ExitStatus;

    fn deref(&self) -> &Self::Target { &self.inner }
}

impl From<portable_pty::ExitStatus> for PtyControlledChildExitStatus {
    fn from(it: portable_pty::ExitStatus) -> Self { Self { inner: it } }
}

impl From<u32> for PtyControlledChildExitStatus {
    fn from(code: u32) -> Self {
        Self {
            inner: portable_pty::ExitStatus::with_exit_code(code),
        }
    }
}

impl From<PtyControlledChildExitStatus> for std::process::ExitStatus {
    fn from(status: PtyControlledChildExitStatus) -> Self {
        status.pty_to_std_exit_status()
    }
}

impl PtyControlledChildExitStatus {
    /// Converts [`PtyControlledChildExitStatus`] to [`std::process::ExitStatus`].
    ///
    /// - Handles Unix wait status format encoding and Windows exit codes
    /// - Clamps large exit codes to `255` to prevent overflow on Unix systems
    /// - On success: uses explicit success status (exit code `0`)
    /// - On failure: encodes exit code in Unix wait status format with bounds checking
    ///
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    /// [`std::process::ExitStatus`]: std::process::ExitStatus
    #[must_use]
    pub fn pty_to_std_exit_status(&self) -> std::process::ExitStatus {
        #[cfg(unix)]
        use std::os::unix::process::ExitStatusExt;
        #[cfg(windows)]
        use std::os::windows::process::ExitStatusExt;

        if self.inner.success() {
            // Success case: use explicit success status
            #[cfg(unix)]
            return std::process::ExitStatus::from_raw(0);
            #[cfg(windows)]
            return std::process::ExitStatus::from_raw(0);
        }
        // Failure case: encode exit code properly
        let code = self.inner.exit_code();

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
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Compile-time validation that [`PTY`] type aliases are correctly defined.
    ///
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    #[test]
    fn validate_pty_type_aliases_compile() {
        #[allow(dead_code)]
        fn check_controller(_: Controller) {}
        #[allow(dead_code)]
        fn check_controlled(_: Controlled) {}
        #[allow(dead_code)]
        fn check_controlled_child(_: ControlledChild) {}
    }
}
