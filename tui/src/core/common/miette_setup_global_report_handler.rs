// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! This module is standalone, you can use it any project that uses
//! [miette](https://docs.rs/miette/latest/miette/index.html) for error handling.
//!
//! If you want to change the default global report handler you can do the following:
//! 1. Customize the default global implementation of the `ReportHandler` trait. (Easy).
//! 2. Register a custom error report handler of your own. (Difficult).
//!
//! Background information on miette's architecture:
//! - Miette allows customization how the report is [`Report`](https://docs.rs/miette/latest/miette/struct.Report.html)
//!   displayed to terminal output (stdout, stderr), when the global hook is activated,
//!   due to a program "erroring out" or "crashing", when the top-level miette handler in
//!   `main() -> miette::Result<_>` is activated. This hook is only activated at the time
//!   that the error is displayed to terminal output, not when it is registered, it is
//!   lazy. So it is possible to detect the terminal width just before the output is
//!   generated to the terminal output (stdout, stderr).
//! - The global default implementation of the [`ReportHandler` trait](https://docs.rs/miette/latest/miette/trait.ReportHandler.html)
//!   is done by [`MietteHandler` struct](https://docs.rs/miette/latest/miette/struct.MietteHandler.html).
//! - Using the [`MietteHandlerOpts`
//!   struct](https://docs.rs/miette/latest/miette/struct.MietteHandlerOpts.html) you can
//!   configure the default `MietteHandler`. Under the hood, `build()` produces a
//!   [`GraphicalReportHandler`
//!   struct](https://docs.rs/miette/latest/miette/struct.GraphicalReportHandler.html)
//!   which is the "real" handler struct.
//! - The [`miette::set_hook`] function is used to register a custom error report handler.
//!   Here's an [example in a test](https://github.com/zkat/miette/blob/6ea86a2248854acf88df345814b6c97d31b8b4d9/tests/test_location.rs#L39)
//!   which registers a custom hook / report handler.

use miette::MietteHandlerOpts;
use tracing::debug;

/// The [`miette::ErrorHook`] is lazily evaluated.
///
/// The terminal width will be calculated just at the time of the global error handler
/// being used. So if an error never occurs, then the terminal width will never be
/// calculated. This is the desired behavior.
pub fn setup_default_miette_global_report_handler(issues_url: &'static str) {
    miette::set_hook(Box::new(|_report| {
        let terminal_width = {
            let it = crossterm::terminal::size()
                .map(|(columns, _rows)| columns)
                .unwrap_or(80) as usize;
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
