// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words epoll EINVAL filedescriptor pollfd kqueue

//! Linux input handling for [`DirectToAnsi`] backend.
//!
//! # Entry Point
//!
//! [`DirectToAnsiInputDevice::next`] is the main async method for reading
//! terminal input with zero-latency [`ESC` key detection].
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
//! <!-- It is ok to use ignore here - shows internal filedescriptor crate implementation,
//! not runnable example -->
//!
//! ```ignore
//! // From filedescriptor crate (simplified)
//! #[cfg(target_os = "macos")]
//! pub fn poll_impl(pfd: &mut [pollfd], duration: Option<Duration>) -> Result<usize> {
//!     // Uses libc::select() instead of libc::poll()
//! }
//! ```
//!
//! ## Future macOS Support
//!
//! To enable [`DirectToAnsi`] on macOS, we would need to:
//! 1. Replace [`mio`] polling with [`filedescriptor::poll()`]
//! 2. Handle [`SIGWINCH`] via [`signal-hook`] with the self-pipe trick (since
//!    [`signal-hook-mio`] requires [`mio`])
//!
//! This is tracked as a potential future enhancement.
//!
//! ## References
//!
//! - [mio issue] - "Polling from /dev/tty on macOS"
//! - [crossterm issue] - "/dev/tty does not work on macOS with kqueue"
//! - [macOS /dev/tty polling blog post] - Detailed technical explanation
//!
//! [`DirectToAnsiInputDevice::next`]: DirectToAnsiInputDevice::next
//! [`DirectToAnsi`]: super
//! [`EINVAL`]: https://man7.org/linux/man-pages/man3/errno.3.html
//! [`ESC` key detection]: DirectToAnsiInputDevice#esc-key-disambiguation-crossterm-more-flag-pattern
//! [`SIGWINCH`]: https://man7.org/linux/man-pages/man7/signal.7.html
//! [`TERMINAL_LIB_BACKEND`]: crate::TERMINAL_LIB_BACKEND
//! [`epoll(7)`]: https://man7.org/linux/man-pages/man7/epoll.7.html
//! [`filedescriptor::poll()`]: https://docs.rs/filedescriptor/latest/filedescriptor/fn.poll.html
//! [`filedescriptor`]: https://docs.rs/filedescriptor
//! [`kqueue(2)`]: https://man.freebsd.org/cgi/man.cgi?query=kqueue&sektion=2
//! [`poll()`]: https://docs.rs/filedescriptor/latest/filedescriptor/fn.poll.html
//! [`poll(2)`]: https://man7.org/linux/man-pages/man2/poll.2.html
//! [`select(2)`]: https://man7.org/linux/man-pages/man2/select.2.html
//! [`signal-hook-mio`]: https://docs.rs/signal-hook-mio
//! [`signal-hook`]: https://docs.rs/signal-hook
//! [crossterm issue]: https://github.com/crossterm-rs/crossterm/issues/500
//! [declined to work around this]: https://github.com/tokio-rs/mio/issues/1377
//! [known Darwin limitation]: https://nathancraddock.com/blog/macos-dev-tty-polling/
//! [macOS /dev/tty polling blog post]: https://nathancraddock.com/blog/macos-dev-tty-polling/
//! [mio issue]: https://github.com/tokio-rs/mio/issues/1377

// Private submodules - organized by functional concern.
mod input_device_public_api;

// Conditionally public for documentation.
#[cfg(any(test, doc))]
pub mod input_device_impl;
#[cfg(not(any(test, doc)))]
mod input_device_impl;

// Conditionally public for documentation.
#[cfg(any(test, doc))]
pub mod paste_state_machine;
#[cfg(not(any(test, doc)))]
mod paste_state_machine;

#[cfg(any(test, doc))]
pub mod stateful_parser;
#[cfg(not(any(test, doc)))]
mod stateful_parser;

#[cfg(any(test, doc))]
pub mod mio_poller;
#[cfg(not(any(test, doc)))]
mod mio_poller;

#[cfg(any(test, doc))]
pub mod channel_types;
#[cfg(not(any(test, doc)))]
mod channel_types;

#[cfg(any(test, doc))]
pub mod protocol_conversion;
#[cfg(not(any(test, doc)))]
mod protocol_conversion;

#[cfg(any(test, doc))]
pub mod at_most_one_instance_assert;
#[cfg(not(any(test, doc)))]
mod at_most_one_instance_assert;

// Re-exports - flatten the public API.
pub use input_device_impl::*;
pub use input_device_public_api::*;
pub use mio_poller::*;

// Documentation-only module pointing to actual PTY tests location.
// Named differently from output::integration_tests to avoid ambiguous glob re-exports.
#[cfg(any(test, doc))]
pub mod integration_tests_docs;
