// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Standalone [`miette`] global report handler setup.
//!
//! This module can be used in any project that uses [`miette`] for error handling. To
//! change the default global report handler, you can either:
//! 1. Customize the default global implementation of [`ReportHandler`]. (Easy).
//! 2. Register a custom error report handler of your own. (Difficult).
//!
//! ## Architecture
//!
//! [`miette`] allows customizing how a [`Report`] is displayed to terminal output
//! ([`stdout`], [`stderr`]) when the program errors out. The global hook is only
//! activated at display time, not when it is registered - it is lazy. This makes it
//! possible to detect the terminal width just before the output is generated.
//!
//! - The default global implementation of [`ReportHandler`] is [`MietteHandler`].
//! - [`MietteHandlerOpts`] configures [`MietteHandler`]. Under the hood,
//!   [`MietteHandlerOpts::build()`] produces a [`GraphicalReportHandler`], which is the
//!   "real" handler struct.
//! - [`miette::set_hook()`] registers a custom error report handler. Here's an [example
//!   in a test] which registers a custom hook.
//!
//! [`GraphicalReportHandler`]: miette::GraphicalReportHandler
//! [`miette::set_hook()`]: miette::set_hook
//! [`miette`]: miette
//! [`MietteHandler`]: miette::MietteHandler
//! [`MietteHandlerOpts::build()`]: miette::MietteHandlerOpts::build
//! [`MietteHandlerOpts`]: miette::MietteHandlerOpts
//! [`Report`]: miette::Report
//! [`ReportHandler`]: miette::ReportHandler
//! [`stderr`]: std::io::stderr
//! [`stdout`]: std::io::stdout
//! [example in a test]:
//!     https://github.com/zkat/miette/blob/6ea86a2248854acf88df345814b6c97d31b8b4d9/tests/test_location.rs#L39

use miette::MietteHandlerOpts;
use tracing::debug;

/// Registers a default [`miette`] global report handler with lazy terminal width
/// detection.
///
/// The [`miette::ErrorHook`] is lazily evaluated - terminal width is calculated only
/// when the error handler is actually invoked. If no error occurs, the terminal width
/// is never calculated.
///
/// [`miette::ErrorHook`]: miette::ErrorHook
/// [`miette`]: miette
pub fn setup_default_miette_global_report_handler(issues_url: &'static str) {
    miette::set_hook(Box::new(|_report| {
        let terminal_width = {
            let it = crossterm::terminal::size().map_or(80, |(columns, _rows)| columns)
                as usize;
            debug!("miette::set_hook -> terminal_width: {}", it);
            it
        };
        Box::new(
            MietteHandlerOpts::new()
                .width(terminal_width)
                .wrap_lines(true)
                .force_graphical(true)
                .rgb_colors(miette::RgbColors::Always)
                .terminal_links(true)
                .unicode(true)
                .context_lines(3)
                .tab_width(4)
                .break_words(true)
                .with_cause_chain()
                .footer(issues_url.to_string())
                .build(),
        )
    }))
    .ok();
}
