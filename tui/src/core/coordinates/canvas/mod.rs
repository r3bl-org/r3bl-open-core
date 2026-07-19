// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

#![rustfmt::skip]

//! # Canvas and Viewport Concept
//!
//! In terminal buffer architectures, the underlying storage manages a potentially
//! infinite 2D space (the **Canvas**), which includes all scrollback history lines.
//! However, only a subset of this buffer is visible on screen at any given time (the
//! **Viewport**).
//!
//! <div class="warning">
//!
//! There is no specific `Canvas` struct or trait in this crate. "Canvas" is a conceptual
//! mental model representing the actual backing data structure (such as [`EditorContent`]
//! for text editors, or types implementing [`CanvasStorage`] like [`GrowableBuffer`] and
//! [`Flat2DArray`] for terminal output). This module provides the coordinate types (e.g.,
//! [`CPos`]) to safely address absolute positions within those storage structures,
//! and provides a concrete [`Viewport`] struct to act as the sliding camera over them.
//!
//! </div>
//!
//! This module defines the Canvas and Viewport concept, ASCII visual representation, and
//! coordinate domain taxonomy across the crate.
//!
//! ```text
//!               1         2         3         4         5
//!     01234567890123456789012345678901234567890123456789012
//!    ┌─────────────────────────────────────────────────────┐  ← Canvas Top (Row 0)
//!   0│                                                     │  ▲
//!   1│                                                     │  │ get_history_len()
//!   2│               (Scrolled Off Lines)                  │  │ == pos.row_index
//!   3│                                                     │  ▼
//! ► 4│               ┌───────────────────────────┐         │  ◄ Viewport Top (Row 4)
//!   5│               │ ▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒ │         │  ▲
//!   6│               │ ▒▒▒▒▒▒▒▒▒Viewport▒▒▒▒▒▒▒▒ │         │  │ size()
//!   7│               │ ▒▒▒▒▒▒Visible Screen▒▒▒▒▒ │         │  │ .row_height
//!   8│               │ ▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒ │         │  ▼
//!   9│               └───────────────────────────┘         │  ← Viewport Bottom
//!    └─────────────────────────────────────────────────────┘
//! ```
//!
//! # Architecture & Domain Flow
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                            Viewport Domain (u16)                            │
//! │  - VPPos { row_index, col_index }                                           │
//! │  - Size { col_width, row_height }                                           │
//! │  - Used by RenderOp, MouseInput, VT-100 Performer                           │
//! └──────────────────────────────────────┬──────────────────────────────────────┘
//!                                        │
//!                         OfsBuf API Boundary (u16 -> usize)
//!                                        │
//! ┌──────────────────────────────────────▼──────────────────────────────────────┐
//! │                    CanvasStorage Trait Abstraction (types.rs)               │
//! │  - get_row(VPRow) -> Option<&[PixelChar]>                                   │
//! │  - try_pan_viewport_to(CPos)                                                │
//! │  - get_viewport() -> Viewport                                               │
//! └───────────────────┬─────────────────────────────────────┬───────────────────┘
//!                     │                                     │
//!        Flat2DArray Adapter                               GrowableBuffer
//! (Fixed-Size Screen Backend)                     (Infinite Scrollback Backend)
//!                     │                                     │
//! ┌───────────────────▼─────────────────┐   ┌───────────────▼───────────────────┐
//! │     Flat2DArray (Canvas Domain)     │   │   GrowableBuffer (Canvas Domain)  │
//! │  - Height/Width: CHeight/CWidth     │   │   VecDeque<PixelCharLine> (usize) │
//! │  - Storage Index: CanvasRow (usize) │   │   - viewport: Viewport            │
//! │  - Fixed origin: CPos(0, 0)         │   │   - origin_pos: CPos (usize)      │
//! └─────────────────────────────────────┘   └───────────────────────────────────┘
//! ```
//!
//! # Coordinate Taxonomy & Disambiguation
//!
//! To prevent subtle bugs where a Viewport-relative index (`[0..row_height)`) is
//! accidentally passed to a Canvas-absolute operation or vice versa, the types in this
//! module wrap underlying coordinate types using the decorator and newtype patterns:
//!
//! - **Viewport Coordinates (Decorator Pattern)** (visible screen window space):
//!   - Positions: [`VPRow`], [`VPCol`], [`VPPos`]
//!   - Sizes: [`VPHeight`], [`VPWidth`]
//!
//! - **Canvas Coordinates (Newtype Pattern)** (absolute storage buffer space):
//!   - Positions: [`CRow`], [`CCol`], [`CPos`]
//!   - Sizes: [`CHeight`], [`CWidth`], [`CSize`]
//!
//! # Design Decision: Decorator vs. Newtype Pattern Rationale
//!
//! This module employs both the **decorator pattern** (for Viewport coordinates) and the
//! **newtype pattern** (for Canvas coordinates) to wrap underlying numeric types.
//!
//! ## Domain Type Safety
//!
//! The fundamental value of wrapping numeric types in this codebase is compile-time type
//! safety. Both patterns enforce strict domain disambiguation. Passing a
//! viewport-relative index where a canvas-absolute index is required (or passing a canvas
//! row where a canvas column is required) causes a compile-time type error. This can be
//! done with both the decorator and newtype patterns.
//!
//! ## Achieve Domain Type Safety by Encapsulation or Delegation
//!
//! To achieve this type safety we differentiate between the following wrapping strategies
//! based on their intent:
//!
//! - **Newtype (Encapsulation)**: Used when wrapping raw primitives (like [`usize`]). The
//!   goal is *restriction*. We hide the raw primitive because its native API allows
//!   arbitrary and potentially unsafe math. We explicitly avoid implementing [`Deref`],
//!   forcing ourselves to manually build up a safe, domain-specific API from scratch by
//!   implementing the traits from [`numeric_value`] (e.g., [`NumericValue`]) directly on
//!   the wrapper (e.g., [`CRow`]).
//!
//! - **Decorator (Delegation)**: Used when wrapping a type that is *already* a safe
//!   domain type (like [`VPRow`] or [`VPPos`]). Technically this is also a newtype.
//!   However, the goal is *layering semantic context* (e.g., this is a "Viewport"
//!   position, or a "Scroll" offset). Because the inner type is already fully modeled
//!   with traits like [`NumericValue`], we don't want to hide it or rewrite those
//!   implementations. Instead, we implement [`Deref`] to transparently delegate to the
//!   inner type, gaining all its rich behaviors for free.
//!
//! ## Examples of Decorator vs Newtype Pattern Usage
//!
//! ### Viewport Coordinates (Decorator Pattern)
//!
//! Viewport types (e.g., [`VPRow`]) wrap the 16-bit primitives ([`VPRow`],
//! [`VPCol`]). We use the decorator pattern here because:
//!
//! 1. **Zero-Boilerplate Trait Delegation via [`Deref`] / [`DerefMut`]**: Because the
//!    inner type ([`VPRow`]) already implements complex domain traits (like
//!    `NumericValue`), the decorator struct (e.g. [`VPRow(pub VPRow)`])
//!    simply implements [`Deref`]. This transparently forwards arithmetic, bounds
//!    checking, and formatting directly to the inner type via Rust's implicit deref
//!    coercion.
//!
//! 2. **Reuse of Existing Infrastructure**: By leveraging the underlying types generated
//!    by [`generate_index_type_impl!`] rather than defining separate primitive types, all
//!    pre-existing trait implementations on the 16-bit primitives are preserved.
//!
//! ### Canvas Coordinates (Newtype Pattern)
//!
//! Canvas types (e.g., [`CRow`]) wrap 64-bit [`usize`] primitives directly.
//! They use the newtype pattern rather than decorating 16-bit primitives to allow canvas
//! storage dimensions to easily exceed the 65,535 line limit of standard terminal APIs
//! (which typically use [`u16`] for screen coordinates).
//!
//! [`numeric_value`]: mod@crate::numeric_value
//! [`Add`]: std::ops::Add
//! [`ArrayBoundsCheck`]: crate::core::ArrayBoundsCheck
//! [`CCol`]: crate::CCol
//! [`CPos`]: crate::CPos
//! [`CRow`]: crate::CRow
//! [`VPCol`]: crate::VPCol
//! [`CursorBoundsCheck`]: crate::CursorBoundsCheck
//! [`Deref`]: std::ops::Deref
//! [`DerefMut`]: std::ops::DerefMut
//! [`Display`]: std::fmt::Display
//! [`generate_index_type_impl!`]: crate::generate_index_type_impl
//! [`VPPos`]: crate::core::VPPos
//! [`VPRow`]: crate::VPRow
//! [`Sub`]: std::ops::Sub
//! [`ViewportBoundsCheck`]: crate::ViewportBoundsCheck
//! [`Viewport`]: crate::Viewport
//! [`EditorContent`]: crate::EditorContent
//! [`GrowableBuffer`]: crate::tui::GrowableBuffer
//! [`Flat2DArray`]: crate::core::Flat2DArray
//! [`CanvasStorage`]: crate::CanvasStorage
//! [`pty_mux`]: mod@crate::core::pty::pty_mux
//! [`VPHeight`]: crate::VPHeight
//! [`VPWidth`]: crate::VPWidth
//! [`NumericValue`]: crate::core::NumericValue
//! [`usize`]: prim@usize

// Attach (Private).
mod canvas_coords;
mod canvas_range_ext;
mod viewport;
mod canvas_camera_ext;
mod canvas_projection_ext;

// Re-export (Public).
pub use canvas_coords::*;
pub use canvas_range_ext::*;
pub use viewport::*;
pub use canvas_camera_ext::*;
pub use canvas_projection_ext::*;