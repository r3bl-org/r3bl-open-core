use r3bl_rs_utils_core::*;
use crossterm::cursor;
/// Interrogate crossterm [crossterm::terminal::size()] to get the size of the terminal window.
pub fn lookup_size() -> CommonResult<Size> {
    let (col, row) = crossterm::terminal::size()?;
    let size: Size = size!(col_count: col, row_count: row);
    Ok(size)
}
pub fn get_inline_row_index() -> ChUnit {
    let (_x, y) = cursor::position().unwrap();
    (y).into()
}
