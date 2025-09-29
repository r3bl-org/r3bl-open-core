// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Comprehensive bounds checking system that eliminates off-by-one errors across
//! diverse programming contexts.
//!
//! This module covers array access, cursor positioning, viewport visibility, and range
//! validation scenarios that commonly cause indexing, positioning and sizing
//! bugs in applications ranging from TUIs to data processing, collections
//! manipulation, and any code working with indices and lengths.
//!
//! It provides this type-safe bounds validation through a two-tier architecture:
//! - foundational traits that provide core operations, and
//! - semantic traits that implement specific use case validation.
//!
//! This architecture embodies several key principles:
//!
//! | Principle        | Description                                                    |
//! |------------------|----------------------------------------------------------------|
//! | Semantic Clarity | Each file in this module has a single, well-defined purpose.   |
//! | Type Safety      | Impossible to make incorrect comparisons at compile time.      |
//! | Performance      | Zero-cost abstractions with compile-time guarantees.           |
//! | Discoverability  | Developers can find what they need based on use case.          |
//! | Maintainability  | Clear boundaries make the system easy to extend.               |
//!
//! ## Core Traits Overview
//!
//! The bounds checking system uses a **two-tier trait architecture**:
//!
//! ### Foundational Traits (Core Operations)
//!
//! These provide the fundamental operations used across all bounds checking patterns:
//!
//! | Trait             | File                 | Key Methods                                                                                                     |
//! |-------------------|----------------------|-----------------------------------------------------------------------------------------------------------------|
//! | [`IndexMarker`]   | [`index_marker.rs`]  | [`convert_to_length()`], [`clamp_to_max_length()`], [`clamp_to_min_index()`], [`clamp_to_range()`] |
//! | [`LengthMarker`]  | [`length_marker.rs`] | [`convert_to_index()`], [`is_overflowed_by()`], [`remaining_from()`], [`clamp_to_max()`]                       |
//!
//! ### Semantic Traits (Use Case Validation)
//!
//! These build on foundational traits to provide specialized validation for specific use
//! cases:
//!
//! | Trait                   | File                    | Key Methods                                                                                                                |
//! |-------------------------|-------------------------|----------------------------------------------------------------------------------------------------------------------------|
//! | [`ArrayBoundsCheck`]    | [`array_bounds.rs`]     | [`overflows()`], [`underflows()`]                                                                                          |
//! | [`CursorBoundsCheck`]   | [`cursor_bounds.rs`]    | [`eol_cursor_position()`], [`is_valid_cursor_position()`], [`clamp_cursor_position()`], [`check_cursor_position_bounds()`] |
//! | [`ViewportBoundsCheck`] | [`viewport_bounds.rs`]  | [`is_in_viewport()`], [`check_viewport_bounds()`]                                                                          |
//! | [`RangeBoundsCheck`]    | [`range_bounds.rs`]     | [`is_valid()`], [`clamp_range_to()`], [`check_index_is_within()`] (supports both [`Range`] and [`RangeInclusive`])        |
//! | [`RangeConversion`]     | [`range_bounds.rs`]     | [`to_exclusive()`] - Convert `RangeInclusive` → `Range` for iteration                                                      |
//!
//! ### Why Import These Traits?
//!
//! In Rust, trait methods are only available when the trait is in scope. You need to
//! import the trait that provides the method you want to use:
//!
//! ```rust
//! use r3bl_tui::{col, width, ArrayBoundsCheck, ArrayOverflowResult}; // ArrayBoundsCheck provides overflows()
//!
//! let index = col(5);
//! let length = width(10);
//!
//! // This works because ArrayBoundsCheck is imported
//! if index.overflows(length) == ArrayOverflowResult::Within { /* safe */ }
//! ```
//!
//! For detailed bounds checking (pattern matching on status), import the semantic trait:
//!
//! ```rust
//! use r3bl_tui::{col, width, ArrayBoundsCheck, ArrayOverflowResult};
//!
//! let index = col(5);
//! let length = width(10);
//!
//! match index.overflows(length) {
//!     ArrayOverflowResult::Within => { /* safe */ }
//!     ArrayOverflowResult::Overflowed => { /* error */ }
//! }
//! ```
//!
//! ## When to Use What
//!
//! The bounds checking system is organized into foundational traits and semantic traits.
//! Use the tables below to quickly find the right trait for your task.
//!
//! ### Foundational Traits (Core Operations)
//!
//! | Task                          | Trait or enum                                               | File                | Key Question                           |
//! |-------------------------------|-------------------------------------------------------------|---------------------|----------------------------------------|
//! | Compare indices to each other | [`IndexMarker`]                                             | [`index_marker.rs`] | "How do indices relate to each other?" |
//! | Work with lengths/sizes       | [`LengthMarker`]                                            | [`length_marker.rs`]| "What can I do with a length value?"   |
//! | Convert between numeric types | [`UnitMarker`]                                             | [`unit_marker.rs`] | "How do I convert to usize/u16?"       |
//! | Status return enum types      | [`ArrayOverflowResult`], [`CursorPositionBoundsStatus`] | [`result_enums.rs`] | "What status types are available?"     |
//!
//! #### When to Use Foundational Traits Directly
//!
//! **📐 Length/space/size calculations & text wrapping** → Use [`LengthMarker`]
//! ```rust
//! use r3bl_tui::{ArrayBoundsCheck, ArrayOverflowResult, width, col, LengthMarker, IndexMarker};
//! # let line_width = width(80);
//! # let cursor_col = col(60);
//! # let text_length = width(25);
//! let remaining = line_width.remaining_from(cursor_col);
//! if text_length.convert_to_index().overflows(remaining) == ArrayOverflowResult::Overflowed { /* wrap to next line */ }
//! ```
//!
//! **🔧 Writing generic bounds functions** → Use [`UnitMarker`]
//! ```rust
//! use r3bl_tui::UnitMarker;
//! fn safe_access<I, L>(index: I, length: L) -> bool
//! where I: UnitMarker, L: UnitMarker {
//!     index.as_usize() < length.as_usize()
//! }
//! ```
//!
//! **🎛️ Check cursor position (EOL detection)** → Use [`CursorPositionBoundsStatus`]
//! ```rust
//! use r3bl_tui::{col, width, CursorBoundsCheck, CursorPositionBoundsStatus};
//! # let cursor = col(5);
//! # let content = width(10);
//! match content.check_cursor_position_bounds(cursor) {
//!     CursorPositionBoundsStatus::AtEnd => { /* cursor after last char */ }
//!     CursorPositionBoundsStatus::Beyond => { /* show error to user */ }
//!     _ => { /* other cases */ }
//! }
//! ```
//!
//! ### Semantic Traits (Use Case Validation)
//!
//! | Task                          | Trait                   | File                   | Key Question                                  |
//! |-------------------------------|-------------------------|------------------------|-----------------------------------------------|
//! | Validate array access safety  | [`ArrayBoundsCheck`]    | [`array_bounds.rs`]    | "Can I access array`[index]` correctly?"         |
//! | Check cursor position bounds  | [`CursorBoundsCheck`]   | [`cursor_bounds.rs`]   | "Can a cursor be placed at position N?"          |
//! | Determine viewport visibility | [`ViewportBoundsCheck`] | [`viewport_bounds.rs`] | "Is this content visible in my viewport?"        |
//! | Validate range structure      | [`RangeBoundsCheck`]    | [`range_bounds.rs`]    | "Is this [`Range`]/[`RangeInclusive`] valid?"    |
//! | Convert range types           | [`RangeConversion`]     | [`range_bounds.rs`]    | "How do I convert inclusive → exclusive range?" |
//!
//! #### When to Use Semantic Traits Directly
//!
//! **🔍 Array access safety checking** → Use [`array_bounds.rs`]
//! ```rust
//! use r3bl_tui::{col, width, ArrayBoundsCheck, ArrayOverflowResult};
//! # let index = col(5);
//! # let length = width(10);
//! // Simple equality check - most common case
//! if index.overflows(length) == ArrayOverflowResult::Within {
//!     // Safe to access array[index]
//! }
//!
//! // Detailed status - when you need pattern matching
//! match index.overflows(length) {
//!     ArrayOverflowResult::Within => { /* safe access */ }
//!     ArrayOverflowResult::Overflowed => { /* handle out of bounds */ }
//! }
//! ```
//!
//! **📍 Cursor position validation** → Use [`cursor_bounds.rs`]
//! ```rust
//! use r3bl_tui::{col, width, CursorPositionBoundsStatus, CursorBoundsCheck};
//! # let pos = col(5);
//! # let content_length = width(10);
//! match content_length.check_cursor_position_bounds(pos) {
//!     CursorPositionBoundsStatus::Within => { /* valid position */ }
//!     _ => { /* handle other cases */ }
//! }
//! ```
//!
//! **👁️ Viewport visibility checking** → Use [`viewport_bounds.rs`]
//! ```rust
//! use r3bl_tui::{col, width, ViewportBoundsCheck};
//! # let index = col(15);
//! # let start = col(10);
//! # let size = width(20);
//! if index.is_in_viewport(start, size) { /* content visible */ }
//! ```
//!
//! **🎯 Range validation & membership** → Use [`range_bounds.rs`]
//!
//! **Range Structure Validation** - Check if range object is well-formed:
//! ```rust
//! use r3bl_tui::{col, width, RangeBoundsCheck};
//! # let buffer_length = width(10);
//! let range = col(2)..col(8);
//! if range.is_valid(buffer_length) {
//!     // Range is valid for iteration
//! }
//! ```
//!
//! **Range Membership Checking** - Check if index is within range:
//! ```rust
//! use r3bl_tui::{row, RangeBoundsCheck, RangeBoundsResult};
//! # let row_pos = row(5);
//! # let char_pos = row(3);
//! // VT-100 scroll region checking (inclusive range)
//! let scroll_region = row(2)..=row(10);
//! if scroll_region.check_index_is_within(row_pos) == RangeBoundsResult::Within {
//!     // Perform scroll operation
//! }
//!
//! // Text selection checking with detailed status
//! let selection = row(1)..=row(5);
//! match selection.check_index_is_within(char_pos) {
//!     RangeBoundsResult::Within => { /* highlight character */ }
//!     RangeBoundsResult::Underflowed => { /* before selection */ }
//!     RangeBoundsResult::Overflowed => { /* after selection */ }
//! }
//!
//! // Simple boolean check using stdlib (when detailed status not needed)
//! if (row(2)..=row(10)).contains(&row_pos) { /* alternative approach */ }
//! ```
//!
//! **Range Type Conversion** - Convert inclusive to exclusive for iteration:
//! ```rust
//! use r3bl_tui::{row, RangeConversion};
//!
//! // VT-100 scroll region (inclusive: both endpoints are valid positions)
//! let scroll_region = row(2)..=row(5);  // Rows 2,3,4,5
//!
//! // Convert to exclusive range for Rust iteration
//! let iter_range = scroll_region.to_exclusive();  // row(2)..row(6)
//!
//! // Use for slice operations, iteration, etc.
//! // buffer.shift_lines_up(iter_range, len(1));
//! ```
//!
//! ### Decision Tree: Which Trait Do I Need?
//!
//! ```text
//! What are you trying to accomplish?
//!
//! Standard bounds checking problems:
//! ├─ "Can I safely access array[index]?" → ArrayBoundsCheck trait
//! ├─ "Where can I place a text cursor?" → CursorBoundsCheck trait
//! ├─ "Is this content visible in viewport?" → ViewportBoundsCheck trait
//! ├─ "Is this Range/RangeInclusive structurally valid?" → RangeBoundsCheck trait
//! └─ "Is this index within a range?" → RangeBoundsCheck::check_index_is_within()
//!
//! Custom/advanced operations:
//! ├─ Writing generic functions for any index/length type → UnitMarker trait
//! ├─ Space calculations, text wrapping, capacity → LengthMarker trait
//! └─ Pattern matching on detailed error conditions → result enums
//!
//! Building complex validation (combine multiple traits):
//! └─ Use foundational traits + semantic traits together
//! ```
//!
//! ### Interval Notation
//!
//! Throughout this documentation, mathematical interval notation is used to precisely
//! describe range boundaries:
//!
//! | Notation | Meaning                          | Example   | Elements Included |
//! |----------|----------------------------------|-----------|-------------------|
//! | `[a, b]` | Both endpoints included (closed) | `[5, 10]` | 5, 6, 7, 8, 9, 10 |
//! | `[a, b)` | Start included, end excluded     | `[5, 10)` | 5, 6, 7, 8, 9     |
//! | `(a, b]` | Start excluded, end included     | `(5, 10]` | 6, 7, 8, 9, 10    |
//! | `(a, b)` | Both endpoints excluded (open )  | `(5, 10)` | 6, 7, 8, 9        |
//!
//! ### Rust Range Syntax
//!
//! | Rust Syntax | Interval Notation | Meaning                      |
//! |-------------|-------------------|------------------------------|
//! | `min..=max` | `[min, max]`      | Both endpoints included      |
//! | `min..max`  | `[min, max)`      | Start included, end excluded |
//!
//! **Example with concrete values:**
//!
//! ```text
//! // Rust: 5..=9
//! // Interval: [5, 9]
//! // Contains: 5, 6, 7, 8, 9  ← 9 IS included
//!
//! // Rust: 5..10
//! // Interval: [5, 10)
//! // Contains: 5, 6, 7, 8, 9  ← 10 is NOT included
//! ```
//!
//! **Key distinction**: `]` (closed bracket) vs `)` (parenthesis):
//! - `]` means the value **IS included** (closed boundary)
//! - `)` means the value is **NOT included** (open boundary)
//!
//! **In this codebase:**
//! - **Exclusive ranges** ([`Range`]): Use `[start, end)` notation - Rust's `5..10`
//! - **Inclusive ranges** ([`RangeInclusive`]): Use `[start, end]` notation - Rust's
//!   `5..=10`
//!
//! For detailed visual comparison of exclusive vs inclusive range boundary treatment,
//! see [Exclusive vs Inclusive Range Comparison] in [`range_bounds.rs`].
//!
//! [Exclusive vs Inclusive Range Comparison]: mod@crate::core::units::bounds_check::range_bounds#exclusive-vs-inclusive-range-comparison
//!
//! ## Trait Distinction Guidance
//!
//! Understanding the subtle differences between similar traits helps you choose the right
//! tool for your specific use case.
//!
//! ### Semantic Trait Distinctions
//!
//! #### **[`ArrayBoundsCheck`] vs [`CursorBoundsCheck`]**: The key difference is whether position-after-end is valid
//!
//! | Aspect              | [`ArrayBoundsCheck`]                                   | [`CursorBoundsCheck`]                  |
//! |---------------------|--------------------------------------------------------|----------------------------------------|
//! | **Validity rule**   | `index < length` (strict)                              | `index <= length` (inclusive)          |
//! | **End position**    | Invalid (would access past array)                      | Valid (cursor after last character)    |
//! | **Use case**        | Safe array/buffer element access                       | Text cursor positioning in editors     |
//! | **Example**         | `buffer[9]` in length-10 array ✓                       | Cursor at position 10 after "hello" ✓  |
//! | **Method hint**     | Use [`overflows()`]                                    | Use [`check_cursor_position_bounds()`] |
//!
//! #### **[`ViewportBoundsCheck`] vs [`RangeBoundsCheck`]**: Both handle content windows but serve different purposes
//!
//! | Aspect               | `ViewportBoundsCheck`                                                              | `RangeBoundsCheck`                                             |
//! |----------------------|------------------------------------------------------------------------------------|----------------------------------------------------------------|
//! | **Window format**    | `(start, size)` - `start` is "index", `size` is "length" <-> `[start, start+size)` | `start..end` - Rust [`Range`] type, start and end both "index" |
//! | **End semantics**    | End value not included (exclusive)                                                 | End value not included (exclusive)                             |
//! | **Primary use**      | Rendering optimization (what's visible?)                                           | Iterator/algorithm parameter validation                        |
//! | **Checks performed** | Is index visible in current view?                                                  | Is Range structurally valid?                                   |
//! | **Example**          | "Is row 15 visible in viewport starting at row 10 with height 20?"                 | "Is range 5..10 valid for buffer len 20?"                      |
//! | **Method hint**      | Use [`is_in_viewport()`] or [`check_viewport_bounds()`]                            | Use [`is_valid()`] or [`clamp_range_to()`]                     |
//!
//! ### Foundational Trait Distinctions
//!
//! #### **[`IndexMarker`] vs [`LengthMarker`]**: Understanding 0-based positions vs 1-based sizes
//!
//! | Aspect               | `IndexMarker` (0-based)            | `LengthMarker` (1-based)                     |
//! |----------------------|------------------------------------|----------------------------------------------|
//! | **What it is**       | Position/location in content       | Size/count of content                        |
//! | **Range**            | `0..length-1` (positions)          | `1..=max_size` (counts)                      |
//! | **Key question**     | "Where am I?"                      | "How much space do I have?"                  |
//! | **Primary methods**  | [`overflows()`], [`underflows()`]  | [`remaining_from()`], [`is_overflowed_by()`] |
//! | **Use case**         | Index validation, range membership | Space calculations, capacity checks          |
//! | **Example**          | "Is cursor at row 5?"              | "Do I have 20 columns of width?"             |
//!
//! #### **When to Use [`UnitMarker`]**
//!
//! Use when writing generic functions that work with any index or length type, regardless
//! of whether it's row/column specific or generic. It provides the lowest-level
//! conversion operations ([`as_usize`], [`as_u16`], [`is_zero`]) that all other traits
//! build upon.
//!
//! ## Getting Started with Bounds Checking
//!
//! This section provides practical guidance for adopting type-safe bounds checking in
//! your code. For a deeper understanding of the underlying type system architecture,
//! see the [Type System Foundation](#type-system-foundation) section.
//!
//! ### Quick Start Guide
//!
//! Adopt bounds checking incrementally in your existing code with these four steps:
//!
//! **Step 1**: Replace raw numeric types with constructors
//! ```rust
//! use r3bl_tui::{col, width};
//! let pos_x = col(5); // Instead of let pos_x = 5_usize;
//! let width = width(10); // Instead of let width = 10_usize;
//! ```
//!
//! **Step 2**: Replace manual bounds checks with safe methods
//! ```rust
//! # use r3bl_tui::{ArrayBoundsCheck, ArrayOverflowResult, col, width, IndexMarker};
//! # let pos_x = col(5);
//! # let width = width(10);
//! if pos_x.overflows(width) == ArrayOverflowResult::Within { /* safe access */ }
//! // Instead of: if pos_x < width { /* manual check without type safety */ }
//! ```
//!
//! **Step 3**: Add pattern matching for array access (buffer/vector elements)
//! ```rust
//! use r3bl_tui::{col, width, ArrayBoundsCheck, ArrayOverflowResult};
//! # let pos_x = col(5);
//! # let width = width(10);
//! match pos_x.overflows(width) {
//!     ArrayOverflowResult::Within => { /* safe to access array[pos_x] */ }
//!     ArrayOverflowResult::Overflowed => { /* index out of bounds */ }
//! }
//! ```
//!
//! **Step 4**: Add pattern matching for cursor positioning (text editors)
//! ```rust
//! use r3bl_tui::{col, width, CursorBoundsCheck, CursorPositionBoundsStatus};
//! # let cursor_pos = col(5);
//! # let content_length = width(10);
//! match content_length.check_cursor_position_bounds(cursor_pos) {
//!     CursorPositionBoundsStatus::Within => { /* cursor inside content */ }
//!     CursorPositionBoundsStatus::AtEnd => { /* cursor after last char - valid! */ }
//!     CursorPositionBoundsStatus::Beyond => { /* cursor position invalid */ }
//!     _ => { /* handle other cases */ }
//! }
//! ```
//!
//! > <div class="warning">
//! >
//! > Steps 3 and 4 show different semantic domains. Choose the one that matches your use
//! > case:
//! >
//! > - **Step 3** ([`ArrayBoundsCheck`]): Buffer/array element access where `index <
//! > length`
//! > - **Step 4** ([`CursorBoundsCheck`]): Text cursor positioning where `index <=
//! > length` (allows cursor after last character)
//! >
//! > See the [semantic trait distinctions](#semantic-trait-distinctions) section for
//! > details.
//! >
//! > </div>
//!
//! ```text
//! Quick Start Progression:
//!
//!   Step 1: Type-Safe Constructors
//!          col(5), width(10)
//!                 │
//!                 ▼
//!   Step 2: Boolean Validation
//!          !index.overflows(length)
//!                 │
//!                 ▼
//!          ┌──────┴──────┐
//!          ▼             ▼
//!     Step 3:        Step 4:
//!   Array Access   Cursor Positioning
//!   (buffer/vec)   (text editor)
//!          │             │
//!          ▼             ▼
//!    index < length   index <= length
//! ```
//!
//! This quick start focuses on the most common bounds checking patterns (array access and
//! cursor positioning). For other use cases like viewport visibility, range validation,
//! scroll regions, and text selections, see the [When to Use What](#when-to-use-what)
//! section and the [Decision Tree](#decision-tree-which-trait-do-i-need).
//!
//! For comprehensive details on each trait's methods and edge cases, see the individual
//! module documentation files. This guide gets you productive quickly, while the detailed
//! trait docs cover advanced patterns and special cases.
//!
//! **For deeper understanding**: See [Example: Type System in
//! Action](#example-type-system-in-action) to see how the type system prevents common
//! errors at compile time.
//!
//! ### Common Mistakes to Avoid
//!
//! **❌ Don't mix row and column types**
//! ```rust,compile_fail
//! use r3bl_tui::{row, width, IndexMarker};  // IndexMarker provides .overflows()
//! // Compiler error - cannot compare RowIndex with ColWidth
//! let row_pos = row(5);
//! let col_width = width(10);
//! row_pos.overflows(col_width); // Won't compile!
//! ```
//!
//! **❌ Don't use raw usize for bounds checking**
//! ```rust
//! let raw_index: usize = 5;
//! let raw_length: usize = 10;
//! // Error-prone - no protection against off-by-one bugs
//! if raw_index < raw_length { /* unsafe! */ }
//! ```
//!
//! **✅ Do use type-safe constructors and methods**
//! ```rust
//! use r3bl_tui::{ArrayBoundsCheck, ArrayOverflowResult, col, width, IndexMarker};
//! let index = col(5);
//! let length = width(10);
//! if index.overflows(length) == ArrayOverflowResult::Within { /* safe! */ }
//! ```
//!
//! ## Type System Foundation
//!
//! The bounds checking system uses two distinct type categories: **Index types**
//! for positions (0-based) and **Length types** for sizes (1-based).
//!
//! This separation, enforced through the [`IndexMarker`] and [`LengthMarker`]
//! traits, prevents entire categories of off-by-one errors and type confusion at compile
//! time.
//!
//! ### Trait Hierarchy
//!
//! Both [`IndexMarker`] and [`LengthMarker`] build on top of [`UnitMarker`] as their
//! super-trait:
//!
//! ```text
//! Trait Hierarchy:
//!
//!                    UnitMarker
//!                   (super-trait)
//!                        │
//!                        │ Provides: as_usize(), as_u16(), is_zero()
//!                        │ Purpose: Generic numeric conversions
//!                        │
//!           ┌────────────┴────────────┐
//!           │                         │
//!      IndexMarker                LengthMarker
//!      (0-based)                  (1-based)
//!           │                         │
//!   Adds: overflows(),        Adds: is_overflowed_by(),
//!         underflows(),             remaining_from(),
//!         clamp_to_*(),             convert_to_index(),
//!         convert_to_length()       clamp_to_max()
//! ```
//!
//! - **[`UnitMarker`]** - The foundational trait providing numeric conversions
//!   ([`UnitMarker::as_usize()`], [`UnitMarker::as_u16()`], [`UnitMarker::is_zero()`])
//!   that enable all higher-level operations. Use this directly when writing generic
//!   bounds checking functions.
//!
//! - **[`IndexMarker`]** - Extends [`UnitMarker`] with 0-based position semantics and
//!   bounds checking operations specific to array indexing.
//!
//! - **[`LengthMarker`]** - Extends [`UnitMarker`] with 1-based size semantics and space
//!   calculation operations specific to container sizes.
//!
//! This hierarchy enables both generic operations (via [`UnitMarker`]) and specialized,
//! type-safe operations (via [`IndexMarker`] and [`LengthMarker`]).
//!
//! ### The [`IndexMarker`] Trait - index or position operations
//!
//! [`IndexMarker`] identifies types that represent positions within content. These are
//! 0-based values where the first position is index 0. The trait provides the
//! foundational operations that enable all bounds checking patterns in the system.
//!
//! ```text
//! Index concept (0-based positioning):
//!
//!                   Associated type `LengthType`
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
//! | Aspect                    | Description                                                                                                |
//! |---------------------------|-------------------------------------------------------------------------------------------------------|
//! | Purpose                   | Marker trait for 0-based index or position indicators with comprehensive bounds checking              |
//! | Types                     | [`Index`], [`RowIndex`], [`ColIndex`], [`ByteIndex`]                                                  |
//! | Associated Type           | `LengthType` - The corresponding 1-based length or size type: [`Index`] -> [`Length`], [`RowIndex`] -> [`RowHeight`], [`ColIndex`] -> [`ColWidth`], [`ByteIndex`] -> [`ByteLength`] |
//!
//! #### Method Categories
//! - **Overflow checking**: [`index.overflows(length)`], [`index.underflows(min_index)`]
//!     - Check if position exceeds container size or falls below minimum bound
//! - **Clamping**: [`index.clamp_to_max_length(length)`],
//!   [`index.clamp_to_min_index(min_index)`], [`index.clamp_to_range(range)`]
//!     - Ensure position stays within valid bounds
//! - **Conversions**: [`index.convert_to_length()`]
//!     - Transform between 0-based index and 1-based length (index + 1)
//!
//! #### Associated Type Relationship
//! Each [`IndexMarker`] has an associated type
//! `LengthType` that must itself have an `IndexType` pointing back, creating a
//! bidirectional type-safe relationship. This prevents comparing incompatible types like
//! [`RowIndex`] with [`ColWidth`].
//!
//! ### The [`LengthMarker`] Trait - length or size operations
//!
//! [`LengthMarker`] identifies types that represent sizes or measurements of content.
//! These are 1-based values where a length of 1 means "one unit of size". The trait
//! provides size-centric operations for space calculations and capacity management.
//!
//! ```text
//! Length concept (1-based size measurement):
//!
//!                  Container with length=10
//!           ╭───────────────────────────────────╮
//!           │                                   │
//! Length:   1   2   3   4   5   6   7   8   9   10
//!         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
//!         │ A │ B │ C │ D │ E │ F │ G │ H │ I │ J │
//!         └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘
//!           ↑                                   ↑
//!      1st unit                           10th unit
//!     (size = 1)                         (size = 10)
//! ```
//!
//! | Aspect          | Description                                                                              |
//! |-----------------|------------------------------------------------------------------------------------------|
//! | Purpose         | Marker trait for 1-based size measurements with space calculation capabilities           |
//! | Types           | [`Length`], [`RowHeight`], [`ColWidth`], [`ByteLength`]                                  |
//! | Associated Type | `IndexType` - The corresponding 0-based index or position type: [`Length`] -> [`Index`], [`RowHeight`] -> [`RowIndex`], [`ColWidth`] -> [`ColIndex`], [`ByteLength`] -> [`ByteIndex`] |
//!
//! #### Method Categories
//! - **Overflow checking**: [`length.is_overflowed_by(index)`]
//!   - Check if index exceeds this size (validates from size perspective)
//! - **Space calculations**: [`length.remaining_from(index)`]
//!   - Calculate available space from position (essential for rendering and text
//!     wrapping)
//! - **Clamping**: [`length.clamp_to_max(max)`]
//!   - Ensure size stays within maximum bounds
//! - **Conversions**: [`length.convert_to_index()`]
//!   - Get last valid position (length - 1)
//!
//! #### Associated Type Relationship
//! Each [`LengthMarker`] has an associated type `IndexType` that must itself have an
//! associated type `LengthType` pointing back, completing the bidirectional relationship.
//! This prevents comparing incompatible types like [`RowIndex`] with [`ColWidth`].
//!
//! ### Bidirectional Type Safety
//!
//! The type system enforces a bidirectional relationship between index and length types
//! through associated type constraints. This creates compile-time guarantees that
//! prevent type confusion:
//!
//! ```text
//! Bidirectional Type Relationships:
//!
//!     IndexMarker                    LengthMarker
//!         │                               │
//!         │  type LengthType ────────────▶│
//!         │                               │
//!         │◀──────────── type IndexType   │
//!         │                               │
//!
//! Concrete Type Pairs:
//!
//!     RowIndex    ◀───────────▶    RowHeight
//!     (0-based row position)       (1-based row count)
//!
//!     ColIndex    ◀───────────▶    ColWidth
//!     (0-based column position)    (1-based column count)
//!
//!     Index       ◀───────────▶    Length
//!     (generic 0-based position)   (generic 1-based size)
//!
//! Compile-Time Prevention:
//!
//! ✗ row_index.overflows(col_width)     // Won't compile!
//! ✗ col_index.overflows(row_height)    // Won't compile!
//! ✓ row_index.overflows(row_height)    // Type-safe ✓
//! ✓ col_index.overflows(col_width)     // Type-safe ✓
//! ```
//!
//! ### Type Mappings and Semantic Domains
//!
//! The system provides three levels of type specificity. This separation ensures that row
//! operations cannot accidentally mix with column operations, preventing bugs like using
//! row positions for column bounds checking.
//!
//! **Generic Types** (domain-agnostic):
//! - [`Index`] ↔ [`Length`] - Use when dimension doesn't matter or for algorithms that
//!   work with any index/length pair. They can easily be converted from one to another.
//!
//! **Terminal-Specific Types** (2D grid semantics):
//! - [`RowIndex`] ↔ [`RowHeight`] - Vertical positioning and sizing in terminal grids.
//!   They can easily be converted from one to another.
//! - [`ColIndex`] ↔ [`ColWidth`] - Horizontal positioning and sizing in terminal grids.
//!   They can easily be converted from one to another.
//!
//! **VT-100 Protocol Types** (not part of bounds checking):
//! - [`TermRow`], [`TermCol`] - 1-based terminal coordinates for ANSI escape sequences
//!   - Located in `vt_100_ansi_parser::term_units` module
//!   - Used exclusively for CSI sequence parsing (`ESC[row;colH`)
//!   - Convert to/from [`Row`]/[`Col`] for buffer operations
//!   - **Not paired**: Both are 1-based positions, neither represents a size/length
//!   - **Different domain**: Terminal protocol coordinates, not buffer bounds checking
//!
//! > <div class="warning">
//! >
//! > Don't confuse [`TermRow`] (1-based terminal coordinate) with [`RowIndex`]
//! > (0-based buffer position) or [`RowHeight`] (1-based buffer size). The bounds
//! > checking
//! > system works on buffer coordinates, while [`TermRow`]/[`TermCol`] are for VT-100
//! > parsing.
//! >
//! > </div>
//!
//! ### Type Safety Guarantees
//!
//! The [`IndexMarker`] and [`LengthMarker`] traits, combined with their bidirectional
//! associated type constraints, provide several compile-time guarantees:
//!
//! - **Dimensional Integrity**: Cannot compare incompatible dimensions
//!    - ✗ [`RowIndex`] vs [`ColWidth`] won't compile
//!    - ✓ [`RowIndex`] vs [`RowHeight`] is type-safe
//!
//! - **Semantic Clarity**: 0-based vs 1-based is explicit in the type
//!    - Index types are always 0-based positions
//!    - Length types are always 1-based sizes
//!    - No confusion about what a value represents
//!
//! - **Consistent Behavior**: Single trait implementations work across all concrete types
//!    - Write generic code once using [`IndexMarker`] / [`LengthMarker`]
//!    - Works correctly for [`RowIndex`], [`ColIndex`], and [`Index`]
//!    - No need to duplicate logic for each concrete type
//!
//! - **Conversion Safety**: Type conversions are explicit and unambiguous
//!    - [`index.convert_to_length()`] always adds 1 (0-based → 1-based)
//!    - [`length.convert_to_index()`] always subtracts 1 (1-based → 0-based)
//!    - Compiler tracks which type family (row/col/generic) you're working with
//!
//! - **Bounds Checking Correctness**: Off-by-one errors caught at compile time
//!    - Array access: `index < length` (strict inequality)
//!    - Cursor position: `index <= length` (allows end position)
//!    - Type system prevents mixing these semantics
//!
//! ### Example: Type System in Action
//!
//! This example demonstrates how the type system guarantees prevent common errors at
//! compile time. For practical adoption guidance, see the
//! [Getting Started with Bounds Checking](#getting-started-with-bounds-checking) section.
//!
//! ```rust
//! use r3bl_tui::{ArrayBoundsCheck, ArrayOverflowResult, IndexMarker, LengthMarker, row, col, height, width};
//!
//! // Type-safe terminal operations
//! let cursor_row = row(5);
//! let terminal_height = height(24);
//! let cursor_col = col(10);
//! let terminal_width = width(80);
//!
//! // These work - types match
//! if cursor_row.overflows(terminal_height) == ArrayOverflowResult::Within {
//!     println!("Row {} is valid", cursor_row.as_usize());
//! }
//!
//! if cursor_col.overflows(terminal_width) == ArrayOverflowResult::Within {
//!     println!("Column {} is valid", cursor_col.as_usize());
//! }
//!
//! // These won't compile - type mismatch caught at compile time!
//! // cursor_row.overflows(terminal_width);   // ✗ Can't compare RowIndex to ColWidth
//! // cursor_col.overflows(terminal_height);  // ✗ Can't compare ColIndex to RowHeight
//!
//! // Conversions are explicit and type-safe
//! let row_as_length = cursor_row.convert_to_length();  // RowIndex → RowHeight
//! let last_col = terminal_width.convert_to_index();    // ColWidth → ColIndex
//! ```
//!
//! ### Related Types Outside the Bounds System
//!
//! Some types work with indices and lengths but don't participate in the
//! [`IndexMarker`]/[`LengthMarker`] type system:
//!
//! - [`ByteOffset`] - Represents relative distances or offsets (not absolute positions or
//!   sizes). Used for specialized calculations like gap buffer operations in the
//!   zero-copy editor implementation. Unlike [`ByteIndex`] and [`ByteLength`] which form
//!   a standard index/length pair, [`ByteOffset`] is intentionally separate from the
//!   bounds checking system.
//!
//! [`ByteOffset`]: crate::ByteOffset
//! [`RowIndex`]: crate::RowIndex
//! [`ColIndex`]: crate::ColIndex
//! [`ByteIndex`]: crate::ByteIndex
//! [`RowHeight`]: crate::RowHeight
//! [`ColWidth`]: crate::ColWidth
//! [`ByteLength`]: crate::ByteLength
//! [`Index`]: crate::Index
//! [`Length`]: crate::Length
//! [`IndexMarker`]: crate::IndexMarker
//! [`LengthMarker`]: crate::LengthMarker
//! [`overflows()`]: crate::ArrayBoundsCheck::overflows
//! [`convert_to_length()`]: crate::IndexMarker::convert_to_length
//! [`clamp_to_max_length()`]: crate::IndexMarker::clamp_to_max_length
//! [`underflows()`]: crate::ArrayBoundsCheck::underflows
//! [`index.overflows(length)`]: crate::ArrayBoundsCheck::overflows
//! [`index.convert_to_length()`]: crate::IndexMarker::convert_to_length
//! [`index.clamp_to_max_length(length)`]: crate::IndexMarker::clamp_to_max_length
//! [`index.underflows(min_index)`]: crate::ArrayBoundsCheck::underflows
//! [`length.convert_to_index()`]: crate::LengthMarker::convert_to_index
//! [`length.is_overflowed_by(index)`]: crate::LengthMarker::is_overflowed_by
//! [`length.remaining_from(index)`]: crate::LengthMarker::remaining_from
//! [`length.clamp_to_max(max)`]: crate::LengthMarker::clamp_to_max
//! [`clamp_to_min_index()`]: crate::IndexMarker::clamp_to_min_index
//! [`clamp_to_range()`]: crate::IndexMarker::clamp_to_range
//! [`index.clamp_to_min_index(min_index)`]: crate::IndexMarker::clamp_to_min_index
//! [`index.clamp_to_range(range)`]: crate::IndexMarker::clamp_to_range
//! [`convert_to_index()`]: crate::LengthMarker::convert_to_index
//! [`is_overflowed_by()`]: crate::LengthMarker::is_overflowed_by
//! [`remaining_from()`]: crate::LengthMarker::remaining_from
//! [`clamp_to_max()`]: crate::LengthMarker::clamp_to_max
//! [`is_valid()`]: crate::RangeBoundsCheck::is_valid
//! [`clamp_range_to()`]: crate::RangeBoundsCheck::clamp_range_to
//! [`check_index_is_within()`]: crate::RangeBoundsCheck::check_index_is_within
//! [`check_cursor_position_bounds()`]: crate::CursorBoundsCheck::check_cursor_position_bounds
//! [`eol_cursor_position()`]: crate::CursorBoundsCheck::eol_cursor_position
//! [`is_valid_cursor_position()`]: crate::CursorBoundsCheck::is_valid_cursor_position
//! [`clamp_cursor_position()`]: crate::CursorBoundsCheck::clamp_cursor_position
//! [`is_in_viewport()`]: crate::ViewportBoundsCheck::is_in_viewport
//! [`check_viewport_bounds()`]: crate::ViewportBoundsCheck::check_viewport_bounds
//! [`array_bounds.rs`]: mod@crate::array_bounds
//! [`cursor_bounds.rs`]: mod@crate::cursor_bounds
//! [`index_marker.rs`]: mod@crate::index_marker
//! [`length_marker.rs`]: mod@crate::length_marker
//! [`range_bounds.rs`]: mod@crate::range_bounds
//! [`viewport_bounds.rs`]: mod@crate::viewport_bounds
//! [`result_enums.rs`]: mod@crate::result_enums
//! [`unit_marker.rs`]: mod@crate::unit_marker
//! [`Range`]: std::ops::Range
//! [`RangeInclusive`]: std::ops::RangeInclusive
//! [`as_usize`]: UnitMarker::as_usize
//! [`as_u16`]: UnitMarker::as_u16
//! [`is_zero`]: UnitMarker::is_zero
//! [`col()`]: crate::col
//! [`row()`]: crate::row
//! [`width()`]: crate::width
//! [`height()`]: crate::height
//! [`Range<Index>`]: std::ops::Range
//! [`to_exclusive()`]: crate::RangeConversion::to_exclusive
//! [`TermRow`]: crate::core::pty_mux::vt_100_ansi_parser::term_units::TermRow
//! [`TermCol`]: crate::core::pty_mux::vt_100_ansi_parser::term_units::TermCol
//! [`Row`]: crate::Row
//! [`Col`]: crate::Col
//! [`as_usize`]: crate::UnitMarker::as_usize
//! [`as_u16`]: crate::UnitMarker::as_u16
//! [`is_zero`]: crate::UnitMarker::is_zero
//! [`UnitMarker`]: crate::UnitMarker

// Attach.
pub mod array_bounds;
pub mod cursor_bounds;
pub mod index_marker;
pub mod length_marker;
pub mod range_bounds;
pub mod result_enums;
pub mod unit_marker;
pub mod viewport_bounds;

// Re-export.
pub use array_bounds::*;
pub use cursor_bounds::*;
pub use index_marker::*;
pub use length_marker::*;
pub use range_bounds::*;
pub use result_enums::*;
pub use unit_marker::*;
pub use viewport_bounds::*;

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::{RowIndex, height, idx, len, row};

    #[test]
    fn test_array_cursor_bounds_semantic_difference() {
        // Critical test: Array bounds and cursor bounds have different semantics
        // Array: index must be < length (for element access)
        // Cursor: position can be <= length (cursor after last char)
        let content_len = len(10);

        // Test the boundary where semantics differ
        let idx_9 = idx(9);
        let idx_10 = idx(10);

        // Index 9: valid for both array access and cursor position
        assert_eq!(idx_9.overflows(content_len), ArrayOverflowResult::Within);
        assert!(content_len.is_valid_cursor_position(idx_9));
        assert_eq!(
            content_len.check_cursor_position_bounds(idx_9),
            CursorPositionBoundsStatus::Within
        );

        // Index 10: INVALID for array access but VALID for cursor (EOL position)
        // This semantic difference is critical for text editors
        assert_eq!(
            idx_10.overflows(content_len),
            ArrayOverflowResult::Overflowed
        );
        assert!(content_len.is_valid_cursor_position(idx_10));
        assert_eq!(
            idx_10.overflows(content_len),
            ArrayOverflowResult::Overflowed
        );
        assert_eq!(
            content_len.check_cursor_position_bounds(idx_10),
            CursorPositionBoundsStatus::AtEnd
        );
    }

    #[test]
    fn test_zero_length_consistency_across_traits() {
        // Cross-cutting concern: All traits must handle empty content consistently
        // This tests the edge case that often causes bugs
        let zero_len = len(0);
        let any_idx = idx(0);

        // Array bounds: should reject ALL indices (no elements to access)
        assert_eq!(any_idx.overflows(zero_len), ArrayOverflowResult::Overflowed);
        assert_eq!(any_idx.overflows(zero_len), ArrayOverflowResult::Overflowed);

        // Cursor bounds: position 0 is valid (cursor at start of empty content)
        assert!(zero_len.is_valid_cursor_position(any_idx));
        assert_eq!(
            zero_len.check_cursor_position_bounds(any_idx),
            CursorPositionBoundsStatus::AtStart
        );
        assert_eq!(zero_len.eol_cursor_position(), any_idx);

        // Viewport: zero-size viewport contains nothing
        let zero_viewport_size = len(0);
        assert!(!any_idx.is_in_viewport(idx(0), zero_viewport_size));

        // This consistency is crucial for avoiding special-case code throughout the
        // system
    }

    #[test]
    fn test_vt100_scroll_region_conversion_in_context() {
        use std::ops::RangeInclusive;

        // Real-world scenario: VT-100 terminals use inclusive ranges for scroll regions
        // but Rust iteration needs exclusive ranges
        let scroll_region: RangeInclusive<RowIndex> = row(2)..=row(10);

        // Convert to exclusive for Rust iteration
        let iter_range = scroll_region.to_exclusive();

        // Verify conversion: inclusive 2..=10 becomes exclusive 2..11
        assert_eq!(iter_range.start, row(2));
        assert_eq!(iter_range.end, row(11));

        // Practical application: Check visibility in viewport
        let viewport_start = row(0);
        let viewport_height = height(15);

        // Verify all rows in scroll region are visible (testing the conversion works
        // correctly)
        for i in 2..11 {
            let row_idx = row(i);
            assert!(
                row_idx.is_in_viewport(viewport_start, viewport_height),
                "Row {i} should be visible in viewport"
            );
        }

        // Edge case: single-row scroll region
        let single_row: RangeInclusive<RowIndex> = row(5)..=row(5);
        let single_exclusive = single_row.to_exclusive();
        assert_eq!(single_exclusive.start, row(5));
        assert_eq!(single_exclusive.end, row(6)); // 5..6 includes only row 5
    }

    #[test]
    fn test_real_world_viewport_scrolling() {
        // Simulate actual text editor viewport management with cursor tracking
        let buffer_height = height(100);
        let viewport_height = height(25);
        let mut viewport_start = row(0);
        let cursor_row = row(30);

        // Step 1: Check if cursor is visible
        if !cursor_row.is_in_viewport(viewport_start, viewport_height) {
            // Step 2: Calculate new viewport position to center cursor
            if cursor_row
                .overflows(height(viewport_start.as_u16() + viewport_height.as_u16()))
                == ArrayOverflowResult::Overflowed
            {
                // Cursor is below viewport - scroll down
                viewport_start = row(cursor_row
                    .as_u16()
                    .saturating_sub(viewport_height.as_u16() / 2));
            }
        }

        // Step 3: Ensure viewport doesn't exceed buffer bounds
        let max_viewport_start = row(buffer_height
            .as_u16()
            .saturating_sub(viewport_height.as_u16()));
        if viewport_start.overflows(height(max_viewport_start.as_u16() + 1))
            == ArrayOverflowResult::Overflowed
        {
            viewport_start = max_viewport_start;
        }

        // Verify cursor is now visible after scrolling
        assert!(cursor_row.is_in_viewport(viewport_start, viewport_height));

        // Additional verification: viewport should be within buffer
        assert_eq!(
            viewport_start.overflows(buffer_height),
            ArrayOverflowResult::Within
        );

        // Test edge case: cursor near bottom of buffer
        let bottom_cursor = row(95);
        let mut test_viewport = row(70);

        // Scroll to show bottom cursor
        if !bottom_cursor.is_in_viewport(test_viewport, viewport_height) {
            test_viewport = row(buffer_height
                .as_u16()
                .saturating_sub(viewport_height.as_u16()));
        }

        assert!(bottom_cursor.is_in_viewport(test_viewport, viewport_height));
        assert_eq!(test_viewport, row(75)); // 100 - 25 = 75
    }
}
