// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Style/Graphics Rendition operations.
//!
//! This module acts as a thin shim layer that delegates to the actual implementation.
//! See the [module-level documentation] for details on the "shim → impl →
//! test" architecture and naming conventions.
//!
//! **Related Files:**
//! - **Implementation**: [`impl_sgr_ops`] - Business logic with unit tests
//! - **Integration Tests**: [`test_sgr_ops`] - Full pipeline testing via public API
//!
//! # Testing Strategy
//!
//! **This shim layer intentionally has no direct unit tests.**
//!
//! This is a deliberate architectural decision: these functions are pure delegation
//! layers with no business logic. Testing is comprehensively handled by:
//! - **Unit tests** in the implementation layer (with `#[test]` functions)
//! - **Integration tests** in [`vt_100_ansi_conformance_tests`] validating the full
//!   pipeline
//!
//! See the [operations module documentation] for the complete testing philosophy
//! and rationale behind this approach.
//!
//! # Architecture Overview
//!
//! ```text
//! ╭─────────────────╮    ╭───────────────╮    ╭─────────────────╮    ╭──────────────╮
//! │ Child Process   │────▶ PTY Master    │────▶ VTE Parser      │────▶ OffscreenBuf │
//! │ (vim, bash...)  │    │ (byte stream) │    │ (state machine) │    │ (terminal    │
//! ╰─────────────────╯    ╰───────────────╯    ╰─────────────────╯    │  buffer)     │
//!        │                                            │              ╰──────────────╯
//!        │                                            │                      │
//!        │                                   ╔════════▼════════╗             │
//!        │                                   ║ Perform Trait   ║             │
//!        │                                   ║ Implementation  ║             │
//!        │                                   ╚═════════════════╝             │
//!        │                                                                   │
//!        │                                   ╭─────────────────╮             │
//!        │                                   │ RenderPipeline  ◀─────────────╯
//!        │                                   │ paint()         │
//!        ╰───────────────────────────────────▶ Terminal Output │
//!                                            ╰─────────────────╯
//! ```
//!
//! # CSI Sequence Processing Flow
//!
//! ```text
//! Application sends "ESC[1;31m" (bold red text)
//!         ↓
//!     PTY Slave (escape sequence)
//!         ↓
//!     PTY Master (byte stream) <- in process_manager.rs
//!         ↓
//!     VTE Parser (parses ESC[...char pattern)
//!         ↓
//!     csi_dispatch() [routes to modules below]
//!         ↓
//!     Route to operations module:
//!       - cursor_ops:: for movement (A,B,C,D,H)
//!       - scroll_ops:: for scrolling (S,T) ╭───────────╮
//!       - sgr_ops:: for styling (m) <----- │THIS MODULE│
//!       - line_ops:: for lines (L,M)       ╰───────────╯
//!       - char_ops:: for chars (@,P,X)
//!         ↓
//!     Update OffscreenBuffer state
//! ```
//!
//! [`impl_sgr_ops`]: crate::tui::terminal_lib_backends::offscreen_buffer::vt_100_ansi_impl::vt_100_impl_sgr_ops
//! [`test_sgr_ops`]: crate::core::pty_mux::vt_100_ansi_parser::vt_100_ansi_conformance_tests::tests::vt_100_test_sgr_ops
//! [module-level documentation]: super::super
//! [operations module documentation]: super
//! [`vt_100_ansi_conformance_tests`]: super::super::vt_100_ansi_conformance_tests

use super::super::{ansi_parser_public_api::AnsiToOfsBufPerformer, protocols::csi_codes};
use crate::{ParamsExt, tui_style_attrib};
use vte::Params;

/// Apply a single SGR parameter.
fn apply_sgr_param(performer: &mut AnsiToOfsBufPerformer, param: u16) {
    match param {
        csi_codes::SGR_RESET => {
            performer.ofs_buf.reset_all_style_attributes();
        }
        csi_codes::SGR_BOLD => {
            performer
                .ofs_buf
                .apply_style_attribute(tui_style_attrib::Bold.into());
        }
        csi_codes::SGR_DIM => {
            performer
                .ofs_buf
                .apply_style_attribute(tui_style_attrib::Dim.into());
        }
        csi_codes::SGR_ITALIC => {
            performer
                .ofs_buf
                .apply_style_attribute(tui_style_attrib::Italic.into());
        }
        csi_codes::SGR_UNDERLINE => {
            performer
                .ofs_buf
                .apply_style_attribute(tui_style_attrib::Underline.into());
        }
        csi_codes::SGR_BLINK => {
            performer
                .ofs_buf
                .apply_style_attribute(tui_style_attrib::BlinkMode::Slow.into());
        }
        csi_codes::SGR_RAPID_BLINK => {
            performer
                .ofs_buf
                .apply_style_attribute(tui_style_attrib::BlinkMode::Rapid.into());
        }
        csi_codes::SGR_REVERSE => {
            performer
                .ofs_buf
                .apply_style_attribute(tui_style_attrib::Reverse.into());
        }
        csi_codes::SGR_HIDDEN => {
            performer
                .ofs_buf
                .apply_style_attribute(tui_style_attrib::Hidden.into());
        }
        csi_codes::SGR_STRIKETHROUGH => {
            performer
                .ofs_buf
                .apply_style_attribute(tui_style_attrib::Strikethrough.into());
        }
        csi_codes::SGR_RESET_BOLD_DIM => {
            performer
                .ofs_buf
                .reset_style_attribute(tui_style_attrib::Bold.into());
        }
        csi_codes::SGR_RESET_ITALIC => {
            performer
                .ofs_buf
                .reset_style_attribute(tui_style_attrib::Italic.into());
        }
        csi_codes::SGR_RESET_UNDERLINE => {
            performer
                .ofs_buf
                .reset_style_attribute(tui_style_attrib::Underline.into());
        }
        csi_codes::SGR_RESET_BLINK => {
            performer
                .ofs_buf
                .reset_style_attribute(tui_style_attrib::BlinkMode::Slow.into());
        }
        csi_codes::SGR_RESET_REVERSE => {
            performer
                .ofs_buf
                .reset_style_attribute(tui_style_attrib::Reverse.into());
        }
        csi_codes::SGR_RESET_HIDDEN => {
            performer
                .ofs_buf
                .reset_style_attribute(tui_style_attrib::Hidden.into());
        }
        csi_codes::SGR_RESET_STRIKETHROUGH => {
            performer
                .ofs_buf
                .reset_style_attribute(tui_style_attrib::Strikethrough.into());
        }
        csi_codes::SGR_FG_BLACK..=csi_codes::SGR_FG_WHITE => {
            performer.ofs_buf.set_foreground_color(param);
        }
        csi_codes::SGR_FG_DEFAULT => {
            performer.ofs_buf.reset_foreground_color();
        }
        csi_codes::SGR_BG_BLACK..=csi_codes::SGR_BG_WHITE => {
            performer.ofs_buf.set_background_color(param);
        }
        csi_codes::SGR_BG_DEFAULT => {
            performer.ofs_buf.reset_background_color();
        }
        csi_codes::SGR_FG_BRIGHT_BLACK..=csi_codes::SGR_FG_BRIGHT_WHITE => {
            performer.ofs_buf.set_foreground_color(param);
        }
        csi_codes::SGR_BG_BRIGHT_BLACK..=csi_codes::SGR_BG_BRIGHT_WHITE => {
            performer.ofs_buf.set_background_color(param);
        }
        _ => {
            // Ignore other unsupported SGR parameters:
            // - SGR 10-21, 26, 30-36, 38-47, 53-60, etc. (reserved/rarely-used)
            // - SGR 38/48 without 5/2 mode (malformed; extended colors handled upstream)
        }
    }
}

/// Handle SGR (Select Graphic Rendition) parameters.
///
/// This function processes SGR parameters and applies them to the offscreen buffer.
/// It supports:
/// - Basic text attributes (bold, italic, underline, etc.)
/// - 16-color ANSI colors (30-37, 40-47, 90-97, 100-107)
/// - 256-color palette (ESC[38:5:nm or ESC[48:5:nm)
/// - RGB true color (ESC[38:2:r:g:bm or ESC[48:2:r:g:bm)
pub fn set_graphics_rendition(performer: &mut AnsiToOfsBufPerformer, params: &Params) {
    let mut idx = 0;
    while let Some(param_slice) = params.extract_nth_many_raw(idx) {
        // Check for extended color sequences first (they consume multiple positions).
        if let Some(color_seq) =
            csi_codes::SgrColorSequence::parse_from_raw_slice(param_slice)
        {
            // Unified method handles routing to foreground/background automatically.
            performer.ofs_buf.apply_extended_color_sequence(color_seq);
        } else if let Some(&first_param) = param_slice.first() {
            // Handle single parameters (existing behavior for basic SGR codes).
            apply_sgr_param(performer, first_param);
        }
        idx += 1;
    }
}
