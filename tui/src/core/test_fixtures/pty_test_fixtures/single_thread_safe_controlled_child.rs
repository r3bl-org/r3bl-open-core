// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words errno

use crate::{ControlledChild, ControlledChildTerminationHandle, ControllerReader,
            ControllerWriter, EIO, PtyPair};
use std::io::{BufRead, BufReader, Read};

/// A bundle of [`PTY`] resources passed to integration test controllers.
///
/// This context is automatically prepared by the [`generate_pty_test!`] macro.
///
/// [`generate_pty_test!`]: crate::generate_pty_test
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
#[allow(missing_debug_implementations)]
pub struct PtyTestContext {
    /// The [`PTY`] pair wrapper.
    ///
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    pub pty_pair: PtyPair,

    /// The controlled child process wrapped in a safety guard.
    pub child: SingleThreadSafeControlledChild,

    /// A buffered reader for the [`PTY`] controller side.
    ///
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    pub buf_reader: BufReader<ControllerReader>,

    /// A writer for sending input to the [`PTY`] controller side.
    ///
    /// On Windows, this writer has already performed the mandatory [`ConPTY`] [`DSR`]
    /// handshake.
    ///
    /// [`ConPTY`]:
    ///     https://learn.microsoft.com/en-us/windows/console/creating-a-pseudoconsole-session
    /// [`DSR`]: crate::DsrSequence
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    pub writer: ControllerWriter,
}

/// Result of reading [`PTY`] output lines until a marker.
///
/// [`PTY`]: mod@crate::core::pty
#[derive(Debug)]
pub struct ReadLinesResult {
    /// Collected output lines (normalized, filtered).
    pub lines: Vec<String>,
    /// Whether the marker string was found in the output.
    pub found_marker: bool,
}

/// Wraps [`ControlledChild`] to prevent the [`PTY`] buffer deadlock described in
/// [`drain_and_wait()`].
///
/// Hides [`wait()`] entirely. The only exit path is [`drain_and_wait()`] to drain the
/// buffer before reaping the child.
///
/// # Two Main Reading Patterns
///
/// This wrapper supports two distinct patterns for reading from a [`PTY`] controller:
///
/// 1. Iterative Command-Response ("Synchronize-and-Forget")
///     * Method: [`read_line_state()`]
///     * Goal: Block the controller until the *next* specific state update is received
///       from the child.
///     * Use Case: Typical keyboard input tests where you send a key, wait for the
///       resulting state update, assert it, and move on.
///
/// 2. Bulk Capture & Visual Analysis
///     * Method: [`read_until_marker()`]
///     * Goal: Capture all intermediate output until a final "DONE" marker.
///     * Use Case: Verifying the *entirety* of rendered output, such as checking for
///       extra blank lines or validating column alignment across multiple rows.
///
/// [`drain_and_wait()`]: crate::pty_test_fixtures::drain_and_wait
/// [`PTY`]: crate::core::pty
/// [`wait()`]: portable_pty::Child::wait
#[allow(missing_debug_implementations)]
pub struct SingleThreadSafeControlledChild {
    child: ControlledChild,
}

impl SingleThreadSafeControlledChild {
    #[must_use]
    pub fn new(child: ControlledChild) -> Self { Self { child } }

    /// Returns a handle that can terminate the child process from another thread.
    ///
    /// The [`generate_pty_test!`] macro passes this handle to [`PtyTestWatchdog`], which
    /// terminates the child process if the controller hangs past the timeout.
    ///
    /// [`generate_pty_test!`]: crate::generate_pty_test
    /// [`PtyTestWatchdog`]: crate::PtyTestWatchdog
    #[must_use]
    pub fn clone_termination_handle(&self) -> ControlledChildTerminationHandle {
        self.child.clone_killer()
    }

    /// Delegates to the standalone [`drain_and_wait()`] function,
    /// taking ownership of `self`, `buf_reader`, and `pty_pair` so all [`PTY`] resources
    /// are cleaned up in one call.
    ///
    /// [`PTY`]: crate::core::pty
    pub fn drain_and_wait<R: Read>(
        mut self,
        buf_reader: BufReader<R>,
        pty_pair: PtyPair,
    ) {
        drain_and_wait(buf_reader, pty_pair, &mut self.child);
    }

    /// Wait for controlled process to signal readiness.
    ///
    /// See [`wait_for_ready`] for details.
    ///
    /// # Errors
    ///
    /// Returns an error if EOF/EIO is reached before the ready signal, or on I/O
    /// failure.
    pub fn wait_for_ready<R: BufRead>(
        &self,
        reader: &mut R,
        ready_signal: &str,
    ) -> Result<(), String> {
        wait_for_ready(reader, ready_signal)
    }

    /// Acts as a **blocking filter** and **synchronizer** for [`PTY`] integration tests.
    ///
    /// This follows the **Iterative Command-Response ("Synchronize-and-Forget")**
    /// pattern. It blocks the controller until it finds a line starting with
    /// `line_prefix_marker`, ignoring any intermediate debug output.
    ///
    /// See [`read_line_state`] for details. For bulk capture, see
    /// [`read_until_marker()`].
    ///
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    pub fn read_line_state<R: BufRead>(
        &self,
        buf_reader: &mut R,
        line_prefix_marker: &str,
    ) -> String {
        read_line_state(buf_reader, line_prefix_marker)
    }

    /// Reads lines from a [`PTY`] controller until a marker string is found, capturing
    /// all intermediate output for analysis.
    ///
    /// This follows the **Bulk Capture & Visual Analysis** pattern. Unlike
    /// [`read_line_state()`], which is a synchronizer for command-response tests, this
    /// method collects every line seen into a [`ReadLinesResult`].
    ///
    /// Use this when your test needs to verify the **entirety** or **sequence** of
    /// rendered output, such as:
    /// - Verifying no extra blank lines appear between log outputs.
    /// - Checking column alignment across multi-line messages.
    /// - Inspecting the full visual state of a virtual terminal buffer.
    ///
    /// See [`read_until_marker`] for implementation details. For iterative
    /// synchronization, see [`read_line_state()`].
    ///
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    pub fn read_until_marker<R: BufRead>(
        &self,
        buf_reader: &mut R,
        marker: &str,
        line_filter: impl Fn(&str) -> bool,
    ) -> ReadLinesResult {
        let (lines, found_marker) = read_until_marker(buf_reader, marker, &line_filter);
        ReadLinesResult {
            lines,
            found_marker,
        }
    }

    /// Obtains a writer for sending input to the controlled process, performing any
    /// necessary platform handshakes (like [`ConPTY`] [`DSR`] on Windows) automatically.
    ///
    /// # Platform Behavior
    ///
    /// - **Windows**: Performs the mandatory [`ConPTY`] [`DSR`] handshake before
    ///   returning the writer. This requires reading from the provided `reader` until the
    ///   [`DSR_CURSOR_POSITION_REQUEST`] is found.
    /// - **Other OSes**: Simply takes the writer from the [`PtyPair`] and returns it
    ///   immediately. This is a zero-cost operation on Unix/macOS.
    ///
    /// # Important
    ///
    /// The returned writer **must be kept alive** for the duration of the test. On
    /// Windows, dropping this writer closes the [`ConPTY`] session and stops all
    /// [`stdout`] forwarding, which will cause tests to hang or fail prematurely.
    ///
    /// # Panics
    ///
    /// Panics if taking the writer from the [`PtyPair`] fails.
    ///
    /// [`ConPTY`]:
    ///     https://learn.microsoft.com/en-us/windows/console/creating-a-pseudoconsole-session
    /// [`DSR_CURSOR_POSITION_REQUEST`]: crate::DSR_CURSOR_POSITION_REQUEST
    /// [`DSR`]: crate::DSR_CURSOR_POSITION_REQUEST
    /// [`OpenConsole.exe`]: https://github.com/microsoft/terminal/tree/main/src/host
    /// [`stdout`]: std::io::Stdout
    pub fn get_writer_with_handshake<R: Read>(
        &self,
        pty_pair: &PtyPair,
        reader: &mut R,
    ) -> ControllerWriter {
        #[cfg(target_os = "windows")]
        {
            use std::io::Write;
            let mut writer = pty_pair
                .controller()
                .take_writer()
                .expect("Failed to take writer");
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(n) if n > 0 => {
                        let req = crate::DSR_CURSOR_POSITION_REQUEST.as_bytes();
                        if buf[..n].windows(req.len()).any(|w| w == req) {
                            let _unused = writer.write_all(b"\x1b[1;1R");
                            let _unused = writer.flush();
                            break;
                        }
                    }
                    _ => break,
                }
            }
            Box::new(writer)
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = reader;
            let writer = pty_pair
                .controller()
                .take_writer()
                .expect("Failed to take writer");
            Box::new(writer)
        }
    }
}

/// Wait for controlled process to signal readiness.
///
/// The controlled process sends a ready signal (e.g., `CONTROLLED_READY\n`) before
/// enabling raw mode.
///
/// This function reads from the given [`BufRead`] until it finds the `ready_signal`.
/// Because it uses a buffered reader, any data arriving *after* the signal (due to OS
/// batching) is preserved in the reader's internal buffer and will be available for
/// subsequent read calls.
///
/// # Errors
///
/// Returns an error message if [`EOF`]/[`EIO`] is reached before the ready signal is
/// found, or if an I/O error occurs.
///
/// [`EIO`]: https://man7.org/linux/man-pages/man3/errno.3.html
/// [`EOF`]: https://en.wikipedia.org/wiki/End-of-file
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
pub fn wait_for_ready<R: BufRead>(
    reader: &mut R,
    ready_signal: &str,
) -> Result<(), String> {
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => return Err(format!("EOF before ready signal '{ready_signal}'")),
            Ok(_) => {
                if line.contains(ready_signal) {
                    return Ok(());
                }
            }
            Err(e) => {
                // EIO (errno 5) is how Linux signals that the controlled side closed.
                if e.raw_os_error() == Some(EIO) {
                    return Err(format!(
                        "EOF (EIO) before ready signal '{ready_signal}'"
                    ));
                }
                return Err(format!("Read error waiting for ready: {e}"));
            }
        }
    }
}

/// Acts as a **blocking filter** and **synchronizer** for [`PTY`] integration tests.
///
/// This follows the **Iterative Command-Response ("Synchronize-and-Forget")** pattern.
///
/// In a [`PTY`] test, the **controlled** process's [`stdout`] and [`stderr`] are merged
/// into a single stream. This function reads this stream line-by-line and performs three
/// critical roles:
///
/// 1. **Filtering**: It distinguishes between **Protocol Messages** (official state
///    reports starting with `line_prefix_marker`) and **Debug Output** (informational
///    tracing like `🔍 ...`). If a line doesn't start with `line_prefix_marker`, it's
///    treated as debug info, printed as a warning, and ignored.
/// 2. **Synchronization**: It blocks the controller until it finds a line starting with
///    `line_prefix_marker`. This effectively waits for the controlled process to finish
///    its current task and report its new state before the controller makes any
///    assertions.
/// 3. **Validation**: It panics on [`EOF`] or read errors. If the stream ends before a
///    state update is received, it usually indicates the controlled process crashed or
///    exited prematurely.
///
/// For bulk capture of intermediate output, see [`read_until_marker()`].
///
/// # Panics
///
/// Panics on EOF or I/O read errors.
///
/// [`EOF`]: https://en.wikipedia.org/wiki/End-of-file
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
/// [`stderr`]: std::io::Stderr
/// [`stdout`]: std::io::Stdout
pub fn read_line_state<R: BufRead>(
    buf_reader: &mut R,
    line_prefix_marker: &str,
) -> String {
    loop {
        let mut line = String::new();
        match buf_reader.read_line(&mut line) {
            Ok(0) => panic!("EOF reached before getting line state"),
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.starts_with(line_prefix_marker) {
                    return trimmed.to_string();
                }
                eprintln!("  ⚠️  Skipping: {trimmed}");
            }
            Err(e) => panic!("Read error: {e}"),
        }
    }
}

/// Drains the [`PTY`] until [`EIO`] or [`EOF`], then waits for the child process to exit.
/// Prevents deadlocks caused by unread [`PTY`] buffer data.
///
/// # Problem
///
/// See the [Two Types of Deadlocks] section in [`PtyPair`] for how this function handles
/// the secondary **buffer-full deadlock** that occurs in **contrived single-threaded
/// tests**.
///
/// This function solves a [`PTY`] buffer deadlock that occurs on macOS (and occasionally
/// on Linux) when a controlled process writes to stderr after the controller has stopped
/// reading. The sequence that causes the deadlock:
///
/// 1. Controller reads [`PTY`] until a marker (e.g., `SUCCESS`, `CONTROLLED_DONE`).
/// 2. Controller stops reading and calls [`ControlledChild::wait()`].
/// 3. Child writes more [`eprintln!()`] after the marker, then calls
///    [`std::process::exit(0)`].
/// 4. [`std::process::exit(0)`] flushes [`stdio`], which **blocks** because the [`PTY`]
///    buffer is full (nobody is reading the controller side).
/// 5. Deadlock happens: controller waits for child, child waits for buffer space.
///
/// macOS [`PTY`] buffers are ~1 KB (vs ~4 KB on Linux), making this trigger frequently.
///
/// # Solution
///
/// 1. **Drop `pty_pair`** — closes the parent's controller [`fd`]. The `buf_reader`'s
///    cloned controller [`fd`] remains valid. The controlled [`fd`] must already be
///    closed by the caller (via [`PtyPair::open_and_spawn()`]) before the controller's
///    reading phase begins; that ensures [`EIO`] (or [`EOF`] on some platforms) arrives
///    when the child process exits rather than only when [`drain_and_wait()`] is reached.
/// 2. **Drain `buf_reader` until [`EIO`] or [`EOF`]** — unblocks the child's
///    [`std::process::exit(0)`] flush. Once the child process exits and its controlled
///    [`fd`]s close, the controller gets [`EIO`] on Linux (or [`EOF`] on some platforms).
/// 3. **[`child.wait()`]** — the child has already exited, so this reaps the zombie
///    immediately.
///
/// # Platform behavior: POSIX [`EOF`] vs Linux [`EIO`]
///
/// When the controlled side is fully closed and the child process exits, the controller's
/// blocking [`read()`] returns different signals depending on the platform:
///
/// - **Linux** returns [`EIO`] (`errno` `5`) -- a Linux-specific kernel behavior where
///   the [`PTY`] controller signals that the controlled side has no remaining open
///   [`fd`]s.
/// - **BSD and other platforms** return a traditional [`EOF`] (blocking [`read()`]
///   returns `0` bytes).
///
/// This function handles both signals (see `Ok(0)` and the [`EIO`] check in the drain
/// loop below). Any code that reads from a [`PTY`] controller must handle both to be
/// cross-platform correct.
///
/// # Arguments
///
/// - `buf_reader` - The buffered reader wrapping a cloned controller reader. Must be the
///   same reader used during the test's reading phase (so buffered data is consumed).
/// - `pty_pair` - The [`PTY`] pair to drop. The controlled side should already have been
///   closed via [`PtyPair::open_and_spawn()`] before the controller's reading phase
///   started.
/// - `child` - The controlled child process to wait on.
///
/// # Panics
///
/// Panics if [`child.wait()`] fails.
///
/// [`child.wait()`]: portable_pty::Child::wait
/// [`ControlledChild::wait()`]: portable_pty::Child::wait
/// [`EIO`]: https://man7.org/linux/man-pages/man3/errno.3.html
/// [`EOF`]: https://en.wikipedia.org/wiki/End-of-file
/// [`fd`]: https://man7.org/linux/man-pages/2/open.2.html
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
/// [`PtyPair::open_and_spawn()`]: crate::PtyPair::open_and_spawn
/// [`PtyPair`]: crate::PtyPair
/// [`read()`]: https://man7.org/linux/man-pages/man2/read.2.html
/// [`std::process::exit(0)`]: std::process::exit
/// [`stdio`]: std::io
/// [Two Types of Deadlocks]: crate::PtyPair#two-types-of-pty-deadlocks
#[allow(clippy::needless_continue)]
pub fn drain_and_wait<R: Read>(
    mut buf_reader: BufReader<R>,
    pty_pair: PtyPair,
    child: &mut ControlledChild,
) {
    // Step 1: Drop pty_pair to release the controller side's main handle.
    // The parent's copy of the controlled fd is already closed (via
    // PtyPair::open_and_spawn), which is the primary resource leak deadlock
    // safeguard. Dropping the pair here ensures the parent is in a clean state
    // where only the buf_reader (a clone) is actively reading.
    drop(pty_pair);

    // Step 2: Drain buf_reader until EIO or EOF. This prevents the termination
    // deadlock where the child's exit() flush blocks on a full PTY buffer.
    // Once the child process's own copies of the controlled fd close, the
    // controller receives EIO on Linux (or EOF on some platforms) here.
    let mut discard_buf = [0u8; 1024];
    loop {
        match buf_reader.read(&mut discard_buf) {
            Ok(0) => break,    // EOF — some platforms signal closure this way.
            Ok(_) => continue, // Discard remaining output.
            Err(e) => {
                // EIO (errno 5) is how Linux signals that the controlled side closed.
                // See: https://lists.archive.carbon60.com/linux/kernel/1790583
                if e.raw_os_error() == Some(EIO) {
                    break;
                }
                // Other errors are unexpected but not fatal — the child may have
                // already exited.
                eprintln!("drain_and_wait: read error during drain: {e}");
                break;
            }
        }
    }

    // Step 3: Reap the child process. It has already exited, so this returns
    // immediately.
    match child.wait() {
        Ok(status) => {
            eprintln!("✅ drain_and_wait: child exited: {status:?}");
        }
        Err(e) => {
            panic!("drain_and_wait: failed to wait for child: {e}");
        }
    }
}

/// Reads from a [`PTY`] controller until a marker string is found, capturing all
/// intermediate output into a vector.
///
/// This follows the **Bulk Capture & Visual Analysis** pattern.
///
/// This is a lower-level implementation for the method
/// [`SingleThreadSafeControlledChild::read_until_marker()`].
///
/// For iterative synchronization, see [`read_line_state()`].
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
pub fn read_until_marker(
    buf_reader: &mut impl std::io::BufRead,
    marker: &str,
    line_filter: &impl Fn(&str) -> bool,
) -> (Vec<String>, bool) {
    let mut lines = Vec::new();
    let mut found_marker = false;

    loop {
        let mut line = String::new();
        match buf_reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {
                let trimmed = normalize_pty_line(&line);
                eprintln!("  <- Controlled output: {trimmed:?}");

                if trimmed.contains(marker) {
                    found_marker = true;
                    break;
                }

                if !trimmed.is_empty() && line_filter(&trimmed) {
                    lines.push(trimmed);
                }
            }
            Err(e) => {
                eprintln!("read_until_marker: read error: {e}");
                break;
            }
        }
    }

    (lines, found_marker)
}

/// Normalizes a [`PTY`] output line by stripping [`ANSI`] escape sequences and carriage
/// returns.
///
/// [`ConPTY`] on Windows injects [`ANSI`] escape sequences (cursor movement, color codes)
/// and `\r\n` line endings into child output. This helper produces a clean,
/// platform-agnostic string for assertion comparisons.
///
/// # Steps
///
/// 1. Strip [`ANSI`] escape sequences via [`strip_ansi_escapes::strip_str`].
/// 2. Normalize `\r\n` → `\n` and remove stray `\r`.
/// 3. Trim leading/trailing whitespace.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
/// [`ConPTY`]:
///     https://learn.microsoft.com/en-us/windows/console/creating-a-pseudoconsole-session
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
#[must_use]
pub fn normalize_pty_line(line: &str) -> String {
    let stripped = strip_ansi_escapes::strip_str(line);
    stripped
        .replace("\r\n", "\n")
        .replace('\r', "")
        .trim()
        .to_string()
}
