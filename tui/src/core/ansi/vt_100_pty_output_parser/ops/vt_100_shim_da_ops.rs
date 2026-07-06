// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Device Attributes ([`DA`]) operations.
//!
//! This module acts as a thin shim layer that delegates to the actual implementation.
//! Refer to the module-level documentation in the ops module for details on the
//! "shim → impl → test" architecture and naming conventions.
//!
//! **Related Files:**
//! - **Implementation**: [`vt_100_impl_da_ops`] - Business logic with unit tests
//! - **Integration Tests**: [`test_da_ops`] - Full pipeline testing via public API
//!
//! # Testing Strategy
//!
//! **This shim layer intentionally has no direct unit tests.**
//!
//! This is a deliberate architectural decision: these functions are pure delegation
//! layers with no business logic. Testing is comprehensively handled by:
//! - **Unit tests** in the implementation layer (with `#[test]` functions)
//! - **Integration tests** in the conformance tests validating the full pipeline
//!
//! For the complete testing philosophy and rationale behind this approach,
//! see the [ops module].
//!
//! # Architecture Overview
//!
//! See the [module-level Architecture Overview].
//!
//! # [`CSI`] Sequence Processing Flow
//!
//! ```text
//! Application sends "ESC [ c" (request device attributes)
//!         ↓
//!     PTY Controlled (escape sequence)
//!         ↓
//!     PTY Controller (byte stream) <- in process_manager.rs
//!         ↓
//!     VTE Parser (parses `ESC [`...char pattern)
//!         ↓
//!     csi_dispatch() [routes to modules below]
//!         ↓
//!     Route to ops module:
//!       - cursor_ops:: for movement (A,B,C,D,H)
//!       - scroll_ops:: for scrolling (S,T)
//!       - sgr_ops:: for styling (m)
//!       - line_ops:: for lines (L,M)
//!       - char_ops:: for chars (@,P,X)
//!       - dsr_ops:: for device status (n)                      ╭───────────╮
//!       - da_ops:: for device attributes (c)                <- │THIS MODULE│
//!         ↓                                                    ╰───────────╯
//!     Update OfsBuf state
//! ```
//!
//! [`CSI`]: crate::CsiSequence
//! [`DA`]: crate::DaSequence
//! [`test_da_ops`]: crate::vt_100_pty_output_conformance_tests::tests::vt_100_test_da_ops
//! [`vt_100_impl_da_ops`]: crate::core::ansi::vt_100_pty_output_parser::ops_impl_ofs_buf::vt_100_impl_da_ops
//! [module-level Architecture Overview]: super#architecture-overview
//! [module-level documentation]: self
//! [ops module]: crate::core::ansi::vt_100_pty_output_parser::ops


use crate::{AnsiToOfsBufPerformer, DEBUG_TUI_VT100_PARSER};

/// Handles Device Attributes ([`DA`]) query responses `CSI c` or `CSI 0 c` received FROM
/// the [`PTY`] child process.
///
/// Also see: [`OfsBufVT100::handle_device_attributes_request()`]
///
/// [`DA`]: crate::DaSequence
/// [`DaSequence`]: crate::DaSequence
/// [`OfsBufVT100::handle_device_attributes_request()`]:
///     crate::OfsBufVT100::handle_device_attributes_request
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
pub fn device_attributes(
    performer: &mut AnsiToOfsBufPerformer,
    params: &vte::Params,
    intermediates: &[u8],
) {
    if !intermediates.is_empty() {
        // We only support DA1. Intermediates mean it's DA2 (>) or DA3 (=) or something
        // else.
        DEBUG_TUI_VT100_PARSER.then(|| {
            tracing::warn!("DA intermediate not supported: {:?}", intermediates);
        });
        return;
    }

    // According to specs, CSI c is the same as CSI 0 c for DA1.
    // vte::Params iterates over array elements representing parameters.
    let mut param_iter = params.iter();
    let is_da1 = match param_iter.next() {
        // CSI c (no params)
        None => true,
        // CSI 0 c (param = 0)
        Some(param) => param[0] == 0,
    };

    if is_da1 {
        performer.ofs_buf_vt_100.handle_device_attributes_request();
    } else {
        DEBUG_TUI_VT100_PARSER.then(|| {
            tracing::warn!("Unsupported DA query params: {:?}", params);
        });
    }
}
