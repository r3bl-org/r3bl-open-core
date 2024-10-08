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
//!
//! The following code blocks the thread that its running on.
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
//! - tokio crate docs:
//!     - [Async in depth, futures, polling, efficient wakers](https://tokio.rs/tokio/tutorial/async)
//!     - [Example of delay (setTimeout) w/out waker, inefficient](https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=7a43cdf047cc17047c3c8b3f137293f0)
//!     - [Example of delay (setTimeout) w/ waker, efficient](https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=6633717db032ffe7af809e9131d7462f)
//!     - [Per task concurrency w/ `select!`, vs parallelism w/ `tokio::spawn`](https://tokio.rs/tokio/tutorial/select#per-task-concurrency)
//! - std docs:
//!     - <https://doc.rust-lang.org/std/future/trait.Future.html>
//!     - <https://doc.rust-lang.org/std/task/struct.Waker.html>
//!     - <https://doc.rust-lang.org/std/pin/index.html>
//! - polling crate docs:
//!     - <https://docs.rs/polling/latest/polling/index.html>
//!     - <https://en.wikipedia.org/wiki/Epoll>
//! - crossterm crate docs:
//!     - <https://github.com/crossterm-rs/crossterm/wiki/Upgrade-from-0.13-to-0.14#115-event-polling>
//!     - <https://github.com/crossterm-rs/crossterm/wiki/Upgrade-from-0.13-to-0.14#111-new-event-api>
//!     - <https://github.com/crossterm-rs/crossterm/blob/master/examples/event-stream-tokio.rs>

use crossterm::event::EventStream;
use futures_util::{FutureExt, StreamExt};
use r3bl_core::call_if_true;

use super::InputEvent;
use crate::DEBUG_TUI_SHOW_TERMINAL_BACKEND;

pub struct AsyncEventStream {
    event_stream: EventStream,
}

impl Default for AsyncEventStream {
    fn default() -> Self {
        Self {
            event_stream: EventStream::new(),
        }
    }
}

impl AsyncEventStream {
    pub async fn try_to_get_input_event(
        async_event_stream: &mut AsyncEventStream,
    ) -> Option<InputEvent> {
        let maybe_event = async_event_stream.event_stream.next().fuse().await;
        match maybe_event {
            Some(Ok(event)) => {
                let input_event: Result<InputEvent, ()> = event.try_into();
                match input_event {
                    Ok(input_event) => Some(input_event),
                    Err(e) => {
                        call_if_true!(DEBUG_TUI_SHOW_TERMINAL_BACKEND, {
                            tracing::error!("Error: {e:?}");
                        });
                        None
                    }
                }
            }
            Some(Err(e)) => {
                call_if_true!(DEBUG_TUI_SHOW_TERMINAL_BACKEND, {
                    tracing::error!("Error: {e:?}");
                });
                None
            }
            _ => None,
        }
    }
}
