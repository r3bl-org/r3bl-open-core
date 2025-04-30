/*
 *   Copyright (c) 2025 R3BL LLC
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

use std::{fs::File,
          io::{BufReader, Read, Write},
          path::PathBuf};

use miette::IntoDiagnostic as _;
use r3bl_tui::CommonResult;

pub fn try_read_file_contents(path: &PathBuf) -> CommonResult<String> {
    let file = File::open(path).into_diagnostic()?;
    let mut reader = BufReader::new(file);
    let mut contents = String::new();
    let _ = reader.read_to_string(&mut contents).into_diagnostic()?;
    Ok(contents)
}

pub fn try_write_file_contents(path: &PathBuf, contents: &str) -> CommonResult<()> {
    let mut file = File::create(path).into_diagnostic()?;
    file.write_all(contents.as_bytes()).into_diagnostic()?;
    Ok(())
}
