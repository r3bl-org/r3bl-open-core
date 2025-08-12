// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use std::time::Duration;

pub const DELAY_MS: u64 = 85;
pub const DELAY_UNIT: Duration = Duration::from_millis(DELAY_MS);
pub const ARTIFICIAL_UI_DELAY: Duration = Duration::from_millis(DELAY_MS * 25);

/// More info: <https://www.unicode.org/charts/script/chart_Braille.html>
pub const BRAILLE_DOTS: [&str; 34] = [
    "⠁", "⠃", "⡇", "⠇", "⡎", "⢟", "⡯", "⡗", "⡞", "⡟", "⡷", "⡾", "⡾", "⣕", "⣗", "⣝", "⡣",
    "⡮", "⡯", "⡳", "⡵", "⣞", "⣟", "⣧", "⣮", "⣯", "⣷", "⣿", "⣼", "⡟", "⡏", "⠇", "⠃", "⠁",
];

pub const BLOCK_DOTS: [&str; 8] = ["█", "▓", "▒", "░", "░", "▒", "▓", "█"];
