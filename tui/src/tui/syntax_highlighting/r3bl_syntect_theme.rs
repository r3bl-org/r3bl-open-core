// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use std::io::{BufReader, Cursor};
use syntect::highlighting::{Theme, ThemeSet};

/// Use a [`std::io::Cursor`] as a fake [`std::fs::File`]:
/// <https://stackoverflow.com/a/41069910/2085356>
///
/// # Errors
///
/// Returns an I/O error if the embedded theme file cannot be loaded or parsed.
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

#[must_use]
pub fn load_default_theme() -> Theme {
    let theme_set = ThemeSet::load_defaults();
    theme_set.themes["base16-ocean.dark"].clone()
}

#[cfg(test)]
mod tests {
    use crate::{throws, try_load_r3bl_theme};

    #[test]
    fn load_theme() -> std::io::Result<()> {
        throws!({
            let theme = try_load_r3bl_theme()?;
            dbg!(&theme);
        });
    }
}
