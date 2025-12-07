// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! PTY integration test: No extra blank line before prompt with SharedWriter.
//!
//! This test validates that when logging via [`SharedWriter`], there is no extra
//! blank line appearing before the prompt. This was a bug where redundant `CHA(1)`
//! escape sequences after newline-terminated data caused visual artifacts.
//!
//! Run with: `cargo test -p r3bl_tui --lib test_pty_shared_writer_no_blank_line -- --nocapture`
//!
//! [`SharedWriter`]: crate::SharedWriter

use crate::{ControlledChild, Deadline, PtyPair, generate_pty_test};
use std::{io::{BufRead, BufReader, Write},
          time::Duration};

generate_pty_test! {
    /// PTY-based integration test: no extra blank line before prompt.
    ///
    /// Validates that [`SharedWriter`] output followed by prompt doesn't create
    /// an extra blank line between the output and the prompt.
    ///
    /// Run with: `cargo test -p r3bl_tui --lib test_pty_shared_writer_no_blank_line -- --nocapture`
    ///
    /// [`SharedWriter`]: crate::SharedWriter
    test_fn: test_pty_shared_writer_no_blank_line,
    controller: pty_controller_entry_point,
    controlled: pty_controlled_entry_point
}

/// PTY Controller: Verify no blank line between log output and prompt.
fn pty_controller_entry_point(pty_pair: PtyPair, mut child: ControlledChild) {
    eprintln!("🚀 PTY Controller: Starting SharedWriter blank line test...");

    let reader = pty_pair
        .controller()
        .try_clone_reader()
        .expect("Failed to clone reader");

    let mut buf_reader = BufReader::new(reader);
    let deadline = Deadline::default();

    eprintln!("📝 PTY Controller: Waiting for controlled process output...");

    // Collect all output lines until we see CONTROLLED_DONE.
    let mut output_lines: Vec<String> = vec![];
    let mut controlled_done = false;

    loop {
        assert!(
            deadline.has_time_remaining(),
            "Timeout: controlled process did not complete within deadline"
        );

        let mut line = String::new();
        match buf_reader.read_line(&mut line) {
            Ok(0) => {
                eprintln!("📝 PTY Controller: EOF reached");
                break;
            }
            Ok(_) => {
                let trimmed = line.trim();
                eprintln!("  ← Controlled output: {trimmed:?}");

                // Skip debug lines from the test framework.
                if trimmed.contains("🔍")
                    || trimmed.contains("TEST_RUNNING")
                    || trimmed.contains("CONTROLLED_STARTING")
                {
                    continue;
                }

                if trimmed.contains("CONTROLLED_DONE") {
                    controlled_done = true;
                    break;
                }

                output_lines.push(trimmed.to_string());
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(e) => panic!("Read error: {e}"),
        }
    }

    assert!(controlled_done, "Controlled process never signaled CONTROLLED_DONE");

    // Analyze the output for blank lines.
    // The output should be something like:
    //   "line 1"
    //   "line 2"
    //   "> "  (or prompt)
    // NOT:
    //   "line 1"
    //   "line 2"
    //   ""  <- blank line (BUG!)
    //   "> "

    eprintln!("\n=== Analyzing output for blank lines ===");
    for (i, line) in output_lines.iter().enumerate() {
        eprintln!("  Line {i}: {line:?}");
    }

    // Check for blank lines before the prompt.
    let mut found_blank_before_prompt = false;
    for i in 0..output_lines.len().saturating_sub(1) {
        let current = &output_lines[i];
        let next = &output_lines[i + 1];

        // If current line is empty and next line looks like a prompt.
        if current.is_empty() && (next.starts_with('>') || next.starts_with("$ ")) {
            found_blank_before_prompt = true;
            eprintln!("  ⚠️  Found blank line at index {i} before prompt!");
        }
    }

    if found_blank_before_prompt {
        panic!("BUG #442: Found extra blank line before prompt! Output: {output_lines:?}");
    }

    eprintln!("✅ PTY Controller: No blank line detected before prompt!");

    // Wait for child to exit.
    match child.wait() {
        Ok(status) => {
            eprintln!("✅ PTY Controller: Controlled process exited: {status:?}");
        }
        Err(e) => {
            panic!("Failed to wait for controlled process: {e}");
        }
    }
}

/// PTY Controlled: Simulate SharedWriter output and check for blank lines.
fn pty_controlled_entry_point() -> ! {
    use crate::{LineStateControlSignal, SharedWriter,
                readline_async::readline_async_impl::LineState};
    use std::sync::{Arc, Mutex as StdMutex};

    println!("CONTROLLED_STARTING");
    std::io::stdout().flush().expect("Failed to flush");

    // Create a channel to receive SharedWriter output.
    let (tx, mut rx) = tokio::sync::mpsc::channel::<LineStateControlSignal>(100);

    // Create LineState and SharedWriter.
    let mut line_state = LineState::new("> ".into(), (80, 24));
    let mut shared_writer = SharedWriter::new(tx);

    // Create a mock terminal output to capture what would be rendered.
    let mock_output = Arc::new(StdMutex::new(Vec::<u8>::new()));
    let mock_output_clone = Arc::clone(&mock_output);

    // Create a simple Write impl that captures output.
    struct MockTerminal(Arc<StdMutex<Vec<u8>>>);
    impl Write for MockTerminal {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    let mut mock_terminal = MockTerminal(mock_output_clone);

    // Render initial prompt.
    line_state.render_and_flush(&mut mock_terminal).unwrap();

    // Simulate logging output (like the bug report).
    writeln!(shared_writer, "line 1").unwrap();
    writeln!(shared_writer, "line 2").unwrap();

    // Process the channel messages (simulating what Readline does).
    let runtime = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    runtime.block_on(async {
        // Give time for messages to arrive.
        tokio::time::sleep(Duration::from_millis(50)).await;

        while let Ok(signal) = rx.try_recv() {
            if let LineStateControlSignal::Line(data) = signal {
                line_state
                    .print_data_and_flush(data.as_bytes(), &mut mock_terminal)
                    .unwrap();
            }
        }
    });

    // Now analyze the mock terminal output.
    let output = mock_output.lock().unwrap();

    // Strip ANSI escape codes for analysis.
    let stripped = strip_ansi_escapes::strip(&*output);
    let stripped_str = String::from_utf8_lossy(&stripped);

    // Print raw output for debugging.
    println!("RAW_OUTPUT_START");
    for line in stripped_str.lines() {
        println!("{line}");
    }
    println!("RAW_OUTPUT_END");

    // Check for consecutive blank lines or blank line before prompt.
    let lines: Vec<&str> = stripped_str.lines().collect();
    let mut has_blank_before_prompt = false;

    for i in 0..lines.len().saturating_sub(1) {
        if lines[i].is_empty() && lines[i + 1].starts_with('>') {
            has_blank_before_prompt = true;
            println!("BLANK_LINE_DETECTED_AT_{i}");
        }
    }

    if has_blank_before_prompt {
        println!("BUG_DETECTED");
    } else {
        println!("NO_BUG");
    }

    println!("CONTROLLED_DONE");
    std::io::stdout().flush().expect("Failed to flush");

    std::process::exit(0);
}
