## Complete Documentation Examples

This document provides full examples of well-documented Rust code following the inverted pyramid principle.

---

## Example 1: Module-Level Documentation

```rust
//! # Terminal State Management
//!
//! This module provides state management for terminal emulator applications.
//! It handles cursor positioning, scrollback buffers, and terminal attributes
//! using a type-safe approach with bounds checking.
//!
//! ## When to Use
//!
//! Use this module when you need to:
//! - Build a terminal emulator or TUI application
//! - Manage cursor state with bounds checking
//! - Handle scrollback buffers efficiently
//! - Track terminal attributes (colors, styles)
//!
//! ## Architecture Overview
//!
//! ```text
//! ┌─────────────────┐
//! │ TerminalState   │
//! └────────┬────────┘
//!          │
//!          ├──▶ CursorState (position, visibility)
//!          ├──▶ ScrollbackBuffer (history, lines)
//!          └──▶ AttributeSet (colors, styles)
//! ```
//!
//! ## Example Usage
//!
//! ```rust
//! use terminal_state::{TerminalState, Size};
//!
//! // Create a new terminal state with 80x24 size
//! let mut term = TerminalState::new(Size::new(80, 24));
//!
//! // Move cursor safely (bounds-checked)
//! term.move_cursor_to(row(10), col(5))?;
//!
//! // Write text at cursor position
//! term.write_str("Hello, world!")?;
//!
//! // Access scrollback history
//! let history = term.scrollback().lines();
//! ```
//!
//! ## Common Pitfalls
//!
//! ❌ **Don't use raw `usize` for positions:**
//! ```rust,ignore
//! let row = 10_usize;  // Could be row index, col index, or length?
//! ```
//!
//! ✅ **Use type-safe wrappers:**
//! ```rust
//! let row = row(10);   // Clearly a RowIndex
//! let col = col(5);    // Clearly a ColIndex
//! ```
//!
//! ## Performance Notes
//!
//! - Scrollback buffer uses a circular buffer for O(1) insertion
//! - Cursor operations are bounds-checked at compile-time where possible
//! - Rendering uses dirty tracking to minimize redraws

// Module contents here...
```

**What makes this good:**

- ✅ Clear purpose statement at the top
- ✅ "When to Use" section helps readers decide if this is the right module
- ✅ ASCII diagram shows architecture at a glance
- ✅ Complete example shows typical usage workflow
- ✅ Common pitfalls with good vs bad examples
- ✅ Performance considerations for users who care

---

## Example 2: Trait-Level Documentation

```rust
/// A terminal backend that can render text and handle input.
///
/// This trait abstracts over different terminal implementations (ncurses, termion,
/// crossterm, etc.) to provide a unified interface for terminal applications.
///
/// # When to Implement
///
/// Implement this trait when you want to:
/// - Add support for a new terminal library
/// - Create a custom rendering backend (e.g., offscreen buffer for testing)
/// - Mock terminal behavior for tests
///
/// # Architecture
///
/// ```text
/// ┌──────────────┐
/// │ Application  │
/// └──────┬───────┘
///        │ uses
///        ▼
/// ┌──────────────┐
/// │   Backend    │ ◀─── This trait
/// │  (trait)     │
/// └──────┬───────┘
///        │ implements
///        ▼
/// ┌──────────────┬──────────────┬──────────────┐
/// │   Crossterm  │   Termion    │  Offscreen   │
/// │   Backend    │   Backend    │   Buffer     │
/// └──────────────┴──────────────┴──────────────┘
/// ```
///
/// # Example Implementation
///
/// ```rust
/// use terminal::{Backend, Result, Position, Style};
///
/// struct MyBackend {
///     // Implementation-specific fields
/// }
///
/// impl Backend for MyBackend {
///     fn write_at(&mut self, pos: Position, text: &str) -> Result<()> {
///         // Move cursor to position
///         // Write text
///         // Return result
///         Ok(())
///     }
///
///     fn set_style(&mut self, style: Style) -> Result<()> {
///         // Apply foreground/background colors
///         // Apply text attributes (bold, italic, etc.)
///         Ok(())
///     }
///
///     // Implement other required methods...
/// }
/// ```
///
/// # Complete Workflow Example
///
/// ```rust
/// use terminal::{Backend, CrosstermBackend, Position, Style};
///
/// // Create backend
/// let mut backend = CrosstermBackend::new()?;
///
/// // Set up styling
/// backend.set_style(Style::default()
///     .fg_color(Color::Blue)
///     .bold())?;
///
/// // Write text
/// backend.write_at(Position::new(row(0), col(0)), "Hello!")?;
///
/// // Render to screen
/// backend.flush()?;
/// ```
///
/// # Common Mistakes
///
/// ## ❌ Forgetting to flush
///
/// ```rust,ignore
/// backend.write_at(pos, "text")?;
/// // Screen doesn't update! Need to flush.
/// ```
///
/// ## ✅ Always flush after writes
///
/// ```rust
/// backend.write_at(pos, "text")?;
/// backend.flush()?;  // Now it appears on screen
/// ```
///
/// # Thread Safety
///
/// This trait is **not** `Send` or `Sync` by default, as most terminal backends
/// are not thread-safe. If you need concurrent access, wrap in `Arc<Mutex<_>>`:
///
/// ```rust
/// use std::sync::{Arc, Mutex};
///
/// let backend = Arc::new(Mutex::new(CrosstermBackend::new()?));
///
/// // Clone for use in threads
/// let backend_clone = Arc::clone(&backend);
/// ```
pub trait Backend {
    /// Write text at the specified position.
    ///
    /// See [trait-level documentation](Backend#example-implementation) for usage.
    fn write_at(&mut self, pos: Position, text: &str) -> Result<()>;

    /// Set the current text style for subsequent writes.
    ///
    /// See [trait-level documentation](Backend#example-implementation) for usage.
    fn set_style(&mut self, style: Style) -> Result<()>;

    // Other methods...
}
```

**What makes this good:**

- ✅ Clear purpose and abstraction at the top
- ✅ "When to Implement" guides users
- ✅ Architecture diagram shows relationship to other components
- ✅ Complete implementation example
- ✅ Full workflow showing typical usage
- ✅ Common mistakes with corrections
- ✅ Thread safety considerations
- ✅ Method docs reference trait-level examples (no duplication)

---

## Example 3: Struct-Level Documentation

```rust
/// An offscreen terminal buffer for testing and rendering.
///
/// `OffscreenBuffer` simulates a terminal without actually rendering to the screen.
/// It's useful for:
/// - Unit testing terminal applications
/// - Rendering frames before displaying them
/// - Capturing terminal output for analysis
///
/// # Example
///
/// ```rust
/// use offscreen_buffer::{OffscreenBuffer, Size};
///
/// // Create 80x24 buffer
/// let mut buffer = OffscreenBuffer::new(Size::new(80, 24));
///
/// // Write some text
/// buffer.write_at(row(0), col(0), "Hello!")?;
/// buffer.write_at(row(1), col(0), "World!")?;
///
/// // Get the rendered content
/// let content = buffer.as_string();
/// assert!(content.contains("Hello!"));
/// assert!(content.contains("World!"));
/// ```
///
/// # Internal Structure
///
/// ```text
/// ┌──────────────────────────┐
/// │   OffscreenBuffer        │
/// ├──────────────────────────┤
/// │ - cells: Vec<Vec<Cell>>  │ ◀── 2D array of cells
/// │ - size: Size             │ ◀── Buffer dimensions
/// │ - cursor: Position       │ ◀── Current cursor position
/// │ - style: Style           │ ◀── Current text style
/// └──────────────────────────┘
/// ```
///
/// # Performance
///
/// - Cell access: O(1) using row/column indices
/// - Rendering: O(rows × cols) when converting to string
/// - Memory: ~16 bytes per cell (size-dependent)
///
/// For a typical 80×24 terminal: ~30KB memory usage.
pub struct OffscreenBuffer {
    cells: Vec<Vec<Cell>>,
    size: Size,
    cursor: Position,
    style: Style,
}

impl OffscreenBuffer {
    /// Creates a new offscreen buffer with the specified size.
    ///
    /// # Example
    ///
    /// ```
    /// use offscreen_buffer::{OffscreenBuffer, Size};
    ///
    /// let buffer = OffscreenBuffer::new(Size::new(80, 24));
    /// ```
    pub fn new(size: Size) -> Self {
        // Implementation...
    }

    /// Writes text at the current cursor position.
    ///
    /// The cursor automatically advances as text is written.
    ///
    /// # Example
    ///
    /// ```
    /// let mut buffer = OffscreenBuffer::new(Size::new(80, 24));
    /// buffer.write("Hello")?;
    /// buffer.write(" ")?;
    /// buffer.write("World")?;
    /// // Cursor is now at position (0, 11)
    /// ```
    pub fn write(&mut self, text: &str) -> Result<()> {
        // Implementation...
    }

    // More methods with minimal examples...
}
```

**What makes this good:**

- ✅ Clear purpose and use cases
- ✅ Practical example at struct level
- ✅ Internal structure diagram
- ✅ Performance characteristics documented
- ✅ Method examples are minimal (just syntax)
- ✅ References struct-level docs for complete workflows

---

## Example 4: Function-Level Documentation

```rust
/// Converts a VT-100 escape sequence to terminal actions.
///
/// This function parses ANSI escape sequences and returns the corresponding
/// terminal action (cursor movement, color change, etc.).
///
/// # Arguments
///
/// * `sequence` - The escape sequence to parse (e.g., "\x1b[1;31m")
///
/// # Returns
///
/// * `Ok(Action)` - Successfully parsed action
/// * `Err(ParseError)` - Invalid or unsupported sequence
///
/// # Example
///
/// ```
/// use vt100::{parse_sequence, Action};
///
/// // Parse color change sequence
/// let action = parse_sequence("\x1b[1;31m")?;
/// assert_eq!(action, Action::SetFgColor(Color::Red));
///
/// // Parse cursor movement
/// let action = parse_sequence("\x1b[10;5H")?;
/// assert_eq!(action, Action::MoveCursor { row: row(9), col: col(4) });
/// ```
///
/// # Supported Sequences
///
/// | Sequence       | Action                  | Example      |
/// |----------------|-------------------------|--------------|
/// | `\x1b[<n>A`    | Move cursor up          | `\x1b[5A`    |
/// | `\x1b[<n>;<m>H`| Move cursor to position | `\x1b[10;20H`|
/// | `\x1b[<n>m`    | Set graphics mode       | `\x1b[1m`    |
///
/// See [VT-100 documentation](https://vt100.net/docs/vt100-ug/) for complete reference.
///
/// # Errors
///
/// Returns `ParseError` if:
/// - Sequence is malformed
/// - Sequence type is unsupported
/// - Parameters are out of valid range
pub fn parse_sequence(sequence: &str) -> Result<Action, ParseError> {
    // Implementation...
}
```

**What makes this good:**

- ✅ Clear purpose
- ✅ Arguments and returns documented
- ✅ Concrete examples with assertions
- ✅ Table of supported sequences for quick reference
- ✅ Error conditions documented
- ✅ Links to external documentation

---

## Example 5: Module with Graduated Documentation

```rust
//! # Bounds Checking Utilities
//!
//! Type-safe bounds checking for array access, cursor positioning, and viewport calculations.
//!
//! ## The Problem
//!
//! Raw `usize` values are ambiguous and error-prone:
//!
//! ```rust,ignore
//! let x = 10;  // Is this an index (0-based) or a length (1-based)?
//! if x < length {  // Off-by-one error waiting to happen
//!     buffer[x]
//! }
//! ```
//!
//! ## The Solution
//!
//! Use type-safe wrappers:
//!
//! ```rust
//! use bounds::{Index, Length, ArrayBoundsCheck};
//!
//! let index = idx(10);      // Clearly an index (0-based)
//! let length = len(100);    // Clearly a length (1-based)
//!
//! if index.overflows(length) {
//!     // Safely caught!
//! }
//! ```
//!
//! ## Quick Reference
//!
//! | Use Case          | Trait              | Key Method                      |
//! |-------------------|--------------------|---------------------------------|
//! | Array access      | `ArrayBoundsCheck` | `index.overflows(length)`       |
//! | Cursor position   | `CursorBoundsCheck`| `length.check_cursor_bounds(pos)`|
//! | Viewport visibility| `ViewportBoundsCheck`| `index.check_viewport_bounds()`|
//!
//! See each trait's documentation for detailed examples.

/// Checks if an array index is within bounds (0-based).
///
/// # Laws
///
/// For valid array access `array[index]`:
/// - Index must satisfy: `0 <= index < length`
/// - Or equivalently: `index < length` (since Index is always >= 0)
///
/// # Example
///
/// ```
/// use bounds::{idx, len, ArrayBoundsCheck, ArrayOverflowResult};
///
/// let buffer_length = len(100);
/// let index = idx(50);
///
/// match index.overflows(buffer_length) {
///     ArrayOverflowResult::Within => {
///         // Safe to access: buffer[50]
///     }
///     ArrayOverflowResult::Overflows => {
///         // Out of bounds!
///     }
/// }
/// ```
pub trait ArrayBoundsCheck {
    fn overflows(&self, length: Length) -> ArrayOverflowResult;
}

/// Checks if a cursor position is valid (can be at end).
///
/// # Laws
///
/// For valid cursor positioning in text:
/// - Cursor must satisfy: `0 <= position <= length`
/// - Note: Cursor CAN be at position `length` (after last character)
///
/// # Example
///
/// ```
/// use bounds::{idx, len, CursorBoundsCheck, CursorPositionBoundsStatus};
///
/// let text_length = len(10);
/// let cursor = idx(10);  // At the end (after last char)
///
/// match text_length.check_cursor_position_bounds(cursor) {
///     CursorPositionBoundsStatus::Within => {
///         // Valid cursor position (can insert text here)
///     }
///     CursorPositionBoundsStatus::Overflows => {
///         // Invalid position!
///     }
/// }
/// ```
pub trait CursorBoundsCheck {
    fn check_cursor_position_bounds(&self, position: Index) -> CursorPositionBoundsStatus;
}
```

**What makes this good:**

- ✅ Module-level: The problem and solution clearly stated
- ✅ Quick reference table for choosing the right trait
- ✅ Trait-level: Mathematical laws documented
- ✅ Concrete examples showing the difference (array vs cursor)
- ✅ Graduated complexity: overview → traits → methods

---

## Summary of Principles

When writing documentation following these examples:

1. **Start broad** - Module/trait level explains "why" and "when"
2. **Use visuals** - ASCII diagrams for architecture
3. **Show complete workflows** - Real-world usage examples
4. **Keep methods minimal** - Just syntax, reference higher-level docs
5. **Document pitfalls** - Show good vs bad examples
6. **Include tables** - Quick reference for complex APIs
7. **Link generously** - Connect related documentation
8. **Test your examples** - All code blocks should compile/run

This creates documentation that teaches concepts at the top and provides quick reference at the bottom!
