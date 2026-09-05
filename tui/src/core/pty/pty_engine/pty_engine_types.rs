// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! OS-level type aliases, constants, and utility functions for [`PTY`] operations.
//! - [`Controller`], [`Controlled`] - [`PTY`] halves
//! - [`ControllerReader`], [`ControllerWriter`] - [`PTY`] I/O streams
//! - [`PtyCommand`], [`PtyControlledChildExitStatus`]: command execution and exit status.
//!
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal

use crate::CommandOutputResult;
use portable_pty::{CommandBuilder, MasterPty, SlavePty};
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

/// Type alias for the writer used in [`PTY`] operations.
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
pub type ControllerWriter = Box<dyn std::io::Write + Send>;

/// Type alias for the reader used in [`PTY`] operations.
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
pub type ControllerReader = Box<dyn std::io::Read + Send>;

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
    fn from(it: portable_pty::ExitStatus) -> PtyControlledChildExitStatus {
        PtyControlledChildExitStatus { inner: it }
    }
}

impl From<u32> for PtyControlledChildExitStatus {
    fn from(code: u32) -> PtyControlledChildExitStatus {
        PtyControlledChildExitStatus {
            inner: portable_pty::ExitStatus::with_exit_code(code),
        }
    }
}

impl From<PtyControlledChildExitStatus> for std::process::ExitStatus {
    fn from(status: PtyControlledChildExitStatus) -> Self {
        CommandOutputResult::make_exit_status(status.exit_code())
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
    }

    #[test]
    fn test_pty_controlled_child_exit_status_to_std_exit_status() {
        // Test From<u32> and Deref:
        let status_ok = PtyControlledChildExitStatus::from(0);
        assert_eq!(status_ok.exit_code(), 0);
        let std_ok: std::process::ExitStatus = status_ok.into();
        assert!(std_ok.success());

        // Test From<portable_pty::ExitStatus> and Into<std::process::ExitStatus>:
        let raw = portable_pty::ExitStatus::with_exit_code(42);
        let status_err = PtyControlledChildExitStatus::from(raw);
        assert_eq!(status_err.exit_code(), 42);
        let std_err: std::process::ExitStatus = status_err.into();
        assert!(!std_err.success());
        assert_eq!(std_err.code(), Some(42));
    }
}
