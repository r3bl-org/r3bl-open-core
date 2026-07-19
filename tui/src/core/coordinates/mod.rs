// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Type-safe coordinate systems for terminal UI applications.
//!
//! This module provides a comprehensive type system for working with coordinates,
//! dimensions, and positions across different domains in terminal applications. The
//! design emphasizes **type safety**, **explicit conversions**, and **preventing
//! off-by-one errors** through carefully structured abstractions.
//!
//! # Architecture Overview
//!
//! The coordinate system is organized into six domains, each serving a specific purpose:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    primitives/                                  │
//! │                   ChUnit (u16)                                  │
//! │               Foundation primitive                              │
//! └──────────────────┬──────────────────────────────────────────────┘
//!                    │
//!         ┌──────────┴──────────┬──────────────┬──────────────┐
//!         │                     │              │              │
//! ┌───────▼───────┐    ┌────────▼────┐  ┌──────▼───┐  ┌───────▼────────┐
//! │viewport_coords│    │ vt_100_     │  │   byte   │  │ percent_spec   │
//! │               │    │ ansi_coords │  │          │  │                │
//! │ Index, Length │    │ 1-based     │  │  usize   │  │  Percentage    │
//! │ 0-based ChUnit│    │ NonZeroU16  │  │  based   │  │     spec       │
//! └───────────────┘    └─────────────┘  └──────────┘  └────────────────┘
//!         │                     │              │              │
//!         └──────────┬──────────┴──────────────┴──────────────┘
//!                    │
//!         ┌──────────▼──────────┐
//!         │   bounds_check/     │
//!         │  Type-safe bounds   │
//!         │   checking traits   │
//!         └─────────────────────┘
//! ```
//!
//! # Design Philosophy
//!
//! ## 1. **Explicit Coordinate Systems**
//!
//! The codebase uses three distinct coordinate systems that must never be mixed:
//!
//! | System                  | Base      | Primitive              | Use Case                                        |
//! | ----------------------- | --------- | ---------------------- | ----------------------------------------------- |
//! | **Viewport**            | 0-based   | [`ChUnit`] ([`u16`])   | Internal app logic, array indexing, crossterm   |
//! | **[`VT-100`] [`ANSI`]** | 1-based   | [`NonZeroU16`]         | [`ANSI`] escape sequence parsing only           |
//! | **Byte**                | 0-based   | [`usize`]              | [`UTF-8`] string/buffer byte positions          |
//!
//! **Why this matters**: [`ANSI`] escape sequences like `ESC[5;10H` use 1-based indexing
//! where `(1,1)` is the top-left corner. Internal data structures and crossterm use
//! 0-based indexing where `(0,0)` is top-left. Byte positions must use [`usize`] for
//! string slicing. Mixing these causes off-by-one errors.
//!
//! ## 2. **Type Safety Over Convenience**
//!
//! Instead of using raw [`usize`] or [`u16`] everywhere, each coordinate type is wrapped
//! in a newtype that:
//! - Prevents mixing incompatible types (e.g., can't add [`VPCol`] to [`VPHeight`])
//! - Makes conversions explicit (e.g., [`term_row.to_zero_based()`])
//! - Provides domain-specific operations (e.g., [`vp_index.overflows(vp_length)`])
//!
//! ## 3. **Index vs Length Distinction**
//!
//! The type system enforces the semantic difference between positions and sizes:
//!
//! ```text
//!              ┌──────── Length=10 (1-based)───────┐
//!              │                                   │
//!            ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
//!            │ A │ B │ C │ D │ E │ F │ G │ H │ I │ J │
//!            └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘
//! Index:       0   1   2   3   4   5   6   7   8   9
//! (0-based)    ↑                   ↑               ↑
//!          index 0              index 5          index 9
//!        (first position)   (middle position)   (last position)
//! ```
//!
//! - **Index types** ([`VPCol`], [`VPRow`]): 0-based positions for array access
//! - **Length types** ([`VPWidth`], [`VPHeight`]): 1-based counts/sizes
//! - **Arithmetic**: [`VPIndex`] + [`VPLength`] = [`VPIndex`], [`VPIndex`] - [`VPLength`] =
//!   [`VPIndex`]
//!
//! # When To Use What: Quick Reference
//!
//! ## Coordinate Type Selection
//!
//! | Your Task                                             | Use These Types                                     |
//! | ----------------------------------------------------- | --------------------------------------------------- |
//! | **Index into [`OfsBuf`] or [`ZeroCopyGapBuffer`]**    | [`VPCol`], [`VPRow`], [`VPPos`]                     |
//! | **Send cursor commands via crossterm**                | [`VPCol`], [`VPRow`] (convert to [`u16`])           |
//! | **Parse [`VT-100`] [`ANSI`] escape sequences**        | [`TermRow`], [`TermCol`]                            |
//! | **Work with [`UTF-8`] byte positions in strings**     | [`ByteIndex`], [`ByteLength`], [`ByteOffset`]       |
//! | **Store dimensions/sizes**                            | [`VPWidth`], [`VPHeight`], [`VPSize`]               |
//! | **Track cursor or caret position**                    | [`VPPos`], [`VPCaret`], [`CCaret`]                  |
//! | **Specify layout constraints or percentage metrics**  | [`Pc`], [`ReqSizePc`]                               |
//!
//! # Common Workflows
//!
//! This module provides building blocks that work together across different coordinate
//! systems. For detailed API usage and type-specific examples, consult the individual
//! type and module documentation:
//!
//! | Workflow                                        | Primary Types                                             | Documentation Links            |
//! | ----------------------------------------------- | --------------------------------------------------------- | ------------------------------ |
//! | **[`VT-100`] [`ANSI`] parsing → buffer access** | [`TermRow`], [`TermCol`] → [`VPRow`], [`VPCol`]           | [`vt_100_ansi_coords`]         |
//! | **Viewport positioning & manipulation**         | [`VPPos`], [`VPSize`], [`VPRow`], [`VPCol`]               | [`viewport_coords`]            |
//! | **Type-safe bounds checking**                   | [`ArrayBoundsCheck`], [`CursorBoundsCheck`]               | [`bounds_check`]               |
//! | **[`UTF-8`] byte-level operations**             | [`ByteIndex`], [`ByteLength`], [`ByteOffset`]             | [`byte`]                       |
//! | **Terminal output (crossterm)**                 | [`VPRow::as_u16()`], [`VPCol::as_u16()`]                  | [`VPPos`], [`viewport_coords`] |
//!
//! **Example: Complete [`VT-100`] to buffer workflow**
//! ```rust
//! use r3bl_tui::{TermRow, TermCol, VPRow, VPCol};
//! use std::num::NonZeroU16;
//!
//! // 1. Parse ANSI sequence "ESC[5;10H"
//! let term_row = TermRow::from_raw_non_zero_value(NonZeroU16::new(5).expect("conversion error"));
//! let term_col = TermCol::from_raw_non_zero_value(NonZeroU16::new(10).expect("conversion error"));
//!
//! // 2. Convert to 0-based viewport coordinates
//! let vp_row: VPRow = term_row.to_zero_based(); // vp_row(4)
//! let vp_col: VPCol = term_col.to_zero_based(); // vp_col(9)
//!
//! // 3. Now safe for viewport array indexing: buffer[vp_row][vp_col]
//! ```
//!
//! # Coordinate System Conversions
//!
//! ```text
//! VT-100 ANSI (1-based)
//!   TermRow(5), TermCol(10)
//!         │
//!         │ .to_zero_based()
//!         ▼
//! Viewport Coords (0-based)
//!   VPRow(4), VPCol(9)
//!         │
//!         │ .as_u16()
//!         ▼
//! Crossterm (0-based u16)
//!   MoveTo(9, 4)
//! ```
//!
//! **Key conversion methods:**
//! - [`TermRow::to_zero_based()`] → [`VPRow`] (1-based → 0-based)
//! - [`VPRow::as_u16()`] → [`u16`] (for crossterm)
//! - [`TermRow::from_zero_based(VPRow)`] → [`TermRow`] (0-based → 1-based)
//!
//! # Submodule Organization
//!
//! - **[`primitives`]**: Foundation primitive type ([`ChUnit`]) used by all other modules
//! - **[`viewport_coords`]**: 0-based coordinates for internal app logic and buffer
//!   indexing. Includes generic types ([`VPIndex`], [`VPLength`]) and concrete types
//!   ([`VPCol`], [`VPRow`], [`VPWidth`], [`VPHeight`], [`VPPos`], [`VPSize`])
//! - **[`vt_100_ansi_coords`]**: 1-based coordinates for [`VT-100`] [`ANSI`] escape sequence
//!   parsing
//! - **[`byte`]**: Byte-level coordinates for [`UTF-8`] text processing
//! - **[`percent_spec`]**: Percentage types ([`Pc`], [`ReqSizePc`]) for UI layout
//!   specifications and telemetry metrics
//! - **[`bounds_check`]**: Type-safe bounds checking traits and utilities
//!
//! [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
//! [`ArrayBoundsCheck`]: bounds_check::ArrayBoundsCheck
//! [`ArrayOverflowResult`]: bounds_check::ArrayOverflowResult
//! [`bounds_check`]: bounds_check
//! [`byte`]: byte
//! [`ByteIndex`]: byte::ByteIndex
//! [`ByteIndexRangeExt`]: byte::ByteIndexRangeExt
//! [`ByteLength`]: byte::ByteLength
//! [`ByteOffset`]: byte::ByteOffset
//! [`CCaret`]: viewport_coords::CCaret
//! [`ChUnit`]: primitives::ChUnit
//! [`CursorBoundsCheck`]: bounds_check::CursorBoundsCheck
//! [`NonZeroU16`]: std::num::NonZeroU16
//! [`OfsBuf`]: crate::tui::OfsBuf
//! [`Pc`]: crate::Pc
//! [`percent_spec`]: percent_spec
//! [`primitives`]: primitives
//! [`RangeBoundsExt::check_range_is_valid_for_length()`]: bounds_check::RangeBoundsExt::check_range_is_valid_for_length
//! [`RangeBoundsExt`]: bounds_check::RangeBoundsExt
//! [`ReqSizePc`]: crate::ReqSizePc
//! [`term_row.to_zero_based()`]: vt_100_ansi_coords::TermRow::to_zero_based
//! [`TermCol`]: vt_100_ansi_coords::TermCol
//! [`TermRow::from_zero_based(VPRow)`]: vt_100_ansi_coords::TermRow::from_zero_based
//! [`TermRow::to_zero_based()`]: vt_100_ansi_coords::TermRow::to_zero_based
//! [`TermRow`]: vt_100_ansi_coords::TermRow
//! [`u16`]: prim@u16
//! [`usize`]: prim@usize
//! [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
//! [`viewport_coords`]: viewport_coords
//! [`ViewportBoundsCheck`]: bounds_check::ViewportBoundsCheck
//! [`vp_index.overflows(vp_length)`]: viewport_coords::VPIndex::overflows
//! [`VPCaret`]: viewport_coords::VPCaret
//! [`VPCol`]: viewport_coords::VPCol
//! [`VPHeight`]: viewport_coords::VPHeight
//! [`VPIndex::check_viewport_bounds()`]: bounds_check::ViewportBoundsCheck::check_viewport_bounds
//! [`VPIndex::overflows()`]: viewport_coords::VPIndex::overflows
//! [`VPIndex`]: viewport_coords::VPIndex
//! [`VPLength::check_cursor_position_bounds()`]: bounds_check::CursorBoundsCheck::check_cursor_position_bounds
//! [`VPLength`]: viewport_coords::VPLength
//! [`VPPos`]: viewport_coords::VPPos
//! [`VPRow`]: viewport_coords::VPRow
//! [`VPSize`]: viewport_coords::VPSize
//! [`VPWidth`]: viewport_coords::VPWidth
//! [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
//! [`vt_100_ansi_coords`]: vt_100_ansi_coords
//! [`ZeroCopyGapBuffer`]: crate::ZeroCopyGapBuffer

#![rustfmt::skip]

// ╔═══════════════════════════════════════════════════════════════════════════╗
// ║                   COORDINATE SYSTEM MODULE ORGANIZATION                   ║
// ║                    (Private modules with public re-exports)               ║
// ║                                                                           ║
// ║ This module follows the pattern from AGENTS.md:                           ║
// ║ - Submodules are kept private (hide internal structure)                   ║
// ║ - Public re-exports provide a flat, stable API surface                    ║
// ║ - Users should import from the flat re-exports, not qualified paths       ║
// ╚═══════════════════════════════════════════════════════════════════════════╝

// Submodule declarations (internal implementation detail).
// Note: These are public to support existing codebase that uses qualified paths.
// New code should avoid importing from qualified paths and instead use the
// public re-exports below.
pub mod bounds_check;
pub mod viewport_coords;
pub mod byte;
pub mod canvas;
pub mod percent_spec;
pub mod primitives;
pub mod vt_100_ansi_coords;

// ═══════════════════════════════════════════════════════════════════════════
// PUBLIC RE-EXPORTS (Flat API Surface - Recommended Way to Import)
// ═══════════════════════════════════════════════════════════════════════════
//
// All public types and traits from submodules are re-exported at this level
// to provide a clean, flat API. Users should import from here.
//
// GOOD:
//    use r3bl_tui::{RowIndex, ColIndex, ViewportBoundsCheck, TermRow, TermCol};
//
// AVOID:
//    use r3bl_tui::core::coordinates::viewport_coords::{VPRow, VPCol};
//    use r3bl_tui::core::coordinates::bounds_check::ViewportBoundsCheck;
//
// ═══════════════════════════════════════════════════════════════════════════

pub use bounds_check::*;
pub use viewport_coords::*;
pub use byte::*;
pub use canvas::*;
pub use percent_spec::*;
pub use primitives::*;
pub use vt_100_ansi_coords::*;

// Rustdoc search link fixes.

#[doc(inline)] // Create doc pages at re-export path so rustdoc search links resolve.
pub use bounds_check::{
    array_bounds_check, cursor_bounds_check, index_ops, length_ops, numeric_value, range,
    result_enums, viewport_bounds_check,
};
#[doc(inline)] // Create doc pages at re-export path so rustdoc search links resolve.
pub use viewport_coords::{
    caret, col_index, col_width, index, length, pos, row_height, row_index, size,
};
