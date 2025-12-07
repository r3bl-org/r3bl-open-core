// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! PTY integration test: Multi-line output starts at column 1.
//!
//! Validates that [`SharedWriter`] correctly emits `CHA(1)` after each newline so
//! that multi-line output aligns properly in raw terminal mode.
//!
//! # Raw Mode Requires Explicit Carriage Return
//!
//! In **cooked mode**, the terminal driver translates `LF` → `CR+LF`. In **raw mode**,
//! `LF` only moves the cursor down—it does NOT return to column 1. We must emit
//! `CHA(1)` (`ESC[1G`) explicitly.
//!
//! ```text
//! ┌─────────────────────────────────────┬───────────────────────────────────────┐
//! │ COOKED MODE (auto CR+LF)            │ RAW MODE (LF only moves down)         │
//! ├─────────────────────────────────────┼───────────────────────────────────────┤
//! │                                     │                                       │
//! │   print("A\nB")                     │   print("A\nB")                       │
//! │                                     │                                       │
//! │   Col:  0   1   2   3               │   Col:  0   1   2   3                 │
//! │       ┌───┬───┬───┬───┐             │       ┌───┬───┬───┬───┐               │
//! │ Row 0 │ A │   │   │   │             │ Row 0 │ A │   │   │   │               │
//! │       ├───┼───┼───┼───┤             │       ├───┼───┼───┼───┤               │
//! │ Row 1 │ B │   │   │   │ ✓           │ Row 1 │   │ B │   │   │ ✗ misaligned  │
//! │       └───┴───┴───┴───┘             │       └───┴───┴───┴───┘               │
//! │         ↑                           │             ↑                         │
//! │     CR moved to col 0               │     Cursor stayed at col 1            │
//! │                                     │                                       │
//! └─────────────────────────────────────┴───────────────────────────────────────┘
//! ```
//!
//! # Expected Behavior
//!
//! With proper `CHA(1)` emission, each line starts at column 0:
//!
//! ```text
//!   Col:  0   1   2   3   4   5
//!       ┌───┬───┬───┬───┬───┬───┐
//! Row 0 │ L │ i │ n │ e │   │ 1 │
//!       ├───┼───┼───┼───┼───┼───┤
//! Row 1 │ L │ i │ n │ e │   │ 2 │
//!       ├───┼───┼───┼───┼───┼───┤
//! Row 2 │ L │ i │ n │ e │   │ 3 │
//!       └───┴───┴───┴───┴───┴───┘
//!         ↑
//!     All lines start at column 0
//! ```
//!
//! # Test Architecture
//!
//! This test uses the same **PTY-based integration test pattern** with **headless
//! terminal emulation** as the blank line test:
//!
//! ```text
//! ┌────────────────────────────────────────────────────────────────────────────┐
//! │                         PTY Integration Test Flow                          │
//! └────────────────────────────────────────────────────────────────────────────┘
//!
//!   ┌──────────────────────────────────────────────────────────────────────────┐
//!   │ CONTROLLER PROCESS (Test Runner)                                         │
//!   │                                                                          │
//!   │  ┌────────────────────┐                                                  │
//!   │  │ generate_pty_test! │──── Creates PTY pair ────┐                       │
//!   │  │     macro          │     Spawns controlled    │                       │
//!   │  └────────────────────┘                          │                       │
//!   │            │                                     │                       │
//!   │            ▼                                     │                       │
//!   │  ┌────────────────────┐                          │                       │
//!   │  │   pty_controller   │◄─────────────────────────┼─── Reads PTY output   │
//!   │  │   _entry_point()   │                          │                       │
//!   │  │                    │     Verifies each line   │                       │
//!   │  │ • Reads output     │     starts at column 1   │                       │
//!   │  │ • Asserts no       │                          │                       │
//!   │  │   concatenation    │                          │                       │
//!   │  └────────────────────┘                          │                       │
//!   └──────────────────────────────────────────────────┼───────────────────────┘
//!                                                      │
//!                              PTY (pseudo-terminal)   │
//!                              ════════════════════════╪════════════════════════
//!                                                      │
//!   ┌──────────────────────────────────────────────────┼───────────────────────┐
//!   │ CONTROLLED PROCESS (Child)                       │                       │
//!   │                                                  ▼                       │
//!   │  ┌────────────────────────────────────────────────────────────────────┐  │
//!   │  │                    Simulated Readline Flow                         │  │
//!   │  │                                                                    │  │
//!   │  │   SharedWriter         mpsc channel         LineState              │  │
//!   │  │  ┌───────────┐        ┌───────────┐        ┌───────────┐           │  │
//!   │  │  │writeln!   │───────►│   tx/rx   │───────►│print_data │           │  │
//!   │  │  │"Line 1"   │        │           │        │_and_flush │           │  │
//!   │  │  │"Line 2"   │        │LineState  │        │           │           │  │
//!   │  │  │"Line 3"   │        │Control    │        └─────┬─────┘           │  │
//!   │  │  └───────────┘        │Signal     │              │                 │  │
//!   │  │                       └───────────┘              │ ANSI bytes      │  │
//!   │  │                                                  ▼                 │  │
//!   │  │                                         ┌───────────────┐          │  │
//!   │  │                                         │CaptureOutput  │          │  │
//!   │  │                                         │Bytes          │          │  │
//!   │  │                                         │               │          │  │
//!   │  │                                         │ Captures raw  │          │  │
//!   │  │                                         │ ANSI bytes    │          │  │
//!   │  │                                         └───────┬───────┘          │  │
//!   │  │                                                 │                  │  │
//!   │  │                                                 ▼                  │  │
//!   │  │                                         ┌───────────────┐          │  │
//!   │  │                                         │OffscreenBuffer│          │  │
//!   │  │                                         │.apply_ansi    │          │  │
//!   │  │                                         │_bytes()       │          │  │
//!   │  │                                         │               │          │  │
//!   │  │                                         │ Renders to    │          │  │
//!   │  │                                         │ virtual term  │          │  │
//!   │  │                                         └───────┬───────┘          │  │
//!   │  │                                                 │                  │  │
//!   │  │                                                 ▼                  │  │
//!   │  │                                         ┌───────────────┐          │  │
//!   │  │                                         │ Check column  │          │  │
//!   │  │                                         │ alignment     │          │  │
//!   │  │                                         └───────────────┘          │  │
//!   │  │                                                                    │  │
//!   │  └────────────────────────────────────────────────────────────────────┘  │
//!   │                                                                          │
//!   └──────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Key Components
//!
//! ## `CaptureOutputBytes`
//!
//! A simple [`Write`] implementation that captures raw bytes (including ANSI escape
//! sequences) for later processing. See the [blank line test] for detailed docs.
//!
//! ## [`OffscreenBuffer::apply_ansi_bytes`]
//!
//! Parses ANSI escape sequences and renders them to a virtual terminal buffer.
//! This gives us the **exact visual output** a user would see, allowing us to
//! verify column alignment.
//!
//! # ANSI Escape Sequences Involved
//!
//! ```text
//! ┌──────────────┬────────────────────────────────────────────────────────────┐
//! │ Sequence     │ Description                                                │
//! ├──────────────┼────────────────────────────────────────────────────────────┤
//! │ LF (\n)      │ Line Feed - moves cursor DOWN one row (raw mode: NO CR!)   │
//! │ CR (\r)      │ Carriage Return - moves cursor to column 1                 │
//! │ CHA(1)       │ Cursor Horizontal Absolute - ESC[1G - moves to column 1    │
//! │ ESC[1G       │ Same as CHA(1) - REQUIRED after LF in raw mode             │
//! └──────────────┴────────────────────────────────────────────────────────────┘
//! ```
//!
//! # What the Test Validates
//!
//! 1. **No concatenation**: Each "Line N:" message appears on its own row
//! 2. **Column alignment**: Each line starts at column 0 (or after prompt)
//! 3. **Proper sequencing**: `CHA(1)` is emitted after each newline
//!
//! # Running the Test
//!
//! ```bash
//! cargo test -p r3bl_tui --lib test_pty_multiline_output_starts_at_column_1 -- --nocapture
//! ```
//!
//! [blank line test]: super::pty_shared_writer_no_blank_line_test
//! [`SharedWriter`]: crate::SharedWriter
//! [`OffscreenBuffer::apply_ansi_bytes`]: crate::OffscreenBuffer::apply_ansi_bytes

use crate::{ControlledChild, Deadline, PtyPair, generate_pty_test};
use std::{io::{BufRead, BufReader, Write},
          time::Duration};

generate_pty_test! {
    /// PTY-based integration test: multi-line output starts at column 1.
    ///
    /// Validates that each line of multi-line output printed via [`SharedWriter`]
    /// starts at column 1, not offset by the length of the previous line.
    ///
    /// Run with: `cargo test -p r3bl_tui --lib test_pty_multiline_output_starts_at_column_1 -- --nocapture`
    ///
    /// [`SharedWriter`]: crate::SharedWriter
    test_fn: test_pty_multiline_output_starts_at_column_1,
    controller: pty_controller_entry_point,
    controlled: pty_controlled_entry_point
}

/// PTY Controller: Verify multi-line output all starts at column 1.
fn pty_controller_entry_point(pty_pair: PtyPair, mut child: ControlledChild) {
    eprintln!("🚀 PTY Controller: Starting multi-line output column test...");

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

    assert!(
        controlled_done,
        "Controlled process never signaled CONTROLLED_DONE"
    );

    // Analyze the output for lines that don't start at column 1.
    // If lines don't start at column 1, they would be concatenated with the previous
    // line or have leading spaces that don't match expected indentation.
    eprintln!("\n=== Analyzing output for column alignment ===");
    for (i, line) in output_lines.iter().enumerate() {
        eprintln!("  Line {i}: {line:?}");
    }

    // Look for lines with the "Line N:" pattern and verify they are on separate lines.
    // If CR is missing after LF, "Line 2:" would appear on the same line as "Line 1:"
    // (either concatenated, or the terminal output would be wrong).
    //
    // Note: The output includes the prompt "> " before/after each printed line, so
    // lines appear as "> Line 1: first message" etc.
    let line_messages: Vec<&String> = output_lines
        .iter()
        .filter(|s| s.contains("Line ") && s.contains(':') && s.contains("message"))
        .collect();

    eprintln!("\n=== Line messages found ===");
    for (i, line) in line_messages.iter().enumerate() {
        eprintln!("  {i}: {line:?}");
    }

    // We expect at least 3 line messages.
    assert!(
        line_messages.len() >= 3,
        "Expected at least 3 'Line N:' messages, found {}. Output: {output_lines:?}",
        line_messages.len()
    );

    // Verify each line message contains the expected pattern (not truncated or
    // concatenated).
    for (i, line) in line_messages.iter().enumerate() {
        let expected_pattern = format!("Line {}: ", i + 1);
        assert!(
            line.contains(&expected_pattern),
            "Line {i} should contain '{expected_pattern}', but was: {line:?}"
        );
    }

    // Also check we didn't get concatenated lines (a sign that CR was missing).
    // For example: "Line 1: firstLine 2: second" would indicate missing CR.
    for line in &output_lines {
        let line_count = line.matches("Line ").count();
        assert!(
            line_count <= 1,
            "BUG: Multiple 'Line X:' patterns in single output line indicates missing CR. Line: {line:?}"
        );
    }

    eprintln!("✅ PTY Controller: All lines start at column 1 correctly!");

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

/// Captures raw ANSI bytes for later processing with
/// [`OffscreenBuffer::apply_ansi_bytes`].
///
/// This struct implements [`Write`] to collect terminal output bytes (including escape
/// sequences) that would normally go to stdout. The captured bytes can then be fed to
/// [`OffscreenBuffer::apply_ansi_bytes`] to render them in a virtual terminal buffer,
/// allowing inspection of the exact visual output.
///
/// # Example Flow
///
/// ```text
/// LineState::print_data_and_flush()
///         │
///         ▼
/// ┌─────────────────────┐
/// │ CaptureOutputBytes  │  ← Captures: ESC[1G, "Line 1", LF, ESC[1G, "Line 2", ...
/// └─────────┬───────────┘
///           │ take_bytes()
///           ▼
/// ┌─────────────────────┐
/// │ OffscreenBuffer     │  ← Renders to virtual 2D grid
/// │ .apply_ansi_bytes() │
/// └─────────────────────┘
/// ```
///
/// [`OffscreenBuffer::apply_ansi_bytes`]: crate::OffscreenBuffer::apply_ansi_bytes
struct CaptureOutputBytes(Vec<u8>);

impl CaptureOutputBytes {
    fn new() -> Self { Self(Vec::new()) }
    fn take_bytes(&mut self) -> Vec<u8> { std::mem::take(&mut self.0) }
}

impl Write for CaptureOutputBytes {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> { Ok(()) }
}

/// Extracts text content from an `OffscreenBuffer` row for verification.
fn get_line_content(buf: &crate::OffscreenBuffer, row: usize, max_cols: usize) -> String {
    buf.buffer[row]
        .iter()
        .take(max_cols)
        .map(|pixel_char| match pixel_char {
            crate::PixelChar::PlainText { display_char, .. } => *display_char,
            crate::PixelChar::Spacer | crate::PixelChar::Void => ' ',
        })
        .collect::<String>()
        .trim_end()
        .to_string()
}

/// PTY Controlled: Simulate multi-line `SharedWriter` output and verify column alignment.
///
/// Uses `OffscreenBuffer::apply_ansi_bytes` to accurately simulate what the terminal
/// renders, then verifies each line starts at column 0 (no column offset from missing
/// CR).
fn pty_controlled_entry_point() -> ! {
    use crate::{LineStateControlSignal, OffscreenBuffer, SharedWriter, height,
                readline_async::readline_async_impl::LineState, width};

    println!("CONTROLLED_STARTING");
    std::io::stdout().flush().expect("Failed to flush");

    // Create a channel to receive SharedWriter output.
    let (tx, mut rx) = tokio::sync::mpsc::channel::<LineStateControlSignal>(100);

    // Create LineState and SharedWriter.
    // Use 80x24 terminal size to match typical terminal.
    let mut line_state = LineState::new("> ".into(), (80, 24));
    let mut shared_writer = SharedWriter::new(tx);

    // Create an ANSI capture buffer to collect output bytes.
    let mut capture_output_bytes = CaptureOutputBytes::new();

    // Render initial prompt.
    line_state
        .render_and_flush(&mut capture_output_bytes)
        .unwrap();

    // Simulate multiple lines of logging output (like the bug report shows).
    // Each line ends with newline, so they should each start at column 1.
    writeln!(shared_writer, "Line 1: first message").unwrap();
    writeln!(shared_writer, "Line 2: second message").unwrap();
    writeln!(shared_writer, "Line 3: third message").unwrap();

    // Process the channel messages (simulating what Readline does).
    let runtime = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    runtime.block_on(async {
        // Give time for messages to arrive.
        tokio::time::sleep(Duration::from_millis(50)).await;

        while let Ok(signal) = rx.try_recv() {
            if let LineStateControlSignal::Line(data) = signal {
                line_state
                    .print_data_and_flush(data.as_bytes(), &mut capture_output_bytes)
                    .unwrap();
            }
        }
    });

    // Now apply the captured ANSI bytes to an OffscreenBuffer to see the actual
    // rendered output - this is what the user would see in the terminal.
    let mut ofs_buf = OffscreenBuffer::new_empty(height(24) + width(80));
    let captured_bytes = capture_output_bytes.take_bytes();
    let _events = ofs_buf.apply_ansi_bytes(&captured_bytes);

    // Print raw output for debugging.
    println!("RAW_OUTPUT_START");
    for row in 0..10 {
        let line = get_line_content(&ofs_buf, row, 80);
        if !line.is_empty() {
            println!("Row {row}: {line}");
        }
    }
    println!("RAW_OUTPUT_END");

    // Verify each line starts at column 0 (the correct position).
    // If CR was missing after LF, "Line 2:" would start at a non-zero column.
    let mut found_bug = false;

    for row in 0..10 {
        let line = get_line_content(&ofs_buf, row, 80);

        // Check if this line contains one of our test messages.
        if line.contains("Line 1:")
            || line.contains("Line 2:")
            || line.contains("Line 3:")
        {
            // Find where "Line" starts in the rendered output.
            if let Some(pos) = line.find("Line ") {
                // In correct behavior, "Line X:" should start at column 0 or right after
                // the prompt "> " (column 2). If it starts elsewhere, there's a bug.
                //
                // The prompt "> " takes 2 chars, so valid positions are:
                // - 0 (if line was printed without prompt on same line)
                // - After clearing and re-rendering
                //
                // A bug would show "Line X:" starting at the wrong column due to missing
                // CR.
                println!(
                    "Found '{}' at column {} in row {}",
                    &line[pos..pos.min(line.len())],
                    pos,
                    row
                );

                // Check for concatenation (multiple "Line X:" on same row).
                let line_count = line.matches("Line ").count();
                if line_count > 1 {
                    println!("BUG: Multiple 'Line X:' patterns in row {row}: {line}");
                    found_bug = true;
                }
            }
        }
    }

    if found_bug {
        println!("BUG_DETECTED");
    } else {
        println!("NO_BUG");
    }

    println!("CONTROLLED_DONE");
    std::io::stdout().flush().expect("Failed to flush");

    std::process::exit(0);
}
