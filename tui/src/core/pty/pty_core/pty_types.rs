// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Core type aliases, constants, and utility functions for [`PTY`] operations.
//! - [`Controller`], [`Controlled`] - [`PTY`] halves
//! - [`ControlledChild`], [`ControlledChildTerminationHandle`] - Child process management
//! - [`ControllerReader`], [`ControllerWriter`] - [`PTY`] I/O streams
//! - [`PtyCommand`], [`PtyControlledChildExitStatus`], [`PtyCompletionHandle`] - Command
//!   execution and exit status
//! - [`pty_to_std_exit_status()`] - Convert [`PtyControlledChildExitStatus`] to
//!   [`std::process::ExitStatus`]
//! - [`InputEventSenderHalf`], [`ReadOnlyOutputEventReceiverHalf`],
//!   [`ReadWriteOutputEventReceiverHalf`] - Channel halves
//!
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal

use super::{PtyInputEvent, PtyReadOnlyOutputEvent, PtyReadWriteOutputEvent};
use portable_pty::{ChildKiller, CommandBuilder, MasterPty, SlavePty};
use std::pin::Pin;
use tokio::{sync::mpsc::{UnboundedReceiver, UnboundedSender},
            task::JoinHandle};

/// Buffer size for reading [`PTY`] output (4KB stack allocation).
///
/// This is used for the read buffer in [`PTY`] operations. The performance bottleneck is
/// not this buffer size but the [`Vec<u8>`] allocations in
/// [`PtyReadWriteOutputEvent::Output`].
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
/// [`Vec<u8>`]: std::vec::Vec
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
/// This enhances readability by making the flow clear: [`crate::PtyCommandBuilder`] `->
/// build() ->` [`PtyCommand`]. This is a validated [`CommandBuilder`] returned by
/// [`crate::PtyCommandBuilder::build()`].
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
pub type PtyControlledChildExitStatus = portable_pty::ExitStatus;

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
pub fn pty_to_std_exit_status(
    status: PtyControlledChildExitStatus,
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

/// Type alias for a pinned completion handle used in [`PTY`] sessions.
///
/// The pinning satisfies Tokio's [`Unpin`] requirement for [`select!`] macro usage. The
/// [`JoinHandle`] returned by [`tokio::spawn()`] doesn't implement [`Unpin`] by default,
/// but [`select!`] requires all futures to be [`Unpin`] for efficient polling without
/// moving them.
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
/// [`select!`]: tokio::select
pub type PtyCompletionHandle =
    Pin<Box<JoinHandle<miette::Result<PtyControlledChildExitStatus>>>>;

/// Type alias for the read-only output event receiver half of a channel.
pub type ReadOnlyOutputEventReceiverHalf = UnboundedReceiver<PtyReadOnlyOutputEvent>;

/// Type alias for the read-write output event receiver half of a channel.
pub type ReadWriteOutputEventReceiverHalf = UnboundedReceiver<PtyReadWriteOutputEvent>;

/// Type alias for the input event sender half of a channel.
pub type InputEventSenderHalf = UnboundedSender<PtyInputEvent>;

#[cfg(test)]
mod tests {
    use super::*;

    /// Compile-time validation that [`PTY`] type aliases are correctly defined.
    ///
    /// This test ensures that the core [`PTY`] type aliases (`Controller`, `Controlled`,
    /// and `ControlledChild`) can be used as function parameters, proving they are
    /// properly defined and usable. If any type alias has incorrect bounds or
    /// missing trait implementations, this test will fail at compile time.
    ///
    /// The functions are marked with `#[allow(dead_code)]` since they are never
    /// called - they only need to compile successfully to validate the type
    /// definitions.
    ///
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    #[test]
    fn validate_pty_type_aliases_compile() {
        // Verify type aliases exist and are correctly defined.
        #[allow(dead_code)]
        fn check_controller(_: Controller) {}
        #[allow(dead_code)]
        fn check_controlled(_: Controlled) {}
        #[allow(dead_code)]
        fn check_controlled_child(_: ControlledChild) {}

        // These are compile-time checks to ensure the types exist.
    }
}
