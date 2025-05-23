/*
 *   Copyright (c) 2024-2025 R3BL LLC
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
