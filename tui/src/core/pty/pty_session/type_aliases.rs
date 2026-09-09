// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::events::{PtyInputEvent, PtyOutputEvent};
use crate::PtyControlledChildExitStatus;
use std::{sync::mpsc::{Receiver, SyncSender},
          thread::JoinHandle};
use tokio::{sync::mpsc::{Receiver as AsyncReceiver, Sender as AsyncSender},
            task::JoinHandle as TokioJoinHandle};

/// Type alias for the orchestrator thread handle used in [`PTY`] sessions.
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
pub type PtyOrchestratorHandle = JoinHandle<miette::Result<PtyControlledChildExitStatus>>;

/// Type alias for the async orchestrator handle.
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
pub type AsyncPtyOrchestratorHandle =
    TokioJoinHandle<miette::Result<PtyControlledChildExitStatus>>;

/// Type alias for an output event receiver half of a synchronous channel.
///
/// Drains events emitted by the reader thread. For details on how backpressure is applied
/// when this receiver is not drained fast enough, see the [Backpressure Architecture].
///
/// [Backpressure Architecture]: crate::core::pty#backpressure-architecture
pub type OutputEventReceiverHalf = Receiver<PtyOutputEvent>;

/// Type alias for an async output event receiver.
pub type AsyncOutputEventReceiverHalf = AsyncReceiver<PtyOutputEvent>;

/// Type alias for an input event sender half of a synchronous channel.
///
/// Uses [`SyncSender`] to provide backpressure when sending input events to the writer
/// thread. For details on how this coordinates with the kernel, see the [Backpressure
/// Architecture].
///
/// [`SyncSender`]: std::sync::mpsc::SyncSender
/// [Backpressure Architecture]: crate::core::pty#backpressure-architecture
pub type InputEventSenderHalf = SyncSender<PtyInputEvent>;

/// Type alias for an async input event sender.
pub type AsyncInputEventSenderHalf = AsyncSender<PtyInputEvent>;
