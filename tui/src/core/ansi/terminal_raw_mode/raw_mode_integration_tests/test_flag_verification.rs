// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words IEXTEN VMIN VTIME OPOST ICRNL INLCR IGNCR IXON ISTRIP

//! [`PTY`]-based integration test for raw mode flag verification.
//!
//! Verifies that [`make_raw()`] sets the correct [`termios`] flags according to the
//! [POSIX terminal API] [`cfmakeraw`] specification. This ensures our implementation
//! matches [`crossterm`] and standard [raw mode] behavior.
//!
//! Checks:
//! - Input modes: [`ICANON`], [`ECHO`], [`ISIG`], [`IEXTEN`] disabled
//! - Output modes: [`OPOST`] disabled
//! - Control modes: [`CS8`] set, 8-bit characters
//! - Special codes: [`VMIN`]=1, [`VTIME`]=0 (byte-by-byte, no timeout)
//!
//! # Run with:
//!
//! ```bash
//! cargo test -p r3bl_tui test_raw_mode_flags -- --nocapture
//! ```
//!
//! [`cfmakeraw`]: https://man7.org/linux/man-pages/man3/cfmakeraw.3.html
//! [`crossterm`]: crossterm
//! [`CS8`]: https://man7.org/linux/man-pages/man3/termios.3.html
//! [`ECHO`]: https://man7.org/linux/man-pages/man3/termios.3.html
//! [`ICANON`]: https://man7.org/linux/man-pages/man3/termios.3.html
//! [`IEXTEN`]: https://man7.org/linux/man-pages/man3/termios.3.html
//! [`ISIG`]: https://man7.org/linux/man-pages/man3/termios.3.html
//! [`make_raw()`]: rustix::termios::Termios::make_raw
//! [`OPOST`]: https://man7.org/linux/man-pages/man3/termios.3.html
//! [`PTY`]: crate::core::pty::pty_engine::pty_pair#what-is-a-pty
//! [`VMIN`]: https://man7.org/linux/man-pages/man3/termios.3.html
//! [`VTIME`]: https://man7.org/linux/man-pages/man3/termios.3.html
//! [POSIX terminal API]: https://man7.org/linux/man-pages/man3/termios.3.html
//! [raw mode]: mod@crate::terminal_raw_mode#raw-mode-vs-cooked-mode

use crate::{BufReadExt, GLYPH_CONTROLLED, GLYPH_CONTROLLER, GLYPH_SUCCESS,
            GLYPH_WAITING, GLYPH_WARNING, MSG_CONTROLLED_STARTING, MSG_FAILED,
            MSG_SUCCESS, PtyTestContext, PtyTestMode, RawModeGuard, VMIN_RAW_MODE,
            VTIME_RAW_MODE, generate_pty_test};
use rustix::termios::{self, ControlModes, InputModes, LocalModes, OutputModes,
                      SpecialCodeIndex};
use std::{io::Write,
          time::{Duration, Instant}};

generate_pty_test! {
    test_fn: test_raw_mode_flags,
    controller: controller,
    controlled: controlled,
    mode: PtyTestMode::Cooked,
}

/// Controller process: verifies that controlled process reports correct flags.
fn controller(context: PtyTestContext) {
    let PtyTestContext {
        pty_pair,
        child,
        mut buf_reader,
        ..
    } = context;

    eprintln!("{GLYPH_CONTROLLER} PTY Controller: Starting flag verification test...");

    eprintln!(
        "{GLYPH_WAITING} PTY Controller: Waiting for controlled process flag checks..."
    );

    let mut controlled_started = false;
    let mut test_passed = false;
    let start_timeout = Instant::now();

    while start_timeout.elapsed() < Duration::from_secs(5) {
        let mut line = String::new();
        match buf_reader.read_line_eio_to_eof(&mut line) {
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

    // Drain PTY and wait for child to prevent macOS PTY buffer deadlock.
    child.drain_and_wait(buf_reader, pty_pair);

    eprintln!("{GLYPH_SUCCESS} PTY Controller: Flag verification test passed!");
}

/// Controlled process: enables raw mode and verifies specific [`termios`] flags. The
/// harness performs [`std::process::exit(0)`] after this function returns.
fn controlled() {
    println!("{MSG_CONTROLLED_STARTING}");
    std::io::stdout().flush().expect("Failed to flush");

    eprintln!("{GLYPH_CONTROLLED} Controlled: Starting flag verification...");

    let stdin = std::io::stdin();

    // Enable raw mode using the guard
    let _guard = match RawModeGuard::new() {
        Ok(guard) => guard,
        Err(err) => {
            eprintln!("{GLYPH_WARNING} Controlled: Failed to enable raw mode: {err}");
            println!("{MSG_FAILED} Could not enable raw mode");
            std::io::stdout().flush().expect("Failed to flush");
            std::process::exit(1);
        }
    };

    eprintln!("{GLYPH_SUCCESS} Controlled: Raw mode enabled, checking flags...");

    // Get terminal settings after enabling raw mode
    let termios = match termios::tcgetattr(&stdin) {
        Ok(termios) => termios,
        Err(err) => {
            eprintln!("{GLYPH_WARNING} Controlled: Failed to get termios: {err}");
            println!("{MSG_FAILED} Could not read termios");
            std::io::stdout().flush().expect("Failed to flush");
            std::process::exit(1);
        }
    };

    // Verify Local Modes (ICANON, ECHO, ISIG, IEXTEN should be OFF)
    if termios.local_modes.contains(LocalModes::ICANON) {
        eprintln!("{GLYPH_WARNING} Controlled: ICANON is still ON (should be OFF)");
        println!("{MSG_FAILED} ICANON not disabled");
        std::io::stdout().flush().expect("Failed to flush");
        std::process::exit(1);
    }
    eprintln!("  {GLYPH_SUCCESS} ICANON is OFF (no line buffering)");

    if termios.local_modes.contains(LocalModes::ECHO) {
        eprintln!("{GLYPH_WARNING} Controlled: ECHO is still ON (should be OFF)");
        println!("{MSG_FAILED} ECHO not disabled");
        std::io::stdout().flush().expect("Failed to flush");
        std::process::exit(1);
    }
    eprintln!("  {GLYPH_SUCCESS} ECHO is OFF (no character echo)");

    if termios.local_modes.contains(LocalModes::ISIG) {
        eprintln!("{GLYPH_WARNING} Controlled: ISIG is still ON (should be OFF)");
        println!("{MSG_FAILED} ISIG not disabled");
        std::io::stdout().flush().expect("Failed to flush");
        std::process::exit(1);
    }
    eprintln!("  {GLYPH_SUCCESS} ISIG is OFF (no signal generation)");

    if termios.local_modes.contains(LocalModes::IEXTEN) {
        eprintln!("{GLYPH_WARNING} Controlled: IEXTEN is still ON (should be OFF)");
        println!("{MSG_FAILED} IEXTEN not disabled");
        std::io::stdout().flush().expect("Failed to flush");
        std::process::exit(1);
    }
    eprintln!("  {GLYPH_SUCCESS} IEXTEN is OFF (no extended processing)");

    // Verify Output Modes (OPOST should be OFF)
    if termios.output_modes.contains(OutputModes::OPOST) {
        eprintln!("{GLYPH_WARNING} Controlled: OPOST is still ON (should be OFF)");
        println!("{MSG_FAILED} OPOST not disabled");
        std::io::stdout().flush().expect("Failed to flush");
        std::process::exit(1);
    }
    eprintln!("  {GLYPH_SUCCESS} OPOST is OFF (no output processing)");

    // Verify Control Modes (CS8 should be set for 8-bit characters)
    if !termios.control_modes.contains(ControlModes::CS8) {
        eprintln!("{GLYPH_WARNING} Controlled: CS8 is not set (should be ON)");
        println!("{MSG_FAILED} CS8 not enabled");
        std::io::stdout().flush().expect("Failed to flush");
        std::process::exit(1);
    }
    eprintln!("  {GLYPH_SUCCESS} CS8 is ON (8-bit characters)");

    // Verify Input Modes (common flags should be OFF)
    let unwanted_input_flags = InputModes::ICRNL
        | InputModes::INLCR
        | InputModes::IGNCR
        | InputModes::IXON
        | InputModes::ISTRIP;

    if termios.input_modes.intersects(unwanted_input_flags) {
        eprintln!("{GLYPH_WARNING} Controlled: Unwanted input modes still set");
        println!("{MSG_FAILED} Input modes not properly disabled");
        std::io::stdout().flush().expect("Failed to flush");
        std::process::exit(1);
    }
    eprintln!("  {GLYPH_SUCCESS} Input processing modes disabled (ICRNL, IXON, etc.)");

    // Verify Special Codes (VMIN=1, VTIME=0 for byte-by-byte reading)
    let vmin = termios.special_codes[SpecialCodeIndex::VMIN];
    let vtime = termios.special_codes[SpecialCodeIndex::VTIME];

    if vmin != VMIN_RAW_MODE {
        eprintln!("{GLYPH_WARNING} Controlled: VMIN={vmin} (expected {VMIN_RAW_MODE})");
        println!("{MSG_FAILED} VMIN not set to {VMIN_RAW_MODE}");
        std::io::stdout().flush().expect("Failed to flush");
        std::process::exit(1);
    }
    eprintln!("  {GLYPH_SUCCESS} VMIN={VMIN_RAW_MODE} (return after 1 byte)");

    if vtime != VTIME_RAW_MODE {
        eprintln!(
            "{GLYPH_WARNING} Controlled: VTIME={vtime} (expected {VTIME_RAW_MODE})"
        );
        println!("{MSG_FAILED} VTIME not set to {VTIME_RAW_MODE}");
        std::io::stdout().flush().expect("Failed to flush");
        std::process::exit(1);
    }
    eprintln!("  {GLYPH_SUCCESS} VTIME={VTIME_RAW_MODE} (no timeout)");

    // All checks passed!
    println!("{MSG_SUCCESS} All termios flags verified");
    std::io::stdout().flush().expect("Failed to flush");

    eprintln!("{GLYPH_CONTROLLED} Controlled: Completed, exiting");
    std::process::exit(0);
}
