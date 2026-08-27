# Task: Fix yield_now Slowdown on Linux (Event-Driven POLLOUT Backpressure)

<!-- cspell:words OFDs OFD termios openpty conpty ptmx rustix POLLOUT NONBLOCK EINTR -->
<!-- cspell:words POLLIN EPOLLET EAGAINT ENXIO ENOENT futexes EAGAIN SIGCONT SIGPROF EBADF -->

## Final Outcome & Accomplishment: 175 FPS on Linux (+65% Improvement)

Replacing timeslice yielding (`yield_now()`) with event-driven `POLLOUT` waiting via
`rustix::event::poll` in `BackpressureStdout` completely eliminated `__sched_yield`
context-switching churn from the render path, allowing Linux `DirectToAnsi` throughput to
exceed macOS blocking I/O:

| Environment                                  | Before Fix (`yield_now`) | After Fix (`BackpressureStdout` / `POLLOUT`) | Improvement                           |
| :------------------------------------------- | :----------------------- | :------------------------------------------- | :------------------------------------ |
| **Linux (`nazmul-mobile.local`, perf mode)** | 106 fps (~9.43 ms)       | **175 fps** (~5.71 ms)                       | **+65% FPS increase** (beats macOS)   |
| **macOS (`nazmul-mac.local`)**               | 163–173 fps (~5.78 ms)   | **163 fps** (~6.13 ms)                       | Native blocking stdio baseline        |
| **`__sched_yield` in flamegraph**            | 30%–40% of paint time    | **0% (100% eliminated)**                     | Complete removal of scheduler latency |

---

## Overview

When running the TUI examples (e.g., `cargo run -q --example tui_apps` selecting app
`0. App with no layout`) across the fleet using identical terminal geometries and the same
terminal emulator (`wezterm`), performance numbers show a noticeable discrepancy:

| Machine                | Reported FPS | Median Frame Latency | Backend Selected (`TERMINAL_LIB_BACKEND`) |
| :--------------------- | :----------- | :------------------- | :---------------------------------------- |
| `nazmul-mac.local`     | 173 fps      | ~5.78 ms             | `Crossterm` (blocking stdio)              |
| `nazmul-mobile.local`  | 106 fps      | ~9.43 ms             | `DirectToAnsi` (`MioPollWorker`)          |
| `nazmul-desktop.local` | 98 fps       | ~10.20 ms            | `DirectToAnsi` (`MioPollWorker`)          |
| `nazmul-win.local`     | 49 fps       | ~20.41 ms            | `Crossterm` (ConPTY overhead)             |

Profiling data on Linux (`flamegraph-benchmark.perf-folded`) reveals that a large portion
of frame time on Linux is spent inside `__sched_yield` syscalls triggered by
`BackpressureStdout::write()` and `flush()`.

## Root Cause Analysis: Why the Slowdown Occurred ONLY on Linux

The root cause of why this performance penalty was observed exclusively on Linux and not
on macOS or Windows lies in how each platform selects its terminal backend and manages
file descriptor flags:

### 1. Platform Backend Selection Difference

The backend is chosen at compile time via `TERMINAL_LIB_BACKEND`:

- **macOS & Windows (`TERMINAL_LIB_BACKEND = Crossterm`)**:
    - `Crossterm` operates entirely on standard blocking stdio.
    - It **never sets `O_NONBLOCK`** on `stdin`.
    - `stdout` remains in standard blocking mode natively.
    - When `stdout.write()` or `flush()` writes large frames, the OS kernel blocks the
      thread natively and resumes execution the moment the terminal emulator consumes
      bytes.
    - Because `stdout` is blocking, `stdout.write()` **never returned
      `ErrorKind::WouldBlock`**.
    - Consequently, macOS and Windows **never executed the `yield_now()` fallback loop**,
      running at full native speed (173 FPS on macOS).

- **Linux (`TERMINAL_LIB_BACKEND = DirectToAnsi`)**:
    - `DirectToAnsi` uses `MioPollWorker` to perform high-performance edge-triggered input
      multiplexing (`epoll` with `EPOLLET`).
    - To enable edge-triggered polling without deadlocking the poller thread,
      `MioPollWorker` sets `O_NONBLOCK` on `stdin` (fd 0).

### 2. The Linux Shared Kernel Open File Description (OFD) Mechanism

On Unix and Linux systems, file descriptor 0 (`stdin`), file descriptor 1 (`stdout`), and
file descriptor 2 (`stderr`) point to the **same underlying Open File Description (OFD)**
for the controlling terminal (`/dev/pts/X` or `/dev/tty`):

1. The `O_NONBLOCK` flag is stored in the kernel OFD (`struct file.f_flags`), not in the
   per-process file descriptor table.
2. When `MioPollWorker` enabled `O_NONBLOCK` on `stdin`, it implicitly modified the shared
   OFD.
3. This unintended side effect caused **`stdout` to become non-blocking as well**.

### 3. The `WouldBlock` and `yield_now()` Churn on Linux

When App 0 painted large frames (e.g., 18 color wheel gradient lines), the generated ANSI
byte payload exceeded the 4,096-byte `n_tty` kernel buffer:

1. Because `stdout` was non-blocking on Linux, `stdout.write()` immediately failed with
   `ErrorKind::WouldBlock` (`EAGAIN`).
2. `BackpressureStdout` caught `WouldBlock` and invoked `std::thread::yield_now()`
   (`sched_yield()` syscall) to let the terminal emulator catch up.
3. `sched_yield` blindly surrendered the CPU timeslice to the OS scheduler without any
   knowledge of when buffer capacity would actually be available.
4. If the buffer was not drained by the next timeslice, repeated context switches created
   massive latency churn (accounting for 30% to 40% of total frame time in flamegraphs),
   dropping Linux throughput from 175 FPS down to 106 FPS.

### Summary Comparison

| Metric / Dimension                  | macOS / Windows               | Linux                                    |
| :---------------------------------- | :---------------------------- | :--------------------------------------- |
| **Backend Selected**                | `Crossterm`                   | `DirectToAnsi` (`MioPollWorker`)         |
| **`stdin` Mode**                    | Blocking                      | Non-blocking (`O_NONBLOCK`)              |
| **`stdout` Mode**                   | Blocking                      | Non-blocking (via shared OFD)            |
| **`stdout.write()` Result on >4KB** | Kernel blocks thread natively | Returns `ErrorKind::WouldBlock`          |
| **Entered `yield_now()` Loop?**     | **No** (never hit)            | **Yes** (triggered on every heavy frame) |
| **Performance Impact**              | None (Baseline 173 FPS)       | Severe (-65% FPS penalty: 106 FPS)       |

---

## Architectural Insight: The Missing Half of the Async Reactor

The `DirectToAnsi` backend was designed to be a pure-Rust, zero-dependency, asynchronous
terminal I/O engine using `mio::Poll`. However, only half of the async reactor equation
was implemented:

```
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                                 Async Terminal Engine                                  │
├───────────────────────────────────────────┬────────────────────────────────────────────┤
│ INPUT HALF (Implemented)                  │ OUTPUT HALF (Missing / Incomplete)         │
├───────────────────────────────────────────┼────────────────────────────────────────────┤
│ - Registered with `mio::Poll`             │ - Synchronous writes to `stdout`           │
│ - Set `O_NONBLOCK` on `stdin`             │ - `stdout` became non-blocking (shared OFD)│
│ - Drains input on `Interest::READABLE`    │ - When buffer fills, hits `WouldBlock`     │
│ - Sleeps in kernel on `POLLIN` (0% CPU)   │ - Blindly yielded CPU via `yield_now()` ❌  │
│                                           │ - Missing: Wait on `POLLOUT` (0% CPU)  ✅   │
└───────────────────────────────────────────┴────────────────────────────────────────────┘
```

---

## The Solution: Event-Driven `POLLOUT` Backpressure (The Winner)

Rather than hacking the filesystem by opening `/dev/tty` handles, the proper and
symmetrical solution is to **complete the async I/O reactor** by giving `stdout` proper
event-driven write-readiness waiting via `POLLOUT`.

### Architectural Asymmetry in `MioPollWorker` for `stdin` vs `stdout`

Why does `stdin` need `mio::Poll`, but `stdout` needs one-shot `rustix::event::poll`?

There is a fundamental architectural asymmetry between reading from `stdin` and writing to
`stdout`:

1. **`stdin` (Continuous, Asynchronous Input)**:
    - Keystrokes, terminal resizes (`SIGWINCH`), and software interrupts arrive at
      unpredictable times.
    - `MioPollWorker` runs a continuous background event loop using `mio::Poll` to
      multiplex these diverse sources and sleep until `Interest::READABLE` (`POLLIN`)
      occurs.
2. **`stdout` (Synchronous, Demand-Driven Frame Output)**:
    - Rendering happens synchronously on frame ticks.
    - `stdout` is **writable 99.9% of the time**. If `stdout` were registered with
      `Interest::WRITABLE` in a continuous `mio::Poll` loop, `epoll_wait` would **never
      sleep** and would spin at 100% CPU because the buffer is almost always ready.
    - `stdout` only needs to wait during the rare microseconds when a frame flush exceeds
      the 4KB kernel buffer and returns `ErrorKind::WouldBlock`.
    - Therefore, `stdout` does not need a continuous reactor loop or a background writer
      thread. It only requires a **synchronous, one-shot wait on `PollFlags::OUT`
      (`POLLOUT`)** directly when `WouldBlock` is encountered.

### Implementation in `BackpressureStdout`

In `tui/src/core/terminal_io/output_device.rs`:

```rust
#[cfg(unix)]
impl Write for BackpressureStdout {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        loop {
            match self.0.write(buf) {
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // Tell the OS kernel: Put this thread on the PTY wait-queue
                    // and wake us the exact microsecond stdout has capacity for writes:
                    let mut poll_fd = [rustix::event::PollFd::new(
                        &self.0,
                        rustix::event::PollFlags::OUT, // POLLOUT
                    )];
                    // Blocks with 0% CPU until buffer space is freed:
                    match rustix::event::poll(&mut poll_fd, None) {
                        // 1. Buffer has space: retry write() on next iteration:
                        Ok(_) => continue,
                        // 2. Interrupted by signal (e.g., SIGWINCH, SIGCONT, SIGPROF):
                        //    The PTY is healthy; retry poll without failing:
                        Err(rustix::io::Errno::INTR) => continue,
                        // 3. Fatal error (e.g., EBADF): fail fast to prevent 100% CPU spin loop:
                        Err(e) => return Err(std::io::Error::from(e)),
                    }
                }
                other => return other,
            }
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        loop {
            match self.0.flush() {
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    let mut poll_fd = [rustix::event::PollFd::new(
                        &self.0,
                        rustix::event::PollFlags::OUT,
                    )];
                    match rustix::event::poll(&mut poll_fd, None) {
                        Ok(_) => continue,
                        Err(rustix::io::Errno::INTR) => continue,
                        Err(e) => return Err(std::io::Error::from(e)),
                    }
                }
                other => return other,
            }
        }
    }
}

#[cfg(not(unix))]
impl Write for BackpressureStdout {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        loop {
            match self.0.write(buf) {
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::yield_now();
                }
                other => return other,
            }
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        loop {
            match self.0.flush() {
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::yield_now();
                }
                other => return other,
            }
        }
    }
}
```

### Signal Interrupts (`EINTR`) and Fatal Error Recovery

The `rustix::event::poll` call returns `Result<usize, rustix::io::Errno>`. Handling this
result requires distinguishing between benign kernel signal interruptions and
unrecoverable system errors:

1. **Why `EINTR` (`Errno::INTR`) Happens**: In Unix, when a thread is sleeping inside a
   blocking system call like `poll(2)`, the kernel interrupts the system call and returns
   `EINTR` whenever a POSIX signal is delivered to the process. In `r3bl_tui`, signals
   occur routinely:
    - `SIGWINCH`: Delivered when the terminal emulator window is resized.
    - `SIGCONT`: Delivered when the process is resumed from background job control (`fg`).
    - `SIGPROF`: Delivered periodically during performance profiling and flamegraph
      sampling.

2. **Why We Retry on `EINTR`**: Receiving `EINTR` does not indicate an error with `stdout`
   or the PTY. It simply means the thread woke up early to handle a signal before
   `POLLOUT` became ready. Retrying the loop (`continue`) ensures no data is dropped and
   the thread returns to waiting on buffer capacity.

3. **Why Non-`EINTR` Errors Must Fail Fast**: If standard output's file descriptor becomes
   invalid (e.g., `EBADF` if the file descriptor was closed unexpectedly), `poll` returns
   immediately without blocking. Blindly ignoring errors or looping without checks would
   turn the polite wait into an infinite 100% CPU busy-spin loop. Converting non-`EINTR`
   errors to `std::io::Error` guarantees that fatal errors propagate upward cleanly.

### Why This Is the Superior Architecture

1. **Completes the Async Reactor**: Symmetrically pairs `POLLIN` (read-readiness on
   `stdin`) with `POLLOUT` (write-readiness on `stdout`).
2. **Zero `sched_yield` Latency**: The Linux kernel suspends the thread on the hardware
   PTY wait-queue and wakes it up sub-microsecond the moment WezTerm issues a `read()` on
   `/dev/ptmx`.
3. **Zero Allocation & Zero Setup**: `rustix::event::poll` operates on a stack-allocated
   `PollFd` without creating extra kernel `epoll` instances or worker threads.
4. **Preserves Synchronous Telemetry**: Frame rendering and flushing remain synchronous,
   ensuring telemetry timers in `telemetry_record!` measure true end-to-end frame latency.
5. **Zero CPU Waste**: 0% CPU consumption while waiting for buffer space (unlike spin
   loops or polling loops).
6. **100% Portable and Pure Rust**: Works with standard `std::io::stdout()`, PTY test
   fixtures (`PtyPair`), Docker containers, and CI runners without opening `/dev/tty` or
   managing fallback file handles.

---

## Implementation Plan

### Phase 1: Implement `POLLOUT` Waiting in `BackpressureStdout`

- [x] **Update `tui/Cargo.toml` dependency**:
    - Add `"event"` feature to `rustix` dependency
      (`features = ["termios", "std", "event"]`).
- [x] **Update `BackpressureStdout::write()` and `flush()`**:
    - On Unix (`#[cfg(unix)]`), replace `std::thread::yield_now()` with
      `rustix::event::poll(&mut [poll_fd], None)` where `poll_fd` watches for
      `rustix::event::PollFlags::OUT`.
    - On non-Unix (`#[cfg(not(unix))]`), retain fallback behavior with
      `std::thread::yield_now()`.
- [x] **Audit error handling**:
    - Handle `EINTR` retry if interrupted by signals during `poll`.
    - Propagate fatal `poll` errors as `std::io::Error` to prevent runaway loops.
- [x] **Document intentional asymmetry and maintain intra-doc link integrity**:
    - Add doc comments in `output_device.rs` (`BackpressureStdout`) explaining why output
      uses one-shot on-demand `POLLOUT` waiting while input uses a continuous `mio::Poll`
      loop.
    - Verify and align rustdoc section anchors referenced by
      `pty_non_blocking_stdout_no_panic_test.rs` and `mio_poller/mod.rs`.
    - Cross-link bidirectionally between `BackpressureStdout` and `mio_poller`.
- [x] **Modularize and shard `core/terminal_io/`**:
    - Extracted `BackpressureStdout` into `tui/src/core/terminal_io/backpressure_stdout/`
      (`mod.rs`, `backpressure_stdout_struct.rs`, `impl_unix.rs`, `impl_win.rs`).
    - Extracted `TerminalModeController` & `FullScreenTuiModeGuard` into
      `tui/src/core/terminal_io/terminal_mode_controller.rs`.
    - Renamed `terminal_io_type_aliases.rs` to `types.rs`.
    - Maintained 100% flat public API via barrel re-exports in
      `tui/src/core/terminal_io/mod.rs`.

### Phase 2: Benchmark and Verification

- [x] **Validate Quality Checks**:
    - Run `./check.fish --check`
    - Run `./check.fish --test`
    - Run `./check.fish --clippy`
    - Run `./check.fish --quick-doc`
    - Run `./check.fish --full`
- [x] **Performance Verification**:
    - Run `./run.fish run-examples-flamegraph-fold --benchmark` and verify `__sched_yield`
      is completely eliminated from hot paths.
    - Verified on fleet machines:
        - Linux (`nazmul-mobile.local` in performance governor mode): **175 FPS** (~5.71
          ms median latency).
        - macOS (`nazmul-mac.local`): **163 FPS** (~6.13 ms median latency).
        - Linux `DirectToAnsi` + `POLLOUT` backpressure now matches/beats macOS blocking
          stdio throughput!

#### Benchmark Results (Before vs. After)

| Environment                                  | Before Fix (`yield_now`) | After Fix (`BackpressureStdout` / `POLLOUT`) | Improvement                           |
| :------------------------------------------- | :----------------------- | :------------------------------------------- | :------------------------------------ |
| **Linux (`nazmul-mobile.local`, perf mode)** | 106 fps (~9.43 ms)       | **175 fps** (~5.71 ms)                       | **+65% FPS increase** (beats macOS)   |
| **macOS (`nazmul-mac.local`)**               | 163–173 fps (~5.78 ms)   | **163 fps** (~6.13 ms)                       | Native blocking stdio baseline        |
| **`__sched_yield` in flamegraph**            | 30%–40% of paint time    | **0% (100% eliminated)**                     | Complete removal of scheduler latency |

---

## Mandatory Manual Review

- [x] `tui/Cargo.toml`
- [x] `tui/src/core/terminal_io/mod.rs`
- [x] `tui/src/core/terminal_io/output_device.rs`
- [x] `tui/src/core/terminal_io/terminal_mode_controller.rs`
- [x] `tui/src/core/terminal_io/types.rs`
- [x] `tui/src/core/terminal_io/backpressure_stdout/mod.rs`
- [x] `tui/src/core/terminal_io/backpressure_stdout/backpressure_stdout_struct.rs`
- [x] `tui/src/core/terminal_io/backpressure_stdout/impl_unix.rs`
- [x] `tui/src/core/terminal_io/backpressure_stdout/impl_win.rs`
- [x] `tui/src/core/terminal_io/backend_compat_tests/pty_non_blocking_stdout_no_panic_test.rs`
- [x] `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/mio_poller/mod.rs`
- [x] `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/mio_poller/handler_stdin.rs`
- [x] `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/mio_poller/mio_poll_worker.rs`
- [x] `task/fix-yield_now-slowdown-on-linux.md`

---

## Appendix: Alternative Approaches Evaluated and Discarded

Below is the comprehensive technical analysis of all alternate approaches that were
considered and why they were not chosen.

### Approach 1: Continuous `mio::Poll` Reactor Loop for Writing (Async Writer Thread)

- **Concept**: Mirror `MioPollWorker` for output by registering `stdout` with `mio::Poll`
  on `Interest::WRITABLE`, using a background thread and an MPSC channel to pass rendered
  byte buffers.
- **Why it was discarded (The 100% CPU Busy Loop Problem)**:

```rust
// Flawed Approach: Continuous mio::Poll registration for stdout
let poll = Poll::new()?;
let mut events = Events::with_capacity(16);
let stdout_raw_fd = std::io::stdout().as_raw_fd();

// ❌ Flaw: Registering Interest::WRITABLE permanently in a reactor loop
poll.registry().register(
    &mut SourceFd(&stdout_raw_fd),
    Token(1),
    Interest::WRITABLE,
)?;

loop {
    // ❌ Bug: Because stdout is writable 99.9% of the time, poll() returns
    // immediately in ~50ns without sleeping. This loop spins at 100% CPU!
    poll.poll(&mut events, None)?;

    for event in events.iter() {
        if event.token() == Token(1) && event.is_writable() {
            // When there is no active render frame, this thread spins continuously
        }
    }
}
```

- **Dynamic Registration Complexity**: To avoid the busy loop, a reactor must dynamically
  register `Interest::WRITABLE` only _after_ `write()` fails with `WouldBlock`, and then
  _immediately deregister_ it after writing:

```rust
// Complex Alternative: Dynamic register/deregister dance per WouldBlock
match stdout.write(buf) {
    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
        // 1. Must register stdout for WRITABLE with the reactor:
        poll.registry().register(&mut SourceFd(&stdout_fd), Token(1), Interest::WRITABLE)?;
        // 2. Wait for event:
        poll.poll(&mut events, None)?;
        // 3. Must immediately deregister WRITABLE to prevent 100% CPU spinning!
        poll.registry().deregister(&mut SourceFd(&stdout_fd))?;
    }
    other => return other,
}
```

This 3-step dance is needlessly complex. `rustix::event::poll(&mut [poll_fd], -1)`
performs this exact kernel wait in a single, zero-allocation syscall without any reactor
bookkeeping.

- **Other Discarded Reasons**:
    - **Heap Allocation Churn**: Passing rendered frame buffers (`Vec<u8>`) across an MPSC
      channel adds continuous allocation and deallocation overhead per frame.
    - **Loss of Telemetry Timing Accuracy**: Rendering would push to a channel and return
      immediately, preventing `telemetry_record!` from measuring true frame flush latency
      to the display.
    - **Queue Backpressure Complexity**: Requires building and managing an async frame
      queue to prevent memory blowup if the app renders faster than WezTerm drains.

### Approach 2: Open an Independent `/dev/tty` File Handle for Output

- **Concept**: Instead of writing to `std::io::stdout()`, `OutputDevice::new_stdout()`
  explicitly calls `File::options().write(true).open("/dev/tty")`. This creates a
  brand-new Open File Description (OFD #2) in kernel space that is in standard blocking
  mode, while `stdin` stays non-blocking on OFD #1.
- **Why it was discarded**:
    - Opening `/dev/tty` relies on filesystem-level handle management and bypasses the
      async reactor model rather than properly solving it.
    - In headless CI, Docker containers without allocated TTYs, or unusual test
      environments, opening `/dev/tty` fails with `ENXIO` / `ENOENT`, requiring fallback
      paths.
    - `POLLOUT` waiting directly solves the problem within the existing `stdout` file
      descriptor without touching initialization or introducing filesystem dependencies.

### Approach 3: Dedicated Blocking Reader Thread for `stdin`

- **Concept**: Eliminate `O_NONBLOCK` from `stdin` completely. Spawn a dedicated reader
  thread that performs standard blocking `stdin.read()` in a loop and sends parsed events
  over a channel. Because `O_NONBLOCK` is never set on `stdin`, `stdout` remains in
  standard blocking mode natively.
- **Why it was discarded**:
    - Rewrites the core `DirectToAnsi` input engine (`MioPollWorker` / RRT), discarding
      the benefits of unified edge-triggered `mio::Poll` handling for `stdin`, `SIGWINCH`
      signals, and synthetic software interrupts.
    - `POLLOUT` solves the output bottleneck without requiring any refactoring of the
      input engine.

### Approach 4: User-Space Condvar / Monitor (`std::sync::Condvar`)

- **Concept**: Use a condition variable / mutex to coordinate waiting and waking when the
  terminal buffer fills up.
- **Why it was discarded**:
    - A `Condvar` is an intra-process synchronization primitive backed by user-space
      futexes.
    - The terminal emulator (WezTerm) runs in an entirely separate external process and
      communicates across the PTY kernel device boundary (`/dev/ptmx` master vs
      `/dev/pts/X` slave). WezTerm cannot access or signal an in-process Condvar in our
      application.
    - The Linux kernel's wait-queue via `poll(POLLOUT)` is the true OS-level Condvar for
      file descriptors across process boundaries.

### Approach 5: Busy-Waiting / Spin Loop (`core::hint::spin_loop`)

- **Concept**: When `stdout.write()` returns `ErrorKind::WouldBlock`, immediately retry in
  a tight spin loop without sleeping or yielding.
- **Why it was discarded**:
    - Pins a CPU core at 100% utilization during every frame flush exceeding 4KB, causing
      severe thermal throttling and battery drain.
    - Actively starves WezTerm's reader thread of CPU scheduler time on constrained
      systems, potentially worsening total render latency.
    - Floods the kernel with hundreds of thousands of failing `write()` syscalls
      (`EAGAIN`) per millisecond.
