/*
 *   Copyright (c) 2024 R3BL LLC
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

use std::{fs::OpenOptions, io::Write, path::Path};

/// This is a simple function that logs a message to a file. This is meant to be used when
/// there are no other logging facilities available.
///
/// # Arguments
/// * `file_path` - The path to the file to log to. If `None`, the default path is `debug.log`.
/// * `message` - The message to log.
pub fn file_log(file_path: Option<&Path>, message: &str) {
    let file_path = file_path.unwrap_or(Path::new("debug.log"));
    let message = if message.ends_with('\n') {
        message.to_string()
    } else {
        format!("{}\n", message)
    };
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(file_path)
        .unwrap();
    file.write_all(message.as_bytes()).unwrap();
}
