// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! PTY session handle types for read-only and read-write communication.

use super::pty_types::{InputEventSenderHalf, PtyCompletionHandle,
                       ReadOnlyOutputEventReceiverHalf, ReadWriteOutputEventReceiverHalf};
use crate::ControlledChildTerminationHandle;
use notify_rust::Notification;

/// Show a desktop notification with error handling.
///
/// This helper function simplifies showing notifications throughout the PTY multiplexer
/// by handling the verbose notification setup and error handling in a single place.
/// Uses a default timeout of 2 seconds for all notifications.
///
/// # Arguments
/// * `title` - The notification title/summary
/// * `message` - The notification body message
///
/// # Example
/// ```ignore
/// show_notification("PTY Mux - Input Event", &format!("Input received: {event:?}"));
/// ```
pub fn show_notification(title: &str, message: &str) {
    if let Err(e) = Notification::new()
        .summary(title)
        .body(message)
        .timeout(notify_rust::Timeout::Milliseconds(1000))
        .show()
    {
        tracing::warn!("Failed to show notification '{}': {}", title, e);
    }
}

/// Session handle for read-only PTY communication.
///
/// # Summary
/// - Unidirectional PTY session for monitoring child process output without input
///   capability
/// - Components: `output_event_receiver_half` (event stream), `completion_handle` (exit
///   status)
/// - Receives combined stdout/stderr, OSC sequences, and process lifecycle events
/// - Used for monitoring long-running processes, capturing command output, or observing
///   terminal applications
/// - Integrates with Tokio async runtime via pinned [`PtyCompletionHandle`] for efficient
///   polling in `select!` macros
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

/// Session handle for read-write PTY communication.
///
/// # Summary
/// - Bidirectional PTY session for full interaction with child processes
/// - Components: `input_event_sender_half` (send input), `output_event_receiver_half`
///   (receive output), `completion_handle` (exit status)
/// - Supports sending keyboard input, control sequences, window resizing, and receiving
///   stdout/stderr output
/// - Used for interactive terminal applications, REPLs, shell sessions, and automated
///   command execution
/// - Integrates with [`super::pty_input_events::PtyInputEvent`] for input and
///   [`super::pty_output_events::PtyReadWriteOutputEvent`] for output handling
#[derive(Debug)]
pub struct PtyReadWriteSession {
    /// Send input TO the child process.
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
    /// # Usage Pattern
    /// ```ignore
    /// // For immediate termination (recommended for shutdown):
    /// session.child_process_terminate_handle.kill()?;  // Kill child process
    /// session.input_event_ch_tx_half.send(PtyInputEvent::Close)?;  // Stop input writer
    ///
    /// // For graceful termination (may hang if child doesn't respond to EOF):
    /// session.input_event_ch_tx_half.send(PtyInputEvent::Close)?;  // Send EOF only
    /// ```
    ///
    /// # See Also
    /// - [`crate::PtyInputEvent::Close`] for input writer termination only
    /// - [`portable_pty::ChildKiller::kill()`] for the underlying kill method
    pub child_process_terminate_handle: ControlledChildTerminationHandle,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pty_session_structs_debug() {
        // Test that the session structs have Debug implemented.
        // We can't easily test the actual structs without spawning processes,
        // but we can verify the types exist and have the expected fields.

        // These will be compile-time checks.
        fn check_debug<T: std::fmt::Debug>() {}

        check_debug::<PtyReadOnlySession>();
        check_debug::<PtyReadWriteSession>();
    }
}
