// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words errno

use super::buf_read_ext::BufReadExt;
use crate::{ControlledChild, ControlledChildTerminationHandle, ControllerWriter, EIO,
            GLYPH_CONTROLLED, GLYPH_SUCCESS, GLYPH_WARNING, PtyPair, ReadLinesResult, ok};
use std::io::{BufRead, BufReader, Read};

/// Wraps [`ControlledChild`] to prevent the [`PTY`] buffer deadlock that occurs when the
/// **controller process** waits for the **controlled process** to exit while the [`PTY`]
/// buffer is full.
///
/// Use this struct instead of [`ControlledChild`] directly (in [`PTY`] tests) as it
/// removes [`wait()`] entirely, leaving the only exit path via
/// [`Self::drain_and_wait()`], which drains the buffer before reaping the child. This is
/// the mechanism that prevents the deadlock. This struct is typically not used directly
/// in tests; instead, it is automatically prepared by the [`generate_pty_test!`] macro.
///
/// # Typical [`PTY`] Test Lifecycle (runs in the controller process)
///
/// The **controlled process** (the child) is the "program under test." It is usually a
/// bare function or a simple event loop that just reads from [`stdin`] and writes to
/// [`stdout`]. It doesn't know it's in a test, and it certainly doesn't know about the
/// deadlock safety guards in place.
///
/// A standard integration test using [`generate_pty_test!`] follows these phases, all of
/// which **run in the controller process**:
///
/// 1. **Startup**: The **controller process** calls [`Self::wait_for_ready()`] to ensure
///    the **controlled process** is initialized. Then, it calls
///    [`Self::get_writer_with_handshake()`] to obtain a writer (handles Windows-specific
///    [`PTY`] initialization automatically).
/// 2. **Interaction**: The **controller process** uses one of the reading patterns below
///    to send input to the **controlled process** and verify its output.
/// 3. **Cleanup**: **MANDATORY**. The **controller process** calls
///    [`Self::drain_and_wait()`] to consume all remaining output from the **controlled
///    process** and reap it. Failure to do this will cause [`PTY`] buffer deadlocks.
///
/// # Two Main Reading Patterns (runs in the controller process)
///
/// The **controller process** uses this wrapper to read from the **controlled process**
/// in two distinct ways:
///
/// 1. Iterative Command-Response ("Wait-for-State")
///     - Method: [`Self::read_line_state()`]
///     - Goal: The **controller process** blocks until the *next* specific state update
///       is received from the **controlled process**.
///     - Use Case: Typical keyboard input tests where you send a key, wait for the
///       resulting state update, assert it, and move on.
///
/// 2. Bulk Capture & Visual Analysis
///     - Method: [`Self::read_until_marker()`]
///     - Goal: The **controller process** captures all intermediate output until a final
///       "DONE" marker is found in the **controlled process's** output.
///     - Use Case: Verifying the *entirety* of rendered output, such as checking for
///       extra blank lines or validating column alignment across multiple rows.
///
/// # Real-world Examples
///
/// For complete, runnable implementations, see:
/// - [`pty_ctrl_w_test.rs`]: Standard command-response test for readline.
/// - [`pty_terminal_events_test.rs`]: Detailed input event parsing and synchronization.
///
/// [`generate_pty_test!`]: crate::generate_pty_test
/// [`pty_ctrl_w_test.rs`]: mod@crate::readline_async::readline_async_integration_tests::pty_ctrl_w_test
/// [`pty_terminal_events_test.rs`]:
///     mod@crate::vt_100_terminal_input_parser::vt_100_parser_integration_tests::pty_terminal_events_test
/// [`PTY`]: crate::core::pty
/// [`stdin`]: std::io::Stdin
/// [`stdout`]: std::io::Stdout
/// [`wait()`]: portable_pty::Child::wait
#[allow(missing_debug_implementations)]
pub struct PtyTestChild {
    child: ControlledChild,
}

impl PtyTestChild {
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
        self.child.clone_termination_handle()
    }

    /// Drains the [`PTY`] until [`EIO`] or [`EOF`], then waits for the child process to
    /// exit. Prevents deadlocks caused by unread [`PTY`] buffer data.
    ///
    /// # Problem
    ///
    /// See the [Two Types of Deadlocks] section in [`PtyPair`] for how this function
    /// handles the secondary **buffer-full deadlock** that occurs in **contrived
    /// single-threaded tests**.
    ///
    /// This function solves a [`PTY`] buffer deadlock that occurs on macOS (and
    /// occasionally on Linux) when a controlled process writes to stderr after the
    /// controller has stopped reading. The sequence that causes the deadlock:
    ///
    /// 1. Controller reads [`PTY`] until a marker (e.g., `SUCCESS`, `CONTROLLED_DONE`).
    /// 2. Controller stops reading and calls [`ControlledChild::wait()`].
    /// 3. Child writes more [`eprintln!()`] after the marker, then calls
    ///    [`std::process::exit(0)`].
    /// 4. [`std::process::exit(0)`] flushes [`stdio`], which **blocks** because the
    ///    [`PTY`] buffer is full (nobody is reading the controller side).
    /// 5. Deadlock happens: controller waits for child, child waits for buffer space.
    ///
    /// macOS [`PTY`] buffers are ~1 KiB (vs ~4 KiB on Linux), making this trigger
    /// frequently.
    ///
    /// # Solution
    ///
    /// 1. **Drop `pty_pair`**: closes the parent's controller [`fd`]. The `buf_reader`'s
    ///    cloned controller [`fd`] remains valid. The controlled [`fd`] must already be
    ///    closed by the caller (via [`PtyPair::open_and_spawn()`]) before the
    ///    controller's reading phase begins; that ensures [`EIO`] (or [`EOF`] on some
    ///    platforms) arrives when the child process exits rather than only when
    ///    [`Self::drain_and_wait()`] is reached.
    /// 2. **Drain `buf_reader` until [`EIO`] or [`EOF`]**: unblocks the child's
    ///    [`std::process::exit(0)`] flush. Once the child process exits and its
    ///    controlled [`fd`]s close, the controller gets [`EIO`] on Linux (or [`EOF`] on
    ///    some platforms).
    /// 3. **[`child.wait()`]**: the child has already exited, so this reaps the zombie
    ///    immediately.
    ///
    /// # Platform behavior: POSIX [`EOF`] vs Linux [`EIO`]
    ///
    /// When the controlled side is fully closed and the child process exits, the
    /// controller's blocking [`read()`] returns different signals depending on the
    /// platform:
    ///
    /// - **Linux** returns [`EIO`] (`errno` `5`) -- a Linux-specific kernel behavior
    ///   where the [`PTY`] controller signals that the controlled side has no remaining
    ///   open [`fd`]s.
    /// - **BSD and other platforms** return a traditional [`EOF`] (blocking [`read()`]
    ///   returns `0` bytes).
    ///
    /// This function handles both signals (see `Ok(0)` and the [`EIO`] check in the drain
    /// loop below). Any code that reads from a [`PTY`] controller must handle both to be
    /// cross-platform correct.
    ///
    /// # Arguments
    ///
    /// - `buf_reader` - The buffered reader wrapping a cloned controller reader. Must be
    ///   the same reader used during the test's reading phase (so buffered data is
    ///   consumed).
    /// - `pty_pair` - The [`PTY`] pair to drop. The controlled side should already have
    ///   been closed via [`PtyPair::open_and_spawn()`] before the controller's reading
    ///   phase started.
    ///
    /// # Panics
    ///
    /// Panics if [`child.wait()`] fails.
    ///
    /// [`child.wait()`]: portable_pty::Child::wait
    /// [`ControlledChild::wait()`]: portable_pty::Child::wait
    /// [`EIO`]: https://man7.org/linux/man-pages/man3/errno.3.html
    /// [`EOF`]: https://en.wikipedia.org/wiki/End-of-file
    /// [`fd`]: https://man7.org/linux/man-pages/man2/open.2.html
    /// [`PTY`]: crate::core::pty::pty_engine::pty_pair#what-is-a-pty
    /// [`PtyPair::open_and_spawn()`]: crate::PtyPair::open_and_spawn
    /// [`PtyPair`]: crate::PtyPair
    /// [`read()`]: https://man7.org/linux/man-pages/man2/read.2.html
    /// [`std::process::exit(0)`]: std::process::exit
    /// [`stdio`]: std::io
    /// [Two Types of Deadlocks]: crate::PtyPair#two-types-of-pty-deadlocks
    #[allow(clippy::needless_continue)]
    pub fn drain_and_wait<R: Read>(
        mut self,
        mut buf_reader: BufReader<R>,
        pty_pair: PtyPair,
    ) -> u32 {
        // Step 1: Drop pty_pair to release the controller side's main handle. The
        // parent's copy of the controlled fd is already closed (via
        // PtyPair::open_and_spawn), which is the primary resource leak deadlock
        // safeguard. Dropping the pair here ensures the parent is in a clean state where
        // only the buf_reader (a clone) is actively reading.
        drop(pty_pair);

        // Step 2: Drain buf_reader until EIO or EOF. This prevents the termination
        // deadlock where the child's exit() flush blocks on a full PTY buffer. Once the
        // child process's own copies of the controlled fd close, the controller receives
        // EIO on Linux (or EOF on some platforms) here.
        let mut discard_buf = [0u8; 1024];
        loop {
            match buf_reader.read(&mut discard_buf) {
                Ok(0) => break,    // EOF — some platforms signal closure this way.
                Ok(_) => continue, // Discard remaining output.
                Err(e) => {
                    // EIO (errno 5) is how Linux signals that the controlled side closed.
                    // See:
                    // https://unix.stackexchange.com/questions/538198/why-blocking-read-on-a-pty-returns-when-process-on-the-other-end-dies
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
        match self.child.wait() {
            Ok(status) => {
                eprintln!("{GLYPH_SUCCESS} drain_and_wait: child exited: {status:?}");
                status.exit_code()
            }
            Err(e) => {
                panic!("drain_and_wait: failed to wait for child: {e}");
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
    /// [`PTY`]: crate::core::pty::pty_engine::pty_pair#what-is-a-pty
    pub fn wait_for_ready<R: BufRead>(
        &self,
        reader: &mut R,
        ready_signal: &str,
    ) -> Result<(), String> {
        loop {
            let mut line = String::new();
            match reader.read_line_eio_to_eof(&mut line) {
                Ok(0) => {
                    return Err(format!(
                        "EOF reached before ready signal '{ready_signal}'"
                    ));
                }
                Ok(_) => {
                    if line.contains(ready_signal) {
                        return ok!();
                    }
                }
                Err(e) => {
                    return Err(format!("Read error waiting for ready: {e}"));
                }
            }
        }
    }

    /// Acts as a **blocking filter** and **synchronizer** for [`PTY`] integration tests.
    ///
    /// This follows the **Iterative Command-Response ("Wait-for-State")** pattern.
    ///
    /// In a [`PTY`] test, the **controlled** process's [`stdout`] and [`stderr`] are
    /// merged into a single stream. This function reads this stream line-by-line and
    /// performs three critical roles:
    ///
    /// 1. **Filtering**: It distinguishes between **Protocol Messages** (official state
    ///    reports that satisfy the `predicate_fn`) and **Debug Output** (informational
    ///    tracing like `🔍 ...`). If a line doesn't satisfy the `predicate_fn` function,
    ///    it's treated as debug info, printed as a warning, and ignored.
    /// 2. **Synchronization**: It blocks the controller until it finds a line satisfying
    ///    the `predicate_fn`. This effectively waits for the controlled process to finish
    ///    its current task and report its new state before the controller makes any
    ///    assertions.
    /// 3. **Validation**: It panics on [`EOF`] or read errors. If the stream ends before
    ///    a state update is received, it usually indicates the controlled process crashed
    ///    or exited prematurely.
    ///
    /// For bulk capture of intermediate output, see [`Self::read_until_marker()`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use r3bl_tui::PtyTestContext;
    /// use std::io::Write;
    ///
    /// fn main() {
    ///     let mut context: PtyTestContext = todo!();
    ///
    ///     // Send a key press to the child process.
    ///     writeln!(context.writer, "a").expect("conversion error");
    ///
    ///     // Block until the child reports its line state contains "a".
    ///     let state = context.child.read_line_state(&mut context.buf_reader, |line| {
    ///         line.contains("Line: a")
    ///     });
    ///     assert_eq!(state, "Line: a");
    /// }
    /// ```
    ///
    /// # Panics
    ///
    /// Panics on [`EOF`] or I/O read errors.
    ///
    /// [`EOF`]: https://en.wikipedia.org/wiki/End-of-file
    /// [`PTY`]: crate::core::pty::pty_engine::pty_pair#what-is-a-pty
    /// [`stderr`]: std::io::Stderr
    /// [`stdout`]: std::io::Stdout
    pub fn read_line_state<R: BufRead>(
        &self,
        buf_reader: &mut R,
        mut predicate_fn: impl FnMut(&str) -> bool,
    ) -> String {
        loop {
            let mut line = String::new();
            match buf_reader.read_line_eio_to_eof(&mut line) {
                Ok(0) => panic!("EOF reached before getting line state"),
                Ok(_) => {
                    let trimmed = line.trim();
                    if predicate_fn(trimmed) {
                        return trimmed.to_string();
                    }
                    eprintln!("  {GLYPH_WARNING} Skipping: {trimmed}");
                }
                Err(e) => panic!("Read error: {e}"),
            }
        }
    }

    /// Reads lines from a [`PTY`] controller until a marker string is found, capturing
    /// all intermediate output for analysis.
    ///
    /// This follows the **Bulk Capture & Visual Analysis** pattern. Unlike
    /// [`Self::read_line_state()`], which is a synchronizer for command-response tests,
    /// this method collects every line seen into a [`ReadLinesResult`].
    ///
    /// Use this when your test needs to verify the **entirety** or **sequence** of
    /// rendered output, such as:
    /// - Verifying no extra blank lines appear between log outputs.
    /// - Checking column alignment across multi-line messages.
    /// - Inspecting the full visual state of a virtual terminal buffer.
    ///
    /// For iterative synchronization, see [`Self::read_line_state()`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use r3bl_tui::PtyTestContext;
    ///
    /// fn main() {
    ///     let mut context: PtyTestContext = todo!();
    ///
    ///     // Capture all output until the child prints "DONE".
    ///     let result = context.child.read_until_marker(
    ///         &mut context.buf_reader,
    ///         "DONE",
    ///         |line| !line.is_empty(), // Optional filter: ignore empty lines.
    ///     );
    ///
    ///     assert!(result.found_marker);
    ///     // result.lines contains all lines captured before "DONE".
    /// }
    /// ```
    ///
    /// [`PTY`]: mod@crate::core::pty
    pub fn read_until_marker<R: BufRead>(
        &self,
        buf_reader: &mut R,
        marker: &str,
        mut line_filter_fn: impl FnMut(&str) -> bool,
    ) -> ReadLinesResult {
        let mut lines = Vec::new();
        let mut found_marker = false;

        loop {
            let mut line = String::new();
            match buf_reader.read_line_eio_to_eof(&mut line) {
                Ok(0) => break,
                Ok(_) => {
                    // Normalize the line: strip ANSI escape sequences and carriage
                    // returns.
                    let trimmed = {
                        let stripped = strip_ansi_escapes::strip_str(&line);
                        stripped
                            .replace("\r\n", "\n")
                            .replace('\r', "")
                            .trim()
                            .to_string()
                    };

                    eprintln!("  {GLYPH_CONTROLLED} Controlled output: {trimmed:?}");

                    if trimmed.contains(marker) {
                        found_marker = true;
                        break;
                    }

                    if !trimmed.is_empty() && line_filter_fn(&trimmed) {
                        lines.push(trimmed);
                    }
                }
                Err(e) => {
                    eprintln!("read_until_marker: read error: {e}");
                    break;
                }
            }
        }

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
    ///   [`DSR_CURSOR_POSITION_REQUEST`] is found and replying with
    ///   [`DSR_CURSOR_POSITION_ORIGIN_RESPONSE`].
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
    /// [`DSR_CURSOR_POSITION_ORIGIN_RESPONSE`]: crate::DSR_CURSOR_POSITION_ORIGIN_RESPONSE
    /// [`DSR_CURSOR_POSITION_REQUEST`]: crate::DSR_CURSOR_POSITION_REQUEST
    /// [`DSR`]: crate::DsrSequence
    /// [`stdout`]: std::io::Stdout
    pub fn get_writer_with_handshake<R: Read>(
        &self,
        pty_pair: &PtyPair,
        reader: &mut R,
    ) -> ControllerWriter {
        #[cfg(target_os = "windows")]
        {
            self.get_writer_windows(pty_pair, reader)
        }
        #[cfg(not(target_os = "windows"))]
        {
            Self::get_writer_unix(pty_pair, reader)
        }
    }

    #[cfg(not(target_os = "windows"))]
    fn get_writer_unix<R: Read>(pty_pair: &PtyPair, _reader: &mut R) -> ControllerWriter {
        let writer = pty_pair
            .controller()
            .take_writer()
            .expect("Failed to take writer");
        Box::new(writer)
    }

    /// Windows-specific implementation of [`Self::get_writer_with_handshake()`].
    ///
    /// # Why the Handshake is Necessary on Windows ([`ConPTY`])
    ///
    /// When [`portable_pty`] creates a Windows Pseudoconsole session via the Win32
    /// [`CreatePseudoConsole`] API, it passes the `PSEUDOCONSOLE_INHERIT_CURSOR` flag.
    /// Because of this flag, the background headless console host (`conhost.exe`)
    /// automatically emits [`DSR_CURSOR_POSITION_REQUEST`] to the output pipe on startup.
    /// It does this to query the outer host terminal for the initial cursor location
    /// before rendering.
    ///
    /// In our [`PTY`] test harness, the test process acts as that host terminal. The
    /// controller reads the stream until it encounters [`DSR_CURSOR_POSITION_REQUEST`]
    /// and immediately replies with [`DSR_CURSOR_POSITION_ORIGIN_RESPONSE`], telling
    /// [`ConPTY`] that the cursor is at row 1, col 1 (origin). Once answered, [`ConPTY`]
    /// completes its initialization and begins delivering child process output.
    ///
    /// # Graceful Fallback (Avoiding 60-Second Watchdog Timeout)
    ///
    /// On Windows, [`ConPTY`] does **not** close the output pipe when the child exits and
    /// `conhost.exe` holds the pipe open until the pseudoconsole is explicitly torn down.
    /// If a child process panics, crashes, or exits during startup before completing the
    /// handshake, calling `reader.read()` blocks indefinitely waiting for a
    /// [`DSR_CURSOR_POSITION_REQUEST`] that will never arrive. This causes the test
    /// runner to hang until the 60-second [`PtyTestWatchdog`] kills the process.
    ///
    /// > On POSIX systems, when a child process exits or panics, the operating system
    /// > closes the replica pseudoterminal file descriptors, causing the controller's
    /// > `reader.read()` to return immediately with `EOF` or `EIO`.
    ///
    /// To fail fast in milliseconds instead of hanging for 60 seconds, this method
    /// queries [`Self::is_child_terminated_windows()`] before reading and between buffer
    /// chunks. If the child has already exited, it breaks out of the handshake loop
    /// immediately and returns the writer so the test controller can reap the child and
    /// report the error without delay.
    ///
    /// [`ConPTY`]:
    ///     https://learn.microsoft.com/en-us/windows/console/creating-a-pseudoconsole-session
    /// [`CreatePseudoConsole`]:
    ///     https://learn.microsoft.com/en-us/windows/console/createpseudoconsole
    /// [`DSR_CURSOR_POSITION_ORIGIN_RESPONSE`]:
    ///     crate::DSR_CURSOR_POSITION_ORIGIN_RESPONSE
    /// [`DSR_CURSOR_POSITION_REQUEST`]: crate::DSR_CURSOR_POSITION_REQUEST
    /// [`portable_pty`]: portable_pty
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    /// [`PtyTestWatchdog`]: crate::PtyTestWatchdog
    #[cfg(target_os = "windows")]
    fn get_writer_windows<R: Read>(
        &self,
        pty_pair: &PtyPair,
        reader: &mut R,
    ) -> ControllerWriter {
        use crate::{DSR_CURSOR_POSITION_ORIGIN_RESPONSE, DSR_CURSOR_POSITION_REQUEST};
        use std::io::Write;

        let mut writer = pty_pair
            .controller()
            .take_writer()
            .expect("Failed to take writer");

        // If the child process has already terminated (e.g., mock test or early exit), it
        // will never emit DSR, so return immediately without blocking.
        if !self.is_child_terminated_windows() {
            let mut buf = [0u8; 4096];
            loop {
                // If the child process has already terminated, break out of the loop.
                if self.is_child_terminated_windows() {
                    break;
                }
                // Read from the reader, up to a page of 4 KiB.
                match reader.read(&mut buf) {
                    Ok(n) if n > 0 => {
                        let expected_dsr_request = DSR_CURSOR_POSITION_REQUEST.as_bytes();
                        let window_size = expected_dsr_request.len();

                        // If ConPTY sent the cursor position request anywhere in this
                        // chunk, reply with the origin position report (1, 1) to satisfy
                        // the handshake and complete.
                        if buf[..n]
                            .windows(window_size)
                            .any(|byte_chunk| byte_chunk == expected_dsr_request)
                        {
                            let _unused = writer.write_all(
                                DSR_CURSOR_POSITION_ORIGIN_RESPONSE.as_bytes(),
                            );
                            let _unused = writer.flush();
                            break;
                        }

                        // If the request was not in this chunk and the child process has
                        // now exited, break out immediately to avoid blocking
                        // indefinitely on the next read call.
                        if self.is_child_terminated_windows() {
                            break;
                        }
                    }
                    _ => break,
                }
            }
        }

        Box::new(writer)
    }

    /// Polls the child process handle without blocking to check if it has exited.
    ///
    /// # How it works
    ///
    /// On Windows, process handles (`hProcess`) are kernel synchronization objects.
    /// While a process is active, its handle remains in a nonsignaled state. When the
    /// process terminates, the kernel transitions its handle to the signaled state.
    ///
    /// Invoking [`WaitForSingleObject`] with a timeout of `0` milliseconds performs
    /// an instantaneous, non-blocking poll. If it returns [`WAIT_OBJECT_0`] (`0`),
    /// the child process has terminated. If it returns `WAIT_TIMEOUT` (`0x102`), the
    /// process is still running.
    ///
    /// This check operates on [`self.child.as_raw_handle()`], allowing a non-blocking
    /// liveness query with only an immutable reference (`&self`), avoiding the need
    /// for `&mut self` required by [`portable_pty::Child::try_wait()`].
    ///
    /// [`WAIT_OBJECT_0`]:
    ///     https://learn.microsoft.com/en-us/windows/win32/api/synchapi/nf-synchapi-waitforsingleobject#return-value
    /// [`WaitForSingleObject`]:
    ///     https://learn.microsoft.com/en-us/windows/win32/api/synchapi/nf-synchapi-waitforsingleobject
    #[cfg(target_os = "windows")]
    fn is_child_terminated_windows(&self) -> bool {
        use std::os::windows::io::RawHandle;

        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn WaitForSingleObject(hHandle: RawHandle, dwMilliseconds: u32) -> u32;
        }
        const WAIT_OBJECT_0: u32 = 0;

        if let Some(raw_handle) = self.child.as_raw_handle() {
            unsafe { WaitForSingleObject(raw_handle, 0) == WAIT_OBJECT_0 }
        } else {
            false
        }
    }
}

// cspell:words PSEUDOCONSOLE nonsignaled conhost
