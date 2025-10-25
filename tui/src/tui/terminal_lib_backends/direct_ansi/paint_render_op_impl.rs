// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`DirectAnsi`] implementation of the [`RenderOpPaint`] trait
//!
//! This implements the [`RenderOpPaint`] trait to execute all [`RenderOpOutput`] variants
//! using [`AnsiSequenceGenerator`]. It tracks cursor position and colors to skip
//! redundant ANSI sequences for optimization.
//!
//! [`RenderOpPaint`]: crate::RenderOpPaint

use super::AnsiSequenceGenerator;
use crate::{LockedOutputDevice, RenderOpFlush, RenderOpOutput, RenderOpPaint,
            RenderOpsLocalData, Size};

/// Implements [`RenderOpPaint`] trait using direct ANSI sequence generation
///
/// [`RenderOpPaint`]: crate::RenderOpPaint
#[derive(Debug)]
pub struct RenderOpImplDirectAnsi;

impl RenderOpPaint for RenderOpImplDirectAnsi {
    fn paint(
        &mut self,
        skip_flush: &mut bool,
        _render_op: &RenderOpOutput,
        _window_size: Size,
        _render_local_data: &mut RenderOpsLocalData,
        _locked_output_device: LockedOutputDevice<'_>,
        is_mock: bool,
    ) {
        // Mock mode is handled at the OutputDevice level.
        // This function always executes fully; the I/O boundary decides whether
        // output is actually written.
        let _ = is_mock;

        // TODO: Implement all RenderOpOutput variants
        // This is a stub for now - implementation happens in Step 3.3
        *skip_flush = false;
    }
}

impl RenderOpFlush for RenderOpImplDirectAnsi {
    fn flush(&mut self, locked_output_device: LockedOutputDevice<'_>) {
        locked_output_device
            .flush()
            .expect("Failed to flush output device");
    }

    fn clear_before_flush(&mut self, locked_output_device: LockedOutputDevice<'_>) {
        let clear_sequence = AnsiSequenceGenerator::clear_screen();
        locked_output_device
            .write_all(clear_sequence.as_bytes())
            .expect("Failed to write clear screen sequence");
        locked_output_device
            .flush()
            .expect("Failed to flush output device");
    }
}
