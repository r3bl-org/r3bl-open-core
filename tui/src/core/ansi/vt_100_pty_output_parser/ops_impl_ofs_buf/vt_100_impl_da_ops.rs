// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Device Attributes ([`DA`]) operations for [`VT-100`]/[`ANSI`] terminal emulation.
//!
//! This module implements [`DA`] operations that correspond to [`ANSI`] [`DA`] sequences
//! handled by the [`vt_100_pty_output_parser::ops::vt_100_shim_da_ops`] module. These
//! include:
//!
//! - **Primary Device Attributes (DA1)** - [`handle_device_attributes_request`]
//!
//! All operations maintain [`VT-100`] compliance and handle proper response queueing for
//! later transmission back to the [`PTY`].
//!
//! This module implements the business logic for [`DA`] operations delegated from the
//! parser shim. The `impl_` prefix follows our naming convention for searchable code
//! organization. See the architecture documentation above for the complete three-layer
//! architecture.
//!
//! [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
//! [`DA`]: crate::DaSequence
//! [`handle_device_attributes_request`]:
//!     crate::OfsBufVT100::handle_device_attributes_request
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
//! [`vt_100_pty_output_parser::ops::vt_100_shim_da_ops`]:
//!     crate::core::ansi::vt_100_pty_output_parser::ops::vt_100_shim_da_ops

use crate::{OfsBufVT100, PtyResponseEvent};

impl OfsBufVT100 {
    /// Handles a Device Attributes ([`DA`]) request by pushing a
    /// [`PrimaryDeviceAttributes`] response event into the pending response queue.
    ///
    /// [`DA`]: crate::DaSequence
    /// [`PrimaryDeviceAttributes`]: crate::PtyResponseEvent::PrimaryDeviceAttributes
    pub fn handle_device_attributes_request(&mut self) {
        self.parser_global_state
            .pending_pty_response_events
            .push(PtyResponseEvent::PrimaryDeviceAttributes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{height, width};

    #[test]
    fn test_handle_device_attributes_request() {
        let size = width(80) + height(24);
        let mut buffer = OfsBufVT100::new_empty(size);

        assert!(buffer.parser_global_state.pending_pty_response_events.is_empty());

        buffer.handle_device_attributes_request();

        assert_eq!(buffer.parser_global_state.pending_pty_response_events.len(), 1);
        assert_eq!(
            buffer.parser_global_state.pending_pty_response_events[0],
            PtyResponseEvent::PrimaryDeviceAttributes
        );
    }
}
