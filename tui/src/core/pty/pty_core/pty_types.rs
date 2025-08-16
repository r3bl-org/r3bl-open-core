// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Core type aliases and constants for PTY operations.

use std::pin::Pin;

use portable_pty::{CommandBuilder, MasterPty, SlavePty};
use tokio::{sync::mpsc::{UnboundedReceiver, UnboundedSender},
            task::JoinHandle};

use super::{PtyInputEvent, PtyOutputEvent};

/// Buffer size for reading PTY output (4KB stack allocation).
///
/// This is used for the read buffer in PTY operations. The performance bottleneck
/// is not this buffer size but the `Vec<u8>` allocations in `PtyOutputEvent::Output`.
pub const READ_BUFFER_SIZE: usize = 4096;

/// Type alias for the controlled half of a PTY (slave).
///
/// This represents the process-side of the PTY that the child process
/// will use for stdin/stdout/stderr.
pub type Controlled = Box<dyn SlavePty + Send>;

/// Type alias for the controller half of a PTY (master).
///
/// This represents the controller half that the parent process uses
/// to read from and write to the child process.
pub type Controller = Box<dyn MasterPty + Send>;

/// Type alias for a spawned child process in a PTY.
pub type ControlledChild = Box<dyn portable_pty::Child + Send + Sync>;

/// Type alias for a validated PTY command ready for execution.
///
/// This enhances readability by making the flow clear: [`crate::PtyCommandBuilder`] `->
/// build() ->` [`PtyCommand`]. This is a validated [`CommandBuilder`] returned by
/// [`crate::PtyCommandBuilder::build`].
pub type PtyCommand = CommandBuilder;

/// Type alias for a pinned completion handle used in PTY sessions.
///
/// The pinning satisfies Tokio's [`Unpin`] requirement for [`select!`] macro usage. The
/// [`JoinHandle`] returned by [`tokio::spawn`] doesn't implement [`Unpin`] by default,
/// but [`select!`] requires all futures to be [`Unpin`] for efficient polling without
/// moving them.
///
/// [`select!`]: tokio::select
pub type PtyCompletionHandle =
    Pin<Box<JoinHandle<miette::Result<portable_pty::ExitStatus>>>>;

/// Type alias for the output event receiver half of a channel.
pub type OutputEventReceiverHalf = UnboundedReceiver<PtyOutputEvent>;

/// Type alias for the input event sender half of a channel.
pub type InputEventSenderHalf = UnboundedSender<PtyInputEvent>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_buffer_size_constant() {
        assert_eq!(READ_BUFFER_SIZE, 4096);
    }

    /// Compile-time validation that PTY type aliases are correctly defined.
    ///
    /// This test ensures that the core PTY type aliases (`Controller`, `Controlled`,
    /// and `ControlledChild`) can be used as function parameters, proving they are
    /// properly defined and usable. If any type alias has incorrect bounds or
    /// missing trait implementations, this test will fail at compile time.
    ///
    /// The functions are marked with `#[allow(dead_code)]` since they are never
    /// called - they only need to compile successfully to validate the type
    /// definitions.
    #[test]
    fn validate_pty_type_aliases_compile() {
        // Verify type aliases exist and are correctly defined
        #[allow(dead_code)]
        fn check_controller(_: Controller) {}
        #[allow(dead_code)]
        fn check_controlled(_: Controlled) {}
        #[allow(dead_code)]
        fn check_controlled_child(_: ControlledChild) {}

        // These are compile-time checks to ensure the types exist
    }
}
