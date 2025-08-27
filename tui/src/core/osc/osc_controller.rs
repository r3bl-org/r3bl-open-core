// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! OSC controller for sending OSC sequences to the terminal.

use miette::IntoDiagnostic;

use super::{OscEvent, osc_codes::OscSequence};
use crate::{core::terminal_io::OutputDevice, lock_output_device_as_mut};

/// Controller for sending OSC sequences to the terminal.
/// This provides a high-level interface for common OSC operations
/// like setting terminal titles and sending progress updates.
pub struct OscController<'a> {
    output_device: &'a OutputDevice,
}

impl std::fmt::Debug for OscController<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OscController")
            .field("output_device", &"<OutputDevice>")
            .finish()
    }
}

impl<'a> OscController<'a> {
    /// Creates a new OSC controller with the given output device.
    #[must_use]
    pub fn new(output_device: &'a OutputDevice) -> Self { Self { output_device } }

    /// Set terminal window title and tab name using OSC 0 sequence.
    /// This is the most commonly supported title-setting sequence across terminals.
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the output device fails.
    pub fn set_title_and_tab(&mut self, text: &str) -> miette::Result<()> {
        let sequence = OscSequence::SetTitleAndIcon(text.to_string());
        self.write_sequence(&sequence.to_string())
    }

    /// Generic method to send any OSC event to the terminal.
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the output device fails.
    pub fn send_event(&mut self, event: OscEvent) -> miette::Result<()> {
        match event {
            OscEvent::SetTitleAndTab(text) => self.set_title_and_tab(&text),
            // For other events, we would need to implement their formatting
            // For now, we'll focus on the title setting functionality
            _ => Ok(()), // Ignore other events for now
        }
    }

    /// Low-level method to write an OSC sequence directly to the output device.
    fn write_sequence(&mut self, sequence: &str) -> miette::Result<()> {
        write!(lock_output_device_as_mut!(self.output_device), "{sequence}")
            .into_diagnostic()?;
        Ok(())
    }
}
