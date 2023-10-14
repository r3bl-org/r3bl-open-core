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

use log::*;
use r3bl_simple_logger::*;
use termcolor::Color;

fn main() {
    let config = ConfigBuilder::new()
        .set_level_color(Level::Error, Some(Color::Rgb(191, 0, 0)))
        .set_level_color(Level::Warn, Some(Color::Rgb(255, 127, 0)))
        .set_level_color(Level::Info, Some(Color::Rgb(192, 192, 0)))
        .set_level_color(Level::Debug, Some(Color::Rgb(63, 127, 0)))
        .set_level_color(Level::Trace, Some(Color::Rgb(127, 127, 255)))
        .build();

    TermLogger::init(
        LevelFilter::Trace,
        config,
        TerminalMode::Stdout,
        ColorChoice::Auto,
    )
    .unwrap();
    error!("Red error");
    warn!("Orange warning");
    info!("Yellow info");
    debug!("Dark green debug");
    trace!("Light blue trace");
}
