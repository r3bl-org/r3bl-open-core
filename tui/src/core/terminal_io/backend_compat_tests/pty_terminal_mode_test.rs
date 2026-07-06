// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`PTY`] integration test for [`TerminalModeController`].
//!
//! # Run with:
//!
//! ```bash
//! cargo test -p r3bl_tui test_pty_terminal_mode_controller -- --nocapture
//! ```
//!
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal

use crate::{OutputDevice, PtyTestContext, PtyTestMode,
            TerminalModeController, generate_pty_test};
use crate::ansi_output::{cursor_visibility, terminal_modes};
use std::io::{Read, Write};

generate_pty_test! {
    test_fn: test_pty_terminal_mode_controller,
    controller: controller,
    controlled: controlled,
    mode: PtyTestMode::Raw,
}

fn controller(context: PtyTestContext) {
    let PtyTestContext {
        pty_pair,
        child,
        mut buf_reader,
        ..
    } = context;

    eprintln!("Terminal Mode Controller Test: Starting...");

    child
        .wait_for_ready(&mut buf_reader, "READY\n")
        .expect("Failed to wait for READY");

    let mut output = Vec::new();
    let mut temp = [0u8; 1024];

    loop {
        match buf_reader.read(&mut temp) {
            Ok(0) => break,
            Ok(n) => {
                output.extend_from_slice(&temp[..n]);
                if let Some(pos) =
                    output.windows(b"DONE".len()).position(|w| w == b"DONE")
                {
                    output.truncate(pos);
                    break;
                }
            }
            Err(e) => {
                // EIO indicates child closed the PTY.
                if let Some(5) = e.raw_os_error() {
                    break;
                }
                panic!("Read error: {e}");
            }
        }
    }

    child.drain_and_wait(buf_reader, pty_pair);

    let output_str = String::from_utf8_lossy(&output);
    eprintln!("Captured output:\n{output_str:?}");

    assert!(
        output_str.contains(terminal_modes::enter_alternate_screen()),
        "Missing enter_alternate_screen"
    );
    assert!(
        output_str.contains(terminal_modes::exit_alternate_screen()),
        "Missing exit_alternate_screen"
    );
    assert!(
        output_str.contains(cursor_visibility::hide_cursor()),
        "Missing hide_cursor"
    );
    assert!(
        output_str.contains(cursor_visibility::show_cursor()),
        "Missing show_cursor"
    );

    assert!(
        output_str.contains(terminal_modes::enable_mouse_tracking()),
        "Missing mouse tracking"
    );
    assert!(
        output_str.contains(terminal_modes::disable_mouse_tracking()),
        "Missing mouse tracking disable"
    );

    assert!(
        output_str.contains(terminal_modes::enable_bracketed_paste()),
        "Missing bracketed paste enable"
    );
    assert!(
        output_str.contains(terminal_modes::disable_bracketed_paste()),
        "Missing bracketed paste disable"
    );

    eprintln!("Terminal Mode Controller Test: Passed!");
}

/// The harness performs [`std::process::exit(0)`] after this function returns.
///
/// [`std::process::exit(0)`]: std::process::exit
fn controlled() {
    println!("READY");
    std::io::stdout().flush().unwrap();

    let device = OutputDevice::new_stdout();

    device.enter_alternate_screen().unwrap();
    device.exit_alternate_screen().unwrap();
    device.hide_cursor().unwrap();
    device.show_cursor().unwrap();
    device.enable_mouse_tracking().unwrap();
    device.disable_mouse_tracking().unwrap();
    device.enable_bracketed_paste().unwrap();
    device.disable_bracketed_paste().unwrap();

    std::io::stdout().write_all(b"DONE").unwrap();
    std::io::stdout().flush().unwrap();
}
