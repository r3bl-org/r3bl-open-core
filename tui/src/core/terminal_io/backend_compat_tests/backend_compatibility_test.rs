// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Backend compatibility tests for [`DirectToAnsiInputDevice`] and
//! [`CrosstermInputDevice`].
//!
//! Verifies both backends produce identical [`InputEvent`] for the same ANSI sequences.
//!
//! # Quick Start
//!
//! Run the **main compatibility test** (compares both backends):
//!
//! ```bash
//! cargo test -p r3bl_tui --lib test_backend_compatibility_comparison -- --ignored --nocapture
//! ```
//!
//! # Architecture
//!
//! The comparison test spawns two subprocess tests and compares their output:
//!
//! ```text
//! test_backend_compatibility_comparison (run this one)
//!   ├── spawns: test_pty_backend_direct_to_ansi (impl detail)
//!   ├── spawns: test_pty_backend_crossterm (impl detail)
//!   └── compares EVENT: output from both
//! ```
//!
//! The individual backend tests exist as implementation details—they're invoked
//! as subprocesses by the comparison test. Run them directly only for debugging:
//!
//! ```bash
//! cargo test -p r3bl_tui --lib test_pty_backend_direct_to_ansi -- --nocapture
//! cargo test -p r3bl_tui --lib test_pty_backend_crossterm -- --nocapture
//! ```
//!
//! # Module Structure
//!
//! - [`generate_test_sequences`] - ANSI sequence builders and test data.
//! - [`controller`] - PTY master (controller) logic.
//! - [`controlled`] - PTY slave (controlled) logic for each backend.
//! - [`comparison`] - Main entry point that compares backend outputs.
//! - [`pty_tests`] - PTY test entry points (invoked as subprocesses).
//!
//! [`CrosstermInputDevice`]: crate::CrosstermInputDevice
//! [`DirectToAnsiInputDevice`]: crate::tui::terminal_lib_backends::direct_to_ansi::DirectToAnsiInputDevice
//! [`InputEvent`]: crate::InputEvent

/// Test Sequence Generation.
mod generate_test_sequences {
    use crate::{ANSI_CSI_BRACKET, ANSI_ESC, ANSI_FUNCTION_KEY_TERMINATOR,
                ANSI_PARAM_SEPARATOR, ANSI_SS3_O, ARROW_DOWN_FINAL, ARROW_LEFT_FINAL,
                ARROW_RIGHT_FINAL, ARROW_UP_FINAL, ASCII_DEL, CONTROL_C, CONTROL_ENTER,
                CONTROL_TAB, FUNCTION_F5_CODE, MODIFIER_ALT, MODIFIER_CTRL,
                MODIFIER_CTRL_SHIFT, MODIFIER_SHIFT, SPECIAL_DELETE_CODE,
                SPECIAL_END_FINAL, SPECIAL_HOME_FINAL, SPECIAL_INSERT_CODE,
                SPECIAL_PAGE_DOWN_CODE, SPECIAL_PAGE_UP_CODE, SS3_F1_FINAL,
                SS3_F2_FINAL, SS3_F3_FINAL, SS3_F4_FINAL};

    /// Builds a CSI sequence: `ESC [ <final>`.
    const fn csi(final_byte: u8) -> [u8; 3] { [ANSI_ESC, ANSI_CSI_BRACKET, final_byte] }

    /// Builds a CSI tilde sequence: `ESC [ <code> ~`.
    fn csi_tilde(code: u16) -> Vec<u8> {
        let mut seq = vec![ANSI_ESC, ANSI_CSI_BRACKET];
        seq.extend(code.to_string().as_bytes());
        seq.push(ANSI_FUNCTION_KEY_TERMINATOR);
        seq
    }

    /// Builds an SS3 sequence: `ESC O <final>`.
    const fn ss3(final_byte: u8) -> [u8; 3] { [ANSI_ESC, ANSI_SS3_O, final_byte] }

    /// Builds a CSI sequence with modifier: `ESC [ 1 ; <mod+1> <final>`.
    ///
    /// Modifier encoding: parameter = 1 + modifier_bits.
    /// - Shift = 2 (1 + [`MODIFIER_SHIFT`])
    /// - Alt = 3 (1 + [`MODIFIER_ALT`])
    /// - Ctrl = 5 (1 + [`MODIFIER_CTRL`])
    /// - Ctrl+Shift = 6 (1 + [`MODIFIER_CTRL_SHIFT`])
    fn csi_modified(modifier: u8, final_byte: u8) -> Vec<u8> {
        let param = 1 + modifier;
        vec![
            ANSI_ESC,
            ANSI_CSI_BRACKET,
            b'1',
            ANSI_PARAM_SEPARATOR,
            b'0' + param,
            final_byte,
        ]
    }

    /// All test sequences for backend compatibility testing.
    ///
    /// Returns (description, ANSI bytes) pairs sent to both backends for comparison.
    pub fn all() -> Vec<(&'static str, Vec<u8>)> {
        vec![
            // Arrow keys (CSI sequences): ESC [ A/B/C/D.
            ("Up Arrow", csi(ARROW_UP_FINAL).to_vec()),
            ("Down Arrow", csi(ARROW_DOWN_FINAL).to_vec()),
            ("Right Arrow", csi(ARROW_RIGHT_FINAL).to_vec()),
            ("Left Arrow", csi(ARROW_LEFT_FINAL).to_vec()),
            // Navigation keys: ESC [ H/F for Home/End, ESC [ n ~ for others.
            ("Home", csi(SPECIAL_HOME_FINAL).to_vec()),
            ("End", csi(SPECIAL_END_FINAL).to_vec()),
            ("Page Up", csi_tilde(SPECIAL_PAGE_UP_CODE)),
            ("Page Down", csi_tilde(SPECIAL_PAGE_DOWN_CODE)),
            ("Insert", csi_tilde(SPECIAL_INSERT_CODE)),
            ("Delete", csi_tilde(SPECIAL_DELETE_CODE)),
            // Function keys (SS3 format): ESC O P/Q/R/S.
            ("F1", ss3(SS3_F1_FINAL).to_vec()),
            ("F2", ss3(SS3_F2_FINAL).to_vec()),
            ("F3", ss3(SS3_F3_FINAL).to_vec()),
            ("F4", ss3(SS3_F4_FINAL).to_vec()),
            // Function keys (CSI format): ESC [ 15 ~.
            ("F5", csi_tilde(FUNCTION_F5_CODE)),
            // Control characters (single bytes).
            ("Ctrl+A", vec![1]), // Ctrl+A = 0x01 = 'A' & 0x1F.
            ("Ctrl+C", vec![CONTROL_C]),
            // Special keys.
            ("Enter", vec![CONTROL_ENTER]),
            ("Tab", vec![CONTROL_TAB]),
            ("Backspace", vec![ASCII_DEL]),
            // Arrow keys with modifiers (xterm format): ESC [ 1 ; <mod+1> A.
            ("Shift+Up", csi_modified(MODIFIER_SHIFT, ARROW_UP_FINAL)),
            ("Ctrl+Up", csi_modified(MODIFIER_CTRL, ARROW_UP_FINAL)),
            ("Alt+Up", csi_modified(MODIFIER_ALT, ARROW_UP_FINAL)),
            (
                "Ctrl+Shift+Up",
                csi_modified(MODIFIER_CTRL_SHIFT, ARROW_UP_FINAL),
            ),
        ]
    }
}

/// Controller (PTY Master) Logic.
mod controller {
    use super::generate_test_sequences;
    use crate::{Deadline, PtyPair};
    use std::{io::{BufRead, BufReader, Write},
              time::Duration};

    /// Shared controller logic for both backends.
    ///
    /// 1. Waits for controlled to be ready.
    /// 2. Sends each test sequence.
    /// 3. Reads and prints parsed [`InputEvent`] from controlled.
    ///
    /// [`InputEvent`]: crate::InputEvent
    pub fn run(backend_name: &str, pty_pair: PtyPair) {
        eprintln!("🚀 {backend_name} Controller: Starting...");

        let mut writer = pty_pair
            .controller()
            .take_writer()
            .expect("Failed to get writer");
        let reader = pty_pair
            .controller()
            .try_clone_reader()
            .expect("Failed to get reader");
        let mut buf_reader = BufReader::new(reader);

        wait_for_ready(&mut buf_reader, backend_name);

        for (desc, bytes) in generate_test_sequences::all() {
            eprintln!("📝 {backend_name} Controller: Sending {desc}...");

            writer.write_all(&bytes).expect("Failed to write sequence");
            writer.flush().expect("Failed to flush");

            std::thread::sleep(Duration::from_millis(50));

            let deadline = Deadline::new(Duration::from_secs(2));
            loop {
                if !deadline.has_time_remaining() {
                    eprintln!(
                        "⚠️  {backend_name} Controller: Timeout waiting for {desc}"
                    );
                    break;
                }

                let mut line = String::new();
                match buf_reader.read_line(&mut line) {
                    Ok(0) => break,
                    Ok(_) => {
                        let trimmed = line.trim();
                        if trimmed.starts_with("EVENT:") {
                            eprintln!("✅ {backend_name} {desc}: {trimmed}");
                            break;
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Err(e) => panic!("Read error: {e}"),
                }
            }
        }

        eprintln!("🧹 {backend_name} Controller: Cleaning up...");
        drop(writer);
    }

    /// Wait for controlled process to signal readiness.
    fn wait_for_ready(
        buf_reader: &mut BufReader<impl std::io::Read>,
        backend_name: &str,
    ) {
        let deadline = Deadline::new(Duration::from_secs(5));

        loop {
            assert!(
                deadline.has_time_remaining(),
                "Timeout waiting for controlled to start"
            );

            let mut line = String::new();
            match buf_reader.read_line(&mut line) {
                Ok(0) => panic!("EOF before controlled ready"),
                Ok(_) => {
                    let trimmed = line.trim();
                    eprintln!("  ← {backend_name} Controlled: {trimmed}");
                    if trimmed.contains("CONTROLLED_READY") {
                        eprintln!("  ✓ {backend_name} Controlled is ready");
                        return;
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(e) => panic!("Read error: {e}"),
            }
        }
    }
}

/// Controlled (PTY Slave) Logic.
mod controlled {
    use std::{io::Write, time::Duration};

    /// Setup: signal ready, enable raw mode.
    pub fn setup(backend_name: &str) {
        println!("CONTROLLED_READY");
        std::io::stdout().flush().expect("Failed to flush");

        eprintln!("🔍 {backend_name} Controlled: Enabling raw mode...");
        if let Err(e) = crate::core::ansi::terminal_raw_mode::enable_raw_mode() {
            eprintln!("⚠️  {backend_name} Controlled: Failed to enable raw mode: {e}");
        }
    }

    /// Teardown: disable raw mode, exit.
    pub fn teardown(backend_name: &str) -> ! {
        drop(crate::core::ansi::terminal_raw_mode::disable_raw_mode());
        eprintln!("🔍 {backend_name} Controlled: Exiting");
        std::process::exit(0);
    }

    /// Macro for the async event loop.
    ///
    /// We use a macro instead of a generic function because `CrosstermInputDevice`
    /// is not `Send` (it contains `Pin<Box<dyn Stream>>`), which prevents using
    /// trait bounds for async functions.
    macro_rules! run_event_loop {
        ($backend_name:expr, $device:expr) => {{
            let inactivity_timeout = Duration::from_secs(3);
            let mut deadline = tokio::time::Instant::now() + inactivity_timeout;

            loop {
                tokio::select! {
                    event = $device.next() => {
                        match event {
                            Some(input_event) => {
                                deadline = tokio::time::Instant::now() + inactivity_timeout;
                                println!("EVENT: {input_event:?}");
                                std::io::stdout().flush().expect("Failed to flush");
                            }
                            None => {
                                eprintln!("🔍 {} Controlled: EOF", $backend_name);
                                break;
                            }
                        }
                    }
                    () = tokio::time::sleep_until(deadline) => {
                        eprintln!("🔍 {} Controlled: Inactivity timeout", $backend_name);
                        break;
                    }
                }
            }
        }};
    }

    /// DirectToAnsi controlled process.
    #[cfg(target_os = "linux")]
    pub fn run_direct_to_ansi() -> ! {
        use crate::tui::terminal_lib_backends::direct_to_ansi::DirectToAnsiInputDevice;

        const NAME: &str = "DirectToAnsi";
        setup(NAME);

        let runtime = tokio::runtime::Runtime::new().expect("Failed to create runtime");
        runtime.block_on(async {
            eprintln!("🔍 {NAME} Controlled: Creating device...");
            let mut device = DirectToAnsiInputDevice::new();
            run_event_loop!(NAME, device);
        });

        teardown(NAME);
    }

    /// Crossterm controlled process.
    #[cfg(target_os = "linux")]
    pub fn run_crossterm() -> ! {
        use crate::CrosstermInputDevice;

        const NAME: &str = "Crossterm";
        setup(NAME);

        let runtime = tokio::runtime::Runtime::new().expect("Failed to create runtime");
        runtime.block_on(async {
            eprintln!("🔍 {NAME} Controlled: Creating device...");
            let mut device = CrosstermInputDevice::new_event_stream();
            run_event_loop!(NAME, device);
        });

        teardown(NAME);
    }
}

/// Compatibility Comparison Test.
#[cfg(target_os = "linux")]
mod comparison {
    use super::generate_test_sequences;
    use std::{collections::HashMap, process::Command};

    /// Runs both backend tests and compares their outputs programmatically.
    ///
    /// Spawns two separate test processes (one for each backend), captures their
    /// `EVENT:` output lines, and compares them to identify any differences.
    ///
    /// Run with:
    /// ```bash
    /// cargo test -p r3bl_tui --lib test_backend_compatibility_comparison -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore] // Run manually - requires interactive terminal.
    pub fn test_backend_compatibility_comparison() {
        eprintln!(
            "🔍 Compatibility Test: Running both backends and comparing outputs..."
        );

        eprintln!("\n📝 Running DirectToAnsi backend test...");
        let direct_output = Command::new("cargo")
            .args([
                "test",
                "-p",
                "r3bl_tui",
                "--lib",
                "test_pty_backend_direct_to_ansi",
                "--",
                "--nocapture",
            ])
            .output()
            .expect("Failed to run DirectToAnsi test");

        let direct_stdout = String::from_utf8_lossy(&direct_output.stdout);
        let direct_stderr = String::from_utf8_lossy(&direct_output.stderr);

        eprintln!("\n📝 Running Crossterm backend test...");
        let crossterm_output = Command::new("cargo")
            .args([
                "test",
                "-p",
                "r3bl_tui",
                "--lib",
                "test_pty_backend_crossterm",
                "--",
                "--nocapture",
            ])
            .output()
            .expect("Failed to run Crossterm test");

        let crossterm_stdout = String::from_utf8_lossy(&crossterm_output.stdout);
        let crossterm_stderr = String::from_utf8_lossy(&crossterm_output.stderr);

        let direct_events = parse_event_lines(&direct_stdout, &direct_stderr);
        let crossterm_events = parse_event_lines(&crossterm_stdout, &crossterm_stderr);

        eprintln!("\n📊 Results:");
        eprintln!("  DirectToAnsi events: {}", direct_events.len());
        eprintln!("  Crossterm events: {}", crossterm_events.len());

        let mut differences: Vec<String> = Vec::new();

        for (desc, _) in generate_test_sequences::all() {
            let direct_event = direct_events.get(desc);
            let crossterm_event = crossterm_events.get(desc);

            match (direct_event, crossterm_event) {
                (Some(d), Some(c)) if d == c => {
                    eprintln!("  ✅ {desc}: Match");
                }
                (Some(d), Some(c)) => {
                    eprintln!("  ❌ {desc}: MISMATCH");
                    eprintln!("      DirectToAnsi: {d}");
                    eprintln!("      Crossterm:    {c}");
                    differences.push(format!("{desc}: DirectToAnsi={d}, Crossterm={c}"));
                }
                (Some(d), None) => {
                    eprintln!("  ⚠️  {desc}: Only DirectToAnsi produced event: {d}");
                    differences.push(format!("{desc}: Only DirectToAnsi={d}"));
                }
                (None, Some(c)) => {
                    eprintln!("  ⚠️  {desc}: Only Crossterm produced event: {c}");
                    differences.push(format!("{desc}: Only Crossterm={c}"));
                }
                (None, None) => {
                    eprintln!("  ⚠️  {desc}: Neither backend produced event");
                    differences.push(format!("{desc}: No events from either backend"));
                }
            }
        }

        eprintln!("\n📋 Summary:");
        if differences.is_empty() {
            eprintln!("  ✅ All events match between backends!");
        } else {
            eprintln!("  ❌ Found {} differences:", differences.len());
            for diff in &differences {
                eprintln!("    - {diff}");
            }
        }
    }

    /// Parse `EVENT:` lines from test output.
    fn parse_event_lines(stdout: &str, stderr: &str) -> HashMap<String, String> {
        let mut events = HashMap::new();
        let combined = format!("{stdout}\n{stderr}");

        for line in combined.lines() {
            let trimmed = line.trim();
            if let Some(event_idx) = trimmed.find("EVENT:") {
                let event_str = trimmed[event_idx..].trim().to_string();
                for (desc, _) in generate_test_sequences::all() {
                    if trimmed.contains(desc) && trimmed.contains("EVENT:") {
                        events.insert(desc.to_string(), event_str.clone());
                        break;
                    }
                }
            }
        }

        events
    }
}

/// PTY Test Entry Points (invoked as subprocesses by [`comparison`]).
#[cfg(target_os = "linux")]
mod pty_tests {
    use super::{controlled, controller};
    use crate::{PtyPair, generate_pty_test};

    mod direct_to_ansi {
        use super::*;

        generate_pty_test! {
            /// PTY test for [`DirectToAnsiInputDevice`] backend.
            ///
            /// Run with:
            /// ```bash
            /// cargo test -p r3bl_tui --lib test_pty_backend_direct_to_ansi -- --nocapture
            /// ```
            ///
            /// [`DirectToAnsiInputDevice`]: crate::tui::terminal_lib_backends::direct_to_ansi::DirectToAnsiInputDevice
            test_fn: test_pty_backend_direct_to_ansi,
            controller: controller_fn,
            controlled: controlled_fn
        }

        fn controller_fn(
            pty_pair: PtyPair,
            mut child: Box<dyn portable_pty::Child + Send + Sync>,
        ) {
            controller::run("DirectToAnsi", pty_pair);

            match child.wait() {
                Ok(status) => {
                    eprintln!(
                        "✅ DirectToAnsi Controller: Controlled process exited: {status:?}"
                    )
                }
                Err(e) => panic!("Failed to wait for controlled process: {e}"),
            }

            eprintln!("✅ DirectToAnsi Controller: Test passed!");
        }

        fn controlled_fn() -> ! { controlled::run_direct_to_ansi(); }
    }

    mod crossterm {
        use super::*;

        generate_pty_test! {
            /// PTY test for [`CrosstermInputDevice`] backend.
            ///
            /// Run with:
            /// ```bash
            /// cargo test -p r3bl_tui --lib test_pty_backend_crossterm -- --nocapture
            /// ```
            ///
            /// [`CrosstermInputDevice`]: crate::CrosstermInputDevice
            test_fn: test_pty_backend_crossterm,
            controller: controller_fn,
            controlled: controlled_fn
        }

        fn controller_fn(
            pty_pair: PtyPair,
            mut child: Box<dyn portable_pty::Child + Send + Sync>,
        ) {
            controller::run("Crossterm", pty_pair);

            match child.wait() {
                Ok(status) => {
                    eprintln!(
                        "✅ Crossterm Controller: Controlled process exited: {status:?}"
                    )
                }
                Err(e) => panic!("Failed to wait for controlled process: {e}"),
            }

            eprintln!("✅ Crossterm Controller: Test passed!");
        }

        fn controlled_fn() -> ! { controlled::run_crossterm(); }
    }
}
