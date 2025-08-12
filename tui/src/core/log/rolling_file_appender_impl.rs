// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use std::path::PathBuf;

/// Note that if you wrap this up in a non blocking writer, it doesn't work. Here's an
/// example of this:
/// `tracing_appender::non_blocking(try_create_rolling_file_appender("foo")?)`
///
/// # Errors
///
/// Returns an error if:
/// - The path has no parent directory
/// - The path has no file name
/// - Insufficient permissions to access the file or directory
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
