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
//! | Principle          | Description                                                    |
//! | ------------------ | -------------------------------------------------------------- |
//! | Semantic Clarity   | Each file in this module has a single, well-defined purpose.   |
//! | Type Safety        | Impossible to make incorrect comparisons at compile time.      |
//! | Performance        | Zero-cost abstractions with compile-time guarantees.           |
//! | Discoverability    | Developers can find what they need based on use case.          |
//! | Maintainability    | Clear boundaries make the system easy to extend.               |
//!
//! ## Core Traits Overview
//!
//! The bounds checking system uses a **two-tier trait architecture**:
//!
//! ### Foundational Traits (Core Operations)
//!
//! These provide the fundamental operations used across all bounds checking patterns:
//!
//! | Trait                    | File                | Key Methods                                                                                                      |
//! | ------------------------ | ------------------- | ---------------------------------------------------------------------------------------------------------------- |
//! | [`NumericConversions`]   | [`numeric_value`]   | [`as_usize()`], [`as_u16()`]                                                                                     |
//! | [`NumericValue`]         | [`numeric_value`]   | Extends [`NumericConversions`], adds [`From`] conversions, [`is_zero()`]                                         |
//! | [`IndexOps`]             | [`index_ops.rs`]    | [`convert_to_length()`], [`clamp_to_max_length()`], [`clamp_to_min_index()`], [`clamp_to_range()`]               |
//! | [`LengthOps`]            | [`length_ops.rs`]   | [`convert_to_index()`], [`index_from_end()`], [`is_overflowed_by()`], [`remaining_from()`], [`clamp_to_max()`]   |
//!
//! ### Semantic Traits (Use Case Validation)
//!
//! These build on foundational traits to provide specialized validation for specific use
//! cases:
//!
//! | Trait                     | File                            | Key Methods                                                                                                                               |
//! | ------------------------- | ------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
//! | [`ArrayBoundsCheck`]      | [`array_bounds_check.rs`]       | [`overflows()`], [`underflows()`]                                                                                                         |
//! | [`CursorBoundsCheck`]     | [`cursor_bounds_check.rs`]      | [`eol_cursor_position()`], [`is_valid_cursor_position()`], [`clamp_cursor_position()`], [`check_cursor_position_bounds()`]                |
//! | [`ViewportBoundsCheck`]   | [`viewport_bounds_check.rs`]    | [`check_viewport_bounds()`]                                                                                                               |
//! | [`RangeBoundsExt`]        | [`range_bounds_check_ext.rs`]   | [`check_range_is_valid_for_length()`], [`clamp_range_to()`], [`check_index_is_within()`] (supports both [`Range`] and [`RangeInclusive`]) |
//! | [`RangeConvertExt`]       | [`range_convert_ext.rs`]        | [`to_exclusive()`] - Convert [`RangeInclusive`] → [`Range`] for iteration                                                                 |
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
//! | Task                               | Trait or enum                                             | File                  | Key Question                                    |
//! | ---------------------------------- | --------------------------------------------------------- | --------------------- | ----------------------------------------------- |
//! | Compare indices to each other      | [`IndexOps`]                                              | [`index_ops`]         | "How do indices relate to each other?"          |
//! | Work with lengths/sizes            | [`LengthOps`]                                             | [`length_ops`]        | "What can I do with a length value?"            |
//! | Read numeric values                | [`NumericConversions`]                                    | [`numeric_value`]     | "How do I read as usize/u16?"                   |
//! | Construct & check numeric values   | [`NumericValue`]                                          | [`numeric_value`]     | "How do I create from integers & check zero?"   |
//! | Status return enum types           | [`ArrayOverflowResult`], [`CursorPositionBoundsStatus`]   | [`result_enums.rs`]   | "What status types are available?"              |
//!
//! #### When to Use Foundational Traits Directly
//!
//! **📐 Length/space/size calculations & text wrapping** → Use [`LengthOps`]
//!
//! ```rust
//! use r3bl_tui::{ArrayBoundsCheck, ArrayOverflowResult, width, col, LengthOps, IndexOps};
//! # let line_width = width(80);
//! # let cursor_col = col(60);
//! # let text_length = width(25);
//! let remaining = line_width.remaining_from(cursor_col);
//! if text_length.convert_to_index().overflows(remaining) == ArrayOverflowResult::Overflowed { /* wrap to next line */ }
//! ```
//!
//! **🔧 Writing generic bounds functions** → Use [`NumericConversions`] or
//! [`NumericValue`]
//!
//! ```rust
//! // Use NumericConversions when you only need to READ values (most common)
//! use r3bl_tui::NumericConversions;
//! fn safe_access<I, L>(index: I, length: L) -> bool
//! where I: NumericConversions, L: NumericConversions {
//!     index.as_usize() < length.as_usize()
//! }
//!
//! // Use NumericValue when you need to CREATE values or check for zero
//! use r3bl_tui::NumericValue;
//! fn process_if_nonzero<T>(value: T) -> Option<usize>
//! where T: NumericValue {
//!     if value.is_zero() { None } else { Some(value.as_usize()) }
//! }
//! ```
//!
//! **🎛️ Check cursor position (EOL detection)** → Use [`CursorPositionBoundsStatus`]
//!
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
//! | Task                            | Trait                     | File                            | Key Question                                      |
//! | ------------------------------- | ------------------------- | ------------------------------- | ------------------------------------------------- |
//! | Validate array access safety    | [`ArrayBoundsCheck`]      | [`array_bounds_check.rs`]       | "Can I access array`[index]` correctly?"          |
//! | Check cursor position bounds    | [`CursorBoundsCheck`]     | [`cursor_bounds_check.rs`]      | "Can a cursor be placed at position N?"           |
//! | Determine viewport visibility   | [`ViewportBoundsCheck`]   | [`viewport_bounds_check.rs`]    | "Is this content visible in my viewport?"         |
//! | Validate range structure        | [`RangeBoundsExt`]        | [`range_bounds_check_ext.rs`]   | "Is this [`Range`]/[`RangeInclusive`] valid?"     |
//! | Convert range types             | [`RangeConvertExt`]       | [`range_convert_ext.rs`]        | "How do I convert inclusive → exclusive range?"   |
//!
//! #### When to Use Semantic Traits Directly
//!
//! **🔍 Array access safety checking** → Use [`array_bounds_check.rs`]
//!
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
//! **📍 Cursor position validation** → Use [`cursor_bounds_check.rs`]
//!
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
//! **👁️ Viewport visibility checking** → Use [`viewport_bounds_check.rs`]
//!
//! ```rust
//! use r3bl_tui::{col, width, ViewportBoundsCheck, RangeBoundsResult};
//! # let index = col(15);
//! # let start = col(10);
//! # let size = width(20);
//! if index.check_viewport_bounds(start, size) == RangeBoundsResult::Within { /* content visible */ }
//! ```
//!
//! **🎯 Range validation & membership** → Use [`range_bounds_check_ext.rs`]
//!
//! **Range Structure Validation** - Check if range object is well-formed:
//!
//! ```rust
//! use r3bl_tui::{col, width, RangeBoundsExt, RangeValidityStatus};
//! # let buffer_length = width(10);
//! let range = col(2)..col(8);
//! if range.check_range_is_valid_for_length(buffer_length) == RangeValidityStatus::Valid {
//!     // Range is valid for iteration
//! }
//! ```
//!
//! **Range Membership Checking** - Check if index is within range:
//!
//! ```rust
//! use r3bl_tui::{row, RangeBoundsExt, RangeBoundsResult};
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
//!
//! ```rust
//! use r3bl_tui::{row, RangeConvertExt};
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
//! ├─ "Is this Range/RangeInclusive structurally valid?" → RangeBoundsExt trait
//! └─ "Is this index within a range?" → RangeBoundsExt::check_index_is_within()
//!
//! Custom/advanced operations:
//! ├─ Writing generic functions for any index/length type → NumericValue trait
//! ├─ Space calculations, text wrapping, capacity → LengthOps trait
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
//! | Notation   | Meaning                            | Example     | Elements Included   |
//! | ---------- | ---------------------------------- | ----------- | ------------------- |
//! | `[a, b]`   | Both endpoints included (closed)   | `[5, 10]`   | 5, 6, 7, 8, 9, 10   |
//! | `[a, b)`   | Start included, end excluded       | `[5, 10)`   | 5, 6, 7, 8, 9       |
//! | `(a, b]`   | Start excluded, end included       | `(5, 10]`   | 6, 7, 8, 9, 10      |
//! | `(a, b)`   | Both endpoints excluded (open )    | `(5, 10)`   | 6, 7, 8, 9          |
//!
//! ### Rust Range Syntax
//!
//! | Rust Syntax   | Interval Notation   | Meaning                        |
//! | ------------- | ------------------- | ------------------------------ |
//! | `min..=max`   | `[min, max]`        | Both endpoints included        |
//! | `min..max`    | `[min, max)`        | Start included, end excluded   |
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
//! see [Exclusive vs Inclusive Range Comparison] in [`range_bounds_check_ext.rs`].
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
//! | Aspect                | [`ArrayBoundsCheck`]                | [`CursorBoundsCheck`]                    |
//! | --------------------- | ----------------------------------- | ---------------------------------------- |
//! | **Validity rule**     | `index < length` (strict)           | `index <= length` (inclusive)            |
//! | **End position**      | Invalid (would access past array)   | Valid (cursor after last character)      |
//! | **Use case**          | Safe array/buffer element access    | Text cursor positioning in editors       |
//! | **Example**           | `buffer[9]` in length-10 array ✓    | Cursor at position 10 after "hello" ✓    |
//! | **Method hint**       | Use [`overflows()`]                 | Use [`check_cursor_position_bounds()`]   |
//!
//! #### **[`ViewportBoundsCheck`] vs [`RangeBoundsExt`]**: Both handle content windows but serve different purposes
//!
//! | Aspect                 | `ViewportBoundsCheck`                                                                | `RangeBoundsExt`                                                    |
//! | ---------------------- | ------------------------------------------------------------------------------------ | ------------------------------------------------------------------- |
//! | **Window format**      | `(start, size)` - `start` is "index", `size` is "length" <-> `[start, start+size)`   | `start..end` - Rust [`Range`] type, start and end both "index"      |
//! | **End semantics**      | End value not included (exclusive)                                                   | End value not included (exclusive)                                  |
//! | **Primary use**        | Rendering optimization (what's visible?)                                             | Iterator/algorithm parameter validation                             |
//! | **Checks performed**   | Is index visible in current view?                                                    | Is Range structurally valid?                                        |
//! | **Example**            | "Is row 15 visible in viewport starting at row 10 with height 20?"                   | "Is range 5..10 valid for buffer len 20?"                           |
//! | **Method hint**        | Use [`check_viewport_bounds()`]                                                      | Use [`check_range_is_valid_for_length()`] or [`clamp_range_to()`]   |
//!
//! ### Foundational Trait Distinctions
//!
//! #### **[`IndexOps`] vs [`LengthOps`]**: Understanding 0-based positions vs 1-based sizes
//!
//! | Aspect                 | `IndexOps` (0-based)                 | `LengthOps` (1-based)                          |
//! | ---------------------- | ------------------------------------ | ---------------------------------------------- |
//! | **What it is**         | Position/location in content         | Size/count of content                          |
//! | **Range**              | `0..length-1` (positions)            | `1..=max_size` (counts)                        |
//! | **Key question**       | "Where am I?"                        | "How much space do I have?"                    |
//! | **Primary methods**    | [`overflows()`], [`underflows()`]    | [`remaining_from()`], [`is_overflowed_by()`]   |
//! | **Use case**           | Index validation, range membership   | Space calculations, capacity checks            |
//! | **Example**            | "Is cursor at row 5?"                | "Do I have 20 columns of width?"               |
//!
//! #### **When to Use [`NumericConversions`] vs [`NumericValue`]**
//!
//! Both traits enable writing generic functions that work with any index or length type.
//! Choose based on what operations you need:
//!
//! **Use [`NumericConversions`] (most common):**
//! - When you only need to **read** numeric values ([`as_usize()`], [`as_u16()`])
//! - For comparisons, calculations, or passing values to other functions
//! - Example: Generic bounds checking that compares index < length
//! - Less restrictive trait bound - works with both zero and non-zero types
//!
//! **Use [`NumericValue`] (when construction needed):**
//! - When you need to **create** new values from integers ([`From<usize>`],
//!   [`From<u16>`])
//! - When you need to check for zero ([`is_zero()`])
//! - Example: Generic factory functions or zero-handling logic
//! - Extends [`NumericConversions`], so includes all reading operations
//!
//! ## Getting Started with Bounds Checking
//!
//! This section provides practical guidance for adopting type-safe bounds checking in
//! your code. For a deeper understanding of the underlying type system architecture,
//! see the [Type System Foundation] section.
//!
//! ### Quick Start Guide
//!
//! Adopt bounds checking incrementally in your existing code with these four steps:
//!
//! **Step 1**: Replace raw numeric types with constructors
//!
//! ```rust
//! use r3bl_tui::{col, width};
//! let pos_x = col(5); // Instead of let pos_x = 5_usize;
//! let width = width(10); // Instead of let width = 10_usize;
//! ```
//!
//! **Step 2**: Replace manual bounds checks with safe methods
//!
//! ```rust
//! # use r3bl_tui::{ArrayBoundsCheck, ArrayOverflowResult, col, width, IndexOps};
//! # let pos_x = col(5);
//! # let width = width(10);
//! if pos_x.overflows(width) == ArrayOverflowResult::Within { /* safe access */ }
//! // Instead of: if pos_x < width { /* manual check without type safety */ }
//! ```
//!
//! **Step 3**: Add pattern matching for array access (buffer/vector elements)
//!
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
//!
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
//! <div class="warning">
//!
//! Steps 3 and 4 show different semantic domains. Choose the one that matches your use
//! case:
//!
//! - **Step 3** ([`ArrayBoundsCheck`]): Buffer/array element access where `index <
//!   length`
//! - **Step 4** ([`CursorBoundsCheck`]): Text cursor positioning where `index <= length`
//!   (allows cursor after last character)
//!
//! See the [semantic trait distinctions] section for
//! details.
//!
//! </div>
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
//! scroll regions, and text selections, see the [When to Use What]
//! section and the [Decision Tree].
//!
//! For comprehensive details on each trait's methods and edge cases, see the individual
//! module documentation files. This guide gets you productive quickly, while the detailed
//! trait docs cover advanced patterns and special cases.
//!
//! **For deeper understanding**: See [Example: Type System in Action] to see how the type
//! system prevents common errors at compile time.
//!
//! ### Common Mistakes to Avoid
//!
//! **❌ Don't mix row and column types**
//!
//! ```rust,compile_fail
//! use r3bl_tui::{row, width, IndexOps};  // IndexOps provides .overflows()
//! // Compiler error - cannot compare RowIndex with ColWidth
//! let row_pos = row(5);
//! let col_width = width(10);
//! row_pos.overflows(col_width); // Won't compile!
//! ```
//!
//! **❌ Don't use raw usize for bounds checking**
//!
//! ```rust
//! let raw_index: usize = 5;
//! let raw_length: usize = 10;
//! // Error-prone - no protection against off-by-one bugs
//! if raw_index < raw_length { /* unsafe! */ }
//! ```
//!
//! **✅ Do use type-safe constructors and methods**
//!
//! ```rust
//! use r3bl_tui::{ArrayBoundsCheck, ArrayOverflowResult, col, width, IndexOps};
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
//! This separation, enforced through the [`IndexOps`] and [`LengthOps`]
//! traits, prevents entire categories of off-by-one errors and type confusion at compile
//! time.
//!
//! ### Trait Hierarchy
//!
//! Both [`IndexOps`] and [`LengthOps`] build on top of [`NumericValue`] as their
//! super-trait, which in turn extends [`NumericConversions`]:
//!
//! ```text
//! Trait Hierarchy:
//!
//!                NumericConversions
//!                   (base trait)
//!                        │
//!                        │ Provides: as_usize(), as_u16()
//!                        │ Purpose: Reading numeric values
//!                        │
//!                        ▼
//!                   NumericValue
//!                 (extends above)
//!                        │
//!                        │ Adds: From<usize>, From<u16>, is_zero()
//!                        │ Purpose: Construction + zero checking
//!                        │
//!           ┌────────────┴────────────┐
//!           │                         │
//!      IndexOps                   LengthOps
//!      (0-based)                  (1-based)
//!           │                         │
//!           ▼                         ▼
//!   Adds: overflows(),        Adds: is_overflowed_by(),
//!         underflows(),             remaining_from(),
//!         clamp_to_*(),             convert_to_index(),
//!         convert_to_length()       clamp_to_max()
//! ```
//!
//! - **[`NumericConversions`]** - The base trait providing numeric reading operations
//!   ([`as_usize()`], [`as_u16()`]). Use this when you only need to read values without
//!   constructing new ones (most common case in generic functions).
//!
//! - **[`NumericValue`]** - Extends [`NumericConversions`] with construction
//!   ([`From<usize>`], [`From<u16>`]) and zero checking ([`is_zero()`]). Use this when
//!   you need to create values or check for zero in generic functions.
//!
//! - **[`IndexOps`]** - Extends [`NumericValue`] with 0-based position semantics and
//!   bounds checking operations specific to array indexing.
//!
//! - **[`LengthOps`]** - Extends [`NumericValue`] with 1-based size semantics and space
//!   calculation operations specific to container sizes.
//!
//! This hierarchy enables both generic operations (via [`NumericConversions`] or
//! [`NumericValue`]) and specialized, type-safe operations (via [`IndexOps`] and
//! [`LengthOps`]).
//!
//! ### The [`IndexOps`] Trait - index or position operations
//!
//! [`IndexOps`] identifies types that represent positions within content. These are
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
//! | Aspect            | Description                                                                                                                                                                         |
//! | ----------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
//! | Purpose           | Marker trait for 0-based index or position indicators with comprehensive bounds checking                                                                                            |
//! | Types             | [`Index`], [`RowIndex`], [`ColIndex`], [`ByteIndex`]                                                                                                                                |
//! | Associated Type   | `LengthType` - The corresponding 1-based length or size type: [`Index`] -> [`Length`], [`RowIndex`] -> [`RowHeight`], [`ColIndex`] -> [`ColWidth`], [`ByteIndex`] -> [`ByteLength`] |
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
//! Each [`IndexOps`] has an associated type
//! `LengthType` that must itself have an `IndexType` pointing back, creating a
//! bidirectional type-safe relationship. This prevents comparing incompatible types like
//! [`RowIndex`] with [`ColWidth`].
//!
//! ### The [`LengthOps`] Trait - length or size operations
//!
//! [`LengthOps`] identifies types that represent sizes or measurements of content.
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
//! | Aspect            | Description                                                                                                                                                                           |
//! | ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
//! | Purpose           | Marker trait for 1-based size measurements with space calculation capabilities                                                                                                        |
//! | Types             | [`Length`], [`RowHeight`], [`ColWidth`], [`ByteLength`]                                                                                                                               |
//! | Associated Type   | `IndexType` - The corresponding 0-based index or position type: [`Length`] -> [`Index`], [`RowHeight`] -> [`RowIndex`], [`ColWidth`] -> [`ColIndex`], [`ByteLength`] -> [`ByteIndex`] |
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
//! Each [`LengthOps`] has an associated type `IndexType` that must itself have an
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
//!     IndexOps                       LengthOps
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
//! □ row_index.overflows(col_width)
//! □ col_index.overflows(row_height)
//! ■ row_index.overflows(row_height)
//! ■ col_index.overflows(col_width)
//!
//! Legend: □ Won't compile | ■ Type-safe and compiles
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
//!   - Located in `vt_100_pty_output_parser::term_units` module
//!   - Used exclusively for CSI sequence parsing (`ESC[row;colH`)
//!   - Convert to/from [`RowIndex`]/[`ColIndex`] for buffer operations
//!   - **Not paired**: Both are 1-based positions, neither represents a size/length
//!   - **Different domain**: Terminal protocol coordinates, not buffer bounds checking
//!
//! <div class="warning">
//!
//! Don't confuse [`TermRow`] (1-based terminal coordinate) with [`RowIndex`]
//! (0-based buffer position) or [`RowHeight`] (1-based buffer size). The bounds
//! checking system works on buffer coordinates, while [`TermRow`]/[`TermCol`] are for
//! VT-100 parsing.
//!
//! </div>
//!
//! ### Type Safety Guarantees
//!
//! The [`IndexOps`] and [`LengthOps`] traits, combined with their bidirectional
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
//!    - Write generic code once using [`IndexOps`] / [`LengthOps`]
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
//! ### The Semantic Traits Layer
//!
//! On top of the foundational traits ([`IndexOps`] and [`LengthOps`]), the bounds
//! checking system provides **semantic traits** that implement specific use-case
//! validation. These traits leverage the type-safe operations from the foundational layer
//! to provide domain-specific bounds checking behaviors.
//!
//! ```text
//! Three-Layer Type System Architecture:
//!
//! ┌──────────────────────────────────────────────────┐
//! │   Semantic Traits Layer (Use Cases)              │
//! ├──────────────────────────────────────────────────┤
//! │ • ArrayBoundsCheck                               │
//! │   - overflows(), underflows()                    │
//! │ • CursorBoundsCheck                              │
//! │   - check_cursor_position_bounds()               │
//! │   - eol_cursor_position()                        │
//! │   - is_valid_cursor_position()                   │
//! │   - clamp_cursor_position()                      │
//! │ • ViewportBoundsCheck                            │
//! │   - check_viewport_bounds()                      │
//! │ • RangeBoundsExt                                 │
//! │   - check_range_is_valid_for_length()            │
//! │   - check_index_is_within()                      │
//! │   - clamp_range_to()                             │
//! │ • RangeConvertExt                                │
//! │   - to_exclusive()                               │
//! └─────────────────┬────────────────────────────────┘
//!                   │ builds on
//! ┌─────────────────▼──────────────────────────────┐
//! │   Foundational Traits (Operations)             │
//! ├────────────────────────────────────────────────┤
//! │ • IndexOps (0-based positions)                 │
//! │   - clamp_to_*(), clamp_to_range()             │
//! │   - convert_to_length()                        │
//! │ • LengthOps (1-based sizes)                    │
//! │   - is_overflowed_by(), remaining_from()       │
//! │   - convert_to_index(), clamp_to_max()         │
//! └─────────────────┬──────────────────────────────┘
//!                   │ extends
//! ┌─────────────────▼───────────────────┐
//! │   Base Trait (Conversions)          │
//! ├─────────────────────────────────────┤
//! │ • NumericValue                      │
//! │   - as_usize(), as_u16(), is_zero() │
//! └─────────────────────────────────────┘
//! ```
//!
//! #### Trait Requirements and Relationships
//!
//! Each semantic trait has specific requirements from the foundational layer:
//!
//! | Semantic Trait            | Required Foundational Trait                        | Purpose                                                    |
//! | ------------------------- | -------------------------------------------------- | ---------------------------------------------------------- |
//! | [`ArrayBoundsCheck`]      | [`IndexOps`] (for the index type)                  | Validates `index < length` for safe array access           |
//! | [`CursorBoundsCheck`]     | [`LengthOps`] (auto-implemented)                   | Validates `index <= length` for cursor positioning         |
//! | [`ViewportBoundsCheck`]   | [`IndexOps`] (auto-implemented)                    | Checks if index is within viewport `[start, start+size)`   |
//! | [`RangeBoundsExt`]        | Associated types with [`IndexOps`]/[`LengthOps`]   | Validates range structure and membership                   |
//! | [`RangeConvertExt`]       | Associated types with [`IndexOps`]                 | Converts between inclusive/exclusive ranges                |
//!
//! #### How Semantic Traits Build on Foundational Traits
//!
//! The semantic traits don't duplicate functionality - they compose the foundational
//! operations to implement specific validation patterns:
//!
//! ```no_run
//! # use r3bl_tui::{ArrayOverflowResult, LengthOps, NumericValue};
//! /// Actual implementation from ArrayBoundsCheck showing how it builds
//! /// on foundational traits
//! pub trait ArrayBoundsCheck<LengthType: LengthOps>
//! where
//!     Self: NumericValue,  // ← Requires base trait for numeric operations
//! {
//!     fn overflows(&self, arg_length: impl Into<LengthType>) -> ArrayOverflowResult
//!     where
//!         LengthType: LengthOps<IndexType = Self>,  // ← Bidirectional type constraint
//!     {
//!         let length: LengthType = arg_length.into();
//!
//!         // Uses NumericValue::is_zero() from base trait
//!         if length.is_zero() {
//!             return ArrayOverflowResult::Overflowed;  // Empty collection edge case
//!         }
//!
//!         // Uses LengthOps::convert_to_index() from foundational trait
//!         if *self > length.convert_to_index() {
//!             ArrayOverflowResult::Overflowed
//!         } else {
//!             ArrayOverflowResult::Within
//!         }
//!     }
//! }
//! ```
//!
//! #### Semantic Trait Characteristics
//!
//! **[`ArrayBoundsCheck`]**:
//! - **Implements on**: Types with [`IndexOps`] (e.g., [`RowIndex`], [`ColIndex`])
//! - **Validates**: Array/buffer access safety (`index < length`)
//! - **Key methods**: [`overflows()`], [`underflows()`]
//! - **Use when**: Accessing array elements, buffer positions
//!
//! **[`CursorBoundsCheck`]**:
//! - **Implements on**: Types with [`LengthOps`] (e.g., [`RowHeight`], [`ColWidth`])
//! - **Validates**: Cursor can be at end position (`index <= length`)
//! - **Key methods**: [`check_cursor_position_bounds()`], [`eol_cursor_position()`]
//! - **Use when**: Text editing, cursor movement, selection endpoints
//!
//! **[`ViewportBoundsCheck`]**:
//! - **Implements on**: Types with [`IndexOps`] (auto-implemented via blanket impl)
//! - **Validates**: Content visibility in viewport (`start <= index < start+size`)
//! - **Key methods**: [`check_viewport_bounds()`]
//! - **Use when**: Rendering, scrolling, window clipping
//!
//! **[`RangeBoundsExt`]**:
//! - **Implements on**: [`Range<Index>`] and [`RangeInclusive<Index>`] types
//! - **Validates**: Range structure validity, index membership
//! - **Key methods**: [`check_range_is_valid_for_length()`], [`check_index_is_within()`]
//! - **Use when**: Iteration bounds, algorithm parameters, selections
//!
//! **[`RangeConvertExt`]**:
//! - **Implements on**: [`RangeInclusive<Index>`] types
//! - **Converts**: Inclusive ranges to exclusive for iteration
//! - **Key methods**: [`to_exclusive()`]
//! - **Use when**: VT-100 scroll regions, converting for Rust iteration
//!
//! #### Complete Type System Integration
//!
//! The semantic traits complete the type system by providing the actual bounds checking
//! behaviors that users interact with. They work seamlessly with the concrete types
//! through the foundational trait requirements:
//!
//! ```text
//! Concrete Type → Foundational Trait → Semantic Trait → Use Case
//!
//! Example flow:
//! ColIndex → implements IndexOps → enables ArrayBoundsCheck → validates buffer[col]
//! ColWidth → implements LengthOps → enables CursorBoundsCheck → validates cursor position
//! ```
//!
//! This three-layer architecture ensures:
//! - **Type safety**: Operations are only available on appropriate types
//! - **Composability**: Semantic traits build on foundational operations
//! - **Discoverability**: Users can find the right trait for their use case
//! - **Maintainability**: Clear separation of concerns across layers
//!
//! ### Implementation Patterns
//!
//! The bounds checking system uses two key Rust patterns to provide ergonomic APIs
//! while working within Rust's trait coherence rules:
//!
//! #### Pattern 1: Extension Traits (Orphan Rule Workaround)
//!
//! Rust's **orphan rule** prevents implementing foreign traits on foreign types. When
//! we need to add bounds checking methods to standard library types like [`Range`]`<T>`
//! or [`RangeInclusive`]`<T>`, we use **extension traits** with an "Ext" suffix.
//!
//! **Extension traits in this module:**
//!
//! | Trait                 | Target Type                                      | Purpose                                       |
//! | --------------------- | ------------------------------------------------ | --------------------------------------------- |
//! | [`RangeBoundsExt`]    | [`Range<Index>`] and [`RangeInclusive<Index>`]   | Validate range structure and membership       |
//! | [`RangeConvertExt`]   | [`RangeInclusive<Index>`]                        | Convert inclusive → exclusive for iteration   |
//!
//! **Why extension traits are needed:**
//!
//! ```text
//! □ Cannot do this (orphan rule violation):
//!   impl Range<ColIndex> {
//!       pub fn check_is_valid(...) { }  // Error: can't add methods to foreign type
//!   }
//!
//! ■ Instead, use extension trait:
//!   pub trait RangeBoundsExt { ... }
//!   impl RangeBoundsExt for Range<ColIndex> { ... }  // OK: impl our trait on foreign type
//! ```
//!
//! **How to use extension traits:**
//!
//! ```rust
//! use r3bl_tui::{col, width, RangeBoundsExt, RangeValidityStatus};
//!
//! let range = col(2)..col(8);
//! let buffer_length = width(10);
//!
//! // Extension trait method available after importing RangeBoundsExt
//! if range.check_range_is_valid_for_length(buffer_length) == RangeValidityStatus::Valid {
//!     // Safe to iterate
//! }
//! ```
//!
//! #### Pattern 2: Blanket Implementations (Zero Boilerplate)
//!
//! For traits that provide default implementations for all methods and don't have type
//! parameters, we use **blanket implementations** to automatically implement the trait
//! for all qualifying types.
//!
//! **Blanket implementations in this module:**
//!
//! | Trait                     | Blanket Impl                                    | Benefit                               |
//! | ------------------------- | ----------------------------------------------- | ------------------------------------- |
//! | [`CursorBoundsCheck`]     | `impl<T: LengthOps> CursorBoundsCheck for T`    | Auto-available on all length types    |
//! | [`ViewportBoundsCheck`]   | `impl<T: IndexOps> ViewportBoundsCheck for T`   | Auto-available on all index types     |
//!
//! **Without blanket impl (tedious boilerplate):**
//!
//! ```rust,compile_fail
//! # use r3bl_tui::{CursorBoundsCheck, ColWidth, RowHeight, Length, ByteLength};
//! impl CursorBoundsCheck for ColWidth {}
//! impl CursorBoundsCheck for RowHeight {}
//! impl CursorBoundsCheck for Length {}
//! impl CursorBoundsCheck for ByteLength {}
//! // ... repeat for every length type
//! ```
//!
//! **With blanket impl (write once, works everywhere):**
//!
//! ```rust,compile_fail
//! # use r3bl_tui::{CursorBoundsCheck, LengthOps};
//! // Single blanket impl in cursor_bounds_check.rs:
//! impl<T: LengthOps> CursorBoundsCheck for T
//! where
//!     T::IndexType: std::ops::Add<Output = T::IndexType>,
//! { }
//!
//! // Now available on ALL LengthOps types automatically!
//! ```
//!
//! **How blanket impls work:**
//!
//! ```rust
//! use r3bl_tui::{width, col, CursorBoundsCheck, CursorPositionBoundsStatus};
//!
//! let line_width = width(10);  // ColWidth type implements LengthOps
//!
//! // CursorBoundsCheck methods work automatically (blanket impl activated!)
//! let eol = line_width.eol_cursor_position();
//! assert_eq!(eol, col(10));
//! ```
//!
//! #### Pattern 3: Manual Implementations (When Blanket Impls Don't Work)
//!
//! Some traits **cannot** use blanket implementations due to **type parameters** that
//! would violate Rust's coherence rules.
//!
//! **Example: [`ArrayBoundsCheck`] requires manual impls**
//!
//! ```rust,compile_fail
//! # use r3bl_tui::{ArrayBoundsCheck, LengthOps, ColWidth, ColIndex, RowHeight, RowIndex};
//! // ArrayBoundsCheck is parameterized over LengthType
//! // (trait definition shown for reference)
//!
//! // Cannot use blanket impl (orphan rule violation)!
//! impl ArrayBoundsCheck<ColWidth> for ColIndex { }
//! impl ArrayBoundsCheck<RowHeight> for RowIndex { }
//! // Error: only traits defined in the current crate can be implemented
//! ```
//!
//! This is acceptable because [`ArrayBoundsCheck`] is typically invoked through
//! [`IndexOps`] methods that provide the ergonomic API, so users rarely interact with
//! the trait directly.
//!
//! #### Implementation Pattern Summary
//!
//! | Pattern                                | When to Use                                 | Examples                                         |
//! | -------------------------------------- | ------------------------------------------- | ------------------------------------------------ |
//! | **Extension Trait (Ext suffix)**       | Adding methods to foreign types (std lib)   | [`RangeBoundsExt`], [`RangeConvertExt`]          |
//! | **Blanket Implementation**             | Trait with no type params, all defaults     | [`CursorBoundsCheck`], [`ViewportBoundsCheck`]   |
//! | **Manual Implementation**              | Trait with type parameters                  | [`ArrayBoundsCheck<LengthType>`]                 |
//!
//! This combination of patterns provides maximum ergonomics while respecting Rust's
//! trait coherence rules and minimizing boilerplate code.
//!
//! ### Example: Type System in Action
//!
//! This example demonstrates how the type system guarantees prevent common errors at
//! compile time. For practical adoption guidance, see the [Getting Started with Bounds
//! Checking] section.
//!
//! ```rust
//! use r3bl_tui::{ArrayBoundsCheck, ArrayOverflowResult, IndexOps, LengthOps, row, col, height, width};
//!
//! // Type-safe terminal operations
//! let cursor_row = row(5);
//! let terminal_height = height(24);
//! let cursor_col = col(10);
//! let terminal_width = width(80);
//!
//! // The following work since types match.
//! if cursor_row.overflows(terminal_height) == ArrayOverflowResult::Within {
//!     println!("Row {} is valid", cursor_row.as_usize());
//! }
//! if cursor_col.overflows(terminal_width) == ArrayOverflowResult::Within {
//!     println!("Column {} is valid", cursor_col.as_usize());
//! }
//!
//! // The following won't compile since type mismatch caught at compile time!
//! // □ cursor_row.overflows(terminal_width);  // Can't compare RowIndex to ColWidth!
//! // □ cursor_col.overflows(terminal_height); // Can't compare ColIndex to RowHeight!
//!
//! // Conversions are explicit and type-safe
//! let row_as_length = cursor_row.convert_to_length();  // RowIndex → RowHeight
//! let last_col = terminal_width.convert_to_index();    // ColWidth → ColIndex
//! ```
//!
//! ### Related Types Outside the Bounds System
//!
//! Some types work with indices and lengths but don't participate in the
//! [`IndexOps`]/[`LengthOps`] type system:
//!
//! - [`ByteOffset`] - Represents relative distances or offsets (not absolute positions or
//!   sizes). Used for specialized calculations like gap buffer operations in the
//!   zero-copy editor implementation. Unlike [`ByteIndex`] and [`ByteLength`] which form
//!   a standard index/length pair, [`ByteOffset`] is intentionally separate from the
//!   bounds checking system.
//!
//! [Decision Tree]: #decision-tree-which-trait-do-i-need
//! [Example: Type System in Action]: #example-type-system-in-action
//! [Exclusive vs Inclusive Range Comparison]: mod@crate::core::coordinates::bounds_check::range_bounds_check_ext#exclusive-vs-inclusive-range-comparison
//! [Getting Started with Bounds Checking]: #getting-started-with-bounds-checking
//! [Type System Foundation]: #type-system-foundation
//! [When to Use What]: #when-to-use-what
//! [`ArrayBoundsCheck`]: crate::ArrayBoundsCheck
//! [`ArrayOverflowResult`]: crate::ArrayOverflowResult
//! [`ByteIndex`]: crate::ByteIndex
//! [`ByteLength`]: crate::ByteLength
//! [`ByteOffset`]: crate::ByteOffset
//! [`ColIndex`]: crate::ColIndex
//! [`ColWidth`]: crate::ColWidth
//! [`CursorBoundsCheck`]: crate::CursorBoundsCheck
//! [`CursorPositionBoundsStatus`]: crate::CursorPositionBoundsStatus
//! [`From<u16>`]: std::convert::From
//! [`From<usize>`]: std::convert::From
//! [`IndexOps`]: crate::IndexOps
//! [`Index`]: crate::Index
//! [`LengthOps`]: crate::LengthOps
//! [`Length`]: crate::Length
//! [`NumericConversions`]: crate::NumericConversions
//! [`NumericValue`]: crate::NumericValue
//! [`Range<Index>`]: std::ops::Range
//! [`RangeBoundsExt`]: crate::RangeBoundsExt
//! [`RangeBoundsResult`]: crate::RangeBoundsResult
//! [`RangeConvertExt`]: crate::RangeConvertExt
//! [`RangeInclusive<Index>`]: std::ops::RangeInclusive
//! [`RangeInclusive`]: std::ops::RangeInclusive
//! [`RangeValidityStatus`]: crate::RangeValidityStatus
//! [`Range`]: std::ops::Range
//! [`RowHeight`]: crate::RowHeight
//! [`RowIndex`]: crate::RowIndex
//! [`TermCol`]: crate::TermCol
//! [`TermRow`]: crate::TermRow
//! [`ViewportBoundsCheck`]: crate::ViewportBoundsCheck
//! [`array_bounds_check.rs`]: mod@crate::array_bounds_check
//! [`as_u16()`]: crate::NumericConversions::as_u16
//! [`as_u16`]: NumericConversions::as_u16
//! [`as_usize()`]: crate::NumericConversions::as_usize
//! [`as_usize`]: NumericConversions::as_usize
//! [`check_cursor_position_bounds()`]: crate::CursorBoundsCheck::check_cursor_position_bounds
//! [`check_index_is_within()`]: crate::RangeBoundsExt::check_index_is_within
//! [`check_range_is_valid_for_length()`]: crate::RangeBoundsExt::check_range_is_valid_for_length
//! [`check_viewport_bounds()`]: crate::ViewportBoundsCheck::check_viewport_bounds
//! [`clamp_cursor_position()`]: crate::CursorBoundsCheck::clamp_cursor_position
//! [`clamp_range_to()`]: crate::RangeBoundsExt::clamp_range_to
//! [`clamp_to_max()`]: crate::LengthOps::clamp_to_max
//! [`clamp_to_max_length()`]: crate::IndexOps::clamp_to_max_length
//! [`clamp_to_min_index()`]: crate::IndexOps::clamp_to_min_index
//! [`clamp_to_range()`]: crate::IndexOps::clamp_to_range
//! [`col()`]: crate::col
//! [`convert_to_index()`]: crate::LengthOps::convert_to_index
//! [`convert_to_length()`]: crate::IndexOps::convert_to_length
//! [`cursor_bounds_check.rs`]: mod@crate::cursor_bounds_check
//! [`eol_cursor_position()`]: crate::CursorBoundsCheck::eol_cursor_position
//! [`height()`]: crate::height
//! [`index.clamp_to_max_length(length)`]: crate::IndexOps::clamp_to_max_length
//! [`index.clamp_to_min_index(min_index)`]: crate::IndexOps::clamp_to_min_index
//! [`index.clamp_to_range(range)`]: crate::IndexOps::clamp_to_range
//! [`index.convert_to_length()`]: crate::IndexOps::convert_to_length
//! [`index.overflows(length)`]: crate::ArrayBoundsCheck::overflows
//! [`index.underflows(min_index)`]: crate::ArrayBoundsCheck::underflows
//! [`index_from_end()`]: crate::LengthOps::index_from_end
//! [`index_ops.rs`]: mod@crate::index_ops
//! [`index_ops`]: mod@crate::index_ops
//! [`is_overflowed_by()`]: crate::LengthOps::is_overflowed_by
//! [`is_valid_cursor_position()`]: crate::CursorBoundsCheck::is_valid_cursor_position
//! [`is_zero()`]: crate::NumericValue::is_zero
//! [`is_zero`]: NumericValue::is_zero
//! [`length.clamp_to_max(max)`]: crate::LengthOps::clamp_to_max
//! [`length.convert_to_index()`]: crate::LengthOps::convert_to_index
//! [`length.is_overflowed_by(index)`]: crate::LengthOps::is_overflowed_by
//! [`length.remaining_from(index)`]: crate::LengthOps::remaining_from
//! [`length_ops.rs`]: mod@crate::length_ops
//! [`length_ops`]: mod@crate::length_ops
//! [`numeric_value`]: mod@crate::numeric_value
//! [`overflows()`]: crate::ArrayBoundsCheck::overflows
//! [`range_bounds_check_ext.rs`]: mod@crate::range_bounds_check_ext
//! [`range_bounds_check_ext`]: mod@crate::range_bounds_check_ext
//! [`range_convert_ext.rs`]: mod@crate::range_convert_ext
//! [`range_convert_ext`]: mod@crate::range_convert_ext
//! [`remaining_from()`]: crate::LengthOps::remaining_from
//! [`result_enums.rs`]: mod@crate::result_enums
//! [`row()`]: crate::row
//! [`to_exclusive()`]: crate::RangeConvertExt::to_exclusive
//! [`underflows()`]: crate::ArrayBoundsCheck::underflows
//! [`viewport_bounds_check.rs`]: mod@crate::viewport_bounds_check
//! [`viewport_bounds_check`]: mod@crate::viewport_bounds_check
//! [`width()`]: crate::width
//! [semantic trait distinctions]: #semantic-trait-distinctions

// Attach.
pub mod array_bounds_check;
pub mod cursor_bounds_check;
pub mod index_ops;
pub mod length_ops;
pub mod numeric_value;
pub mod range_bounds_check_ext;
pub mod range_convert_ext;
pub mod result_enums;
pub mod viewport_bounds_check;

// Re-export.
pub use array_bounds_check::*;
pub use cursor_bounds_check::*;
pub use index_ops::*;
pub use length_ops::*;
pub use numeric_value::*;
pub use range_bounds_check_ext::*;
pub use range_convert_ext::*;
pub use result_enums::*;
pub use viewport_bounds_check::*;

// Integration tests.
#[cfg(test)]
mod integration_tests;
