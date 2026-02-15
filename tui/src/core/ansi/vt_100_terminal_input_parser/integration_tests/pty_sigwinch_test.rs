// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words tcgetwinsize

//! PTY integration test for SIGWINCH signal handling.
//!
//! This test validates that [`DirectToAnsiInputDevice`] correctly receives and handles
//! the SIGWINCH signal when the terminal is resized. Unlike the ANSI resize sequence
//! test in [`pty_terminal_events_test`], this test triggers a **real SIGWINCH signal**
//! by calling the PTY's `resize()` method.
//!
//! # Signal vs Sequence
//!
//! Terminal resize can be communicated in two ways:
//! - **SIGWINCH signal**: Sent by the kernel when the PTY size changes (tested here)
//! - **ANSI sequence**: `CSI 8;rows;cols t` sent by the terminal (tested in
//!   [`pty_terminal_events_test`])
//!
//! The [`DirectToAnsiInputDevice`] uses [`tokio::signal::unix::Signal`] to listen for
//! SIGWINCH, then queries the terminal size using `tcgetwinsize()`. This test verifies
//! that entire flow works correctly in a real PTY environment.
//!
//! # Test Architecture
//!
//! ```text
//! Controller                          Controlled (child process)
//! ──────────                          ─────────────────────────────
//!     │                                       │
//!     │  1. spawn in PTY (24×80)              │
//!     ├──────────────────────────────────────▶│
//!     │                                       │ 2. setup DirectToAnsiInputDevice
//!     │                                       │    • tokio::signal::unix::Signal
//!     │                                       │      listens for SIGWINCH
//!     │  3. wait for "CONTROLLED_READY"       │
//!     │◀──────────────────────────────────────┤
//!     │                                       │
//!     │  4. resize PTY to 100×30              │
//!     ├───────────────────────────────────────│──▶ kernel sends SIGWINCH
//!     │                                       │
//!     │                                       │ 5. tokio::select! wakes up
//!     │                                       │    on sigwinch_receiver.recv()
//!     │                                       │
//!     │                                       │ 6. get_size_rustix() calls
//!     │                                       │    tcgetwinsize(stdout) → (100,30)
//!     │                                       │
//!     │                                       │ 7. Returns InputEvent::Resize(100×30)
//!     │  8. verify "Resize: 100x30"           │
//!     │◀──────────────────────────────────────┤
//!     │                                       │
//! ```
//!
//! [`DirectToAnsiInputDevice`]: crate::direct_to_ansi::DirectToAnsiInputDevice
//! [`pty_terminal_events_test`]: super::pty_terminal_events_test

use crate::{ControlledChild, InputEvent, PtyPair, PtyTestMode, generate_pty_test,
            tui::terminal_lib_backends::direct_to_ansi::DirectToAnsiInputDevice};
use portable_pty::PtySize;
use std::{io::{BufRead, BufReader, Write},
          time::Duration};

/// Ready signal sent by controlled process after initialization.
const CONTROLLED_READY: &str = "CONTROLLED_READY";

generate_pty_test! {
    test_fn: test_pty_sigwinch,
    controller: pty_controller_entry_point,
    controlled: pty_controlled_entry_point,
    mode: PtyTestMode::Raw,
}

/// PTY Controller: Resize the PTY and verify the controlled process receives SIGWINCH.
fn pty_controller_entry_point(mut pty_pair: PtyPair, mut child: ControlledChild) {
    eprintln!("🚀 PTY Controller: Starting SIGWINCH test...");

    let reader = pty_pair
        .controller()
        .try_clone_reader()
        .expect("Failed to get reader");
    let mut buf_reader = BufReader::new(reader);

    eprintln!("📝 PTY Controller: Waiting for controlled process to start...");

    // Wait for controlled to confirm it's running. The controlled process sends
    // TEST_RUNNING and CONTROLLED_READY immediately on startup.
    let mut test_running_seen = false;

    loop {
        let mut line = String::new();
        match buf_reader.read_line(&mut line) {
            Ok(0) => panic!("EOF reached before controlled started"),
            Ok(_) => {
                let trimmed = line.trim();
                eprintln!("  ← Controlled output: {trimmed}");

                if trimmed.contains("TEST_RUNNING") {
                    test_running_seen = true;
                    eprintln!("  ✓ Test is running in controlled");
                }
                if trimmed.contains(CONTROLLED_READY) {
                    eprintln!("  ✓ Controlled process confirmed running!");
                    break;
                }
            }
            Err(e) => panic!("Read error while waiting for controlled: {e}"),
        }
    }

    assert!(
        test_running_seen,
        "Controlled test never started running (no TEST_RUNNING output)"
    );

    // Wait a bit for the controlled process to set up its signal handler.
    std::thread::sleep(Duration::from_millis(200));

    // Resize the PTY - this sends SIGWINCH to the controlled process.
    let new_size = PtySize {
        rows: 30,
        cols: 100,
        pixel_width: 0,
        pixel_height: 0,
    };

    eprintln!(
        "📐 PTY Controller: Resizing PTY to {}x{}...",
        new_size.cols, new_size.rows
    );

    pty_pair
        .controller_mut()
        .resize(new_size)
        .expect("Failed to resize PTY");

    eprintln!("  ✓ PTY resized, SIGWINCH should have been sent");

    // Wait for the controlled process to report the resize event.
    // The controlled process handles SIGWINCH and prints the new size immediately.
    let mut resize_received = false;

    loop {
        let mut line = String::new();
        match buf_reader.read_line(&mut line) {
            Ok(0) => {
                eprintln!("  ⚠️  EOF reached");
                break;
            }
            Ok(_) => {
                let trimmed = line.trim();
                eprintln!("  ← Controlled output: {trimmed}");

                // Look for resize event output.
                if trimmed.starts_with("Resize:") {
                    // Verify dimensions are correct.
                    if trimmed.contains("100") && trimmed.contains("30") {
                        eprintln!("  ✓ Received correct resize event: {trimmed}");
                        resize_received = true;
                        break;
                    }
                    eprintln!(
                        "  ⚠️  Resize dimensions don't match expected 100x30: {trimmed}"
                    );
                }
            }
            Err(e) => panic!("Read error: {e}"),
        }
    }

    assert!(
        resize_received,
        "Did not receive resize event from controlled process"
    );

    eprintln!("🧹 PTY Controller: Cleaning up...");

    // The controlled process should exit on its own after receiving the event.
    match child.wait() {
        Ok(status) => {
            eprintln!("✅ PTY Controller: Controlled process exited: {status:?}");
        }
        Err(e) => {
            eprintln!("⚠️  PTY Controller: Error waiting for controlled: {e}");
        }
    }

    eprintln!("✅ PTY Controller: SIGWINCH test passed!");
}

/// PTY Controlled: Set up signal handler and wait for SIGWINCH.
fn pty_controlled_entry_point() -> ! {
    println!("{CONTROLLED_READY}");
    std::io::stdout().flush().expect("Failed to flush");

    let runtime = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    runtime.block_on(async {
        eprintln!("🔍 PTY Controlled: Starting DirectToAnsiInputDevice...");
        let mut input_device = DirectToAnsiInputDevice::new();
        eprintln!("🔍 PTY Controlled: Device created, waiting for SIGWINCH...");

        let inactivity_timeout = Duration::from_secs(5);
        let mut inactivity_deadline = tokio::time::Instant::now() + inactivity_timeout;

        loop {
            tokio::select! {
                event_result = input_device.next() => {
                    match event_result {
                        Some(InputEvent::Resize(size)) => {
                            eprintln!("🔍 PTY Controlled: Received resize event: {size:?}");

                            // Output in a format the controller can parse.
                            println!("Resize: {}x{}", size.col_width.as_usize(), size.row_height.as_usize());
                            std::io::stdout().flush().expect("Failed to flush stdout");

                            // Exit after receiving the resize event.
                            eprintln!("🔍 PTY Controlled: Resize received, exiting");
                            break;
                        }
                        Some(event) => {
                            // Ignore other events (keyboard input, etc.)
                            eprintln!("🔍 PTY Controlled: Ignoring non-resize event: {event:?}");
                            inactivity_deadline = tokio::time::Instant::now() + inactivity_timeout;
                        }
                        None => {
                            eprintln!("🔍 PTY Controlled: EOF reached");
                            break;
                        }
                    }
                }
                () = tokio::time::sleep_until(inactivity_deadline) => {
                    eprintln!("🔍 PTY Controlled: Inactivity timeout (5 seconds with no SIGWINCH), exiting");
                    break;
                }
            }
        }

        eprintln!("🔍 PTY Controlled: Completed");
    });

    eprintln!("🔍 PTY Controlled: Exiting");
    std::process::exit(0);
}
