// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words CLOEXEC errno ptmx isatty TIOCGWINSZ Xenix DUPFD SETFD fcntl

//! See [`PtyPair`] for the main wrapper struct.

use super::{Controlled, ControlledChild, Controller, PtyCommand};
use crate::Size;

/// Owns both halves of a [`PTY`] pair and manages the controlled side's lifecycle.
///
/// # Do not use the [`portable_pty::PtyPair`] directly
///
/// Use this wrapper instead, since it provides a safe API that programmatically prevents
/// deadlocks from leaked parent controlled [`fd`]s and normalizes platform-specific
/// [`PTY`] I/O signaling ([`EOF`] vs [`EIO`]) into a reliable [`PTY`] session lifecycle.
///
/// When a [`PTY`] pair is created, both the parent and child processes initially hold
/// copies of the controlled [`fd`]. This creates a [critical deadlock
/// risk](#resource-leaking-deadlock): the kernel only delivers a termination signal
/// ([`EIO`] on Linux, [`EOF`] on others) to the controller when **all** copies of the
/// controlled [`fd`] are closed.
///
/// If the parent process fails to drop its bootstrapping copy, the kernel's reference
/// count never reaches zero. Consequently, any blocking [`read()`] on the controller will
/// hang indefinitely—even after the child has exited—as it waits for a signal that can
/// never be delivered.
///
/// [`PtyPair`] structurally eliminates this risk by wrapping the controlled side in an
/// [`Option`] and programmatically closing it immediately after spawning. This ensures
/// controlled side's only ones remaining, guaranteeing that the controller's reader
/// receives a termination signal. The library's reader tasks and test fixtures (like
/// [`pty_test_fixtures::drain_and_wait()`]) then programmatically handle both [`EOF`]
/// (`Ok(0)`) and [`EIO`] ([`Err`] with `errno 5` on Linux) to ensure clean,
/// cross-platform termination.
///
/// # Higher-level Sessions
///
/// While [`PtyPair`] is the low-level "engine" that manages kernel resources and safe
/// initialization, your [`TUI`] or [`readline_async`] app will typically interact with
/// this higher-level session type for actual process communication: [`PtySession`].
///
/// For a visual representation of how these layers fit together, see the [3-layer
/// Functional Stack].
///
/// | [`PtyPair`]                                                    | [`PtySession`]                                   |
/// | :------------------------------------------------------------- | :----------------------------------------------- |
/// | Short-lived (initialization phase)                             | Long-lived (duration of the process)             |
/// | Blocking ([`std::read()`], [`std::write()`], [`std::spawn()`]) | Non-blocking ([`tokio`] channels, [`select!`])   |
/// | Prevents [`fd`] leaks and deadlocks                            | Manages task completion and cleanup              |
///
/// # [`PTY`] Primer
///
/// A [`PTY`] (pseudoterminal, introduced in [`4.2BSD`] in 1983, standardized in
/// [`POSIX.1-2001`]) is a kernel-provided virtual terminal. It lets your application
/// spawn a child process that believes it is running in a fully interactive terminal
/// environment. Yet it runs without any terminal device attached. What is a "terminal
/// device"? Any of the following:
///
/// - [physical terminal] (1960s-80s) - hardware teletypewriter, [DEC video terminals]
///   (1978) terminal hardware.
/// - [terminal emulator] window (1984 to now) - GUI apps like [`xterm`] (1984) and
///   [`WezTerm`] (2018) that create their own [`PTY`].
/// - [kernel virtual console] (1984 to now) - first on [`Xenix`] (1984), then on [Linux
///   (1992)]: `Ctrl+Alt+F1-F6` aka `/dev/tty1-/dev/tty6`.
///
/// ## Child process perspective (spawned)
///
/// The **child process** gets a [`/dev/pts/N`] device - the same kind that terminal
/// emulators like [`xterm`], [`WezTerm`] or [`Alacritty`] get - so it can do all of the
/// following via the standard [POSIX terminal API]:
/// - call [`isatty()`] and get `true`,
/// - set [raw mode] with [`tcsetattr()`],
/// - query window size with [`ioctl(TIOCGWINSZ)`],
/// - send [`ANSI`] escape sequences,
/// - read keystrokes.
///
/// When [`portable_pty's spawn_command()`] creates the child process, it redirects each
/// of its streams to the controlled [`fd`]. Here's how the streams are connected:
///
/// | Stream     | [`fd`] | Direction                                                |
/// | :--------- | :----- | :------------------------------------------------------- |
/// | [`stdin`]  | `0`    | Child process reads its input from the controlled [`fd`] |
/// | [`stdout`] | `1`    | Child process writes its output to the controlled [`fd`] |
/// | [`stderr`] | `2`    | Child process writes its errors to the controlled [`fd`] |
///
/// From the child process's perspective, it is talking to a real terminal, and it has no
/// idea it is inside a [`PTY`] 🤯.
///
/// <!-- inception animated gif -->
/// <img
/// src="https://raw.githubusercontent.com/r3bl-org/r3bl-open-core/main/docs/image/inception.gif"
/// width="350" height="250" alt="Inception GIF">
///
/// ## Parent process perspective (your application)
///
/// The **parent process** (your application) is essentially doing what a terminal
/// emulator does, but programmatically instead of with a GUI window (with user
/// interaction). It uses the **controller** side to read from and write to the **child
/// process** - writing to its input and reading from its output.
///
/// In [`PTY`] terminology, the parent process is your Rust application, the one that:
/// 1. First calls [`openpty()`] [`syscall`] to create the [`PTY`].
/// 2. Then calls [`portable_pty's spawn_command()`] to spawn the child process. The child
///    process is the program spawned inside the [`PTY`] (e.g. the [`top`] binary or
///    another Rust application binary).
///
/// Each side of the [`PTY`] (the parent process and child process) gets an [`fd`].
/// However, each side uses it differently:
///
/// 1. The parent process (your Rust application) gets the **controller [`fd`]** - a
///    single bidirectional [`fd`] that it reads from and writes to directly. No standard
///    I/O stream ([`stdin`]/[`stdout`]/[`stderr`]) mapping occurs.
/// 2. The child process (the spawned binary) gets the **controlled [`fd`]** - a single
///    bidirectional [`fd`] that the kernel maps to the child's standard I/O streams
///    ([`stdin`]/[`stdout`]/[`stderr`]).
///    - The child process does not interact with the controller [`fd`] at all.
///    - However, the parent process also gets a copy of the controlled [`fd`] as a
///      bootstrapping artifact when it creates the child process. This artifact becomes
///      the source of the potential deadlock 💀.
///
/// ## How the controlled [`fd`] ends up in the parent
///
/// First the **parent process** calls [`openpty()`], and it gets the **controlled
/// [`fd`]**. This bootstrapping artifact is needed to spawn the child (which does not
/// exist yet).
///
/// Then the parent process calls [`portable_pty's spawn_command()`], which delegates to
/// Rust's [`Command::spawn()`]. Before spawning, [`portable_pty`] wires the controlled
/// [`fd`] to the child's [`stdin`]/[`stdout`]/[`stderr`] via
/// [`Command::stdin()`]/[`Command::stdout()`]/[`Command::stderr()`]. Internally,
/// [`Command::spawn()`] calls [`fork()`] + [`dup2()`] + [`exec()`] (the [fork-exec]
/// pattern) to map the controlled [`fd`] to the child process's [`fd`] `0`, `1`, and `2`.
///
/// After spawning, the parent process has no use for the controlled [`fd`] anymore and it
/// **should** drop the controlled [`fd`] to avoid the deadlock described above. But this
/// does not happen in the [`portable_pty`] code 💣.
///
/// ## What this struct does
///
/// This is precisely what this [`PtyPair`] struct does - it drops the controlled [`fd`]
/// so this deadlock condition can't occur, due to it's type design. By leveraging Rust
/// type system to enforce this, we make this illegal state unrepresentable, and thus
/// prevent the deadlock at compile time. 🙌
///
/// ## Kernel in the middle
///
/// The kernel's [`PTY`] driver - the actual bidirectional channel - sits between the
/// controller [`fd`] (the parent-side endpoint) and the controlled [`fd`] (the child-side
/// endpoint):
///
/// ```text
/// Input (Parent → Child): │  Output (Child → Parent):
/// ┌────────────────┐      │  ┌─────────────────┐
/// │ PARENT PROCESS │      │  │ CHILD PROCESS   │
/// │                │      │  │                 │
/// │ Parent writes  │      │  │ Child writes    │
/// │       │        │      │  │ (stdout+stderr) │
/// │       │        │      │  │        │        │
/// │       ▼        │      │  │        ▼        │
/// │ controller fd  │      │  │ controlled fd   │
/// │       │        │      │  │        │        │
/// └───────│────────┘      │  └────────│────────┘
///         │               │           │
/// ┌───────▼───────────────────────────▼────────┐
/// │                  Kernel PTY                │
/// │                    driver                  │
/// └───────┬───────────────────────────┬────────┘
///         │               │           │
/// ┌───────▼────────┐      │  ┌────────▼────────┐
/// │ CHILD PROCESS  │      │  │ PARENT PROCESS  │
/// │       │        │      │  │        │        │
/// │ controlled fd  │      │  │ controller fd   │
/// │       │        │      │  │        │        │
/// │       ▼        │      │  │        ▼        │
/// │ Child reads    │      │  │ Parent reads    │
/// │ (stdin)        │      │  │                 │
/// └────────────────┘      │  └─────────────────┘
/// ```
///
/// In cooked mode, the driver performs [line discipline] processing (echoing typed
/// characters, translating Ctrl+C into [`SIGINT`], buffering until Enter). In [raw mode],
/// bytes pass straight through unmodified.
///
/// # Controlled side lifecycle
///
/// The **controlled [`fd`]** is the **child process**'s doorway into the kernel's [`PTY`]
/// channel. The kernel tracks how many processes hold copies of this [`fd`], and only
/// returns [`EIO`]/[`EOF`] from blocking [`read()`] [`syscall`] on the controller (in the
/// **parent process**'s thread) when all copies close.
///
/// Specifically, on Unix-like systems, the kernel maintains a reference count on the file
/// description that the controlled [`fd`] points to. In Linux, the internal kernel object
/// that does this is [`struct file`]. Every file descriptor is just an integer index into
/// a process's [`fd`] table, and multiple [`fd`]s can point to the same underlying kernel
/// file description. While terminology differs on Windows, the core principle is the
/// same: the resource remains open as long as any process holds a reference to it.
///
/// ## Resource-leaking deadlock
///
/// Your code (running in the parent process) makes two [`portable_pty`] calls indirectly
/// by using [`PtyPair`]. Both of the following steps are combined in the primary
/// [`PtyPair::open_and_spawn()`] method. Both controller and controlled [`fd`]s get
/// created.
///
/// 1. **[`portable_pty's openpty()`]** - Called by [`PtyPair::open_raw_pair()`], creates
///    the initial [`PTY`] pair.
///    - Under the hood, this calls the POSIX [`openpty(3)`] system call, which opens
///      [`/dev/ptmx`] and creates the initial [`PTY`] pair.
///    - Returns one controller [`fd`] and one controlled [`fd`], both owned by the parent
///      process. We need the controlled [`fd`] in the next step to spawn the child.
///
/// 2. **[`portable_pty's spawn_command()`]** - Called by
///    [`PtyPair::spawn_command_and_close_controlled()`], spawns the child process with
///    the given command.
///    - Builds a [`Command`] from the [`portable_pty's CommandBuilder`] passed to
///      [`PtyPair::spawn_command_and_close_controlled()`].
///    - Wires the controlled [`fd`] to the child's [`stdin`]/[`stdout`]/[`stderr`] via
///      [`Command::stdin()`]/[`Command::stdout()`]/[`Command::stderr()`]. Then calls
///      [`Command::spawn()`], which internally calls [`fork()`] → [`dup2()`] → [`exec()`]
///      (the [fork-exec] pattern) to create **new** controlled [`fd`]s (`0`, `1`, `2`) in
///      the child.
///    - After [`exec()`], the child only has these 3 [`dup2()`]'d copies of the
///      controlled [`fd`].
///      - The original controlled [`fd`] (inherited from [`fork()`]) is marked
///        [`FD_CLOEXEC`] by [`portable_pty`] after [`openpty()`], so [`exec()`]
///        auto-closes it in the child.
///      - The child briefly held 4, but [`exec()`] drops the original controlled [`fd`],
///        leaving 3.
///    - Crucially, the parent's copy of the controlled [`fd`] (created in Step 1) is
///      completely unaffected - this is the bootstrap artifact that causes the deadlock
///      if not explicitly dropped 💣. And that is exactly what
///      [`PtyPair::spawn_command_and_close_controlled()`] does.
///
/// To understand why relying on [`portable_pty's spawn_command()`] alone leads to a
/// deadlock, we must look at the system state immediately after spawning the child (at
/// the very end of Step 2 above). At this point, [two processes] still hold controlled
/// [`fd`]s:
/// 1. The parent process (via [`PtyPair`]) - This is problematic and can cause deadlock
///    💀.
/// 2. The child process (via its [`stdin`]/[`stdout`]/[`stderr`]) - This is ok.
///
/// Here's where the problem arises - the kernel delivers [`EIO`] (or [`EOF`]) to the
/// controller reader only when **all** controlled [`fd`]s are closed. The parent's copy
/// can cause the deadlock, if it is not dropped. Without dropping it the kernel's
/// reference count never hits zero, it never gets the [`EIO`] (or [`EOF`]), and blocking
/// [`read()`] on the controller blocks forever.
///
/// ```text
/// Deadlock: parent still holds controlled fd 💀
///
/// ┌───────────────────────────────────┐
/// │          PARENT PROCESS           │
/// │                                   │
/// │  controller fd     controlled fd  │ ← leftover from openpty()
/// │       │                 │         │
/// └───────│─────────────────│─────────┘
///         │                 │
/// ┌───────▼─────────────────▼─────────┐
/// │          Kernel PTY driver        │
/// │                                   │
/// │    refcount(controlled fd) = 4    │ ← dup2() creates 3 child fds;
/// │    (parent: 1, child: 3)          │   FD_CLOEXEC closes the 4th at exec() ●●
/// └─────────────────────────┬─────────┘
///                           │
/// ┌─────────────────────────▼─────────┐
/// │          CHILD PROCESS            │
/// │                                   │
/// │  controlled fd (via dup2())       │
/// │  → stdin / stdout / stderr        │
/// └───────────────────────────────────┘
///
/// Child exits → refcount = 1 (still > 0)
///             → Kernel does NOT signal EOF
///             → Parent process thread in blocking read() blocks forever 💀
///
/// ●● Why this happens:
/// - portable_pty delegates to Rust's Command::spawn(), which internally calls
///   fork() + dup2() + exec().
/// - After fork(), the child inherits the parent's controlled fd (refcount 2).
/// - Then dup2() x3 creates stdin, stdout, and stderr copies in the child
///   (refcount 5).
/// - portable_pty sets FD_CLOEXEC on the original controlled fd immediately
///   after openpty() via fcntl(F_SETFD).
/// - During exec(), the original controlled fd auto-closes in the child, but
///   the dup2()'d copies (0, 1, 2) survive because dup2() clears the
///   FD_CLOEXEC flag (refcount 4).
/// - Crucially, the parent still holds its original copy - this is the +1 that
///   causes the deadlock!
/// ```
///
/// And this is where the `_and_close_controlled` part of
/// [`PtyPair::spawn_command_and_close_controlled()`] comes into play to prevent this.
/// This method spawns the child and immediately closes the parent's controlled [`fd`] in
/// a single step, so the child process's copies are the only ones left. When the child
/// exits, the kernel delivers [`EIO`]/[`EOF`] to the controller reader as expected 🙌.
///
/// ```text
/// Fixed: PtyPair drops parent's controlled fd 🙌
///
/// ┌───────────────────────────────────┐
/// │          PARENT PROCESS           │
/// │                                   │
/// │  controller fd     controlled fd  │
/// │       │              ╳ dropped    │
/// └───────│───────────────────────────┘
///         │
/// ┌───────▼───────────────────────────┐
/// │          Kernel PTY driver        │
/// │                                   │
/// │    refcount(controlled fd) = 3    │
/// │    (only child holds it)          │
/// └─────────────────────────┬─────────┘
///                           │
/// ┌─────────────────────────▼─────────┐
/// │          CHILD PROCESS            │
/// │                                   │
/// │  controlled fd (via dup2())       │
/// │  → stdin / stdout / stderr        │
/// └───────────────────────────────────┘
///
/// Child exits → refcount = 0
///             → Kernel signals EOF or EIO
///             → Parent process thread in blocking read() returns cleanly 🙌
/// ```
///
/// [`portable_pty::PtyPair`] has no API for closing one side independently (its two `pub`
/// fields drop together). This struct wraps the controlled side in [`Option`] so
/// [`PtyPair::spawn_command_and_close_controlled()`] can drop it while the controller
/// stays alive.
///
/// **POSIX [`EOF`] vs Linux [`EIO`].** When all controlled [`fd`]s close, the parent's
/// blocking [`read()`] call on the controller (in the parent process's thread) returns
/// one of two signals depending on the platform:
///
/// - [`EOF`] (`0` bytes returned) - traditionally means "no more data right now, but the
///   channel is still valid". BSD/macOS uses this for controlled-side closure.
/// - [`EIO`] (`errno` `5`) - means "the I/O channel itself is broken, and the other end
///   doesn't exist anymore". Linux uses this as the more semantically precise signal.
///
/// Any code that reads from the controller must handle **both** to avoid cross-platform
/// polling bugs. For more information take a look at the following:
/// - The [kernel patch that documents this behavior].
/// - The [`pty_test_fixtures::drain_and_wait()`] implementation for an example of correct
///   cross-platform handling.
///
/// # Two Types of Deadlocks
///
/// Understanding these two distinct deadlock scenarios is critical for safe [`PTY`]
/// orchestration.
///
/// ### 1. [Resource-leaking deadlock](#resource-leaking-deadlock) (Production & Test)
/// - **Cause**: The parent process leaks its bootstrapping copy of the controlled [`fd`].
/// - **Effect**: The kernel's reference count for the controlled side stays above zero
///   even after the child process exits.
/// - **Symptom**: Blocking [`read()`] on the controller side hangs forever instead of
///   returning [`EIO`] or [`EOF`].
/// - **Solution**: [`PtyPair::open_and_spawn()`] (or
///   [`PtyPair::spawn_command_and_close_controlled()`]) ensures the parent's copy is
///   closed immediately after spawning.
///
/// ### 2. Buffer-full deadlock (Contrived Single-threaded Tests)
///
/// > <div class="warning">
/// > This is not needed in production code where reading happens in a dedicated
/// > background thread.
/// > </div>
///
/// - **Cause**: The controller thread stops performing [`read()`] operations to call
///   [`ControlledChild::wait()`], but the child process attempts to write more data
///   (e.g., final output to [`stdout`] or [`stderr`]) before exiting.
/// - **Effect**: The kernel's [`PTY`] buffer (typically 1KB on macOS, 4KB on Linux) fills
///   up.
/// - **Symptom**: The child blocks on its final [`write()`] [`syscall`], while the parent
///   blocks on its [`ControlledChild::wait()`] call.
/// - **Solution**: [`pty_test_fixtures::drain_and_wait()`] (used in test fixtures)
///   ensures all remaining bytes are consumed until [`EIO`]/[`EOF`] before calling
///   [`ControlledChild::wait()`].
///
/// # Deadlock-safe API design
///
/// The API is designed so that every codepath either closes the controlled [`fd`]
/// automatically or never exposes it. There is no way to obtain a raw [`Controlled`]
/// value, so the deadlock described in [Controlled side lifecycle] is structurally
/// impossible.
///
/// 1. [`PtyPair::open_and_spawn()`] is the primary way to spawn a child.
///    [`PtyPair::spawn_command_and_close_controlled()`] is another way. Both close the
///    parent's controlled [`fd`] immediately after spawning, so the child's copies are
///    the only ones left, eliminating any chance of [resource-leaking
///    deadlock](#resource-leaking-deadlock).
/// 2. [`PtyPair::into_controller()`] is the only way to extract an owned [`Controller`].
///    It drops the controlled side automatically if it is still open, as a safety net,
///    eliminating any change of [resource-leaking deadlock](#resource-leaking-deadlock).
///
/// The example below shows [`PtyPair::open_and_spawn()`] handling everything in a single
/// call:
///
/// ```no_run
/// use r3bl_tui::{PtyCommand, PtyPair, size, width, height};
///
/// let (pty_pair, _child) = PtyPair::open_and_spawn(
///     size(width(80) + height(24)),
///     PtyCommand::new("top")
/// ).unwrap();
///
/// // Reading from the reader will return `EIO` or `EOF` once the child process exits.
/// let reader = pty_pair.controller().try_clone_reader().unwrap();
/// ```
///
/// # File Descriptor Ownership
///
/// Understanding which process holds which [`fd`] is essential for reasoning about
/// [`EOF`] delivery and deadlocks. The actual kernel file descriptors live deep inside
/// [`portable_pty`]'s trait objects:
///
/// ```text
/// PtyPair
///   ├── controller: Controller                (Box<dyn MasterPty + Send>)
///   │     └── UnixMasterPty                   (portable_pty internal)
///   │           └── fd: PtyFd → RawFd         ← kernel controller fd
///   │
///   └── maybe_controlled: Option<Controlled>  (Box<dyn SlavePty + Send>)
///         └── UnixSlavePty                    (portable_pty internal)
///               └── fd: PtyFd → RawFd         ← kernel controlled fd
/// ```
///
/// When a [`Controlled`] value is dropped, the [`Drop`] chain closes the kernel [`fd`],
/// as shown here:
///
/// ```text
/// Controlled (Box<dyn SlavePty>) → UnixSlavePty → PtyFd → FileDescriptor → close(fd)
/// ```
///
/// This is why [`PtyPair::spawn_command_and_close_controlled()`] works: it calls
/// [`Option::take()`] to move the [`Controlled`] out, then drops it.
///
/// # Inclusive terminology
///
/// [`PtyPair`] replaces [`portable_pty::PtyPair`]'s master/slave naming with
/// controller/controlled, per [Inclusive Naming Initiative - Tier 1 Terms].
///
/// [3-layer Functional Stack]: crate::core::pty#the-functional-stack
/// [`/dev/ptmx`]: https://man7.org/linux/man-pages/man4/ptmx.4.html
/// [`/dev/pts/N`]: https://man7.org/linux/man-pages/man4/pts.4.html
/// [`4.2BSD`]: https://en.wikipedia.org/wiki/Berkeley_Software_Distribution#4.2BSD
/// [`Alacritty`]: https://alacritty.org/
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
/// [`Command::spawn()`]: std::process::Command::spawn
/// [`Command::stderr()`]: std::process::Command::stderr
/// [`Command::stdin()`]: std::process::Command::stdin
/// [`Command::stdout()`]: std::process::Command::stdout
/// [`Command`]: std::process::Command
/// [`ControlledChild::wait()`]: portable_pty::Child::wait
/// [`controller()`]: PtyPair::controller
/// [`controller_mut()`]: PtyPair::controller_mut
/// [`DefaultPtySize`]: crate::DefaultPtySize
/// [`dup2()`]: https://man7.org/linux/man-pages/man2/dup2.2.html
/// [`EIO`]: https://man7.org/linux/man-pages/man3/errno.3.html
/// [`EOF`]: https://en.wikipedia.org/wiki/End-of-file
/// [`exec()`]: https://man7.org/linux/man-pages/man3/exec.3.html
/// [`FD_CLOEXEC`]: https://man7.org/linux/man-pages/man2/fcntl.2.html
/// [`fd`]: https://man7.org/linux/man-pages/man2/open.2.html
/// [`fork()`]: https://man7.org/linux/man-pages/man2/fork.2.html
/// [`ioctl(TIOCGWINSZ)`]: https://man7.org/linux/man-pages/man2/ioctl_tty.2.html
/// [`isatty()`]: https://man7.org/linux/man-pages/man3/isatty.3.html
/// [`openpty()`]: https://man7.org/linux/man-pages/man3/openpty.3.html
/// [`openpty(3)`]: https://man7.org/linux/man-pages/man3/openpty.3.html
/// [`portable_pty's CommandBuilder`]: portable_pty::CommandBuilder
/// [`portable_pty's openpty()`]: portable_pty::PtySystem::openpty
/// [`portable_pty's spawn_command()`]: portable_pty::SlavePty::spawn_command
/// [`POSIX.1-2001`]: https://pubs.opengroup.org/onlinepubs/009695399/
/// [`pty_test_fixtures::drain_and_wait()`]: crate::pty_test_fixtures::drain_and_wait
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
/// [`PtyPair::into_controller()`]: PtyPair::into_controller
/// [`PtyPair::open_raw_pair()`]: Self::open_raw_pair
/// [`PtyPair::spawn_command_and_close_controlled()`]:
///     Self::spawn_command_and_close_controlled
/// [`PtySession`]: crate::PtySession
/// [`read()`]: https://man7.org/linux/man-pages/man2/read.2.html
/// [`readline_async`]: crate::readline_async::ReadlineAsyncContext::try_new
/// [`select!`]: tokio::select
/// [`SIGINT`]: https://man7.org/linux/man-pages/7/signal.7.html
/// [`std::read()`]: std::io::Read::read
/// [`std::spawn()`]: Self::open_and_spawn
/// [`std::write()`]: std::io::Write::write
/// [`stderr`]: std::io::stderr
/// [`stdin`]: std::io::stdin
/// [`stdout`]: std::io::stdout
/// [`struct file`]:
///     https://elixir.bootlin.com/linux/v6.19.3/source/include/linux/fs.h#L1256
/// [`syscall`]: https://man7.org/linux/man-pages/man2/syscalls.2.html
/// [`tcsetattr()`]: https://man7.org/linux/man-pages/man3/tcsetattr.3.html
/// [`tokio`]: tokio
/// [`top`]: https://man7.org/linux/man-pages/1/top.1.html
/// [`TUI`]: crate::tui::TerminalWindow::main_event_loop
/// [`WezTerm`]: https://wezfurlong.org/wezterm/
/// [`write()`]: https://man7.org/linux/man-pages/man2/write.2.html
/// [`Xenix`]: https://en.wikipedia.org/wiki/Xenix
/// [`xterm`]: https://en.wikipedia.org/wiki/Xterm
/// [Controlled side lifecycle]: #resource-leaking-deadlock
/// [DEC video terminals]: https://vt100.net/shuford/terminal/dec.html
/// [fork-exec]: https://en.wikipedia.org/wiki/Fork-exec
/// [Inclusive Naming Initiative - Tier 1 Terms]:
///     https://inclusivenaming.org/word-lists/tier-1/
/// [kernel patch that documents this behavior]:
///     https://lists.archive.carbon60.com/linux/kernel/1790583
/// [kernel virtual console]: https://en.wikipedia.org/wiki/Virtual_console
/// [line discipline]: https://en.wikipedia.org/wiki/Line_discipline
/// [Linux (1992)]: https://en.wikipedia.org/wiki/Linux_console
/// [physical terminal]: https://en.wikipedia.org/wiki/Computer_terminal
/// [portable_pty's `CommandBuilder`]: portable_pty::CommandBuilder
/// [POSIX terminal API]: https://man7.org/linux/man-pages/man3/termios.3.html
/// [raw mode]: mod@crate::terminal_raw_mode#raw-mode-vs-cooked-mode
/// [terminal emulator]: https://en.wikipedia.org/wiki/Terminal_emulator
/// [two processes]: #file-descriptor-ownership
#[allow(missing_debug_implementations)]
pub struct PtyPair {
    /// Controller side of the [`PTY`].
    ///
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    pub controller: Controller,

    /// Controlled side of the [`PTY`], held as [`Option`] so it can be closed early. For
    /// more information, see:
    /// - [File Descriptor ownership].
    /// - [Resource-leaking deadlock].
    ///
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    /// [File Descriptor ownership]: #file-descriptor-ownership
    /// [Resource-leaking deadlock]: #resource-leaking-deadlock
    pub maybe_controlled: Option<Controlled>,
}

impl PtyPair {
    /// Opens a [`PTY`] pair with the given size and immediately spawns the command.
    ///
    /// This is the safest way to initialize a [`PTY`] as it automatically drops the
    /// parent's copy of the controlled [`fd`] after spawning.
    ///
    /// # Errors
    ///
    /// Returns an error if the [`PTY`] system fails to open a pair, or if the command
    /// fails to spawn.
    ///
    /// [`fd`]: https://man7.org/linux/man-pages/man2/open.2.html
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    pub fn open_and_spawn(
        arg_size: impl Into<Size>,
        command: PtyCommand,
    ) -> miette::Result<(PtyPair, ControlledChild)> {
        let mut pair = PtyPair::open_raw_pair(arg_size.into())?;
        let child = PtyPair::spawn_command_and_close_controlled(&mut pair, command)?;
        Ok((pair, child))
    }

    /// Access the controller side of the [`PTY`].
    ///
    /// The controller side is used by the parent process to read output from and write
    /// input to the controlled child process.
    ///
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    #[must_use]
    pub fn controller(&self) -> &Controller { &self.controller }

    /// Access the controller side mutably.
    pub fn controller_mut(&mut self) -> &mut Controller { &mut self.controller }

    /// Consumes the pair and returns the owned controller side.
    ///
    /// If the controlled side is still open, it is dropped automatically to prevent
    /// deadlocks from a leaked parent [`fd`]. For more information, see:
    /// - [File Descriptor ownership].
    /// - [Resource-leaking deadlock].
    ///
    /// [`fd`]: https://man7.org/linux/man-pages/man2/open.2.html
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    /// [File Descriptor ownership]: #file-descriptor-ownership
    /// [Resource-leaking deadlock]: #resource-leaking-deadlock
    #[must_use]
    pub fn into_controller(self) -> Controller {
        drop(self.maybe_controlled);
        self.controller
    }
}

/// Associated functions for the [`PtyPair`] struct that are not methods.
impl PtyPair {
    /// Opens a raw [`PTY`] pair without spawning a child process.
    ///
    /// This is an internal implementation step used by [`PtyPair::open_and_spawn()`].
    ///
    /// - It is exposed as an associated function for use-cases that need to separate
    ///   [`PTY`] creation from spawning (e.g., tests that only need a raw [`Controller`]
    ///   for testing).
    /// - You can pass [`crate::DefaultPtySize`] for standard dimensions for a [`PTY`]
    ///   used in tests.
    ///
    /// # Returns
    ///
    /// A [`PtyPair`] with both controller and controlled sides open. The returned
    /// [`PtyPair`] still holds the controlled [`fd`].
    ///
    /// You must either:
    /// - Call [`PtyPair::spawn_command_and_close_controlled()`] to spawn and close the
    ///   controlled [`fd`].
    /// - Call [`PtyPair::into_controller()`] which drops the controlled side as a safety
    ///   net.
    ///
    /// See [`PtyPair`]'s [Controlled side lifecycle] section for why the controlled
    /// [`fd`] must be closed before reading from the controller.
    ///
    /// # Errors
    ///
    /// Returns an error if the [`PTY`] system fails to open a pair.
    ///
    /// [`fd`]: https://man7.org/linux/man-pages/man2/open.2.html
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    /// [Controlled side lifecycle]: #resource-leaking-deadlock
    pub fn open_raw_pair(
        arg_pty_size: impl Into<portable_pty::PtySize>,
    ) -> miette::Result<PtyPair> {
        let pty_size = arg_pty_size.into();
        let raw_pair = portable_pty::native_pty_system()
            .openpty(pty_size)
            .map_err(|e| miette::miette!("Failed to open PTY: {e}"))?;
        Ok(PtyPair::from(raw_pair))
    }

    /// Spawns a command on the controlled side and immediately closes the controlled
    /// [`fd`].
    ///
    /// This is an internal implementation step used by [`PtyPair::open_and_spawn()`]. It
    /// is exposed as an associated function for use-cases that need to separate [`PTY`]
    /// creation from spawning (e.g., tests that configure the pair before spawning).
    ///
    /// See [`PtyPair`]'s [Controlled side lifecycle] section for why the controlled
    /// [`fd`] must be closed immediately after spawning.
    ///
    /// # Returns
    ///
    /// The spawned [`ControlledChild`] process handle. The controlled [`fd`] is closed
    /// before returning.
    ///
    /// See the [Two Types of Deadlocks] section for how this prevents the primary
    /// **resource-leaking deadlock**.
    ///
    /// # Errors
    ///
    /// Returns an error if the controlled side has already been consumed (e.g., by a
    /// previous call to this function or by [`PtyPair::into_controller()`]), or if the
    /// command fails to spawn.
    ///
    /// [`fd`]: https://man7.org/linux/man-pages/man2/open.2.html
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    /// [Controlled side lifecycle]: #resource-leaking-deadlock
    /// [Two Types of Deadlocks]: #two-types-of-pty-deadlocks
    pub fn spawn_command_and_close_controlled(
        pair: &mut PtyPair,
        command: PtyCommand,
    ) -> miette::Result<ControlledChild> {
        let controlled = pair.maybe_controlled.as_ref().ok_or_else(|| {
            miette::miette!(
                "Controlled side already consumed - was this pair already spawned \
                or created via open_and_spawn()?"
            )
        })?;
        let child = controlled
            .spawn_command(command)
            .map_err(|e| miette::miette!("{e:#}"))?;
        drop(pair.maybe_controlled.take());
        Ok(child)
    }
}

/// Converts a [`portable_pty::PtyPair`] to a [`PtyPair`] wrapper.
impl From<portable_pty::PtyPair> for PtyPair {
    fn from(it: portable_pty::PtyPair) -> Self {
        Self {
            controller: it.master,
            maybe_controlled: Some(it.slave),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PtyCommand, core::pty::pty_engine::pty_size::DefaultPtySize, height,
                size, width};

    #[test]
    fn test_open_raw_pair_default_size() {
        let result = PtyPair::open_raw_pair(DefaultPtySize);
        assert!(result.is_ok());
    }

    #[test]
    fn test_open_raw_pair_custom_size() {
        let result = PtyPair::open_raw_pair(size(width(100) + height(30)));
        assert!(result.is_ok());
    }

    #[test]
    fn test_open_and_spawn_success() {
        #[cfg(unix)]
        let command = {
            let mut cmd = PtyCommand::new("echo");
            cmd.arg("test");
            cmd
        };
        #[cfg(windows)]
        let command = {
            let mut cmd = PtyCommand::new("cmd.exe");
            cmd.args(["/c", "echo", "test"]);
            cmd
        };

        let result = PtyPair::open_and_spawn(DefaultPtySize, command);
        assert!(result.is_ok());
        let (pty_pair, _child) = result.unwrap();
        assert!(pty_pair.maybe_controlled.is_none());
    }
}
