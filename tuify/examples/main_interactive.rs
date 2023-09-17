/*
 *   Copyright (c) 2023 R3BL LLC
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

use std::io::Result;

use r3bl_rs_utils_core::*;
use r3bl_tuify::*;

fn main() -> Result<()> {
    call_if_true!(TRACE, {
        try_to_set_log_level(log::LevelFilter::Trace).ok();
        log_debug("Start logging...".to_string());
        log_debug(format!("og_size: {:?}", get_size()?).to_string());
    });

    // Get display size.
    let max_width_col_count: usize =
        get_size().map(|it| it.col_count).unwrap_or(ch!(80)).into();
    let max_height_row_count: usize = 5;

    let user_input = select_from_list(
        [
            "item 1", "item 2", "item 3", "item 4", "item 5", "item 6", "item 7",
            "item 8", "item 9", "item 10",
        ]
        .iter()
        .map(|it| it.to_string())
        .collect(),
        max_height_row_count,
        max_width_col_count,
        // SelectionMode::Single,
        SelectionMode::Multiple,
    );

    match &user_input {
        Some(it) => {
            println!("User selected: {:?}", it);
        }
        None => println!("User did not select anything"),
    }

    call_if_true!(TRACE, {
        log_debug(format!("user_input: {:?}", user_input).to_string());
        log_debug("Stop logging...".to_string());
    });

    Ok(())
}
