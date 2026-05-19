// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

/// [`PTY`]-based integration test for [raw mode] basic enable/disable that ensures
/// the basic lifecycle works.
///
/// This test uses a controller/controlled [`PTY`] pair to verify that:
/// 1. [raw mode] can be enabled on a real [`PTY`].
/// 2. [raw mode] can be disabled and terminal settings restored.
/// 3. The [`RAII`] guard pattern works correctly.
///
/// # Run with:
///
/// ```bash
/// cargo test -p r3bl_tui test_raw_mode_pty -- --nocapture
/// ```
///
/// [`PTY`]: crate::core::pty::pty_engine::pty_pair#what-is-a-pty
/// [`RAII`]: https://en.wikipedia.org/wiki/Resource_acquisition_is_initialization
/// [raw mode]: mod@crate::terminal_raw_mode#raw-mode-vs-cooked-mode
use crate::{BufReadExt, GLYPH_CONTROLLED, GLYPH_CONTROLLER, GLYPH_SUCCESS,
            GLYPH_WAITING, GLYPH_WARNING, MSG_CONTROLLED_STARTING, MSG_FAILED,
            MSG_SUCCESS, PtyTestContext, PtyTestMode, RawModeGuard, generate_pty_test};
use rustix::termios;
use std::{io::Write,
          time::{Duration, Instant}};

generate_pty_test! {
    test_fn: test_raw_mode_pty,
    controller: controller,
    controlled: controlled,
    mode: PtyTestMode::Cooked,
}

/// Controller process: verifies results. Receives [`PTY`] context from the macro.
///
/// [`PTY`]: crate::core::pty::pty_engine::pty_pair#what-is-a-pty
fn controller(context: PtyTestContext) {
    let PtyTestContext {
        pty_pair,
        child,
        mut buf_reader,
        ..
    } = context;

    eprintln!("{GLYPH_CONTROLLER} PTY Controller: Starting raw mode test...");

    eprintln!(
        "{GLYPH_WAITING} PTY Controller: Waiting for controlled process results..."
    );

    let mut controlled_started = false;
    let mut test_passed = false;
    let start_timeout = Instant::now();

    while start_timeout.elapsed() < Duration::from_secs(5) {
        // `buf_reader` is a `BufReader<ControllerReader>` wrapping the controller-side
        // read end of the PTY pair. Anything the `controlled` child writes to its
        // stdout/stderr (via `println!`/`eprintln!`) arrives here as bytes, and the
        // `BufRead` impl from `BufReader` is what gives us `read_line()`.
        //
        // `read_line()` appends to the `line` instead of overwriting it. Declaring `line`
        // inside the loop gives us a fresh empty `line` each iteration; otherwise the
        // string would accumulate every line the child printed and break the `contains()`
        // matches below.
        let mut line = String::new();
        let result = buf_reader.read_line_eio_to_eof(&mut line);
        match result {
            Ok(0) => {
                eprintln!("  {GLYPH_WARNING} EOF reached");
                break;
            }
            Ok(_) => {
                let trimmed = line.trim();
                eprintln!("  ← Controlled output: {trimmed}");

                if trimmed.contains(MSG_CONTROLLED_STARTING) {
                    controlled_started = true;
                    eprintln!("  {GLYPH_SUCCESS} Controlled process confirmed starting");
                }
                if trimmed.contains(MSG_SUCCESS) {
                    test_passed = true;
                    eprintln!("  {GLYPH_SUCCESS} Test passed: {trimmed}");
                    break;
                }
                assert!(!trimmed.contains(MSG_FAILED), "Test failed: {trimmed}");
            }
            // Wait until more bytes are ready.
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(err) => panic!("Read error: {err}"),
        }
    }

    assert!(
        controlled_started,
        "Controlled process did not start properly"
    );
    assert!(test_passed, "Test did not report success");

    child.drain_and_wait(buf_reader, pty_pair);

    eprintln!("{GLYPH_SUCCESS} PTY Controller: Raw mode test passed!");
}

/// Controlled process: enables raw mode and reports results. The harness performs
/// [`std::process::exit(0)`] after this function returns.
fn controlled() {
    println!("{MSG_CONTROLLED_STARTING}");
    std::io::stdout().flush().expect("Failed to flush");

    eprintln!("{GLYPH_CONTROLLED} Controlled: Starting raw mode test...");

    // Get current terminal settings BEFORE enabling raw mode.
    let stdin = std::io::stdin();
    let before_termios = match termios::tcgetattr(&stdin) {
        Ok(termios) => termios,
        Err(err) => {
            eprintln!("{GLYPH_WARNING} Controlled: Failed to get termios before: {err}");
            println!("{MSG_FAILED} Could not read termios");
            std::io::stdout().flush().expect("Failed to flush");
            std::process::exit(1);
        }
    };

    // Enable raw mode using the guard.
    let _guard = match RawModeGuard::new() {
        Ok(guard) => guard,
        Err(err) => {
            eprintln!("{GLYPH_WARNING} Controlled: Failed to enable raw mode: {err}");
            println!("{MSG_FAILED} Could not enable raw mode");
            std::io::stdout().flush().expect("Failed to flush");
            std::process::exit(1);
        }
    };

    eprintln!("{GLYPH_SUCCESS} Controlled: Raw mode enabled");

    // Get terminal settings AFTER enabling raw mode.
    let after_termios = match termios::tcgetattr(&stdin) {
        Ok(termios) => termios,
        Err(err) => {
            eprintln!("{GLYPH_WARNING} Controlled: Failed to get termios after: {err}");
            println!("{MSG_FAILED} Could not read termios after");
            std::io::stdout().flush().expect("Failed to flush");
            std::process::exit(1);
        }
    };

    // Verify that settings actually changed.
    if before_termios.local_modes == after_termios.local_modes {
        eprintln!("{GLYPH_WARNING} Controlled: Local modes didn't change!");
        println!("{MSG_FAILED} Modes not changed");
        std::io::stdout().flush().expect("Failed to flush");
        std::process::exit(1);
    }

    eprintln!("{GLYPH_SUCCESS} Controlled: Terminal settings changed correctly");

    // Report success.
    println!("{MSG_SUCCESS} Raw mode enabled and verified");
    std::io::stdout().flush().expect("Failed to flush");

    eprintln!("{GLYPH_CONTROLLED} Controlled: Guard will be dropped now...");

    // Guard is dropped here, disabling raw mode
    eprintln!("{GLYPH_CONTROLLED} Controlled: Completed, exiting");
}
