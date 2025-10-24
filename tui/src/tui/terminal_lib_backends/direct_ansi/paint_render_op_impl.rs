// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`DirectAnsi`] implementation of the [`PaintRenderOp`] trait
//!
//! This implements the [`PaintRenderOp`] trait to execute all [`RenderOpIR`] variants using
//! [`AnsiSequenceGenerator`]. It tracks cursor position and colors to skip redundant ANSI
//! sequences for optimization.

use super::AnsiSequenceGenerator;
use crate::{Flush, LockedOutputDevice, PaintRenderOp, RenderOpIR, RenderOpsLocalData, Size};

/// Implements [`PaintRenderOp`] trait using direct ANSI sequence generation
#[derive(Debug)]
pub struct RenderOpImplDirectAnsi;

impl PaintRenderOp for RenderOpImplDirectAnsi {
    fn paint(
        &mut self,
        skip_flush: &mut bool,
        _render_op: &RenderOpIR,
        _window_size: Size,
        _render_local_data: &mut RenderOpsLocalData,
        _locked_output_device: LockedOutputDevice<'_>,
        is_mock: bool,
    ) {
        if is_mock {
            return; // Skip rendering in mock mode
        }

        // TODO: Implement all RenderOpIR variants
        // This is a stub for now
        *skip_flush = false;
    }
}

impl Flush for RenderOpImplDirectAnsi {
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
