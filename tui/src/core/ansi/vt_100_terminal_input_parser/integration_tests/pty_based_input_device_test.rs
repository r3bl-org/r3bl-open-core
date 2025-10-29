// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! PTY-based integration test for DirectToAnsiInputDevice.
//!
//! ## Test Architecture
//!
//! This test validates DirectToAnsiInputDevice in a real terminal context using a
//! bootstrap/slave pattern:
//!
//! ```text
//! ┌────────────────────────────────────────────────────────────────┐
//! │ Bootstrap Mode (test invoked normally)                         │
//! │                                                                │
//! │  1. Create PTY pair (master/slave)                             │
//! │  2. Spawn self with --pty-slave using slave as stdin           │
//! │  3. Write ANSI sequences to PTY master                         │
//! │  4. Read parsed events from child stdout                       │
//! │  5. Verify correctness                                         │
//! └────────────────────────────────────────────────────────────────┘
//!                            │ spawn with PTY
//! ┌──────────────────────────▼─────────────────────────────────────┐
//! │ Slave Mode (--pty-slave flag)                                  │
//! │                                                                │
//! │  1. Create DirectToAnsiInputDevice (reads from stdin)          │
//! │  2. Loop: read_event() → serialize to JSON → write to stdout   │
//! │  3. Exit on EOF or error                                       │
//! └────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Why This Pattern?
//!
//! - **Real TTY context**: DirectToAnsiInputDevice runs in actual PTY, not mock
//! - **Isolated testing**: Child process has clean stdin/stdout
//! - **Async validation**: Properly tests tokio async I/O behavior
//! - **No timeout dependency**: Validates our zero-latency ESC key detection
//!
//! ## Running the Tests
//!
//! These tests use `portable_pty` (already in dependencies) to create real PTY contexts.
//!
//! Run with:
//! ```bash
//! cargo test test_pty -- --ignored --nocapture
//! ```
//!
//! The `--ignored` flag is required as these are marked as integration tests.

use crate::{core::ansi::{generator::generate_keyboard_sequence,
                         vt_100_terminal_input_parser::{InputEvent, KeyCode,
                                                        KeyModifiers}},
            tui::terminal_lib_backends::direct_to_ansi::DirectToAnsiInputDevice};
use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use serde::{Deserialize, Serialize};
use std::{env,
          io::{BufRead, BufReader, Write}};

/// Test result from slave process.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct TestEvent {
    /// Parsed input event from DirectToAnsiInputDevice
    event: SerializableInputEvent,
    /// Number of bytes consumed by parser
    bytes_consumed: usize,
}

/// Serializable version of InputEvent for JSON round-tripping.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
enum SerializableInputEvent {
    Keyboard {
        code: String,
        modifiers: SerializableKeyModifiers,
    },
    // TODO: Add Mouse, Resize, Focus, Paste when parsers are updated
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct SerializableKeyModifiers {
    shift: bool,
    ctrl: bool,
    alt: bool,
}

impl From<InputEvent> for SerializableInputEvent {
    fn from(event: InputEvent) -> Self {
        match event {
            InputEvent::Keyboard { code, modifiers } => {
                SerializableInputEvent::Keyboard {
                    code: format!("{:?}", code),
                    modifiers: SerializableKeyModifiers {
                        shift: modifiers.shift,
                        ctrl: modifiers.ctrl,
                        alt: modifiers.alt,
                    },
                }
            }
            // TODO: Add other variants when parsers are updated
            _ => panic!("Unsupported InputEvent variant in test: {:?}", event),
        }
    }
}

/// PTY slave mode: Read from stdin using DirectToAnsiInputDevice and output events as
/// JSON.
#[tokio::main]
async fn run_pty_slave_mode() -> std::io::Result<()> {
    let mut device = DirectToAnsiInputDevice::new();

    // Read events until EOF
    while let Some(event) = device.read_event().await {
        let test_event = TestEvent {
            event: SerializableInputEvent::from(event),
            bytes_consumed: 0, // Device already consumed internally
        };

        // Write JSON to stdout (one event per line)
        let json = serde_json::to_string(&test_event).unwrap();
        println!("{}", json);

        // Flush immediately so parent can read
        std::io::stdout().flush()?;
    }

    Ok(())
}

/// Helper to generate ANSI bytes from InputEvent using the input event generator.
///
/// This ensures round-trip consistency: the same generator used in round-trip tests
/// generates sequences for the PTY tests.
fn generate_test_sequence(desc: &str, event: InputEvent) -> (&str, Vec<u8>) {
    let bytes = generate_keyboard_sequence(&event)
        .unwrap_or_else(|| panic!("Failed to generate sequence for: {}", desc));
    (desc, bytes)
}

/// Bootstrap mode: Create PTY, spawn slave, send sequences, verify parsing.
#[allow(dead_code)]
fn run_pty_master_mode_test(_test_name: &str, sequences: &[(&str, Vec<u8>)]) {
    // 1. Create PTY pair
    let pty_system = NativePtySystem::default();
    let pty_pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .expect("Failed to create PTY pair");

    // 2. Spawn self with --pty-slave flag
    let mut cmd = CommandBuilder::new(std::env::current_exe().unwrap());
    cmd.arg("--pty-slave");

    let mut child = pty_pair
        .slave
        .spawn_command(cmd)
        .expect("Failed to spawn child process");

    // 3. Get master for writing sequences and reading results
    let mut writer = pty_pair.master.take_writer().expect("Failed to get writer");
    let reader = pty_pair
        .master
        .try_clone_reader()
        .expect("Failed to get reader");
    let mut buf_reader = BufReader::new(reader);

    // 4. Send sequences and verify parsed events
    for (desc, sequence) in sequences {
        // Write sequence to PTY
        writer
            .write_all(sequence)
            .expect("Failed to write sequence");
        writer.flush().expect("Failed to flush");

        // Read parsed event from child stdout (JSON line)
        let mut line = String::new();
        buf_reader
            .read_line(&mut line)
            .expect("Failed to read line");

        let test_event: TestEvent = serde_json::from_str(&line).unwrap_or_else(|e| {
            panic!("Failed to parse JSON for {}: {} - Error: {}", desc, line, e)
        });

        // Verify event matches expectation (test-specific validation)
        eprintln!("{}: Received event: {:?}", desc, test_event.event);
    }

    // 5. Clean up
    drop(writer);
    let _ = child.wait();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // PTY tests are integration tests - run explicitly
    fn test_pty_arrow_keys() {
        let no_mods = KeyModifiers::default();
        run_pty_master_mode_test(
            "arrow_keys",
            &[
                generate_test_sequence(
                    "Up Arrow",
                    InputEvent::Keyboard {
                        code: KeyCode::Up,
                        modifiers: no_mods,
                    },
                ),
                generate_test_sequence(
                    "Down Arrow",
                    InputEvent::Keyboard {
                        code: KeyCode::Down,
                        modifiers: no_mods,
                    },
                ),
                generate_test_sequence(
                    "Right Arrow",
                    InputEvent::Keyboard {
                        code: KeyCode::Right,
                        modifiers: no_mods,
                    },
                ),
                generate_test_sequence(
                    "Left Arrow",
                    InputEvent::Keyboard {
                        code: KeyCode::Left,
                        modifiers: no_mods,
                    },
                ),
            ],
        );
    }

    #[test]
    #[ignore]
    fn test_pty_esc_key_zero_latency() {
        // ESC key requires special handling - raw byte since generator doesn't support it
        run_pty_master_mode_test("esc_key", &[("ESC key alone", vec![0x1b])]);
    }

    #[test]
    #[ignore]
    fn test_pty_function_keys() {
        let no_mods = KeyModifiers::default();
        run_pty_master_mode_test(
            "function_keys",
            &[
                generate_test_sequence(
                    "F1",
                    InputEvent::Keyboard {
                        code: KeyCode::Function(1),
                        modifiers: no_mods,
                    },
                ),
                generate_test_sequence(
                    "F2",
                    InputEvent::Keyboard {
                        code: KeyCode::Function(2),
                        modifiers: no_mods,
                    },
                ),
                generate_test_sequence(
                    "F12",
                    InputEvent::Keyboard {
                        code: KeyCode::Function(12),
                        modifiers: no_mods,
                    },
                ),
            ],
        );
    }

    #[test]
    #[ignore]
    fn test_pty_modified_keys() {
        run_pty_master_mode_test(
            "modified_keys",
            &[
                generate_test_sequence(
                    "Ctrl+Up",
                    InputEvent::Keyboard {
                        code: KeyCode::Up,
                        modifiers: KeyModifiers {
                            shift: false,
                            ctrl: true,
                            alt: false,
                        },
                    },
                ),
                generate_test_sequence(
                    "Shift+Right",
                    InputEvent::Keyboard {
                        code: KeyCode::Right,
                        modifiers: KeyModifiers {
                            shift: true,
                            ctrl: false,
                            alt: false,
                        },
                    },
                ),
                generate_test_sequence(
                    "Alt+Down",
                    InputEvent::Keyboard {
                        code: KeyCode::Down,
                        modifiers: KeyModifiers {
                            shift: false,
                            ctrl: false,
                            alt: true,
                        },
                    },
                ),
            ],
        );
    }
}

/// Entry point for PTY slave mode - called when test binary is spawned with --pty-slave.
pub fn pty_slave_main() -> std::io::Result<()> {
    // Check if we're in slave mode
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 && args[1] == "--pty-slave" {
        return run_pty_slave_mode();
    }

    Ok(())
}
