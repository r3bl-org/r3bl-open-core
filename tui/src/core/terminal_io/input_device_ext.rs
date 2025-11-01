// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Using [`crossterm::event::poll`] is non-blocking, but also inefficient. The following
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
//! The following code uses [`crossterm::event::read`] and blocks the thread that its
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

use crate::InputEvent;

/// Trait for reading input events asynchronously.
///
/// Provides a unified interface for different input device backends to deliver
/// terminal input events.
///
/// # Implementing backends
///
/// Each backend implementation should:
/// - Filter out unsupported event types (Key Release/Repeat, Paste events)
/// - Convert crossterm events to [`InputEvent`]
/// - Return `None` on error or stream closure
pub trait InputDeviceExt {
  async fn next_input_event(&mut self) -> Option<InputEvent>;
}
