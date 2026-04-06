# Task: Fix mio_poller Edge-Triggered Polling

## Overview
The `mio` crate on Unix uses edge-triggered `epoll` (`EPOLLET`). This means the OS only notifies the application when a socket transitions from "empty" to "has data". 

If the application does not completely drain the socket during a single `poll()` wakeup, the remaining data will sit in the kernel buffer indefinitely. The application will *never* receive another notification for that remaining data because no new empty-to-ready state transition occurred.

## The Bug
Currently, the `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/mio_poller/handler_stdin.rs` module has a critical flaw: `consume_stdin_input_with_sender` performs exactly **one** `.read()` call per wakeup.

If the thread is delayed (for instance, by synchronous debug logging blocking the thread) and multiple keystrokes or DSR responses arrive in the meantime, the single `.read()` call will pull some bytes but might leave the rest behind in the OS buffer. Because the buffer wasn't fully drained back to empty, the edge-trigger is never reset. The `mio::Poll` thread goes back to sleep and will never wake up for those stranded bytes, causing a permanent deadlock where the UI stops responding to input.

## The Fix
To properly handle edge-triggered sockets, we must drain the socket until it explicitly returns `ErrorKind::WouldBlock`.

1. **Refactor `consume_stdin_input_with_sender`**: Wrap the `.read()` call and its `match` block inside a `loop { ... }`.
2. **Continue Processing**: Continue reading and parsing bytes inside the loop.
3. **Exit Condition**: Only break the loop and return `Continuation::Continue` when `read()` returns `Err(ref e) if e.kind() == ErrorKind::WouldBlock` (indicating the socket is fully drained).
4. **EOF Handling**: Break the loop and return `Continuation::Stop` on `Ok(0)` (EOF) or any other fatal error.

## Implementation Steps
- [ ] In `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/mio_poller/handler_stdin.rs`, modify `consume_stdin_input_with_sender` to use a `loop`.
- [ ] Ensure that `parse_stdin_bytes_with_sender` is called on each successful read chunk.
- [ ] If `parse_stdin_bytes_with_sender` returns `Continuation::Stop`, break the loop and return `Continuation::Stop`.
- [ ] Handle `ErrorKind::Interrupted` (`EINTR`) by continuing the loop (retrying the read immediately).
- [ ] Ensure that `WouldBlock` breaks the loop and yields back to `mio::Poll` to wait for the next edge trigger.