// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words OPOST

// XMARK: snapshot comparison PTY test

//! Backend compatibility tests for output backends.
//!
//! Verifies [`RenderOpPaintImplDirectToAnsi`] and [`PaintRenderOpImplCrossterm`] produce
//! visually identical terminal output for the same [`RenderOpOutput`] sequences. This is
//! a **snapshot test** that compares whether both backends produce the same rendered
//! output for the same given terminal size and capabilities in a real PTY environment.
//! Manually sets raw mode for each backend directly (not using the production
//! [`terminal_raw_mode::enable_raw_mode()`] dispatcher which selects based on
//! [`TERMINAL_LIB_BACKEND`]).
//!
//! # Platform
//!
//! **Linux only.** These tests are gated by `#[cfg(all(any(test, doc), target_os =
//! "linux"))]` because [`DirectToAnsi`] is currently Linux-only. The raw mode
//! implementations used are:
//!
//! | Backend          | Raw Mode Implementation                                                |
//! | ---------------- | ---------------------------------------------------------------------- |
//! | [`DirectToAnsi`] | [`terminal_raw_mode::raw_mode_unix::enable_raw_mode()`] (rustix-based) |
//! | [`Crossterm`]    | [`crossterm::terminal::enable_raw_mode()`]                             |
//!
//! # Quick Start
//!
//! Run the **main compatibility test** (compares both backends):
//!
//! ```bash
//! cargo test -p r3bl_tui --lib test_backend_compat_output_compare -- --nocapture
//! ```
//!
//! # Architecture
//!
//! The test uses PTY-based process isolation with **real output devices**:
//!
//! Each backend test manually:
//! 1. Sets raw mode for each backend directly (**not** using the production
//!    [`terminal_raw_mode::enable_raw_mode()`] dispatcher which selects based on
//!    [`TERMINAL_LIB_BACKEND`]).
//! 2. Set color capabilities to Truecolor using [`global_color_support::set_override`].
//!
//! ```text
//!           ┌──────────────────────────────────────────────────────────────┐
//!           │                  SNAPSHOT COMPARISON TEST                    │
//!           │ Raw ANSI byte sequences may DIFFER between backends, but     │
//!           │ resulting terminal STATE (OffscreenBuffer) must be IDENTICAL │
//!           │                                                              │
//!           │   ┌─────────────────┐         ┌─────────────────┐            │
//!           │   │ OffscreenBuffer │   ==    │ OffscreenBuffer │            │
//!           │   │    (direct)     │─────────│   (crossterm)   │            │
//!           │   └────────▲────────┘         └───▲─────────────┘            │
//!           │            │                      │                          │
//!           │     apply_ansi_bytes()          apply_ansi_bytes()           │
//!           │            │                      │                          │
//!           └────────────┼──────────────────────┼──────────────────────────┘
//!                        │                      │
//!  ┌─────────────────────┼─────────────┐   ┌────┼──────────────────────────────┐
//!  │         PTY PAIR #1 │             │   │    │  PTY PAIR #2                 │
//!  │ ┌───────────────────┴───────────┐ │   │ ┌──┴────────────────────────────┐ │
//!  │ │ CONTROLLER                    │ │   │ │ CONTROLLER                    │ │
//!  │ │ • Wait for "CONTROLLED_READY" │ │   │ │ • Wait for "CONTROLLED_READY" │ │
//!  │ │ • Read raw bytes until signal │ │   │ │ • Read raw bytes until signal │ │
//!  │ │ • Return ANSI bytes           │ │   │ │ • Return ANSI bytes           │ │
//!  │ └───────────────▲───────────────┘ │   │ └───────────────▲───────────────┘ │
//!  │                 │                 │   │                 │                 │
//!  │       ══════════╪═══════════      │   │       ══════════╪═══════════      │
//!  │         PTY Channel (raw)         │   │         PTY Channel (raw)         │
//!  │       ══════════╪═══════════      │   │       ══════════╪═══════════      │
//!  │                 │                 │   │                 │                 │
//!  │ ┌───────────────┴───────────────┐ │   │ ┌───────────────┴───────────────┐ │
//!  │ │ CONTROLLED                    │ │   │ │ CONTROLLED                    │ │
//!  │ │                               │ │   │ │                               │ │
//!  │ │  RenderOps                    │ │   │ │  RenderOps                    │ │
//!  │ │      │                        │ │   │ │      │                        │ │
//!  │ │      ▼                        │ │   │ │      ▼                        │ │
//!  │ │  ┌────────────────────────┐   │ │   │ │  ┌────────────────────────┐   │ │
//!  │ │  │ RenderOpPaintImpl      │   │ │   │ │  │ PaintRenderOpImpl      │   │ │
//!  │ │  │ DirectToAnsi           │   │ │   │ │  │ Crossterm              │   │ │
//!  │ │  └───────────┬────────────┘   │ │   │ │  └───────────┬────────────┘   │ │
//!  │ │              │                │ │   │ │              │                │ │
//!  │ │              ▼                │ │   │ │              ▼                │ │
//!  │ │        OutputDevice           │ │   │ │        OutputDevice           │ │
//!  │ │              │                │ │   │ │              │                │ │
//!  │ │              ▼                │ │   │ │              ▼                │ │
//!  │ │      stdout (controlled fd)   │ │   │ │      stdout (controlled fd)   │ │
//!  │ └───────────────────────────────┘ │   │ └───────────────────────────────┘ │
//!  └───────────────────────────────────┘   └───────────────────────────────────┘
//! ```
//!
//! # Why `OffscreenBuffer` Comparison?
//!
//! The raw ANSI bytes from each backend may differ in encoding (e.g., crossterm might
//! use different CSI parameter formats), but they should produce **identical terminal
//! state**. By applying bytes to [`OffscreenBuffer`]s and comparing those, we test
//! semantic equivalence rather than byte-for-byte equality. This is a rendered output
//! snapshot test that confirms that both backends "look the same" at the end, regardless
//! what ANSI escape code encoded byte sequences they used to get there.
//!
//! # Module Structure
//!
//! - [`test_backend_compat_output_compare`] - Main test that compares backend outputs.
//! - [`generate_test_render_ops`] - Test render operations.
//! - [`controller`] - PTY master logic that captures all bytes.
//! - [`controlled_crossterm`] - Crossterm backend controlled process.
//! - [`controlled_direct_to_ansi`] - DirectToAnsi backend controlled process.
//!
//! [`Crossterm`]: crate::TerminalLibBackend::Crossterm
//! [`DirectToAnsi`]: crate::TerminalLibBackend::DirectToAnsi
//! [`OffscreenBuffer`]: crate::OffscreenBuffer
//! [`PaintRenderOpImplCrossterm`]: crate::tui::terminal_lib_backends::crossterm_backend::PaintRenderOpImplCrossterm
//! [`RenderOpOutput`]: crate::RenderOpOutput
//! [`RenderOpPaintImplDirectToAnsi`]: crate::tui::terminal_lib_backends::direct_to_ansi::RenderOpPaintImplDirectToAnsi
//! [`TERMINAL_LIB_BACKEND`]: crate::tui::terminal_lib_backends::TERMINAL_LIB_BACKEND
//! [`terminal_raw_mode::raw_mode_unix::enable_raw_mode()`]: crate::core::ansi::terminal_raw_mode::raw_mode_unix::enable_raw_mode
//! [`terminal_raw_mode::enable_raw_mode()`]: crate::core::ansi::terminal_raw_mode::enable_raw_mode
//! [`test_backend_compat_output_compare`]: fn@test_backend_compat_output_compare

use crate::{ColorSupport, InlineString, OffscreenBuffer, OutputDevice, PtyPair,
            RenderOpOutput, RenderOpPaint, RenderOpsLocalData, Size, TuiStyle,
            TuiStyleAttribs, col,
            core::ansi::terminal_raw_mode,
            global_color_support, height, lock_output_device_as_mut, pos,
            render_op::RenderOpCommon,
            row, spawn_controlled_in_pty,
            terminal_lib_backends::{crossterm_backend::PaintRenderOpImplCrossterm,
                                    direct_to_ansi::RenderOpPaintImplDirectToAnsi},
            tui_color, tui_style_attrib, width};
use std::io::{Read, Write};

/// Test window size for output compatibility tests.
const TEST_WIDTH: u16 = 80;
const TEST_HEIGHT: u16 = 24;

/// Completion signal sent by controlled process after all ANSI output.
/// Uses null bytes which won't appear in normal ANSI sequences.
const COMPLETION_SIGNAL: &[u8] = b"\x00\x00\x00DONE";

/// Environment variable to indicate controlled process mode.
const PTY_CONTROLLED_ENV_VAR: &str = "R3BL_PTY_OUTPUT_TEST_CONTROLLED";

/// Ready signal sent by controlled process after initialization.
const CONTROLLED_READY: &str = "CONTROLLED_READY";

/// Runs both backend tests and compares their rendered outputs.
///
/// Creates PTY pairs directly (no subprocess indirection), captures raw ANSI
/// output from each backend, applies to [`OffscreenBuffer`]s, and compares.
///
/// Run with:
/// ```bash
/// cargo test -p r3bl_tui --lib test_backend_compat_output_compare -- --nocapture
/// ```
#[test]
pub fn test_backend_compat_output_compare() {
    // Check if we're running as a controlled process.
    if let Ok(backend) = std::env::var(PTY_CONTROLLED_ENV_VAR) {
        match backend.as_str() {
            "direct_to_ansi" => controlled_direct_to_ansi::run(),
            "crossterm" => controlled_crossterm::run(),
            _ => panic!("Unknown backend: {backend}"),
        }
    }

    eprintln!(
        "Output Compatibility Test: Running both backends and comparing OffscreenBuffers..."
    );

    // Run DirectToAnsi backend via PTY.
    eprintln!("\nRunning DirectToAnsi backend...");
    let direct_bytes = controller::run(spawn_controlled_in_pty(
        "direct_to_ansi",
        PTY_CONTROLLED_ENV_VAR,
        "test_backend_compat_output_compare",
        TEST_HEIGHT,
        TEST_WIDTH,
    ));
    eprintln!("  Captured {} bytes", direct_bytes.len());

    // Run Crossterm backend via PTY.
    eprintln!("\nRunning Crossterm backend...");
    let crossterm_bytes = controller::run(spawn_controlled_in_pty(
        "crossterm",
        PTY_CONTROLLED_ENV_VAR,
        "test_backend_compat_output_compare",
        TEST_HEIGHT,
        TEST_WIDTH,
    ));
    eprintln!("  Captured {} bytes", crossterm_bytes.len());

    // Create OffscreenBuffers and apply the captured ANSI bytes.
    let buffer_size = height(TEST_HEIGHT) + width(TEST_WIDTH);

    let mut buffer_direct = OffscreenBuffer::new_empty(buffer_size);
    let mut buffer_crossterm = OffscreenBuffer::new_empty(buffer_size);

    eprintln!("\nApplying bytes to OffscreenBuffers...");
    drop(buffer_direct.apply_ansi_bytes(&direct_bytes));
    drop(buffer_crossterm.apply_ansi_bytes(&crossterm_bytes));

    // Compare the OffscreenBuffers.
    eprintln!("\nComparing OffscreenBuffers...");

    if buffer_direct == buffer_crossterm {
        eprintln!("  OffscreenBuffers are IDENTICAL!");
        eprintln!("  Both backends produce the same terminal state.");
    } else {
        eprintln!("  OffscreenBuffers DIFFER!");

        // Show detailed diff.
        if let Some(diff) = buffer_direct.diff(&buffer_crossterm) {
            eprintln!("  {} positions differ:", diff.len());
            for (i, (pos, pixel_char)) in diff.iter().enumerate().take(10) {
                eprintln!("    [{i}] {pos:?}: {pixel_char:?}");
            }
            if diff.len() > 10 {
                eprintln!("    ... and {} more", diff.len() - 10);
            }
        }

        // Print raw byte comparison for debugging.
        eprintln!("\n  Raw byte comparison (first 200 bytes):");
        eprintln!(
            "    DirectToAnsi: {:?}",
            &direct_bytes[..direct_bytes.len().min(200)]
        );
        eprintln!(
            "    Crossterm:    {:?}",
            &crossterm_bytes[..crossterm_bytes.len().min(200)]
        );
    }

    // Assert equality for test pass/fail.
    assert_eq!(
        buffer_direct, buffer_crossterm,
        "OffscreenBuffers should be identical for both backends"
    );
}

/// Controller (PTY Master) Logic - captures all raw bytes.
mod controller {
    use super::*;

    /// Capture all raw ANSI bytes from the controlled process.
    ///
    /// Reads from PTY until it sees the completion signal, then strips the signal
    /// and returns just the ANSI bytes. The controlled process sends the completion
    /// signal immediately after finishing its output, so blocking reads work.
    pub fn run((backend_name, pty_pair): (&str, PtyPair)) -> Vec<u8> {
        eprintln!("{backend_name} Controller: Starting...");

        let mut reader = pty_pair
            .controller()
            .try_clone_reader()
            .expect("Failed to get reader");

        // Wait for CONTROLLED_READY (line-based, before OPOST is disabled).
        wait_for_ready(&mut reader, backend_name);

        // Read raw bytes until we see the completion signal.
        let mut all_bytes = Vec::new();
        let mut temp = [0u8; 4096];

        loop {
            match reader.read(&mut temp) {
                Ok(0) => panic!("EOF before completion signal"),
                Ok(n) => {
                    all_bytes.extend_from_slice(&temp[..n]);

                    // Check if we received the completion signal.
                    if all_bytes.ends_with(COMPLETION_SIGNAL) {
                        // Strip the completion signal.
                        all_bytes.truncate(all_bytes.len() - COMPLETION_SIGNAL.len());
                        eprintln!(
                            "{backend_name} Controller: Got completion signal ({} ANSI bytes)",
                            all_bytes.len()
                        );
                        break;
                    }
                }
                Err(e) => panic!("Read error: {e}"),
            }
        }

        all_bytes
    }

    /// Wait for controlled process to signal readiness.
    ///
    /// The controlled process sends `CONTROLLED_READY` immediately on startup, so
    /// blocking reads work reliably here. No timeout needed since we control both sides.
    fn wait_for_ready(reader: &mut impl Read, backend_name: &str) {
        let mut buffer = Vec::new();
        let mut temp = [0u8; 256];

        loop {
            match reader.read(&mut temp) {
                Ok(0) => panic!("EOF before controlled ready"),
                Ok(n) => {
                    buffer.extend_from_slice(&temp[..n]);
                    let text = String::from_utf8_lossy(&buffer);

                    if text.contains(CONTROLLED_READY) {
                        eprintln!("  {backend_name} Controlled is ready");
                        return;
                    }
                }
                Err(e) => panic!("Read error: {e}"),
            }
        }
    }
}

/// Crossterm backend controlled process.
mod controlled_crossterm {
    use super::*;

    /// Crossterm controlled process entry point.
    ///
    /// Uses `crossterm::terminal::enable_raw_mode()` explicitly,
    /// which is what crossterm uses internally.
    pub fn run() -> ! {
        // 1. Signal ready (before enabling raw mode so newlines work normally).
        println!("{}", super::CONTROLLED_READY);
        std::io::stdout().flush().expect("Failed to flush");

        // 2. Enable raw mode using Crossterm's raw mode.
        drop(crossterm::terminal::enable_raw_mode());

        // 3. Set color support to Truecolor for consistent output.
        global_color_support::set_override(ColorSupport::Truecolor);

        let window_size = Size::new((width(TEST_WIDTH), height(TEST_HEIGHT)));
        let output_device = OutputDevice::new_stdout();
        let ops = generate_test_render_ops::all();

        // 4. Execute render ops.
        let mut state = RenderOpsLocalData {
            cursor_pos: pos(row(0) + col(0)),
            fg_color: None,
            bg_color: None,
        };
        let mut skip_flush = false;
        let mut painter = PaintRenderOpImplCrossterm;

        for op in &ops {
            painter.paint(
                &mut skip_flush,
                op,
                window_size,
                &mut state,
                lock_output_device_as_mut!(output_device),
                output_device.is_mock,
            );
        }

        // 5. Flush to ensure all bytes are written.
        {
            let mut locked = output_device.lock();
            locked.flush().unwrap();
        }

        // 6. Send completion signal.
        std::io::stdout()
            .write_all(COMPLETION_SIGNAL)
            .expect("Failed to write completion signal");
        std::io::stdout().flush().expect("Failed to flush");

        // 7. Cleanup and exit.
        global_color_support::clear_override();
        std::process::exit(0);
    }
}

/// DirectToAnsi backend controlled process.
mod controlled_direct_to_ansi {
    use super::*;

    /// DirectToAnsi controlled process entry point.
    ///
    /// Uses [`terminal_raw_mode::raw_mode_unix::enable_raw_mode()`] directly (the
    /// rustix-based implementation) to explicitly test the DirectToAnsi backend's raw
    /// mode.
    pub fn run() -> ! {
        // 1. Signal ready (before enabling raw mode so newlines work normally).
        println!("{}", super::CONTROLLED_READY);
        std::io::stdout().flush().expect("Failed to flush");

        // 2. Enable raw mode using DirectToAnsi's raw mode (rustix-based).
        drop(terminal_raw_mode::raw_mode_unix::enable_raw_mode());

        // 3. Set color support to Truecolor for consistent output.
        global_color_support::set_override(crate::ColorSupport::Truecolor);

        let window_size = Size::new((width(TEST_WIDTH), height(TEST_HEIGHT)));
        let output_device = OutputDevice::new_stdout();
        let ops = generate_test_render_ops::all();

        // 4. Execute render ops.
        let mut state = RenderOpsLocalData {
            cursor_pos: pos(row(0) + col(0)),
            fg_color: None,
            bg_color: None,
        };
        let mut skip_flush = false;
        let mut painter = RenderOpPaintImplDirectToAnsi;

        for op in &ops {
            painter.paint(
                &mut skip_flush,
                op,
                window_size,
                &mut state,
                lock_output_device_as_mut!(output_device),
                output_device.is_mock,
            );
        }

        // 5. Flush to ensure all bytes are written.
        {
            let mut locked = output_device.lock();
            locked.flush().unwrap();
        }

        // 6. Send completion signal.
        std::io::stdout()
            .write_all(COMPLETION_SIGNAL)
            .expect("Failed to write completion signal");
        std::io::stdout().flush().expect("Failed to flush");

        // 7. Cleanup and exit.
        global_color_support::clear_override();
        std::process::exit(0);
    }
}

/// Test Render Operations Generation.
mod generate_test_render_ops {
    use super::*;

    /// All test render operation sequences for backend compatibility testing.
    ///
    /// Returns render ops that will be executed by both backends for comparison.
    /// The resulting terminal state (`OffscreenBuffer`) should be identical.
    #[allow(clippy::too_many_lines, clippy::vec_init_then_push)]
    pub fn all() -> Vec<RenderOpOutput> {
        let mut ops = Vec::new();

        // Start with a clean slate.
        ops.push(RenderOpOutput::Common(RenderOpCommon::ClearScreen));
        ops.push(RenderOpOutput::Common(
            RenderOpCommon::MoveCursorPositionAbs(pos(row(0) + col(0))),
        ));

        // Test 1: Plain text at origin.
        ops.push(
            RenderOpOutput::CompositorNoClipTruncPaintTextWithAttributes(
                InlineString::from("Hello, World!"),
                None,
            ),
        );

        // Test 2: Plain text at position.
        ops.push(RenderOpOutput::Common(
            RenderOpCommon::MoveCursorPositionAbs(pos(row(5) + col(10))),
        ));
        ops.push(
            RenderOpOutput::CompositorNoClipTruncPaintTextWithAttributes(
                InlineString::from("Text at (5, 10)"),
                None,
            ),
        );

        // Test 3: Cursor absolute move + text.
        ops.push(RenderOpOutput::Common(
            RenderOpCommon::MoveCursorPositionAbs(pos(row(10) + col(20))),
        ));
        ops.push(
            RenderOpOutput::CompositorNoClipTruncPaintTextWithAttributes(
                InlineString::from("X"),
                None,
            ),
        );

        // Test 4: Foreground color red.
        ops.push(RenderOpOutput::Common(
            RenderOpCommon::MoveCursorPositionAbs(pos(row(1) + col(0))),
        ));
        let red_style = TuiStyle {
            color_fg: Some(tui_color!(red)),
            ..Default::default()
        };
        ops.push(
            RenderOpOutput::CompositorNoClipTruncPaintTextWithAttributes(
                InlineString::from("Red text"),
                Some(red_style),
            ),
        );

        // Test 5: Background color blue.
        ops.push(RenderOpOutput::Common(
            RenderOpCommon::MoveCursorPositionAbs(pos(row(2) + col(0))),
        ));
        let blue_bg_style = TuiStyle {
            color_bg: Some(tui_color!(blue)),
            ..Default::default()
        };
        ops.push(
            RenderOpOutput::CompositorNoClipTruncPaintTextWithAttributes(
                InlineString::from("Blue bg"),
                Some(blue_bg_style),
            ),
        );

        // Test 6: RGB foreground color (orange).
        ops.push(RenderOpOutput::Common(
            RenderOpCommon::MoveCursorPositionAbs(pos(row(3) + col(0))),
        ));
        let orange_style = TuiStyle {
            color_fg: Some(tui_color!(255, 128, 0)),
            ..Default::default()
        };
        ops.push(
            RenderOpOutput::CompositorNoClipTruncPaintTextWithAttributes(
                InlineString::from("Orange RGB"),
                Some(orange_style),
            ),
        );

        // Test 7: Bold text.
        ops.push(RenderOpOutput::Common(
            RenderOpCommon::MoveCursorPositionAbs(pos(row(4) + col(0))),
        ));
        let bold_style = TuiStyle {
            attribs: TuiStyleAttribs::from(tui_style_attrib::Bold),
            ..Default::default()
        };
        ops.push(
            RenderOpOutput::CompositorNoClipTruncPaintTextWithAttributes(
                InlineString::from("Bold"),
                Some(bold_style),
            ),
        );

        // Test 8: Italic text.
        ops.push(RenderOpOutput::Common(
            RenderOpCommon::MoveCursorPositionAbs(pos(row(6) + col(0))),
        ));
        let italic_style = TuiStyle {
            attribs: TuiStyleAttribs::from(tui_style_attrib::Italic),
            ..Default::default()
        };
        ops.push(
            RenderOpOutput::CompositorNoClipTruncPaintTextWithAttributes(
                InlineString::from("Italic"),
                Some(italic_style),
            ),
        );

        // Test 9: Underline text.
        ops.push(RenderOpOutput::Common(
            RenderOpCommon::MoveCursorPositionAbs(pos(row(7) + col(0))),
        ));
        let underline_style = TuiStyle {
            attribs: TuiStyleAttribs::from(tui_style_attrib::Underline),
            ..Default::default()
        };
        ops.push(
            RenderOpOutput::CompositorNoClipTruncPaintTextWithAttributes(
                InlineString::from("Underline"),
                Some(underline_style),
            ),
        );

        // Test 10: Styled colored text (green on black, bold + underline).
        ops.push(RenderOpOutput::Common(
            RenderOpCommon::MoveCursorPositionAbs(pos(row(8) + col(5))),
        ));
        let styled_style = TuiStyle {
            color_fg: Some(tui_color!(green)),
            color_bg: Some(tui_color!(black)),
            attribs: tui_style_attrib::Bold + tui_style_attrib::Underline,
            ..Default::default()
        };
        ops.push(
            RenderOpOutput::CompositorNoClipTruncPaintTextWithAttributes(
                InlineString::from("Styled!"),
                Some(styled_style),
            ),
        );

        // Reset attributes at the end.
        ops.push(RenderOpOutput::Common(RenderOpCommon::ResetColor));

        ops
    }
}
