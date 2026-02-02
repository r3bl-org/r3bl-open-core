// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words EINTR wakeup kqueue epoll ttimeoutlen eventfd userspace

//! # Architecture Overview
//!
//! This module encapsulates all state and logic for the [`mio`] poller thread. It
//! manages the following:
//!
//! ## Resources Managed
//!
//! | Resource                                | Responsibility                                                                                                                   |
//! | :-------------------------------------- | :------------------------------------------------------------------------------------------------------------------------------- |
//! | [**Poll**][`mio::Poll`]                 | Wait efficiently for [`stdin`] data and [`SIGWINCH`] signals                                                                     |
//! | [**Stdin**][`stdin`]                    | Read bytes into buffer -> handle using [VT100 input parser] and [paste state machine] to generate [`PollerEvent::Stdin`]         |
//! | [**Signals**][`signal_hook_mio`]        | Drain signal ([`SIGWINCH`]) and generate [`PollerEvent::Signal`]                                                                 |
//! | [**Channel**][`tokio::sync::broadcast`] | Publish [`PollerEvent`] variants to async consumers                                                                              |
//!
//! ## Quick Reference
//!
//! | Item                                             | Description                                                         |
//! | :----------------------------------------------- | :------------------------------------------------------------------ |
//! | [`MioPollWorker`]                                | Core struct: holds poll handle, buffers, parser (implements RRT)    |
//! | [`MioPollWaker`]                                 | Waker to interrupt blocked poll (implements RRT [`RRTWaker`])       |
//! | [`MioPollWorkerFactory`]                         | Factory to create worker and waker together                         |
//! | [`SourceRegistry`]                               | Holds [`stdin`] and [`SIGWINCH`] signal handles                     |
//! | [`SourceKindReady`]                              | Enum mapping [`mio::Token`] ↔ source kind for dispatch              |
//! | [`dispatch_with_tx()`]                           | Routes ready events to appropriate handlers                         |
//! | [`consume_stdin_input_with_tx()`]                | Reads and parses [`stdin`] bytes into [`InputEvent`]s               |
//! | [`consume_pending_signals_with_tx()`]            | Drains [`SIGWINCH`] signals, sends [`SignalEvent::Resize`]          |
//! | [VT100 input parser] ([`StatefulInputParser`])   | Accumulates bytes, parses [`VT100InputEventIR`] with ESC handling   |
//! | [paste state machine] ([`PasteCollectionState`]) | Collects text between bracketed paste markers                       |
//!
//! # How It Works
//!
//! Our design separates these two:
//!
//! 1. **Blocking I/O** - a dedicated thread is the designated reader of [`stdin`] and
//!    handler of [`SIGWINCH`], blocking on [`mio::Poll::poll()`].
//! 2. **Multiple async consumers** - The [`tokio::sync::broadcast`] channel bridges sync
//!    and async worlds, supporting multiple consumers that each receive all events. Each
//!    consumer is a [`tokio`] task that awaits on the receiver end of the broadcast channel.
//!
//! The sections below explain each component in detail.
//!
//! ## The [`mio`]-poller Thread
//!
//! A dedicated [`std::thread`] runs for the lifetime of the app that spawns it, using
//! [`mio::Poll`] to efficiently wait on multiple event sources:
//!
//! ```text
//! ┌────────────────────────────────────┐           ┌─────────────────────────────────┐
//! │ Dedicated Thread (std::thread)     │           │ Async Consumers (tokio runtime) │
//! │                                    │           │                                 │
//! │ mio::Poll waits on:                ├───────────▶ rx.recv().await (fan-out)       │
//! │   • stdin fd (Token 0)             │ broadcast │                                 │
//! │   • SIGWINCH signal (Token 1)      │           │                                 │
//! │   • ReceiverDropWaker (Token 2)    │           │                                 │
//! └────────────────────────────────────┘           └─────────────────────────────────┘
//! ```
//!
//! ## Thread Lifecycle
//!
//! The [`mio_poller`] thread can be **relaunched** if it exits. See the
//! [`resilient_reactor_thread`] module for comprehensive documentation including:
//!
//! - Thread Lifecycle Overview — spawn → exit → respawn sequence
//! - The Inherent Race Condition — why the race exists and how we handle it
//! - What Happens If We Exit Blindly — the zombie device scenario
//! - Why Thread Reuse Is Safe — resource safety table
//!
//!
//! See [Device Lifecycle] for a detailed diagram showing how threads spawn and exit with
//! each app lifecycle, and [Related Tests] for PTY-based integration tests validating
//! the lifecycle behavior.
//!
//! Key properties:
//! - **No external termination**: Other code cannot forcibly terminate or cancel this
//!   thread—this is an [OS/threading limitation] ([Rust discussion], [Rust workarounds]).
//!   However, external code CAN **signal** the thread to exit gracefully by dropping the
//!   [`DirectToAnsiInputDevice`]—each TUI app creates one on startup and drops it on
//!   exit (a process may run multiple TUI apps sequentially). Dropping the device drops
//!   its internal [`SubscriberGuard`], waking the thread to check [`receiver_count()`].
//!   When all receivers are dropped, the thread exits on its own.
//! - It is the **designated reader** of [`stdin`]—other code should access input events
//!   via the broadcast channel, not by reading [`stdin`] directly.
//!
//! <div class="warning">
//!
//! **No exclusive access**: Any thread in the process can call [`std::io::stdin()`] and
//! read from it—there is no OS or Rust mechanism to prevent this. If another thread reads
//! from [`stdin`], bytes will be **stolen** from this thread, causing interleaved reads
//! that corrupt the input stream and break the VT100 parser state machine.
//!
//! </div>
//!
//! There are two distinct exit mechanisms:
//!
//! 1. Thread Self-Termination (process continues)
//!
//!    The thread exits gracefully while the process continues running. There are no not
//!    a silent failure in the thread, since async consumers are notified of the error
//!    case. This allows async consumers to react (e.g., save state, clean up) before the
//!    application decides to exit:
//!
//!    | Trigger                    | Behavior                                                                                                          |
//!    | :------------------------- | :---------------------------------------------------------------------------------------------------------------- |
//!    | [`stdin`] [`EOF`]          | [`read()`] returns `0` → sends [`StdinEvent::Eof`] → thread exits; see [`EOF`] note below)                        |
//!    | I/O error (not [`EINTR`])  | Sends [`StdinEvent::Error`] → thread exits (see [`EINTR`] handling below)                                         |
//!    | All receivers dropped      | [`SubscriberGuard::drop()`] wakes thread → checks `receiver_count() == 0` → exits (~1ms); see below               |
//!
//!    **Note on [`EOF`]**: TUI apps run in [raw mode], where `Ctrl+D` is just
//!    [`CONTROL_D`]—it doesn't trigger [`EOF`].
//!     - In [canonical mode], the kernel's [line discipline] interprets `Ctrl+D` as
//!       [`VEOF`] and signals [`EOF`] to readers.
//!     - In [raw mode], it passes bytes through unchanged. [`EOF`] only occurs when
//!       [`stdin`] is actually closed:
//!
//!    | Scenario                         | What Happens                                                        |
//!    | :------------------------------- | :------------------------------------------------------------------ |
//!    | Close terminal window            | Terminal emulator closes [PTY] controller → controlled gets [`EOF`] |
//!    | [SSH] connection drops           | sshd closes [PTY] controller → controlled gets [`EOF`]              |
//!    | `screen`/`tmux` session killed   | Multiplexer closes [PTY] controller → controlled gets [`EOF`]       |
//!    | Kill terminal emulator process   | Same as closing window                                              |
//!    | Network timeout ([SSH])          | sshd eventually closes connection → [`EOF`]                         |
//!    | Pipe closed                      | Writer closes pipe → reader gets [`EOF`]                            |
//!
//!    **Note on receiver drop (thread restart)**: When all [`broadcast::Receiver`]s are
//!    dropped, the thread exits. This typically happens when:
//!    - [`DirectToAnsiInputDevice`] is dropped (it holds a receiver internally)
//!    - A TUI app exits and drops its input device
//!    - All async consumers finish and drop their receivers
//!
//!    **The thread is restartable**: When the next [`DirectToAnsiInputDevice`] is created
//!    (or [`subscribe()`] is called), the terminated thread is detected via
//!    the liveness flag, and a new thread is spawned automatically. This allows sequential TUI
//!    apps in the same process to share the input system seamlessly.
//!
//! 2. Process Termination (OS kills everything)
//!
//!    When the process itself terminates, the OS kills all threads immediately—no cleanup
//!    code runs in the [`mio`] thread:
//!
//!    | Trigger                    | Behavior                                               |
//!    | :------------------------- | :----------------------------------------------------- |
//!    | `main()` returns           | Process exits → OS terminates all threads              |
//!    | [`std::process::exit()`]   | OS terminates process → all threads killed             |
//!    | `Ctrl+C` / [`SIGINT`]      | OS terminates process → all threads killed             |
//!
//! This is safe because:
//! - [`SINGLETON`] is a static [`Mutex`], never dropped until process exit.
//! - The thread is doing nothing when blocked—[`mio`] uses efficient OS primitives.
//! - There are no resources to leak—[`stdin`] is [`fd`][file descriptor] `0`, which is
//!   not owned by us.
//!
//! #### `EINTR` Handling
//!
//! [`EINTR`] ([`ErrorKind::Interrupted`]) occurs when a signal interrupts a blocking
//! [`syscall`]. Both [`poll()`] and [`read()`] can return this error. Unlike other
//! errors, [`EINTR`] is **retried** (not sent as [`StdinEvent::Error`])—the operation simply
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
//! [The Problems section in `DirectToAnsiInputDevice`].
//!
//! **Why is this not implemented for macOS?** See [Why Linux-Only?] in
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
//! The channel sends [`PollerEvent`] variants to the async side:
//! - [`Stdin(Input(InputEvent))`] - parsed keyboard/mouse input
//! - [`Signal(Resize)`] - terminal window changed size
//! - [`Stdin(Eof)`] - stdin closed
//! - [`Stdin(Error)`] - I/O error
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
//! [`Arc<AtomicBool>`]: std::sync::atomic::AtomicBool
//!
//! [100ms `ttimeoutlen` delay]: https://vi.stackexchange.com/questions/24925/usage-of-timeoutlen-and-ttimeoutlen
//! [AsRawFd::as_raw_fd]: std::os::unix::io::AsRawFd::as_raw_fd
//! [Device Lifecycle]: super::DirectToAnsiInputDevice#device-lifecycle
//! [OS/threading limitation]: https://man7.org/linux/man-pages/man3/pthread_cancel.3.html
//! [PTY]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [Related Tests]: crate::core::resilient_reactor_thread#related-tests
//! [Rust discussion]: https://internals.rust-lang.org/t/thread-cancel-support/3056
//! [Rust workarounds]: https://matklad.github.io/2018/03/03/stopping-a-rust-worker.html
//! [SSH]: https://en.wikipedia.org/wiki/Secure_Shell
//! [The Problems section in `DirectToAnsiInputDevice`]: super::DirectToAnsiInputDevice#the-problems
//! [VT100 input parser]: super::stateful_parser::StatefulInputParser
//! [Why Linux-Only?]: super#why-linux-only
//! [`CONTROL_D`]: crate::core::ansi::CONTROL_D
//! [`DirectToAnsiInputDevice`]: super::DirectToAnsiInputDevice
//! [`EINTR`]: https://man7.org/linux/man-pages/man3/errno.3.html
//! [`EOF`]: https://en.wikipedia.org/wiki/End-of-file
//! [`ErrorKind::Interrupted`]: std::io::ErrorKind::Interrupted
//! [`InputEvent`]: crate::InputEvent
//! [`MioPollWaker`]: mio_poll_waker::MioPollWaker
//! [`MioPollWorkerFactory`]: mio_poll_worker::MioPollWorkerFactory
//! [`MioPollWorker`]: mio_poll_worker::MioPollWorker
//! [`Mutex`]: std::sync::Mutex
//! [`PasteCollectionState`]: super::paste_state_machine::PasteCollectionState
//! [`PollerEvent::Signal`]: super::channel_types::PollerEvent::Signal
//! [`PollerEvent::Stdin`]: super::channel_types::PollerEvent::Stdin
//! [`PollerEvent`]: super::channel_types::PollerEvent
//! [`RRTWaker`]: crate::core::resilient_reactor_thread::RRTWaker
//! [`SIGINT`]: signal_hook::consts::SIGINT
//! [`SIGWINCH`]: signal_hook::consts::SIGWINCH
//! [`SINGLETON`]: super::input_device_impl::global_input_resource::SINGLETON
//! [`Signal(Resize)`]: super::channel_types::SignalEvent::Resize
//! [`SignalEvent::Resize`]: super::channel_types::SignalEvent::Resize
//! [`SourceFd`]: mio::unix::SourceFd
//! [`SourceKindReady`]: sources::SourceKindReady
//! [`SourceRegistry`]: sources::SourceRegistry
//! [`StatefulInputParser`]: super::stateful_parser::StatefulInputParser
//! [`Stdin(Eof)`]: super::channel_types::StdinEvent::Eof
//! [`Stdin(Error)`]: super::channel_types::StdinEvent::Error
//! [`Stdin(Input(InputEvent))`]: super::channel_types::StdinEvent::Input
//! [`StdinEvent::Eof`]: super::channel_types::StdinEvent::Eof
//! [`StdinEvent::Error`]: super::channel_types::StdinEvent::Error
//! [`SubscriberGuard::drop()`]: crate::core::resilient_reactor_thread::SubscriberGuard#impl-Drop-for-SubscriberGuard
//! [`SubscriberGuard`]: crate::core::resilient_reactor_thread::SubscriberGuard
//! [`VEOF`]: https://man7.org/linux/man-pages/man3/termios.3.html
//! [`VT100InputEventIR`]: crate::core::ansi::vt_100_terminal_input_parser::VT100InputEventIR
//! [`broadcast::Receiver`]: tokio::sync::broadcast::Receiver
//! [`consume_pending_signals_with_tx()`]: handler_signals::consume_pending_signals_with_tx
//! [`consume_stdin_input_with_tx()`]: handler_stdin::consume_stdin_input_with_tx
//! [`crossterm`]: ::crossterm
//! [`dispatch_with_tx()`]: dispatcher::dispatch_with_tx
//! [`epoll`]: https://man7.org/linux/man-pages/man7/epoll.7.html
//! [`handle_receiver_drop_waker_with_tx()`]: handler_receiver_drop::handle_receiver_drop_waker_with_tx
//! [`kqueue`]: https://man.freebsd.org/cgi/man.cgi?query=kqueue&sektion=2
//! [`mio::Poll`]: mio::Poll
//! [`mio::Token`]: mio::Token
//! [`mio::Waker`]: mio::Waker
//! [`mio_poller`]: mod@self
//! [`poll()`]: https://man7.org/linux/man-pages/man2/poll.2.html
//! [`poll.poll(&mut events, None)`]: mio::Poll::poll
//! [`read()`]: https://man7.org/linux/man-pages/man2/read.2.html
//! [`receiver_count()`]: tokio::sync::broadcast::Sender::receiver_count
//! [`resilient_reactor_thread`]: crate::core::resilient_reactor_thread
//! [`rustix::event::poll()`]: https://docs.rs/rustix/latest/rustix/event/fn.poll.html
//! [`select()`]: https://man7.org/linux/man-pages/man2/select.2.html
//! [`signal_hook_mio`]: signal_hook_mio
//! [`std::thread`]: std::thread
//! [`stdin`]: std::io::stdin
//! [`subscribe()`]: crate::core::resilient_reactor_thread::RRTSafeGlobalState::subscribe
//! [`syscall`]: https://man7.org/linux/man-pages/man2/syscalls.2.html
//! [`tokio::io::stdin()`]: tokio::io::stdin
//! [`tokio::select!`]: tokio::select
//! [`tokio::sync::broadcast`]: tokio::sync::broadcast
//! [`tx.send()`]: tokio::sync::broadcast::Sender::send
//! [`waker.wake()`]: mio::Waker::wake
//! [canonical mode]: crate::core::ansi::terminal_raw_mode#raw-mode-vs-cooked-mode
//! [file descriptor]: https://en.wikipedia.org/wiki/File_descriptor
//! [line discipline]: https://en.wikipedia.org/wiki/Line_discipline
//! [paste state machine]: super::paste_state_machine::PasteCollectionState
//! [raw mode]: crate::core::ansi::terminal_raw_mode#raw-mode-vs-cooked-mode

// Skip rustfmt for rest of file.
// https://stackoverflow.com/a/75910283/2085356
#![cfg_attr(rustfmt, rustfmt_skip)]

// RRT-based worker and waker types.
#[cfg(any(test, doc))]
pub mod mio_poll_waker;
#[cfg(not(any(test, doc)))]
mod mio_poll_waker;

#[cfg(any(test, doc))]
pub mod mio_poll_worker;
#[cfg(not(any(test, doc)))]
mod mio_poll_worker;

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

#[cfg(any(test, doc))]
pub mod handler_receiver_drop;
#[cfg(not(any(test, doc)))]
mod handler_receiver_drop;

// Re-export public API.
pub use mio_poll_waker::*;
pub use mio_poll_worker::*;
pub use sources::*;

