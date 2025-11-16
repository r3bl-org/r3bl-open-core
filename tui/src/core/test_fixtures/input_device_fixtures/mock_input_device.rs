// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{CrosstermEventResult, InlineVec, InputEvent, PinnedInputStream,
            gen_input_stream, gen_input_stream_with_delay};
use futures_util::{FutureExt, StreamExt};
use std::time::Duration;

/// Mock input device for testing that yields synthetic events from a vector.
///
/// Used by integration tests and unit tests to simulate user input without
/// requiring actual terminal interaction.
///
/// ## Examples
///
/// ```no_run
/// use r3bl_tui::MockInputDevice;
/// use smallvec::smallvec;
/// use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
///
/// let events = smallvec![
///     Ok(Event::Key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE))),
///     Ok(Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))),
/// ];
/// let mut device = MockInputDevice::new(events);
/// ```
pub struct MockInputDevice {
    resource: PinnedInputStream<CrosstermEventResult>,
}

impl std::fmt::Debug for MockInputDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MockInputDevice")
            .field("resource", &"<EventStream>")
            .finish()
    }
}

impl MockInputDevice {
    /// Create a new mock input device that yields events from the given vector.
    #[must_use]
    pub fn new(generator_vec: InlineVec<CrosstermEventResult>) -> Self {
        Self {
            resource: gen_input_stream(generator_vec),
        }
    }

    /// Create a new mock input device with a delay between events.
    ///
    /// Useful for testing timing-sensitive behavior or simulating realistic
    /// user input speed.
    #[must_use]
    pub fn new_with_delay(
        generator_vec: InlineVec<CrosstermEventResult>,
        delay: Duration,
    ) -> Self {
        Self {
            resource: gen_input_stream_with_delay(generator_vec, delay),
        }
    }
}

impl MockInputDevice {
    pub async fn next(&mut self) -> Option<InputEvent> {
        loop {
            let maybe_result_event = self.resource.next().fuse().await;
            match maybe_result_event {
                Some(Ok(event)) => {
                    let input_event = InputEvent::try_from(event);
                    if let Ok(input_event) = input_event {
                        return Some(input_event);
                    }
                    // Conversion errors are expected (filtered events)
                    // Continue reading next event
                }
                Some(Err(e)) => {
                    tracing::error!(
                        message = "Error reading mock input event.",
                        error = ?e,
                    );
                    return None;
                }
                None => return None,
            }
        }
    }
}
