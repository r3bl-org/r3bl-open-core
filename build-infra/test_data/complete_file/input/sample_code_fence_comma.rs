// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words epoll EINVAL filedescriptor pollfd kqueue

//! Linux input handling for [`DirectToAnsi`] backend.
//!
//! # Platform Support
//!
//! This module is **Linux-only** (gated by `#[cfg(target_os = "linux")]`).
//! On macOS and Windows, use the Crossterm backend instead (set via
//! [`TERMINAL_LIB_BACKEND`]).
//!
//! ## Why Linux-Only?
//!
//! This module uses [`mio`] for async I/O multiplexing. [`mio`] provides a clean
//! platform abstraction over OS-specific polling mechanisms:
//! - **Linux**: [`epoll(7)`] - works correctly with PTY/tty file descriptors
//! - **macOS**: [`kqueue(2)`] - **broken for PTY/tty polling**
//!
//! macOS's [`kqueue(2)`] returns [`EINVAL`] when attempting to monitor `/dev/tty` or
//! PTY file descriptors. This is a [known Darwin limitation] with no planned fix.
//! The [`mio`] maintainers have [declined to work around this] since it would require
//! mixing [`kqueue(2)`] with [`select(2)`].
//!
//! ## How Crossterm Solves This
//!
//! Crossterm uses the [`filedescriptor`] crate which provides a [`poll()`] wrapper:
//! - On Linux: uses [`poll(2)`] directly
//! - On macOS: uses [`select(2)`] instead (which works with PTY/tty)
//!
//! ```rust,ignore
//! // From filedescriptor crate (simplified)
//! #[cfg(target_os = "macos")]
//! pub fn poll_impl(pfd: &mut [pollfd], duration: Option<Duration>) -> Result<usize> {
//!     // Uses libc::select() instead of libc::poll()
//! }
//! ```
//!
//! ## Future macOS Support
//!
//! To enable `DirectToAnsi` on macOS, we would need to:
//! 1. Replace [`mio`] polling with [`filedescriptor::poll()`]
//! 2. Handle [`SIGWINCH`] via [`signal-hook`] with the self-pipe trick (since
//!    [`signal-hook-mio`] requires [`mio`])
//!
//! This is tracked as a potential future enhancement.
//!
//! [`DirectToAnsi`]: mod@super
//! [`DirectToAnsiInputDevice::try_read_event`]: DirectToAnsiInputDevice::try_read_event
//! [`TERMINAL_LIB_BACKEND`]: crate::TERMINAL_LIB_BACKEND
//! [`epoll(7)`]: https://man7.org/linux/man-pages/man7/epoll.7.html
//! [`kqueue(2)`]: https://man.freebsd.org/cgi/man.cgi?query=kqueue&sektion=2
//! [`poll(2)`]: https://man7.org/linux/man-pages/man2/poll.2.html
//!
//! ## References
//!
//! - [mio issue #1377] - "Polling from /dev/tty on macOS"
//! - [crossterm issue #500] - "/dev/tty does not work on macOS with kqueue"
//! - [macOS /dev/tty polling blog post] - Detailed technical explanation
//!
//! # Entry Point
//!
//! [`DirectToAnsiInputDevice::try_read_event`] is the main async method for reading
//! terminal input with zero-latency `ESC` key detection.
//!
//! [`select(2)`]: https://man7.org/linux/man-pages/man2/select.2.html
//! [`EINVAL`]: https://man7.org/linux/man-pages/man3/errno.3.html
//! [`SIGWINCH`]: https://man7.org/linux/man-pages/man7/signal.7.html
//! [`filedescriptor`]: https://docs.rs/filedescriptor
//! [`filedescriptor::poll()`]: https://docs.rs/filedescriptor/latest/filedescriptor/fn.poll.html
//! [`mio`]: https://docs.rs/mio
//! [`poll()`]: https://docs.rs/filedescriptor/latest/filedescriptor/fn.poll.html
//! [`signal-hook`]: https://docs.rs/signal-hook
//! [`signal-hook-mio`]: https://docs.rs/signal-hook-mio
//! [crossterm issue #500]: https://github.com/crossterm-rs/crossterm/issues/500
//! [declined to work around this]: https://github.com/tokio-rs/mio/issues/1377
//! [known Darwin limitation]: https://nathancraddock.com/blog/macos-dev-tty-polling/
//! [macOS /dev/tty polling blog post]: https://nathancraddock.com/blog/macos-dev-tty-polling/
//! [mio issue #1377]: https://github.com/tokio-rs/mio/issues/1377

// Private submodules - organized by functional concern.
mod input_device;

// Conditionally public for documentation (to allow rustdoc links from mio_poller docs).
#[cfg(any(test, doc))]
pub mod paste_state_machine;
#[cfg(not(any(test, doc)))]
mod paste_state_machine;

#[cfg(any(test, doc))]
pub mod stateful_parser;
#[cfg(not(any(test, doc)))]
mod stateful_parser;

// Conditionally public for documentation (to allow rustdoc links from public items).
#[cfg(any(test, doc))]
pub mod mio_poller;
#[cfg(not(any(test, doc)))]
mod mio_poller;

// Conditionally public for documentation (to allow rustdoc links from public items).
#[cfg(any(test, doc))]
pub mod global_input_resource;
#[cfg(not(any(test, doc)))]
mod global_input_resource;

#[cfg(any(test, doc))]
pub mod types;
#[cfg(not(any(test, doc)))]
mod types;

#[cfg(any(test, doc))]
pub mod protocol_conversion;
#[cfg(not(any(test, doc)))]
mod protocol_conversion;

// Re-exports - flatten the public API.
pub use input_device::*;

// Documentation-only module pointing to actual PTY tests location.
// Named differently from output::integration_tests to avoid ambiguous glob re-exports.
#[cfg(any(test, doc))]
pub mod integration_tests_docs;
