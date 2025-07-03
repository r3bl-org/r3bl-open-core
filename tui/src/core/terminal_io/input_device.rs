/*
 *   Copyright (c) 2024-2025 R3BL LLC
 *   All rights reserved.
 *
 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
 */

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
    pub async fn next(&mut self) -> miette::Result<crossterm::event::Event> {
        match self.resource.next().fuse().await {
            Some(it) => it.into_diagnostic(),
            None => miette::bail!("Failed to get next event from input source."),
        }
    }
}
