/*
 *   Copyright (c) 2022 R3BL LLC
 *   All rights reserved.
 *
 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
 */

use r3bl_rs_utils_core::*;

// ┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
// ┃ Misc crossterm lookup commands ┃
// ┛                                ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
/// Interrogate crossterm [crossterm::terminal::size()] to get the size of the terminal window.
pub fn lookup_size() -> CommonResult<Size> {
  let (col, row) = crossterm::terminal::size()?;
  let size: Size = size!(cols: col, rows: row);
  Ok(size)
}
