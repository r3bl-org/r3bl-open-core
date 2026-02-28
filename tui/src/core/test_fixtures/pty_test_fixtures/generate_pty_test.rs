// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

/// Whether the macro should enable raw mode in the controlled process before calling the
/// controlled function.
///
/// Most [`pty`] tests need raw mode so the line discipline passes keystrokes through
/// immediately (without waiting for Enter). Tests that manage raw mode themselves (e.g.,
/// tests for raw mode toggling) or that never read stdin should use [`Cooked`].
///
/// [`Cooked`]: PtyTestMode::Cooked
/// [`pty`]: crate::core::pty
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtyTestMode {
    /// Enable raw mode in the controlled process before calling the controlled
    /// function. Use this for tests that read keystrokes via
    /// [`DirectToAnsiInputDevice`] or any stdin-reading code that expects raw mode.
    ///
    /// [`DirectToAnsiInputDevice`]: crate::DirectToAnsiInputDevice
    Raw,
    /// Do not change the terminal mode. The controlled process starts in the
    /// default cooked mode. Use for tests that manage raw mode themselves or that
    /// never read stdin.
    Cooked,
}

/// Macro that generates [`pty`]-based integration tests with automatic test name
/// injection.
///
/// Use this macro for **single-feature [`pty`] tests** that test one specific behavior
/// (e.g., raw mode, Ctrl+W deletion, terminal event parsing). For **multi-backend
/// comparison tests** that need to run the same controlled code with different backends
/// and compare results, use [`spawn_controlled_in_pty`] instead.
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
/// This macro handles the boilerplate for [`pty`]-based integration tests:
///
/// 1. **Process routing**: Routes to controller or controlled code based on environment
///    variable
/// 2. **[`pty`] setup**: Creates [`pty`] pair and spawns controlled process automatically
/// 3. **Debug output**: Prints diagnostic messages for troubleshooting
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
///    parse [`ANSI`] sequences, others send input and verify responses
/// 2. **Flexible resource usage**: Some tests need writers (to send input), some only
///    need readers (to observe output)
/// 3. **Explicit dependencies**: Controller function signature clearly shows what it
///    needs (`pty_pair`, `child`) instead of hiding dependencies in macro magic
/// 4. **Testability**: Each controller function is independently testable with mock
///    [`pty`] pairs
///
/// # Notes
///
/// - The macro creates a 24x80 [`pty`] pair (standard terminal size)
/// - The controlled function MUST call `std::process::exit(0)` to prevent test recursion
/// - When `mode: PtyTestMode::Raw`, the macro enables raw mode in the controlled process
///   before calling the controlled function. When `mode: PtyTestMode::Cooked`, the
///   controlled process starts in the default cooked mode.
/// - Verification logic is YOUR responsibility in controller function
///
/// ## [`pty`] Stream Behavior
///
/// **Important:** In a [`pty`], stdout and stderr are **merged into a single stream**
/// from the controller's perspective. This means:
///
/// - Controlled's `println!()` and `eprintln!()` both go to the same merged stream
/// - Controller reads both streams together via
///   `pty_pair.controller().try_clone_reader()`
/// - There is NO semantic difference between stdout and stderr in [`pty`] tests
/// - Use **content-based filtering** to distinguish messages, not stream type
///
/// ## Example
///
/// <!-- It is ok to use ignore here - this is a conceptual example showing [`pty`] stream
/// merging behavior, not a complete compilable example -->
///
/// ```ignore
/// // Controlled code (both println! & eprintln! go to the same stream!)
/// println!("Line: hello, Cursor: 5");        // Protocol message
/// println!("🔍 PTY Controlled: Event: ..."); // Debug message (use println!, not eprintln!)
///
/// // Controller code (filters by content, not stream)
/// if line.starts_with("Line:") {
///     // Protocol message - parse it
/// } else {
///     // Debug message - skip it
/// }
/// ```
///
/// This is why all output in [`pty`] controlled processes should use `println!()` - using
/// `eprintln!()` creates a false impression that stderr is handled differently.
///
/// For complete [`pty`] test implementations, see:
/// - [`raw_mode_integration_tests`] - Tests raw mode itself
/// - [`integration_tests`] - Tests input parsing
///
/// # Parameters
///
/// - `test_fn`: The test function name (used as identifier, not string)
/// - `controller`: A function that accepts `(pty_pair, child)` parameters
/// - `controlled`: A function or expression that runs in the controlled process (must not
///   return)
/// - `mode`: A [`PtyTestMode`] value - [`Raw`] to enable raw mode before calling the
///   controlled function, or [`Cooked`] to leave the terminal in its default cooked mode
///
/// # Controller Function Signature
///
/// Your controller function receives:
/// - `pty_pair: PtyPair` - The [`pty`] pair wrapper for communication
/// - `child: SingleThreadSafeControlledChild` - The spawned controlled process wrapped in
///   a guard that enforces correct cleanup (see [`SingleThreadSafeControlledChild`])
///
/// You can then:
/// - Get a reader: `pty_pair.controller().try_clone_reader()`
/// - Get a writer: `pty_pair.controller_mut().take_writer()`
/// - Drain [`pty`] and wait: `child.drain_and_wait(buf_reader, pty_pair)` — consumes
///   child, prevents macOS [`pty`] buffer deadlocks (see [`drain_pty_and_wait`])
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
/// [`Cooked`]: PtyTestMode::Cooked
/// [`drain_pty_and_wait`]: crate::drain_pty_and_wait
/// [`generate_pty_test!`]: crate::generate_pty_test
/// [`integration_tests`]:
///     mod@crate::core::ansi::vt_100_terminal_input_parser::integration_tests
/// [`pty_types`]: mod@crate::core::pty::pty_core::pty_types
/// [`pty`]: crate::core::pty
/// [`raw_mode_integration_tests`]:
///     mod@crate::core::ansi::terminal_raw_mode::integration_tests
/// [`Raw`]: PtyTestMode::Raw
/// [`SingleThreadSafeControlledChild`]: crate::SingleThreadSafeControlledChild
/// [`spawn_controlled_in_pty`]: crate::spawn_controlled_in_pty
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
            use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
            use $crate::PtyPair;

            const PTY_CONTROLLED_ENV_VAR: &str = "R3BL_PTY_TEST_CONTROLLED";

            // Immediate debug output to confirm test is running
            let pty_controlled_env_var = std::env::var(PTY_CONTROLLED_ENV_VAR);
            eprintln!("🔍 TEST ENTRY: {} env = {:?}", PTY_CONTROLLED_ENV_VAR, pty_controlled_env_var);

            // Also print to stdout to ensure it gets through PTY
            println!("TEST_RUNNING");
            std::io::stdout().flush().expect("Failed to flush stdout");

            // Check if we're running as the controlled process
            if pty_controlled_env_var.is_ok() {
                eprintln!("🔍 TEST: {} detected, running controlled mode", PTY_CONTROLLED_ENV_VAR);
                println!("CONTROLLED_STARTING");
                std::io::stdout().flush().expect("Failed to flush stdout");

                // Enable raw mode if requested by the test.
                if $mode == $crate::PtyTestMode::Raw {
                    if let Err(e) = $crate::enable_raw_mode() {
                        eprintln!("Failed to enable raw mode: {e}");
                    }
                }

                // Run the controlled logic (never returns - exits process).
                $controlled_fn();

                // Restore cooked mode for correctness. In practice this is
                // unreachable because controlled functions call
                // std::process::exit(0), but we include it so the
                // enable/disable contract is explicit.
                #[allow(unreachable_code)]
                if $mode == $crate::PtyTestMode::Raw {
                    drop($crate::disable_raw_mode());
                }
            }

            // Otherwise, run as controller - create PTY and spawn controlled
            eprintln!("🚀 TEST: No {} var, running as controller", PTY_CONTROLLED_ENV_VAR);

            // Create PTY pair
            let pty_system = NativePtySystem::default();
            let raw_pty_pair = pty_system
                .openpty(PtySize {
                    rows: 24,
                    cols: 80,
                    pixel_width: 0,
                    pixel_height: 0,
                })
                .expect("Failed to create PTY pair");

            // Wrap the PTY pair to use controller/controlled terminology
            let pty_pair = PtyPair::from(raw_pty_pair);

            eprintln!("🔍 Controller: PTY pair created");

            // Spawn controlled process
            let test_binary =
                std::env::current_exe().expect("Failed to get current executable");
            let mut cmd = CommandBuilder::new(&test_binary);
            cmd.env(PTY_CONTROLLED_ENV_VAR, "1");
            cmd.env("RUST_BACKTRACE", "1");
            cmd.args(&["--test-threads", "1", "--nocapture", stringify!($test_name)]);

            eprintln!("🚀 Controller: Spawning controlled process...");
            let child = pty_pair
                .controlled()
                .spawn_command(cmd)
                .expect("Failed to spawn controlled process");

            // Wrap in SingleThreadSafeControlledChild — bare child.wait() is now impossible.
            // The only exit path is drain_and_wait(), preventing PTY buffer deadlocks.
            let child = $crate::SingleThreadSafeControlledChild::new(child);

            // Start watchdog timer — kills child after timeout if controller hangs.
            let killer = child.clone_termination_handle();
            let _watchdog = $crate::PtyTestWatchdog::new(
                killer,
                $crate::PTY_TEST_WATCHDOG_TIMEOUT,
            );

            // Call user's controller function with PTY resources.
            // When this returns, _watchdog drops → sets cancelled flag → thread exits
            // cleanly on wake.
            $controller_fn(pty_pair, child);
        }
    };
}
