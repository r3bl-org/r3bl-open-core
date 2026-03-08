// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

/// Macro that generates [`pty`]-based integration tests with automatic test name
/// injection.
///
/// Use this macro for **single-feature [`pty`] tests** that test one specific behavior
/// (e.g., [raw mode], Ctrl+W deletion, terminal event parsing).
///
/// For **multi-backend comparison tests** that need to run the same controlled code with
/// different backends and compare results, use [`spawn_controlled_in_pty`] instead.
///
/// # When to Use This Macro vs [`spawn_controlled_in_pty`]
///
/// | Scenario                                          | Use                         |
/// | ------------------------------------------------- | --------------------------- |
/// | Testing a single feature in a [`pty`] environment | [`generate_pty_test!`]      |
/// | Comparing two backends produce identical results  | [`spawn_controlled_in_pty`] |
/// | One test function, one controlled process         | [`generate_pty_test!`]      |
/// | One test function, multiple controlled processes  | [`spawn_controlled_in_pty`] |
///
/// This macro handles generating the boilerplate for complex process creation, routing,
/// and orchestration, for [`pty`]-based end to end and integration tests:
///
/// 1. **Process routing**: Routes to controller or controlled code based on environment
///    variable. The same test binary is spawned two times, and depending on the
///    environment variable, one process assumes the role of the controlled and the other
///    one, the controller.
/// 2. **[`pty`] setup**: Creates [`pty`] pair so you don't have to do this manually.
/// 3. **Debug output**: Prints diagnostic messages for troubleshooting.
///
/// The macro creates the [`pty`] and spawns the process, then passes these resources to
/// your controller function so you can focus on verification logic.
///
/// # Architecture
///
/// ```text
/// ┌──────────────────────────────────────────────────────────────┐
/// │ Test Function (entry point)                                  │
/// │  - Macro detects role via environment variable               │
/// │  - Routes to controller or controlled function               │
/// └────────────┬────────────────────────────────┬────────────────┘
///              │                                │
///     Controller Path                  Controlled Path
///              │                                │
/// ┌────────────▼───────────┐    ┌───────────────▼────────────────┐
/// │ Macro: PTY Setup       │    │ Controlled Function            │
/// │ - Creates PTY pair     │    │ - Enables raw mode (if needed) │
/// │ - Spawns controlled    ├────► - Execute test logic           │
/// │ - Passes to controller │    │ - Output via stdout/stderr     │
/// └────────────┬───────────┘    └────────────▲─┬─────────────────┘
///              │                             │ │
/// ┌────────────▼──────────────────┐          │ │
/// │ Controller Function           │          │ │ PTY I/O
/// │ - Receives pty_pair           │          │ │ stdin, stdout/stderr
/// │ - Receives child              │          │ │
/// │ - Writes input to child (opt) ├──────────┘ │
/// │ - Reads results from child    ◄────────────┘
/// │ - Verifies assertions         │
/// │ - Waits for child exit        │
/// └───────────────────────────────┘
/// ```
///
/// # Controller-Controlled Orchestration
///
/// The controller and controlled communicate through the kernel [`pty`] buffer using real
/// I/O without mocks. This is the entire purpose of the test fixture: exercise real
/// terminal code paths. A typical exchange looks like:
///
/// 1. Controlled prints a ready marker (e.g. [`CONTROLLED_READY`]) to [`stdout`].
/// 2. Controller calls [`wait_for_ready()`] to synchronize with the child.
/// 3. Controller writes raw bytes (e.g. [`ANSI`] key sequences) to the [`pty`] writer.
/// 4. Controlled reads those bytes from its [`stdin`], processes them, and prints
///    results.
/// 5. Controller calls [`read_line_state()`] to read results back and asserts
///    correctness.
/// 6. Controller calls [`drain_and_wait()`] to cleanly exit the test.
///
/// ## [`PTY`] Buffer Deadlock
///
/// Because the controller does all of this on a **single thread**, calling bare
/// [`wait()`] without first draining the [`pty`] buffer causes a deadlock - the
/// controller waits for the child to exit while the child's [`exit()`] flush blocks on a
/// full buffer that nobody is reading. See [`drain_and_wait()`] for the full deadlock
/// sequence and solution.
///
/// <div class="warning">
///
/// See [two types of PTY deadlock] for more details.
///
/// </div>
///
/// ## How This Macro Prevents It
///
/// This macro wraps the child in [`SingleThreadSafeControlledChild`], which hides
/// [`wait()`] entirely. The only exit path is [`drain_and_wait()`]. A [`PtyTestWatchdog`]
/// runs alongside as a safety net - if the controller hangs for any reason, the watchdog
/// terminates the child process after [`timeout`], converting an infinite hang into a
/// bounded test failure.
///
/// <div class="warning">
///
/// **This is not needed in production code.** Production [`pty`] sessions run [`wait()`]
/// inside [`tokio::task::spawn_blocking`] while separate [`tokio`] tasks concurrently
/// drain the buffer. Reading and waiting happen on different tasks, so the buffer never
/// fills up.
///
/// </div>
///
/// # Design Rationale: Dependency Injection at the Macro Level
///
/// **Why the macro passes [`pty`] resources as parameters instead of handling
/// everything:**
///
/// The macro creates [`pty`] infrastructure but delegates verification to your controller
/// function. This is **dependency injection** - the macro injects resources your function
/// needs, but doesn't dictate how to use them.
///
/// **Benefits of this approach:**
///
/// 1. **Different verification strategies**: Some tests check terminal settings, others
///    parse [`ANSI`] sequences, others send input and verify responses.
/// 2. **Flexible resource usage**: Some tests need writers (to send input), some only
///    need readers (to observe output).
/// 3. **Explicit dependencies**: Controller function signature clearly shows what it
///    needs, instead of hiding dependencies in macro magic.
/// 4. **Testability via Dependency Injection**: Because the controller function is
///    decoupled from the macro's process spawning logic, you can unit-test your
///    verification assertions in isolation. While the macro's primary purpose is **real
///    I/O integration testing** (no mocks), this [`DI`] pattern allows you to pass a
///    'mock' or manually-controlled [`pty`] pair to the controller function to test its
///    logic without the overhead of a real subprocess. Examples of mocks include
///    memory-backed buffers like [`OffscreenBuffer`].
///
/// # Notes
///
/// - The macro creates a [standard terminal size] [`pty`] pair.
/// - The controlled function does not need to call [`std::process::exit(0)`] since this
///   macro takes care of this automatically.
/// - When [`mode: PtyTestMode::Raw`], the macro enables raw mode in the controlled
///   process before calling the controlled function. When [`mode: PtyTestMode::Cooked`],
///   the controlled process starts in the default cooked mode.
/// - Verification logic is YOUR responsibility in controller function.
///
/// ## [`pty`] Stream Behavior
///
/// **Important:** In a [`pty`], [`stdout`] and [`stderr`] are **merged into a single
/// stream** from the controller's perspective. This means:
///
/// - Controlled's `println!()` and `eprintln!()` both go to the same merged stream.
/// - Controller reads both streams together via
///   [`pty_pair.controller().try_clone_reader()`].
/// - There is NO semantic difference between [`stdout`] and [`stderr`] in [`pty`] tests.
/// - Use **content-based filtering** to distinguish messages, not stream type.
///
/// ## Example
///
/// ```no_run
/// # use std::io::BufRead;
/// # use r3bl_tui::SingleThreadSafeControlledChild;
/// # fn _doc_test(child: SingleThreadSafeControlledChild, mut buf_reader: impl BufRead) {
/// // Controlled code (both println! & eprintln! go to the same stream!)
/// println!("Line: hello, Cursor: 5");        // Protocol message
/// println!("🔍 PTY Controlled: Event: ..."); // Debug message
///
/// // Controller code (uses read_line_state to handle filtering and synchronization)
/// let result = child.read_line_state(&mut buf_reader, "Line:");
/// assert_eq!(result, "Line: hello, Cursor: 5");
/// # }
/// ```
///
/// This is why all output in [`pty`] controlled processes should use `println!()` - using
/// `eprintln!()` creates a false impression that stderr is handled differently.
///
/// For complete [`pty`] test implementations, see:
/// - [`raw_mode_integration_tests`] - Tests raw mode itself.
/// - [`integration_tests`] - Tests input parsing.
///
/// # Arguments
///
/// - `test_fn`: The test function name (used as identifier, not string).
/// - `controller`: A function that accepts a [`PtyTestContext`] parameter.
/// - `controlled`: A function or expression that runs in the controlled process (must not
///   return).
/// - `mode`: A [`PtyTestMode`] value - [`Raw`] to enable raw mode before calling the
///   controlled function, or [`Cooked`] to leave the terminal in its default cooked mode.
///
/// # Controller Function Signature
///
/// Your controller function receives a [`PtyTestContext`] which bundles the following:
/// - [`pty_pair: PtyPair`] - The [`PTY`] pair that provides the controller and controlled
///   ends of the terminal used by the controller and controlled processes.
/// - [`child: SingleThreadSafeControlledChild`] - A handle to the spawned controlled
///   process, wrapped in a guard that enforces correct cleanup (see
///   [`SingleThreadSafeControlledChild`]).
/// - [`buf_reader: BufReader<ControllerReader>`] - A buffered reader for the [`PTY`]
///   controller side.
/// - [`writer: ControllerWriter`] - A writer for sending input to the [`PTY`] controller
///   side.
///
/// You can then:
/// - Use [`context.buf_reader`]: To read output from the child.
/// - Use [`context.writer`]: To write input to the child.
/// - Use [`context.child.drain_and_wait(context.buf_reader, context.pty_pair)`]: To
///   cleanly exit the test, preventing macOS [`PTY`] buffer deadlocks (see
///   [`drain_and_wait()`]).
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
/// [`buf_reader: BufReader<ControllerReader>`]: field@crate::PtyTestContext::buf_reader
/// [`child: SingleThreadSafeControlledChild`]: field@crate::PtyTestContext::child
/// [`context.buf_reader`]: field@crate::PtyTestContext::buf_reader
/// [`context.child.drain_and_wait(context.buf_reader, context.pty_pair)`]:
///     crate::SingleThreadSafeControlledChild::drain_and_wait
/// [`context.writer`]: field@crate::PtyTestContext::writer
/// [`CONTROLLED_READY`]: crate::pty_test_fixtures::CONTROLLED_READY
/// [`Cooked`]: PtyTestMode::Cooked
/// [`DI`]: https://en.wikipedia.org/wiki/Dependency_injection
/// [`drain_and_wait()`]: crate::SingleThreadSafeControlledChild::drain_and_wait
/// [`exit()`]: std::process::exit
/// [`generate_pty_test!`]: crate::generate_pty_test
/// [`integration_tests`]:
///     mod@crate::vt_100_terminal_input_parser::integration_tests
/// [`mode: PtyTestMode::Cooked`]: PtyTestMode::Cooked
/// [`mode: PtyTestMode::Raw`]: PtyTestMode::Raw
/// [`OffscreenBuffer`]: crate::OffscreenBuffer
/// [`pty_pair.controller().try_clone_reader()`]:
///     portable_pty::MasterPty::try_clone_reader
/// [`pty_pair.controller_mut().take_writer()`]: portable_pty::MasterPty::take_writer
/// [`pty_pair: PtyPair`]: field@crate::PtyTestContext::pty_pair
/// [`pty_types`]: mod@crate::pty_engine::pty_engine_types
/// [`pty`]: crate::core::pty
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
/// [`PtyTestContext`]: crate::PtyTestContext
/// [`PtyTestWatchdog`]: crate::PtyTestWatchdog
/// [`raw_mode_integration_tests`]:
///     mod@crate::terminal_raw_mode::integration_tests
/// [`Raw`]: PtyTestMode::Raw
/// [`read_line_state()`]: crate::SingleThreadSafeControlledChild::read_line_state
/// [`SingleThreadSafeControlledChild`]: crate::SingleThreadSafeControlledChild
/// [`spawn_controlled_in_pty`]: crate::spawn_controlled_in_pty
/// [`std::process::exit(0)`]: std::process::exit
/// [`stderr`]: std::io::Stderr
/// [`stdin`]: std::io::Stdin
/// [`stdout`]: std::io::Stdout
/// [`timeout`]: crate::PTY_TEST_WATCHDOG_TIMEOUT
/// [`tokio`]: tokio
/// [`wait()`]: portable_pty::Child::wait
/// [`wait_for_ready()`]:
///     crate::pty_test_fixtures::SingleThreadSafeControlledChild::wait_for_ready
/// [`writer: ControllerWriter`]: field@crate::PtyTestContext::writer
/// [raw mode]: crate::terminal_raw_mode#raw-mode-vs-cooked-mode
/// [standard terminal size]: crate::DefaultPtySize
/// [two types of PTY deadlock]: crate::PtyPair#two-types-of-deadlocks
#[macro_export]
macro_rules! generate_pty_test {
    (
        $(#[$meta:meta])*
        test_fn: $test_name:ident,
        controller: $controller_fn:expr,
        controlled: $controlled_fn:expr,
        mode: $mode:expr $(,)?
    ) => {
        $(#[$meta])*
        #[test]
        fn $test_name() {
            use std::io::Write;
            use $crate::{PtyCommand, PtyPair};

            // Immediate debug output to confirm test is running
            let pty_controlled_env_var = std::env::var($crate::PTY_CONTROLLED_ENV_VAR);
            eprintln!("🔍 TEST ENTRY: {} env = {:?}", $crate::PTY_CONTROLLED_ENV_VAR, pty_controlled_env_var);

            // Also print to stdout to ensure it gets through PTY
            println!("{}", $crate::TEST_RUNNING);
            std::io::stdout().flush().expect("Failed to flush stdout");

            // Check if we're running as the controlled process
            if pty_controlled_env_var.is_ok() {
                eprintln!("🔍 TEST: {} detected, running controlled mode", $crate::PTY_CONTROLLED_ENV_VAR);
                println!("{}", $crate::CONTROLLED_STARTING);
                std::io::stdout().flush().expect("Failed to flush stdout");

                // Enable raw mode if requested by the test.
                if $mode == $crate::PtyTestMode::Raw {
                    if let Err(e) = $crate::enable_raw_mode() {
                        eprintln!("Failed to enable raw mode: {e}");
                    }
                }

                // Run the controlled logic.
                $controlled_fn();

                // Restore cooked mode for correctness.
                #[allow(unreachable_code)]
                if $mode == $crate::PtyTestMode::Raw {
                    drop($crate::disable_raw_mode());
                }

                // Exit the process, so there is no recursion.
                std::process::exit(0);
            }

            // Otherwise, run as controller - create PTY and spawn controlled
            eprintln!("🚀 TEST: No {} var, running as controller", $crate::PTY_CONTROLLED_ENV_VAR);

            // Spawn controlled process
            let test_binary =
                std::env::current_exe().expect("Failed to get current executable");
            let mut cmd = PtyCommand::new(&test_binary);
            cmd.env($crate::PTY_CONTROLLED_ENV_VAR, "1");
            cmd.env("RUST_BACKTRACE", "1");
            cmd.args(&["--test-threads", "1", "--nocapture", stringify!($test_name)]);

            eprintln!("🚀 Controller: Spawning controlled process...");
            let (pty_pair, child) = PtyPair::open_and_spawn($crate::DefaultPtySize, cmd)
                .expect("Failed to spawn controlled process");
            eprintln!("🔍 Controller: PTY pair created");
            eprintln!("🔍 Controller: Controlled side closed (parent no longer holds controlled fd)");

            // Wrap in SingleThreadSafeControlledChild — bare child.wait() is now impossible.
            // The only exit path is drain_and_wait(), preventing PTY buffer deadlocks.
            let child = $crate::SingleThreadSafeControlledChild::new(child);

            // Start watchdog timer — kills child after timeout if controller hangs.
            let termination_handle = child.clone_termination_handle();
            let _watchdog = $crate::PtyTestWatchdog::new(
                termination_handle,
            );

            // Call user's controller function with PTY resources.
            // When this returns, _watchdog drops → sets cancelled flag → thread exits
            // cleanly on wake.
            let reader = pty_pair
                .controller()
                .try_clone_reader()
                .expect("Failed to clone reader");
            let mut buf_reader = std::io::BufReader::new(reader);

            // Perform handshake (no-op on Unix, mandatory on Windows).
            let writer = child.get_writer_with_handshake(&pty_pair, &mut buf_reader);

            let context = $crate::PtyTestContext {
                pty_pair,
                child,
                buf_reader,
                writer,
            };

            $controller_fn(context);
        }
    };
}

/// Whether the macro should enable raw mode in the controlled process before calling the
/// controlled function.
///
/// Most [`pty`] tests need [raw mode] so the line discipline passes keystrokes through
/// immediately (without waiting for Enter). Tests that manage [raw mode] themselves
/// (e.g., tests for [raw mode] toggling) or that never read [`stdin`] should use
/// [`Cooked`].
///
/// [`Cooked`]: PtyTestMode::Cooked
/// [`pty`]: crate::core::pty
/// [`stdin`]: std::io::Stdin
/// [raw mode]: crate::terminal_raw_mode#raw-mode-vs-cooked-mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtyTestMode {
    /// Enable [raw mode] in the controlled process before calling the controlled
    /// function. Use this for tests that read keystrokes via [`DirectToAnsiInputDevice`]
    /// or any [`stdin`]-reading code that expects [raw mode].
    ///
    /// [`DirectToAnsiInputDevice`]: crate::DirectToAnsiInputDevice
    /// [`stdin`]: std::io::Stdin
    /// [raw mode]: crate::terminal_raw_mode#raw-mode-vs-cooked-mode
    Raw,
    /// Do not change the terminal mode. The controlled process starts in the
    /// default [cooked mode]. Use for tests that manage raw mode themselves or that
    /// never read [`stdin`].
    ///
    /// [`stdin`]: std::io::Stdin
    /// [cooked mode]: crate::terminal_raw_mode#raw-mode-vs-cooked-mode
    Cooked,
}
