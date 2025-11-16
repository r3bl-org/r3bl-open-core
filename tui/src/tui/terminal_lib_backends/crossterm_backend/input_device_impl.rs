// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{CrosstermEventResult, DEBUG_TUI_SHOW_TERMINAL_BACKEND, InputEvent,
            PinnedInputStream};
use crossterm::event::EventStream;
use futures_util::{FutureExt, StreamExt};
use miette::IntoDiagnostic;

/// Crossterm-based input device implementation.
///
/// Uses `crossterm::event::EventStream` for async terminal input reading.
pub struct CrosstermInputDevice {
    pub resource: PinnedInputStream<CrosstermEventResult>,
}

impl std::fmt::Debug for CrosstermInputDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CrosstermInputDevice")
            .field("resource", &"<EventStream>")
            .finish()
    }
}

impl CrosstermInputDevice {
    /// Create a new Crossterm input device with an event stream.
    #[must_use]
    pub fn new_event_stream() -> Self {
        Self {
            resource: Box::pin(EventStream::new()),
        }
    }

    /// Get the next raw crossterm event (used internally).
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The input event stream has been closed
    /// - An I/O error occurs while reading input
    /// - The terminal is not available
    async fn next_raw(&mut self) -> miette::Result<crossterm::event::Event> {
        match self.resource.next().fuse().await {
            Some(it) => it.into_diagnostic(),
            None => miette::bail!("Failed to get next event from input source."),
        }
    }
}

impl CrosstermInputDevice {
    pub async fn next(&mut self) -> Option<InputEvent> {
        loop {
            let maybe_result_event = self.next_raw().fuse().await;
            match maybe_result_event {
                Ok(event) => {
                    let input_event = InputEvent::try_from(event);
                    if let Ok(input_event) = input_event {
                        return Some(input_event);
                    }
                    // Conversion errors are expected in the following cases:
                    // 1. Key Release/Repeat events (filtered in InputEvent::try_from).
                    // 2. Paste events (not supported).
                    //
                    // These are normal occurrences, not bugs. We simply continue
                    // reading the next event. The TryFrom implementations handle
                    // all expected cases by returning Err(()), so we don't need
                    // to panic or log errors here.
                }
                Err(e) => {
                    DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                        tracing::error!(
                            message = "Error reading input event.",
                            error = ?e,
                        );
                    });
                    return None;
                }
            }
        }
    }
}
