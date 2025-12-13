// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

/// Macro that generates PTY-based integration tests with automatic test name injection.
///
/// This macro handles the boilerplate for PTY-based integration tests:
///
/// 1. **Process routing**: Routes to controller or controlled code based on environment
///    variable
/// 2. **PTY setup**: Creates PTY pair and spawns controlled process automatically
/// 3. **Debug output**: Prints diagnostic messages for troubleshooting
///
/// The macro creates the PTY and spawns the process, then passes these resources
/// to your controller function so you can focus on verification logic.
///
/// # Architecture
///
/// ```text
/// ┌─────────────────────────────────────────────────────────────┐
/// │ Test Function (entry point)                                 │
/// │  - Macro detects role via environment variable              │
/// │  - Routes to controller or controlled function              │
/// └────────────┬────────────────────────────────┬───────────────┘
///              │                                │
///     Controller Path                  Controlled Path
///              │                                │
/// ┌────────────▼───────────┐    ┌───────────────▼───────────────┐
/// │ Macro: PTY Setup       │    │ Controlled Function           │
/// │ - Creates PTY pair     │    │ - Enable raw mode (if needed) │
/// │ - Spawns controlled    ├────▶ - Execute test logic          │
/// │ - Passes to controller │    │ - Output via stdout/stderr    │
/// └────────────┬───────────┘    └────────────▲─┬────────────────┘
///              │                             │ │
/// ┌────────────▼──────────────────┐          │ │
/// │ Controller Function           │          │ │ PTY I/O
/// │ - Receives pty_pair           │          │ │ stdin, stdout/stderr
/// │ - Receives child              │          │ │
/// │ - Writes input to child (opt) ├──────────┘ │
/// │ - Reads results from child    ◀────────────┘
/// │ - Verifies assertions         │
/// │ - Waits for child exit        │
/// └───────────────────────────────┘
/// ```
///
/// # Design Rationale: Dependency Injection at the Macro Level
///
/// **Why the macro passes PTY resources as parameters instead of handling everything:**
///
/// The macro creates PTY infrastructure but delegates verification to your controller
/// function. This is **dependency injection** - the macro injects resources your function
/// needs, but doesn't dictate how to use them.
///
/// **Benefits of this approach:**
///
/// 1. **Different verification strategies**: Some tests check terminal settings, others
///    parse ANSI sequences, others send input and verify responses
/// 2. **Flexible resource usage**: Some tests need writers (to send input), some only
///    need readers (to observe output)
/// 3. **Explicit dependencies**: Controller function signature clearly shows what it
///    needs (`pty_pair`, `child`) instead of hiding dependencies in macro magic
/// 4. **Testability**: Each controller function is independently testable with mock PTY
///    pairs
///
/// # Notes
///
/// - The macro creates a 24x80 PTY pair (standard terminal size)
/// - The controlled function MUST call `std::process::exit(0)` to prevent test recursion
/// - Raw mode management is YOUR responsibility in controlled function
/// - Verification logic is YOUR responsibility in controller function
///
/// ## PTY Stream Behavior
///
/// **Important:** In a PTY, stdout and stderr are **merged into a single stream** from
/// the controller's perspective. This means:
///
/// - Controlled's `println!()` and `eprintln!()` both go to the same merged stream
/// - Controller reads both streams together via
///   `pty_pair.controller().try_clone_reader()`
/// - There is NO semantic difference between stdout and stderr in PTY tests
/// - Use **content-based filtering** to distinguish messages, not stream type
///
/// ## Example
///
/// <!-- It is ok to use ignore here - this is a conceptual example showing PTY stream
/// merging behavior, not a complete compilable example -->
/// ```ignore
/// // Controlled code (both go to the same stream!)
/// println!("Line: hello, Cursor: 5");      // Protocol message
/// println!("🔍 PTY Controlled: Event: ...");    // Debug message (use println!, not eprintln!)
///
/// // Controller code (filters by content, not stream)
/// if line.starts_with("Line:") {
///     // Protocol message - parse it
/// } else {
///     // Debug message - skip it
/// }
/// ```
///
/// This is why all output in PTY controlled processes should use `println!()` - using
/// `eprintln!()` creates a false impression that stderr is handled differently.
///
/// For complete PTY test implementations, see:
/// - [`raw_mode_integration_tests`] - Tests raw mode itself
/// - [`integration_tests`] - Tests input parsing
///
/// # Parameters
///
/// - `test_fn`: The test function name (used as identifier, not string)
/// - `controlled`: A function or expression that runs in the controlled process (must not
///   return)
/// - `controller`: A function that accepts `(pty_pair, child)` parameters
///
/// # Controller Function Signature
///
/// Your controller function receives:
/// - `pty_pair: PtyPair` - The PTY pair wrapper for communication
/// - `child: Box<dyn portable_pty::Child + Send + Sync>` - The spawned controlled process
///
/// You can then:
/// - Get a reader: `pty_pair.controller().try_clone_reader()`
/// - Get a writer: `pty_pair.controller_mut().take_writer()`
/// - Wait for child: `child.wait()`
///
/// [`raw_mode_integration_tests`]: mod@crate::core::ansi::terminal_raw_mode::integration_tests
/// [`integration_tests`]: mod@crate::core::ansi::vt_100_terminal_input_parser::integration_tests
#[macro_export]
macro_rules! generate_pty_test {
    (
        $(#[$meta:meta])*
        test_fn: $test_name:ident,
        controller: $controller_fn:expr,
        controlled: $controlled_fn:expr
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

                // Run the controlled logic (never returns - exits process)
                $controlled_fn();
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

            // Call user's controller function with PTY resources
            $controller_fn(pty_pair, child);
        }
    };
}
