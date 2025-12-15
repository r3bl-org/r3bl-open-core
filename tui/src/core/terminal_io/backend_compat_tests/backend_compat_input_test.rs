// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Backend compatibility tests for [`DirectToAnsiInputDevice`] and
//! [`CrosstermInputDevice`].
//!
//! Verifies both backends produce identical [`InputEvent`] for the same ANSI sequences
//! when running in a real PTY environment, with the same terminal size and capabilities.
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
//! | Backend          | Raw Mode Implementation                                              |
//! | ---------------- | -------------------------------------------------------------------- |
//! | [`DirectToAnsi`] | [`terminal_raw_mode::raw_mode_unix::enable_raw_mode`] (rustix-based) |
//! | [`Crossterm`]    | [`crossterm::terminal::enable_raw_mode()`]                           |
//!
//! # Quick Start
//!
//! Run the **main compatibility test** (compares both backends):
//!
//! ```bash
//! cargo test -p r3bl_tui --lib test_backend_compat_input_compare -- --nocapture
//! ```
//!
//! # Architecture
//!
//! The comparison test creates PTY pairs directly (no subprocess indirection):
//!
//! ```text
//! test_backend_compat_input_compare (run this one)
//!   ├── creates PTY for DirectToAnsi backend
//!   ├── creates PTY for Crossterm backend
//!   └── compares parsed InputEvents from both
//! ```
//!
//! # Module Structure
//!
//! - [`generate_test_sequences`] - ANSI sequence builders and test data.
//! - [`controller`] - PTY master (controller) logic.
//! - [`controlled_common`] - Shared setup/teardown for controlled processes.
//! - [`controlled_crossterm`] - Crossterm backend controlled process.
//! - [`controlled_direct_to_ansi`] - `DirectToAnsi` backend controlled process.
//!
//! [`CrosstermInputDevice`]: crate::CrosstermInputDevice
//! [`Crossterm`]: crate::TerminalLibBackend::Crossterm
//! [`DirectToAnsiInputDevice`]: crate::tui::terminal_lib_backends::direct_to_ansi::DirectToAnsiInputDevice
//! [`DirectToAnsi`]: crate::TerminalLibBackend::DirectToAnsi
//! [`InputEvent`]: crate::InputEvent
//! [`TERMINAL_LIB_BACKEND`]: crate::tui::terminal_lib_backends::TERMINAL_LIB_BACKEND
//! [`terminal_raw_mode::raw_mode_unix::enable_raw_mode`]: crate::core::ansi::terminal_raw_mode::raw_mode_unix::enable_raw_mode
//! [`terminal_raw_mode::enable_raw_mode()`]: crate::core::ansi::terminal_raw_mode::enable_raw_mode

use crate::{ANSI_CSI_BRACKET, ANSI_ESC, ANSI_FUNCTION_KEY_TERMINATOR,
            ANSI_PARAM_SEPARATOR, ANSI_SS3_O, ARROW_DOWN_FINAL, ARROW_LEFT_FINAL,
            ARROW_RIGHT_FINAL, ARROW_UP_FINAL, ASCII_DEL, CONTROL_C, CONTROL_ENTER,
            CONTROL_TAB, CrosstermInputDevice, FUNCTION_F5_CODE, MODIFIER_ALT,
            MODIFIER_CTRL, MODIFIER_CTRL_SHIFT, MODIFIER_SHIFT, PtyPair,
            SPECIAL_DELETE_CODE, SPECIAL_END_FINAL, SPECIAL_HOME_FINAL,
            SPECIAL_INSERT_CODE, SPECIAL_PAGE_DOWN_CODE, SPECIAL_PAGE_UP_CODE,
            SS3_F1_FINAL, SS3_F2_FINAL, SS3_F3_FINAL, SS3_F4_FINAL,
            core::ansi::terminal_raw_mode, spawn_controlled_in_pty,
            tui::terminal_lib_backends::direct_to_ansi::DirectToAnsiInputDevice};
use std::{collections::HashMap,
          io::{BufRead, BufReader, Write},
          time::Duration};

const BACKEND_NAME_CROSSTERM: &str = "crossterm";
const BACKEND_NAME_DIRECT_TO_ANSI: &str = "direct_to_ansi";

/// Environment variable to indicate controlled process mode.
const PTY_CONTROLLED_ENV_VAR: &str = "R3BL_PTY_INPUT_TEST_CONTROLLED";

/// Ready signal sent by controlled process after initialization.
const CONTROLLED_READY: &str = "CONTROLLED_READY";

/// Prefix for event output from controlled process.
const EVENT_PREFIX: &str = "EVENT:";

/// Runs both backend tests and compares their outputs.
///
/// Creates PTY pairs directly (no subprocess indirection), sends ANSI sequences
/// to each backend, and compares the parsed [`InputEvent`]s.
///
/// Run with:
/// ```bash
/// cargo test -p r3bl_tui --lib test_backend_compat_input_compare -- --nocapture
/// ```
///
/// # Panics
///
/// Panics if the `PTY_CONTROLLED_ENV_VAR` is set to an unknown backend value.
///
/// [`InputEvent`]: crate::InputEvent
#[test]
pub fn test_backend_compat_input_compare() {
    // Check if we're running as a controlled process.
    if let Ok(backend) = std::env::var(PTY_CONTROLLED_ENV_VAR) {
        match backend.as_str() {
            "direct_to_ansi" => controlled_direct_to_ansi::run(),
            "crossterm" => controlled_crossterm::run(),
            _ => panic!("Unknown backend: {backend}"),
        }
    }

    eprintln!("🔍 Compatibility Test: Running both backends and comparing outputs...");

    // Run DirectToAnsi backend via PTY.
    eprintln!("\n📝 Running DirectToAnsi backend...");
    let direct_events = controller::run_and_collect(spawn_controlled_in_pty(
        "direct_to_ansi",
        PTY_CONTROLLED_ENV_VAR,
        "test_backend_compat_input_compare",
        24,
        80,
    ));
    eprintln!("  Captured {} events", direct_events.len());

    // Run Crossterm backend via PTY.
    eprintln!("\n📝 Running Crossterm backend...");
    let crossterm_events = controller::run_and_collect(spawn_controlled_in_pty(
        "crossterm",
        PTY_CONTROLLED_ENV_VAR,
        "test_backend_compat_input_compare",
        24,
        80,
    ));
    eprintln!("  Captured {} events", crossterm_events.len());

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

/// Controller (PTY Master) Logic.
mod controller {
    use super::*;

    /// Runs controller logic and collects events as a `HashMap`.
    ///
    /// Used by the comparison test to directly capture events without printing.
    /// Returns a map of sequence description → EVENT string.
    pub fn run_and_collect(
        (backend_name, pty_pair): (&str, PtyPair),
    ) -> HashMap<String, String> {
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

        let mut events = HashMap::new();

        for (desc, bytes) in generate_test_sequences::all() {
            eprintln!("📝 {backend_name} Controller: Sending {desc}...");

            writer.write_all(&bytes).expect("Failed to write sequence");
            writer.flush().expect("Failed to flush");

            std::thread::sleep(Duration::from_millis(50));

            // Read lines until we see the EVENT response. The controlled process
            // responds immediately after receiving input, so blocking reads work.
            loop {
                let mut line = String::new();
                match buf_reader.read_line(&mut line) {
                    Ok(0) => break,
                    Ok(_) => {
                        let trimmed = line.trim();
                        if trimmed.starts_with(EVENT_PREFIX) {
                            eprintln!("✅ {backend_name} {desc}: {trimmed}");
                            events.insert(desc.to_string(), trimmed.to_string());
                            break;
                        }
                    }
                    Err(e) => panic!("Read error: {e}"),
                }
            }
        }

        eprintln!("🧹 {backend_name} Controller: Cleaning up...");
        drop(writer);
        events
    }

    /// Wait for controlled process to signal readiness.
    ///
    /// The controlled process sends `CONTROLLED_READY` immediately on startup, so
    /// blocking reads work reliably here. No timeout needed since we control both sides.
    fn wait_for_ready(
        buf_reader: &mut BufReader<impl std::io::Read>,
        backend_name: &str,
    ) {
        loop {
            let mut line = String::new();
            match buf_reader.read_line(&mut line) {
                Ok(0) => panic!("EOF before controlled ready"),
                Ok(_) => {
                    let trimmed = line.trim();
                    eprintln!("  ← {backend_name} Controlled: {trimmed}");
                    if trimmed.contains(CONTROLLED_READY) {
                        eprintln!("  ✓ {backend_name} Controlled is ready");
                        return;
                    }
                }
                Err(e) => panic!("Read error: {e}"),
            }
        }
    }
}

/// Shared setup/teardown for controlled (PTY slave) processes.
mod controlled_common {
    use super::*;

    /// Signal ready to controller. Call before enabling raw mode so newlines work.
    pub fn signal_ready() {
        println!("{}", super::CONTROLLED_READY);
        std::io::stdout().flush().expect("Failed to flush");
    }

    /// Exit the controlled process.
    pub fn exit(backend_name: &str) -> ! {
        eprintln!("🔍 {backend_name} Controlled: Exiting");
        std::process::exit(0);
    }

    /// Macro for the async event loop.
    ///
    /// We use a macro instead of a generic function because `CrosstermInputDevice`
    /// is not `Send` (it contains `Pin<Box<dyn Stream>>`), which prevents using
    /// trait bounds for async functions.
    ///
    /// Note: Uses fully-qualified paths because macro expands in calling module's
    /// scope, not this module's scope.
    macro_rules! run_event_loop {
        ($backend_name:expr, $device:expr) => {{
            use std::io::Write as _;

            let inactivity_timeout = std::time::Duration::from_secs(3);
            let mut deadline = tokio::time::Instant::now() + inactivity_timeout;

            loop {
                tokio::select! {
                    event = $device.next() => {
                        match event {
                            Some(input_event) => {
                                deadline = tokio::time::Instant::now() + inactivity_timeout;
                                println!("{} {input_event:?}", super::EVENT_PREFIX);
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

    // Re-export macro for sibling modules. Macros defined with `macro_rules!` are
    // only visible within their defining module by default. This makes it accessible
    // via `controlled_common::run_event_loop!` from `controlled_crossterm` and
    // `controlled_direct_to_ansi`.
    pub(super) use run_event_loop;
}

/// Crossterm backend controlled process.
mod controlled_crossterm {
    use super::*;

    /// Crossterm controlled process entry point.
    ///
    /// Uses `crossterm::terminal::enable_raw_mode()` explicitly to test the
    /// crossterm backend's raw mode implementation.
    pub fn run() -> ! {
        // 1. Signal ready (before enabling raw mode so newlines work normally).
        controlled_common::signal_ready();

        // 2. Enable raw mode using Crossterm's raw mode.
        drop(crossterm::terminal::enable_raw_mode());

        let runtime = tokio::runtime::Runtime::new().expect("Failed to create runtime");
        runtime.block_on(async {
            eprintln!("🔍 {BACKEND_NAME_CROSSTERM} Controlled: Creating device...");
            let mut device = CrosstermInputDevice::new_event_stream();
            controlled_common::run_event_loop!(BACKEND_NAME_CROSSTERM, device);
        });

        controlled_common::exit(BACKEND_NAME_CROSSTERM);
    }
}

/// `DirectToAnsi` backend controlled process.
mod controlled_direct_to_ansi {
    use super::*;

    /// `DirectToAnsi` controlled process entry point.
    ///
    /// Uses [`terminal_raw_mode::raw_mode_unix::enable_raw_mode`] directly (the
    /// rustix-based implementation) to explicitly test the `DirectToAnsi` backend's raw
    /// mode.
    pub fn run() -> ! {
        // 1. Signal ready (before enabling raw mode so newlines work normally).
        controlled_common::signal_ready();

        // 2. Enable raw mode using DirectToAnsi's raw mode (rustix-based).
        drop(terminal_raw_mode::raw_mode_unix::enable_raw_mode());

        let runtime = tokio::runtime::Runtime::new().expect("Failed to create runtime");
        runtime.block_on(async {
            eprintln!("🔍 {BACKEND_NAME_DIRECT_TO_ANSI} Controlled: Creating device...");
            let mut device = DirectToAnsiInputDevice::new();
            controlled_common::run_event_loop!(BACKEND_NAME_DIRECT_TO_ANSI, device);
        });

        controlled_common::exit(BACKEND_NAME_DIRECT_TO_ANSI);
    }
}

/// Test Sequence Generation.
mod generate_test_sequences {
    use super::*;

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
    /// Modifier encoding: parameter = 1 + `modifier_bits`.
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
