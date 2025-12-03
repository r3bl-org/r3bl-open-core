// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words EINTR

//! Dedicated stdin reader thread using [`std::io::stdin()`].
//!
//! **We do NOT use [`tokio::io::stdin()`]** - it has fundamental issues with
//! cancellation in [`tokio::select!`]. Instead, we spawn a dedicated thread that
//! performs **blocking** reads using [`std::io::stdin()`] from the standard library,
//! and sends results through a [`tokio::sync::mpsc`] channel.
//!
//! # Lifecycle
//!
//! This thread can't be terminated or cancelled, so it safely owns stdin exclusively. The
//! OS is responsible for cleaning it up when the process exits. When the main function
//! exits, the OS will automatically clean up all threads and file descriptors.
//!
//! | Exit Mechanism             | How Thread Exits                             |
//! | -------------------------- | -------------------------------------------- |
//! | Ctrl+C / `SIGINT`          | OS terminates process → all threads killed   |
//! | [`std::process::exit()`]   | OS terminates process → all threads killed   |
//! | `main()` returns           | Rust runtime exits → OS terminates process   |
//! | `stdin` EOF                | `read()` returns 0 → thread exits naturally  |
//!
//! This is ok because:
//! - [`GLOBAL_INPUT_CORE`] lives forever - it's a [`LazyLock`]`<...>` static, never
//!   dropped until process exit.
//! - Thread is doing nothing - blocked on read, not consuming CPU
//! - No resources to leak - stdin is fd 0, not owned by us
//! - This matches [`crossterm`] - they also rely on process exit for cleanup
//!
//! # The Problem with [`tokio::io::stdin()`]
//!
//! [`tokio::io::stdin()`] is **not truly async** - it spawns blocking reads on Tokio's
//! blocking thread pool. When used in [`tokio::select!`], if another branch wins (e.g.,
//! `SIGWINCH` arrives), the stdin read is "cancelled" but the blocking thread continues
//! running. The next call to `stdin.read()` then conflicts with the still-running
//! background read, causing undefined behavior.
//!
//! ```text
//! BROKEN PATTERN:
//! ┌──────────────────────────────────────────────────────────────────────┐
//! │ tokio::select! {                                                     │
//! │   result = tokio_stdin.read() => { ... }  // Spawns blocking thread! │
//! │   signal = sigwinch.recv() => { ... }     // True async ✓            │
//! │ }                                                                    │
//! └──────────────────────────────────────────────────────────────────────┘
//!
//! When `SIGWINCH` wins:
//!   1. [`tokio::select!`] "cancels" `tokio_stdin.read()` future
//!   2. BUT the blocking thread keeps running in the background
//!   3. Next `tokio_stdin.read()` → undefined behavior
//!      (two threads reading `tokio_stdin`!)
//! ```
//!
//! # The Solution: Dedicated Thread with Channel
//!
//! **We use [`std::io::stdin()`]** (NOT [`tokio::io::stdin()`]) in a dedicated thread.
//! This thread performs blocking reads and sends results through a [`tokio::sync::mpsc`]
//! channel. The async side receives from this channel, which is truly async and
//! properly cancel-safe.
//!
//!
//! ```text
//! FIXED PATTERN:
//! ┌─────────────────────────┐       ┌─────────────────────────────────┐
//! │ Dedicated Thread        │       │ Async Task                      │
//! │ (std::thread::spawn)    │       │                                 │
//! │                         │       │ tokio::select! {                │
//! │ loop {                  │──────▶│   bytes = rx.recv() => { ... }  │
//! │   stdin.read_blocking() │ mpsc  │   signal = sigwinch => { ... }  │
//! │   tx.send(bytes)        │       │ }                               │
//! │ }                       │       │                                 │
//! └─────────────────────────┘       └─────────────────────────────────┘
//!
//! When SIGWINCH wins:
//!   1. select! cancels rx.recv() future
//!   2. Thread continues reading, but that's fine - it owns stdin exclusively
//!   3. Next rx.recv() gets the data the thread read
//!   4. No undefined behavior! ✓
//! ```
//!
//! # Why This Works
//!
//! - **Single owner**: Only one thread ever reads from stdin
//! - **True cancel safety**: Channel receive is truly async, not blocking
//! - **No data loss**: Data read by the thread waits in the channel
//! - **Process lifetime**: Thread runs until process exits (stdin EOF)
//!
//! # Thread Cleanup
//!
//! The dedicated thread self-terminates gracefully in these scenarios:
//!
//! 1. **EOF on stdin**: When stdin is closed (e.g., pipe closed, Ctrl+D), `read()`
//!    returns 0 bytes. The thread sends [`StdinReadResult::Eof`] and exits.
//!
//! 2. **I/O error**: On read errors (except `EINTR` which is retried), the thread sends
//!    [`StdinReadResult::Error`] and exits.
//!
//! 3. **Receiver dropped**: When [`GLOBAL_INPUT_CORE`] is dropped (process exit), the
//!    channel receiver is dropped. The next `tx.send()` returns `Err`, and the thread
//!    exits gracefully.
//!
//! 4. **Process exit**: When the process terminates, the OS automatically cleans up all
//!    threads and file descriptors. No explicit cleanup needed.
//!
//! **No resource leaks**: The thread doesn't own any resources that need explicit
//! cleanup. `std::io::stdin()` is just a handle to fd 0, not a new file descriptor.
//!
//! [`GLOBAL_INPUT_CORE`]: super::singleton::GLOBAL_INPUT_CORE
//! [`LazyLock`]: std::sync::LazyLock
//! [`crossterm`]: ::crossterm
//! [`std::io::stdin()`]: std::io::stdin
//! [`std::process::exit()`]: std::process::exit
//! [`tokio::io::stdin()`]: tokio::io::stdin
//! [`tokio::select!`]: tokio::select
//! [`tokio::sync::mpsc`]: tokio::sync::mpsc

use super::buffer::STDIN_READ_BUFFER_SIZE;
use crate::tui::DEBUG_TUI_SHOW_TERMINAL_BACKEND;
use std::io::Read as _;

/// Result of a stdin read operation, sent through the channel.
#[derive(Debug)]
pub enum StdinReadResult {
    /// Successfully read bytes from stdin.
    Data(Vec<u8>),
    /// EOF reached (0 bytes read).
    Eof,
    /// Error occurred during read.
    Error(std::io::ErrorKind),
}

/// Sender end of the stdin channel, held by the reader thread.
pub type StdinSender = tokio::sync::mpsc::UnboundedSender<StdinReadResult>;

/// Receiver end of the stdin channel, used by the async input device.
pub type StdinReceiver = tokio::sync::mpsc::UnboundedReceiver<StdinReadResult>;

/// Creates a channel and spawns the dedicated stdin reader thread.
///
/// # Returns
///
/// The receiver end of the channel. The sender is moved into the spawned thread.
///
/// # Thread Lifetime
///
/// The thread runs until:
/// - stdin reaches EOF (returns `StdinReadResult::Eof`)
/// - An I/O error occurs (returns `StdinReadResult::Error`)
/// - The receiver is dropped (send fails, thread exits gracefully)
///
/// Since the receiver is stored in [`GLOBAL_INPUT_CORE`], the thread effectively
/// runs for the process lifetime.
///
/// [`GLOBAL_INPUT_CORE`]: super::singleton::GLOBAL_INPUT_CORE
pub fn spawn_stdin_reader_thread() -> StdinReceiver {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

    std::thread::Builder::new()
        .name("stdin-reader".into())
        .spawn(move || stdin_reader_loop(tx))
        .expect("Failed to spawn stdin reader thread");

    rx
}

/// The main loop of the stdin reader thread.
///
/// Continuously reads from stdin and sends results through the channel until:
/// - EOF is reached
/// - An error occurs
/// - The channel receiver is dropped
fn stdin_reader_loop(tx: StdinSender) {
    let mut stdin = std::io::stdin().lock();
    let mut buffer = [0u8; STDIN_READ_BUFFER_SIZE];

    loop {
        match stdin.read(&mut buffer) {
            Ok(0) => {
                // EOF reached.
                DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                    tracing::debug!(message = "stdin-reader-thread: EOF (0 bytes)");
                });
                drop(tx.send(StdinReadResult::Eof));
                break;
            }
            Ok(n) => {
                // Successfully read n bytes.
                DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                    tracing::debug!(
                        message = "stdin-reader-thread: read bytes",
                        bytes_read = n
                    );
                });
                let data = buffer[..n].to_vec();
                if tx.send(StdinReadResult::Data(data)).is_err() {
                    // Receiver dropped - exit gracefully.
                    DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                        tracing::debug!(
                            message = "stdin-reader-thread: receiver dropped, exiting"
                        );
                    });
                    break;
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => {
                // EINTR - retry immediately (loop continues).
            }
            Err(e) => {
                // Other error - send and exit.
                DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                    tracing::debug!(
                        message = "stdin-reader-thread: error",
                        error = ?e
                    );
                });
                drop(tx.send(StdinReadResult::Error(e.kind())));
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stdin_read_result_debug() {
        // Verify Debug trait is implemented correctly.
        let data_result = StdinReadResult::Data(vec![0x1B, 0x5B, 0x41]);
        let debug_str = format!("{:?}", data_result);
        assert!(debug_str.contains("Data"));

        let eof_result = StdinReadResult::Eof;
        let debug_str = format!("{:?}", eof_result);
        assert!(debug_str.contains("Eof"));

        let error_result = StdinReadResult::Error(std::io::ErrorKind::WouldBlock);
        let debug_str = format!("{:?}", error_result);
        assert!(debug_str.contains("Error"));
    }
}
