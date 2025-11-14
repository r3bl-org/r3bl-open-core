// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Local state tracking for render operations optimization.
//!
//! # You Are Here: **Stage 5 Optimization Data**
//!
//! This state structure is used by Stage 5 (Backend Executor) for optimization:
//!
//! ```text
//! [Stage 1: App/Component]
//!   ↓
//! [Stage 2: Pipeline]
//!   ↓
//! [Stage 3: Compositor]
//!   ↓
//! [Stage 4: Backend Converter]
//!   ↓
//! [Stage 5: Backend Executor] ← YOU ARE HERE (RenderOpsLocalData used here)
//!   ↓
//! [Stage 6: Terminal]
//! ```
//!
//! <div class="warning">
//!
//! **For the complete 6-stage rendering pipeline with visual diagrams and stage
//! reference table**, see the [rendering pipeline overview].
//!
//! </div>
//!
//! # Purpose
//!
//! Maintains the last known terminal state to avoid sending redundant escape sequences
//! when the state hasn't changed. This optimization significantly reduces the amount of
//! data sent to the terminal.
//!
//! Used by [`crate::PaintRenderOpImplCrossterm`] (Backend Executor) to track cursor
//! position and colors, avoiding redundant commands.
//!
//! [rendering pipeline overview]: mod@crate::terminal_lib_backends#rendering-pipeline-architecture

use crate::{Pos, TuiColor};

/// Local state tracking for render operations optimization.
///
/// Maintains the last known terminal state to avoid sending redundant
/// escape sequences when the state hasn't changed. This significantly
/// reduces the amount of data sent to the terminal.
#[derive(Default, Debug)]
pub struct RenderOpsLocalData {
    /// Current cursor position in the terminal.
    ///
    /// Used to determine if cursor movement commands need to be sent
    /// when rendering at a new position.
    pub cursor_pos: Pos,

    /// Last known foreground color.
    ///
    /// Tracks the current foreground color to avoid sending redundant
    /// color escape sequences when the color hasn't changed.
    pub fg_color: Option<TuiColor>,

    /// Last known background color.
    ///
    /// Tracks the current background color to avoid sending redundant
    /// color escape sequences when the color hasn't changed.
    pub bg_color: Option<TuiColor>,
}
