// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Core type aliases and constants for PTY operations.

use super::{PtyInputEvent, PtyReadOnlyOutputEvent, PtyReadWriteOutputEvent};
use portable_pty::{ChildKiller, CommandBuilder, MasterPty, SlavePty};
use std::pin::Pin;
use tokio::{sync::mpsc::{UnboundedReceiver, UnboundedSender},
            task::JoinHandle};

/// Buffer size for reading PTY output (4KB stack allocation).
///
/// This is used for the read buffer in PTY operations. The performance bottleneck is not
/// this buffer size but the [`Vec`]`<`[`u8`]`>` allocations in
/// [`PtyReadWriteOutputEvent::Output`].
pub const READ_BUFFER_SIZE: usize = 4096;

/// Wrapper around [`portable_pty::PtyPair`] that provides controller/controlled
/// terminology.
///
/// This type intentionally shadows [`portable_pty::PtyPair`] to prevent direct use of
/// [`portable_pty`]'s master/slave terminology. It provides clean accessor methods that
/// align with our codebase's inclusive language policy.
///
/// See: [Inclusive Naming Initiative - Tier 1 Terms](https://inclusivenaming.org/word-lists/tier-1/)
///
/// # Example
///
/// ```no_run
/// use r3bl_tui::PtyPair;
/// use portable_pty::{PtySystem, NativePtySystem, PtySize};
///
/// let pty_system = NativePtySystem::default();
/// let raw_pair = pty_system.openpty(PtySize::default()).unwrap();
/// let pty_pair = PtyPair::from(raw_pair);
///
/// // Access controller side (library's "master")
/// let reader = pty_pair.controller().try_clone_reader().unwrap();
///
/// // Access controlled side (library's "slave")
/// // (typically used for spawning child processes)
/// ```
#[allow(missing_debug_implementations)]
pub struct PtyPair {
    inner: portable_pty::PtyPair,
}

impl PtyPair {
    /// Create a new wrapper from a raw `portable_pty::PtyPair`.
    #[must_use]
    pub fn new(inner: portable_pty::PtyPair) -> Self { Self { inner } }

    /// Access the controller side of the PTY (library's "master").
    ///
    /// The controller side is used by the parent process to read output from
    /// and write input to the controlled child process.
    #[must_use]
    pub fn controller(&self) -> &Controller { &self.inner.master }

    /// Access the controller side mutably.
    pub fn controller_mut(&mut self) -> &mut Controller { &mut self.inner.master }

    /// Access the controlled side of the PTY (library's "slave").
    ///
    /// The controlled side is typically used for spawning child processes that
    /// will use this PTY for their stdin/stdout/stderr.
    #[must_use]
    pub fn controlled(&self) -> &Controlled { &self.inner.slave }

    /// Access the controlled side mutably.
    pub fn controlled_mut(&mut self) -> &mut Controlled { &mut self.inner.slave }

    /// Split the pair into separate controller and controlled halves.
    ///
    /// This consumes the wrapper and returns the individual components.
    #[must_use]
    pub fn split(self) -> (Controller, Controlled) {
        (self.inner.master, self.inner.slave)
    }

    /// Get the inner `portable_pty::PtyPair` for direct library access.
    ///
    /// This is provided for cases where you need to interact directly with the
    /// `portable_pty` API, but should be used sparingly.
    #[must_use]
    pub fn into_inner(self) -> portable_pty::PtyPair { self.inner }
}

impl From<portable_pty::PtyPair> for PtyPair {
    fn from(inner: portable_pty::PtyPair) -> Self { Self::new(inner) }
}

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

/// Type alias for the writer used in PTY operations.
pub type ControllerWriter = Box<dyn std::io::Write + Send>;

/// Type alias for the reader used in PTY operations.
pub type ControllerReader = Box<dyn std::io::Read + Send>;

/// Type alias for a controlled child termination handle.
pub type ControlledChildTerminationHandle = Box<dyn ChildKiller + Send + Sync>;

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

/// Type alias for the read-only output event receiver half of a channel.
pub type ReadOnlyOutputEventReceiverHalf = UnboundedReceiver<PtyReadOnlyOutputEvent>;

/// Type alias for the read-write output event receiver half of a channel.
pub type ReadWriteOutputEventReceiverHalf = UnboundedReceiver<PtyReadWriteOutputEvent>;

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
