// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words EINTR wakeup kqueue epoll ttimeoutlen

//! # Architecture Overview
//!
//! This module encapsulates all state and logic for the [`mio`] poller thread. It owns
//! and manages the following:
//!
//! ## Resources Managed
//!
//! | Resource                                | Responsibility                                                                                                                   |
//! | :-------------------------------------- | :------------------------------------------------------------------------------------------------------------------------------- |
//! | [**Poll**][`mio::Poll`]                 | Wait efficiently for [`stdin`] data and [`SIGWINCH`] signals                                                                     |
//! | [**Stdin**][`stdin`]                    | Read bytes into buffer -> handle using [VT100 input parser] and [paste state machine] to generate [`ReaderThreadMessage::Event`] |
//! | [**Signals**][`signal_hook_mio`]        | Drain signal ([`SIGWINCH`]) and generate [`ReaderThreadMessage::Resize`]                                                         |
//! | [**Channel**][`tokio::sync::broadcast`] | Publish [`ReaderThreadMessage`] variants to async consumers                                                                      |
//!
//! ## Quick Reference
//!
//! | Item                                             | Description                                                        |
//! | :----------------------------------------------- | :----------------------------------------------------------------- |
//! | [`MioPollerThread`]                              | Core struct: owns poll handle, buffers, parser, and channel sender |
//! | [`MioPollerThread::spawn_thread()`]              | Entry point: spawns the dedicated [`mio-poller`] thread            |
//! | [`MioPollerThread::start()`]                     | Main event loop: blocks on [`mio::Poll`], dispatches events        |
//! | [`SourceRegistry`]                               | Holds [`stdin`] and [`SIGWINCH`] signal handles                    |
//! | [`SourceKindReady`]                              | Enum mapping [`mio::Token`] ↔ source kind for dispatch             |
//! | [`dispatch()`]                                   | Routes ready events to appropriate handlers                        |
//! | [`consume_stdin_input()`]                        | Reads and parses stdin bytes into [`InputEvent`]s                  |
//! | [`consume_pending_signals()`]                    | Drains [`SIGWINCH`] signals, sends [`Resize`]                      |
//! | [VT100 input parser] ([`StatefulInputParser`])   | Accumulates bytes, parses [`VT100InputEventIR`] with ESC handling  |
//! | [paste state machine] ([`PasteCollectionState`]) | Collects text between bracketed paste markers                      |
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
//! A dedicated [`std::thread`] runs for the process lifetime,
//! using [`mio::Poll`] to
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
//! exclusively. There are two distinct exit mechanisms:
//!
//! #### Thread Self-Termination (process continues)
//!
//! The thread exits gracefully while the process continues running. This allows async
//! consumers to react (e.g., save state, clean up) before the application decides to exit:
//!
//! | Trigger                    | Behavior                                               |
//! | :------------------------- | :----------------------------------------------------- |
//! | [`stdin`] [`EOF`]          | [`read()`] returns 0 → sends [`Eof`] → thread exits    |
//! | I/O error (not [`EINTR`])  | Sends [`Error`] → thread exits                         |
//! | Receiver dropped           | [`tx.send()`] returns [`Err`] → thread exits           |
//!
//! #### Process Termination (OS kills everything)
//!
//! When the process itself terminates, the OS kills all threads immediately—no cleanup
//! code runs in the [`mio`] thread:
//!
//! | Trigger                    | Behavior                                               |
//! | :------------------------- | :----------------------------------------------------- |
//! | `main()` returns           | Process exits → OS terminates all threads              |
//! | [`std::process::exit()`]   | OS terminates process → all threads killed             |
//! | `Ctrl+C` / [`SIGINT`]      | OS terminates process → all threads killed             |
//!
//! This is safe because:
//! - [`INPUT_RESOURCE`] is a [`LazyLock`]`<...>` static, never dropped until process exit.
//! - The thread is doing nothing when blocked—[`mio`] uses efficient OS primitives.
//! - There are no resources to leak—[`stdin`] is [`fd`][file descriptor] `0`, which is
//!   not owned by us.
//!
//! #### EINTR Handling
//!
//! [`EINTR`] ([`ErrorKind::Interrupted`]) occurs when a signal interrupts a blocking
//! [`syscall`]. Both [`poll()`] and [`read()`] can return this error. Unlike other
//! errors, [`EINTR`] is **retried** (not sent as [`Error`])—the operation simply
//! resumes on the next loop iteration.
//!
//! ## What is [`mio`]?
//!
//! [`mio`] provides **synchronous I/O multiplexing** - a thin wrapper around OS
//! primitives:
//! - **Linux**: [`epoll`]
//! - **macOS**: [`kqueue`]
//!
//! It's *blocking* but efficient - [`poll.poll(&mut events, None)`] blocks the thread until
//! something happens on either [file descriptor]. Unlike [`select()`] or raw [`poll()`], [`mio`] uses the
//! optimal [`syscall`] per platform.
//!
//! <div class="warning">
//!
//! **Why not tokio for stdin?** Because [`tokio::io::stdin()`] uses a blocking threadpool
//! internally, and cancelling a [`tokio::select!`] branch doesn't stop the underlying
//! read - it keeps running as a "zombie", causing the problems described in
//! [`global_input_resource`].
//!
//! **Why is [`MioPollerThread`] not implemented for macOS?** See [Why Linux-Only?] in
//! the parent module—macOS [`kqueue`] can't poll PTY/tty file descriptors.
//!
//! </div>
//!
//! ## The Two File Descriptors
//!
//! A [file descriptor] (`fd`) is a Unix integer handle to an I/O resource (file, socket,
//! pipe, etc.). Two `fd`s are registered with [`mio`]'s registry so a single [`poll()`] call
//! can wait on either becoming ready:
//!
//! **1. `stdin` `fd`** - The raw file descriptor (`fd 0`) for standard input, obtained via
//! [`std::io::stdin().as_raw_fd()`][AsRawFd::as_raw_fd]. We wrap it in [`SourceFd`] so [`mio`] can poll it:
//!
//! <!-- It is ok to use ignore here -->
//!
//! ```ignore
//! registry.register(&mut SourceFd(&stdin_fd), SourceKindReady::Stdin.to_token(), Interest::READABLE)
//! ```
//!
//! **2. Signal watcher fd** - Signals aren't file descriptors, so [`signal_hook_mio`]
//! provides a clever adapter: it creates an internal pipe that becomes readable when
//! [`SIGWINCH`] arrives. This lets [`mio`] wait on signals just like any other fd:
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
//! We use [`mio`] instead of raw [`poll()`] or [`rustix::event::poll()`] because [`mio`]
//! provides a clean platform abstraction over [`epoll`] on Linux. See [Why Linux-Only?]
//! for why this module doesn't support macOS.
//!
//! # ESC Detection Limitations
//!
//! Both the `ESC` key and escape sequences (like `Up Arrow` = `ESC [ A`) start with the
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
//! [100ms `ttimeoutlen` delay]: https://vi.stackexchange.com/questions/24925/usage-of-timeoutlen-and-ttimeoutlen
//! [AsRawFd::as_raw_fd]: std::os::unix::io::AsRawFd::as_raw_fd
//! [SSH]: https://en.wikipedia.org/wiki/Secure_Shell
//! [VT100 input parser]: super::stateful_parser::StatefulInputParser
//! [Why Linux-Only?]: super#why-linux-only
//! [`EINTR`]: https://man7.org/linux/man-pages/man3/errno.3.html
//! [`EOF`]: https://en.wikipedia.org/wiki/End-of-file
//! [`Eof`]: super::types::ReaderThreadMessage::Eof
//! [`Err`]: std::result::Result::Err
//! [`ErrorKind::Interrupted`]: std::io::ErrorKind::Interrupted
//! [`Error`]: super::types::ReaderThreadMessage::Error
//! [`Event(InputEvent)`]: super::types::ReaderThreadMessage::Event
//! [`INPUT_RESOURCE`]: super::global_input_resource::INPUT_RESOURCE
//! [`InputEvent`]: crate::InputEvent
//! [`LazyLock`]: std::sync::LazyLock
//! [`MioPollerThread::spawn_thread()`]: poller_thread::MioPollerThread::spawn_thread
//! [`MioPollerThread::start()`]: poller_thread::MioPollerThread::start
//! [`MioPollerThread`]: poller_thread::MioPollerThread
//! [`PasteCollectionState`]: super::paste_state_machine::PasteCollectionState
//! [`ReaderThreadMessage::Event`]: super::types::ReaderThreadMessage::Event
//! [`ReaderThreadMessage::Resize`]: super::types::ReaderThreadMessage::Resize
//! [`ReaderThreadMessage`]: super::types::ReaderThreadMessage
//! [`Resize`]: super::types::ReaderThreadMessage::Resize
//! [`SIGINT`]: signal_hook::consts::SIGINT
//! [`SIGWINCH`]: signal_hook::consts::SIGWINCH
//! [`SourceFd`]: mio::unix::SourceFd
//! [`SourceKindReady`]: sources::SourceKindReady
//! [`SourceRegistry`]: sources::SourceRegistry
//! [`StatefulInputParser`]: super::stateful_parser::StatefulInputParser
//! [`VT100InputEventIR`]: crate::core::ansi::vt_100_terminal_input_parser::VT100InputEventIR
//! [`consume_pending_signals()`]: handler_signals::consume_pending_signals
//! [`consume_stdin_input()`]: handler_stdin::consume_stdin_input
//! [`crossterm`]: ::crossterm
//! [`dispatch()`]: dispatcher::dispatch
//! [`epoll`]: https://man7.org/linux/man-pages/man7/epoll.7.html
//! [`global_input_resource`]: super::global_input_resource#the-problems
//! [`kqueue`]: https://man.freebsd.org/cgi/man.cgi?query=kqueue&sektion=2
//! [`mio-poller`]: mod@self
//! [`mio::Poll`]: mio::Poll
//! [`mio::Token`]: mio::Token
//! [`mio`]: mio
//! [`poll()`]: https://man7.org/linux/man-pages/man2/poll.2.html
//! [`poll.poll(&mut events, None)`]: mio::Poll::poll
//! [`read()`]: https://man7.org/linux/man-pages/man2/read.2.html
//! [`rustix::event::poll()`]: rustix::event::poll
//! [`select()`]: https://man7.org/linux/man-pages/man2/select.2.html
//! [`signal_hook_mio`]: signal_hook_mio
//! [`std::thread`]: std::thread
//! [`stdin`]: std::io::stdin
//! [`syscall`]: https://en.wikipedia.org/wiki/System_call
//! [`tokio::io::stdin()`]: tokio::io::stdin
//! [`tokio::select!`]: tokio::select
//! [`tokio::sync::broadcast`]: tokio::sync::broadcast
//! [`tx.send()`]: tokio::sync::broadcast::Sender::send
//! [file descriptor]: https://en.wikipedia.org/wiki/File_descriptor
//! [paste state machine]: super::paste_state_machine::PasteCollectionState

// Skip rustfmt for rest of file.
// https://stackoverflow.com/a/75910283/2085356
#![cfg_attr(rustfmt, rustfmt_skip)]

// Conditionally public for docs and tests (enables intra-doc links).
#[cfg(any(test, doc))]
pub mod poller_thread;
#[cfg(not(any(test, doc)))]
mod poller_thread;

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
pub use poller_thread::*;
pub use sources::*;
