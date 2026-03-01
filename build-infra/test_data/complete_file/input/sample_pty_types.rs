// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words CLOEXEC

//! Core type aliases and constants for [`PTY`] operations. See individual type
//! definitions for details:
//! - [`PtyPair`] - Inclusive-language wrapper around [`portable_pty::PtyPair`]
//! - [`Controller`], [`Controlled`] - [`PTY`] halves
//! - [`ControlledChild`], [`ControlledChildTerminationHandle`] - Child process management
//! - [`ControllerReader`], [`ControllerWriter`] - [`PTY`] I/O streams
//! - [`PtyCommand`], [`PtyCompletionHandle`] - Command execution
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
/// not this buffer size but the [`Vec`]`<`[`u8`]`>` allocations in
/// [`PtyReadWriteOutputEvent::Output`].
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
pub const READ_BUFFER_SIZE: usize = 4_096;

/// Owns both halves of a [`PTY`] pair and manages the controlled side's lifecycle.
///
/// # [`PTY`] Primer
///
/// In [`PTY`] terminology, the **parent process** is your Rust application (the one that
/// creates the [`PTY`] and calls [`spawn_command()`]), and the **child process** is the
/// program spawned inside the [`PTY`] (e.g. the [`top`] binary or another Rust
/// application binary).
///
/// The parent process uses the **controller** side to read and write to the child
/// process. It writes to the child process's input, and reads from the child process's
/// output.
///
/// The child process gets the **controlled** side connected to its standard I/O streams.
/// When [`spawn_command()`] creates the child process, it redirects each of its streams
/// to the controlled [`fd`]. Here's how the streams are connected:
///
/// | Stream     | [`fd`] | Direction                                                |
/// | :--------- | :----- | :------------------------------------------------------- |
/// | [`stdin`]  | `0`    | Child process reads its input from the controlled [`fd`] |
/// | [`stdout`] | `1`    | Child process writes its output to the controlled [`fd`] |
/// | [`stderr`] | `2`    | Child process writes its errors to the controlled [`fd`] |
///
/// From the child process's perspective, it is talking to a real terminal - it has no
/// idea it is inside a [`PTY`].
///
/// # What this struct does
///
/// ## Inclusive terminology
/// Replaces [`portable_pty::PtyPair`]'s master/slave naming with controller/controlled,
/// per [Inclusive Naming Initiative - Tier 1 Terms].
///
/// ## Controlled side lifecycle
/// The two halves have different lifetimes:
/// - The **controller** must stay alive while the child process is running so the parent
///   process can read from and write to it.
/// - The **controlled** side must be closed immediately after spawning the child process
///   (see [`close_controlled`]).
///
/// **Why closing matters:** as long as the parent process holds the controlled [`fd`]
/// open, the kernel will not deliver [`EOF`] to the controller reader -- even after the
/// child process exits. This causes permanent deadlocks in any read loop waiting for
/// child output to end.
///
/// **Why this struct exists:** [`portable_pty::PtyPair`] has no API for closing the
/// controlled side independently -- its two `pub` fields can only be dropped together.
/// This struct destructures them into separate fields with an [`Option`] on the
/// controlled side so [`close_controlled`] can drop it while the controller stays alive.
///
/// # File Descriptor Ownership
///
/// Understanding which process holds which [`fd`] is essential for reasoning about
/// [`EOF`] delivery and deadlocks. The actual kernel file descriptors live deep inside
/// [`portable_pty`]'s trait objects:
///
/// ```text
/// PtyPair
///   ├── controller: Controller             (Box<dyn MasterPty + Send>)
///   │     └── UnixMasterPty                (portable_pty internal)
///   │           └── fd: PtyFd → RawFd      ← kernel controller fd
///   │
///   └── maybe_controlled: Option<Controlled>  (Box<dyn SlavePty + Send>)
///         └── UnixSlavePty                 (portable_pty internal)
///               └── fd: PtyFd → RawFd      ← kernel controlled fd
/// ```
///
/// When a [`Controlled`] value is dropped, the `Drop` chain propagates through `Box<dyn
/// SlavePty>` → `UnixSlavePty` → `PtyFd` → `FileDescriptor` → `close(`[`fd`]`)`, closing
/// the kernel [`fd`]. This is why [`close_controlled`] works: it calls `Option::take()`
/// to move the [`Controlled`] out, then drops it.
///
/// Both [`fd`]s are marked [`FD_CLOEXEC`] by [`portable_pty`], so the child process does
/// not inherit the parent's copies on `exec()`. The child only gets the **new**
/// controlled [`fd`]s that [`spawn_command()`] explicitly creates for
/// stdin/stdout/stderr.
///
/// # Controlled Side Lifecycle
///
/// The controlled side [`fd`] must be closed in the parent process promptly after
/// spawning the child. As long as the parent holds the controlled [`fd`] open, reading
/// from the controller side will never deliver `EOF` even after the child exits. This
/// causes permanent deadlocks in read loops that wait for child output.
///
/// Use [`close_controlled`] immediately after [`spawn_command()`] to prevent this:
///
/// ```no_run
/// use r3bl_tui::PtyPair;
/// use portable_pty::{PtySystem, NativePtySystem, PtySize, CommandBuilder};
///
/// let pty_system = NativePtySystem::default();
/// let raw_pair = pty_system.openpty(PtySize::default()).unwrap();
/// let mut pty_pair = PtyPair::from(raw_pair);
///
/// // Spawn child using the controlled side.
/// let _child = pty_pair.controlled().spawn_command(CommandBuilder::new("cat")).unwrap();
///
/// // Close the controlled side immediately - parent no longer needs it.
/// // Without this, reads from the controller will never see EOF after the child exits,
/// // causing deadlocks in any loop that waits for child output to end.
/// pty_pair.close_controlled();
///
/// // Now reads from the controller will return EOF once the child exits.
/// let reader = pty_pair.controller().try_clone_reader().unwrap();
/// ```
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
/// // Access controller side.
/// let reader = pty_pair.controller().try_clone_reader().unwrap();
///
/// // Access controlled side (typically used for spawning child processes).
/// ```
///
/// [`FD_CLOEXEC`]: https://man7.org/linux/man-pages/man2/fcntl.2.html
/// [`close_controlled`]: PtyPair::close_controlled
/// [`EOF`]: https://en.wikipedia.org/wiki/End-of-file
/// [`fd`]: https://man7.org/linux/man-pages/man2/open.2.html
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
/// [`spawn_command()`]: portable_pty::SlavePty::spawn_command
/// [`stderr`]: std::io::stderr
/// [`stdin`]: std::io::stdin
/// [`stdout`]: std::io::stdout
/// [`top`]: https://man7.org/linux/man-pages/man1/top.1.html
/// [Inclusive Naming Initiative - Tier 1 Terms]:
///     https://inclusivenaming.org/word-lists/tier-1/
#[allow(missing_debug_implementations)]
pub struct PtyPair {
    /// Controller side of the [`PTY`].
    ///
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    pub controller: Controller,

    /// Controlled side of the [`PTY`], held as `Option` so it can be closed early.
    ///
    /// After the child process is spawned, the parent no longer needs the controlled
    /// [`fd`]. Keeping it open prevents [`EOF`] from being delivered to controller
    /// readers after the child exits. Call [`close_controlled`] to drop it
    /// immediately after spawning.
    ///
    /// [`close_controlled`]: PtyPair::close_controlled
    /// [`EOF`]: https://en.wikipedia.org/wiki/End-of-file
    /// [`fd`]: https://man7.org/linux/man-pages/man2/open.2.html
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    pub maybe_controlled: Option<Controlled>,
}

impl PtyPair {
    /// Creates a new wrapper from a raw `portable_pty::PtyPair`.
    #[must_use]
    pub fn new(inner: portable_pty::PtyPair) -> Self {
        Self {
            controller: inner.master,
            maybe_controlled: Some(inner.slave),
        }
    }

    /// Access the controller side of the [`PTY`].
    ///
    /// The controller side is used by the parent process to read output from
    /// and write input to the controlled child process.
    ///
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    #[must_use]
    pub fn controller(&self) -> &Controller { &self.controller }

    /// Access the controller side mutably.
    pub fn controller_mut(&mut self) -> &mut Controller { &mut self.controller }

    /// Access the controlled side of the [`PTY`].
    ///
    /// The controlled side is typically used for spawning child processes that
    /// will use this [`PTY`] for their stdin/stdout/stderr.
    ///
    /// # Panics
    ///
    /// Panics if [`close_controlled`] has already been called.
    ///
    /// [`close_controlled`]: PtyPair::close_controlled
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    #[must_use]
    pub fn controlled(&self) -> &Controlled {
        self.maybe_controlled
            .as_ref()
            .expect("controlled side already closed via close_controlled()")
    }

    /// Access the controlled side mutably.
    ///
    /// # Panics
    ///
    /// Panics if [`close_controlled`] has already been called.
    ///
    /// [`close_controlled`]: PtyPair::close_controlled
    pub fn controlled_mut(&mut self) -> &mut Controlled {
        self.maybe_controlled
            .as_mut()
            .expect("controlled side already closed via close_controlled()")
    }

    /// Closes the controlled side of the [`PTY`] in the parent process.
    ///
    /// Call this immediately after spawning the child process. As long as the parent
    /// holds the controlled [`fd`] open, reads from the controller side will never
    /// deliver [`EOF`] even after the child exits. This causes permanent deadlocks in
    /// any read loop that waits for child output to end.
    ///
    /// After this call, [`controlled`] and [`controlled_mut`] will panic. The
    /// [`controller`] and [`controller_mut`] sides remain unaffected.
    ///
    /// [`controlled_mut`]: PtyPair::controlled_mut
    /// [`controlled`]: PtyPair::controlled
    /// [`controller_mut`]: PtyPair::controller_mut
    /// [`controller`]: PtyPair::controller
    /// [`EOF`]: https://en.wikipedia.org/wiki/End-of-file
    /// [`fd`]: https://man7.org/linux/man-pages/man2/open.2.html
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    pub fn close_controlled(&mut self) { drop(self.maybe_controlled.take()); }

    /// Split the pair into separate controller and controlled halves.
    ///
    /// This consumes the wrapper and returns the individual components.
    ///
    /// # Panics
    ///
    /// Panics if [`close_controlled`] has already been called.
    ///
    /// [`close_controlled`]: PtyPair::close_controlled
    #[must_use]
    pub fn split(self) -> (Controller, Controlled) {
        let controlled = self
            .maybe_controlled
            .expect("controlled side already closed via close_controlled()");
        (self.controller, controlled)
    }
}

impl From<portable_pty::PtyPair> for PtyPair {
    fn from(inner: portable_pty::PtyPair) -> Self { Self::new(inner) }
}

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
/// [`crate::PtyCommandBuilder::build`].
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
pub type PtyCommand = CommandBuilder;

/// Type alias for a pinned completion handle used in [`PTY`] sessions.
///
/// The pinning satisfies Tokio's [`Unpin`] requirement for [`select!`] macro usage. The
/// [`JoinHandle`] returned by [`tokio::spawn`] doesn't implement [`Unpin`] by default,
/// but [`select!`] requires all futures to be [`Unpin`] for efficient polling without
/// moving them.
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
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
