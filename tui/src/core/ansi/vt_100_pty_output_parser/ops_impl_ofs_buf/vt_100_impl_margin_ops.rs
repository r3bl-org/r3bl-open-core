// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Scroll margin operations for [`VT-100`]/[`ANSI`] terminal emulation.
//!
//! This module implements scroll margin operations that correspond to [`ANSI`] sequences
//! handled by the [`vt_100_pty_output_parser::ops::margin_ops`] module. These include:
//!
//! - **[`DECSTBM`]** (Set Top and Bottom Margins) - [`set_scroll_margins`]
//! - **Reset margins** - [`reset_scroll_margins`]
//!
//! All operations maintain [`VT-100`] compliance and handle proper scroll region
//! boundaries for terminal operations.
//!
//! This module implements the business logic for margin operations delegated from the
//! parser shim. The `impl_` prefix follows our naming convention for searchable code
//! organization. See the architecture documentation above for the complete three-layer
//! architecture.
//!
//! [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
//! [`DECSTBM`]: https://vt100.net/docs/vt510-rm/DECSTBM.html
//! [`reset_scroll_margins`]: crate::core::ansi::OfsBufVT100::reset_scroll_margins
//! [`set_scroll_margins`]: crate::core::ansi::OfsBufVT100::set_scroll_margins
//! [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
//! [`vt_100_pty_output_parser::ops::margin_ops`]:
//!     crate::core::ansi::vt_100_pty_output_parser::ops::vt_100_shim_margin_ops

#[allow(clippy::wildcard_imports)]
use super::super::*;
use crate::{core::coordinates::{TermRow, bounds_check::LengthOps},
            vp_height};

impl OfsBufVT100 {
    /// Reset scroll margins to full screen (no restrictions).
    ///
    /// This disables any active scroll region and allows operations to affect the entire
    /// buffer.
    pub fn reset_scroll_margins(&mut self) {
        self.get_parser_global_state_mut().scroll_region_top = None;
        self.get_parser_global_state_mut().scroll_region_bottom = None;
    }

    /// Set top and bottom scroll margins for the buffer.
    ///
    /// Operations like scrolling and line insertion/deletion will be restricted to this
    /// region.
    ///
    /// Validates input parameters and sets margins only if valid. Invalid parameters
    /// (e.g., top >= bottom) are logged and ignored per [`VT-100`] spec.
    ///
    /// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
    pub fn set_scroll_margins(&mut self, top: TermRow, bottom: TermRow) {
        let buffer_height = self
            .get_active_screen_buffer_mut()
            .get_viewport()
            .get_height();

        // Use type-safe bounds checking: convert TermRow to VPHeight for
        // clamping.
        let clamped_bottom = vp_height(bottom.as_u16()).clamp_to_max(buffer_height);

        if !(top.as_u16() < clamped_bottom.as_u16() && *clamped_bottom <= *buffer_height)
        {
            tracing::warn!(
                "Invalid scroll margins: top={}, bottom={}, buffer_height={:?}",
                top.as_u16(),
                bottom.as_u16(),
                buffer_height
            );
            return;
        }

        let clamped_bottom_row: TermRow = clamped_bottom.convert_to_index().into();

        self.get_parser_global_state_mut().scroll_region_top = Some(top);
        self.get_parser_global_state_mut().scroll_region_bottom =
            Some(clamped_bottom_row);
    }
}

#[cfg(test)]
mod tests_margin_ops {
    use crate::{OfsBufVT100, term_row, vp_height, vp_width,
                vt_100_pty_output_conformance_tests::nz};

    fn create_test_buffer() -> OfsBufVT100 {
        let size = vp_width(10) + vp_height(6);
        OfsBufVT100::new_empty(size)
    }

    #[test]
    fn test_reset_scroll_margins() {
        let mut buffer = create_test_buffer();

        // Set some margins first.
        buffer.get_parser_global_state_mut().scroll_region_top = Some(term_row(nz(2)));
        buffer.get_parser_global_state_mut().scroll_region_bottom = Some(term_row(nz(4)));

        buffer.reset_scroll_margins();

        // Should be reset to None.
        assert!(
            buffer
                .get_parser_global_state_mut()
                .scroll_region_top
                .is_none()
        );
        assert!(
            buffer
                .get_parser_global_state_mut()
                .scroll_region_bottom
                .is_none()
        );
    }

    #[test]
    fn test_set_scroll_margins_valid() {
        let mut buffer = create_test_buffer();

        buffer.set_scroll_margins(term_row(nz(2)), term_row(nz(4)));

        // Check that margins were set.
        assert_eq!(
            buffer.get_parser_global_state_mut().scroll_region_top,
            Some(term_row(nz(2)))
        );
        assert_eq!(
            buffer.get_parser_global_state_mut().scroll_region_bottom,
            Some(term_row(nz(4)))
        );
    }

    #[test]
    fn test_set_scroll_margins_invalid_top_greater_than_bottom() {
        let mut buffer = create_test_buffer();

        buffer.set_scroll_margins(term_row(nz(4)), term_row(nz(2)));

        // Margins should remain unchanged. (None).
        assert!(
            buffer
                .get_parser_global_state_mut()
                .scroll_region_top
                .is_none()
        );
        assert!(
            buffer
                .get_parser_global_state_mut()
                .scroll_region_bottom
                .is_none()
        );
    }

    #[test]
    fn test_set_scroll_margins_bottom_exceeds_buffer() {
        let mut buffer = create_test_buffer();

        // Try to set bottom margin beyond buffer height (buffer height is 6).
        buffer.set_scroll_margins(term_row(nz(2)), term_row(nz(10)));

        // Bottom should be clamped to buffer height.
        assert_eq!(
            buffer.get_parser_global_state_mut().scroll_region_top,
            Some(term_row(nz(2)))
        );
        assert_eq!(
            buffer.get_parser_global_state_mut().scroll_region_bottom,
            Some(term_row(nz(6)))
        );
    }

    #[test]
    fn test_set_scroll_margins_equal_top_and_bottom() {
        let mut buffer = create_test_buffer();

        buffer.set_scroll_margins(term_row(nz(3)), term_row(nz(3)));

        // Margins should remain unchanged.
        assert!(
            buffer
                .get_parser_global_state_mut()
                .scroll_region_top
                .is_none()
        );
        assert!(
            buffer
                .get_parser_global_state_mut()
                .scroll_region_bottom
                .is_none()
        );
    }
}
