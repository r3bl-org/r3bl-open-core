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

use std::io::{BufReader, Cursor};

use syntect::highlighting::*;

pub fn try_load_r3bl_theme() -> std::io::Result<Theme> {
    // Load bytes from file asset.
    let theme_bytes = include_bytes!("assets/r3bl.tmTheme");

    // Cursor implements Seek for the byte array.
    let cursor = Cursor::new(theme_bytes);

    // Wrap the cursor in a BufReader.
    let mut buf_reader = BufReader::new(cursor);

    // Load the theme from the BufReader.
    let Ok(theme) = ThemeSet::load_from_reader(&mut buf_reader) else {
    return Err(std::io::Error::new(
      std::io::ErrorKind::InvalidData,
      "Failed to load theme",
    ));
  };

    Ok(theme)
}

pub fn load_default_theme() -> Theme {
    let theme_set = ThemeSet::load_defaults();
    theme_set.themes["base16-ocean.dark"].clone()
}
