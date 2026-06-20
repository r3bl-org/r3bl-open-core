// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`OSC`] (Operating System Command) operations for [`VT-100`]/[`ANSI`] terminal
//! emulation.
//!
//! This module implements [`OSC`] operations that correspond to [`ANSI`] [`OSC`]
//! sequences handled by the [`vt_100_pty_output_parser::ops::osc_ops`] module. These
//! include:
//!
//! - `ESC ] 0` (Set Icon Name and Window Title) - [`handle_title_and_icon`]
//! - `ESC ] 1` (Set Icon Name) - [`handle_title_and_icon`]
//! - `ESC ] 2` (Set Window Title) - [`handle_title_and_icon`]
//! - `ESC ] 8` (Hyperlinks) - [`handle_hyperlink`]
//!
//! All operations maintain [`VT-100`] compliance and handle proper [`OSC`] event queueing
//! for later transmission to the rendering layer.
//!
//! This module implements the business logic for [`OSC`] operations delegated from the
//! parser shim. The `impl_` prefix follows our naming convention for searchable code
//! organization. See the architecture documentation above for the complete three-layer
//! architecture.
//!
//! [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
//! [`handle_hyperlink`]: crate::OfsBufVT100::handle_hyperlink
//! [`handle_title_and_icon`]: crate::OfsBufVT100::handle_title_and_icon
//! [`OSC`]: crate::osc_codes::OscSequence
//! [`print_char()`]: crate::OfsBufVT100::print_char
//! [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
//! [`vt_100_pty_output_parser::ops::osc_ops`]:
//!     crate::core::ansi::vt_100_pty_output_parser::ops::vt_100_shim_osc_ops

#[allow(clippy::wildcard_imports)]
use super::super::*;
use crate::core::osc::OscEvent;

impl OfsBufVT100 {
    /// Handle:
    /// - `ESC ] 0` (Set Icon Name and Window Title),
    /// - `ESC ] 1` (Set Icon Name),
    /// - `ESC ] 2` (Set Window Title) sequences.
    ///
    /// Sets window title and/or icon name by queuing an [`OSC`] event.
    ///
    /// [`OSC`]: crate::osc_codes::OscSequence
    pub fn handle_title_and_icon(&mut self, title: &str) {
        self.parser_global_state
            .pending_osc_events
            .push(OscEvent::SetTitleAndTab(title.to_string()));
    }

    /// Handle `ESC ] 8` hyperlink sequences.
    ///
    /// Creates hyperlinks with URI by queuing an [`OSC`] event. The display text is
    /// handled separately via [`print_char()`] calls.
    ///
    /// [`OSC`]: crate::osc_codes::OscSequence
    pub fn handle_hyperlink(&mut self, uri: &str) {
        self.parser_global_state
            .pending_osc_events
            .push(OscEvent::Hyperlink {
                uri: uri.to_string(),
                text: String::new(), // Text is handled separately via print()
            });
    }
}

#[cfg(test)]
mod tests_osc_ops {
    use super::*;
    use crate::{OfsBufVT100, height, width};

    fn create_test_buffer() -> OfsBufVT100 {
        let size = width(10) + height(6);
        OfsBufVT100::new_empty(size)
    }

    #[test]
    fn test_handle_title_and_icon() {
        let mut buffer = create_test_buffer();

        // Initially no pending OSC events.
        assert!(buffer.parser_global_state.pending_osc_events.is_empty());

        buffer.handle_title_and_icon("My Window Title");

        // Should have one SetTitleAndTab event.
        assert_eq!(buffer.parser_global_state.pending_osc_events.len(), 1);
        if let OscEvent::SetTitleAndTab(title) =
            &buffer.parser_global_state.pending_osc_events[0]
        {
            assert_eq!(title, "My Window Title");
        } else {
            panic!("Expected SetTitleAndTab event");
        }
    }

    #[test]
    fn test_handle_hyperlink() {
        let mut buffer = create_test_buffer();

        buffer.handle_hyperlink("https://example.com");

        // Should have one Hyperlink event.
        assert_eq!(buffer.parser_global_state.pending_osc_events.len(), 1);
        if let OscEvent::Hyperlink { uri, text } =
            &buffer.parser_global_state.pending_osc_events[0]
        {
            assert_eq!(uri, "https://example.com");
            assert_eq!(text, ""); // Text is handled separately
        } else {
            panic!("Expected Hyperlink event");
        }
    }

    #[test]
    fn test_multiple_osc_events() {
        let mut buffer = create_test_buffer();

        buffer.handle_title_and_icon("Title 1");
        buffer.handle_hyperlink("https://link1.com");
        buffer.handle_title_and_icon("Title 2");

        // Should have three events queued.
        assert_eq!(buffer.parser_global_state.pending_osc_events.len(), 3);

        // Check order is preserved.
        assert!(matches!(
            buffer.parser_global_state.pending_osc_events[0],
            OscEvent::SetTitleAndTab(_)
        ));
        assert!(matches!(
            buffer.parser_global_state.pending_osc_events[1],
            OscEvent::Hyperlink { .. }
        ));
        assert!(matches!(
            buffer.parser_global_state.pending_osc_events[2],
            OscEvent::SetTitleAndTab(_)
        ));
    }

    #[test]
    fn test_empty_title() {
        let mut buffer = create_test_buffer();

        buffer.handle_title_and_icon("");

        assert_eq!(buffer.parser_global_state.pending_osc_events.len(), 1);
        if let OscEvent::SetTitleAndTab(title) =
            &buffer.parser_global_state.pending_osc_events[0]
        {
            assert_eq!(title, "");
        } else {
            panic!("Expected SetTitleAndTab event");
        }
    }

    #[test]
    fn test_empty_uri() {
        let mut buffer = create_test_buffer();

        buffer.handle_hyperlink("");

        assert_eq!(buffer.parser_global_state.pending_osc_events.len(), 1);
        if let OscEvent::Hyperlink { uri, text } =
            &buffer.parser_global_state.pending_osc_events[0]
        {
            assert_eq!(uri, "");
            assert_eq!(text, "");
        } else {
            panic!("Expected Hyperlink event");
        }
    }
}
