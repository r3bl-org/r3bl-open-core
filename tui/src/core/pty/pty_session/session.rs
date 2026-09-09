// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::type_aliases::{AsyncInputEventSenderHalf, AsyncOutputEventReceiverHalf,
                          AsyncPtyOrchestratorHandle, InputEventSenderHalf,
                          OutputEventReceiverHalf, PtyOrchestratorHandle};
use crate::ControlledChildTerminationHandle;

/// Handle for a [`PTY`] session.
///
/// This is returned by [`PtySessionBuilder::start()`].
///
/// Holds the synchronous communication channels and lifecycle handles for interacting
/// with the background threads running the child process.
///
/// For asynchronous applications that require a [`tokio::select!`] loop, use
/// [`PtySessionBuilder::start_async()`] instead, which returns an [`AsyncPtySession`].
/// See the [Session Layer] documentation for details.
///
/// # Examples
///
/// Draining output events and awaiting child process completion synchronously:
///
/// ```
/// # #[cfg(not(unix))]
/// # fn main() {}
/// # #[cfg(unix)]
/// use r3bl_tui::{ok, PtyOutputEvent, PtySessionBuilder};
///
/// # #[cfg(unix)]
/// fn main() -> miette::Result<()> {
///     let mut session = PtySessionBuilder::new("echo")
///         .cli_arg("hello")
///         .start()?;
///
///     while let Ok(event) = session.rx_output_event.recv() {
///         match event {
///             PtyOutputEvent::Output(_bytes) => { /* render bytes */ }
///             PtyOutputEvent::Exit(_status) => { break; }
///             _ => {}
///         }
///     }
///
///     let _status = session.orchestrator_task_handle.join();
///     ok!()
/// }
/// ```
///
/// [`AsyncPtySession`]: crate::AsyncPtySession
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
/// [`PtyInputEvent`]: crate::PtyInputEvent
/// [`PtyOutputEvent`]: crate::PtyOutputEvent
/// [`PtySessionBuilder::start()`]: crate::PtySessionBuilder::start
/// [`PtySessionBuilder::start_async()`]: crate::PtySessionBuilder::start_async
/// [`tokio::select!`]: tokio::select
/// [Session Layer]: mod@crate::pty_session
#[derive(Debug)]
pub struct PtySession {
    /// Send [`PtyInputEvent`] events to the child process.
    ///
    /// [`PtyInputEvent`]: crate::PtyInputEvent
    pub tx_input_event: InputEventSenderHalf,

    /// Receive [`PtyOutputEvent`] events from the child process.
    ///
    /// [`PtyOutputEvent`]: crate::PtyInputEvent
    pub rx_output_event: OutputEventReceiverHalf,

    /// Handle to join spawned process orchestration and completion. Returns the final
    /// exit status.
    pub orchestrator_task_handle: PtyOrchestratorHandle,

    /// Handle to explicitly terminate the child process if needed.
    pub child_process_termination_handle: ControlledChildTerminationHandle,
}

/// Handle for an asynchronous [`PTY`] session.
///
/// This is returned by [`PtySessionBuilder::start_async()`].
///
/// Holds the asynchronous communication channels and lifecycle handles for interacting
/// with the background threads running the child process. It is designed for use in
/// [`tokio::select!`] loops.
///
/// # Examples
///
/// Driving an asynchronous session with a [`tokio::select!`] loop:
///
/// ```
/// # #[cfg(not(unix))]
/// # fn main() {}
/// # #[cfg(unix)]
/// use r3bl_tui::{ok, PtyOutputEvent, PtySessionBuilder};
///
/// # #[cfg(unix)]
/// #[tokio::main]
/// # #[cfg(unix)]
/// async fn main() -> miette::Result<()> {
///     let mut session = PtySessionBuilder::new("echo")
///         .cli_arg("hello")
///         .start_async()?;
///
///     loop {
///         tokio::select! {
///             // 1. Handle output from the PTY.
///             Some(event) = session.rx_output_event.recv() => {
///                 match event {
///                     PtyOutputEvent::Output(_bytes) => { /* render bytes */ }
///                     PtyOutputEvent::Exit(_status) => { break; }
///                     _ => {}
///                 }
///             }
///             // 2. Await process orchestration and completion.
///             _status = &mut session.orchestrator_task_handle => {
///                 break;
///             }
///         }
///     }
///     ok!()
/// }
/// ```
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
/// [`PtyInputEvent`]: crate::PtyInputEvent
/// [`PtyOutputEvent`]: crate::PtyOutputEvent
/// [`PtySessionBuilder::start_async()`]: crate::PtySessionBuilder::start_async
/// [`tokio::select!`]: tokio::select
#[derive(Debug)]
pub struct AsyncPtySession {
    /// Send [`PtyInputEvent`] events to the child process.
    ///
    /// [`PtyInputEvent`]: crate::PtyInputEvent
    pub tx_input_event: AsyncInputEventSenderHalf,

    /// Receive [`PtyOutputEvent`] events from the child process.
    ///
    /// [`PtyOutputEvent`]: crate::PtyOutputEvent
    pub rx_output_event: AsyncOutputEventReceiverHalf,

    /// Handle to await spawned process orchestration and completion. Returns the final
    /// exit status.
    pub orchestrator_task_handle: AsyncPtyOrchestratorHandle,

    /// Handle to explicitly terminate the child process if needed.
    pub child_process_termination_handle: ControlledChildTerminationHandle,
}
