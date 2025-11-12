// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Render operations type system for the TUI rendering pipeline.
//!
//! # You Are Here
//!
//! This module defines the **core data types** that flow through all stages of the
//! rendering pipeline. These types are the "lingua franca" that all stages speak.
//!
//! ```text
//! [Stage 1: App/Component]
//!   ↓
//! [Stage 2: Pipeline]
//!   ↓
//! [Stage 3: Compositor]
//!   ↓
//!   ┌─────────────────────────────┐
//!   │ YOU ARE HERE: Defines      │
//!   │ RenderOpIR, RenderOpOutput │
//!   └─────────────────────────────┘
//!   ↓
//! [Stage 4: Backend Converter]
//!   ↓
//! [Stage 5: Backend Executor]
//!   ↓
//! [Stage 6: Terminal]
//! ```
//!
//! **Input**: Component code produces [`RenderOpIR`] operations
//! **Output**: These types are consumed by all downstream stages
//! **Role**: Define the contract between all layers
//!
//! > **For the complete rendering architecture**, see [`mod@super`] (parent module).
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
//! The split between [`RenderOpIR`] and [`RenderOpOutput`] provides compile-time
//! guarantees:
//!
//! - **Component code** uses [`RenderOpIR`] with clipping-aware operations
//! - **Backend code** uses [`RenderOpOutput`] with post-clipping operations
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
//!
//! # Architectural Patterns Used Across Submodules
//!
//! ## The "You Are Here" Diagram
//!
//! Each submodule includes a simplified "You Are Here" diagram showing where in the
//! rendering pipeline that module's types are used. This helps orient developers
//! when reading individual files. The complete diagram above shows the full context.
//!
//! ## Semantic Boundaries
//!
//! The design enforces critical architectural boundaries through the type system:
//!
//! - **[`RenderOpIR`] cannot be executed** - must flow through Compositor first
//! - **[`RenderOpOutput`] cannot be created by components** - only by backend converters
//! - **[`RenderOpsExec`] trait only on Output** - prevents bypassing the Compositor
//!
//! These boundaries ensure data flows correctly through the pipeline and guarantees
//! (like text clipping and style application) are maintained.
//!
//! ## Ergonomic Factory Methods
//!
//! The [`RenderOpCommonExt`] trait provides factory methods for common operations,
//! available on both IR and Output types. This avoids repetitive wrapping like
//! `RenderOpIR::Common(RenderOpCommon::MoveCursorPositionAbs(pos))` in favor of
//! `RenderOpIR::move_cursor(pos)`.

// Skip rustfmt for rest of file.
// https://stackoverflow.com/a/75910283/2085356
#![cfg_attr(rustfmt, rustfmt_skip)]

// Private modules - implementation details.
mod render_op_common;
mod render_op_common_ext;
mod render_op_debug_format;
mod render_op_flush;
mod render_op_output;
mod render_op_paint;
mod render_ops_exec;
mod render_ops_local_data;

// Module is public only when building documentation or tests.
// This allows rustdoc links to work while keeping it private in release builds.
#[cfg(any(test, doc))]
pub mod render_op_ir;
#[cfg(not(any(test, doc)))]
mod render_op_ir;

// Public re-exports - stable API surface.
pub use render_op_common::*;
pub use render_op_common_ext::*;
pub use render_op_debug_format::*;
pub use render_op_flush::*;
pub use render_op_ir::*;
pub use render_op_output::*;
pub use render_op_paint::*;
pub use render_ops_exec::*;
pub use render_ops_local_data::*;
