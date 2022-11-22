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

use std::io::{BufRead, BufReader};

use r3bl_rs_utils_core::*;
use r3bl_tui::SPACER;
use tokio::fs::File;

pub async fn run_app() -> CommonResult<()> {
    let mut my_lolcat = Lolcat::default();

    println!("{my_lolcat:?}");

    let file = File::open("Cargo.toml").await?;
    let file = file.into_std().await;

    let (terminal_size_cols, _terminal_size_rows) = crossterm::terminal::size().unwrap();

    let buffer_reader = BufReader::new(file);
    for (index, line) in buffer_reader.lines().enumerate() {
        let line = line.unwrap();

        let line_number_str = format!("{}: ", index + 1);
        let line_number_str_width = UnicodeString::from(&line_number_str).display_width;

        let max_content_width: usize = (terminal_size_cols - *line_number_str_width).into();

        let line_wrapped_vec = textwrap::wrap(&line, max_content_width);

        for (index, item_line) in line_wrapped_vec.iter().enumerate() {
            if index == 0 {
                println!("\r{line_number_str}{}", my_lolcat.format_str(item_line));
            } else {
                println!(
                    "\r{}{}",
                    SPACER.repeat(line_number_str_width.into()),
                    my_lolcat.format_str(item_line)
                );
            }
        }
    }

    Ok(())
}
