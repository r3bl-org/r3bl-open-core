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
//! │ buffer_coords │    │ vt_100_     │  │   byte   │  │ percent_spec   │
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
//! | System          | Base    | Primitive            | Use Case                                      |
//! |-----------------|---------|----------------------|-----------------------------------------------|
//! | **Buffer**      | 0-based | [`ChUnit`] ([`u16`]) | Internal app logic, array indexing, crossterm |
//! | **VT-100 ANSI** | 1-based | [`NonZeroU16`]       | ANSI escape sequence parsing only             |
//! | **Byte**        | 0-based | [`usize`]            | UTF-8 string/buffer byte positions            |
//!
//! **Why this matters**: ANSI escape sequences like `ESC[5;10H` use 1-based indexing
//! where `(1,1)` is the top-left corner. Internal data structures and crossterm use
//! 0-based indexing where `(0,0)` is top-left. Byte positions must use [`usize`] for
//! string slicing. Mixing these causes off-by-one errors.
//!
//! ## 2. **Type Safety Over Convenience**
//!
//! Instead of using raw [`usize`] or [`u16`] everywhere, each coordinate type is wrapped
//! in a newtype that:
//! - Prevents mixing incompatible types (e.g., can't add [`ColIndex`] to [`RowHeight`])
//! - Makes conversions explicit (e.g., [`term_row.to_zero_based()`])
//! - Provides domain-specific operations (e.g., [`index.overflows(length)`])
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
//! - **Index types** ([`ColIndex`], [`RowIndex`]): 0-based positions for array access
//! - **Length types** ([`ColWidth`], [`RowHeight`]): 1-based counts/sizes
//! - **Arithmetic**: [`Index`] + [`Length`] = [`Index`], [`Index`] - [`Length`] =
//!   [`Index`]
//!
//! # When To Use What: Quick Reference
//!
//! ## Coordinate Type Selection
//!
//! | Your Task                                                    | Use These Types                                 |
//! |--------------------------------------------------------------|-------------------------------------------------|
//! | **Index into [`OffscreenBuffer`] or [`ZeroCopyGapBuffer`]**  | [`ColIndex`], [`RowIndex`], [`Pos`]             |
//! | **Send cursor commands via crossterm**                       | [`ColIndex`], [`RowIndex`] (convert to [`u16`]) |
//! | **Parse VT-100 ANSI escape sequences**                       | [`TermRow`], [`TermCol`]                        |
//! | **Work with UTF-8 byte positions in strings**                | [`ByteIndex`], [`ByteLength`], [`ByteOffset`]   |
//! | **Store dimensions/sizes**                                   | [`ColWidth`], [`RowHeight`], [`Size`]           |
//! | **Track cursor or caret position**                           | [`Pos`], [`CaretRaw`], [`CaretScrAdj`]          |
//! | **Specify layout constraints or percentage metrics**         | [`Pc`], [`ReqSizePc`]                           |
//!
//! # Common Workflows
//!
//! This module provides building blocks that work together across different coordinate
//! systems. For detailed API usage and type-specific examples, consult the individual
//! type and module documentation:
//!
//! | Workflow                                | Primary Types                                         | Documentation Links        |
//! |-----------------------------------------|-------------------------------------------------------|----------------------------|
//! | **VT-100 ANSI parsing → buffer access** | [`TermRow`], [`TermCol`] → [`RowIndex`], [`ColIndex`] | [`vt_100_ansi_coords`]     |
//! | **Buffer positioning & manipulation**   | [`Pos`], [`Size`], [`RowIndex`], [`ColIndex`]         | [`buffer_coords`]          |
//! | **Type-safe bounds checking**           | [`ArrayBoundsCheck`], [`CursorBoundsCheck`]           | [`bounds_check`]           |
//! | **UTF-8 byte-level operations**         | [`ByteIndex`], [`ByteLength`], [`ByteOffset`]         | [`byte`]                   |
//! | **Terminal output (crossterm)**         | [`RowIndex::as_u16()`], [`ColIndex::as_u16()`]        | [`Pos`], [`buffer_coords`] |
//!
//! **Example: Complete VT-100 to buffer workflow**
//! ```rust
//! use r3bl_tui::{TermRow, TermCol, RowIndex, ColIndex};
//! use std::num::NonZeroU16;
//!
//! // 1. Parse ANSI sequence "ESC[5;10H"
//! let term_row = TermRow::from_raw_non_zero_value(NonZeroU16::new(5).unwrap());
//! let term_col = TermCol::from_raw_non_zero_value(NonZeroU16::new(10).unwrap());
//!
//! // 2. Convert to 0-based buffer coordinates
//! let buffer_row: RowIndex = term_row.to_zero_based(); // RowIndex(4)
//! let buffer_col: ColIndex = term_col.to_zero_based(); // ColIndex(9)
//!
//! // 3. Now safe for array indexing: buffer[buffer_row][buffer_col]
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
//! Buffer Coords (0-based)
//!   RowIndex(4), ColIndex(9)
//!         │
//!         │ .as_u16()
//!         ▼
//! Crossterm (0-based u16)
//!   MoveTo(9, 4)
//! ```
//!
//! **Key conversion methods:**
//! - [`TermRow::to_zero_based()`] → [`RowIndex`] (1-based → 0-based)
//! - [`RowIndex::as_u16()`] → [`u16`] (for crossterm)
//! - [`TermRow::from_zero_based(RowIndex)`] → [`TermRow`] (0-based → 1-based)
//!
//! # Submodule Organization
//!
//! - **[`primitives`]**: Foundation primitive type ([`ChUnit`]) used by all other modules
//! - **[`buffer_coords`]**: 0-based coordinates for internal app logic and buffer
//!   indexing. Includes generic types ([`Index`], [`Length`]) and concrete types
//!   ([`ColIndex`], [`RowIndex`], [`ColWidth`], [`RowHeight`], [`Pos`], [`Size`])
//! - **[`vt_100_ansi_coords`]**: 1-based coordinates for VT-100 ANSI escape sequence
//!   parsing
//! - **[`byte`]**: Byte-level coordinates for UTF-8 text processing
//! - **[`percent_spec`]**: Percentage types ([`Pc`], [`ReqSizePc`]) for UI layout
//!   specifications and telemetry metrics
//! - **[`bounds_check`]**: Type-safe bounds checking traits and utilities
//!
//! [`ChUnit`]: primitives::ChUnit
//! [`Index`]: buffer_coords::Index
//! [`Length`]: buffer_coords::Length
//! [`ColIndex`]: buffer_coords::ColIndex
//! [`RowIndex`]: buffer_coords::RowIndex
//! [`ColWidth`]: buffer_coords::ColWidth
//! [`RowHeight`]: buffer_coords::RowHeight
//! [`Pos`]: buffer_coords::Pos
//! [`Size`]: buffer_coords::Size
//! [`CaretRaw`]: buffer_coords::CaretRaw
//! [`CaretScrAdj`]: buffer_coords::CaretScrAdj
//! [`TermRow`]: vt_100_ansi_coords::TermRow
//! [`TermCol`]: vt_100_ansi_coords::TermCol
//! [`ByteIndex`]: byte::ByteIndex
//! [`ByteLength`]: byte::ByteLength
//! [`ByteOffset`]: byte::ByteOffset
//! [`ByteIndexRangeExt`]: byte::ByteIndexRangeExt
//! [`Pc`]: crate::Pc
//! [`ReqSizePc`]: crate::ReqSizePc
//! [`ArrayBoundsCheck`]: bounds_check::ArrayBoundsCheck
//! [`CursorBoundsCheck`]: bounds_check::CursorBoundsCheck
//! [`ViewportBoundsCheck`]: bounds_check::ViewportBoundsCheck
//! [`RangeBoundsExt`]: bounds_check::RangeBoundsExt
//! [`ArrayOverflowResult`]: bounds_check::ArrayOverflowResult
//! [`primitives`]: primitives
//! [`buffer_coords`]: buffer_coords
//! [`vt_100_ansi_coords`]: vt_100_ansi_coords
//! [`byte`]: byte
//! [`percent_spec`]: percent_spec
//! [`bounds_check`]: bounds_check
//! [`OffscreenBuffer`]: crate::OffscreenBuffer
//! [`ZeroCopyGapBuffer`]: crate::ZeroCopyGapBuffer
//! [`TermRow::to_zero_based()`]: vt_100_ansi_coords::TermRow::to_zero_based
//! [`Index::overflows()`]: buffer_coords::Index::overflows
//! [`Length::check_cursor_position_bounds()`]: bounds_check::CursorBoundsCheck::check_cursor_position_bounds
//! [`Index::check_viewport_bounds()`]: bounds_check::ViewportBoundsCheck::check_viewport_bounds
//! [`RangeBoundsExt::check_range_is_valid_for_length()`]: bounds_check::RangeBoundsExt::check_range_is_valid_for_length
//! [`NonZeroU16`]: std::num::NonZeroU16
//! [`usize`]: prim@usize
//! [`u16`]: prim@u16
//! [`term_row.to_zero_based()`]: vt_100_ansi_coords::TermRow::to_zero_based
//! [`index.overflows(length)`]: buffer_coords::Index::overflows
//! [`TermRow::from_zero_based(RowIndex)`]: vt_100_ansi_coords::TermRow::from_zero_based

// Skip rustfmt for rest of file.
// https://stackoverflow.com/a/75910283/2085356
#![cfg_attr(rustfmt, rustfmt_skip)]

// ╔════════════════════════════════════════════════════════════════════════════╗
// ║                   COORDINATE SYSTEM MODULE ORGANIZATION                   ║
// ║                    (Private modules with public re-exports)               ║
// ║                                                                            ║
// ║ This module follows the pattern from CLAUDE.md:                          ║
// ║ - Submodules are kept private (hide internal structure)                  ║
// ║ - Public re-exports provide a flat, stable API surface                   ║
// ║ - Users should import from the flat re-exports, not qualified paths      ║
// ╚════════════════════════════════════════════════════════════════════════════╝

// Submodule declarations (internal implementation detail).
// Note: These are public to support existing codebase that uses qualified paths.
// New code should avoid importing from qualified paths and instead use the
// public re-exports below.
pub mod bounds_check;
pub mod buffer_coords;
pub mod byte;
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
// ✅ GOOD:
//    use r3bl_tui::{RowIndex, ColIndex, ViewportBoundsCheck, TermRow, TermCol};
//
// ❌ AVOID:
//    use r3bl_tui::core::coordinates::buffer_coords::{RowIndex, ColIndex};
//    use r3bl_tui::core::coordinates::bounds_check::ViewportBoundsCheck;
//
// ═══════════════════════════════════════════════════════════════════════════

pub use bounds_check::*;
pub use buffer_coords::*;
pub use byte::*;
pub use percent_spec::*;
pub use primitives::*;
pub use vt_100_ansi_coords::*;
