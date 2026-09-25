// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{ClipboardTarget, OscPtyEvent, osc_codes::OscSequence};
use crate::{Pc, TerminalColorRole, core::terminal_io::OutputDevice, ok};
use miette::IntoDiagnostic;

/// Sender for emitting [`OSC`] (Operating System Command) sequences to the terminal's
/// standard output (`stdout`).
///
/// This provides a high-level, ergonomic interface for common outbound [`OSC`] operations
/// such as sending color queries, setting terminal window titles, updating taskbar
/// progress, managing hyperlinks, and setting the host clipboard.
///
/// ## Architectural Context
///
/// For an overview of the bidirectional [`OSC`] pipeline (outbound request generation vs.
/// inbound response decoding) and platform support, see the [module documentation][docs].
///
/// [`OSC`]: crate::OscSequence
/// [docs]: mod@crate::core::ansi::osc#architecture--mental-model-bidirectional-osc-pipeline
#[allow(missing_debug_implementations)]
pub struct OscSender<'a> {
    output_device: &'a OutputDevice,
}

impl<'a> OscSender<'a> {
    /// Creates a new [`OSC`] sender with the given output device.
    ///
    /// [`OSC`]: crate::OscSequence
    #[must_use]
    pub fn new(output_device: &'a OutputDevice) -> Self { Self { output_device } }

    // -------------------------------------------------------------------------
    // Core Sequence Dispatch
    // -------------------------------------------------------------------------

    /// Sends a pre-constructed [`OscSequence`] directly to the terminal.
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the output device fails.
    ///
    /// [`OscSequence`]: crate::OscSequence
    pub fn send_sequence(&mut self, sequence: &OscSequence) -> miette::Result<()> {
        self.write_sequence(&sequence.to_string())
    }

    /// Dispatches an [`OscPtyEvent`] as the corresponding outbound sequence.
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the output device fails.
    ///
    /// [`OscPtyEvent`]: crate::OscPtyEvent
    pub fn send_event(&mut self, event: OscPtyEvent) -> miette::Result<()> {
        match event {
            OscPtyEvent::SetTitleAndTab(text) => self.send_set_title_and_tab(&text),
            OscPtyEvent::ProgressUpdate(percent) => self.send_set_progress(percent),
            OscPtyEvent::ProgressCleared => self.send_clear_progress(),
            OscPtyEvent::Hyperlink { uri, text: _ } => {
                self.send_set_hyperlink(&uri, None)
            }
            _ => ok!(),
        }
    }

    // -------------------------------------------------------------------------
    // Color Queries (Outbound requests; response arrives asynchronously on stdin)
    // -------------------------------------------------------------------------

    /// Sends a query for the terminal color assigned to a specific role
    /// (e.g. [`TerminalColorRole::Background`], [`TerminalColorRole::Foreground`]).
    ///
    /// The terminal emulator responds asynchronously by writing an [`OSC`] color report
    /// sequence back into `stdin`, which is framed by [`vt_100_terminal_input_parser`]
    /// and parsed by [`parse_osc_response`] into [`InputEvent::TerminalColor`].
    ///
    /// > ⚠️ **Platform Availability**: Inbound decoding of terminal color responses is
    /// > Linux-only via [`DirectToAnsiInputDevice`]. On macOS and Windows (Crossterm),
    /// > replies are misparsed as phantom keypresses. For details, see
    /// > [Platform & Backend Availability][osc-platform-support].
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the output device fails.
    ///
    /// [`DirectToAnsiInputDevice`]: crate::DirectToAnsiInputDevice
    /// [`InputEvent::TerminalColor`]: crate::InputEvent::TerminalColor
    /// [`OSC`]: crate::OscSequence
    /// [`parse_osc_response`]: crate::core::ansi::vt_100_terminal_input_parser::chunk_decoder::terminal_events::osc::parse_osc_response
    /// [`vt_100_terminal_input_parser`]: crate::core::ansi::vt_100_terminal_input_parser
    /// [osc-platform-support]: mod@crate::core::ansi::osc#platform--backend-availability
    pub fn send_color_query(&mut self, role: TerminalColorRole) -> miette::Result<()> {
        let sequence = OscSequence::ColorQuery(role);
        self.send_sequence(&sequence)
    }

    /// Sends a query for the terminal's default text background color ([`OSC`] 11).
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the output device fails.
    ///
    /// [`OSC`]: crate::OscSequence
    pub fn send_background_color_query(&mut self) -> miette::Result<()> {
        self.send_color_query(TerminalColorRole::Background)
    }

    /// Short alias for [`send_background_color_query`].
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the output device fails.
    ///
    /// [`send_background_color_query`]: Self::send_background_color_query
    pub fn send_bg_color_query(&mut self) -> miette::Result<()> {
        self.send_background_color_query()
    }

    /// Sends a query for the terminal's default text foreground color ([`OSC`] 10).
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the output device fails.
    ///
    /// [`OSC`]: crate::OscSequence
    pub fn send_foreground_color_query(&mut self) -> miette::Result<()> {
        self.send_color_query(TerminalColorRole::Foreground)
    }

    /// Short alias for [`send_foreground_color_query`].
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the output device fails.
    ///
    /// [`send_foreground_color_query`]: Self::send_foreground_color_query
    pub fn send_fg_color_query(&mut self) -> miette::Result<()> {
        self.send_foreground_color_query()
    }

    /// Sends a query for the terminal's text cursor color ([`OSC`] 12).
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the output device fails.
    ///
    /// [`OSC`]: crate::OscSequence
    pub fn send_cursor_color_query(&mut self) -> miette::Result<()> {
        self.send_color_query(TerminalColorRole::Cursor)
    }

    // -------------------------------------------------------------------------
    // Window & Tab Titles (OSC 0, 1, 2)
    // -------------------------------------------------------------------------

    /// Sets terminal window title and tab name using [`OSC`] 0 sequence.
    /// This is the most commonly supported title-setting sequence across terminals.
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the output device fails.
    ///
    /// [`OSC`]: crate::OscSequence
    pub fn send_set_title_and_tab(&mut self, text: &str) -> miette::Result<()> {
        let sequence = OscSequence::SetTitleAndIcon(text.to_string());
        self.send_sequence(&sequence)
    }

    /// Sets terminal window title using [`OSC`] 2 sequence.
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the output device fails.
    ///
    /// [`OSC`]: crate::OscSequence
    pub fn send_set_title(&mut self, text: &str) -> miette::Result<()> {
        let sequence = OscSequence::SetTitle(text.to_string());
        self.send_sequence(&sequence)
    }

    /// Sets terminal icon name using [`OSC`] 1 sequence.
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the output device fails.
    ///
    /// [`OSC`]: crate::OscSequence
    pub fn send_set_icon(&mut self, text: &str) -> miette::Result<()> {
        let sequence = OscSequence::SetIcon(text.to_string());
        self.send_sequence(&sequence)
    }

    // -------------------------------------------------------------------------
    // Taskbar & Tab Progress (OSC 9;4)
    // -------------------------------------------------------------------------

    /// Sets the terminal taskbar or tab progress indicator using [`OSC`] 9;4.
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the output device fails.
    ///
    /// [`OSC`]: crate::OscSequence
    pub fn send_set_progress(&mut self, percent: Pc) -> miette::Result<()> {
        let sequence = OscSequence::ProgressUpdate(percent);
        self.send_sequence(&sequence)
    }

    /// Clears / removes the terminal taskbar or tab progress indicator using [`OSC`] 9;4.
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the output device fails.
    ///
    /// [`OSC`]: crate::OscSequence
    pub fn send_clear_progress(&mut self) -> miette::Result<()> {
        let sequence = OscSequence::ProgressCleared;
        self.send_sequence(&sequence)
    }

    // -------------------------------------------------------------------------
    // Clickable Hyperlinks (OSC 8)
    // -------------------------------------------------------------------------

    /// Starts a hyperlink using [`OSC`] 8 sequence.
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the output device fails.
    ///
    /// [`OSC`]: crate::OscSequence
    pub fn send_set_hyperlink(
        &mut self,
        uri: &str,
        maybe_link_correlation_id: Option<&str>,
    ) -> miette::Result<()> {
        let sequence = OscSequence::HyperlinkStart {
            uri: uri.to_string(),
            maybe_link_correlation_id: maybe_link_correlation_id.map(ToString::to_string),
        };
        self.send_sequence(&sequence)
    }

    /// Closes a hyperlink using [`OSC`] 8 end sequence.
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the output device fails.
    ///
    /// [`OSC`]: crate::OscSequence
    pub fn send_clear_hyperlink(&mut self) -> miette::Result<()> {
        let sequence = OscSequence::HyperlinkEnd;
        self.send_sequence(&sequence)
    }

    // -------------------------------------------------------------------------
    // Host Clipboard (OSC 52)
    // -------------------------------------------------------------------------

    /// Copies text to the system or primary host clipboard via [`OSC`] 52.
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the output device fails.
    ///
    /// [`OSC`]: crate::OscSequence
    pub fn send_set_clipboard(
        &mut self,
        target: ClipboardTarget,
        data: &str,
    ) -> miette::Result<()> {
        let sequence = OscSequence::ClipboardSet {
            target,
            data: data.to_string(),
        };
        self.send_sequence(&sequence)
    }

    /// Copies text to the host system clipboard via [`OSC`] 52.
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the output device fails.
    ///
    /// [`OSC`]: crate::OscSequence
    pub fn send_set_system_clipboard(&mut self, data: &str) -> miette::Result<()> {
        self.send_set_clipboard(ClipboardTarget::System, data)
    }

    // -------------------------------------------------------------------------
    // Private Helpers
    // -------------------------------------------------------------------------

    /// Low-level method to write an [`OSC`] sequence directly to the output device.
    ///
    /// [`OSC`]: crate::OscSequence
    fn write_sequence(&mut self, sequence: &str) -> miette::Result<()> {
        self.output_device.write(|writer| {
            write!(writer, "{sequence}").into_diagnostic()?;
            Ok::<(), miette::Report>(())
        })?;
        ok!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{OutputDevice, OutputDeviceExt, pc};

    #[test]
    fn test_osc_sender_send_set_progress() {
        let (output_device, mock) = OutputDevice::new_mock();
        let mut sender = OscSender::new(&output_device);
        sender.send_set_progress(pc!(50).unwrap()).unwrap();
        assert_eq!(mock.get_copy_of_buffer_as_string(), "\x1b]9;4;1;50\x1b\\");

        sender.send_clear_progress().unwrap();
        assert_eq!(
            mock.get_copy_of_buffer_as_string(),
            "\x1b]9;4;1;50\x1b\\\x1b]9;4;0;0\x1b\\"
        );
    }

    #[test]
    fn test_osc_sender_send_set_title_and_tab() {
        let (output_device, mock) = OutputDevice::new_mock();
        let mut sender = OscSender::new(&output_device);
        sender.send_set_title_and_tab("My Title").unwrap();
        assert_eq!(mock.get_copy_of_buffer_as_string(), "\x1b]0;My Title\x07");
    }

    #[test]
    fn test_osc_sender_send_set_title() {
        let (output_device, mock) = OutputDevice::new_mock();
        let mut sender = OscSender::new(&output_device);
        sender.send_set_title("Window").unwrap();
        assert_eq!(mock.get_copy_of_buffer_as_string(), "\x1b]2;Window\x07");
    }

    #[test]
    fn test_osc_sender_send_set_icon() {
        let (output_device, mock) = OutputDevice::new_mock();
        let mut sender = OscSender::new(&output_device);
        sender.send_set_icon("Icon").unwrap();
        assert_eq!(mock.get_copy_of_buffer_as_string(), "\x1b]1;Icon\x07");
    }

    #[test]
    fn test_osc_sender_send_hyperlinks() {
        let (output_device, mock) = OutputDevice::new_mock();
        let mut sender = OscSender::new(&output_device);
        sender
            .send_set_hyperlink("https://example.com", None)
            .unwrap();
        assert_eq!(
            mock.get_copy_of_buffer_as_string(),
            "\x1b]8;;https://example.com\x07"
        );

        sender.send_clear_hyperlink().unwrap();
        assert_eq!(
            mock.get_copy_of_buffer_as_string(),
            "\x1b]8;;https://example.com\x07\x1b]8;;\x07"
        );

        let (output_device_with_id, mock_with_id) = OutputDevice::new_mock();
        let mut sender_with_id = OscSender::new(&output_device_with_id);
        sender_with_id
            .send_set_hyperlink("https://example.com", Some("id123"))
            .unwrap();
        assert_eq!(
            mock_with_id.get_copy_of_buffer_as_string(),
            "\x1b]8;id123;https://example.com\x07"
        );
    }

    #[test]
    fn test_osc_sender_send_color_queries() {
        let (output_device, mock) = OutputDevice::new_mock();
        let mut sender = OscSender::new(&output_device);
        sender.send_bg_color_query().unwrap();
        sender.send_fg_color_query().unwrap();
        sender.send_cursor_color_query().unwrap();
        assert_eq!(
            mock.get_copy_of_buffer_as_string(),
            "\x1b]11;?\x07\x1b]10;?\x07\x1b]12;?\x07"
        );
    }

    #[test]
    fn test_osc_sender_send_clipboard() {
        let (output_device, mock) = OutputDevice::new_mock();
        let mut sender = OscSender::new(&output_device);
        sender.send_set_system_clipboard("Hello").unwrap();
        assert_eq!(
            mock.get_copy_of_buffer_as_string(),
            "\x1b]52;c;SGVsbG8=\x07"
        );

        let (output_device_primary, mock_primary) = OutputDevice::new_mock();
        let mut sender_primary = OscSender::new(&output_device_primary);
        sender_primary
            .send_set_clipboard(ClipboardTarget::Primary, "World")
            .unwrap();
        assert_eq!(
            mock_primary.get_copy_of_buffer_as_string(),
            "\x1b]52;p;V29ybGQ=\x07"
        );
    }

    #[test]
    fn test_osc_sender_send_event() {
        let (output_device, mock) = OutputDevice::new_mock();
        let mut sender = OscSender::new(&output_device);
        sender
            .send_event(OscPtyEvent::SetTitleAndTab("EventTitle".to_string()))
            .unwrap();
        sender
            .send_event(OscPtyEvent::ProgressUpdate(pc!(75).unwrap()))
            .unwrap();
        sender.send_event(OscPtyEvent::ProgressCleared).unwrap();
        sender
            .send_event(OscPtyEvent::Hyperlink {
                uri: "https://example.com".to_string(),
                text: "Example".to_string(),
            })
            .unwrap();
        // Wildcard no-op branch should not write anything new to the buffer.
        sender.send_event(OscPtyEvent::BuildError).unwrap();
        sender
            .send_event(OscPtyEvent::IndeterminateProgress)
            .unwrap();

        assert_eq!(
            mock.get_copy_of_buffer_as_string(),
            "\x1b]0;EventTitle\x07\x1b]9;4;1;75\x1b\\\x1b]9;4;0;0\x1b\\\x1b]8;;https://example.com\x07"
        );
    }
}
