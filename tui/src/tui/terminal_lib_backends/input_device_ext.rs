/*
 *   Copyright (c) 2022-2025 R3BL LLC
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

//! Using [crossterm::event::poll] is non-blocking, but also inefficient. The following
//! code will generate some CPU utilization while idling.
//!
//! ```
//! use std::time::Duration;
//! use std::io;
//! use crossterm::event::{read, poll};
//!
//! fn print_events() -> io::Result<bool> {
//!     loop {
//!         if poll(Duration::from_millis(100))? {
//!             // It's guaranteed that `read` won't block, because `poll` returned
//!             // `Ok(true)`.
//!             println!("{:?}", read()?);
//!         } else {
//!             // Timeout expired, no `Event` is available
//!         }
//!     }
//! }
//! ```
//!
//! The following code uses [crossterm::event::read] and blocks the thread that its
//! running on.
//!
//! ```
//! use crossterm::event::read;
//! use std::io;
//!
//! fn print_events() -> io::Result<bool> {
//!     loop {
//!         // Blocks until an `Event` is available
//!         println!("{:?}", read()?);
//!     }
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

use futures_util::FutureExt;

use super::InputEvent;
use crate::{InputDevice, DEBUG_TUI_SHOW_TERMINAL_BACKEND};

pub trait InputDeviceExt {
    #[allow(async_fn_in_trait)]
    async fn next_input_event(&mut self) -> Option<InputEvent>;
}

impl InputDeviceExt for InputDevice {
    async fn next_input_event(&mut self) -> Option<InputEvent> {
        let maybe_result_event = self.next().fuse().await;
        match maybe_result_event {
            Ok(event) => {
                let input_event = InputEvent::try_from(event);
                match input_event {
                    Ok(input_event) => Some(input_event),
                    Err(e) => {
                        DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                            // % is Display, ? is Debug.
                            tracing::error!(
                                message = "Error converting input event.",
                                error = ?e,
                            );
                        });
                        None
                    }
                }
            }
            Err(e) => {
                DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                    // % is Display, ? is Debug.
                    tracing::error!(
                        message = "Error reading input event.",
                        error = ?e,
                    );
                });
                None
            }
        }
    }
}
