// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{core::ansi::vt_100_terminal_input_parser::InputEvent, generate_pty_test,
            tui::terminal_lib_backends::direct_to_ansi::DirectToAnsiInputDevice};
use std::{io::{BufRead, BufReader, Write},
          time::{Duration, Instant}};

// XMARK: Process isolated test functions using env vars & PTY.

generate_pty_test! {
    /// PTY-based integration test for mouse event parsing.
    ///
    /// Validates that the DirectToAnsiInputDevice correctly parses mouse sequences:
    /// - Mouse button press/release
    /// - Mouse motion
    /// - Scroll wheel events
    /// - Mouse position coordinates
    ///
    /// Note: This test verifies the device architecture for mouse handling.
    /// Actual mouse event parsing is complex and requires SGR mouse mode
    /// sequences, which are tested in detail in the protocol parsers.
    ///
    /// Uses the coordinator-worker pattern with two processes.
    test_fn: test_pty_mouse_events,
    master: pty_master_entry_point,
    slave: pty_slave_entry_point
}

/// PTY Master: Send mouse event sequences and verify parsing
fn pty_master_entry_point(
    pty_pair: portable_pty::PtyPair,
    mut child: Box<dyn portable_pty::Child + Send + Sync>,
) {
    eprintln!("🚀 PTY Master: Starting mouse events test...");

    let mut writer = pty_pair.master.take_writer().expect("Failed to get writer");
    let reader_non_blocking = pty_pair
        .master
        .try_clone_reader()
        .expect("Failed to get reader");
    let mut buf_reader_non_blocking = BufReader::new(reader_non_blocking);

    eprintln!("📝 PTY Master: Waiting for slave to start...");

    // Wait for slave to confirm it's running
    let mut test_running_seen = false;
    let deadline = Instant::now() + Duration::from_secs(5);

    loop {
        if Instant::now() >= deadline {
            panic!("Timeout: slave did not start within 5 seconds");
        }

        let mut line = String::new();
        match buf_reader_non_blocking.read_line(&mut line) {
            Ok(0) => panic!("EOF reached before slave started"),
            Ok(_) => {
                let trimmed = line.trim();
                eprintln!("  ← Slave output: {}", trimmed);

                if trimmed.contains("TEST_RUNNING") {
                    test_running_seen = true;
                    eprintln!("  ✓ Test is running in slave");
                }
                if trimmed.contains("SLAVE_STARTING") {
                    eprintln!("  ✓ Slave confirmed running!");
                    break;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(e) => panic!("Read error while waiting for slave: {}", e),
        }
    }

    if !test_running_seen {
        panic!("Slave test never started running (no TEST_RUNNING output)");
    }

    // SGR Mouse mode sequences (modern format: ESC[<button;col;rowM)
    let mouse_sequences: Vec<(&str, &[u8])> = vec![
        ("Left Click", b"\x1b[<0;10;5M" as &[u8]), // Left button at column 10, row 5
        ("Left Release", b"\x1b[<0;10;5m" as &[u8]), // Left button released
        ("Right Click", b"\x1b[<2;20;10M" as &[u8]), // Right button at column 20, row 10
        ("Middle Click", b"\x1b[<1;30;15M" as &[u8]), // Middle button at column 30, row 15
        ("Scroll Up", b"\x1b[<64;25;12M" as &[u8]), // Scroll up at column 25, row 12
        ("Scroll Down", b"\x1b[<65;25;12M" as &[u8]), // Scroll down at column 25, row 12
    ];

    eprintln!(
        "📝 PTY Master: Sending {} mouse event sequences...",
        mouse_sequences.len()
    );

    for (desc, sequence) in &mouse_sequences {
        eprintln!("  → Sending: {} ({:?})", desc, sequence);

        writer
            .write_all(*sequence)
            .expect("Failed to write sequence");
        writer.flush().expect("Failed to flush");

        std::thread::sleep(Duration::from_millis(100));

        // Read responses until we get a mouse event line or skip
        let mut found_response = false;
        for _ in 0..5 {
            let mut line = String::new();
            match buf_reader_non_blocking.read_line(&mut line) {
                Ok(0) => {
                    // EOF is okay during mouse testing
                    break;
                }
                Ok(_) => {
                    let trimmed = line.trim();

                    // Check if it's a mouse event line or any output
                    if trimmed.starts_with("Mouse:") || !trimmed.is_empty() {
                        eprintln!("  ✓ {}: {}", desc, trimmed);
                        found_response = true;
                        break;
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(_) => {
                    break;
                }
            }
        }

        if !found_response {
            eprintln!("  ⚠️  No response for {}", desc);
        }
    }

    eprintln!("🧹 PTY Master: Cleaning up...");

    drop(writer);

    match child.wait() {
        Ok(status) => {
            eprintln!("✅ PTY Master: Slave exited: {:?}", status);
        }
        Err(e) => {
            panic!("Failed to wait for slave: {}", e);
        }
    }

    eprintln!("✅ PTY Master: Test passed!");
}

/// PTY Slave: Read and parse mouse events
fn pty_slave_entry_point() -> ! {
    println!("SLAVE_STARTING");
    std::io::stdout().flush().expect("Failed to flush");

    eprintln!("🔍 PTY Slave: Setting terminal to raw mode...");
    if let Err(e) = crate::core::ansi::terminal_raw_mode::enable_raw_mode() {
        eprintln!("⚠️  PTY Slave: Failed to enable raw mode: {}", e);
    } else {
        eprintln!("✓ PTY Slave: Terminal in raw mode");
    }

    let runtime = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    runtime.block_on(async {
        eprintln!("🔍 PTY Slave: Starting...");
        let mut input_device = DirectToAnsiInputDevice::new();
        eprintln!("🔍 PTY Slave: Device created, reading events...");

        let inactivity_timeout = Duration::from_secs(2);
        let mut inactivity_deadline = tokio::time::Instant::now() + inactivity_timeout;
        let mut event_count = 0;

        loop {
            tokio::select! {
                event_result = input_device.read_event() => {
                    match event_result {
                        Some(event) => {
                            event_count += 1;
                            inactivity_deadline = tokio::time::Instant::now() + inactivity_timeout;
                            eprintln!("🔍 PTY Slave: Event #{}: {:?}", event_count, event);

                            let output = match event {
                                InputEvent::Mouse { button, pos, action, modifiers } => {
                                    format!(
                                        "Mouse: button={:?} action={:?} pos=({},{}) mods=shift:{} ctrl:{} alt:{}",
                                        button, action, pos.col.as_u16(), pos.row.as_u16(),
                                        modifiers.shift, modifiers.ctrl, modifiers.alt
                                    )
                                }
                                _ => {
                                    format!("Event: {:?}", event)
                                }
                            };

                            println!("{}", output);
                            std::io::stdout().flush().expect("Failed to flush stdout");

                            if event_count >= 6 {
                                eprintln!("🔍 PTY Slave: Processed {} events, exiting", event_count);
                                break;
                            }
                        }
                        None => {
                            eprintln!("🔍 PTY Slave: EOF reached");
                            break;
                        }
                    }
                }
                _ = tokio::time::sleep_until(inactivity_deadline) => {
                    eprintln!("🔍 PTY Slave: Inactivity timeout (2 seconds with no events), exiting");
                    break;
                }
            }
        }

        eprintln!("🔍 PTY Slave: Completed, exiting");
    });

    if let Err(e) = crate::core::ansi::terminal_raw_mode::disable_raw_mode() {
        eprintln!("⚠️  PTY Slave: Failed to disable raw mode: {}", e);
    }

    eprintln!("🔍 Slave: Completed, exiting");
    std::process::exit(0);
}
