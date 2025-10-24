// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Render operations type system for the TUI rendering pipeline.
//!
//! # You Are Here
//!
//! This module defines the **core data types** that flow through all stages of the
//! rendering pipeline. These types are the "lingua franca" that all stages speak.
//!
//! ```text
//! [S1: App/Component] → [S2: Pipeline] → [S3: Compositor] →
//! [S4: Backend Converter] → [S5: Backend Executor] → [S6: Terminal]
//!                ▲
//!     ┌──────────┴─────────────────┐
//!     │ YOU ARE HERE: Defines      │
//!     │ RenderOpIR, RenderOpOutput │
//!     └────────────────────────────┘
//! ```
//!
//! **Input**: Component code produces [`RenderOpIR`] operations
//! **Output**: These types are consumed by all downstream stages
//! **Role**: Define the contract between all layers
//!
//! > **For the complete rendering architecture**, see [`super`] (parent module) and
//! > [`super::super::README.md`].
//!
//! ## What This Module Provides
//!
//! This module provides a type-safe, two-stage rendering system that separates
//! high-level component operations (IR) from low-level terminal operations (Output):
//!
//! - **[`RenderOpIR`]** - Operations with built-in clipping info (used by components)
//! - **[`RenderOpOutput`]** - Post-clipping operations (used by backend)
//! - **[`RenderOpCommon`]** - 27 shared operations available in both contexts
//! - **[`RenderOpsLocalData`]** - Optimization state to avoid redundant terminal commands
//! - **[`RenderOpCommonExt`]** - Ergonomic helper trait for creating common operations
//!
//! # Type Safety Benefits
//!
//! The split between `RenderOpIR` and `RenderOpOutput` provides compile-time guarantees:
//!
//! - **Component code** uses `RenderOpIR` with clipping-aware operations
//! - **Backend code** uses `RenderOpOutput` with post-clipping operations
//! - **Impossible to mix** IR and Output operations incorrectly
//!
//! # Module Organization
//!
//! This module follows the **private modules with public re-exports** pattern for a clean
//! API:
//!
//! - `render_op_common` - 27 operations shared between IR and Output layers
//! - `render_op_ir` - Intermediate representation for component/app layer
//! - `render_op_output` - Terminal output operations for backend layer
//! - `render_op_common_ext` - Ergonomic helper trait for both IR and Output
//! - `render_op_local_data` - State tracking for render optimization
//! - `render_op_flush` - Terminal output flushing control
//!
//! All types are re-exported at the module level for ergonomic imports.

// Re-export TERMINAL_LIB_BACKEND constant from parent module.
pub use super::TERMINAL_LIB_BACKEND;

// Private modules - implementation details.
mod render_op_common;
mod render_op_common_ext;
mod render_op_flush;
mod render_op_ir;
mod render_op_local_data;
mod render_op_output;

// Public re-exports - stable API surface.
pub use render_op_common::*;
pub use render_op_common_ext::*;
pub use render_op_flush::*;
pub use render_op_ir::*;
pub use render_op_local_data::*;
pub use render_op_output::*;
// Re-export trait for formatting debug output (used by backend implementations).
use std::fmt::{Formatter, Result};

/// Trait for formatting [`RenderOpCommon`] instances in debug output.
///
/// This trait abstracts debug formatting logic, allowing different
/// terminal backends to provide their own specialized debug representations
/// of common render operations.
pub trait DebugFormatRenderOp {
    /// Formats the `RenderOpCommon` for debug output.
    ///
    /// # Errors
    ///
    /// Returns a formatting error if writing to the formatter fails.
    fn fmt_debug(&self, this: &RenderOpCommon, f: &mut Formatter<'_>) -> Result;
}
