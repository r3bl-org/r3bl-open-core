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

use std::path::PathBuf;

/// Note that if you wrap this up in a non blocking writer, as shown below, it doesn't
/// work:
///
/// ```ignore
/// tracing_appender::non_blocking(try_create_rolling_file_appender("foo")?);
/// ```
pub fn try_create(
    path_str: &str,
) -> miette::Result<tracing_appender::rolling::RollingFileAppender> {
    let path = PathBuf::from(&path_str);

    let parent = path.parent().ok_or_else(|| {
        miette::miette!(
            format!("Can't access current folder {}. It might not exist, or don't have required permissions.",
            path.display())
        )
    })?;

    let file_stem = path.file_name().ok_or_else(|| {
        miette::miette!(format!(
        "Can't access file name {}. It might not exist, or don't have required permissions.",
        path.display()
    ))
    })?;

    Ok(tracing_appender::rolling::never(parent, file_stem))
}
