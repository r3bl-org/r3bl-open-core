// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Interactive terminal observation test to establish ground truth.
//!
//! This test captures raw bytes from real terminal interactions to establish ground truth
//! about ANSI coordinate systems and actual sequences sent by terminal emulators.
//!
//! # Run Instructions
//!
//! ```bash
//! cargo test observe_terminal -- --ignored --nocapture
//! ```
//!
//! Follow the on-screen prompts to interact with your terminal (click mouse, press keys).
//! The test will capture raw bytes and save findings to a log file.
//!
//! Alternatively, you can also run the following to capture keyboard input only:
//! ```bash
//! cat -v
//! # Now type keys and observe the raw escape sequences printed
//! ```

use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use std::{io::{Result as IoResult, Write},
          time::Duration};
use tokio::io::AsyncReadExt;

// ============================================================================
// Data Types
// ============================================================================

/// Describes a single test case in the observation sequence.
#[derive(Clone)]
#[allow(clippy::struct_field_names)]
struct TestCase {
    prompt_title: &'static str,
    prompt_action: &'static str,
    prompt_detail: &'static str,
}

// ============================================================================
// Main Test
// ============================================================================

#[tokio::test]
#[ignore = "Manual test: cargo test observe_terminal -- --ignored --nocapture"]
async fn observe_terminal() -> IoResult<()> {
    // Skip in CI
    if is_ci::cached() {
        println!("⏭️  Skipped in CI (requires interactive terminal)");
        return Ok(());
    }

    let terminal_name = detect_terminal_name();

    // Enable raw mode for entire test
    enable_raw_mode()?;

    let mut stdout = std::io::stdout();

    // Print header in raw mode with proper cursor positioning
    write_raw_message(
        &mut stdout,
        "\n╔═══════════════════════════════════════════════════════╗",
    )?;
    write_raw_message(
        &mut stdout,
        "║   VT-100 Terminal Input Observation Test              ║",
    )?;
    write_raw_message(
        &mut stdout,
        "║   Phase 1: Establish Ground Truth                     ║",
    )?;
    write_raw_message(
        &mut stdout,
        "╚═══════════════════════════════════════════════════════╝",
    )?;
    write_raw_message(&mut stdout, &format!("\n🖥️  Terminal: {terminal_name}\n"))?;

    // Enable terminal capture mode with diagnostic output
    write_raw_message(&mut stdout, "🔧 Diagnostic Info:")?;
    write_raw_message(
        &mut stdout,
        "   Sending ANSI codes to enable mouse tracking...",
    )?;
    enable_terminal_capture_mode()?;
    write_raw_message(
        &mut stdout,
        "   ✅ All ANSI codes sent (check stderr for details)",
    )?;
    write_raw_message(&mut stdout, "")?;

    // Run capture phase with real-time output
    run_capture_phase_with_output(&mut stdout).await?;

    // Cleanup
    disable_terminal_capture_mode()?;
    disable_raw_mode()?;
    std::thread::sleep(Duration::from_millis(200));
    std::io::stdout().flush()?;

    write!(
        stdout,
        "\n\r╔════════════════════════════════════════════════════════╗\n\r"
    )?;
    write!(
        stdout,
        "║   ✅ Observation Test Complete                         ║\n\r"
    )?;
    write!(
        stdout,
        "╚════════════════════════════════════════════════════════╝\n\r\n\r"
    )?;
    stdout.flush()?;

    Ok(())
}

// ============================================================================
// Output Helpers for Raw Mode
// ============================================================================

/// Write a message in raw mode with proper cursor positioning.
/// Uses \n\r (line feed + carriage return) for correct positioning.
fn write_raw_message(stdout: &mut std::io::Stdout, message: &str) -> IoResult<()> {
    write!(stdout, "{message}\n\r")?;
    stdout.flush()?;
    Ok(())
}

// ============================================================================
// Input Buffer Draining
// ============================================================================

/// Drain any buffered input from stdin to ensure fresh reads.
/// Repeatedly reads with a short timeout until no more data arrives,
/// clearing any queued events from previous tests.
async fn drain_stdin_buffer<R: tokio::io::AsyncRead + Unpin>(
    stdin: &mut R,
    drain_timeout: Duration,
) {
    let mut buffer = vec![0u8; 256];
    loop {
        match tokio::time::timeout(drain_timeout, stdin.read(&mut buffer)).await {
            Ok(Ok(0) | Err(_)) | Err(_) => break,  // EOF, timeout, or read error
            Ok(Ok(_)) => {}                        // Got data, keep draining
        }
    }
}

// ============================================================================
// Capture Phase
// ============================================================================

async fn run_capture_phase_with_output(stdout: &mut std::io::Stdout) -> IoResult<()> {
    let tests = get_test_cases();
    let delay_for_user = Duration::from_secs(3);
    let delay_between_tests = Duration::from_secs(2);
    let drain_timeout = Duration::from_millis(100);

    let mut stdin = tokio::io::stdin();
    let mut buffer = vec![0u8; 256];

    for test in tests {
        // Drain any buffered input from previous test before printing the prompt
        // This ensures each read() gets fresh input for THIS test, not delayed input
        drain_stdin_buffer(&mut stdin, drain_timeout).await;

        // Print test prompt
        write_raw_message(stdout, "╭─────────────────────────────────────────╮")?;
        write_raw_message(stdout, &format!("│ {:<39} │", test.prompt_title))?;
        write_raw_message(stdout, "╰─────────────────────────────────────────╯")?;
        write_raw_message(stdout, test.prompt_action)?;

        if !test.prompt_detail.is_empty() {
            write_raw_message(stdout, &format!("   {}", test.prompt_detail))?;
        }
        write_raw_message(stdout, "   Waiting for input...\n")?;

        // Capture input (fresh from buffer, not delayed)
        let (raw_bytes, timed_out) = tokio::select! {
            result = stdin.read(&mut buffer) => {
                match result {
                    Ok(n) => (buffer[..n].to_vec(), false),
                    Err(_) => (Vec::new(), false),
                }
            }
            () = tokio::time::sleep(delay_for_user) => {
                (Vec::new(), true)
            }
        };

        // Display results immediately
        if timed_out {
            write_raw_message(stdout, "⏱️  TIMEOUT: No input received\n")?;
        } else if !raw_bytes.is_empty() {
            write_raw_message(stdout, &format!("📦 Raw bytes (hex): {raw_bytes:02x?}"))?;
            write_raw_message(
                stdout,
                &format!(
                    "🔤 Escaped string: {:?}",
                    String::from_utf8_lossy(&raw_bytes)
                ),
            )?;

            // Parse and display parsed output
            // Try SGR mouse format first (starts with ESC[<)
            if raw_bytes.starts_with(b"\x1b[<") {
                if let Some(event) = extract_sgr_mouse_event(&raw_bytes) {
                    write_raw_message(
                        stdout,
                        &format!(
                            "🎯 Parsed: {} (code={}) at col={}, row={}",
                            event.description, event.button_code, event.col, event.row
                        ),
                    )?;
                }
            } else if let Some(desc) = extract_keyboard_code(&raw_bytes) {
                write_raw_message(stdout, &format!("⌨️  Parsed: {desc}"))?;
            }
        } else {
            write_raw_message(stdout, "❌ ERROR: No input captured\n")?;
        }

        write_raw_message(stdout, "")?; // blank line
        tokio::time::sleep(delay_between_tests).await;
    }

    Ok(())
}

// ============================================================================
// Test Case Configuration
// ============================================================================

fn get_test_cases() -> Vec<TestCase> {
    vec![
        TestCase {
            prompt_title: "TEST 1: Mouse - Top-Left Corner        ",
            prompt_action: "👆 Click the TOP-LEFT corner of this terminal window",
            prompt_detail: "(Where row 1, column 1 would be)",
        },
        TestCase {
            prompt_title: "TEST 2: Mouse - Middle of Screen       ",
            prompt_action: "👆 Click roughly the MIDDLE of the terminal",
            prompt_detail: "(Around row 12, column 40 on typical terminal)",
        },
        TestCase {
            prompt_title: "TEST 3: Keyboard - Arrow Up             ",
            prompt_action: "⬆️  Press the UP ARROW key",
            prompt_detail: "",
        },
        TestCase {
            prompt_title: "TEST 4: Keyboard - Ctrl+Up              ",
            prompt_action: "⌨️  Press CTRL+UP ARROW together",
            prompt_detail: "",
        },
        TestCase {
            prompt_title: "TEST 5: Mouse - Scroll Wheel Up         ",
            prompt_action: "🖱️  Scroll mouse wheel UP",
            prompt_detail: "",
        },
    ]
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Enable raw mode and mouse tracking for terminal capture.
/// Logs all ANSI codes being sent to stderr for diagnostic purposes.
fn enable_terminal_capture_mode() -> IoResult<()> {
    use std::io::Write as _;

    let mut stdout = std::io::stdout();

    let sequences = [
        ("SGR mouse (1006)", b"\x1b[?1006h"),
        ("X11 mouse (1000)", b"\x1b[?1000h"),
        ("Focus events (1004)", b"\x1b[?1004h"),
        ("Bracketed paste (2004)", b"\x1b[?2004h"),
    ];

    for (name, seq) in &sequences {
        stdout.write_all(*seq)?;
        eprintln!("📤 Sent: {name} = {seq:02x?}");
    }

    stdout.flush()?;
    Ok(())
}

/// Disable mouse tracking and restore terminal to normal mode.
fn disable_terminal_capture_mode() -> IoResult<()> {
    use std::io::Write as _;

    let mut stdout = std::io::stdout();

    // Disable mouse tracking
    stdout.write_all(b"\x1b[?1000l")?;
    stdout.write_all(b"\x1b[?1003l")?;
    stdout.write_all(b"\x1b[?1015l")?;
    stdout.write_all(b"\x1b[?1006l")?;

    // Disable focus events
    stdout.write_all(b"\x1b[?1004l")?;

    // Disable bracketed paste
    stdout.write_all(b"\x1b[?2004l")?;

    stdout.flush()?;
    Ok(())
}

/// Detect which terminal emulator is running by checking environment variables.
fn detect_terminal_name() -> String {
    // Check multiple env vars that different terminals set
    if let Ok(prog) = std::env::var("TERM_PROGRAM") {
        return prog;
    }

    if let Ok(term) = std::env::var("TERM") {
        // Common TERM values
        return match term.as_str() {
            "xterm" | "xterm-256color" => "xterm".to_string(),
            "gnome" | "gnome-256color" => "GNOME Terminal".to_string(),
            "screen" | "screen-256color" => "screen/tmux".to_string(),
            "alacritty" => "Alacritty".to_string(),
            "kitty" => "Kitty".to_string(),
            "linux" => "Linux Console".to_string(),
            other => format!("{other} (from $TERM)"),
        };
    }

    "Unknown Terminal".to_string()
}

/// Describes a parsed SGR mouse event (button/action, coordinates, and human-readable
/// description).
#[derive(Debug)]
struct SgrMouseEvent {
    button_code: u16,
    col: u16,
    row: u16,
    action_char: char,
    description: String,
}

/// Manually parse SGR mouse sequence and provide human-readable descriptions.
/// Handles clicks, drags, releases, and scroll wheel events.
/// Expected format: `ESC[<button;col;row` followed by `M` (press) or `m` (release)
fn extract_sgr_mouse_event(raw: &[u8]) -> Option<SgrMouseEvent> {
    // Expected format: ESC[<Cb;Cx;CyM or ESC[<Cb;Cx;Cym
    if !raw.starts_with(b"\x1b[<") {
        return None;
    }

    // Remove prefix (ESC[< = 3 bytes) and suffix (M/m = 1 byte)
    // Format: ESC [ < button ; col ; row M/m
    //         1   2 3 ^content^               ^last
    let content = std::str::from_utf8(&raw[3..raw.len().saturating_sub(1)]).ok()?;
    let action_char = *raw.last()? as char;

    // Parse semicolon-separated values
    let parts: Vec<&str> = content.split(';').collect();
    if parts.len() < 3 {
        return None;
    }

    let button_code = parts[0].parse::<u16>().ok()?;
    let col = parts[1].parse::<u16>().ok()?;
    let row = parts[2].parse::<u16>().ok()?;

    // Map button codes to human-readable descriptions
    let description = describe_sgr_button_event(button_code, action_char);

    Some(SgrMouseEvent {
        button_code,
        col,
        row,
        action_char,
        description,
    })
}

/// Describe SGR mouse button codes and actions.
/// Button codes: 0-2 = left/middle/right, 64-67 = scroll, etc.
fn describe_sgr_button_event(button_code: u16, action_char: char) -> String {
    let button_name = match button_code {
        0 => "Left Click",
        1 => "Middle Click",
        2 => "Right Click",
        // Scroll wheel events follow XTerm SGR mouse protocol standard:
        // - 64 = Wheel Down (scroll down)
        // - 65 = Wheel Up (scroll up)
        // Note: If you have "Natural Scrolling" enabled in your OS (Ubuntu, macOS, etc),
        // the codes will be inverted (user scrolls up → code 65, user scrolls down → code
        // 64). The raw protocol values below follow the XTerm standard.
        // Check GNOME natural scrolling setting with:
        //   gsettings get org.gnome.desktop.peripherals.mouse natural-scroll
        64 => "Wheel Down",
        65 => "Wheel Up",
        66 => "Scroll Right",
        67 => "Scroll Left",
        // Motion events (when 1003 is enabled - we don't enable it, but just in case)
        32..=34 => {
            let base = match button_code - 32 {
                0 => "Left",
                1 => "Middle",
                2 => "Right",
                _ => "Unknown",
            };
            return format!("{base} (dragging)");
        }
        _ => "Unknown Button",
    };

    // action_char is 'M' for press/scroll, 'm' for release
    let action_desc = match action_char {
        'M' => "press",
        'm' => "release",
        _ => "unknown action",
    };

    // Scroll events only have 'M', not separate press/release
    if (64..=67).contains(&button_code) {
        button_name.to_string()
    } else {
        format!("{button_name} ({action_desc})")
    }
}

/// Legacy function for backward compatibility - returns basic tuple.
/// Recommend using `extract_sgr_mouse_event()` instead for full details.
#[allow(dead_code)]
fn extract_sgr_coordinates(raw: &[u8]) -> Option<(u16, u16, char)> {
    extract_sgr_mouse_event(raw).map(|evt| (evt.col, evt.row, evt.action_char))
}

/// Manually parse keyboard sequence without using the parser.
fn extract_keyboard_code(raw: &[u8]) -> Option<String> {
    // Simple CSI sequence parsing
    let content = std::str::from_utf8(raw).ok()?;

    match content {
        "\x1b[A" => Some("Up Arrow".to_string()),
        "\x1b[B" => Some("Down Arrow".to_string()),
        "\x1b[C" => Some("Right Arrow".to_string()),
        "\x1b[D" => Some("Left Arrow".to_string()),
        "\x1b[H" => Some("Home".to_string()),
        "\x1b[F" => Some("End".to_string()),
        "\x1b[1;5A" => Some("Ctrl+Up".to_string()),
        "\x1b[1;5B" => Some("Ctrl+Down".to_string()),
        "\x1b[1;5C" => Some("Ctrl+Right".to_string()),
        "\x1b[1;5D" => Some("Ctrl+Left".to_string()),
        "\x1b[3~" => Some("Delete".to_string()),
        "\x1b[2~" => Some("Insert".to_string()),
        "\x1b[5~" => Some("PageUp".to_string()),
        "\x1b[6~" => Some("PageDown".to_string()),
        other => {
            // Show hex representation for unknown sequences
            let hex: String = other
                .bytes()
                .map(|b| format!("{b:02x}"))
                .collect::<Vec<_>>()
                .join(" ");
            Some(format!("Unknown (hex: {hex})"))
        }
    }
}
