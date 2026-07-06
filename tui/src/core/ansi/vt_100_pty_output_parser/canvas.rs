// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{EraseDisplayMode, EraseLineMode, Length, Pos, RowIndex};
use std::ops::Range;

/// The fundamental [`VT-100`] abstraction. The [`OfsBufVT100`] parser uses this trait via
/// an accessor [`get_active_canvas()`], allowing the [`VT-100`] shim operations to
/// interact directly with the trait object without duplicating all these methods on
/// [`OfsBufVT100`] itself.
///
/// [`get_active_canvas()`]:
///     crate::core::ansi::vt_100_pty_output_parser::OfsBufVT100::get_active_canvas
/// [`OfsBufVT100`]: crate::core::ansi::vt_100_pty_output_parser::OfsBufVT100
/// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
#[allow(rustdoc::broken_intra_doc_links)]
pub trait Canvas {
    // --- Cursor Operations ---

    /// Gets the current cursor position.
    fn get_cursor_pos(&self) -> Pos;

    /// Sets the cursor position to an exact coordinate. Maps to `CSI H` or `CSI f`.
    fn set_cursor_pos(&mut self, pos: Pos);

    /// Moves the cursor up by `how_many` cells. Maps to `CSI A` ([`CUU`]).
    ///
    /// [`CUU`]: https://vt100.net/docs/vt510-rm/CUU.html
    fn move_cursor_up(&mut self, how_many: Length);

    /// Moves the cursor down by `how_many` cells. Maps to `CSI B` ([`CUD`]).
    ///
    /// [`CUD`]: https://vt100.net/docs/vt510-rm/CUD.html
    fn move_cursor_down(&mut self, how_many: Length);

    /// Moves the cursor forward (right) by `how_many` cells. Maps to `CSI C` ([`CUF`]).
    ///
    /// [`CUF`]: https://vt100.net/docs/vt510-rm/CUF.html
    fn move_cursor_right(&mut self, how_many: Length);

    /// Moves the cursor backward (left) by `how_many` cells. Maps to `CSI D` ([`CUB`]).
    ///
    /// [`CUB`]: https://vt100.net/docs/vt510-rm/CUB.html
    fn move_cursor_left(&mut self, how_many: Length);

    // --- Character Operations ---

    /// Prints a single character at the current cursor position and advances the cursor.
    fn print_char(&mut self, ch: char);

    /// Inserts `how_many` blank characters at the cursor position. Existing characters
    /// shift to the right. Maps to `CSI @` ([`ICH`]).
    ///
    /// [`ICH`]: https://vt100.net/docs/vt510-rm/ICH.html
    fn insert_chars(&mut self, how_many: Length);

    /// Deletes `how_many` characters at the cursor position. Characters to the right of
    /// the cursor shift left to fill the gap. Maps to `CSI P` ([`DCH`]).
    ///
    /// [`DCH`]: https://vt100.net/docs/vt510-rm/DCH.html
    fn delete_chars(&mut self, how_many: Length);

    /// Erases (clears with spaces) `how_many` characters at the cursor position, without
    /// shifting any other characters. Maps to `CSI X` ([`ECH`]).
    ///
    /// [`ECH`]: https://vt100.net/docs/vt510-rm/ECH.html
    fn clear_chars(&mut self, how_many: Length);

    // --- Line Operations ---

    /// Inserts `how_many` blank lines at the cursor row, shifting existing lines down
    /// within the `scroll_region`. Maps to `CSI L` ([`IL`]).
    ///
    /// [`IL`]: https://vt100.net/docs/vt510-rm/IL.html
    fn insert_lines(&mut self, how_many: Length, scroll_region: Range<RowIndex>);

    /// Deletes `how_many` lines starting at the cursor row, shifting lines up from the
    /// bottom of the `scroll_region` to fill the gap. Maps to `CSI M` ([`DL`]).
    ///
    /// [`DL`]: https://vt100.net/docs/vt510-rm/DL.html
    fn delete_lines(&mut self, how_many: Length, scroll_region: Range<RowIndex>);

    // --- Clear Operations ---

    /// Clears portions of the entire display across the Y-axis. Maps to `CSI J` ([`ED`]).
    ///
    /// The exact behavior depends on the [`EraseDisplayMode`] (e.g. cursor to end,
    /// beginning to cursor, or the entire screen).
    ///
    /// [`ED`]: https://vt100.net/docs/vt510-rm/ED.html
    fn clear_canvas(&mut self, mode: EraseDisplayMode);

    /// Clears portions of the current line across the X-axis. Maps to `CSI K` ([`EL`]).
    ///
    /// The exact behavior depends on the [`EraseLineMode`] (e.g. cursor to end of line,
    /// beginning of line to cursor, or the entire line).
    ///
    /// [`EL`]: https://vt100.net/docs/vt510-rm/EL.html
    fn clear_line(&mut self, mode: EraseLineMode);

    // --- Scroll Operations ---

    /// Scrolls the defined `scroll_region` up by `how_many` lines. New blank lines are
    /// added at the bottom of the region. Maps to `CSI S` ([`SU`]).
    ///
    /// [`SU`]: https://vt100.net/docs/vt510-rm/SU.html
    fn scroll_up(&mut self, how_many: Length, scroll_region: Range<RowIndex>);

    /// Scrolls the defined `scroll_region` down by `how_many` lines. New blank lines are
    /// added at the top of the region. Maps to `CSI T` ([`SD`]).
    ///
    /// [`SD`]: https://vt100.net/docs/vt510-rm/SD.html
    fn scroll_down(&mut self, how_many: Length, scroll_region: Range<RowIndex>);

    // --- Memory ---

    /// Returns the approximate memory footprint of the underlying data structure in
    /// bytes.
    fn get_mem_size(&self) -> usize;
}
