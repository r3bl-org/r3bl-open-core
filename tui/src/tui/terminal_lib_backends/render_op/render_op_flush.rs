// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::LockedOutputDevice;

/// Controls the behavior when flushing terminal output.
///
/// Determines whether to simply flush the output buffer or to clear
/// the screen before flushing.
#[derive(Debug, Clone, Copy)]
pub enum FlushKind {
    /// Flush the output buffer without clearing.
    JustFlush,
    /// Clear the screen before flushing the output buffer.
    ClearBeforeFlush,
}

/// Trait for controlling terminal output flushing behavior.
///
/// This trait provides methods to flush pending terminal output and optionally
/// clear the terminal before flushing. Essential for ensuring that render
/// operations are actually displayed on the terminal.
///
/// # You Are Here
///
/// ```text
/// [Stage 1: App/Component] → [Stage 2: Pipeline] → [Stage 3: Compositor] →
/// [Stage 4: Backend Converter] → [Stage 5: Backend Executor] → [Stage 6: Terminal]
///                                                       ↓↓↓↓
///                          Flush trait is called here to display output
/// ```
///
/// See [`crate::render_op`] module documentation for shared architectural patterns
/// and the rendering pipeline overview.
///
/// # Purpose
///
/// Provides methods for managing when and how terminal output is flushed
/// to the screen, along with [`FlushKind`] for different behaviors.
///
/// Used by [`crate::PaintRenderOpImplCrossterm`] (Backend Executor) to control when
/// rendered content is actually displayed to the user.
pub trait RenderOpFlush {
    /// Flushes pending output to the terminal.
    ///
    /// This method ensures that all buffered terminal output is written
    /// and displayed immediately.
    fn flush(&mut self, locked_output_device: LockedOutputDevice<'_>);

    /// Clears the terminal before flushing output.
    ///
    /// This method first clears the terminal screen, then flushes any
    /// pending output. Useful for ensuring a clean display state.
    fn clear_before_flush(&mut self, locked_output_device: LockedOutputDevice<'_>);
}
