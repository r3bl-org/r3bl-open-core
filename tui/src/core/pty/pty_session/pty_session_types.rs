// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{PtyControlledChildExitStatus, PtyInputEvent, PtyOutputEvent};
use tokio::{sync::mpsc::{Receiver, Sender},
            task::JoinHandle};

/// Type alias for the orchestrator handle used in [`PTY`] sessions.
///
/// [`JoinHandle`] is already [`Unpin`], so no [`Pin`] wrapper is needed for use in
/// [`select!`] branches. See [Core Async Concepts] for more details.
///
/// [`Pin`]: std::pin::Pin
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
/// [`select!`]: tokio::select
/// [Core Async Concepts]: crate::main_event_loop_impl#core-async-concepts-pin-and-unpin
pub type PtyOrchestratorHandle = JoinHandle<miette::Result<PtyControlledChildExitStatus>>;

/// Type alias for an output event receiver half of a channel.
pub type OutputEventReceiverHalf = Receiver<PtyOutputEvent>;

/// Type alias for an input event sender half of a channel.
pub type InputEventSenderHalf = Sender<PtyInputEvent>;

/// Whether to capture a particular data stream from the [`PTY`].
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureFlag {
    Capture,
    NoCapture,
}

/// Whether to detect terminal cursor mode changes from the [`PTY`].
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectFlag {
    Detect,
    NoDetect,
}
