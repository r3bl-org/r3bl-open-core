// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! # [`PTY`] Module
//!
//! This module provides a high-level, synchronous interface (with an opt-in async
//! adapter) for spawning and controlling processes in [pseudoterminals] ([`PTY`]s). It is
//! designed to be the foundational engine for terminal multiplexers (like [`tmux`]),
//! interactive shells, and TUI applications.
//!
//! ## The Developer's Journey
//!
//! Imagine you're building a terminal multiplexer (like [`tmux`]), or a coding agent
//! (like [`agy-cli`]) in which you must run programs and capture their output while
//! providing them input (and these programs must think they are running in an interactive
//! terminal). You will need to handle the following tasks:
//!
//! 1. **Spawning**: You need to start a shell process (like [`bash`]) inside a [`PTY`] so
//!    it thinks it's talking to a real terminal (see [What is a `TTY`]).
//! 2. **Orchestration**: You need to manage the lifecycle of that shell process,
//!    capturing its output while sending your keystrokes (from your app) to it (as if
//!    this input was going directly into the managed shell process).
//! 3. **Multiplexing**: You might want to provide the ability to spawn multiple shell
//!    processes, and switch between them instantly, keeping each shell's state alive even
//!    when it is not visible.
//!
//! This module provides the building blocks for this journey, organized into a
//! three-layer functional stack.
//!
//! ## The [`PTY`] Core Concept
//!
//! At its core, a [`PTY`] (pseudoterminal) provides an emulated terminal device without
//! requiring physical hardware. It consists of a pair (one controller and one controlled)
//! of file descriptors ([`fd`]) connected by the OS kernel:
//!
//! 1. The app you're building (like [`tmux`]) holds the **controller [`fd`]** (a single
//!    bidirectional descriptor) to write input and read output.
//! 2. The child process (that your app launches, e.g., `top`) holds the **controlled
//!    [`fd`]** (mapped by the kernel to [`stdin`], [`stdout`], and [`stderr`]). To the
//!    child, this device is indistinguishable from a real hardware terminal ([`isatty()`]
//!    returns `true`).
//!
//! The OS kernel connects them bidirectionally:
//! - Reading from the controller drains bytes emitted by the child process on [`stdout`]
//!   ([`fd`] `1`) and [`stderr`] ([`fd`] `2`). Both are multiplexed into the same stream,
//!   which isn't intuitive. So, when the child process calls [`println!`] or
//!   [`eprintln!`], the output enters the same merged stream. The controller cannot
//!   distinguish which bytes were written to [`stdout`] versus [`stderr`].
//! - Writing to the controller feeds bytes directly into the child process's [`stdin`]
//!   ([`fd`] `0`).
//!
//! ```text
//!   CONTROLLER FD                                 CONTROLLED FD
//!  (Our Application)       OS KERNEL CONDUIT     (Child Process)
//! ┌─────────────────┐                           ┌─────────────────┐
//! │  write (input)  │──────────────────────────►│  stdin          │
//! │                 │                           ├─────────────────┤
//! │                 │                           │  stdout         │
//! │  read (output)  │◄──── [ Gate 1 Buffer ] ◄──┤    + (merged)   │
//! │                 │      (4 KB to 64 KB)      │  stderr         │
//! └─────────────────┘                           └─────────────────┘
//! ```
//!
//! ## The Functional Stack
//!
//! The [`PTY`] module is structured into three distinct layers of abstraction, with a 1-1
//! mapping to each step in the developer's journey shown above. However, it is inverted:
//! step 1 in the journey maps to the last layer below.
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────────────────┐
//! │                              APPLICATION LAYER             (3. Multiplexing) │
//! │         (e.g., PTY Mux, TUI App, Terminal Emulator, readline_async App)      │
//! └──────────────────────────────────────┬───────────────────────────────────────┘
//!                                        │
//! ┌──────────────────────────────────────▼───────────────────────────────────────┐
//! │                                SESSION LAYER              (2. Orchestration) │
//! │        (Thread orchestration, event channels, OSC, opt-in async adapter)     │
//! │              ┌───────────────────────────────────────────────┐               │
//! │              │           mod pty_session (Session layer)     │               │
//! │              └───────────────────────────────────────────────┘               │
//! └──────────────────────────────────────┬───────────────────────────────────────┘
//!                                        │
//! ┌──────────────────────────────────────▼───────────────────────────────────────┐
//! │                                 ENGINE LAYER                   (1. Spawning) │
//! │          (OS-level PTY pair creation, deadlock prevention, I/O)              │
//! │              ┌───────────────────────────────────────────────┐               │
//! │              │         mod pty_engine (PtyPair, etc.)        │               │
//! │              └───────────────────────────────────────────────┘               │
//! └──────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! 1. **Application Layer**: This is your code. Whether it's the built-in [`PTYMux`] or
//!    your custom [`TUI`] or [`readline_async`] app, this layer consumes events from the
//!    Session Layer and sends inputs generated by the end user interacting with your app,
//!    to the Session Layer.
//! 2. **[Session Layer]**: The thread orchestration layer. It bridges the gap between
//!    synchronous OS [`PTY`] I/O and your application using dedicated OS threads and
//!    synchronous channels (with an optional Tokio async adapter). This is where the
//!    **Thread Trio** lives.
//! 3. **[Engine Layer]**: The low-level foundation. It handles the tricky business of
//!    opening OS [`PTY`] pairs and programmatically preventing deadlocks caused by leaked
//!    file descriptors. See [`PtyPair`] for details (and the [`PTY` Primer]).
//!
//! ## The Life of a Session
//!
//! Understanding the lifecycle of a [`PTY`] session, from its birth to its eventual
//! teardown, is essential for building robust [`TUI`] and [`readline_async`] applications
//! (that use this [`pty` module]). This journey involves coordinated interactions across
//! all three layers of the functional stack.
//!
//! ### 1. Birth (Spawning)
//!
//! It all begins when your app ([`TUI`] or [`readline_async`]) uses
//! [`PtySessionBuilder::start()`] to start a session. This kicks off the **initialization
//! sequence** across all three layers.
//!
//! - **Session Layer**: [`PtySessionBuilder::start()`] orchestrates the entire startup.
//!   First it uses the **Engine Layer** to spawn the child process in the [`PTY`]
//!   environment (see [Child process perspective]), then wraps it in the **Thread Trio**
//!   ([Reader], [Writer], [Orchestrator]), and finally sets up the [MPSC channels].
//! - **Engine Layer**: Called by the **Session Layer**, [`PtyPair::open_and_spawn()`]
//!   creates the OS-level [`PTY`] pair and spawns the child process. Crucially, it
//!   immediately drops the parent's copy of the **controlled** file descriptor to prevent
//!   [resource-leaking deadlocks] at birth.
//! - **App Layer**: Once the initialization sequence is complete, your application
//!   receives a [`PtySession`] handle, ready to begin interaction.
//!
//! ### 2. Life (Interaction)
//!
//! Once the session is running, and your app is running, a balanced bidirectional data
//! flow begins.
//!
//! - **App Layer**: Sends to the session and receives events from it:
//!   - Sends [`PtyInputEvent`]s, e.g., keyboard input, and resize requests. See
//!     [`ProcessManager::send_input()`] and [`ProcessManager::handle_terminal_resize()`])
//!     for examples.
//!   - Receives [`PtyOutputEvent`]s, e.g., process output, [`OSC`] sequences, and exit
//!     status. See [`ProcessManager::poll_all_processes()`]) for examples.
//! - **Session Layer**: The thread trio works to connect the app layer to the engine.
//!   - The [Writer Thread] pumps input events to the engine.
//!   - The [Reader Thread] drains output from the engine.
//!   - The [Orchestrator Thread] monitors the child process's lifecycle.
//! - **Engine Layer**: Manages the low-level [`Controller`] and **controlled** process
//!   I/O.
//!
//! ### 3. The Great Beyond (Teardown & Cleanup)
//!
//! A session can end in two ways: gracefully (initiated by the child) or forcefully
//! (initiated by your app).
//!
//! #### Child-Initiated (Graceful Exit)
//!
//! 1. **Child Process Exits**: The process running inside the [`PTY`] (e.g., a shell
//!    after an `exit` command) terminates.
//! 2. **Engine Layer**: The OS closes the **controlled** side of the [`PTY`]. Because
//!    [`PtyPair`] already dropped its copy of the controlled FD at birth, the kernel's
//!    reference count hits zero.
//! 3. **Signal**: The kernel sends a termination signal: [`EOF`] (or [`EIO`] on Linux) to
//!    the [`Controller`] reader.
//! 4. **Session Layer**:
//!    - The [Reader Thread] sees this signal and exits cleanly.
//!    - The [Orchestrator Thread] (which was waiting on [`ControlledChild::wait()`])
//!      detects the exit.
//!    - It joins both the [Reader] and [Writer] threads to ensure all I/O is drained.
//!    - Finally, it sends a [`PtyOutputEvent::Exit`] event to the App.
//!
//! #### App-Initiated (Forced Shutdown)
//!
//! When your application needs to shut down (e.g., the user quits your app), it should
//! follow this **Critical Shutdown Pattern**:
//!
//! 1. **Kill**: Forcefully terminate the child process using the
//!    [`PtySession::child_process_termination_handle`].
//! 2. **Close**: Send a [`PtyInputEvent::Close`] to signal the [Writer Thread] to stop.
//! 3. **Drop**: Drop the [`PtySession`] handle.
//!
//! #### The Final Cleanup (RAII and the Drop Chain)
//!
//! When your app drops the [`PtySession`] handle, Rust's automatic resource management
//! ([`RAII`]) triggers a cleanup chain across all internal components:
//!
//! - **Session Layer**:
//!   - The [MPSC channels] are closed when their halves are dropped.
//!   - The thread trio join handles are dropped in a cascading chain:
//!     - [`PtySession`] drops the [Orchestrator Thread] handle.
//!     - The Orchestrator, in turn, drops its internal [Reader Thread] and [Writer
//!       Thread] handles.
//! - **Engine Layer**:
//!   - The [`Controller`] (from the [Engine Layer]) is dropped.
//!   - The [`PtyPair`] (and its inner [`MasterPty`]) leverages [`RAII`] to guarantee that
//!     all OS-level file descriptors are closed. This eliminates the risk of
//!     [resource-leaking deadlocks] and ensures a clean system state.
//!
//! ## The Thread Trio
//!
//! Every active [`PTY`] session is powered by a set of specialized threads (the **Thread
//! Trio**).
//!
//! | Thread           | Role             | Type         | Responsibility                                                                                        |
//! | :--------------- | :--------------- | :----------- | :---------------------------------------------------------------------------------------------------- |
//! | **Reader**       | 📥 [`PTY`] ➜ App | **Thread**   | [`spawn_pty_reader_thread()`]: Reads raw bytes, processes [`OSC`], sends events to App.               |
//! | **Writer**       | 📤 App ➜ [`PTY`] | **Thread**   | [`spawn_pty_writer_thread()`]: Receives input events from App and writes them to [`stdin`].           |
//! | **Orchestrator** | 🏁 Lifecycle     | **Thread**   | [`spawn_pty_orchestrator_thread()`]: Manages child process, waits for exit, and coordinates shutdown. |
//!
//! ## Technical Deep Dive
//!
//! ### Session Architecture
//!
//! The **Thread Trio** provides bidirectional communication between your app and the
//! child process.
//!
//! ```text
//!        ┌───────────────────────────────────────────────────────────────────────┐
//! ┌──────▼───────┐   ┌──────────────────────────────────────────┐                │
//! │ Your Program │   │          Orchestrator Thread (1)         │                │
//! │ (App Layer)  │   │            ↙               ↘             │                │
//! │ Sends Input  ├───► Writer Thread (3)     Reader Thread (2)  ── Sends Output ─┘
//! │ Events       │   │ (App → PTY)             (PTY → App)      │  Events
//! │              │   └──────┬────────────────────────────▲──────┘
//! │              │          │                            │
//! │              │   ┌──────▼────────────────────────────┴──────┐
//! │              │   │ ┊Controller┊  ← PTY Pair →  ┊Controlled┊ │
//! └──────────────┘   └──────┬────────────────────────────▲──────┘
//!                           │                            │
//!                    ┌──────▼────────────────────────────┴──────┐
//!                    │            Child Process                 │
//!                    └──────────────────────────────────────────┘
//! Legends:
//! (1): The **Orchestrator** thread ([`pty-orchestrator`]): spawns other threads, manages lifecycle, and coordinates shutdown.
//! (2): The **Reader** thread ([`pty-reader`]): reads output from PTY and sends events to App.
//! (3): The **Writer** thread ([`pty-writer`]): receives input from App and writes to PTY.
//! ```
//!
//! ### Thread Coordination & Lifecycle
//!
//! | Time | Orchestrator Thread                 | Reader Thread    | Writer Thread       |
//! | :--- | :---------------------------------- | :--------------- | :------------------ |
//! | 0    | 🛫 Spawn child                      |                  |                     |
//! | 1    | 🛫 Spawn threads                    | 🛫 Start read    | 🛫 Start            |
//! | 2    | 🛬 Wait [`ControlledChild::wait()`] | 📖 Read data     | 📥 Wait input       |
//! | 3    | 🛬 Wait threads                     | 🛬 Exit (on EOF) | 🛬 Exit (on Close)  |
//! | 4    | 📤 Send Exit event                  |                  |                     |
//! | 5    | ✅ Return status                    |                  |                     |
//!
//! Legends:
//! ```txt
//! Thread lifecycle:  🛫 start  | 🛬 wait/exit | ✅ done
//! IO operations:     📖 read   | ✍️ write
//! Send/receive pair: 📤 send   | 📥 receive
//! ```
//!
//! ### Channel Architecture
//!
//! ```text
//! Your Program
//!      │
//!      │ (input events)
//!      │
//! ┌────▼────────────────────────────────┐
//! │    Input Channel                    │
//! │    (bounded sync channel)           │
//! └─────────────────────────────────────┘
//!      │
//!      │ (Writer Thread)
//!      │
//! ┌────▼────────────────────────────────┐
//! │    PTY Controller                   │
//! │    (write to spawned)               │
//! └─────────────────────────────────────┘
//!      │
//! ┌────▼────────────────────────────────┐
//! │    Spawned Process Input            │
//! └─────────────────────────────────────┘
//!
//! ──── Spawned process boundary ────────
//!
//! ┌─────────────────────────────────────┐
//! │    Spawned Process Output           │
//! └─────────────────────────────────────┘
//!      │
//!      │
//! ┌────▼────────────────────────────────┐
//! │    PTY Controller                   │
//! │    (read from spawned)              │
//! └─────────────────────────────────────┘
//!      │
//!      │ (Reader Thread)
//!      │
//! ┌────▼────────────────────────────────┐
//! │     Output Channel                  │
//! │     (bounded sync channel)          │
//! └─────────────────────────────────────┘
//!      │
//!      │ (output events)
//!      │
//!      ▼
//! Your Program
//! ```
//!
//! ### Backpressure Architecture
//!
//! As illustrated in the [PTY Core Concept] conduit diagram above, the OS kernel connects
//! the controller and controlled file descriptors via an internal kernel conduit. Flow
//! control between your application and the child process is managed across two
//! cooperative gates:
//!
//! 1. **Gate 1: The OS Kernel [`PTY`] Buffer (Kernel Space)**:
//!    - The OS kernel buffers output between the two file descriptors (typically [`4 KB`]
//!      on Linux PTYs, and up to [`64 KB`] on Windows [`ConPTY`] pipes).
//!    - When full, the OS kernel blocks the child's [`write()`] system call: the child is
//!      "Put to Sleep" / "Blocked" ([`TASK_INTERRUPTIBLE`], State `S`), consuming zero
//!      CPU while remaining fully responsive to kill signals and Ctrl+C.
//! 2. **Gate 2: The Synchronous Channel Buffer (User Space)**:
//!    - The dedicated reader thread ([`pty-reader`]) drains the controller and forwards
//!      events via [`SyncSender::send()`].
//!    - The channel capacity is bounded by [`DefaultSize::PtyChannelBufferSize`].
//!    - If the application falls behind draining [`OutputEventReceiverHalf`], Gate 2
//!      fills up and [`SyncSender::send()`] blocks the [`pty-reader`] thread, stopping it
//!      from draining the controller and allowing Gate 1 to fill up.
//!
//! ## Why Sync and not Async?
//!
//! The original implementation of this module was built around Tokio tasks and
//! asynchronous streams. However, following the principle of fit-for-purpose design
//! rather than "async-by-default" instinct, we converted the core of this engine to
//! synchronous OS threads and bounded standard library channels.
//!
//! ### 1:1 Process Pipes vs. 1:N Network Servers
//!
//! High-concurrency async runtimes like Tokio are purpose-built for 1:N network servers
//! multiplexing tens of thousands of concurrent sockets using non-blocking OS readiness
//! events ([`epoll`], [`kqueue`], [`IOCP`]).
//!
//! In contrast, a [`PTY`] session manages a 1:1 local process pipe. On Unix and Windows,
//! file descriptors and pipe I/O cannot be truly non-blocking in the same way network
//! sockets are; async runtimes frequently delegate file and pipe I/O to native OS thread
//! pools under the hood anyway. Forcing a 1:1 pipe into an asynchronous runtime
//! introduces [accidental complexity] (without any performance benefits):
//!
//! - Runtime and Startup Overhead: Spawning an async runtime carries non-trivial cold
//!   start latency, thread pool spin-up, and timer wheel setup. Pure OS threads start in
//!   sub-millisecond time.
//! - Cancellation Safety and Stream Corruption: Dropping an async future at an `.await`
//!   point (e.g., within a [`select!`] branch) can abort read or write operations
//!   mid-stream, leading to dropped bytes, corrupted state, or orphaned child processes.
//! - Deterministic Cleanup: With synchronous threads, Rust's [`RAII`] guarantees clean,
//!   linear teardown. Dropping the session handle cascades through the thread handles,
//!   joins all background threads, closes OS file descriptors, and prevents zombie
//!   processes.
//! - Transparent Debugging: Synchronous stack traces are straightforward and direct. They
//!   avoid the nested state machine frames and future poll wrappers typical of async
//!   runtimes.
//!
//! ### Best of Both Worlds: Opt-In Async Adapter
//!
//! Choosing a synchronous core does not mean abandoning asynchronous applications.
//! Applications driving complex interactive interfaces (such as terminal multiplexers or
//! multi-stream event loops) can opt into the async interface via
//! [`PtySessionBuilder::start_async()`], which returns an [`AsyncPtySession`]. This
//! adapter uses lightweight bridge tasks to map synchronous channels into [`tokio`]
//! channels for seamless integration with [`select!`].
//!
//! For an in-depth exploration of the engineering trade-offs between synchronous threads
//! and async runtimes for 1:1 process pipes, see the article: [To Async or Not to Async:
//! Building a Rust MCP Server for rust-analyzer].
//!
//! ## Terminal Emulation & [`terminfo`] Masquerading
//!
//! Because the internal rendering engine ([`DirectToAnsi`]) speaks raw, standard
//! [`ANSI`], the [`pty_mux`] does not require you to create or distribute a custom
//! [`terminfo`] database (something like `TERM=r3bl-pty`).
//!
//! Instead, the engine relies on **masquerading** as a standard terminal emulator (e.g.,
//! [`xterm-256color`]). This tricks the child process into behaving as if it were running
//! in that emulator, leveraging the host OS's existing [`terminfo`] database. This design
//! provides several major benefits:
//!
//! 1. **Universal Compatibility**: Every CLI app (`vim`, `htop`, `bat`) already has
//!    battle-tested support for [`xterm-256color`].
//! 2. **Zero Configuration**: Users do not need to configure their child apps or shell
//!    profiles to recognize a custom [`TERM`] variable.
//! 3. **Zero Deployment Dependencies**: It avoids the need for users to manually install
//!    a custom [`terminfo`] database (like `r3bl-pty`) into `/usr/share/terminfo/` on
//!    every host machine (which requires root access).
//!
//! Here are the technical details of how masquerading works:
//! - By default, child processes inherit the parent's environment (including [`TERM`]).
//! - If specific capabilities are needed (like [`OSC 9`] progress bars from [`cargo`]),
//!   the caller can explicitly inject an environment variable like
//!   [`TERM=xterm-256color`] into the child session via the [`PtySessionBuilder`].
//! - The child process will query its local OS [`terminfo`] database for
//!   [`xterm-256color`], emit standard [`ANSI`] sequences, and the [`pty_mux`] will parse
//!   them.
//!
//! > **Note on TUI Rendering**: While child processes use [`terminfo`] masquerading, our
//! > own rendering engine entirely bypasses [`terminfo`]. See the [`direct_to_ansi` mod
//! > docs: Bypassing `terminfo`] section for why this provides robustness over SSH.
//!
//! ## Main Types
//!
//! - [`PtySessionBuilder`]: Builder for configuring and starting sessions via [`start()`]
//!   or [`start_async()`].
//! - [`PtySession`]: Synchronous [`PTY`] session handle.
//! - [`AsyncPtySession`]: Asynchronous [`PTY`] session handle.
//! - [`PtyOutputEvent`]: Unified events received from [`PTY`] processes.
//! - [`PtyInputEvent`]: Input types that can be sent to interactive sessions.
//!
//! [`4 KB`]: https://elixir.bootlin.com/linux/latest/source/drivers/tty/n_tty.c
//! [`64 KB`]: https://man7.org/linux/man-pages/man7/pipe.7.html#PIPE_CAPACITY
//! [`agy-cli`]: https://antigravity.google/product/antigravity-cli
//! [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
//! [`AsyncPtySession`]: crate::AsyncPtySession
//! [`bash`]: https://www.gnu.org/software/bash/
//! [`cargo`]: https://github.com/rust-lang/cargo
//! [`ConPTY`]:
//!     https://learn.microsoft.com/en-us/windows/console/creating-a-pseudoconsole-session
//! [`ControlledChild::wait()`]: portable_pty::Child::wait
//! [`DefaultSize::PtyChannelBufferSize`]: crate::DefaultSize::PtyChannelBufferSize
//! [`direct_to_ansi` mod docs: Bypassing `terminfo`]:
//!     mod@crate::tui::terminal_lib_backends::direct_to_ansi#architecture-note-bypassing-terminfo
//! [`DirectToAnsi`]: crate::tui::TerminalLibBackend::DirectToAnsi
//! [`EIO`]: https://man7.org/linux/man-pages/man3/errno.3.html
//! [`EOF`]: https://en.wikipedia.org/wiki/End-of-file
//! [`epoll`]: https://man7.org/linux/man-pages/man7/epoll.7.html
//! [`eprintln!`]: std::eprintln
//! [`fd`]: https://man7.org/linux/man-pages/man2/open.2.html
//! [`IOCP`]: https://learn.microsoft.com/en-us/windows/win32/fileio/i-o-completion-ports
//! [`isatty()`]: https://man7.org/linux/man-pages/man3/isatty.3.html
//! [`kqueue`]: https://man.freebsd.org/cgi/man.cgi?query=kqueue
//! [`MasterPty`]: portable_pty::MasterPty
//! [`OSC 9`]: crate::osc_codes::OscSequence
//! [`OSC`]: crate::osc_codes::OscSequence
//! [`OutputEventReceiverHalf`]: crate::OutputEventReceiverHalf
//! [`println!`]: std::println
//! [`ProcessManager::handle_terminal_resize()`]:
//!     crate::ProcessManager::handle_terminal_resize
//! [`ProcessManager::poll_all_processes()`]: crate::ProcessManager::poll_all_processes
//! [`ProcessManager::send_input()`]: crate::ProcessManager::send_input
//! [`pty-orchestrator`]:
//!     crate::pty_session::threads::orchestrator::spawn_pty_orchestrator_thread
//! [`pty-reader`]: crate::pty_session::threads::reader::spawn_pty_reader_thread
//! [`pty-writer`]: crate::pty_session::threads::writer::spawn_pty_writer_thread
//! [`pty_mux`]: mod@crate::core::pty::pty_mux
//! [`pty` module]: mod@crate::core::pty
//! [`PTY` Primer]: crate::pty_engine::pty_pair::PtyPair#pty-primer
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`PtyInputEvent`]: crate::PtyInputEvent
//! [`PTYMux`]: crate::PTYMux
//! [`PtyOutputEvent::Exit`]: crate::PtyOutputEvent::Exit
//! [`PtyOutputEvent`]: crate::PtyOutputEvent
//! [`PtyPair::open_and_spawn()`]: crate::PtyPair::open_and_spawn
//! [`PtyPair`]: crate::PtyPair
//! [`PtySession::child_process_termination_handle`]:
//!     field@crate::PtySession::child_process_termination_handle
//! [`PtySession`]: crate::PtySession
//! [`PtySessionBuilder::start()`]: crate::PtySessionBuilder::start
//! [`PtySessionBuilder::start_async()`]: crate::PtySessionBuilder::start_async
//! [`PtySessionBuilder`]: crate::PtySessionBuilder
//! [`RAII`]: https://en.wikipedia.org/wiki/Resource_acquisition_is_initialization
//! [`readline_async`]: crate::readline_async::ReadlineAsyncContext::try_new
//! [`select!`]: tokio::select
//! [`spawn_pty_orchestrator_thread()`]:
//!     crate::pty_session::threads::orchestrator::spawn_pty_orchestrator_thread
//! [`spawn_pty_reader_thread()`]:
//!     crate::pty_session::threads::reader::spawn_pty_reader_thread
//! [`spawn_pty_writer_thread()`]:
//!     crate::pty_session::threads::writer::spawn_pty_writer_thread
//! [`start()`]: [`PtySessionBuilder::start()`]
//! [`start_async()`]: crate::PtySessionBuilder::start_async
//! [`stderr`]: std::io::stderr
//! [`stdin`]: std::io::Stdin
//! [`stdout`]: std::io::stdout
//! [`SyncSender::send()`]: std::sync::mpsc::SyncSender::send
//! [`TASK_INTERRUPTIBLE`]:
//!     https://man7.org/linux/man-pages/man1/ps.1.html#PROCESS_STATE_CODES
//! [`TERM=xterm-256color`]: https://en.wikipedia.org/wiki/Xterm#256-color_mode
//! [`TERM`]: https://man7.org/linux/man-pages/man7/term.7.html
//! [`terminfo`]: https://en.wikipedia.org/wiki/Terminfo
//! [`tmux`]: https://github.com/tmux/tmux
//! [`tokio`]: tokio
//! [`TUI`]: crate::tui::TerminalWindow::main_event_loop
//! [`write()`]: https://man7.org/linux/man-pages/man2/write.2.html
//! [`xterm-256color`]: https://en.wikipedia.org/wiki/Xterm#256-color_mode
//! [accidental complexity]: https://www.youtube.com/watch?v=Cum5uN2634o
//! [Child process perspective]:
//!     crate::pty_engine::pty_pair::PtyPair#child-process-perspective
//! [Engine Layer]: crate::pty_engine
//! [MPSC channels]: std::sync::mpsc
//! [Orchestrator Thread]:
//!     crate::pty_session::threads::orchestrator::spawn_pty_orchestrator_thread
//! [Orchestrator]:
//!     crate::pty_session::threads::orchestrator::spawn_pty_orchestrator_thread
//! [pseudoterminals]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [PTY Core Concept]: #the-pty-core-concept
//! [Reader Thread]: crate::pty_session::threads::reader::spawn_pty_reader_thread
//! [Reader]: crate::pty_session::threads::reader::spawn_pty_reader_thread
//! [resource-leaking deadlocks]:
//!     crate::pty_engine::pty_pair::PtyPair#resource-leaking-deadlock
//! [Session Layer]: crate::pty_session
//! [Session layer]: crate::pty_session
//! [To Async or Not to Async: Building a Rust MCP Server for rust-analyzer]:
//!     https://developerlife.com/2026/08/22/to-async-or-not-to-async-rust-mcp-server/
//! [What is a `TTY`]: crate::pty_engine::pty_pair::PtyPair#what-is-a-tty
//! [Writer Thread]: crate::pty_session::threads::writer::spawn_pty_writer_thread
//! [Writer]: crate::pty_session::threads::writer::spawn_pty_writer_thread

#![rustfmt::skip]

// Attach.
pub mod pty_engine;
pub mod pty_mux;
pub mod pty_session;

#[cfg(test)]
mod e2e_tests;

// Re-export.
pub use pty_engine::*;
pub use pty_mux::*;
pub use pty_session::*;

// Rustdoc search link fixes.

#[doc(inline)] // Create doc pages at re-export path so rustdoc search links resolve.
pub use pty_session::threads;

// cspell:words terminfo IOCP kqueue
