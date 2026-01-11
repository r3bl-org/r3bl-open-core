// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words tcgetwinsize winsize EINTR SIGWINCH kqueue epoll wakeup eventfd bcast
// cspell:words reinit

//! Implementation details for [`DirectToAnsiInputDevice`].
//!
//! This module contains the global singleton ([`global_input_resource::SINGLETON`]), its
//! container ([`InputResource`]), operations module ([`global_input_resource`]), and the
//! RAII subscription handle ([`InputDeviceSubscriptionHandle`]).
//!
//! See [`DirectToAnsiInputDevice`] for the big picture (architecture, lifecycle, I/O
//! pipeline).
//!
//! [`DirectToAnsiInputDevice`]: super::DirectToAnsiInputDevice

use super::{channel_types::PollerEventReceiver,
            mio_poller::{LivenessState, MioPollerThread, PollerBridge, SourceKindReady}};
use crate::tui::DEBUG_TUI_SHOW_TERMINAL_BACKEND;
use mio::{Poll, Waker};
use std::sync::{Arc, Mutex};

/// **Payload** stored in [`SINGLETON`] when [allocated] (removed on [deallocation]).
///
/// This struct is the **payload** inside the static [`SINGLETON`] container. It's created
/// when a thread spawns and destroyed when the thread exits, following the thread
/// create/destroy lifecycle. The [`SINGLETON`] itself is static (lives for the process
/// lifetime), but this [`InputResource`] comes and goes.
///
/// It contains:
/// 1. [`thread_to_singleton_bridge`]: Communication bridge (broadcast sender, liveness) —
///    shared with [`MioPollerThread`]
/// 2. [`waker`]: Shutdown signal — only needed by [`InputDeviceSubscriptionHandle`],
///    **not** passed to [`MioPollerThread`]
///
/// See [`DirectToAnsiInputDevice`] for the big picture (architecture, lifecycle, I/O
/// pipeline).
///
/// # Waker Coupled To Poll
///
/// The [`Waker`] was created from the same [`Poll`] instance passed to
/// [`MioPollerThread::new()`]. They share an OS-level bond:
///
/// ```text
/// Poll (epoll/kqueue) ──owns──► Registry ──creates──► Waker
/// ```
///
/// When [`waker.wake()`] is called, it triggers an event that [`poll()`] returns.
/// **If [`Poll`] is dropped, this [`Waker`] becomes useless** — it would signal an event
/// mechanism that no longer exists.
///
/// This is why the slow path in [`allocate()`] replaces **both** [`Poll`] and [`Waker`]
/// together. See [Poll → Registry → Waker Chain] for how they're created.
///
/// # Why Waker Is Not Passed to the Thread
///
/// The thread doesn't need a reference to [`Waker`] — it only needs to *respond* to wake
/// events. When [`allocate()`] creates the [`Poll`] and [`Waker`], the waker is
/// registered with [`Poll`]'s registry (see [Poll → Registry → Waker Chain]). This means:
///
/// - When **any** [`InputDeviceSubscriptionHandle`] calls [`waker.wake()`], the thread's
///   [`poll()`] returns with a [`ReceiverDropWaker`] token
/// - The thread handles this via [`handle_receiver_drop_waker()`], checking if it should
///   exit
///
/// The singleton keeps the [`Waker`] as a **distribution point** — cloning it to each
/// [`InputDeviceSubscriptionHandle`] on subscription. The thread never touches it
/// directly.
///
/// [Poll → Registry → Waker Chain]: global_input_resource::SINGLETON#poll--registry--waker-chain
/// [`DirectToAnsiInputDevice`]: super::DirectToAnsiInputDevice
/// [`InputDeviceSubscriptionHandle`]: InputDeviceSubscriptionHandle
/// [`MioPollerThread::new()`]: super::mio_poller::MioPollerThread::new
/// [`MioPollerThread`]: super::mio_poller::MioPollerThread
/// [`Poll`]: mio::Poll
/// [`ReceiverDropWaker`]: super::mio_poller::SourceKindReady::ReceiverDropWaker
/// [`SINGLETON`]: global_input_resource::SINGLETON
/// [`Waker`]: mio::Waker
/// [`allocate()`]: global_input_resource::allocate
/// [`handle_receiver_drop_waker()`]: super::mio_poller::handler_receiver_drop::handle_receiver_drop_waker
/// [`poll()`]: mio::Poll::poll
/// [`thread_to_singleton_bridge`]: InputResource::thread_to_singleton_bridge
/// [`waker.wake()`]: mio::Waker::wake
/// [`waker`]: InputResource::waker
/// [allocated]: global_input_resource::allocate
/// [deallocation]: InputDeviceSubscriptionHandle#impl-Drop-for-InputDeviceSubscriptionHandle
#[allow(missing_debug_implementations)]
pub struct InputResource {
    /// Communication bridge to the [`mio_poller`] thread.
    ///
    /// Shared via [`Arc::clone()`] on thread spawn — no [`Mutex`] needed because
    /// [`broadcast_tx`] and [`thread_liveness`] are internally thread-safe
    /// (tokio broadcast sender + [`AtomicBool`]).
    ///
    /// See [`PollerBridge`] for thread lifecycle and race condition handling.
    ///
    /// [`AtomicBool`]: std::sync::atomic::AtomicBool
    /// [`broadcast_tx`]: PollerBridge::broadcast_tx
    /// [`mio_poller`]: super::mio_poller
    /// [`thread_liveness`]: PollerBridge::thread_liveness
    pub thread_to_singleton_bridge: Arc<PollerBridge>,

    /// Waker to signal thread shutdown. Cloned to each
    /// [`InputDeviceSubscriptionHandle`].
    ///
    /// See [Waker Coupled To Poll] for why this must be replaced together with [`Poll`],
    /// and [Why Waker Is Not Passed to the Thread] for usage.
    ///
    /// [Waker Coupled To Poll]: InputResource#waker-coupled-to-poll
    /// [Why Waker Is Not Passed to the Thread]: InputResource#why-waker-is-not-passed-to-the-thread
    /// [`Poll`]: mio::Poll
    pub waker: Arc<mio::Waker>,
}

/// Operations on the process-global input resource singleton.
///
/// This module encapsulates the global [`SINGLETON`] static and provides all operations
/// on it. The singleton is `pub` for doc links, but callers should use these functions
/// rather than accessing it directly.
///
/// See [`InputResource`] for what the singleton holds when active.
///
/// [`SINGLETON`]: global_input_resource::SINGLETON
pub mod global_input_resource {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// **Static container** (lives for process lifetime) that holds an [`InputResource`]
    /// **payload** (ephemeral, follows thread lifecycle). The payload is created when
    /// [`allocate()`] spawns a thread, and [removed when the thread exits].
    ///
    /// Lifecycle states:
    /// - **Inert** (`None`) until [`allocate()`] spawns the poller thread
    ///   ([`MioPollerThread`])
    /// - **Active** (`Some`) while thread is running
    /// - **Dormant** (`Some` with terminated liveness) when all
    ///   [`InputDeviceSubscriptionHandle`]s drop and thread exits
    /// - **Reactivates** on next [`allocate()`] call (spawns fresh thread, replaces
    ///   payload)
    ///
    /// This is NOT "allocate once, lives forever" — supports full restart cycles.
    ///
    /// Use the functions in this module ([`allocate()`], [`is_thread_running()`], etc.)
    /// rather than accessing this directly.
    ///
    /// See [`InputResource`] for what the payload contains when active.
    ///
    /// # Why `Mutex<Option<T>>`?
    ///
    /// **Deferred initialization** — we can't create [`InputResource`] at `static` init
    /// time:
    ///
    /// | Operation         | Const?    | Why not?                               |
    /// | :---------------- | :-------- | :------------------------------------- |
    /// | [`Poll::new()`]   | No        | [`Syscall`] (creates epoll/kqueue fd)  |
    /// | [`Waker::new()`]  | No        | Requires Poll's registry (see below)   |
    /// | [`Arc::new()`]    | No        | Heap allocation                        |
    ///
    /// **Why [`syscalls`] can't be `const`:** In Rust, **all** `static` variables must be
    /// initialized with `const` expressions — this is a language rule, not a choice. The
    /// compiler evaluates these expressions at compile time and embeds the result in the
    /// binary. [`Syscalls`] ask the OS to do something (create an [`epoll`] [`fd`],
    /// allocate memory), which is impossible during compilation. The OS doesn't exist at
    /// compile time, and these operations have side effects that can't be "undone."
    ///
    /// Since [`Mutex::new(None)`] **is** `const` (just initializes memory layout), we use
    /// [`Option<T>`] to defer the [`syscalls`] until the first [`allocate()`] call at
    /// runtime.
    ///
    /// **Replacement on restart** — when the thread terminates and restarts (slow path),
    /// we need to replace the entire [`InputResource`] with fresh [`Poll`] + [`Waker`] +
    /// [`PollerBridge`]. [`Option::replace()`] makes this clean.
    ///
    /// **Note:** Fallibility is NOT the reason — we panic on [`syscall`] failure anyway.
    /// Even if these operations were infallible, we'd still need [`Option<T>`] because
    /// they're not `const`.
    ///
    /// # Poll → Registry → Waker Chain
    ///
    /// The [`Waker`] is tightly coupled to its [`Poll`]:
    ///
    /// ```text
    /// Poll::new()           // Creates OS event mechanism (epoll fd / kqueue)
    ///       │
    ///       ▼
    /// poll.registry()       // Handle to register interest
    ///       │
    ///       ▼
    /// Waker::new(registry)  // Registers with THIS Poll's mechanism
    ///       │
    ///       ▼
    /// waker.wake()          // Triggers event → poll.poll() returns
    /// ```
    ///
    /// This is why the slow path replaces **both** [`Poll`] and Waker together — a
    /// [`Waker`] is useless without its parent [`Poll`].
    ///
    /// # Usage
    ///
    /// - Use [`allocate()`] to subscribe to input events & signals.
    /// - See [Architecture] for why global state is necessary.
    /// - See [`MioPollerThread`] for thread lifecycle details.
    ///
    /// [`Option<T>`]: Option
    /// [Architecture]: super::super::DirectToAnsiInputDevice#architecture
    /// [`Mutex::new(None)`]: std::sync::Mutex::new
    /// [`Poll::new()`]: mio::Poll::new
    /// [`Poll`]: mio::Poll
    /// [`Syscall`]: https://en.wikipedia.org/wiki/System_call
    /// [`Syscalls`]: https://en.wikipedia.org/wiki/System_call
    /// [`Waker::new()`]: mio::Waker::new
    /// [`Waker`]: mio::Waker
    /// [`allocate()`]: allocate
    /// [`epoll`]: https://man7.org/linux/man-pages/man7/epoll.7.html
    /// [`fd`]: https://en.wikipedia.org/wiki/File_descriptor
    /// [`is_thread_running()`]: is_thread_running
    /// [`syscall`]: https://en.wikipedia.org/wiki/System_call
    /// [`syscalls`]: https://en.wikipedia.org/wiki/System_call
    /// [removed when the thread exits]: InputDeviceSubscriptionHandle#impl-Drop-for-InputDeviceSubscriptionHandle
    pub static SINGLETON: Mutex<Option<InputResource>> = Mutex::new(None);

    /// Subscribe your async consumer to the global input resource, in order to receive
    /// input events.
    ///
    /// The global `static` singleton [`SINGLETON`] contains one
    /// [`broadcast::Sender`]. This channel acts as a bridge between the only sync
    /// [`MioPollerThread`] and the many async consumers. We don't need to capture the
    /// broadcast channel itself in the singleton, only the sender, since it is
    /// trivial to create new receivers from it.
    ///
    /// # Returns
    ///
    /// A new [`InputDeviceSubscriptionHandle`] that independently receives all input
    /// events and resize signals.
    ///
    /// # Multiple Async Consumers
    ///
    /// Each caller gets their own receiver via [`broadcast::Sender::subscribe()`]. Here
    /// are examples of callers:
    /// - TUI app that receives all input events.
    /// - Logger receives all input events (independently).
    /// - Debug recorder receives all input events (independently).
    ///
    /// # Thread Spawning
    ///
    /// On first call, this spawns the [`mio`] poller thread via
    /// [`MioPollerThread::new()`] which uses [`mio::Poll`] to wait on both [`stdin`] data
    /// and [`SIGWINCH`] signals. See the [Thread Lifecycle] section in
    /// [`MioPollerThread`] for details on thread lifetime and exit conditions.
    ///
    /// # Two Allocation Paths
    ///
    /// | Condition                | Path          | What Happens                             |
    /// | ------------------------ | ------------- | ---------------------------------------- |
    /// | `liveness == Running`    | **Fast path** | Reuse existing thread + [`PollerBridge`] |
    /// | `liveness == Terminated` | **Slow path** | Replace all, spawn new thread            |
    ///
    /// ## Fast Path (Thread Reuse)
    ///
    /// If the thread is still running, we **reuse everything**:
    /// - Same [`PollerBridge`] (same broadcast channel, same liveness tracker)
    /// - Same [`mio::Poll`] + [`Waker`] (still registered, still valid)
    /// - Same thread (continues serving the new subscriber)
    ///
    /// This handles the [race condition] where a new subscriber appears before the
    /// thread checks [`receiver_count()`]. See [`PollerBridge`] for complete
    /// documentation on [The Inherent Race Condition] and [Why Thread Reuse Is Safe].
    ///
    /// ## Slow Path (Thread Restart)
    ///
    /// If the thread has terminated, the existing [`PollerBridge`] is **orphaned** —
    /// no thread is feeding events into its broadcast channel. We must **replace
    /// everything**:
    /// - New [`PollerBridge`] (fresh broadcast channel + liveness tracker)
    /// - New [`mio::Poll`] + [`Waker`] (old ones were dropped with old thread)
    /// - New thread (spawned to serve the new subscriber)
    ///
    ///
    /// # Panics
    ///
    /// Panics if:
    /// 1. Thread spawning fails; see [`MioPollerThread::new()`] for details.
    /// 2. The [`SINGLETON`] mutex is poisoned.
    /// 3. The [`SINGLETON`] is `None` after initialization (invariant violation).
    ///
    /// [The Inherent Race Condition]: PollerBridge#the-inherent-race-condition
    /// [Thread Lifecycle]: MioPollerThread#thread-lifecycle
    /// [Why Thread Reuse Is Safe]: PollerBridge#why-thread-reuse-is-safe
    /// [`PollerBridge`]: PollerBridge
    /// [`SIGWINCH`]: signal_hook::consts::SIGWINCH
    /// [`SINGLETON`]: SINGLETON
    /// [`Waker`]: mio::Waker
    /// [`broadcast::Sender::subscribe()`]: tokio::sync::broadcast::Sender::subscribe
    /// [`broadcast::Sender`]: tokio::sync::broadcast::Sender
    /// [`mio::Poll`]: mio::Poll
    /// [`receiver_count()`]: tokio::sync::broadcast::Sender::receiver_count
    /// [`stdin`]: std::io::stdin
    /// [race condition]: PollerBridge#the-inherent-race-condition
    pub fn allocate() -> InputDeviceSubscriptionHandle {
        let mut guard = SINGLETON.lock().expect(
            "SINGLETON mutex poisoned: another thread panicked while holding this lock. \
             Terminal input is unavailable. This is unrecoverable.",
        );

        // Fast path check: can we reuse the existing thread + PollerBridge + Waker?
        // See `PollerBridge` docs for race condition handling.
        let apply_fast_path_thread_reuse =
            guard.as_ref().is_some_and(|input_resource_state| {
                input_resource_state
                    .thread_to_singleton_bridge
                    .thread_liveness
                    .is_running()
                    == LivenessState::Running
            });

        // SLOW PATH: Thread terminated (or never started) → create new everything.
        // The existing PollerBridge is "orphaned" (no thread feeding it).
        if !apply_fast_path_thread_reuse {
            // New Poll (the old one was dropped with the old thread).
            let new_poll = Poll::new().expect(
                "Failed to create mio::Poll: OS denied epoll/kqueue creation. \
                 Check ulimit -n (max open files) or /proc/sys/fs/epoll/max_user_watches.",
            );
            let new_registry = new_poll.registry();
            // New Waker (must be tied to the new Poll's registry).
            let new_waker =
                Waker::new(new_registry, SourceKindReady::ReceiverDropWaker.to_token())
                    .expect(
                        "Failed to create mio::Waker: eventfd/pipe creation failed. \
                     Check ulimit -n (max open files).",
                    );

            // New PollerBridge (fresh broadcast channel + liveness tracker).
            let bridge = Arc::new(PollerBridge::new());

            // New thread (to feed events into the new PollerBridge).
            MioPollerThread::new(new_poll, Arc::clone(&bridge));

            // Replace the old (orphaned) InputResource with the new one.
            guard.replace(InputResource {
                thread_to_singleton_bridge: bridge,
                waker: Arc::new(new_waker),
            });
        }

        // FAST PATH (or after slow path): Use the current InputResource.
        // - Fast path: reuses existing thread + PollerBridge + Waker.
        // - Slow path: uses the newly created ones from above.
        debug_assert!(guard.is_some());
        let input_resource_state = guard.as_ref().unwrap();

        InputDeviceSubscriptionHandle {
            maybe_poller_rx: Some(
                input_resource_state
                    .thread_to_singleton_bridge
                    .broadcast_tx
                    .subscribe(),
            ),
            mio_poller_thread_waker: Arc::clone(&input_resource_state.waker),
        }
    }

    /// Checks if the [`mio_poller`] thread is currently running.
    ///
    /// This is useful for testing thread lifecycle behavior and debugging.
    ///
    /// # Returns
    ///
    /// - [`LivenessState::Running`] if the thread is running.
    /// - [`LivenessState::Terminated`] if [`SINGLETON`] is uninitialized or the thread
    ///   has exited.
    ///
    /// See [Device Lifecycle] in [`DirectToAnsiInputDevice`] for details on how threads
    /// spawn and exit.
    ///
    /// [Device Lifecycle]: crate::DirectToAnsiInputDevice#device-lifecycle
    /// [`DirectToAnsiInputDevice`]: crate::DirectToAnsiInputDevice
    /// [`mio_poller`]: crate::direct_to_ansi::input::mio_poller
    #[allow(clippy::redundant_closure_for_method_calls)]
    pub fn is_thread_running() -> LivenessState {
        SINGLETON
            .lock()
            .ok()
            .and_then(|guard| {
                guard.as_ref().map(|state| {
                    state
                        .thread_to_singleton_bridge
                        .thread_liveness
                        .is_running()
                })
            })
            .unwrap_or(LivenessState::Terminated)
    }

    /// Queries how many receivers are subscribed to the input broadcast channel.
    ///
    /// This is useful for testing thread lifecycle behavior and debugging.
    ///
    /// # Returns
    ///
    /// The number of active receivers, or `0` if [`SINGLETON`] is uninitialized.
    ///
    /// The [`mio_poller`] thread exits gracefully when this count reaches `0` (all
    /// receivers dropped). See [Device Lifecycle] in [`DirectToAnsiInputDevice`] for
    /// details.
    ///
    /// [Device Lifecycle]: crate::DirectToAnsiInputDevice#device-lifecycle
    /// [`DirectToAnsiInputDevice`]: crate::DirectToAnsiInputDevice
    /// [`mio_poller`]: crate::direct_to_ansi::input::mio_poller
    pub fn get_receiver_count() -> usize {
        SINGLETON
            .lock()
            .ok()
            .and_then(|guard| {
                guard.as_ref().map(|state| {
                    state
                        .thread_to_singleton_bridge
                        .broadcast_tx
                        .receiver_count()
                })
            })
            .unwrap_or(0)
    }

    /// Returns the current thread generation number.
    ///
    /// Each time a new [`mio_poller`] thread is spawned, the generation increments. This
    /// allows tests to verify whether a thread was reused or relaunched:
    ///
    /// - **Same generation**: Thread was reused (device B subscribed before thread
    ///   exited).
    /// - **Different generation**: Thread was relaunched (a new thread was spawned).
    ///
    /// # Returns
    ///
    /// The current generation number, or `0` if [`SINGLETON`] is uninitialized.
    ///
    /// See [Device Lifecycle] in [`DirectToAnsiInputDevice`] for details on thread
    /// spawn/exit/relaunch.
    ///
    /// [Device Lifecycle]: crate::DirectToAnsiInputDevice#device-lifecycle
    /// [`DirectToAnsiInputDevice`]: crate::DirectToAnsiInputDevice
    /// [`mio_poller`]: crate::direct_to_ansi::input::mio_poller
    pub fn get_thread_generation() -> u8 {
        SINGLETON
            .lock()
            .ok()
            .and_then(|guard| {
                guard.as_ref().map(|state| {
                    state.thread_to_singleton_bridge.thread_liveness.generation
                })
            })
            .unwrap_or(0)
    }

    /// Subscribe to input events from an existing thread.
    ///
    /// This is a lightweight operation that creates a new subscriber to the existing
    /// broadcast channel. Use this for additional consumers (logging, debugging, etc.)
    /// after a [`DirectToAnsiInputDevice`] has been created.
    ///
    /// When the returned handle is dropped, it notifies the [`mio_poller`] thread to
    /// check if it should exit (when all subscribers are dropped, the thread exits).
    ///
    /// # Panics
    ///
    /// - If the [`SINGLETON`] mutex is poisoned (another thread panicked while holding
    ///   the lock).
    /// - If no device exists yet. Call [`allocate`] first.
    ///
    /// [`DirectToAnsiInputDevice`]: crate::DirectToAnsiInputDevice
    /// [`mio_poller`]: super::super::mio_poller
    pub fn subscribe_to_existing() -> InputDeviceSubscriptionHandle {
        let guard = SINGLETON.lock().expect(
            "SINGLETON mutex poisoned: another thread panicked while holding this lock.",
        );

        let state = guard.as_ref().expect(
            "subscribe_to_existing() called before DirectToAnsiInputDevice::new(). \
             Create a device first, then call device.subscribe().",
        );

        InputDeviceSubscriptionHandle {
            maybe_poller_rx: Some(
                state.thread_to_singleton_bridge.broadcast_tx.subscribe(),
            ),
            mio_poller_thread_waker: Arc::clone(&state.waker),
        }
    }
}

/// Receiver wrapper that wakes the [`mio_poller`] thread on drop.
///
/// # Drop behavior
///
/// When this handle is dropped:
/// 1. [`maybe_poller_rx`] is dropped first, which causes Tokio's broadcast channel to
///    atomically decrement the [`Sender`]'s internal [`receiver_count()`].
/// 2. Then [`mio::Waker::wake()`] interrupts the poll loop.
/// 3. The [`mio_poller`] thread wakes and [`handle_receiver_drop_waker()`] checks
///    [`receiver_count()`] to decide if the thread should exit (when count reaches `0`).
///
/// # Race Condition and Correctness
///
/// There is a race window between when the receiver is dropped and when
/// [`handle_receiver_drop_waker()`] checks [`receiver_count()`]. This is the **fast-path
/// thread reuse** scenario — if a new subscriber appears during the window, the thread
/// correctly continues serving it instead of exiting.
///
/// See [`PollerBridge`] for comprehensive documentation:
/// - [The Inherent Race Condition] — timeline diagram
/// - [What Happens If We Exit Blindly] — zombie device scenario
/// - [Why Thread Reuse Is Safe] — resource safety table
///
/// [The Inherent Race Condition]: super::mio_poller::PollerBridge#the-inherent-race-condition
/// [What Happens If We Exit Blindly]: super::mio_poller::PollerBridge#what-happens-if-we-exit-blindly
/// [Why Thread Reuse Is Safe]: super::mio_poller::PollerBridge#why-thread-reuse-is-safe
/// [`PollerBridge`]: super::mio_poller::PollerBridge
/// [`Sender`]: tokio::sync::broadcast::Sender
/// [`handle_receiver_drop_waker()`]: super::mio_poller::handler_receiver_drop::handle_receiver_drop_waker
/// [`maybe_poller_rx`]: Self::maybe_poller_rx
/// [`mio_poller`]: super::mio_poller
/// [`receiver_count()`]: tokio::sync::broadcast::Sender::receiver_count
#[allow(missing_debug_implementations)]
pub struct InputDeviceSubscriptionHandle {
    /// The actual broadcast receiver for poller events.
    pub maybe_poller_rx: Option<PollerEventReceiver>,

    /// Waker to signal the [`mio_poller`] thread.
    ///
    /// [`mio_poller`]: super::mio_poller
    pub mio_poller_thread_waker: Arc<mio::Waker>,
}

impl Drop for InputDeviceSubscriptionHandle {
    /// Drops receiver then wakes thread. See [Drop behavior] for the full mechanism.
    /// Also see [`DirectToAnsiInputDevice`'s drop behavior] for when this is triggered.
    ///
    /// [Drop behavior]: InputDeviceSubscriptionHandle#drop-behavior
    /// [`DirectToAnsiInputDevice`'s drop behavior]: super::DirectToAnsiInputDevice#drop-behavior
    fn drop(&mut self) {
        // Drop receiver first so Sender::receiver_count() decrements.
        drop(self.maybe_poller_rx.take());

        // Wake the thread so it can check if it should exit.
        let wake_result = self.mio_poller_thread_waker.wake();

        // Log failure (non-fatal: thread may have already exited).
        if let Err(err) = wake_result {
            DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                tracing::debug!(
                    message = "InputDeviceSubscriptionHandle::drop: wake failed",
                    error = ?err
                );
            });
        }
    }
}

/// Comprehensive testing is performed in PTY integration tests:
/// - [`test_pty_input_device`]
/// - [`test_pty_mouse_events`]
/// - [`test_pty_keyboard_modifiers`]
/// - [`test_pty_utf8_text`]
/// - [`test_pty_terminal_events`]
///
/// [`test_pty_input_device`]: crate::core::ansi::vt_100_terminal_input_parser::integration_tests::pty_input_device_test::test_pty_input_device
/// [`test_pty_keyboard_modifiers`]: crate::core::ansi::vt_100_terminal_input_parser::integration_tests::pty_keyboard_modifiers_test::test_pty_keyboard_modifiers
/// [`test_pty_mouse_events`]: crate::core::ansi::vt_100_terminal_input_parser::integration_tests::pty_mouse_events_test::test_pty_mouse_events
/// [`test_pty_terminal_events`]: crate::core::ansi::vt_100_terminal_input_parser::integration_tests::pty_terminal_events_test::test_pty_terminal_events
/// [`test_pty_utf8_text`]: crate::core::ansi::vt_100_terminal_input_parser::integration_tests::pty_utf8_text_test::test_pty_utf8_text
#[cfg(test)]
mod tests {
    use super::super::{paste_state_machine::PasteCollectionState,
                       protocol_conversion::convert_input_event};
    use crate::{ByteOffset, InputEvent,
                core::ansi::vt_100_terminal_input_parser::{VT100InputEventIR,
                                                           VT100KeyCodeIR,
                                                           VT100KeyModifiersIR,
                                                           VT100PasteModeIR,
                                                           parse_keyboard_sequence,
                                                           parse_utf8_text}};

    #[tokio::test]
    async fn test_event_parsing() {
        // Test event parsing from buffer - verify parsers handle different sequence
        // types. These tests use the parser functions directly since they don't
        // need the device.

        // Test 1: Parse UTF-8 text (simplest case).
        let buffer: &[u8] = b"A";
        if let Some((vt100_event, bytes_consumed)) = parse_utf8_text(buffer) {
            assert_eq!(bytes_consumed, ByteOffset(1));
            if let Some(canonical_event) = convert_input_event(vt100_event) {
                assert!(matches!(canonical_event, InputEvent::Keyboard(_)));
            } else {
                panic!("Failed to convert UTF-8 text event");
            }
        } else {
            panic!("Failed to parse UTF-8 text 'A'");
        }

        // Test 2: ESC key (single byte).
        let esc_buffer: [u8; 1] = [0x1B];
        assert_eq!(esc_buffer.len(), 1);
        assert_eq!(esc_buffer[0], 0x1B);

        // Test 3: CSI sequence for keyboard (Up Arrow: ESC [ A).
        let csi_buffer: [u8; 3] = [0x1B, 0x5B, 0x41];
        if let Some((vt100_event, bytes_consumed)) = parse_keyboard_sequence(&csi_buffer)
        {
            assert_eq!(bytes_consumed, ByteOffset(3));
            if let Some(canonical_event) = convert_input_event(vt100_event) {
                assert!(matches!(canonical_event, InputEvent::Keyboard(_)));
            } else {
                panic!("Failed to convert keyboard event");
            }
        }
    }

    #[tokio::test]
    async fn test_paste_state_machine_basic() {
        // Test: Basic paste collection - Start marker, text, End marker.
        // Use a local paste_state to test the state machine logic directly.
        let mut paste_state = PasteCollectionState::Inactive;

        // Verify initial state is Inactive.
        assert!(matches!(paste_state, PasteCollectionState::Inactive));

        // Simulate receiving Paste(Start) event.
        let start_event = VT100InputEventIR::Paste(VT100PasteModeIR::Start);
        match (&mut paste_state, &start_event) {
            (
                state @ PasteCollectionState::Inactive,
                VT100InputEventIR::Paste(VT100PasteModeIR::Start),
            ) => {
                *state = PasteCollectionState::Accumulating(String::new());
            }
            _ => panic!("State machine should handle Paste(Start)"),
        }

        // Verify we're now collecting.
        assert!(matches!(paste_state, PasteCollectionState::Accumulating(_)));

        // Simulate receiving keyboard events (the pasted text).
        for ch in &['H', 'e', 'l', 'l', 'o'] {
            let keyboard_event = VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Char(*ch),
                modifiers: VT100KeyModifiersIR::default(),
            };
            match (&mut paste_state, &keyboard_event) {
                (
                    PasteCollectionState::Accumulating(buffer),
                    VT100InputEventIR::Keyboard {
                        code: VT100KeyCodeIR::Char(ch),
                        ..
                    },
                ) => {
                    buffer.push(*ch);
                }
                _ => panic!("State machine should accumulate text while collecting"),
            }
        }

        // Simulate receiving Paste(End) event.
        let end_event = VT100InputEventIR::Paste(VT100PasteModeIR::End);
        let collected_text = match (&mut paste_state, &end_event) {
            (
                state @ PasteCollectionState::Accumulating(_),
                VT100InputEventIR::Paste(VT100PasteModeIR::End),
            ) => {
                if let PasteCollectionState::Accumulating(text) =
                    std::mem::replace(state, PasteCollectionState::Inactive)
                {
                    text
                } else {
                    panic!("Should have collected text")
                }
            }
            _ => panic!("State machine should handle Paste(End)"),
        };

        // Verify we collected the correct text.
        assert_eq!(collected_text, "Hello");

        // Verify we're back to Inactive state.
        assert!(matches!(paste_state, PasteCollectionState::Inactive));
    }

    #[tokio::test]
    async fn test_paste_state_machine_multiline() {
        // Test: Paste with newlines.
        let mut paste_state = PasteCollectionState::Inactive;

        // Start collection.
        match &mut paste_state {
            state @ PasteCollectionState::Inactive => {
                *state = PasteCollectionState::Accumulating(String::new());
            }
            PasteCollectionState::Accumulating(_) => panic!(),
        }

        // Accumulate "Line1\nLine2".
        for ch in "Line1\nLine2".chars() {
            match &mut paste_state {
                PasteCollectionState::Accumulating(buffer) => {
                    buffer.push(ch);
                }
                PasteCollectionState::Inactive => panic!(),
            }
        }

        // End collection.
        let text = match &mut paste_state {
            state @ PasteCollectionState::Accumulating(_) => {
                if let PasteCollectionState::Accumulating(t) =
                    std::mem::replace(state, PasteCollectionState::Inactive)
                {
                    t
                } else {
                    panic!()
                }
            }
            _ => panic!(),
        };

        assert_eq!(text, "Line1\nLine2");
    }

    #[tokio::test]
    async fn test_paste_state_machine_orphaned_end() {
        // Test: Orphaned End marker (without Start) should be handled gracefully.
        let mut paste_state = PasteCollectionState::Inactive;

        // Should be Inactive initially.
        assert!(matches!(paste_state, PasteCollectionState::Inactive));

        // Receive End marker without Start - should emit empty paste.
        let end_event = VT100InputEventIR::Paste(VT100PasteModeIR::End);
        let result = match (&mut paste_state, &end_event) {
            (
                PasteCollectionState::Inactive,
                VT100InputEventIR::Paste(VT100PasteModeIR::End),
            ) => Some(InputEvent::BracketedPaste(String::new())),
            _ => None,
        };

        assert!(matches!(result, Some(InputEvent::BracketedPaste(s)) if s.is_empty()));

        // Should still be Inactive.
        assert!(matches!(paste_state, PasteCollectionState::Inactive));
    }

    #[tokio::test]
    async fn test_paste_state_machine_empty_paste() {
        // Test: Empty paste (Start immediately followed by End).
        let mut paste_state = PasteCollectionState::Inactive;

        // Start.
        match &mut paste_state {
            state @ PasteCollectionState::Inactive => {
                *state = PasteCollectionState::Accumulating(String::new());
            }
            _ => panic!(),
        }

        // End (without any characters in between).
        let text = match &mut paste_state {
            state @ PasteCollectionState::Accumulating(_) => {
                if let PasteCollectionState::Accumulating(t) =
                    std::mem::replace(state, PasteCollectionState::Inactive)
                {
                    t
                } else {
                    panic!()
                }
            }
            _ => panic!(),
        };

        assert_eq!(text, "");
    }
}
