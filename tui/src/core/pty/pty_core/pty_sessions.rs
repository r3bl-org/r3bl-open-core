// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`PTY`] session handle types for read-only and read-write communication.
//!
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal

use super::pty_types::{InputEventSenderHalf, PtyCompletionHandle,
                       ReadOnlyOutputEventReceiverHalf, ReadWriteOutputEventReceiverHalf};
use crate::ControlledChildTerminationHandle;
use notify_rust::Notification;

/// Default notification display duration.
pub const NOTIFICATION_TIMEOUT_MS: u32 = 1_000;

/// Shows a desktop notification with error handling.
///
/// This helper function simplifies showing notifications throughout the PTY multiplexer
/// by handling the verbose notification setup and error handling in a single place. Uses
/// a default timeout of [`NOTIFICATION_TIMEOUT_MS`] for all notifications.
///
/// # Arguments
/// * `title` - The notification title/summary
/// * `message` - The notification body message
pub fn show_notification(title: &str, message: &str) {
    if let Err(e) = Notification::new()
        .summary(title)
        .body(message)
        .timeout(notify_rust::Timeout::Milliseconds(NOTIFICATION_TIMEOUT_MS))
        .show()
    {
        tracing::warn!("Failed to show notification '{}': {}", title, e);
    }
}

/// A unidirectional [`PTY`] session handle for monitoring child process output.
///
/// - Receives combined stdout/stderr, [`OSC`] sequences, and process lifecycle events
///   without input capability
/// - Used for monitoring long-running processes, capturing command output, or observing
///   terminal applications
///
/// [`OSC`]: crate::OscEvent
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
#[derive(Debug)]
pub struct PtyReadOnlySession {
    /// Receives output events from the child process (combined stdout/stderr).
    pub output_evt_ch_rx_half: ReadOnlyOutputEventReceiverHalf,
    /// Await this `completion_handle` for process completion.
    ///
    /// Pinned to satisfy Tokio's Unpin requirement for select! macro usage in tests and
    /// other async coordination patterns. The `JoinHandle` returned by `tokio::spawn`
    /// doesn't implement Unpin by default, but select! requires all futures to be
    /// Unpin for efficient polling without moving them.
    pub pinned_boxed_session_completion_handle: PtyCompletionHandle,
}

/// A bidirectional [`PTY`] session handle for full interaction with child processes.
///
/// - Sends keyboard input, control sequences, and window resizing via [`PtyInputEvent`]
/// - Receives stdout/stderr output via [`PtyReadWriteOutputEvent`]
/// - Used for interactive terminal applications, REPLs, shell sessions, and automated
///   command execution
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
/// [`PtyInputEvent`]: crate::PtyInputEvent
/// [`PtyReadWriteOutputEvent`]: crate::PtyReadWriteOutputEvent
#[derive(Debug)]
pub struct PtyReadWriteSession {
    /// Sends input TO the child process.
    pub input_event_ch_tx_half: InputEventSenderHalf,

    /// Receive output FROM the child process (combined stdout/stderr).
    pub output_event_receiver_half: ReadWriteOutputEventReceiverHalf,

    /// Await this `completion_handle` for process completion.
    ///
    /// Pinned to satisfy Tokio's Unpin requirement for select! macro usage in tests and
    /// other async coordination patterns. The `JoinHandle` returned by `tokio::spawn`
    /// doesn't implement Unpin by default, but select! requires all futures to be
    /// Unpin for efficient polling without moving them.
    pub pinned_boxed_session_completion_handle: PtyCompletionHandle,

    /// Handle to forcefully terminate the child process.
    ///
    /// This handle is cloned from the spawned child process using
    /// [`portable_pty::ChildKiller::clone_killer()`] and provides the ability to send
    /// SIGTERM/SIGKILL signals to terminate the child process from outside the
    /// completion task.
    ///
    /// **Critical for clean shutdown**: Unlike [`crate::PtyInputEvent::Close`] which
    /// only stops the input writer and sends EOF, calling `kill()` on this handle
    /// will forcefully terminate the child process, allowing `child.wait()` in the
    /// completion task to return immediately.
    ///
    /// # Usage Patterns
    ///
    /// **For immediate termination (recommended for shutdown):**
    /// - Call `kill()` on this handle to forcefully terminate the child process
    /// - Send `PtyInputEvent::Close` to stop the input writer
    /// - This ensures clean shutdown without waiting for the child to respond
    ///
    /// **For graceful termination (may hang if child doesn't respond):**
    /// - Send only `PtyInputEvent::Close` to send EOF to the child
    /// - Wait for the child to exit naturally
    /// - Use this approach if the child process needs time to clean up resources
    ///
    /// # See Also
    /// - [`crate::PtyInputEvent::Close`] for input writer termination only
    /// - [`portable_pty::ChildKiller::kill()`] for the underlying kill method
    pub child_process_terminate_handle: ControlledChildTerminationHandle,
}
