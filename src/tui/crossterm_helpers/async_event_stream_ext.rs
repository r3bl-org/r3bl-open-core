/*
 *   Copyright (c) 2022 R3BL LLC
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

//! Using `poll()` is inefficient. The following code will generate some CPU utilization while
//! idling.
//!
//! ```ignore
//! loop {
//!   if poll(Duration::from_millis(500))? { // This is inefficient.
//!     let input_event: InputEvent = read()?.into();
//!     if handle_input_event(input_event).await.is_err() {
//!       break;
//!     };
//!   }
//! }
//! ```
//[! The following code blocks the thread that its running on.
//!
//! ```ignore
//! async fn repl_blocking() -> CommonResult<()> {
//!   throws!({
//!     println_raw!("Type Ctrl+q to exit repl.");
//!     loop {
//!       let input_event: InputEvent = read()?.into();
//!       let result = handle_input_event(input_event).await;
//!       if result.is_err() {
//!         break;
//!       };
//!     }
//!   });
//! }
//! ```
//!
//! Docs:
//! - <https://github.com/crossterm-rs/crossterm/wiki/Upgrade-from-0.13-to-0.14#115-event-polling>
//! - <https://github.com/crossterm-rs/crossterm/wiki/Upgrade-from-0.13-to-0.14#111-new-event-api>
//! - <https://github.com/crossterm-rs/crossterm/blob/master/examples/event-stream-tokio.rs>

use async_trait::async_trait;
use crossterm::event::*;
use futures_util::{FutureExt, StreamExt};

use crate::*;

pub struct AsyncEventStream {
  event_stream: EventStream,
}

impl AsyncEventStream {
  pub async fn try_to_get_input_event(&mut self) -> Option<InputEvent> {
    self.event_stream.try_to_get_input_event().await
  }
}

impl Default for AsyncEventStream {
  fn default() -> Self {
    Self {
      event_stream: EventStream::new(),
    }
  }
}

#[async_trait]
pub trait EventStreamExt {
  /// Try and read an [Event] from the [EventStream], and convert it into an [InputEvent]. This is a
  /// non-blocking call. It returns an [InputEvent] wrapped in a [Option]. [None] is returned if
  /// there was an error.
  async fn try_to_get_input_event(&mut self) -> Option<InputEvent>;
}

#[async_trait]
impl EventStreamExt for EventStream {
  async fn try_to_get_input_event(&mut self) -> Option<InputEvent> {
    let maybe_event = self.next().fuse().await;
    match maybe_event {
      Some(Ok(event)) => {
        let input_event: Result<InputEvent, ()> = event.try_into();
        match input_event {
          Ok(input_event) => Some(input_event),
          Err(e) => {
            call_if_debug_true!(log_no_err!(ERROR, "Error: {:?}", e));
            None
          }
        }
      }
      Some(Err(e)) => {
        call_if_debug_true!(log_no_err!(ERROR, "Error: {:?}", e));
        None
      }
      _ => None,
    }
  }
}
