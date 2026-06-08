# Task: Fix mio_poller Edge-Triggered Polling

## Overview

The `mio` crate on Unix uses edge-triggered `epoll` (`EPOLLET`). This means the OS only
notifies the application when a socket transitions from "empty" to "has data".

If the application does not completely drain the socket during a single `poll()` wakeup,
the remaining data will sit in the kernel buffer indefinitely. The application will
_never_ receive another notification for that remaining data because no new empty-to-ready
state transition occurred.

## The Bug

Currently, the
`tui/src/tui/terminal_lib_backends/direct_to_ansi/input/mio_poller/handler_stdin.rs`
module has a critical flaw: `consume_stdin_input_with_sender` performs exactly **one**
`.read()` call per wakeup.

If the thread is delayed (for instance, by synchronous debug logging blocking the thread)
and multiple keystrokes or DSR responses arrive in the meantime, the single `.read()` call
will pull some bytes but might leave the rest behind in the OS buffer. Because the buffer
wasn't fully drained back to empty, the edge-trigger is never reset. The `mio::Poll`
thread goes back to sleep and will never wake up for those stranded bytes, causing a
permanent deadlock where the UI stops responding to input.

## The Fix

To properly handle edge-triggered sockets, we must drain the socket until it explicitly
returns `ErrorKind::WouldBlock`.

1. **Refactor `consume_stdin_input_with_sender`**: Wrap the `.read()` call and its `match`
   block inside a `loop { ... }`.
2. **Continue Processing**: Continue reading and parsing bytes inside the loop.
3. **Exit Condition**: Only break the loop and return `Continuation::Continue` when
   `read()` returns `Err(ref e) if e.kind() == ErrorKind::WouldBlock` (indicating the
   socket is fully drained).
4. **EOF Handling**: Break the loop and return `Continuation::Stop` on `Ok(0)` (EOF) or
   any other fatal error.
5. **Non-blocking stdin**: To prevent `read()` from blocking indefinitely when the buffer
   is empty, `stdin` MUST be set to non-blocking mode. In `mio_poll_worker.rs`, we use
   `rustix::fs::fcntl_setfl` to set `OFlags::NONBLOCK` when the worker is created, and
   restore the original flags when it is dropped to prevent breaking the terminal. This
   was required to fix the following tests that were deadlocking after the `loop`
   refactor:
   - `test_pty_mio_poller_thread_lifecycle`
   - `test_pty_mio_poller_subscribe`
   - `test_production_factory_restart_cycle`

## Implementation Steps

- [x] In
      `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/mio_poller/handler_stdin.rs`,
      modify `consume_stdin_input_with_sender` to use a `loop`.
- [x] Ensure that `parse_stdin_bytes_with_sender` is called on each successful read chunk.
- [x] If `parse_stdin_bytes_with_sender` returns `Continuation::Stop`, break the loop and
      return `Continuation::Stop`.
- [x] Handle `ErrorKind::Interrupted` (`EINTR`) by continuing the loop (retrying the read
      immediately).
- [x] Ensure that `WouldBlock` breaks the loop and yields back to `mio::Poll` to wait for
      the next edge trigger.
- [x] In
  `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/mio_poller/mio_poll_worker.rs`,
  set `stdin` to non-blocking during initialization and restore it on `Drop`.
- [x] Fix `pty_test_color_detection::test_color_detection_in_pty` which failed because the
  test environment had `TERM=dumb`, causing our new non-blocking `stdin` changes (or other
  environment variables) to expose a flaw where the test expected a color terminal but
  didn't explicitly set a color-capable `TERM`.

## Mandatory Manual Review
- [x] `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/mio_poller/handler_stdin.rs`
- [x]
  `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/mio_poller/mio_poll_worker.rs`
- [x]
  `tui/src/core/ansi/detect_color/detect_color_integration_tests/pty_test_color_detection.rs`

# Fix side effect of this fix (stdout is also non blocking now)

- [x] Fix bug introduced by mio-poller-edge-triggered-polling:
  https://github.com/r3bl-org/r3bl-open-core/issues/453
  - [x] Implemented `FullBufferWaitingStdout` in `OutputDevice::new_stdout()` to yield
    execution and retry when encountering `ErrorKind::WouldBlock`.

## Plan: Integration Test & Rustdoc Updates

### Phase 1: Rustdoc Updates

- [x] In `tui/src/core/terminal_io/output_device.rs`:
  - [x] Rename `RetryStdout` to `pub struct FullBufferWaitingStdout`.
  - [x] Add rustdocs to `OutputDevice::new_stdout()` (around line 35) explaining that it
    wraps `stdout` in `FullBufferWaitingStdout` to handle non-blocking writes.
  - [x] Overhaul the `FullBufferWaitingStdout` rustdocs to include the "Mental Model" (why
    it exists) and an educational section on "Blocking vs. Busy-Waiting vs. Yielding" to
    explain exactly what `yield_now()` is doing.
  - [x] In the `FullBufferWaitingStdout` rustdocs, add intra-doc links to
    `crate::tui::terminal_lib_backends::direct_to_ansi::input::mio_poller::handler_stdin::consume_stdin_input_with_sender#why-we-need-non-blocking-read`,
    the `original_stdin_flags` field in
    `crate::tui::terminal_lib_backends::direct_to_ansi::input::mio_poller::MioPollWorker`,
    and `#method.drop`.

- [x] In
  `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/mio_poller/mio_poll_worker.rs`:
  - [x] Add a new heading & section documenting the side-effect that setting `O_NONBLOCK`
    on `stdin` also makes `stdout` non-blocking since they share the same file
    description.
  - [x] Link to
    `super::handler_stdin::consume_stdin_input_with_sender#how-this-affects-stdout-as-well`.
  - [x] Add intra-doc links to ``[`crate::core::terminal_io::FullBufferWaitingStdout`]``
    and ``[`crate::core::terminal_io::OutputDevice::new_stdout`]``.

- [x] In
      `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/mio_poller/handler_stdin.rs`:
  - [x] In `# Why We Need Non-Blocking Read` item 1, explicitly state that setting non-blocking mode on `stdin` uses `O_NONBLOCK`.
  - [x] In the rustdocs for `consume_stdin_input_with_sender()`, add a new section titled
    `## How this affects stdout as well` just before the `# Returns` section (and do not repeat `O_NONBLOCK` detail there).
  - [x] In this new section, add intra-doc links pointing to `super::MioPollWorker`,
    specifically highlighting the `original_stdin_flags` field and `#method.drop` where
    the stdout side effect originates.

### Phase 2: Integration Test for OutputDevice `WouldBlock` Recovery

- [x] Add a new PTY integration test using the `generate_pty_test!` macro.
- [x] **Location:**
  `tui/src/core/terminal_io/backend_compat_tests/pty_non_blocking_stdout_no_panic_test.rs` (or
  create a new `integration_tests/` folder in `terminal_io/`).
- [x] **Test Architecture:**
  - Create a **single integration test** that executes both scenarios sequentially within
    the same PTY slave process.
  - Spawn the child process (PTY slave).
  - In the slave, explicitly set `stdin` to non-blocking using `rustix` (replicating the
    condition created by `MioPollWorker`).
  - Create an instance of `OutputDevice::new_stdout()`.
  - **Scenario 1 (Small Write):** Perform a small write (e.g., a short string). Assert
    that it succeeds immediately. This verifies that standard output still behaves
    normally under the non-blocking flag when the PTY buffer has plenty of space.
  - **Scenario 2 (Massive Write):** Immediately following the small write, perform a
    massive, continuous write operation that exceeds the PTY buffer capacity. Assert that
    the write successfully completes without panicking on an `EAGAIN` / `WouldBlock`
    error. This explicitly triggers and tests the `FullBufferWaitingStdout` yield/retry
    loop.

### Phase 3: Manual Review

- [x] **Mandatory manual review:** Verify every file modified in this phase for correct
  implementation and ensure no regressions.
  - [x] `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/mio_poller/mio_poll_worker.rs`
  - [x] `tui/src/core/terminal_io/output_device.rs`
  - [x] `tui/src/core/terminal_io/backend_compat_tests/pty_non_blocking_stdout_no_panic_test.rs`
  - [x] `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/mio_poller/handler_stdin.rs`

- [x] Manual test verification
  - [x] Run `run.fish run-examples` and use `shell_async`. Then run `cat README.md` which
        works. This is a very large 151KB file.

- [ ] Create a commit closing https://github.com/r3bl-org/r3bl-open-core/issues/453 and
      move this commit down (there's a WIP commit in progress). Then push only this commit
      to origin main branch (do not push the WIP commit in progress).