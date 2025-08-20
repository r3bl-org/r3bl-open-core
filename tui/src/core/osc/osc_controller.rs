// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! OSC controller for sending OSC sequences to the terminal.

use std::io::Write;
use crate::{
    core::terminal_io::OutputDevice,
    lock_output_device_as_mut,
};
use super::{osc_codes, OscEvent};
use miette::IntoDiagnostic;

/// Controller for sending OSC sequences to the terminal.
/// This provides a high-level interface for common OSC operations
/// like setting terminal titles and sending progress updates.
pub struct OscController<'a> {
    output_device: &'a OutputDevice,
}

impl<'a> OscController<'a> {
    /// Creates a new OSC controller with the given output device.
    pub fn new(output_device: &'a OutputDevice) -> Self {
        Self { output_device }
    }

    /// Set terminal window title and tab name using OSC 0 sequence.
    /// This is the most commonly supported title-setting sequence across terminals.
    pub fn set_title_and_tab(&mut self, text: &str) -> miette::Result<()> {
        let sequence = format!(
            "{}{}{}",
            osc_codes::OSC0_SET_TITLE_AND_TAB,
            text,
            osc_codes::BELL_TERMINATOR
        );
        self.write_sequence(&sequence)
    }

    /// Generic method to send any OSC event to the terminal.
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
        write!(
            lock_output_device_as_mut!(self.output_device),
            "{}",
            sequence
        ).into_diagnostic()?;
        Ok(())
    }
}