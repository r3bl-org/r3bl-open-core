// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Character set translation operations.

/// Translate DEC Special Graphics characters to Unicode box-drawing characters.
/// Used when `character_set` is `DECGraphics` (after ESC ( 0).
#[must_use] 
pub fn translate_dec_graphics(c: char) -> char {
    match c {
        'j' => '┘', // Lower right corner
        'k' => '┐', // Upper right corner
        'l' => '┌', // Upper left corner
        'm' => '└', // Lower left corner
        'n' => '┼', // Crossing lines
        'q' => '─', // Horizontal line
        't' => '├', // Left "T"
        'u' => '┤', // Right "T"
        'v' => '┴', // Bottom "T"
        'w' => '┬', // Top "T"
        'x' => '│', // Vertical line
        _ => c,     // Pass through unmapped characters
    }
}