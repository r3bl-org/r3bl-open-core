// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crossterm::event::EventStream;
use futures_util::{FutureExt, StreamExt};
use miette::IntoDiagnostic;

use crate::{CrosstermEventResult, PinnedInputStream};

/// This struct represents an input device that can be used to read from the terminal. See
/// [`crate::InputDeviceExt`] for testing features.
#[allow(missing_debug_implementations)]
pub struct InputDevice {
    pub resource: PinnedInputStream<CrosstermEventResult>,
}

impl InputDevice {
    #[must_use]
    pub fn new_event_stream() -> InputDevice {
        InputDevice {
            resource: Box::pin(EventStream::new()),
        }
    }
}

impl InputDevice {
    /// # Errors
    ///
    /// Returns an error if:
    /// - The input event stream has been closed
    /// - An I/O error occurs while reading input
    /// - The terminal is not available
    pub async fn next(&mut self) -> miette::Result<crossterm::event::Event> {
        match self.resource.next().fuse().await {
            Some(it) => it.into_diagnostic(),
            None => miette::bail!("Failed to get next event from input source."),
        }
    }
}
