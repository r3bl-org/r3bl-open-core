// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words EINTR wakeup kqueue epoll ttimeoutlen EINVAL

//! # Architecture Overview
//!
//! This module encapsulates all state and logic for the [`mio`] poller thread. It owns
//! and manages the following:
//!
//! | Resource    | Responsibility                                                                                                                   |
//! | ----------- | -------------------------------------------------------------------------------------------------------------------------------- |
//! | **Poll**    | Wait efficiently for [`stdin`] data and [`SIGWINCH`] signals                                                                     |
//! | **Stdin**   | Read bytes into buffer -> handle using [VT100 input parser] and [paste state machine] to generate [`ReaderThreadMessage::Event`] |
//! | **Signals** | Drain signals and generate [`ReaderThreadMessage::Resize`]                                                                       |
//! | **Channel** | Publish [`ReaderThreadMessage`] variants to async consumers                                                                      |
//!
//! # How It Works
//!
//! Our design separates these two:
//!
//! 1. **Blocking I/O** (the [`mio`] thread owns [`stdin`] exclusively).
//! 2. **Async consumption** ([`tokio`] tasks await on channel). The
//!    [`tokio::sync::broadcast`] channel bridges sync and async worlds, supporting
//!    multiple consumers that each receive all events.
//!
//! The sections below explain each component in detail.
//!
//! ## The [`mio`]-poller Thread
//!
//! A dedicated [`std::thread`] runs for the process lifetime, using [`mio::Poll`] to
//! efficiently wait on multiple file descriptors:
//!
//! ```text
//! ┌────────────────────────────────────┐           ┌─────────────────────────────────┐
//! │ Dedicated Thread (std::thread)     │           │ Async Consumers (tokio runtime) │
//! │                                    │           │                                 │
//! │ mio::Poll waits on:                ├───────────▶ rx.recv().await (fan-out)       │
//! │   • stdin fd (Token 0)             │ broadcast │                                 │
//! │   • SIGWINCH signal (Token 1)      │           │                                 │
//! └────────────────────────────────────┘           └─────────────────────────────────┘
//! ```
//!
//! ### Thread Lifecycle
//!
//! The dedicated thread can't be terminated or cancelled, and it safely owns [`stdin`]
//! exclusively. The OS is responsible for cleaning it up when the process exits.
//!
//! | Exit Mechanism           | How Thread Exits                            |
//! | ------------------------ | ------------------------------------------- |
//! | Ctrl+C / `SIGINT`        | OS terminates process → all threads killed  |
//! | [`std::process::exit()`] | OS terminates process → all threads killed  |
//! | `main()` returns         | Rust runtime exits → OS terminates process  |
//! | [`stdin`] EOF            | `read()` returns 0 → thread exits naturally |
//!
//! This is ok because:
//! - [`INPUT_RESOURCE`] lives for `'static` (the lifetime of the process) - it's a
//!   [`LazyLock`]`<...>` static, never dropped until process exit.
//! - Thread is doing nothing when blocked - [`mio`] uses efficient OS primitives.
//! - No resources to leak - [`stdin`] is `fd` `0`, not owned by us.
//!
//! The thread self-terminates gracefully in these scenarios:
//! 1. **EOF on [`stdin`]**: When [`stdin`] is closed (e.g., pipe closed, `Ctrl+D`),
//!    `read()` returns 0 bytes. The thread sends [`ReaderThreadMessage::Eof`] and exits.
//! 2. **I/O error**: On read errors (except `EINTR` which is retried), the thread sends
//!    [`ReaderThreadMessage::Error`] and exits.
//! 3. **Receiver dropped**: When [`INPUT_RESOURCE`] is dropped (process exit), the channel
//!    receiver is dropped. The next `tx.send()` returns `Err`, and exits.
//!
//! ## What is [`mio`]?
//!
//! [`mio`] provides **synchronous I/O multiplexing** - a thin wrapper around OS
//! primitives:
//! - **Linux**: [`epoll`]
//! - **macOS**: [`kqueue`]
//!
//! It's *blocking* but efficient - `poll.poll(&mut events, None)` blocks the thread until
//! something happens on either fd. Unlike [`select()`] or raw [`poll()`], mio uses the
//! optimal syscall per platform.
//!
//! **Why not tokio for stdin?** Because [`tokio::io::stdin()`] uses a blocking threadpool
//! internally, and cancelling a [`tokio::select!`] branch doesn't stop the underlying
//! read - it keeps running as a "zombie", causing the problems described in
//! [`global_input_resource`].
//!
//! **Why is this not implemented for macOS?** Because macOS [`kqueue`] returns `EINVAL`
//! when polling PTY/tty file descriptors - a known Darwin kernel limitation. On macOS, we
//! use the [`crossterm`] backend instead. For details:
//! - [Blog post explaining the issue]
//! - [mio-issue]
//! - [crossterm-issue]
//!
//! ## The Two File Descriptors
//!
//! A file descriptor (fd) is a Unix integer handle to an I/O resource (file, socket,
//! pipe, etc.). Two fds are registered with [`mio`]'s registry so a single `poll()` call
//! can wait on either becoming ready:
//!
//! **1. `stdin` fd** - The raw file descriptor (fd 0) for standard input, obtained via
//! `std::io::stdin().as_raw_fd()`. We wrap it in [`SourceFd`] so mio can poll it:
//!
//! <!-- It is ok to use ignore here -->
//!
//! ```ignore
//! registry.register(&mut SourceFd(&stdin_fd), SourceKindReady::Stdin.to_token(), Interest::READABLE)
//! ```
//!
//! **2. Signal watcher fd** - Signals aren't file descriptors, so [`signal_hook_mio`]
//! provides a clever adapter: it creates an internal pipe that becomes readable when
//! `SIGWINCH` arrives. This lets [`mio`] wait on signals just like any other fd:
//!
//! <!-- It is ok to use ignore here -->
//!
//! ```ignore
//! let mut signals = Signals::new([SIGWINCH])?;  // Creates internal pipe
//! registry.register(&mut signals, SourceKindReady::Signals.to_token(), Interest::READABLE)
//! ```
//!
//! ## Parsing and the Channel
//!
//! When bytes arrive from [`stdin`], they flow through a parsing pipeline:
//!
//! ```text
//! Raw bytes → Parser::advance() → VT100InputEventIR → Paste state machine → InputEvent → Channel
//! ```
//!
//! The parser handles three tricky cases:
//! - **`ESC` disambiguation**: The `more` flag indicates if more bytes might be waiting.
//!   If `read_count == buffer_size`, we wait before deciding a lone `ESC` is the `ESC`
//!   key.
//! - **Chunked input**: The buffer accumulates bytes until a complete sequence is parsed.
//! - **UTF-8**: Multi-byte characters can span multiple reads.
//!
//! The channel sends [`ReaderThreadMessage`] variants to the async side:
//! - [`Event(InputEvent)`] - parsed keyboard/mouse input
//! - [`Resize`] - terminal window changed size
//! - [`Eof`] - stdin closed
//! - [`Error`] - I/O error
//!
//! # Why [`mio`] Instead of Raw [`poll()`]?
//!
//! We use [`mio`] instead of raw [`poll()`] or [`rustix::event::poll()`] because:
//!
//! - **macOS compatibility**: [`poll()`] cannot monitor `/dev/tty` on macOS, but [`mio`]
//!   uses [`kqueue`] which works correctly.
//! - **Platform abstraction**: [`mio`] uses the optimal syscall per platform ([`epoll`]
//!   on Linux, [`kqueue`] on macOS/BSD).
//!
//! # ESC Detection Limitations
//!
//! Both the `ESC` key and escape sequences (like Up Arrow = `ESC [ A`) start with the
//! same byte (`1B`). When we read a lone `ESC` byte, is it the `ESC` key or the start of
//! a sequence?
//!
//! ## The `more` Flag Heuristic
//!
//! We use [`crossterm`]'s `more` flag pattern: `more = (read_count == buffer_size)`. The
//! idea is that if `read()` fills the entire buffer, more data is probably waiting in
//! the kernel. So:
//!
//! - `more == true` + lone `ESC` → wait (might be start of escape sequence)
//! - `more == false` + lone `ESC` → emit `ESC` key (no more data waiting)
//!
//! ## Why This is a Heuristic, Not a Guarantee
//!
//! **This approach assumes that if `read()` returns fewer bytes than the buffer size, all
//! pending data has been consumed.** This is usually true, but not guaranteed:
//!
//! - **Local terminals**: Escape sequences are typically written atomically, so they
//!   arrive complete in one `read()`. The heuristic works well.
//! - **Over [SSH]**: TCP can fragment data arbitrarily. If `ESC` arrives in one packet
//!   and `[ A` in the next (even microseconds later), we might incorrectly emit `ESC`.
//! - **High latency networks**: The more latency and packet fragmentation, the higher the
//!   chance of incorrect `ESC` detection.
//!
//! ## Why Not Use a Timeout Like `vim`?
//!
//! Vim uses a [100ms `ttimeoutlen` delay] - if no more bytes arrive within 100ms after
//! `ESC`, it's the `ESC` key. This is more reliable but adds latency to every `ESC`
//! keypress.
//!
//! We chose the `more` flag heuristic (following [`crossterm`]) because:
//! - Zero latency for `ESC` key in the common case (local terminal).
//! - Acceptable behavior for most [SSH] connections (TCP usually delivers related bytes
//!   together). In our testing there were no issues over [SSH].
//! - The failure mode (`ESC` emitted early) is annoying but not catastrophic.
//!
//! **Trade-off**: Faster `ESC` response vs. occasional incorrect detection on
//! high-latency connections.
//!
//!     https://vi.stackexchange.com/questions/24925/usage-of-timeoutlen-and-ttimeoutlen
//!
//! [100ms `ttimeoutlen` delay]:
//! [Blog post explaining the issue]: https://nathancraddock.com/blog/macos-dev-tty-polling/
//! [SSH]: https://en.wikipedia.org/wiki/Secure_Shell
//! [VT100 input parser]: super::stateful_parser::StatefulInputParser
//! [`Eof`]: super::types::ReaderThreadMessage::Eof
//! [`Error`]: super::types::ReaderThreadMessage::Error
//! [`Event(InputEvent)`]: super::types::ReaderThreadMessage::Event
//! [`INPUT_RESOURCE`]: super::global_input_resource::INPUT_RESOURCE
//! [`LazyLock`]: std::sync::LazyLock
//! [`ReaderThreadMessage::Eof`]: super::types::ReaderThreadMessage::Eof
//! [`ReaderThreadMessage::Error`]: super::types::ReaderThreadMessage::Error
//! [`ReaderThreadMessage::Event`]: super::types::ReaderThreadMessage::Event
//! [`ReaderThreadMessage::Resize`]: super::types::ReaderThreadMessage::Resize
//! [`ReaderThreadMessage`]: super::types::ReaderThreadMessage
//! [`Resize`]: super::types::ReaderThreadMessage::Resize
//! [`SIGWINCH`]: signal_hook::consts::SIGWINCH
//! [`SourceFd`]: mio::unix::SourceFd
//! [`crossterm`]: ::crossterm
//! [`epoll`]: https://man7.org/linux/man-pages/man7/epoll.7.html
//! [`global_input_resource`]: super::global_input_resource
//! [`kqueue`]: https://man.freebsd.org/cgi/man.cgi?query=kqueue&sektion=2
//! [`mio::Poll`]: mio::Poll
//! [`mio`]: mio
//! [`poll()`]: https://man7.org/linux/man-pages/man2/poll.2.html
//! [`rustix::event::poll()`]: rustix::event::poll
//! [`select()`]: https://man7.org/linux/man-pages/man2/select.2.html
//! [`signal_hook_mio`]: signal_hook_mio
//! [`std::thread`]: std::thread
//! [`stdin`]: std::io::stdin
//! [`tokio::io::stdin()`]: tokio::io::stdin
//! [`tokio::select!`]: tokio::select
//! [`tokio::sync::broadcast`]: tokio::sync::broadcast
//! [crossterm-issue]: https://github.com/crossterm-rs/crossterm/issues/500
//! [mio-issue]: https://github.com/tokio-rs/mio/issues/1377
//! [paste state machine]: super::paste_state_machine::PasteCollectionState

// Skip rustfmt for rest of file.
// https://stackoverflow.com/a/75910283/2085356
#![cfg_attr(rustfmt, rustfmt_skip)]

// Conditionally public for docs and tests (enables intra-doc links).
#[cfg(any(test, doc))]
pub mod poller;
#[cfg(not(any(test, doc)))]
mod poller;

#[cfg(any(test, doc))]
pub mod sources;
#[cfg(not(any(test, doc)))]
mod sources;

#[cfg(any(test, doc))]
pub mod dispatcher;
#[cfg(not(any(test, doc)))]
mod dispatcher;

#[cfg(any(test, doc))]
pub mod handler_signals;
#[cfg(not(any(test, doc)))]
mod handler_signals;

#[cfg(any(test, doc))]
pub mod handler_stdin;
#[cfg(not(any(test, doc)))]
mod handler_stdin;

// Re-export public API.
pub use poller::*;
pub use sources::*;
