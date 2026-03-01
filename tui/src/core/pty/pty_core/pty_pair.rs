// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words CLOEXEC errno ptmx isatty TIOCGWINSZ Xenix DUPFD SETFD fcntl

//! See [`PtyPair`] for the main wrapper struct.

use super::{Controlled, ControlledChild, Controller, PtyCommand};
use crate::{Size, height, size, width};

/// Owns both halves of a [`PTY`] pair and manages the controlled side's lifecycle.
///
/// <div class="warning">
///
/// Do not use the [`portable_pty::PtyPair`] directly - this wrapper provides a safe API
/// that prevents deadlocks from leaked parent controlled [`fd`]s and abstracts away
/// platform differences in [`PTY`] I/O signaling ([`EOF`] vs [`EIO`]) by documenting the
/// correct cross-platform handling pattern.
///
/// When the parent process creates a [`PTY`] pair and spawns a child, both parent and
/// child processes hold copies of the controlled [`fd`]. When the controller (in the
/// parent process) calls the blocking [`read()`] [`syscall`] to get data from the child
/// process's output, then a potential for deadlock exists.
///
/// The kernel only returns [`EIO`] (input/output error) or [`EOF`] (end of file) from
/// blocking [`read()`] when **all** controlled [`fd`]s are closed. So if the parent
/// process does not drop its copy (_which it does not really need, and has due to some
/// POSIX function orchestration reasons_), the kernel's reference count never hits zero
/// and the blocking [`read()`] [`syscall`] blocks forever.
///
/// This hangs up the thread that called the blocking [`read()`] in the parent process;
/// the thread waits indefinitely for an [`EIO`] or [`EOF`] that will never arrive. This
/// typically happens after the child process has already exited (having closed its 3
/// copies of the [`fd`], while the parent holds on to its one).
///
/// [`PtyPair`] prevents this by closing the parent's controlled [`fd`] immediately after
/// spawning.
///
/// </div>
///
/// # [`PTY`] Primer
///
/// A [`PTY`] (pseudoterminal, introduced in [`4.2BSD`] in 1983, standardized in
/// `POSIX.1-2001`) is a kernel-provided virtual terminal. It lets your application spawn
/// a child process that believes it is running in a fully interactive terminal
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
/// src="https://media4.giphy.com/media/v1.Y2lkPTc5MGI3NjExeDB3cGV2M2s2bW42b3Vhc3I5eHA3Y2pob2MwOWdibGV0NHRvaG14YSZlcD12MV9pbnRlcm5hbF9naWZfYnlfaWQmY3Q9Zw/kOIbusN7fPnkk/giphy.gif"
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
/// does not happen in the [`portable_pty`] code 💣💀.
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
/// Specifically, the kernel maintains a reference count on the file description (the
/// internal kernel object, `struct file` in Linux) that the controlled [`fd`] points to.
/// Every file descriptor is just an integer index into a process's [`fd`] table, and
/// multiple [`fd`]s can point to the same underlying kernel file description.
///
/// Your code (the parent process) makes two [`portable_pty`] calls that create controlled
/// [`fd`]s:
/// 1. First via [`PtyPair::new_with_size()`] or [`PtyPair::new_with_default_size()`] to
///    create the initial [`PTY`] pair.
/// 2. Then via [`PtyPair::spawn_command_and_close_controlled()`] to spawn the child
///    process with the given command.
///
/// Here are the details around the underlying [`portable_pty`] calls that are made.
///
/// 1. **[`portable_pty's openpty()`]** - Called by [`PtyPair::new_with_size()`] or
///    [`PtyPair::new_with_default_size()`].
///    - Under the hood, this calls the POSIX [`openpty(3)`] system call, which opens
///      [`/dev/ptmx`] and creates the initial [`PTY`] pair.
///    - Returns one controller [`fd`] and one controlled [`fd`], both owned by the parent
///      process.
///
/// 2. **[`portable_pty's spawn_command()`]** - Called by
///    [`PtyPair::spawn_command_and_close_controlled()`].
///    - Builds a [`Command`] from the [portable_pty's `CommandBuilder`] passed to
///      [`PtyPair::spawn_command_and_close_controlled()`].
///    - Wires the controlled [`fd`] to the child's [`stdin`]/[`stdout`]/[`stderr`] via
///      [`Command::stdin()`]/[`Command::stdout()`]/[`Command::stderr()`]. Then calls
///      [`Command::spawn()`], which internally calls [`fork()`] → [`dup2()`] → [`exec()`]
///      (the [fork-exec] pattern) to create **new** controlled [`fd`]s (`0`, `1`, `2`) in
///      the child.
///    - After [`exec()`], the child only has these 3 `dup2()`'d copies. The original
///      controlled [`fd`] (inherited from [`fork()`]) is marked [`FD_CLOEXEC`] by
///      [`portable_pty`] after [`openpty()`], so [`exec()`] auto-closes it in the child
///      (the child briefly held 4, but `exec()` drops the original, leaving 3).
///    - Crucially, [`FD_CLOEXEC`] only fires when [`exec()`] is called, and only the
///      child process calls [`exec()`]. The parent's copy of the controlled [`fd`] is
///      completely unaffected - this is the bootstrap artifact that causes the deadlock
///      if not explicitly dropped.
///
/// After [`portable_pty's spawn_command()`], [two processes] hold controlled [`fd`]s:
/// 1. the parent process (via [`PtyPair`]) and
/// 2. the child process (via its [`stdin`]/[`stdout`]/[`stderr`]).
///
/// Here's where the problem arises - the kernel delivers [`EIO`] (or [`EOF`]) to the
/// controller reader only when **all** controlled [`fd`]s are closed. The parent's copy
/// can cause the deadlock - if it is not dropped, the kernel's reference count never hits
/// zero and blocking [`read()`] on the controller blocks forever.
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
/// │    refcount(controlled fd) = 4    │ ← dup2 creates 3 child fds;
/// │    (parent: 1, child: 3)          │   FD_CLOEXEC closes the 4th at exec **
/// └─────────────────────────┬─────────┘
///                           │
/// ┌─────────────────────────▼─────────┐
/// │          CHILD PROCESS            │
/// │                                   │
/// │  controlled fd (via dup2)         │
/// │  → stdin / stdout / stderr        │
/// └───────────────────────────────────┘
///
/// Child exits → refcount = 1 (still > 0)
///             → Kernel does NOT signal EOF
///             → Parent process thread in blocking read() blocks forever 💀
///
/// ** portable_pty delegates to Rust's Command::spawn(), which internally
///    calls fork() + dup2() + exec(). After fork(), the child inherits the
///    parent's controlled fd (refcount 2). Then dup2() x3 creates stdin,
///    stdout, stderr copies (refcount 5). FD_CLOEXEC on the original
///    controlled fd auto-closes it at exec(), bringing refcount to 4.
///    portable_pty sets FD_CLOEXEC on the original controlled fd immediately
///    after openpty() via fcntl(F_SETFD). The dup2()'d copies (fd 0, 1, 2)
///    do NOT have FD_CLOEXEC - dup2() clears it - so they survive exec().
/// ```
///
/// And this is where the `_and_close_controlled` part of
/// [`PtyPair::spawn_command_and_close_controlled()`] comes into play to prevent this.
/// This method spawns the child and immediately closes the parent's controlled [`fd`] in
/// one atomic step, so the child process's copies are the only ones left. When the child
/// exits, the kernel delivers [`EIO`]/[`EOF`] to the controller reader as expected 🙌.
///
/// ```text
/// Fixed: PtyPair drops parent's controlled fd 🙌
///
/// ┌───────────────────────────────────┐
/// │          PARENT PROCESS           │
/// │                                   │
/// │  controller fd     controlled fd  │
/// │       │               ╳ dropped   │
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
/// │  controlled fd (via dup2)         │
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
///   channel is still valid." BSD/macOS uses this for controlled-side closure.
/// - [`EIO`] (`errno` `5`) - means "the I/O channel itself is broken - the other end
///   doesn't exist anymore." Linux uses this as the more semantically precise signal.
///
/// Any code that reads from the controller must handle **both** to avoid cross-platform
/// polling bugs. See the [kernel patch that documents this behavior] and the
/// [`drain_pty_and_wait`] implementation for an example of correct cross-platform
/// handling.
///
/// # Deadlock-safe API design
///
/// The API is designed so that every codepath either closes the controlled [`fd`]
/// automatically or never exposes it. There is no way to obtain a raw [`Controlled`]
/// value, so the deadlock described in [Controlled side lifecycle] is structurally
/// impossible.
///
/// 1. [`spawn_command_and_close_controlled()`] is the only way to spawn a child. It
///    closes the parent's controlled [`fd`] immediately after spawning, so the child's
///    copies are the only ones left.
/// 2. [`into_controller()`] is the only way to extract an owned [`Controller`]. It drops
///    the controlled side automatically if it is still open, as a safety net.
///
/// The example below shows [`PtyPair::spawn_command_and_close_controlled()`] handling
/// both spawn and controlled-side closure in a single call:
///
/// ```no_run
/// use r3bl_tui::{PtyCommand, PtyPair, size, width, height};
///
/// let mut pty_pair = PtyPair::new_with_size(size(width(80) + height(24))).unwrap();
///
/// // Spawn child and close the controlled side in one step.
/// let _child = pty_pair.spawn_command_and_close_controlled(PtyCommand::new("cat")).unwrap();
///
/// // Reads from the controller will return EOF once the child exits.
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
/// This is why [`spawn_command_and_close_controlled()`] works: it calls
/// [`Option::take()`] to move the [`Controlled`] out, then drops it.
///
/// # Inclusive terminology
///
/// [`PtyPair`] replaces [`portable_pty::PtyPair`]'s master/slave naming with
/// controller/controlled, per [Inclusive Naming Initiative - Tier 1 Terms].
///
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
/// [`controller()`]: PtyPair::controller
/// [`controller_mut()`]: PtyPair::controller_mut
/// [`drain_pty_and_wait`]: crate::drain_pty_and_wait
/// [`dup2()`]: https://man7.org/linux/man-pages/man2/dup2.2.html
/// [`EIO`]: https://man7.org/linux/man-pages/man3/errno.3.html
/// [`EOF`]: https://en.wikipedia.org/wiki/End-of-file
/// [`exec()`]: https://man7.org/linux/man-pages/man3/exec.3.html
/// [`FD_CLOEXEC`]: https://man7.org/linux/man-pages/man2/fcntl.2.html
/// [`fd`]: https://man7.org/linux/man-pages/man2/open.2.html
/// [`fork()`]: https://man7.org/linux/man-pages/man2/fork.2.html
/// [`into_controller()`]: PtyPair::into_controller
/// [`ioctl(TIOCGWINSZ)`]: https://man7.org/linux/man-pages/man2/ioctl_tty.2.html
/// [`isatty()`]: https://man7.org/linux/man-pages/man3/isatty.3.html
/// [`new_with_default_size()`]: PtyPair::new_with_default_size
/// [`new_with_size()`]: PtyPair::new_with_size
/// [`openpty()`]: https://man7.org/linux/man-pages/man3/openpty.3.html
/// [`openpty(3)`]: https://man7.org/linux/man-pages/man3/openpty.3.html
/// [`portable_pty's openpty()`]: portable_pty::PtySystem::openpty
/// [`portable_pty's spawn_command()`]: portable_pty::SlavePty::spawn_command
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
/// [`PtyPair::spawn_command_and_close_controlled()`]:
///     PtyPair::spawn_command_and_close_controlled
/// [`read()`]: https://man7.org/linux/man-pages/man2/read.2.html
/// [`SIGINT`]: https://man7.org/linux/man-pages/man7/signal.7.html
/// [`spawn_command_and_close_controlled()`]: PtyPair::spawn_command_and_close_controlled
/// [`stderr`]: std::io::stderr
/// [`stdin`]: std::io::stdin
/// [`stdout`]: std::io::stdout
/// [`syscall`]: https://man7.org/linux/man-pages/man2/syscalls.2.html
/// [`tcsetattr()`]: https://man7.org/linux/man-pages/man3/tcsetattr.3.html
/// [`top`]: https://man7.org/linux/man-pages/man1/top.1.html
/// [`WezTerm`]: https://wezfurlong.org/wezterm/
/// [`Xenix`]: https://en.wikipedia.org/wiki/Xenix
/// [`xterm`]: https://en.wikipedia.org/wiki/Xterm
/// [Controlled side lifecycle]: #controlled-side-lifecycle
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
/// [raw mode]: mod@crate::core::ansi::terminal_raw_mode#raw-mode-vs-cooked-mode
/// [terminal emulator]: https://en.wikipedia.org/wiki/Terminal_emulator
/// [two processes]: #file-descriptor-ownership
#[allow(missing_debug_implementations)]
pub struct PtyPair {
    /// Controller side of the [`PTY`].
    ///
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    pub controller: Controller,

    /// Controlled side of the [`PTY`], held as `Option` so it can be closed early.
    ///
    /// After the child process is spawned, the parent process no longer needs the
    /// controlled [`fd`]. Keeping it open prevents [`EIO`] (or [`EOF`]) from being
    /// delivered to controller readers after the child process exits.
    /// [`PtyPair::spawn_command_and_close_controlled()`] handles this automatically.
    ///
    /// [`EIO`]: https://man7.org/linux/man-pages/man3/errno.3.html
    /// [`EOF`]: https://en.wikipedia.org/wiki/End-of-file
    /// [`fd`]: https://man7.org/linux/man-pages/man2/open.2.html
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    pub maybe_controlled: Option<Controlled>,
}

impl PtyPair {
    /// Creates a new [`PTY`] pair with the specified terminal size.
    ///
    /// # Errors
    ///
    /// Returns an error if the [`PTY`] system fails to open a pair.
    ///
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    pub fn new_with_size(pty_size: Size) -> miette::Result<Self> {
        let pty_system = portable_pty::native_pty_system();
        let raw_pair = pty_system
            .openpty(pty_size.into())
            .map_err(|e| miette::miette!("Failed to open PTY: {e}"))?;
        Ok(Self::from(raw_pair))
    }

    /// Creates a new [`PTY`] pair with the standard 80x24 terminal size.
    ///
    /// # Errors
    ///
    /// Returns an error if the [`PTY`] system fails to open a pair.
    ///
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    pub fn new_with_default_size() -> miette::Result<Self> {
        Self::new_with_size(size(width(80) + height(24)))
    }

    /// Creates a new wrapper from a raw [`portable_pty::PtyPair`].
    #[must_use]
    pub fn new(inner: portable_pty::PtyPair) -> Self {
        Self {
            controller: inner.master,
            maybe_controlled: Some(inner.slave),
        }
    }

    /// Access the controller side of the [`PTY`].
    ///
    /// The controller side is used by the parent process to read output from
    /// and write input to the controlled child process.
    ///
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    #[must_use]
    pub fn controller(&self) -> &Controller { &self.controller }

    /// Access the controller side mutably.
    pub fn controller_mut(&mut self) -> &mut Controller { &mut self.controller }

    /// Consumes the pair and returns the owned controller side.
    ///
    /// If the controlled side is still open, it is dropped automatically to prevent
    /// deadlocks from a leaked parent [`fd`].
    ///
    /// [`fd`]: https://man7.org/linux/man-pages/man2/open.2.html
    #[must_use]
    pub fn into_controller(self) -> Controller {
        drop(self.maybe_controlled);
        self.controller
    }

    /// Spawns a command on the controlled side and immediately closes it.
    ///
    /// This prevents deadlocks by ensuring the parent process never holds the controlled
    /// [`fd`] after spawning. The child process retains its own copies of the controlled
    /// [`fd`]s ([`stdin`]/[`stdout`]/[`stderr`]), which close when the child exits -
    /// delivering [`EIO`]/[`EOF`] to the controller reader.
    ///
    /// # Errors
    ///
    /// Returns an error if the command fails to spawn in the [`PTY`].
    ///
    /// # Panics
    ///
    /// Panics if the controlled side has already been consumed by a previous call to
    /// this method or [`into_controller()`].
    ///
    /// [`EIO`]: https://man7.org/linux/man-pages/man3/errno.3.html
    /// [`EOF`]: https://en.wikipedia.org/wiki/End-of-file
    /// [`fd`]: https://man7.org/linux/man-pages/man2/open.2.html
    /// [`into_controller()`]: PtyPair::into_controller
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    /// [`stderr`]: std::io::stderr
    /// [`stdin`]: std::io::stdin
    /// [`stdout`]: std::io::stdout
    pub fn spawn_command_and_close_controlled(
        &mut self,
        command: PtyCommand,
    ) -> miette::Result<ControlledChild> {
        let controlled = self
            .maybe_controlled
            .as_ref()
            .expect("controlled side already consumed");
        let child = controlled
            .spawn_command(command)
            .map_err(|e| miette::miette!("{e:#}"))?;
        drop(self.maybe_controlled.take());
        Ok(child)
    }
}

/// Converts a [`Size`] to a [`portable_pty::PtySize`].
impl From<Size> for portable_pty::PtySize {
    fn from(size: Size) -> Self {
        Self {
            rows: size.row_height.0.as_u16(),
            cols: size.col_width.0.as_u16(),
            pixel_width: 0,
            pixel_height: 0,
        }
    }
}

/// Converts a [`portable_pty::PtyPair`] to a [`PtyPair`] wrapper.
impl From<portable_pty::PtyPair> for PtyPair {
    fn from(it: portable_pty::PtyPair) -> Self { Self::new(it) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PtyCommand, height, size, width};

    #[test]
    fn test_new_with_default_size_creates_pty_pair() {
        let result = PtyPair::new_with_default_size();
        assert!(result.is_ok());
    }

    #[test]
    fn test_new_with_custom_size() {
        let result = PtyPair::new_with_size(size(width(100) + height(30)));
        assert!(result.is_ok());
    }

    #[test]
    fn test_spawn_command_and_close_controlled() {
        let mut pty_pair = PtyPair::new_with_default_size().unwrap();

        #[cfg(unix)]
        let command = {
            let mut cmd = PtyCommand::new("echo");
            cmd.arg("test");
            cmd
        };
        #[cfg(windows)]
        let command = {
            let mut cmd = PtyCommand::new("cmd");
            cmd.args(["/c", "echo", "test"]);
            cmd
        };

        let result = pty_pair.spawn_command_and_close_controlled(command);
        assert!(result.is_ok());
    }
}
