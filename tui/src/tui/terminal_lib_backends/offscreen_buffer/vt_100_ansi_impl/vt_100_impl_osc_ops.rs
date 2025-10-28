// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! OSC (Operating System Command) operations for VT100/ANSI terminal emulation.
//!
//! This module implements OSC operations that correspond to ANSI OSC
//! sequences handled by the `vt_100_ansi_parser::operations::osc_ops` module. These
//! include:
//!
//! - **OSC 0/1/2** (Set Title/Icon) - [`handle_title_and_icon`]
//! - **OSC 8** (Hyperlinks) - [`handle_hyperlink`]
//!
//! All operations maintain VT100 compliance and handle proper OSC event
//! queueing for later transmission to the rendering layer.
//!
//! This module implements the business logic for OSC operations delegated from
//! the parser shim. The `impl_` prefix follows our naming convention for searchable
//! code organization. See the architecture documentation above
//! for the complete three-layer architecture.
//!
//! **Related Files:**
//!
//! [`handle_title_and_icon`]: crate::OffscreenBuffer::handle_title_and_icon
//! [`handle_hyperlink`]: crate::OffscreenBuffer::handle_hyperlink

#[allow(clippy::wildcard_imports)]
use super::super::*;
use crate::core::osc::OscEvent;

impl OffscreenBuffer {
    /// Handle OSC title and icon sequences (OSC 0, 1, 2).
    /// Sets window title and/or icon name by queuing an OSC event.
    pub fn handle_title_and_icon(&mut self, title: &str) {
        self.ansi_parser_support
            .pending_osc_events
            .push(OscEvent::SetTitleAndTab(title.to_string()));
    }

    /// Handle OSC 8 hyperlink sequences.
    /// Creates hyperlinks with URI by queuing an OSC event.
    /// The display text is handled separately via `print()` calls.
    pub fn handle_hyperlink(&mut self, uri: &str) {
        self.ansi_parser_support
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
    use crate::{height, width};

    fn create_test_buffer() -> OffscreenBuffer {
        let size = width(10) + height(6);
        OffscreenBuffer::new_empty(size)
    }

    #[test]
    fn test_handle_title_and_icon() {
        let mut buffer = create_test_buffer();

        // Initially no pending OSC events.
        assert!(buffer.ansi_parser_support.pending_osc_events.is_empty());

        buffer.handle_title_and_icon("My Window Title");

        // Should have one SetTitleAndTab event.
        assert_eq!(buffer.ansi_parser_support.pending_osc_events.len(), 1);
        if let OscEvent::SetTitleAndTab(title) =
            &buffer.ansi_parser_support.pending_osc_events[0]
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
        assert_eq!(buffer.ansi_parser_support.pending_osc_events.len(), 1);
        if let OscEvent::Hyperlink { uri, text } =
            &buffer.ansi_parser_support.pending_osc_events[0]
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
        assert_eq!(buffer.ansi_parser_support.pending_osc_events.len(), 3);

        // Check order is preserved.
        assert!(matches!(
            buffer.ansi_parser_support.pending_osc_events[0],
            OscEvent::SetTitleAndTab(_)
        ));
        assert!(matches!(
            buffer.ansi_parser_support.pending_osc_events[1],
            OscEvent::Hyperlink { .. }
        ));
        assert!(matches!(
            buffer.ansi_parser_support.pending_osc_events[2],
            OscEvent::SetTitleAndTab(_)
        ));
    }

    #[test]
    fn test_empty_title() {
        let mut buffer = create_test_buffer();

        buffer.handle_title_and_icon("");

        assert_eq!(buffer.ansi_parser_support.pending_osc_events.len(), 1);
        if let OscEvent::SetTitleAndTab(title) =
            &buffer.ansi_parser_support.pending_osc_events[0]
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

        assert_eq!(buffer.ansi_parser_support.pending_osc_events.len(), 1);
        if let OscEvent::Hyperlink { uri, text } =
            &buffer.ansi_parser_support.pending_osc_events[0]
        {
            assert_eq!(uri, "");
            assert_eq!(text, "");
        } else {
            panic!("Expected Hyperlink event");
        }
    }
}
