// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

/// Macro for PTY test coordinator with automatic PTY setup.
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
///              ▼                                ▼
/// ┌────────────────────────┐    ┌──────────────────────────────┐
/// │ Macro: PTY Setup       │    │ Slave Function               │
/// │ - Creates PTY pair     │    │ - Enable raw mode (if needed)│
/// │ - Spawns slave         │────▶ - Execute test logic         │
/// │ - Passes to master fn  │    │ - Report via stdout          │
/// └────────────┬───────────┘    └──────────────┬───────────────┘
///              │                               │ PTY stdout
///              ▼                               │
/// ┌────────────────────────┐                   │
/// │ Master Function        │                   │
/// │ - Receives pty_pair    │                   │
/// │ - Receives child       │                   │
/// │ - Reads results        │◀───────────────── ┘
/// │ - Verifies assertions  │
/// │ - Waits for child      │
/// └────────────────────────┘
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
/// # Syntax
///
/// ```ignore
/// run_test_in_isolated_process_with_pty!(
///     env_var: "MY_TEST_SLAVE",
///     test_name: "test_my_feature",  // Name of the #[test] function
///     slave: run_my_slave,
///     master: run_my_master
/// );
///
/// fn run_my_slave() -> ! {
///     println!("SLAVE_STARTING");
///     // ... test operations ...
///     std::process::exit(0);
/// }
///
/// fn run_my_master(
///     pty_pair: portable_pty::PtyPair,
///     mut child: Box<dyn portable_pty::Child + Send + Sync>
/// ) {
///     // Macro has already created PTY and spawned slave
///     // You just handle verification logic
///     let reader = pty_pair.master.try_clone_reader().unwrap();
///     let mut writer = pty_pair.master.take_writer().unwrap();  // If needed
///
///     // Your verification logic...
///     child.wait().unwrap();
/// }
/// ```
///
/// # Parameters
///
/// - `env_var`: The environment variable name used to identify the slave process
/// - `test_name`: The name of the test function (used to spawn correct test)
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
/// # Examples of code using this macro
///
/// For complete PTY test implementations, see:
/// - [`test_raw_mode_pty`] - Tests raw mode itself
/// - [`pty_based_input_device_test`] - Tests input parsing
///
/// [`test_raw_mode_pty`]: mod@crate::core::ansi::terminal_raw_mode::integration_tests
/// [`pty_based_input_device_test`]: mod@crate::core::ansi::vt_100_terminal_input_parser::integration_tests
#[macro_export]
macro_rules! run_test_in_isolated_process_with_pty {
    (
        env_var: $env_var:expr,
        test_name: $test_name:expr,
        slave: $slave_fn:expr,
        master: $master_fn:expr
    ) => {{
        use std::io::Write;

        // Immediate debug output to confirm test is running
        let pty_slave = std::env::var($env_var);
        eprintln!("🔍 TEST ENTRY: {} env = {:?}", $env_var, pty_slave);

        // Also print to stdout to ensure it gets through PTY
        println!("TEST_RUNNING");
        std::io::stdout().flush().expect("Failed to flush stdout");

        // Skip in CI if running as master
        if pty_slave.is_err() && is_ci::cached() {
            println!("⏭️  Skipped in CI (requires interactive terminal)");
            return;
        }

        // Check if we're running as the slave process
        if pty_slave.is_ok() {
            eprintln!("🔍 TEST: {} detected, running slave mode", $env_var);
            println!("SLAVE_STARTING");
            std::io::stdout().flush().expect("Failed to flush stdout");

            // Run the slave logic (never returns - exits process)
            $slave_fn();
        }

        // Otherwise, run as master - create PTY and spawn slave
        eprintln!("🚀 TEST: No {} var, running as master", $env_var);

        // Create PTY pair
        use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};

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
        cmd.env($env_var, "1");
        cmd.env("RUST_BACKTRACE", "1");
        cmd.args(&["--test-threads", "1", "--nocapture", $test_name]);

        eprintln!("🚀 Master: Spawning slave process...");
        let child = pty_pair
            .slave
            .spawn_command(cmd)
            .expect("Failed to spawn slave process");

        // Call user's master function with PTY resources
        $master_fn(pty_pair, child);
    }};
}
