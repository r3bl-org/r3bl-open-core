// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words EINTR SIGWINCH epoll kqueue

//! Core traits for the Resilient Reactor Thread (RRT) pattern.
//!
//! This module defines the trait abstractions that allow the RRT infrastructure to work
//! with any blocking [I/O] mechanism:
//!
//! - [`ThreadWaker`]: How to interrupt a blocked thread
//! - [`ThreadWorker`]: The actual work loop running on the thread
//! - [`ThreadWorkerFactory`]: Creates coupled worker + waker together
//! - [`Continuation`]: Whether to continue or stop the work loop
//!
//! [I/O]: https://en.wikipedia.org/wiki/Input/output

use crate::core::common::Continuation;
use tokio::sync::broadcast::Sender;

/// Waker abstraction for interrupting a blocking thread.
///
/// Each RRT implementation provides its own waker that knows how to interrupt its
/// specific blocking mechanism ([`mio::Poll`], TCP accept, pipe read, etc.).
///
/// # Concrete Implementation
///
/// See [`mio_poller`] for a concrete implementation that uses [`mio::Waker`] to interrupt
/// a thread blocking on [`mio::Poll::poll()`]. That implementation monitors:
/// - **[`stdin`]**: Keyboard/mouse input via terminal
/// - **Signal fd**: Terminal resize via [`SIGWINCH`]
///
/// # Implementor Notes
///
/// The `wake()` method will be called from [`SubscriberGuard::drop()`], potentially from
/// any thread. Implementations must be thread-safe.
///
/// Common implementations:
/// - **[`mio::Waker`]**: Triggers an event that [`mio::Poll::poll()`] returns
/// - **Self-pipe**: Writes a byte to a pipe to interrupt `select()`/`poll()`
/// - **Connect-to-self**: Connects to a listening socket to interrupt [`accept()`]
///
/// # Example
///
/// ```no_run
/// # use r3bl_tui::core::resilient_reactor_thread::ThreadWaker;
/// struct MioPollWaker(mio::Waker);
///
/// impl ThreadWaker for MioPollWaker {
///     fn wake(&self) -> std::io::Result<()> {
///         self.0.wake()
///     }
/// }
/// ```
///
/// [`SIGWINCH`]: signal_hook::consts::SIGWINCH
/// [`SubscriberGuard::drop()`]: super::SubscriberGuard
/// [`accept()`]: std::net::TcpListener::accept
/// [`mio::Poll::poll()`]: mio::Poll::poll
/// [`mio_poller`]: crate::terminal_lib_backends::direct_to_ansi::input::mio_poller
/// [`stdin`]: std::io::stdin
pub trait ThreadWaker: Send + Sync + 'static {
    /// Wake the thread so it can check if it should exit.
    ///
    /// Called by [`SubscriberGuard::drop()`] to signal the thread. The thread then
    /// checks [`receiver_count()`] to decide whether to exit.
    ///
    /// # Errors
    ///
    /// Returns an error if the wake signal cannot be sent. This is typically non-fatal
    /// (the thread may have already exited), but implementations should log failures.
    ///
    /// [`SubscriberGuard::drop()`]: super::SubscriberGuard
    /// [`receiver_count()`]: tokio::sync::broadcast::Sender::receiver_count
    fn wake(&self) -> std::io::Result<()>;
}

/// Worker that runs on a dedicated thread.
///
/// Implements the actual work loop logic. Called repeatedly by the framework until
/// [`Continuation::Stop`] is returned.
///
/// # Concrete Implementation
///
/// See [`mio_poller`] for a concrete implementation that:
/// - Blocks on [`mio::Poll::poll()`] waiting for [`stdin`] or [`SIGWINCH`]
/// - Parses raw bytes into [`InputEvent`]s via [`try_parse_input_event()`]
/// - Broadcasts [`PollerEvent`]s to async consumers
///
/// # Design Rationale
///
/// The `poll_once() → Continuation` design (vs a `run()` method that owns the loop)
/// provides:
///
/// - **Framework control**: Can inject logging, metrics, health checks between iterations
/// - **Single responsibility**: Worker handles events, framework handles lifecycle
/// - **Testability**: Can unit test `poll_once()` in isolation
///
/// # Implementor Notes
///
/// The `poll_once()` method should:
/// 1. Block waiting for events (poll, select, recv, etc.)
/// 2. Process ready events, broadcasting via `tx`
/// 3. Check shutdown conditions and return [`Continuation::Stop`] when appropriate
///
///
/// # Example
///
/// ```no_run
/// # use r3bl_tui::core::resilient_reactor_thread::ThreadWorker;
/// # use r3bl_tui::Continuation;
/// # use tokio::sync::broadcast::Sender;
/// # use std::io::ErrorKind;
/// #
/// # #[derive(Clone)]
/// # struct PollerEvent;
/// # struct MioPollWorker {
/// #     poll: mio::Poll,
/// #     events: mio::Events,
/// #     saw_wake_token: bool,
/// # }
/// impl ThreadWorker for MioPollWorker {
///     type Event = PollerEvent;
///
///     fn poll_once(&mut self, tx: &Sender<Self::Event>) -> Continuation {
///         // Block on mio::Poll
///         match self.poll.poll(&mut self.events, None) {
///             Ok(()) => {
///                 for event in &self.events {
///                     // Process event, maybe broadcast
///                     let _ = tx.send(PollerEvent);
///                 }
///                 // Check if we should exit
///                 if tx.receiver_count() == 0 && self.saw_wake_token {
///                     return Continuation::Stop;
///                 }
///                 Continuation::Continue
///             }
///             // EINTR: syscall interrupted by signal, safe to retry
///             Err(e) if e.kind() == ErrorKind::Interrupted => {
///                 Continuation::Continue
///             }
///             Err(_) => Continuation::Stop,
///         }
///     }
/// }
/// ```
///
/// [`InputEvent`]: crate::InputEvent
/// [`PollerEvent`]: crate::terminal_lib_backends::direct_to_ansi::input::channel_types::PollerEvent
/// [`SIGWINCH`]: signal_hook::consts::SIGWINCH
/// [`mio::Poll::poll()`]: mio::Poll::poll
/// [`mio_poller`]: crate::terminal_lib_backends::direct_to_ansi::input::mio_poller
/// [`stdin`]: std::io::stdin
/// [`try_parse_input_event()`]: crate::core::ansi::vt_100_terminal_input_parser::try_parse_input_event
pub trait ThreadWorker: Send + 'static {
    /// Event type this worker produces.
    ///
    /// Must be `Clone + Send + 'static` for the broadcast channel.
    type Event: Clone + Send + 'static;

    /// Run one iteration of the work loop.
    ///
    /// Called in a loop by the framework. Return [`Continuation::Continue`] to keep
    /// running, or [`Continuation::Stop`] to exit the thread.
    ///
    /// The `tx` sender is provided for broadcasting events to subscribers. Use
    /// [`tx.receiver_count()`] to check if anyone is still listening.
    ///
    /// [`tx.receiver_count()`]: tokio::sync::broadcast::Sender::receiver_count
    fn poll_once(&mut self, tx: &Sender<Self::Event>) -> Continuation;
}

/// Factory that creates coupled worker and waker together.
///
/// Solves the **chicken-egg problem** where waker creation depends on resources that the
/// worker owns. For example, with mio: a [`mio::Waker`] needs the [`mio::Poll`]'s
/// registry to be created, but the `Poll` must move to the spawned thread. Meanwhile,
/// [`ThreadState`] needs the waker stored so subscribers can call [`wake()`].
///
/// The solution is that [`setup()`] creates **both** together: the worker (which owns
/// `Poll`) moves to the spawned thread, while the waker (created from `Poll`'s registry)
/// is stored in [`ThreadState`].
///
/// # Concrete Implementation
///
/// See [`mio_poller`] for a concrete factory that:
/// 1. Creates [`mio::Poll`] (OS event mechanism: [`epoll`] on Linux, [`kqueue`] on macOS)
/// 2. Creates [`mio::Waker`] from the Poll's registry
/// 3. Registers [`stdin`] and signal fd ([`SIGWINCH`]) with the Poll
/// 4. Returns worker (owns Poll) and waker (stored in [`ThreadState`])
///
///
/// # Implementor Notes
///
/// The `setup()` method should:
/// 1. Create any OS resources (Poll, sockets, pipes)
/// 2. Create the waker from those resources
/// 3. Create the worker with whatever it needs
/// 4. Return both together
///
/// The framework will then:
/// - Store the waker in [`ThreadState`] (shared via Arc)
/// - Move the worker to a new thread
/// - Run the worker's `poll_once()` in a loop
///
/// # Example
///
/// ```no_run
/// # use r3bl_tui::core::resilient_reactor_thread::{ThreadWaker, ThreadWorker, ThreadWorkerFactory};
/// # use r3bl_tui::Continuation;
/// # use tokio::sync::broadcast::Sender;
/// #
/// # const WAKE_TOKEN: mio::Token = mio::Token(0);
/// # #[derive(Clone)]
/// # struct PollerEvent;
/// # struct MioPollWaker(mio::Waker);
/// # impl ThreadWaker for MioPollWaker {
/// #     fn wake(&self) -> std::io::Result<()> { self.0.wake() }
/// # }
/// # struct MioPollWorker;
/// # impl MioPollWorker {
/// #     fn new(_poll: mio::Poll) -> std::io::Result<Self> { Ok(Self) }
/// # }
/// # impl ThreadWorker for MioPollWorker {
/// #     type Event = PollerEvent;
/// #     fn poll_once(&mut self, _tx: &Sender<Self::Event>) -> Continuation { todo!() }
/// # }
/// #
/// struct MioPollWorkerFactory;
///
/// impl ThreadWorkerFactory for MioPollWorkerFactory {
///     type Event = PollerEvent;
///     type Worker = MioPollWorker;
///     type Waker = MioPollWaker;
///     type SetupError = std::io::Error;
///
///     fn setup() -> Result<(Self::Worker, Self::Waker), Self::SetupError> {
///         let poll = mio::Poll::new()?;
///         let waker = mio::Waker::new(poll.registry(), WAKE_TOKEN)?;
///         let worker = MioPollWorker::new(poll)?;
///         Ok((worker, MioPollWaker(waker)))
///     }
/// }
/// ```
///
/// [`SIGWINCH`]: signal_hook::consts::SIGWINCH
/// [`ThreadState`]: super::ThreadState
/// [`epoll`]: https://man7.org/linux/man-pages/man7/epoll.7.html
/// [`kqueue`]: https://man.freebsd.org/cgi/man.cgi?query=kqueue
/// [`mio_poller`]: crate::terminal_lib_backends::direct_to_ansi::input::mio_poller
/// [`setup()`]: Self::setup
/// [`stdin`]: std::io::stdin
/// [`wake()`]: ThreadWaker::wake
pub trait ThreadWorkerFactory: Send + 'static {
    /// Event type broadcast to subscribers.
    type Event: Clone + Send + 'static;

    /// Worker type that runs on the thread.
    type Worker: ThreadWorker<Event = Self::Event>;

    /// Waker type for interrupting the worker.
    type Waker: ThreadWaker;

    /// Error type for setup failures.
    ///
    /// Must implement [`std::error::Error`] + [`Send`] + [`Sync`] for use with
    /// [`miette::IntoDiagnostic`].
    type SetupError: std::error::Error + Send + Sync + 'static;

    /// Create worker and waker together.
    ///
    /// - Worker → moves to spawned thread
    /// - Waker → stored in [`ThreadState`] for [`SubscriberGuard`]s to call `wake()`
    ///
    /// # Errors
    ///
    /// Returns an error if OS resources cannot be created (file descriptors, sockets,
    /// etc.). The error will be propagated to the caller of
    /// [`ThreadSafeGlobalState::allocate()`].
    ///
    /// [`SubscriberGuard`]: super::SubscriberGuard
    /// [`ThreadSafeGlobalState::allocate()`]: super::ThreadSafeGlobalState::allocate
    /// [`ThreadState`]: super::ThreadState
    fn setup() -> Result<(Self::Worker, Self::Waker), Self::SetupError>;
}
