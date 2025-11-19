// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

/// Macro that generates PTY-based integration tests with automatic test name injection.
///
/// This macro handles the boilerplate for PTY-based integration tests:
///
/// 1. **CI detection**: Automatically skips the test in CI environments
/// 2. **Process routing**: Routes to master or slave code based on environment variable
/// 3. **PTY setup**: Creates PTY pair and spawns slave process automatically
/// 4. **Debug output**: Prints diagnostic messages for troubleshooting
///
/// The macro creates the PTY and spawns the process, then passes these resources
/// to your master function so you can focus on verification logic.
///
/// # Architecture
///
/// ```text
/// ┌─────────────────────────────────────────────────────────────┐
/// │ Test Function (entry point)                                 │
/// │  - Macro detects role via environment variable              │
/// │  - Routes to master or slave function                       │
/// │  - Skips in CI environments automatically                   │
/// └────────────┬────────────────────────────────┬───────────────┘
///              │                                │
///       Master Path                      Slave Path
///              │                                │
/// ┌────────────▼───────────┐    ┌───────────────▼───────────────┐
/// │ Macro: PTY Setup       │    │ Slave Function                │
/// │ - Creates PTY pair     │    │ - Enable raw mode (if needed) │
/// │ - Spawns slave         ├────▶ - Execute test logic          │
/// │ - Passes to master fn  │    │ - Output via stdout/stderr    │
/// └────────────┬───────────┘    └────────────▲─┬────────────────┘
///              │                             │ │
/// ┌────────────▼──────────────────┐          │ │
/// │ Master Function               │          │ │ PTY I/O
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
/// The macro creates PTY infrastructure but delegates verification to your master
/// function. This is **dependency injection** - the macro injects resources your function
/// needs, but doesn't dictate how to use them.
///
/// **Benefits of this approach:**
///
/// 1. **Different verification strategies**: Some tests check terminal settings, others
///    parse ANSI sequences, others send input and verify responses
/// 2. **Flexible resource usage**: Some tests need writers (to send input), some only
///    need readers (to observe output)
/// 3. **Explicit dependencies**: Master function signature clearly shows what it needs
///    (`pty_pair`, `child`) instead of hiding dependencies in macro magic
/// 4. **Testability**: Each master function is independently testable with mock PTY pairs
///
/// # Notes
///
/// - The macro creates a 24x80 PTY pair (standard terminal size)
/// - The slave function MUST call `std::process::exit(0)` to prevent test recursion
/// - Raw mode management is YOUR responsibility in slave function
/// - Verification logic is YOUR responsibility in master function
///
/// ## PTY Stream Behavior
///
/// **Important:** In a PTY, stdout and stderr are **merged into a single stream** from
/// the master's perspective. This means:
///
/// - Slave's `println!()` and `eprintln!()` both go to the same merged stream
/// - Master reads both streams together via `pty_pair.master.try_clone_reader()`
/// - There is NO semantic difference between stdout and stderr in PTY tests
/// - Use **content-based filtering** to distinguish messages, not stream type
///
/// ## Example
///
/// <!-- It is ok to use ignore here - this is a conceptual example showing PTY stream
/// merging behavior, not a complete compilable example -->
/// ```ignore
/// // Slave code (both go to the same stream!)
/// println!("Line: hello, Cursor: 5");      // Protocol message
/// println!("🔍 PTY Slave: Event: ...");    // Debug message (use println!, not eprintln!)
///
/// // Master code (filters by content, not stream)
/// if line.starts_with("Line:") {
///     // Protocol message - parse it
/// } else {
///     // Debug message - skip it
/// }
/// ```
///
/// This is why all output in PTY slaves should use `println!()` - using `eprintln!()`
/// creates a false impression that stderr is handled differently.
///
/// For complete PTY test implementations, see:
/// - [`raw_mode_integration_tests`] - Tests raw mode itself
/// - [`integration_tests`] - Tests input parsing
///
/// # Parameters
///
/// - `test_fn`: The test function name (used as identifier, not string)
/// - `slave`: A function or expression that runs in the slave process (must not return)
/// - `master`: A function that accepts `(pty_pair, child)` parameters
///
/// # Master Function Signature
///
/// Your master function receives:
/// - `pty_pair: portable_pty::PtyPair` - The PTY pair for communication
/// - `child: Box<dyn portable_pty::Child + Send + Sync>` - The spawned slave process
///
/// You can then:
/// - Get a reader: `pty_pair.master.try_clone_reader()`
/// - Get a writer: `pty_pair.master.take_writer()`
/// - Wait for child: `child.wait()`
///
/// [`raw_mode_integration_tests`]: mod@crate::core::ansi::terminal_raw_mode::integration_tests
/// [`integration_tests`]: mod@crate::core::ansi::vt_100_terminal_input_parser::integration_tests
#[macro_export]
macro_rules! generate_pty_test {
    (
        $(#[$meta:meta])*
        test_fn: $test_name:ident,
        master: $master_fn:expr,
        slave: $slave_fn:expr
    ) => {
        $(#[$meta])*
        #[test]
        fn $test_name() {
            use std::io::Write;
            use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};

            const PTY_SLAVE_ENV_VAR: &str = "R3BL_PTY_TEST_SLAVE";

            // Immediate debug output to confirm test is running
            let pty_slave_env_var = std::env::var(PTY_SLAVE_ENV_VAR);
            eprintln!("🔍 TEST ENTRY: {} env = {:?}", PTY_SLAVE_ENV_VAR, pty_slave_env_var);

            // Also print to stdout to ensure it gets through PTY
            println!("TEST_RUNNING");
            std::io::stdout().flush().expect("Failed to flush stdout");

            // Skip in CI if running as master
            if pty_slave_env_var.is_err() && is_ci::cached() {
                println!("⏭️  Skipped in CI (requires interactive terminal)");
                return;
            }

            // Check if we're running as the slave process
            if pty_slave_env_var.is_ok() {
                eprintln!("🔍 TEST: {} detected, running slave mode", PTY_SLAVE_ENV_VAR);
                println!("SLAVE_STARTING");
                std::io::stdout().flush().expect("Failed to flush stdout");

                // Run the slave logic (never returns - exits process)
                $slave_fn();
            }

            // Otherwise, run as master - create PTY and spawn slave
            eprintln!("🚀 TEST: No {} var, running as master", PTY_SLAVE_ENV_VAR);

            // Create PTY pair
            let pty_system = NativePtySystem::default();
            let pty_pair = pty_system
                .openpty(PtySize {
                    rows: 24,
                    cols: 80,
                    pixel_width: 0,
                    pixel_height: 0,
                })
                .expect("Failed to create PTY pair");

            eprintln!("🔍 Master: PTY pair created");

            // Spawn slave process
            let test_binary =
                std::env::current_exe().expect("Failed to get current executable");
            let mut cmd = CommandBuilder::new(&test_binary);
            cmd.env(PTY_SLAVE_ENV_VAR, "1");
            cmd.env("RUST_BACKTRACE", "1");
            cmd.args(&["--test-threads", "1", "--nocapture", stringify!($test_name)]);

            eprintln!("🚀 Master: Spawning slave process...");
            let child = pty_pair
                .slave
                .spawn_command(cmd)
                .expect("Failed to spawn slave process");

            // Call user's master function with PTY resources
            $master_fn(pty_pair, child);
        }
    };
}
