// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words tcgetwinsize winsize EINTR SIGWINCH kqueue epoll wakeup eventfd bcast

//! Public API: [`DirectToAnsiInputDevice`].
//!
//! The main user-facing type for async terminal input. See
//! [`input_device_impl`] for internal implementation details.
//!
//! [`input_device_impl`]: super::input_device_impl

use super::{channel_types::{PollerEvent, SignalEvent, StdinEvent},
            input_device_impl::{InputSubscriberGuard, global_input_resource}};
use crate::{InputEvent, core::resilient_reactor_thread::RRTEvent,
            tui::DEBUG_TUI_SHOW_TERMINAL_BACKEND};
use std::fmt::Debug;
use tokio::sync::broadcast::error::RecvError;

/// Async input device for [`DirectToAnsi`] backend.
///
/// One of two real [`InputDevice`] backends (the other being [`CrosstermInputDevice`]).
/// Selected via [`TERMINAL_LIB_BACKEND`] on Linux; talks directly to the terminal using
/// ANSI/VT100 protocols without relying on [`crossterm`] for terminal I/O.
///
/// This **newtype** delegates to [`SINGLETON`] for [`std::io::Stdin`] reading and buffer
/// management. This process global singleton supports restart cycles with thread reuse
/// (fast path) to handle race conditions when apps rapidly create and drop input devices.
///
/// It manages asynchronous reading from terminal [`stdin`] via dedicated thread +
/// channel:
/// - [`stdin`] channel receiver (process global singleton, outlives device instances)
/// - Parsing happens in the reader thread using the [`more` flag pattern]
/// - [ESC key disambiguation]: waits for more bytes only when data is likely pending
/// - Dispatch to protocol parsers (keyboard, mouse, terminal events, UTF-8)
///
/// # Architecture
///
/// This module provides cancel-safe async terminal input for a process, by bridging a
/// synchronous [`mio`]-based reader thread with async consumers via a [`broadcast`]
/// channel. It handles keyboard input (including ANSI escape sequences for arrow keys,
/// function keys, etc.) and terminal resize signals ([`SIGWINCH`]) reliably, even over
/// [SSH].
///
/// ## [Loosely Coupled And Strongly Coherent]
///
/// The [`broadcast`] channel **decouples** the reader thread from async consumers,
/// enabling independent lifecycle management:
/// - **Reuse existing thread** (fast path) when apps rapidly stop/start
/// - **Create new thread** when none exists or previous one terminated
/// - **Destroy thread** cleanly when no consumers need input
///
/// This decoupling also allows **multiple async consumers** to receive all input events
/// simultaneously—useful for debugging, logging, or event recording alongside the primary
/// TUI app consumer.
///
/// ## Why This Design? (Historical Context)
///
/// Our original "Tokio-heavy" approach created a [`DirectToAnsiInputDevice`] instance
/// on-demand, one-instance-per-app (which was not process-bound, rather it was bound to
/// each app-instance). It used:
/// - [`tokio::io::stdin()`] for input handling
/// - [`tokio::signal`] for [`SIGWINCH`] handling
///
/// ### The Problems
///
/// **This caused three problems that led us to the current design:**
///
/// 1. **UI freeze on resize.** [Tokio's stdin] uses a blocking threadpool. In the past,
///    in [`next()`], when [`tokio::select!`] cancelled a [`tokio::io::stdin()`] read to
///    handle [`SIGWINCH`], the blocking read kept running in the background. The next
///    read conflicted with this "zombie" read leading to a UI freeze.
///
/// 2. **Dropped keystrokes.** Creating a new [`stdin`] handle lost access to data already
///    in the kernel buffer. When TUI "App A" exited and "App B" started, keystrokes typed
///    during the transition vanished. This was easily reproducible by:
///    - Running `cargo run --examples tui_apps`.
///    - Starting one app, exiting, **dropped keystrokes**, starting another, exit,
///      **dropped keystrokes**, starting another, and so on.
///
/// 3. **Flawed `ESC` detection over [SSH].** Our original approach had flawed logic for
///    distinguishing the `ESC` key from escape sequences (like `ESC [ A` for Up Arrow).
///    It worked locally but failed over [SSH]. We now use [`crossterm`]'s `more` flag
///    heuristic (see [ESC Detection Limitations] in [`MioPollWorker`]).
///
/// ### The Solution
///
/// A **process bound global singleton** with a dedicated reader thread that is the
/// **designated reader** of [`stdin`]. The thread uses [`mio::Poll`] to wait on both
/// [`stdin`] data and [`SIGWINCH`] signals.
///
/// <div class="warning">
///
/// **No exclusive access**: Any thread can call [`std::io::stdin()`] and read from it—
/// there is no OS or Rust mechanism to prevent this. If another thread reads from
/// [`stdin`], bytes will be stolen, causing interleaved reads that corrupt the input
/// stream and break the VT100 parser. See [No exclusive access] in [`MioPollWorker`].
///
/// </div>
///
/// Although sync and blocking, [`mio`] is efficient. It uses OS primitives ([`epoll`] on
/// Linux, [`kqueue`] on BSD/macOS) that put the thread to sleep until data arrives,
/// consuming no CPU while waiting. See [How It Works] in [`MioPollWorker`] for details.
///
/// ```text
///     Process-bound Global Singleton                       Async Consumers
/// ┌─────────────────────────────────────┐           ┌─────────────────────────────┐
/// │ Sync Blocking (std::thread + mio)   │           │ Primary: TUI input handler  │
/// │                                     │           │ Optional: Debug logger      │
/// │ Designated reader of:               │           │ Optional: Event recorder    │
/// │   • stdin (not exclusive access!)   │           │                             │
/// │   • Parser state                    │           │                             │
/// │   • SIGWINCH watcher                │           │                             │
/// │                                     │ broadcast │                             │
/// │ sender.send(InputEvent)  ───────────┼──────► receiver.recv().await            │
/// │                                     │ channel   │ (cancel-safe, fan-out!)     │
/// └─────────────────────────────────────┘    │      └─────────────────────────────┘
///                                            ▼
///                                     Sync -> Async Bridge
/// ```
///
/// This solves the first two problems completely:
/// 1. **Cancel-safe**: Channel receive is truly async - no zombie reads
/// 2. **Data preserved**: Global state survives TUI app lifecycle transitions in the same
///    process.
///
/// To solve the third problem for `ESC` detection, we use [`crossterm`]'s `more` flag
/// heuristic (see [ESC Detection Limitations] in [`MioPollWorker`]).
///
/// ## Architecture Overview
///
/// ```text
/// ┌─────────────────────────────────────────────────────────────────────────┐
/// │ SINGLETON (static Mutex<Option<...>>)                                   │
/// │ internal:                                                               │
/// │  • mio-poller thread: holds sender, reads stdin, runs vt100 parser      │
/// │ external:                                                               │
/// │  • stdin_receiver: broadcast receiver (async consumers recv() from here)│
/// └───────────────────────────────────────┬─────────────────────────────────┘
///                                         │
///            ┌────────────────────────────┴─────────────────────────┐
///            │                                                      │
/// ┌──────────▼───────────────────┐            ┌─────────────────────▼───────┐
/// │ DirectToAnsiInputDevice A    │            │ DirectToAnsiInputDevice B   │
/// │   (TUI App context)          │            │   (Readline context)        │
/// │   • Zero-sized handle        │            │   • Zero-sized handle       │
/// │   • Delegates to global      │            │   • Delegates to global     │
/// └──────────────────────────────┘            └─────────────────────────────┘
///
/// 🎉 Data preserved during transitions - same channel used throughout!
/// ```
///
/// The key insight: [`stdin`] handles must persist across device lifecycle boundaries.
/// Multiple [`DirectToAnsiInputDevice`] instances can be created and dropped, but they
/// all share the same underlying channel and process global (singleton) reader thread.
///
/// See [`MioPollWorker`] for details on how the mio poller thread works, including file
/// descriptor handling, parsing, thread lifecycle, and ESC detection limitations.
///
/// # Device Lifecycle
///
/// A single process can create and drop [`DirectToAnsiInputDevice`] instances repeatedly.
/// The global [`SINGLETON`] `static` persists, but the **thread** spawns and exits with
/// each app lifecycle:
///
/// ```text
/// ┌───────────────────────────────────────────────────────────────────────────────┐
/// │ PROCESS LIFETIME                                                              │
/// │                                                                               │
/// │ SINGLETON: Mutex<Option<InputResource>>                                       │
/// │ (static persists, but contents are replaced on each thread spawn)             │
/// │                                                                               │
/// │ ═════════════════════════════════════════════════════════════════════════════ │
/// │                                                                               │
/// │ ┌───────────────────────────────────────────────────────────────────────────┐ │
/// │ │ TUI app A lifecycle                                                       │ │
/// │ │                                                                           │ │
/// │ │  1. DirectToAnsiInputDevice::new()                                        │ │
/// │ │  2. next() → subscribe()                                                  │ │
/// │ │  3. SINGLETON is None → initialize_global_input_resource()                │ │
/// │ │       • Creates broadcast channel, shared_waker_slot, liveness: Running   │ │
/// │ │       • Spawns mio-poller thread #1                                       │ │
/// │ │       • thread #1 owns MioPollWorker struct                               │ │
/// │ │  4. TUI app A runs, receiving events from receiver                        │ │
/// │ │  5. TUI app A exits → device dropped → receiver dropped                   │ │
/// │ │  6. Thread #1 detects 0 receivers → exits gracefully                      │ │
/// │ │  7. MioPollWorker::drop() → liveness = Terminated                         │ │
/// │ │                                                                           │ │
/// │ └───────────────────────────────────────────────────────────────────────────┘ │
/// │                                                                               │
/// │ ┌───────────────────────────────────────────────────────────────────────────┐ │
/// │ │ TUI app B lifecycle                                                       │ │
/// │ │                                                                           │ │
/// │ │  1. DirectToAnsiInputDevice::new()                                        │ │
/// │ │  2. next() → subscribe()                                                  │ │
/// │ │  3. SINGLETON has state, but liveness == Terminated                       │ │
/// │ │       → needs_init = true → initialize_global_input_resource()            │ │
/// │ │       • Swaps shared_waker_slot, creates fresh liveness: Running          │ │
/// │ │       • Spawns mio-poller thread #2 (NOT the same as #1!)                 │ │
/// │ │       • thread #2 owns its own MioPollWorker struct                       │ │
/// │ │  4. TUI app B runs, receiving events from receiver                        │ │
/// │ │  5. TUI app B exits → device dropped → receiver dropped                   │ │
/// │ │  6. Thread #2 detects 0 receivers → exits gracefully                      │ │
/// │ │  7. MioPollWorker::drop() → liveness = Terminated                         │ │
/// │ │                                                                           │ │
/// │ └───────────────────────────────────────────────────────────────────────────┘ │
/// │                                                                               │
/// │ ... pattern repeats for App C, D, etc. ...                                    │
/// │                                                                               │
/// └───────────────────────────────────────────────────────────────────────────────┘
/// ```
///
/// **Key insight**: The [`mio_poller`] thread is NOT persistent across the lifetime of
/// the process. Each app lifecycle spawns a new thread. The liveness tracking enables
/// this by allowing [`subscribe()`] to detect when a thread has exited and spawn a new
/// one.
///
/// ## Why Keystrokes Aren't Lost During Transitions
///
/// Given the [Device Lifecycle] above—where threads exit and restart between apps—a
/// natural question arises: **why don't keystrokes get lost during the transition?**
///
/// The historical problem (see [The Problems]) was that the old "Tokio-heavy" approach
/// created a new [`tokio::io::stdin()`] handle per app. When App A exited and App B
/// started, keystrokes typed during the transition vanished because
/// [`tokio::io::stdin()`] uses **application-level buffering**—when that handle is
/// dropped, its internal buffer is lost forever.
///
/// The current design provides **three layers of protection**:
///
/// | Layer                       | Protection Mechanism                                                                                           |
/// | :-------------------------- | :------------------------------------------------------------------------------------------------------------- |
/// | **Kernel buffer persists**  | Even after thread restart, unread bytes remain in the kernel's [`fd`] `0` buffer                               |
/// | **No app-level buffering**  | Direct [`std::io::Stdin`] reads with immediate parsing—no internal buffer to lose                              |
/// | **Fast-path reuse**         | If new app subscribes before thread exits, existing thread continues; see [`pty_mio_poller_thread_reuse_test`] |
///
/// ### Data Flow During App Switching
///
/// ```text
/// ┌──────────────────────────────────────────────────────────────────────────┐
/// │ App A exits, drops receiver                                              │
/// │   • SubscriberGuard::drop() calls waker.wake()                           │
/// │   • Thread may continue running (fast) OR exit (slow)                    │
/// └──────────────────────────────┬───────────────────────────────────────────┘
///                                │
/// ┌──────────────────────────────▼───────────────────────────────────────────┐
/// │ User types keystrokes during transition                                  │
/// │   • Bytes arrive in kernel stdin buffer (fd 0)                           │
/// │   • If thread still running: reads immediately, sends to channel         │
/// │   • If thread exited: kernel buffer holds bytes until new thread reads   │
/// └──────────────────────────────┬───────────────────────────────────────────┘
///                                │
/// ┌──────────────────────────────▼───────────────────────────────────────────┐
/// │ App B starts, calls DirectToAnsiInputDevice::new()                       │
/// │   • subscribe() checks liveness flag                                     │
/// │   • If Running: reuses existing thread (no gap in reading)               │
/// │   • If Terminated: spawns new thread → reads kernel buffer → no data loss│
/// └──────────────────────────────────────────────────────────────────────────┘
/// ```
///
/// The key insight: the **kernel's [`stdin`] buffer for [`fd`] `0` persists** regardless
/// of which thread is reading. Unlike [`tokio::io::stdin()`]'s application-level buffer,
/// the kernel buffer survives handle creation/destruction. When a new thread calls
/// [`std::io::stdin()`], it gets a handle to the **same kernel buffer** containing any
/// unread bytes.
///
/// ## Call Chain to [`subscribe()`]
///
/// ```text
/// DirectToAnsiInputDevice::new()                 (input_device.rs)
///     │
///     └─► subscribe()                            (input_device.rs)
///             │
///             ├─► SINGLETON.lock()
///             │
///             ├─► needs_init = None || liveness == Terminated
///             │       │
///             │       └─► if needs_init: initialize_global_input_resource()
///             │               │
///             │               ├─► Init sender, shared_waker_slot wrapper (LazyLock)
///             │               ├─► F::create() → swap shared_waker_slot, create liveness
///             │               └─► Spawn thread with worker
///             │
///             └─► return SubscriberGuard { receiver, shared_waker_slot } ← new subscriber
///
/// DirectToAnsiInputDevice::next()                (input_device.rs)
///     │
///     └─► stdin_receiver.recv().await
/// ```
///
/// **Key points:**
/// - [`DirectToAnsiInputDevice`] is a newtype holding [`SubscriberGuard`]
/// - Global state ([`SINGLETON`]) persists - channel and thread survive device drops
/// - Eager subscription - each device subscribes at construction time in [`new()`]
/// - Thread liveness check - if thread died, next subscribe reinitializes everything
///
/// # Full I/O Pipeline
///
/// This device sits in the backend executor layer, bridging raw I/O to the protocol
/// parser, then converting protocol IR to the public API:
///
/// ```text
/// ┌───────────────────────────────────────────────────────────────────┐
/// │ Raw ANSI bytes: "1B[A" (hex)                                      │
/// │ std::io::stdin in mio-poller thread (SINGLETON)                   │
/// └────────────────────────────┬──────────────────────────────────────┘
///                              │
/// ┌────────────────────────────▼──────────────────────────────────────┐
/// │ mio-poller thread (input_device.rs)                               │
/// │   • mio::Poll waits on stdin data + SIGWINCH signals              │
/// │   • Parses bytes using `more` flag for ESC disambiguation         │
/// │   • Applies paste state machine                                   │
/// │   • Sends InputEvent through broadcast channel                    │
/// │                                                                   │
/// │ vt_100_terminal_input_parser/ (Protocol Layer - IR)               │
/// │   try_parse_input_event() dispatches to:                          │
/// │   ├─ parse_keyboard_sequence() → VT100InputEventIR::Keyboard      │
/// │   ├─ parse_mouse_sequence()    → VT100InputEventIR::Mouse         │
/// │   ├─ parse_terminal_event()    → VT100InputEventIR::Focus/Resize  │
/// │   └─ parse_utf8_text()         → VT100InputEventIR::Keyboard      │
/// │                                                                   │
/// │ protocol_conversion.rs (IR → Public API)                          │
/// │   convert_input_event()           VT100InputEventIR → InputEvent  │
/// │   convert_key_code_to_keypress()  VT100KeyCodeIR → KeyPress       │
/// └────────────────────────────┬──────────────────────────────────────┘
///                              │ broadcast channel
/// ┌────────────────────────────▼──────────────────────────────────────┐
/// │ THIS DEVICE: DirectToAnsiInputDevice (Backend Executor)           │
/// │   • Zero-sized handle struct (delegates to SINGLETON)             │
/// │   • Receives pre-parsed InputEvent from channel                   │
/// │   • Resize events carry Size directly (queried at signal time)    │
/// └────────────────────────────┬──────────────────────────────────────┘
///                              │
/// ┌────────────────────────────▼──────────────────────────────────────┐
/// │ Public API (Application Layer)                                    │
/// │   InputEvent::Keyboard(KeyPress)                                  │
/// │   InputEvent::Mouse(MouseInput)                                   │
/// │   InputEvent::Resize(Size)                                        │
/// │   InputEvent::Focus(FocusEvent)                                   │
/// │   InputEvent::BracketedPaste(String)                              │
/// │   InputEvent::Shutdown(ShutdownReason)                            │
/// └───────────────────────────────────────────────────────────────────┘
/// ```
///
/// For details on thread lifecycle (spawn/exit/relaunch), see the [Device Lifecycle]
/// section above.
///
/// # Data Flow Diagram
///
/// Here's the complete data flow for [`next()`]:
///
/// ```text
/// ┌───────────────────────────────────────────────────────────────────────────┐
/// │ 0. DirectToAnsiInputDevice::new() called                                  │
/// │    └─► subscribe() (eager, at construction time)                          │
/// │        └─► If no thread running: spawns mio-poller thread                 │
/// │                                                                           │
/// │ 1. next() called                                                          │
/// │    └─► Loop: stdin_receiver.recv().await                                  │
/// │         ├─► Event(e) → return e                                           │
/// │         ├─► Resize(size) → return InputEvent::Resize(size)                │
/// │         └─► Eof/Error → return None                                       │
/// └───────────────────────────────────▲───────────────────────────────────────┘
///                                     │ broadcast channel
/// ┌───────────────────────────────────┴───────────────────────────────────────┐
/// │ 2. mio-poller thread                                                      │
/// │    std::thread::spawn("mio-poller")                                       │
/// │                                                                           │
/// │    Uses mio::Poll to wait on stdin data + SIGWINCH signals:               │
/// │    ┌───────────────────────────────────────────────────────────────────┐  │
/// │    │ loop {                                                            │  │
/// │    │   poll.poll(&mut events, None)?;        // Wait for stdin/signal  │  │
/// │    │   let n = stdin.read(&mut buffer)?;     // Read available bytes   │  │
/// │    │   let more = n == TTY_BUFFER_SIZE;      // ESC disambiguation     │  │
/// │    │   parser.advance(&buffer[..n], more);   // Parse with `more` flag │  │
/// │    │   for event in parser { sender.send(Event(event))?; }             │  │
/// │    │ }                                                                 │  │
/// │    └───────────────────────────────────────────────────────────────────┘  │
/// │    See module docs for thread lifecycle (exits when all receivers drop)   │
/// └───────────────────────────────────────────────────────────────────────────┘
/// ```
///
/// # Underlying Protocol Parser
///
/// - [`vt_100_terminal_input_parser`]: The protocol parser that converts raw bytes to
///   [`VT100InputEventIR`]. This device calls [`try_parse_input_event`] to perform the
///   actual parsing.
///
/// # ESC Key Disambiguation (crossterm `more` flag pattern)
///
/// **The Problem**: Distinguishing ESC key presses from escape sequences (e.g., Up Arrow
/// = `ESC [ A`). When we see a lone `0x1B` byte, is it the ESC key or the start of an
/// escape sequence?
///
/// **The Solution**: We use crossterm's `more` flag pattern—a clever heuristic based on
/// read buffer fullness:
///
/// ```text
/// let n = stdin.read(&mut buffer)?;  // Read available bytes
/// let more = n == TTY_BUFFER_SIZE;   // true if buffer was filled completely
///
/// // In parser:
/// if buffer == [ESC] && more {
///     return None;  // Wait for more bytes (likely escape sequence)
/// } else if buffer == [ESC] && !more {
///     return ESC key;  // No more data, user pressed ESC
/// }
/// ```
///
/// ## How It Works
///
/// - **`more = true`**: Read filled the entire buffer, meaning more data is likely
///   waiting in the kernel buffer. Wait before deciding—this `ESC` is probably the start
///   of an escape sequence.
/// - **`more = false`**: Read returned fewer bytes than buffer size, meaning we've
///   drained all available input. A lone `ESC` is the ESC key.
///
/// ## Why This Works
///
/// Terminal emulators send escape sequences atomically in a single `write()` [`syscall`].
/// When you press Up Arrow, the terminal sends `ESC [ A` (3 bytes) together. The kernel
/// buffers these bytes, and our `read()` typically gets all of them at once.
///
/// ```text
/// User presses Up Arrow
///   ↓
/// Terminal: write(stdout, "1B[A" (hex), 3)  ← One syscall, 3 bytes
///   ↓
/// Kernel buffer: [1B, 5B, 41]               ← All bytes arrive together
///   ↓
/// stdin.read() → 3 bytes                    ← We get all 3 bytes
///   ↓
/// more = (3 == 1024) = false                ← Buffer not full
///   ↓
/// Parser sees [ESC, '[', 'A']               → Up Arrow event ✓
/// ```
///
/// ## SSH and High-Latency Connections
///
/// Over SSH with network latency, bytes might arrive in separate packets. The `more` flag
/// handles this correctly:
///
/// ```text
/// First packet:  [ESC]       read() → 1 byte, more = false
///                            BUT: next poll() wakes immediately when more data arrives
/// Second packet: ['[', 'A']  read() → 2 bytes
///                            Parser accumulates: [ESC, '[', 'A'] → Up Arrow ✓
/// ```
///
/// The key insight: if bytes arrive separately, the next `mio::Poll` wake happens almost
/// immediately when more data arrives. The parser accumulates bytes across reads, so
/// escape sequences are correctly reassembled.
///
/// ## Attribution
///
/// This pattern is adapted from crossterm's `mio.rs` implementation. See the
/// [Architecture] section above for details on our mio-based architecture.
///
/// We looked at [`crossterm`]'s source code for design inspiration:
/// 1. **Global state pattern**: [`crossterm`] uses a global [`INTERNAL_EVENT_READER`]
///    that holds the `tty` file descriptor and event buffer, ensuring data in the kernel
///    buffer is not lost when [`EventStream`] instances are created and dropped. And we
///    have the same global singleton pattern here.
/// 2. **[`mio`]-based polling**: Their [`mio.rs`] uses [`mio::Poll`] with
///    [`signal-hook-mio`] for [`SIGWINCH`] and we do the same.
/// 3. **ESC disambiguation**: The `more` flag heuristic for distinguishing ESC key from
///    escape sequences without timeouts. We inherit both its benefits (zero latency) and
///    limitations (see [ESC Detection Limitations] in [`MioPollWorker`]).
/// 4. **Process-lifetime cleanup**: They rely on OS cleanup at process exit rather than
///    explicit thread termination, and so do we.
///
/// # Drop behavior
///
/// When this device is dropped:
/// 1. [`super::at_most_one_instance_assert::release()`] is called, allowing a new device
///    to be created.
/// 2. Rust's drop glue drops [`Self::subscriber_guard`], triggering [`SubscriberGuard`'s
///    drop behavior] (thread lifecycle protocol).
///
/// For the complete lifecycle diagram including the [race condition] where a fast
/// subscriber can reuse the existing thread, see the [inherent race condition] docs.
///
/// [Architecture]: Self#architecture
/// [Device Lifecycle]: Self#device-lifecycle
/// [ESC Detection Limitations]: super::mio_poller#esc-detection-limitations
/// [ESC key disambiguation]: Self#esc-key-disambiguation-crossterm-more-flag-pattern
/// [How It Works]: super::mio_poller#how-it-works
/// [Loosely Coupled And Strongly Coherent]:
///     https://developerlife.com/2015/11/05/loosely-coupled-strongly-coherent/
/// [No exclusive access]: super::mio_poller#no-exclusive-access
/// [SSH]: https://en.wikipedia.org/wiki/Secure_Shell
/// [The Problems]: Self#the-problems
/// [Tokio's stdin]: tokio::io::stdin
/// [`CrosstermInputDevice`]:
///     crate::tui::terminal_lib_backends::crossterm_backend::CrosstermInputDevice
/// [`DirectToAnsi`]: mod@crate::direct_to_ansi
/// [`EventStream`]: crossterm::event::EventStream
/// [`INTERNAL_EVENT_READER`]:
///     https://github.com/crossterm-rs/crossterm/blob/0.29/src/event.rs#L149
/// [`InputDevice`]: crate::InputDevice
/// [`MioPollWorker`]: super::mio_poller::MioPollWorker
/// [`SIGWINCH`]: signal_hook::consts::SIGWINCH
/// [`SINGLETON`]: super::input_device_impl::global_input_resource::SINGLETON
/// [`SubscriberGuard`]: crate::core::resilient_reactor_thread::SubscriberGuard
/// [`SubscriberGuard`'s drop behavior]:
///     crate::core::resilient_reactor_thread::SubscriberGuard#drop-behavior
/// [`TERMINAL_LIB_BACKEND`]: crate::tui::TERMINAL_LIB_BACKEND
/// [`VT100InputEventIR`]:
///     crate::core::ansi::vt_100_terminal_input_parser::VT100InputEventIR
/// [`broadcast`]: tokio::sync::broadcast
/// [`crossterm`]: crossterm
/// [`epoll`]: https://man7.org/linux/man-pages/man7/epoll.7.html
/// [`fd`]: https://en.wikipedia.org/wiki/File_descriptor
/// [`kqueue`]: https://man.freebsd.org/cgi/man.cgi?query=kqueue&sektion=2
/// [`mio.rs`]:
///     https://github.com/crossterm-rs/crossterm/blob/0.29/src/event/source/unix/mio.rs
/// [`mio::Poll`]: mio::Poll
/// [`mio_poller`]: super::mio_poller
/// [`more` flag pattern]: Self#esc-key-disambiguation-crossterm-more-flag-pattern
/// [`new()`]: Self::new
/// [`next()`]: Self::next
/// [`pty_mio_poller_thread_reuse_test`]:
///     crate::core::ansi::vt_100_terminal_input_parser::integration_tests::pty_mio_poller_thread_reuse_test
/// [`signal-hook-mio`]: signal_hook_mio
/// [`std::io::Stdin`]: std::io::Stdin
/// [`std::io::stdin()`]: std::io::stdin
/// [`stdin`]: std::io::stdin
/// [`subscribe()`]: crate::core::resilient_reactor_thread::RRT::subscribe
/// [`super::at_most_one_instance_assert::release()`]:
///     super::at_most_one_instance_assert::release
/// [`syscall`]: https://man7.org/linux/man-pages/man2/syscalls.2.html
/// [`tokio::io::stdin()`]: tokio::io::stdin
/// [`tokio::select!`]: tokio::select
/// [`tokio::signal`]: tokio::signal
/// [`try_parse_input_event`]:
///     crate::core::ansi::vt_100_terminal_input_parser::try_parse_input_event
/// [`vt_100_terminal_input_parser`]: mod@crate::core::ansi::vt_100_terminal_input_parser
/// [inherent race condition]:
///     crate::core::resilient_reactor_thread#the-inherent-race-condition
/// [race condition]: crate::core::resilient_reactor_thread#the-inherent-race-condition
pub struct DirectToAnsiInputDevice {
    /// This device's subscription to the global input broadcast channel.
    ///
    /// Initialized eagerly in [`new()`] to ensure the thread sees a receiver if one is
    /// needed. This is critical for correct thread lifecycle management: if a device is
    /// dropped and a new one created "immediately", the new device's subscription must
    /// be visible to the thread when it checks [`receiver_count()`].
    ///
    /// Uses [`InputSubscriberGuard`] to wake the [`mio_poller`] thread when dropped,
    /// allowing thread lifecycle management.
    ///
    /// [`InputSubscriberGuard`]: super::input_device_impl::InputSubscriberGuard
    /// [`mio_poller`]: crate::direct_to_ansi::input::mio_poller
    /// [`new()`]: Self::new
    /// [`receiver_count()`]: tokio::sync::broadcast::Sender::receiver_count
    pub subscriber_guard: InputSubscriberGuard,
}

impl DirectToAnsiInputDevice {
    /// Create the input device.
    ///
    /// # Singleton Semantics
    ///
    /// Only ONE [`DirectToAnsiInputDevice`] can exist at a time. There is only one
    /// [`stdin`], so having multiple "devices" is semantically incorrect.
    ///
    /// To get additional receivers for logging, debugging, or multiple consumers, use
    /// [`subscribe()`] instead of calling [`new()`] again.
    ///
    /// # Panics
    ///
    /// Panics if called while another device exists. The panic message guides you to use
    /// [`subscribe()`] for additional receivers.
    ///
    /// # Thread Lifecycle
    ///
    /// Creates the [`mio_poller`] thread **eagerly** if it doesn't exist. This is
    /// critical for correct lifecycle: if device A is dropped and device B is created
    /// immediately, device B's subscription must be visible to the thread when it
    /// checks [`receiver_count()`] (before it decides to exit). See the [race condition
    /// documentation] in [`SubscriberGuard`] for details.
    ///
    /// [`SubscriberGuard`]: super::input_device_impl::InputSubscriberGuard
    /// [`mio_poller`]: crate::direct_to_ansi::input::mio_poller
    /// [`new()`]: Self::new
    /// [`receiver_count()`]: tokio::sync::broadcast::Sender::receiver_count
    /// [`stdin`]: std::io::Stdin
    /// [`subscribe()`]: Self::subscribe
    /// [race condition documentation]: super::input_device_impl::InputSubscriberGuard#race-condition-and-correctness
    #[must_use]
    pub fn new() -> Self {
        super::at_most_one_instance_assert::claim_and_assert();
        Self {
            subscriber_guard: global_input_resource::SINGLETON.subscribe().expect(
                "Failed to subscribe to input events: OS denied resource creation. \
                 Check ulimit -n (max open files) or available memory.",
            ),
        }
    }

    /// Get an additional subscriber (async consumer) to input events.
    ///
    /// Use this for logging, debugging, or multiple concurrent consumers. Each
    /// subscriber independently receives all input events. When dropped, notifies
    /// the [`mio_poller`] thread to check if it should exit.
    ///
    /// Even though we don't use `&self` in this method, it creates a capability gate. You
    /// can only [`subscribe()`] if you have allocated a device using [`new()`], enforcing
    /// the invariant that subscription requires prior allocation.
    ///
    /// See [`pty_mio_poller_subscribe_test`] for integration tests demonstrating
    /// broadcast semantics (both device and subscriber receive the same events).
    ///
    /// [`mio_poller`]: super::mio_poller
    /// [`new()`]: Self::new
    /// [`pty_mio_poller_subscribe_test`]: crate::core::ansi::vt_100_terminal_input_parser::integration_tests::pty_mio_poller_subscribe_test
    /// [`subscribe()`]: Self::subscribe
    #[must_use]
    pub fn subscribe(&self) -> InputSubscriberGuard {
        global_input_resource::SINGLETON.subscribe_to_existing()
    }

    /// Read the next input event asynchronously.
    ///
    /// # Returns
    ///
    /// `None` if [`stdin`] is closed ([`EOF`]). Or [`InputEvent`] variants for:
    /// - **Keyboard**: Character input, arrow keys, function keys, modifiers (with 0ms
    ///   `ESC` latency)
    /// - **Mouse**: Clicks, drags, motion, scrolling with position and modifiers
    /// - **Resize**: Terminal window size change (rows, cols)
    /// - **Focus**: Terminal gained/lost focus
    /// - **Paste**: Bracketed paste mode text
    /// - **Shutdown**: The RRT framework shut down the input thread ([`RestartPolicy`]
    ///   exhausted or panic caught). See [`ShutdownReason`] for details.
    ///
    /// # Usage
    ///
    /// This method is called once by the main event loop of the program using this
    /// [`InputDevice`] and this [`DirectToAnsiInputDevice`] struct is persisted for the
    /// entire lifetime of the program's event loop. Typical usage pattern:
    ///
    /// ```no_run
    /// # #[cfg(target_os = "linux")]
    /// # fn main() -> miette::Result<()> { tokio::runtime::Runtime::new().unwrap().block_on(async_main()) }
    /// # #[cfg(target_os = "linux")]
    /// # async fn async_main() -> miette::Result<()> {
    /// use r3bl_tui::DirectToAnsiInputDevice;
    /// use r3bl_tui::InputEvent;
    /// use tokio::signal;
    ///
    /// // Create device once at startup, reuse until program exit
    /// let mut input_device = DirectToAnsiInputDevice::new();
    /// # let mut sigint = signal::unix::signal(signal::unix::SignalKind::interrupt())
    /// #     .map_err(|e| miette::miette!("Failed to setup signal handler: {}", e))?;
    ///
    /// // Main event loop - handle multiple concurrent event sources with tokio::select!
    /// loop {
    ///     tokio::select! {
    ///         // Handle terminal input events
    ///         input_event = input_device.next() => {
    ///             match input_event {
    ///                 Some(InputEvent::Keyboard(key_press)) => {
    ///                     // Handle keyboard input
    ///                 }
    ///                 Some(InputEvent::Mouse(mouse_input)) => {
    ///                     // Handle mouse input
    ///                 }
    ///                 Some(InputEvent::Resize(size)) => {
    ///                     // Handle terminal resize
    ///                 }
    ///                 Some(InputEvent::BracketedPaste(text)) => {
    ///                     // Handle bracketed paste
    ///                 }
    ///                 Some(InputEvent::Focus(_)) => {
    ///                     // Handle focus events
    ///                 }
    ///                 Some(InputEvent::Shutdown(reason)) => {
    ///                     // Input thread shut down (restart policy exhausted or panic).
    ///                     // Exit or try re-subscribing.
    ///                     eprintln!("Input thread shutdown: {reason:?}");
    ///                     break;
    ///                 }
    ///                 None => {
    ///                     // stdin closed (EOF) - signal program to exit
    ///                     break;
    ///                 }
    ///             }
    ///         }
    ///         // Handle system signals (e.g., Ctrl+C)
    ///         _ = sigint.recv() => {
    /// #           eprintln!("Received SIGINT, shutting down...");
    /// #           break;
    ///         }
    ///         // Handle other concurrent tasks as needed
    ///         // _ = some_background_task => { ... }
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// # #[cfg(not(target_os = "linux"))]
    /// # fn main() {}
    /// ```
    ///
    /// **Key points about the code above:**
    /// - The device can be **created and dropped multiple times** - the process-global
    ///   [`SINGLETON`], its broadcast channel, and the kernel's [`stdin`] buffer all
    ///   persist across device lifetimes.
    /// - This method is **called repeatedly** by the async main event loop to get input
    ///   events as they arrive.
    /// - Returns [`None`] when [`stdin`] is closed (program should exit).
    ///
    /// # Implementation
    ///
    /// Async loop receiving pre-parsed events:
    /// 1. Check event queue for buffered events (from previous reads)
    /// 2. Wait for events from [`stdin`] reader channel (yields until data ready)
    /// 3. Apply paste state machine and return event
    ///
    /// Events arrive fully parsed from the reader thread. See [ESC key disambiguation]
    /// for zero-latency `ESC` detection.
    ///
    /// # Cancel Safety
    ///
    /// This method is cancel-safe. The internal broadcast channel receive
    /// ([`tokio::sync::broadcast::Receiver::recv`]) is truly cancel-safe: if cancelled,
    /// the data remains in the channel for the next receive.
    ///
    /// See the [Architecture] section for why we use a dedicated thread with
    /// [`mio::Poll`] and channel instead of [`tokio::io::stdin()`] (which is NOT
    /// cancel-safe).
    ///
    /// # Panics
    ///
    /// Panics if called after [`Drop`] has already been invoked (internal invariant).
    ///
    /// [Architecture]: Self#architecture
    /// [ESC key disambiguation]: Self#esc-key-disambiguation-crossterm-more-flag-pattern
    /// [`EOF`]: https://en.wikipedia.org/wiki/End-of-file
    /// [`InputDevice::next()`]: crate::InputDevice::next
    /// [`InputDevice`]: crate::InputDevice
    /// [`RestartPolicy`]: crate::core::resilient_reactor_thread::RestartPolicy
    /// [`SINGLETON`]: super::input_device_impl::global_input_resource::SINGLETON
    /// [`Self::next()`]: Self::next
    /// [`ShutdownReason`]: crate::core::resilient_reactor_thread::ShutdownReason
    /// [`mio::Poll`]: mio::Poll
    /// [`stdin`]: std::io::stdin
    /// [`tokio::io::stdin()`]: tokio::io::stdin
    /// [`tokio::sync::broadcast::Receiver::recv`]: tokio::sync::broadcast::Receiver::recv
    pub async fn next(&mut self) -> Option<InputEvent> {
        // Receiver was subscribed eagerly in new() - just use it.
        let subscriber_guard = &mut self.subscriber_guard;

        // Wait for fully-formed InputEvents through the broadcast channel.
        loop {
            let poller_rx_result = subscriber_guard.receiver.recv().await;

            let rrt_event = match poller_rx_result {
                // Got a message from the channel.
                Ok(msg) => msg,
                // The sender was dropped.
                Err(RecvError::Closed) => {
                    // Channel closed - reader thread exited.
                    return None;
                }
                // This receiver fell behind and messages were dropped.
                Err(RecvError::Lagged(skipped)) => {
                    // This receiver fell behind - some messages were dropped.
                    // Log and continue from the current position.
                    DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                        eprintln!(
                            "DirectToAnsiInputDevice: receiver lagged, skipped {skipped} messages"
                        );
                    });
                    continue;
                }
            };

            match rrt_event {
                RRTEvent::Worker(poller_event) => match poller_event {
                    PollerEvent::Stdin(StdinEvent::Input(event)) => {
                        return Some(event);
                    }
                    PollerEvent::Stdin(StdinEvent::Eof | StdinEvent::Error) => {
                        return None;
                    }
                    PollerEvent::Signal(SignalEvent::Resize(size)) => {
                        return Some(InputEvent::Resize(size));
                    }
                },
                RRTEvent::Shutdown(reason) => {
                    return Some(InputEvent::Shutdown(reason));
                }
            }
        }
    }
}

impl Debug for DirectToAnsiInputDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DirectToAnsiInputDevice")
            .field("global_resource", &"<SINGLETON>")
            .finish()
    }
}

impl Default for DirectToAnsiInputDevice {
    fn default() -> Self { Self::new() }
}

impl Drop for DirectToAnsiInputDevice {
    /// Clears gate, then Rust drops [`Self::subscriber_guard`], which triggers
    /// [`SubscriberGuard::drop()`]. See [Drop behavior] for full mechanism.
    ///
    /// [Drop behavior]: DirectToAnsiInputDevice#drop-behavior
    /// [`SubscriberGuard::drop()`]: super::input_device_impl::InputSubscriberGuard#drop-behavior
    fn drop(&mut self) { super::at_most_one_instance_assert::release(); }
}
