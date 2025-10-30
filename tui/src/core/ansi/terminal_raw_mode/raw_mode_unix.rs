// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Unix/Linux/macOS implementation of raw mode using rustix's safe termios API.

use miette::miette;
use rustix::termios::{self, ControlModes, InputModes, LocalModes, OptionalActions,
                      OutputModes, SpecialCodeIndex, Termios};
use std::{io,
          sync::{LazyLock, Mutex}};

/// Stores the original terminal settings to restore later.
/// Using [`std::sync::LazyLock`] (stabilized in Rust 1.80) instead of `once_cell`.
static ORIGINAL_TERMIOS: LazyLock<Mutex<Option<Termios>>> =
    LazyLock::new(|| Mutex::new(None));

/// Enable raw mode on the terminal (Unix/Linux/macOS implementation).
///
/// Uses rustix's type-safe termios API to:
/// 1. Save the original terminal settings for restoration
/// 2. Disable canonical mode, echo, and signal generation
/// 3. Set VMIN=1, VTIME=0 for immediate byte-by-byte reading
///
/// See [`mod@crate::core::ansi::terminal_raw_mode`] for conceptual overview and usage.
///
/// # Errors
///
/// Returns miette diagnostic errors if:
/// - Terminal attributes cannot be retrieved or set
/// - Mutex lock is poisoned
pub fn enable_raw_mode() -> miette::Result<()> {
    let stdin = io::stdin();
    let mut termios = termios::tcgetattr(&stdin)
        .map_err(|e| miette::miette!("failed to retrieve terminal attributes: {e}"))?;

    // Save original settings
    {
        let mut original = ORIGINAL_TERMIOS
            .lock()
            .map_err(|e| miette!("terminal settings lock poisoned: {e}"))?;

        if original.is_none() {
            // rustix's Termios doesn't implement Copy, so we need to clone
            *original = Some(termios.clone());
        }
    }

    // Modify settings for raw mode using rustix's type-safe API
    // Based on cfmakeraw() implementation
    termios.input_modes.remove(
        InputModes::IGNBRK
            | InputModes::BRKINT
            | InputModes::PARMRK
            | InputModes::ISTRIP
            | InputModes::INLCR
            | InputModes::IGNCR
            | InputModes::ICRNL
            | InputModes::IXON,
    );
    termios.output_modes.remove(OutputModes::OPOST);
    termios.local_modes.remove(
        LocalModes::ECHO
            | LocalModes::ECHONL
            | LocalModes::ICANON
            | LocalModes::ISIG
            | LocalModes::IEXTEN,
    );
    termios
        .control_modes
        .remove(ControlModes::CSIZE | ControlModes::PARENB);
    termios.control_modes.insert(ControlModes::CS8);

    // Set minimum bytes and timeout for read
    termios.special_codes[SpecialCodeIndex::VMIN] = 1; // Read at least 1 byte
    termios.special_codes[SpecialCodeIndex::VTIME] = 0; // No timeout

    // Apply the new settings
    termios::tcsetattr(&stdin, OptionalActions::Now, &termios)
        .map_err(|e| miette::miette!("failed to set terminal attributes: {e}"))?;

    Ok(())
}

/// Disable raw mode and restore original terminal settings (Unix/Linux/macOS implementation).
///
/// Restores the terminal settings saved by `enable_raw_mode()`. No-op if raw mode was never enabled.
///
/// # Errors
///
/// Returns miette diagnostic errors if:
/// - Terminal attributes cannot be set
/// - Mutex lock is poisoned
pub fn disable_raw_mode() -> miette::Result<()> {
    let original = ORIGINAL_TERMIOS
        .lock()
        .map_err(|e| miette!("terminal settings lock poisoned: {e}"))?;

    if let Some(ref termios) = *original {
        let stdin = io::stdin();
        termios::tcsetattr(&stdin, OptionalActions::Now, termios)
            .map_err(|e| miette::miette!("failed to set terminal attributes: {e}"))?;
    }
    Ok(())
}
