// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words O_NONBLOCK POLLOUT EINTR SIGCONT SIGPROF EBADF devtty

use std::io::Stdout;

#[cfg(doc)]
/// Make the link to this shorter when used in rustdoc for [`BackpressureStdout`].
use crate::tui::terminal_lib_backends::direct_to_ansi::input::mio_poller::{
    self, handler_stdin::consume_stdin_input_with_sender,
};

/// A cross-platform wrapper around [`stdout`] that provides backpressure handling when
/// the OS terminal output buffer is full.
///
/// # Context
///
/// This struct is the output counterpart to our asynchronous input engine
/// ([`mio_poller`]). While [`mio_poller`] uses [`mio`] to continuously poll [`stdin`] for
/// incoming input events:
///
/// - **On Unix (Linux, macOS):** [`BackpressureStdout`] uses [`rustix::event::poll`] to
///   synchronously wait for write readiness ([`POLLOUT`]) on demand whenever the OS
///   buffer fills up.
///
/// - **On Windows:** It politely yields the thread timeslice via [`yield_now()`] as a
///   fallback.
///
/// # Platform-Specific Behavior and Zero-Overhead Pass-Through
///
/// While [`BackpressureStdout`] is used inside [`OutputDevice`] across all supported
/// operating systems to provide a uniform terminal output interface, its backpressure
/// wait path is primarily exercised on Linux:
///
/// - **Linux ([`DirectToAnsi`] Backend):** [`MioPollWorker`] sets `O_NONBLOCK` on [`stdin`],
///   implicitly converting [`stdout`] into non-blocking mode via the shared Open File
///   Description (OFD). When a frame payload exceeds the 4,096-byte kernel buffer,
///   [`write()`] returns [`WouldBlock`], and [`BackpressureStdout`] puts the thread on the
///   kernel [`PTY`] wait-queue using [`rustix::event::poll`] on [`POLLOUT`].
///
/// - **macOS & Windows ([`Crossterm`] Backend):** Crossterm uses standard blocking stdio and
///   never puts [`stdin`] into non-blocking mode. Consequently, [`stdout`] remains in
///   standard blocking mode natively. Calls to [`write()`] and [`flush()`] succeed
///   immediately or block natively in the OS kernel without ever returning
///   [`WouldBlock`]. On macOS and Windows, this struct acts as a zero-overhead
///   pass-through.
///
/// # Why [`Stdout`] Needs Backpressure Handling on Linux with [`DirectToAnsi`]
///
/// 1. **Problem:** We made [`stdin`] non-blocking on Linux to perform high-performance
///    [edge-triggered polling] with [`mio`] without deadlocking the poller thread (see
///    [Why We Need Non-Blocking Read] and [`original_stdin_flags`] in [`MioPollWorker`]).
///    On Linux, [`stdin`] and [`stdout`] share the exact same underlying Open File
///    Description (OFD) for the controlling terminal ([`/dev/tty`] or `/dev/pts/X`), so
///    [`stdout`] became non-blocking too, which we don't want.
///
/// 2. **Symptom:** When painting a UI frame (a render pass where the application flushes
///    a screen of text, cursor movements, and [`ANSI`] color codes to [`stdout`]) or
///    streaming any large chunk of text (like a massive [`cat`] command), the total byte
///    payload often exceeds the OS's terminal buffer size. On Linux, this is 4,096 bytes
///    ([`n_tty`]'s [`N_TTY_BUF_SIZE`]). When the kernel buffer fills up (`payload_bytes >
///    4,096`), it normally pauses the thread calling [`write()`] (or [`flush()`]) until
///    the terminal emulator drains the buffer. But because [`stdout`] is non-blocking
///    now, Linux returns a "buffer full, try again later" error ([`WouldBlock`]).
///
/// 3. **Solution:** We built this cross-platform struct to wrap [`Stdout`] in order to
///    catch the [`WouldBlock`] error across all operating systems, and react to it. For
///    the architectural rationale comparing continuous input vs. on-demand output
///    polling, see [Architectural Asymmetry: Continuous Input vs. On-Demand Output] in
///    [`mio_poller`] module docs. Here are the approaches we take on different OSes:
///
///    - **On Unix (Linux, macOS):** Instead of context-switching away with
///      [`yield_now()`] (which adds latency), it places the thread on the kernel [`PTY`]
///      wait-queue using [`rustix::event::poll`] on [`POLLOUT`]. The kernel puts the
///      thread into a sleep state (consuming 0 CPU cycles) and wakes it sub-microsecond
///      the exact moment the terminal emulator drains buffer space.
///
///    - **On Windows:** It politely yields the thread timeslice via [`yield_now()`] to
///      allow the terminal emulator time to consume pending output. This is not
///      performant and adds latency.
///
/// # Signal Interrupts ([`EINTR`]) and Fatal Error Handling
///
/// When calling [`rustix::event::poll`], the return value distinguishes between signal
/// interruptions and fatal errors:
///
/// 1. **Why [`EINTR`] occurs:** On Linux, when a thread is sleeping inside a system call
///    like [`rustix::event::poll`], receiving an asynchronous POSIX signal causes the
///    kernel to wake the thread early with the [`EINTR`] error code. Examples of POSIX
///    signals in our application include [`SIGWINCH`] when the terminal window is
///    resized, [`SIGCONT`] when resuming from background job control, or [`SIGPROF`]
///    during profiler sampling.
///
/// 2. **Why we retry on [`EINTR`]:** The [`stdout`] file descriptor is healthy; the
///    thread was merely awakened to deliver the signal. Retrying the poll/write loop
///    ensures seamless frame delivery without dropped bytes.
///
/// 3. **Why fatal OS errors fail fast:** If [`stdout`] is closed or invalid (returning
///    [`EBADF`]), [`rustix::event::poll`] fails immediately. Returning that
///    [`std::io::Error`] ensures the error propagates up cleanly rather than getting
///    stuck in an infinite retry loop.
///
/// [`/dev/tty`]: crate::pty_engine::pty_pair::PtyPair#controlling-terminal-alias-devtty
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
/// [`cat`]: https://en.wikipedia.org/wiki/Cat_(Unix)
/// [`Crossterm`]: crate::tui::TerminalLibBackend::Crossterm
/// [`DirectToAnsi`]: crate::tui::TerminalLibBackend::DirectToAnsi
/// [`drop()`]:
///     crate::tui::terminal_lib_backends::direct_to_ansi::input::mio_poller::MioPollWorker#method.drop
/// [`EBADF`]: https://man7.org/linux/man-pages/man3/errno.3.html
/// [`EINTR`]: https://man7.org/linux/man-pages/man3/errno.3.html
/// [`flush()`]: std::io::Write::flush
/// [`mio_poller`]: mio_poller
/// [`mio`]: mio
/// [`MioPollWorker`]:
///     crate::tui::terminal_lib_backends::direct_to_ansi::input::mio_poller::MioPollWorker
/// [`N_TTY_BUF_SIZE`]: https://github.com/torvalds/linux/blob/master/drivers/tty/n_tty.c
/// [`n_tty`]: https://docs.kernel.org/driver-api/tty/n_tty.html
/// [`original_stdin_flags`]:
///     crate::tui::terminal_lib_backends::direct_to_ansi::input::mio_poller::MioPollWorker::original_stdin_flags
/// [`OutputDevice`]: crate::OutputDevice
/// [`POLLOUT`]: https://man7.org/linux/man-pages/man2/poll.2.html
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
/// [`rustix::event::poll`]: rustix::event::poll
/// [`SIGCONT`]: https://man7.org/linux/man-pages/man7/signal.7.html
/// [`SIGPROF`]: https://man7.org/linux/man-pages/man7/signal.7.html
/// [`SIGWINCH`]: https://man7.org/linux/man-pages/man7/signal.7.html
/// [`stdin`]: std::io::stdin
/// [`Stdout`]: std::io::Stdout
/// [`stdout`]: std::io::stdout
/// [`WouldBlock`]: std::io::ErrorKind::WouldBlock
/// [`write()`]: std::io::Write::write
/// [`yield_now()`]: std::thread::yield_now
/// [Architectural Asymmetry: Continuous Input vs. On-Demand Output]:
///     mio_poller#architectural-asymmetry-continuous-input-vs-on-demand-output
/// [edge-triggered polling]:
///     consume_stdin_input_with_sender#edge-triggered-vs-level-triggered-polling
/// [Why We Need Non-Blocking Read]:
///     consume_stdin_input_with_sender#why-we-need-non-blocking-read
#[derive(Debug)]
pub struct BackpressureStdout(pub Stdout);
