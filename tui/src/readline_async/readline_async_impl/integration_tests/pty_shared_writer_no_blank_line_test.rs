// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`PTY`] integration test: No extra blank line before prompt with [`SharedWriter`].
//!
//! Validates that [`SharedWriter`] output followed by a prompt redraw does not create an
//! unwanted blank line. This ensures [`CHA(1)`] is only emitted when necessary (not
//! redundantly after newline-terminated data).
//!
//! # Expected Behavior
//!
//! The prompt should appear immediately after the last line of output:
//!
//! ```text
//!   Row 0: line 1
//!   Row 1: line 2
//!   Row 2: >           ← prompt immediately follows output
//! ```
//!
//! **Not** with an extra blank line:
//!
//! ```text
//!   Row 0: line 1
//!   Row 1: line 2
//!   Row 2:             ← unwanted blank line
//!   Row 3: >
//! ```
//!
//! The blank line would occur if redundant [`CHA(1)`] sequences were emitted
//! after newline-terminated data, moving the cursor to column 1 on a new line before
//! rendering the prompt.
//!
//! # Test Architecture
//!
//! This test uses a **[`PTY`]-based integration test pattern** with **headless terminal
//! emulation** to verify exact rendered output:
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
//!   │  │                    │     Verifies NO blank    │                       │
//!   │  │ • Reads output     │     line before prompt   │                       │
//!   │  │ • Asserts results  │                          │                       │
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
//!   │  │  │"line 1"   │        │           │        │_and_flush │           │  │
//!   │  │  │"line 2"   │        │LineState  │        │           │           │  │
//!   │  │  └───────────┘        │Control    │        └─────┬─────┘           │  │
//!   │  │                       │Signal     │              │                 │  │
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
//!   │  │                                         │ Inspect rows  │          │  │
//!   │  │                                         │ for blank     │          │  │
//!   │  │                                         │ lines         │          │  │
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
//! A simple [`Write`] implementation that captures raw bytes (including [`ANSI`] escape
//! sequences) for later processing:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │ CaptureOutputBytes                                                          │
//! ├─────────────────────────────────────────────────────────────────────────────┤
//! │                                                                             │
//! │   impl Write:                                                               │
//! │   ┌─────────────┐    ┌─────────────────────────────────────────────────┐    │
//! │   │ write(buf)  │───►│ Vec<u8>: [0x1b, '[', '1', 'G', 'l', 'i', ...]   │    │
//! │   └─────────────┘    └─────────────────────────────────────────────────┘    │
//! │                                                                             │
//! │   take_bytes():      Returns Vec<u8> and clears internal buffer             │
//! │                                                                             │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## [`OffscreenBuffer::apply_ansi_bytes`]
//!
//! Parses [`ANSI`] escape sequences and renders them to a virtual terminal buffer, giving
//! us the **exact visual output** a user would see:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │ apply_ansi_bytes() Data Flow                                                │
//! ├─────────────────────────────────────────────────────────────────────────────┤
//! │                                                                             │
//! │   Raw bytes:  "Hello\x1b[2;1HWorld"                                         │
//! │                      │                                                      │
//! │                      ▼                                                      │
//! │              ┌──────────────┐                                               │
//! │              │  VTE Parser  │  (vte crate)                                  │
//! │              └──────┬───────┘                                               │
//! │                     │                                                       │
//! │                     ▼                                                       │
//! │   ┌─────────────────────────────────────────────────────────────────────┐   │
//! │   │ AnsiToOfsBufPerformer callbacks:                                    │   │
//! │   │   • print('H'), print('e'), print('l'), print('l'), print('o')      │   │
//! │   │   • csi_dispatch([2, 1], 'H') → cursor to row 2, col 1              │   │
//! │   │   • print('W'), print('o'), print('r'), print('l'), print('d')      │   │
//! │   └─────────────────────────────────────────────────────────────────────┘   │
//! │                     │                                                       │
//! │                     ▼                                                       │
//! │   ┌─────────────────────────────────────────────────────────────────────┐   │
//! │   │ OffscreenBuffer (2D grid of PixelChars)                             │   │
//! │   │                                                                     │   │
//! │   │   Col:  0   1   2   3   4                                           │   │
//! │   │       ┌───┬───┬───┬───┬───┐                                         │   │
//! │   │ Row 0 │ H │ e │ l │ l │ o │                                         │   │
//! │   │       ├───┼───┼───┼───┼───┤                                         │   │
//! │   │ Row 1 │ W │ o │ r │ l │ d │  ← cursor moved here by CSI             │   │
//! │   │       └───┴───┴───┴───┴───┘                                         │   │
//! │   └─────────────────────────────────────────────────────────────────────┘   │
//! │                                                                             │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # [`ANSI`] Escape Sequences Involved
//!
//! This test specifically validates behavior around these escape sequences:
//!
//! ```text
//! ┌──────────────┬────────────────────────────────────────────────────────────┐
//! │ Sequence     │ Description                                                │
//! ├──────────────┼────────────────────────────────────────────────────────────┤
//! │ LF (\n)      │ Line Feed - moves cursor DOWN one row (raw mode: NO CR!)   │
//! │ CR (\r)      │ Carriage Return - moves cursor to column 1                 │
//! │ CHA(1)       │ Cursor Horizontal Absolute - ESC[1G - moves to column 1    │
//! │ ESC[1G       │ Same as CHA(1)                                             │
//! └──────────────┴────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Running the Test
//!
//! ```bash
//! cargo test -p r3bl_tui --lib test_pty_shared_writer_no_blank_line -- --nocapture
//! ```
//!
//! [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
//! [`CHA(1)`]: crate::CsiSequence::CursorHorizontalAbsolute
//! [`OffscreenBuffer::apply_ansi_bytes`]: crate::OffscreenBuffer::apply_ansi_bytes
//! [`PTY`]: crate::core::pty
//! [`SharedWriter`]: crate::SharedWriter
use crate::{ControlledChild, LineStateControlSignal, OffscreenBuffer, PtyPair,
            PtyTestMode, SharedWriter, height, read_lines_and_drain,
            readline_async::readline_async_impl::LineState, width};
use std::{io::Write, time::Duration};

generate_pty_test! {
    /// Verifies no extra blank line appears between [`SharedWriter`] output and
    /// the prompt.
    ///
    /// See the [module docs] for test architecture and expected behavior.
    ///
    /// [`SharedWriter`]: crate::SharedWriter
    /// [module docs]: self
    test_fn: test_pty_shared_writer_no_blank_line,
    controller: pty_controller_entry_point,
    controlled: pty_controlled_entry_point,
    mode: PtyTestMode::Cooked,
}

/// [`PTY`] Controller: Verify no blank line between log output and prompt.
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
fn pty_controller_entry_point(pty_pair: PtyPair, mut child: ControlledChild) {
    eprintln!("🚀 PTY Controller: Starting SharedWriter blank line test...");

    let result =
        read_lines_and_drain(pty_pair, &mut child, "CONTROLLED_DONE", |trimmed| {
            // Skip debug lines from the test framework.
            !trimmed.contains("🔍")
                && !trimmed.contains("TEST_RUNNING")
                && !trimmed.contains("CONTROLLED_STARTING")
        });

    assert!(
        result.found_marker,
        "Controlled process never signaled CONTROLLED_DONE"
    );

    let output_lines = &result.lines;

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

    assert!(
        !found_blank_before_prompt,
        "Found extra blank line before prompt! Output: {output_lines:?}"
    );

    eprintln!("✅ PTY Controller: No blank line detected before prompt!");
}

/// Captures raw [`ANSI`] bytes for later processing with
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
/// │ CaptureOutputBytes  │  ← Captures: ESC[1G, "line 1", LF, ESC[1G, "> ", ...
/// └─────────┬───────────┘
///           │ take_bytes()
///           ▼
/// ┌─────────────────────┐
/// │ OffscreenBuffer     │  ← Renders to virtual 2D grid
/// │ .apply_ansi_bytes() │
/// └─────────────────────┘
/// ```
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
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

/// Extracts text content from an [`OffscreenBuffer`] row for verification.
///
/// [`OffscreenBuffer`]: crate::OffscreenBuffer
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

/// [`PTY`] controlled process: simulates [`SharedWriter`] output and checks for blank
/// lines before the prompt via [`OffscreenBuffer::apply_ansi_bytes`].
///
/// See the [module docs] for the full test architecture.
///
/// [`OffscreenBuffer::apply_ansi_bytes`]: crate::OffscreenBuffer::apply_ansi_bytes
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
/// [`SharedWriter`]: crate::SharedWriter
/// [module docs]: self
fn pty_controlled_entry_point() -> ! {
    println!("CONTROLLED_STARTING");
    std::io::stdout().flush().expect("Failed to flush");

    // Create a channel to receive SharedWriter output.
    let (tx, mut rx) = tokio::sync::mpsc::channel::<LineStateControlSignal>(100);

    // Create LineState and SharedWriter.
    let mut line_state = LineState::new("> ".into(), (80, 24));
    let mut shared_writer = SharedWriter::new(tx);

    // Create an ANSI capture buffer to collect output bytes.
    let mut capture_output_bytes = CaptureOutputBytes::new();

    // Render initial prompt.
    line_state
        .render_and_flush(&mut capture_output_bytes)
        .unwrap();

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
                    .print_data_and_flush(data.as_bytes(), &mut capture_output_bytes)
                    .unwrap();
            }
        }
    });

    // Apply the captured ANSI bytes to an OffscreenBuffer to see the actual
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

    // Check for blank lines before the prompt in the rendered output.
    // A blank line before prompt would appear as an empty row followed by a row
    // starting with ">".
    let mut has_blank_before_prompt = false;

    for row in 0..23 {
        let current = get_line_content(&ofs_buf, row, 80);
        let next = get_line_content(&ofs_buf, row + 1, 80);

        if current.is_empty() && next.starts_with('>') {
            has_blank_before_prompt = true;
            println!("BLANK_LINE_DETECTED_AT_ROW_{row}");
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
