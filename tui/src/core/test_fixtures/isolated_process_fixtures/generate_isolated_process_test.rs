// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Spawns the current test executable in an isolated child process. See
//! [`generate_isolated_process_test!`] for details.
//!
//! [`generate_isolated_process_test!`]: macro@crate::generate_isolated_process_test

/// Generates a test that spawns itself in an isolated process to avoid global state
/// contamination (environment variables, static variables, current working directory).
///
/// Use this macro for tests that need to modify global state that cannot be easily mocked
/// or localized (e.g., environment variables, [`std::env::set_current_dir`], or static
/// variables used by third-party libraries).
///
/// For **[`PTY`]-based integration tests**, use [`generate_pty_test!`] instead.
///
/// # When to Use This Macro vs [`generate_pty_test!`]
///
/// | Feature          | `generate_isolated_process_test!` | `generate_pty_test!`          |
/// | ---------------- | --------------------------------- | ----------------------------- |
/// | Isolation Type   | Standard process (no [`PTY`])     | [`PTY`]-based process         |
/// | Primary Purpose  | Global state (env/cwd/statics)    | Terminal UI & Event handling  |
/// | Async Support    | Use async variant                 | Native async support          |
/// | Windows Support  | Yes                               | Yes (via [`ConPTY`])          |
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
/// │ Macro: Child Setup     │    │ Controlled Function            │
/// │ - Sets ENV_VAR         │    │ - Modifies global state        │
/// │ - Spawns self process  ├────► - Executes test logic          │
/// │ - Captures stdio       │    │ - Prints to stdout/stderr      │
/// └────────────┬───────────┘    └────────────▲─┬─────────────────┘
///              │                             │ │
/// ┌────────────▼──────────────────┐          │ │
/// │ Controller Function           │          │ │ Process I/O
/// │ - Receives child Output       │          │ │ stdin, stdout, stderr
/// │ - Verifies child exit status  │          │ │
/// │ - Asserts on captured output  ◄────────────┘
/// └───────────────────────────────┘
/// ```
///
/// # Arguments
/// - `test_name`: The name of the test function.
/// - `controller_fn`: A function that receives the [`std::process::Output`] from the
///   spawned process.
/// - `controlled_fn`: A function that runs the actual test logic in the isolated process.
/// - `stdin`, `stdout`, `stderr`: [`std::process::Stdio`] configurations for the child.
///
/// # Example
/// ```no_run
/// # use r3bl_tui::generate_isolated_process_test;
/// generate_isolated_process_test!(
///     test_my_feature,
///     controller,
///     controlled,
///     std::process::Stdio::null(),
///     std::process::Stdio::piped(),
///     std::process::Stdio::piped()
/// );
///
/// fn controller(output: std::process::Output) {
///     assert!(output.status.success());
///     let stdout = String::from_utf8_lossy(&output.stdout);
///     assert!(stdout.contains("Hello from child"));
/// }
///
/// fn controlled() {
///     println!("Hello from child");
/// }
/// ```
///
/// [`ConPTY`]:
///     https://learn.microsoft.com/en-us/windows/console/creating-a-pseudoconsole-session
/// [`generate_pty_test!`]: crate::generate_pty_test
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
#[macro_export]
macro_rules! generate_isolated_process_test {
    (
        $(#[$meta:meta])*
        $test_name:ident,
        $controller_fn:ident,
        $controlled_fn:ident,
        $stdin:expr, $stdout:expr, $stderr:expr
    ) => {
        $(#[$meta])*
        #[test]
        fn $test_name() {
            if std::env::var($crate::ISOLATED_PROCESS_ENV_VAR).is_ok() {
                let _unused = $controlled_fn();
                std::process::exit(0);
            }

            let output = $crate::spawn_isolated_process(
                stringify!($test_name), $stdin, $stdout, $stderr,
            );
            $controller_fn(output);
        }
    };
}

/// Generates an asynchronous test that spawns itself in an isolated process to avoid
/// global state contamination (environment variables, static variables, current working
/// directory).
///
/// This is the asynchronous version (using [`tokio`]). For synchronous tests, use
/// [`generate_isolated_process_test!`] instead.
///
/// # Architecture
///
/// The architecture is identical to [`generate_isolated_process_test!`], but using
/// `async`/`.await` and `#[tokio::test]`.
///
/// # Arguments
/// - `test_name`: The name of the test function.
/// - `controller_fn`: A function that receives the [`std::process::Output`] from the
///   spawned process. This is always sync (it just inspects the child's output).
/// - `controlled_fn`: An `async` function that runs the actual test logic in the isolated
///   process.
/// - `stdin`, `stdout`, `stderr`: [`std::process::Stdio`] configurations for the child.
///
/// # Example
/// ```no_run
/// # use r3bl_tui::generate_async_isolated_process_test;
/// generate_async_isolated_process_test!(
///     test_my_async_feature,
///     controller,
///     controlled,
///     std::process::Stdio::null(),
///     std::process::Stdio::piped(),
///     std::process::Stdio::piped()
/// );
///
/// fn controller(output: std::process::Output) {
///     assert!(output.status.success());
/// }
///
/// async fn controlled() {
///     // Run async logic here
/// }
/// ```
///
/// [`generate_isolated_process_test!`]: macro@crate::generate_isolated_process_test
/// [`tokio`]: tokio
#[macro_export]
macro_rules! generate_async_isolated_process_test {
    (
        $(#[$meta:meta])*
        $test_name:ident,
        $controller_fn:ident, $controlled_fn:ident,
        $stdin:expr, $stdout:expr, $stderr:expr
    ) => {
        $(#[$meta])*
        #[tokio::test]
        async fn $test_name() {
            if std::env::var($crate::ISOLATED_PROCESS_ENV_VAR).is_ok() {
                let _unused = $controlled_fn().await;
                std::process::exit(0);
            }

            let output = $crate::spawn_isolated_process(
                stringify!($test_name), $stdin, $stdout, $stderr,
            );
            $controller_fn(output);
        }
    };
}

/// Environment variable used to signal that the current process is the controlled
/// (isolated) child. The controller sets this before spawning; the controlled checks it
/// on entry.
pub const ISOLATED_PROCESS_ENV_VAR: &str = "R3BL_TEST_ISOLATED_PROCESS";

/// Spawns the current test executable in an isolated child process and returns its
/// [`Output`].
///
/// This is the controller-side logic shared by both [`generate_isolated_process_test!`]
/// and [`generate_async_isolated_process_test!`]. It:
/// 1. Calls [`suppress_wer_dialogs()`] to prevent Windows crash dialogs.
/// 2. Builds a command via [`new_isolated_test_command()`] with `--test-threads 1
///    --nocapture` and the given `test_name`.
/// 3. Sets `RUST_BACKTRACE=1` and the isolation environment variable.
/// 4. Configures [`stdin`], [`stdout`], [`stderr`] as specified by the caller.
/// 5. Runs the command and returns the captured [`Output`].
///
/// # Panics
///
/// Panics if the child process cannot be spawned.
///
/// [`generate_async_isolated_process_test!`]: crate::generate_async_isolated_process_test
/// [`generate_isolated_process_test!`]: macro@crate::generate_isolated_process_test
/// [`new_isolated_test_command()`]: crate::new_isolated_test_command
/// [`Output`]: std::process::Output
/// [`stderr`]: std::io::stderr
/// [`stdin`]: std::io::stdin
/// [`stdout`]: std::io::stdout
/// [`suppress_wer_dialogs()`]: crate::suppress_wer_dialogs
#[must_use]
pub fn spawn_isolated_process(
    test_name: &str,
    stdin: std::process::Stdio,
    stdout: std::process::Stdio,
    stderr: std::process::Stdio,
) -> std::process::Output {
    crate::suppress_wer_dialogs();

    let mut cmd = crate::new_isolated_test_command();
    cmd.args(["--test-threads", "1", "--nocapture", test_name])
        .env(ISOLATED_PROCESS_ENV_VAR, "1")
        .env("RUST_BACKTRACE", "1")
        .stdin(stdin)
        .stdout(stdout)
        .stderr(stderr);

    cmd.output().expect("Failed to spawn child process")
}
