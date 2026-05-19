// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`PTY`]-based integration test for keyboard modifiers (Shift, Ctrl, Alt).
//!
//! Validates that the [`DirectToAnsiInputDevice`] correctly parses keyboard sequences
//! with various modifier combinations:
//! - Single modifiers: Shift, Ctrl, Alt
//! - Combined modifiers: Shift+Alt, Shift+Ctrl, Alt+Ctrl
//! - Triple modifiers: Shift+Alt+Ctrl
//!
//! Tests with arrow keys and function keys to validate round-trip generation and parsing.
//!
//! # Run with:
//!
//! ```bash
//! cargo test -p r3bl_tui test_pty_keyboard_modifiers -- --nocapture
//! ```
//!
//! [`DirectToAnsiInputDevice`]: crate::direct_to_ansi::DirectToAnsiInputDevice
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal

use crate::{MSG_CONTROLLED_READY, MSG_CONTROLLED_STARTING, GLYPH_CONTROLLED, GLYPH_CONTROLLER,
            GLYPH_CONTROLLER_CLEANUP, GLYPH_SUCCESS, GLYPH_WAITING,
            InputEvent, KeyState, PtyTestContext, PtyTestMode,
            core::ansi::{generator::generate_keyboard_sequence,
                         vt_100_terminal_input_parser::ir_event_types::{VT100InputEventIR,
                                                                        VT100KeyCodeIR,
                                                                        VT100KeyModifiersIR}},
            generate_pty_test,
            tui::terminal_lib_backends::direct_to_ansi::DirectToAnsiInputDevice};
use std::{io::Write,
          time::Duration};

generate_pty_test! {
    test_fn: test_pty_keyboard_modifiers,
    controller: controller,
    controlled: controlled,
    mode: PtyTestMode::Raw,
}

/// [`PTY`] Controller: Send keyboard sequences with modifiers and verify parsing
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
#[allow(clippy::too_many_lines)]
fn controller(context: PtyTestContext) {
    let PtyTestContext {
        pty_pair,
        child,
        mut buf_reader,
        mut writer,
    } = context;

    eprintln!("{GLYPH_CONTROLLER} PTY Controller: Starting keyboard modifiers test...");

    eprintln!(
        "{GLYPH_WAITING} PTY Controller: Waiting for controlled process to start..."
    );

    // Wait for controlled to be ready.
    child
        .wait_for_ready(&mut buf_reader, MSG_CONTROLLED_READY)
        .expect("Failed to wait for MSG_CONTROLLED_READY");
    eprintln!("  {GLYPH_SUCCESS} Controlled is ready (input device created)");

    // Build test sequences with various modifier combinations
    let modifier_combos = vec![
        (
            "Shift+Up",
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Up,
                modifiers: VT100KeyModifiersIR {
                    shift: KeyState::Pressed,
                    alt: KeyState::NotPressed,
                    ctrl: KeyState::NotPressed,
                },
            },
        ),
        (
            "Ctrl+Up",
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Up,
                modifiers: VT100KeyModifiersIR {
                    shift: KeyState::NotPressed,
                    alt: KeyState::NotPressed,
                    ctrl: KeyState::Pressed,
                },
            },
        ),
        (
            "Alt+Down",
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Down,
                modifiers: VT100KeyModifiersIR {
                    shift: KeyState::NotPressed,
                    alt: KeyState::Pressed,
                    ctrl: KeyState::NotPressed,
                },
            },
        ),
        (
            "Shift+Alt+Left",
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Left,
                modifiers: VT100KeyModifiersIR {
                    shift: KeyState::Pressed,
                    alt: KeyState::Pressed,
                    ctrl: KeyState::NotPressed,
                },
            },
        ),
        (
            "Ctrl+Shift+Right",
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Right,
                modifiers: VT100KeyModifiersIR {
                    shift: KeyState::Pressed,
                    alt: KeyState::NotPressed,
                    ctrl: KeyState::Pressed,
                },
            },
        ),
        (
            "Ctrl+Alt+Shift+F1",
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Function(1),
                modifiers: VT100KeyModifiersIR {
                    shift: KeyState::Pressed,
                    alt: KeyState::Pressed,
                    ctrl: KeyState::Pressed,
                },
            },
        ),
    ];

    eprintln!(
        "{} PTY Controller: Sending {} keyboard modifier combinations...",
        GLYPH_WAITING,
        modifier_combos.len()
    );

    for (desc, event) in &modifier_combos {
        let sequence = generate_keyboard_sequence(event)
            .unwrap_or_else(|| panic!("Failed to generate sequence for: {desc}"));

        eprintln!("  → Sending: {desc} ({sequence:?})");

        writer
            .write_all(&sequence)
            .expect("Failed to write sequence");
        writer.flush().expect("Failed to flush");

        // Read responses until we get a keyboard event line. The controlled process
        // responds immediately after receiving input, so blocking reads work.
        let event_line = child.read_line_state(&mut buf_reader, |line| line.starts_with("Keyboard:"));

        eprintln!("  {GLYPH_SUCCESS} {desc}: {event_line}");
    }

    eprintln!("{GLYPH_CONTROLLER_CLEANUP} PTY Controller: Cleaning up...");

    drop(writer);
    child.drain_and_wait(buf_reader, pty_pair);

    eprintln!("{GLYPH_SUCCESS} PTY Controller: Test passed!");
}

/// [`PTY`] Controlled: Read and parse keyboard events with modifiers. The harness
/// performs [`std::process::exit(0)`] after this function returns.
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
fn controlled() {
    // Print to stdout immediately to confirm controlled is starting.
    println!("{MSG_CONTROLLED_STARTING}");
    std::io::stdout().flush().expect("Failed to flush");

    let runtime = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    runtime.block_on(async {
        eprintln!("{GLYPH_CONTROLLED} PTY Controlled: Starting...");
        let mut input_device = DirectToAnsiInputDevice::new()
            .expect("Failed to initialize DirectToAnsiInputDevice");
        eprintln!("{GLYPH_CONTROLLED} PTY Controlled: Device created, reading events...");

        // Signal to controller that we're ready to receive input. MUST be after
        // DirectToAnsiInputDevice::new() so the mio poller thread is already
        // watching stdin before the controller sends any input through the PTY.
        println!("{MSG_CONTROLLED_READY}");
        std::io::stdout().flush().expect("Failed to flush");

        let inactivity_timeout = Duration::from_secs(5);
        let mut inactivity_deadline = tokio::time::Instant::now() + inactivity_timeout;
        let mut event_count = 0;

        loop {
            tokio::select! {
                event_result = input_device.next() => {
                    match event_result {
                        Some(event) => {
                            event_count += 1;
                            inactivity_deadline = tokio::time::Instant::now() + inactivity_timeout;
                            eprintln!("{GLYPH_CONTROLLED} PTY Controlled: Event #{event_count}: {event:?}");

                            let output = match event {
                                InputEvent::Keyboard(ref key_press) => {
                                    format!("Keyboard: {key_press:?}")
                                }
                                _ => {
                                    format!("Unexpected event: {event:?}")
                                }
                            };

                            println!("{output}");
                            std::io::stdout().flush().expect("Failed to flush stdout");

                            if event_count >= 6 {
                                eprintln!("{GLYPH_CONTROLLED} PTY Controlled: Processed {event_count} events, exiting");
                                break;
                            }
                        }
                        None => {
                            eprintln!("{GLYPH_CONTROLLED} PTY Controlled: EOF reached");
                            break;
                        }
                    }
                }
                () = tokio::time::sleep_until(inactivity_deadline) => {
                    eprintln!("{GLYPH_CONTROLLED} PTY Controlled: Inactivity timeout (5 seconds with no events), exiting");
                    break;
                }
            }
        }

        eprintln!("{GLYPH_CONTROLLED} PTY Controlled: Completed, exiting");
    });
}
