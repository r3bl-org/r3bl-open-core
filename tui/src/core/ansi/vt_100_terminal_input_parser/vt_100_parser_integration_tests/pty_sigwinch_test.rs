// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words tcgetwinsize

//! [`PTY`] integration test for [`SIGWINCH`] signal handling.
//!
//! This test validates that [`DirectToAnsiInputDevice`] correctly receives and handles
//! the [`SIGWINCH`] signal when the terminal is resized. Unlike the [`ANSI`] resize
//! sequence test in [`pty_terminal_events_test`], this test triggers a **real
//! [`SIGWINCH`] signal** by calling the [`PTY`]'s `resize()` method.
//!
//! # Signal vs Sequence
//!
//! Terminal resize can be communicated in two ways:
//! - **[`SIGWINCH`] signal**: Sent by the kernel when the [`PTY`] size changes (tested
//!   here)
//! - **[`ANSI`] sequence**: `CSI 8;rows;cols t` sent by the terminal (tested in
//!   [`pty_terminal_events_test`])
//!
//! The [`DirectToAnsiInputDevice`] uses [`tokio::signal::unix::Signal`] to listen for
//! [`SIGWINCH`], then queries the terminal size using `tcgetwinsize()`. This test
//! verifies that entire flow works correctly in a real [`PTY`] environment.
//!
//! # Test Architecture
//!
//! ```text
//! Controller                          Controlled (child process)
//! ──────────                          ─────────────────────────────
//!     │                                       │
//!     │  1. spawn in PTY (24×80)              │
//!     ├──────────────────────────────────────►│
//!     │                                       │ 2. setup DirectToAnsiInputDevice
//!     │                                       │    • tokio::signal::unix::Signal
//!     │                                       │      listens for SIGWINCH
//!     │  3. wait for "CONTROLLED_READY"       │
//!     │◄──────────────────────────────────────┤
//!     │                                       │
//!     │  4. resize PTY to 100×30              │
//!     ├───────────────────────────────────────│──► kernel sends SIGWINCH
//!     │                                       │
//!     │                                       │ 5. tokio::select! wakes up
//!     │                                       │    on sigwinch_receiver.recv()
//!     │                                       │
//!     │                                       │ 6. get_size_rustix() calls
//!     │                                       │    tcgetwinsize(stdout) → (100,30)
//!     │                                       │
//!     │                                       │ 7. Returns InputEvent::Resize(100×30)
//!     │  8. verify "Resize: 100x30"           │
//!     │◄──────────────────────────────────────┤
//!     │                                       │
//! ```
//!
//! # Run with:
//!
//! ```bash
//! cargo test -p r3bl_tui test_pty_sigwinch -- --nocapture
//! ```
//!
//! [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
//! [`DirectToAnsiInputDevice`]: crate::direct_to_ansi::DirectToAnsiInputDevice
//! [`pty_terminal_events_test`]: super::pty_terminal_events_test
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`SIGWINCH`]: https://man7.org/linux/man-pages/man7/signal.7.html

use crate::{MSG_CONTROLLED_READY, MSG_CONTROLLED_STARTING, GLYPH_CONTROLLED, GLYPH_CONTROLLER,
            GLYPH_CONTROLLER_CLEANUP, GLYPH_SUCCESS, GLYPH_WAITING,
            InputEvent, PtyTestContext, PtyTestMode, Size,
            generate_pty_test, height, size,
            tui::terminal_lib_backends::direct_to_ansi::DirectToAnsiInputDevice, width};
use std::{io::Write,
          time::Duration};

generate_pty_test! {
    test_fn: test_pty_sigwinch,
    controller: controller,
    controlled: controlled,
    mode: PtyTestMode::Raw,
}

/// [`PTY`] Controller: Resize the [`PTY`] and verify the controlled process receives
/// [`SIGWINCH`].
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
/// [`SIGWINCH`]: https://man7.org/linux/man-pages/man7/signal.7.html
fn controller(context: PtyTestContext) {
    let PtyTestContext {
        mut pty_pair,
        child,
        mut buf_reader,
        ..
    } = context;

    eprintln!("{GLYPH_CONTROLLER} PTY Controller: Starting SIGWINCH test...");
    eprintln!(
        "{GLYPH_WAITING} PTY Controller: Waiting for controlled process to start..."
    );

    // Wait for controlled to confirm it's running and ready. The controlled process sends
    // TEST_RUNNING, CONTROLLED_STARTING, and CONTROLLED_READY on startup.
    child
        .wait_for_ready(&mut buf_reader, MSG_CONTROLLED_READY)
        .expect("Failed to wait for MSG_CONTROLLED_READY");
    eprintln!("  {GLYPH_SUCCESS} Controlled is ready (input device created)");

    // Wait a bit for the controlled process to set up its signal handler.
    std::thread::sleep(Duration::from_millis(200));

    // Resize the PTY - this sends SIGWINCH to the controlled process.
    let new_size: Size = size(width(100) + height(30));

    eprintln!(
        "📐 PTY Controller: Resizing PTY to {:?}x{:?}...",
        new_size.col_width, new_size.row_height
    );

    pty_pair
        .controller_mut()
        .resize(new_size.into())
        .expect("Failed to resize PTY");

    eprintln!("  {GLYPH_SUCCESS} PTY resized, SIGWINCH should have been sent");

    // Wait for the controlled process to report the resize event.
    // The controlled process handles SIGWINCH and prints the new size immediately.
    child.read_line_state(&mut buf_reader, |line| {
        line.starts_with("Resize:") && line.contains("100") && line.contains("30")
    });

    eprintln!("{GLYPH_CONTROLLER_CLEANUP} PTY Controller: Cleaning up...");

    child.drain_and_wait(buf_reader, pty_pair);

    eprintln!("{GLYPH_SUCCESS} PTY Controller: SIGWINCH test passed!");
}

/// [`PTY`] Controlled: Set up signal handler and wait for [`SIGWINCH`]. The harness
/// performs [`std::process::exit(0)`] after this function returns.
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
/// [`SIGWINCH`]: https://man7.org/linux/man-pages/man7/signal.7.html
fn controlled() {
    // Print to stdout immediately to confirm controlled is starting.
    println!("{MSG_CONTROLLED_STARTING}");
    std::io::stdout().flush().expect("Failed to flush");

    let runtime = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    runtime.block_on(async {
        eprintln!("{GLYPH_CONTROLLED} PTY Controlled: Starting DirectToAnsiInputDevice...");
        let mut input_device = DirectToAnsiInputDevice::new()
            .expect("Failed to initialize DirectToAnsiInputDevice");
        eprintln!("{GLYPH_CONTROLLED} PTY Controlled: Device created, waiting for SIGWINCH...");

        // Signal to controller that we're ready to receive input. MUST be after
        // DirectToAnsiInputDevice::new() so the mio poller thread is already
        // watching stdin before the controller sends any input through the PTY.
        println!("{MSG_CONTROLLED_READY}");
        std::io::stdout().flush().expect("Failed to flush");

        let inactivity_timeout = Duration::from_secs(5);
        let mut inactivity_deadline = tokio::time::Instant::now() + inactivity_timeout;

        loop {
            tokio::select! {
                event_result = input_device.next() => {
                    match event_result {
                        Some(InputEvent::Resize(size)) => {
                            eprintln!("{GLYPH_CONTROLLED} PTY Controlled: Received resize event: {size:?}");

                            // Output in a format the controller can parse.
                            println!("Resize: {}x{}", size.col_width.as_usize(), size.row_height.as_usize());
                            std::io::stdout().flush().expect("Failed to flush stdout");

                            // Exit after receiving the resize event.
                            eprintln!("{GLYPH_CONTROLLED} PTY Controlled: Resize received, exiting");
                            break;
                        }
                        Some(event) => {
                            // Ignore other events (keyboard input, etc.)
                            eprintln!("{GLYPH_CONTROLLED} PTY Controlled: Ignoring non-resize event: {event:?}");
                            inactivity_deadline = tokio::time::Instant::now() + inactivity_timeout;
                        }
                        None => {
                            eprintln!("{GLYPH_CONTROLLED} PTY Controlled: EOF reached");
                            break;
                        }
                    }
                }
                () = tokio::time::sleep_until(inactivity_deadline) => {
                    eprintln!("{GLYPH_CONTROLLED} PTY Controlled: Inactivity timeout (5 seconds with no SIGWINCH), exiting");
                    break;
                }
            }
        }

        eprintln!("{GLYPH_CONTROLLED} PTY Controlled: Completed");
    });
}
