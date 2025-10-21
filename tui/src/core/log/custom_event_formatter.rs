// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! # Custom event formatter for tracing
//!
//! This module provides a custom event formatter ([`CustomEventFormatter`]) for the
//! [`tracing`] crate that produces beautifully formatted, colorized log output optimized
//! for terminal UIs. The formatter is designed to work seamlessly with the R3BL TUI
//! library's color scheme and terminal handling capabilities.
//!
//! ## Features
//!
//! - **Rich visual formatting**: Produces colorized output with timestamps, log levels,
//!   span context, and structured field content
//! - **Terminal-aware**: Automatically adapts to terminal width and handles Unicode/emoji
//!   content correctly
//! - **Two-line format**: Each log entry consists of a header line (timestamp, level,
//!   message) and body lines (additional field content)
//! - **Text wrapping**: Long content is intelligently wrapped to fit terminal width
//! - **Color-coded levels**: Different log levels (ERROR, WARN, INFO, DEBUG, TRACE) have
//!   distinct colors and sigils
//!
//! ## Log entry structure
//!
//! Each formatted log entry follows this structure:
//! ```text
//! <timestamp> [span_context] <level_sigil>: <spacer><message_heading>
//! <spacer><bullet> <field_name>
//! <spacer>  <field_value_wrapped_to_terminal_width>
//! <spacer><separator_line>
//! ```
//!
//! ## Usage examples
//!
//! ### Basic setup
//!
//! To use this formatter with the `tracing` crate, register it with `tracing_subscriber`:
//!
//! ```no_run
//! use tracing_subscriber::{fmt::SubscriberBuilder, registry::LookupSpan};
//! use r3bl_tui::log::{CustomEventFormatter, try_initialize_logging_global, DisplayPreference};
//! use r3bl_tui::SharedWriter;
//!
//! // Method 1: Direct registration with tracing_subscriber
//! let subscriber = SubscriberBuilder::default()
//!     .event_format(CustomEventFormatter)
//!     .finish();
//!
//! // Method 2: Using R3BL's helper function (recommended)
//! # async fn fun() -> Result<(), Box<dyn std::error::Error>> {
//! try_initialize_logging_global(DisplayPreference::Stdout)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! # }
//! ```
//!
//! ### Logging with dynamic messages
//!
//! The formatter supports dynamic message composition using the special `message` field:
//!
//! ```rust
//! use tracing::info;
//! use r3bl_tui::inline_string;
//!
//! // Using the message field for dynamic content
//! let counter = 42;
//! info!(
//!     message = %inline_string!("[{}] Task completed successfully!", counter),
//!     "duration_ms" = 150,
//!     "task_id" = "task_001"
//! );
//! ```
//!
//! ## Special field handling
//!
//! The formatter has special handling for the `message` field to enable dynamic content:
//!
//! - When a field is named "message", its value becomes the heading text
//! - The value is set to an empty string to prevent duplicate display
//! - Other fields become key-value pairs in the body
//! - Empty field values are automatically skipped to avoid blank lines
//!
//! ## Color scheme
//!
//! The formatter uses a carefully designed color scheme:
//! - **ERROR**: Pink/red tones for critical issues
//! - **WARN**: Orange tones for warnings
//! - **INFO**: Salmon/orange tones for general information
//! - **DEBUG**: Yellow tones for debugging information
//! - **TRACE**: Purple tones for detailed tracing
//! - **Body text**: Subtle gray tones for readability
//! - **Headings**: Colorful "lolcat" rainbow effect for visual appeal
//!
//! ## Architecture
//!
//! The module is structured with:
//! - [`CustomEventFormatter`]: Main formatter struct implementing [`FormatEvent`]
//! - [`FieldContentParams`]: Parameter struct to avoid "too many arguments" clippy
//!   warnings
//! - [`VisitEventAndPopulateOrderedMapWithFields`]: Custom field visitor for structured
//!   data
//! - Helper methods for formatting timestamps, log levels, and content
//! - Constants module with color definitions and formatting parameters
//!
//! To use this [`CustomEventFormatter`] with the `tracing` crate, you must register it
//! with the `tracing_subscriber` crate. This is done in [`crate::create_fmt`!]. Here's an
//! example of how you can do this with the `tracing` crate:
//!
//! ### Basic registration example
//!
//! ```rust
//! # use tracing_subscriber::{fmt::SubscriberBuilder, registry::LookupSpan};
//! # use r3bl_tui::log::CustomEventFormatter;
//! let subscriber = SubscriberBuilder::default()
//!     .event_format(CustomEventFormatter)
//!     .finish();
//! ```
//!
//! ## Implementation details
//!
//! The reason for the special logic in
//! [`VisitEventAndPopulateOrderedMapWithFields::record_debug`] and the
//! [`CustomEventFormatter::format_event`] method regarding empty field values is to
//! enable dynamic message composition. This allows the `message` field to contain
//! variable content (such as strings built with [`crate::glyphs`]) rather than being
//! limited to string literals.
//!
//! The tracing crate treats the `message` field specially - it's automatically injected
//! when logging macros are called with a single expression:
//! - `info!(foobar)` - becomes `message = "foobar"`
//! - `info!("foobar")` - becomes `message = "foobar"`
//! - `info!(format!("{}{}", "foo", "bar"))` - becomes `message = "foobar"`
//!
//! For R3BL crates, the convention is:
//! - The `message` field forms the colorful header line
//! - Additional fields form key-value pairs in the body
//! - Empty field values are skipped to avoid blank lines

use crate::{ColWidth, ColorWheel, GCStringOwned, InlineString, OrderedMap, RgbValue,
            TuiColor, TuiStyle, cli_text, fg_color, get_terminal_width, glyphs,
            inline_string, new_style, remove_escaped_quotes, truncate_from_right,
            tui_color, tui_style_attrib, usize, width};
use chrono::Local;
use const_format::formatcp;
use custom_event_formatter_constants::{BODY_FG_COLOR, BODY_FG_COLOR_BRIGHT,
                                       DEBUG_FG_COLOR, DEBUG_SIGIL,
                                       ENTRY_SEPARATOR_CHAR, ERROR_FG_COLOR,
                                       ERROR_SIGIL, FIRST_LINE_PREFIX, HEADING_BG_COLOR,
                                       INFO_FG_COLOR, INFO_SIGIL, LEVEL_SUFFIX,
                                       SUBSEQUENT_LINE_PREFIX, TRACE_FG_COLOR,
                                       TRACE_SIGIL, WARN_FG_COLOR, WARN_SIGIL};
use std::{fmt::{self},
          sync::LazyLock};
use textwrap::{Options, WordSeparator, wrap};
use tracing::{Event, Subscriber,
              field::{Field, Visit}};
use tracing_subscriber::{fmt::{FormatEvent, FormatFields},
                         registry::LookupSpan};

/// This is the "marker" struct that is used to register this formatter with the
/// `tracing_subscriber` crate. Various traits are implemented for this struct.
#[derive(Debug, Default)]
pub struct CustomEventFormatter;

/// Parameters for formatting field content.
#[derive(Debug)]
pub struct FieldContentParams<'a> {
    heading: &'a str,
    body: &'a str,
    line_width_used: ColWidth,
    max_display_width: ColWidth,
    text_wrap_options: &'a Options<'a>,
    spacer: &'a str,
}

// Colors: <https://en.wikipedia.org/wiki/ANSI_escape_code>
#[rustfmt::skip]
pub mod custom_event_formatter_constants {
    use super::{formatcp, glyphs, RgbValue};

    pub const FIRST_LINE_PREFIX: &str = formatcp!(
        "{sp}{ch}{sp}",
        sp = glyphs::SPACER_GLYPH,
        ch = glyphs::FANCY_BULLET_GLYPH
    );
    pub const SUBSEQUENT_LINE_PREFIX: &str = formatcp!("{sp}", sp = glyphs::SPACER_GLYPH);
    pub const LEVEL_SUFFIX: &str = ":";

    pub const ERROR_SIGIL: &str = "E";
    pub const WARN_SIGIL: &str = "W";
    pub const INFO_SIGIL: &str = "I";
    pub const DEBUG_SIGIL: &str = "D";
    pub const TRACE_SIGIL: &str = "T";

    pub const ENTRY_SEPARATOR_CHAR: &str =
        formatcp!("{ch}", ch = glyphs::TOP_UNDERLINE_GLYPH);

    pub const BODY_FG_COLOR: RgbValue =        RgbValue{red:175,green: 175,blue: 175};
    pub const BODY_FG_COLOR_BRIGHT: RgbValue = RgbValue{red:200,green: 200,blue: 200};
    pub const HEADING_BG_COLOR: RgbValue =     RgbValue{red:70,green: 70,blue: 90};
    pub const INFO_FG_COLOR: RgbValue =        RgbValue{red:233,green: 150,blue: 122};
    pub const ERROR_FG_COLOR: RgbValue =       RgbValue{red:255,green: 182,blue: 193};
    pub const WARN_FG_COLOR: RgbValue =        RgbValue{red:255,green: 140,blue: 0};
    pub const DEBUG_FG_COLOR: RgbValue =       RgbValue{red:255,green: 255,blue: 0};
    pub const TRACE_FG_COLOR: RgbValue =       RgbValue{red:186,green: 85,blue: 211};
}

/// Cache for colorized log headings.
mod heading_cache {
    use super::{LazyLock, TuiStyle};
    use std::hash::{Hash, Hasher};

    /// Key for caching colorized headings.
    #[derive(Clone, PartialEq, Eq, Debug)]
    pub struct HeadingCacheKey {
        text: String,
        style: Option<TuiStyle>,
    }

    impl HeadingCacheKey {
        pub fn new(text: &str, style: Option<TuiStyle>) -> Self {
            Self {
                text: text.to_string(),
                style,
            }
        }
    }

    impl Hash for HeadingCacheKey {
        fn hash<H: Hasher>(&self, state: &mut H) {
            self.text.hash(state);
            // Hash the style if present.
            if let Some(style) = &self.style {
                // Hash all style attributes.
                style.id.hash(state);
                style.color_fg.hash(state);
                style.color_bg.hash(state);
                style.attribs.bold.hash(state);
                style.attribs.italic.hash(state);
                style.attribs.underline.hash(state);
                style.attribs.dim.hash(state);
                style.attribs.reverse.hash(state);
                style.attribs.hidden.hash(state);
                style.attribs.strikethrough.hash(state);
                style.computed.hash(state);
                style.padding.hash(state);
            }
        }
    }

    /// Global cache for colorized headings.
    ///
    /// This cache stores the results of `lolcat_into_string` operations on
    /// log headings, which are often repeated (e.g., field names like "message",
    /// "error", etc.). The cache significantly reduces the overhead of colorization
    /// in hot paths.
    pub static COLORIZED_HEADING_CACHE: LazyLock<
        crate::ThreadSafeLruCache<HeadingCacheKey, String>,
    > = LazyLock::new(|| crate::new_threadsafe_lru_cache(1000));
}

mod helpers {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Write formatted timestamp to the writer.
    pub fn write_timestamp(
        f: &mut tracing_subscriber::fmt::format::Writer<'_>,
        spacer: &str,
    ) -> fmt::Result {
        let timestamp = Local::now();
        let timestamp_str = inline_string!(
            "{sp}{ts}{sp}",
            ts = timestamp.format("%I:%M%P"),
            sp = spacer
        );
        let timestamp_str_fmt = cli_text(
            timestamp_str,
            new_style!(
                italic
                color_fg: {TuiColor::Rgb(BODY_FG_COLOR_BRIGHT)}
                color_bg: {TuiColor::Rgb(HEADING_BG_COLOR)}
            ),
        );
        write!(f, "\n{timestamp_str_fmt}")
    }

    /// Write formatted span context to the writer.
    pub fn write_span_context<S, N>(
        ctx: &tracing_subscriber::fmt::FmtContext<'_, S, N>,
        f: &mut tracing_subscriber::fmt::format::Writer<'_>,
    ) -> fmt::Result
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
        N: for<'a> FormatFields<'a> + 'static,
    {
        match ctx.lookup_current() {
            Some(scope) => {
                let scope_str = inline_string!("[{}] ", scope.name());
                let scope_str_fmt = cli_text(
                    scope_str,
                    new_style!(
                        italic
                        color_fg: {TuiColor::Rgb(BODY_FG_COLOR_BRIGHT)}
                        color_bg: {TuiColor::Rgb(HEADING_BG_COLOR)}
                    ),
                );
                write!(f, "{scope_str_fmt}")
            }
            None => Ok(()),
        }
    }

    /// Get level string and style based on the event's log level.
    pub fn get_level_info(
        event: &Event<'_>,
        spacer: &str,
    ) -> (InlineString, crate::TuiStyle) {
        let mut style = new_style!();
        let level_str = match *event.metadata().level() {
            tracing::Level::ERROR => {
                style.color_fg = Some(TuiColor::Rgb(ERROR_FG_COLOR));
                inline_string!("{ERROR_SIGIL}{LEVEL_SUFFIX}{spacer}")
            }
            tracing::Level::WARN => {
                style.color_fg = Some(TuiColor::Rgb(WARN_FG_COLOR));
                inline_string!("{WARN_SIGIL}{LEVEL_SUFFIX}{spacer}")
            }
            tracing::Level::INFO => {
                style.color_fg = Some(TuiColor::Rgb(INFO_FG_COLOR));
                inline_string!("{INFO_SIGIL}{LEVEL_SUFFIX}{spacer}")
            }
            tracing::Level::DEBUG => {
                style.color_fg = Some(TuiColor::Rgb(DEBUG_FG_COLOR));
                inline_string!("{DEBUG_SIGIL}{LEVEL_SUFFIX}{spacer}")
            }
            tracing::Level::TRACE => {
                style.color_fg = Some(TuiColor::Rgb(TRACE_FG_COLOR));
                inline_string!("{TRACE_SIGIL}{LEVEL_SUFFIX}{spacer}")
            }
        };
        style.color_bg = Some(TuiColor::Rgb(HEADING_BG_COLOR));
        style.attribs.bold = Some(tui_style_attrib::Bold);
        (level_str, style)
    }

    /// Write formatted log level to the writer.
    pub fn write_log_level(
        f: &mut tracing_subscriber::fmt::format::Writer<'_>,
        event: &Event<'_>,
        spacer: &str,
    ) -> fmt::Result {
        let (level_str, style) = helpers::get_level_info(event, spacer);
        let level_str_fmt = cli_text(level_str, style);
        write!(f, "{level_str_fmt}")
    }

    /// Calculate line width used by timestamp, span context, and level.
    pub fn calculate_header_width<S, N>(
        ctx: &tracing_subscriber::fmt::FmtContext<'_, S, N>,
        event: &Event<'_>,
        spacer: &str,
    ) -> ColWidth
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
        N: for<'a> FormatFields<'a> + 'static,
    {
        let spacer_display_width = GCStringOwned::from(spacer).width();
        let mut line_width_used = width(0);

        // Timestamp width
        let timestamp = Local::now();
        let timestamp_str = inline_string!(
            "{sp}{ts}{sp}",
            ts = timestamp.format("%I:%M%P"),
            sp = spacer
        );
        line_width_used += GCStringOwned::from(&timestamp_str).width();

        // Span context width.
        if let Some(scope) = ctx.lookup_current() {
            let scope_str = inline_string!("[{}] ", scope.name());
            line_width_used += GCStringOwned::from(&scope_str).width();
        }

        // Level width
        let (level_str, _) = helpers::get_level_info(event, spacer);
        let level_str_display_width = GCStringOwned::from(&level_str).width();
        line_width_used += spacer_display_width;
        line_width_used += level_str_display_width;

        line_width_used
    }

    /// Write formatted field content (heading and body).
    pub fn write_field_content(
        f: &mut tracing_subscriber::fmt::format::Writer<'_>,
        FieldContentParams {
            heading,
            body,
            line_width_used,
            max_display_width,
            text_wrap_options,
            spacer,
        }: FieldContentParams<'_>,
    ) -> fmt::Result {
        let spacer_display_width = GCStringOwned::from(spacer).width();

        // Write heading line.
        let heading = remove_escaped_quotes(heading);
        let line_1_width = {
            let it = max_display_width - line_width_used - spacer_display_width;
            width(*it)
        };
        let truncated_heading = truncate_from_right(&heading, line_1_width, false);
        let line_1_text = inline_string!(
            "{spacer}{heading}",
            spacer = spacer,
            heading = truncated_heading.as_ref()
        );

        // Check cache for colorized heading.
        let style = Some(new_style!(bold));
        let cache_key = super::heading_cache::HeadingCacheKey::new(&line_1_text, style);

        let line_1_text_fmt = if let Ok(mut cache_guard) =
            super::heading_cache::COLORIZED_HEADING_CACHE.lock()
        {
            if let Some(cached) = cache_guard.get(&cache_key) {
                cached.clone()
            } else {
                let colorized = ColorWheel::lolcat_into_string(&line_1_text, style);
                cache_guard.insert(cache_key, colorized.clone());
                colorized
            }
        } else {
            // Cache lock failed, compute without caching.
            ColorWheel::lolcat_into_string(&line_1_text, style)
        };

        writeln!(f, "{line_1_text_fmt}")?;

        // Write body lines.
        if !body.is_empty() {
            let body_text = remove_escaped_quotes(body);
            let wrapped_lines = wrap(&body_text, text_wrap_options);
            for body_line in &wrapped_lines {
                // Note: padding is disabled (false) to avoid allocations in this hot
                // path. This function is called on every render in the
                // main event loop.
                let truncated_body_line =
                    truncate_from_right(body_line, max_display_width, false);
                let body_line_fmt = cli_text(
                    truncated_body_line.as_ref(),
                    new_style!(
                        color_fg: {TuiColor::Rgb(BODY_FG_COLOR)}
                    ),
                );
                writeln!(f, "{body_line_fmt}")?;
            }
        }

        Ok(())
    }
}

impl<S, N> FormatEvent<S, N> for CustomEventFormatter
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    /// Format the event into 2 lines:
    /// 1. Heading: Timestamp, span context, level and message truncated to the available
    ///    visible width (90 columns).
    /// 2. Body: Body text wrapped to 90 columns wide.
    ///
    /// This function takes into account text that can contain emoji.
    ///
    /// The reason for the strange logic in
    /// [`VisitEventAndPopulateOrderedMapWithFields::record_debug`] and the
    /// [`CustomEventFormatter::format_event`] skipping empty field value lines is that we
    /// wanted to be able to have a `message` field where a String can be used instead of
    /// "stringify!" which just dumps the string literal. This does not allow the message
    /// to be a variable, which means it can't be composed using other glyphs, such as the
    /// ones from [`crate::glyphs`]. To get around this limitation, the following logic
    /// was implemented.
    fn format_event(
        &self,
        ctx: &tracing_subscriber::fmt::FmtContext<'_, S, N>,
        mut f: tracing_subscriber::fmt::format::Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let spacer = glyphs::SPACER_GLYPH;

        // Write header components (timestamp, span context, log level).
        helpers::write_timestamp(&mut f, spacer)?;
        helpers::write_span_context(ctx, &mut f)?;
        helpers::write_log_level(&mut f, event, spacer)?;

        // Calculate header width for field content formatting.
        let line_width_used = helpers::calculate_header_width(ctx, event, spacer);

        // Extract and format field content.
        let mut order_map = OrderedMap::<InlineString, InlineString>::default();
        event.record(&mut VisitEventAndPopulateOrderedMapWithFields {
            inner: &mut order_map,
        });

        let max_display_width = get_terminal_width();
        let text_wrap_options = Options::new(usize(*max_display_width))
            .initial_indent(FIRST_LINE_PREFIX)
            .subsequent_indent(SUBSEQUENT_LINE_PREFIX)
            .word_separator(WordSeparator::UnicodeBreakProperties);

        // Write field content.
        for (heading, body) in order_map.iter() {
            helpers::write_field_content(
                &mut f,
                FieldContentParams {
                    heading,
                    body,
                    line_width_used,
                    max_display_width,
                    text_wrap_options: &text_wrap_options,
                    spacer,
                },
            )?;
        }

        // Write the terminating line separator.
        writeln!(f, "{}", build_spacer(max_display_width))
    }
}

#[derive(Debug)]
pub struct VisitEventAndPopulateOrderedMapWithFields<'a> {
    pub inner: &'a mut OrderedMap<InlineString, InlineString>,
}

impl Visit for VisitEventAndPopulateOrderedMapWithFields<'_> {
    /// The reason for the strange logic in
    /// [`VisitEventAndPopulateOrderedMapWithFields::record_debug`] and the
    /// [`CustomEventFormatter::format_event`] skipping empty field value lines is that we
    /// wanted to be able to have a `message` field where a String can be used instead of
    /// "stringify!" which just dumps the string literal. This does not allow the message
    /// to be a variable, which means it can't be composed using other glyphs, such as the
    /// ones from [`crate::glyphs`]. To get around this limitation, the following logic
    /// was implemented.
    ///
    /// There is some strange logic in here to handle the `message` field. The `message`
    /// field is a special field that is added by the `tracing` crate. In the example
    /// below, the statements are identical:
    ///
    /// ```
    /// use tracing::{info};
    /// info!(message = "This is a test log entry");
    /// info!("This is a test log entry");
    /// ```
    ///
    /// The way `r3bl_*` crates use tracing is more formalized. The assumption (invariant)
    /// is that calls to `info!`, `warn!`, `error!`, etc. will always have a `message`
    /// field which forms the header. Then there must be a body key-value pair (field:
    /// name and value) that forms the body of the log entry. There may be multiple
    /// key-value pairs in the body.
    ///
    /// When a field only has "message" field name, then it's value is taken to be the
    /// name. And the value is then set to an empty string. Empty values cause the
    /// [`CustomEventFormatter::format_event`] to skip writing the body line.
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        let field_name = field.name();
        let field_value = inline_string!("{:?}", value);
        if field_name == "message" {
            self.inner.insert(field_value, "".into());
        } else {
            self.inner.insert(field_name.into(), field_value);
        }
    }

    /// Override `record_str` to use [`std::fmt::Display`] formatting instead of [`Debug`]
    /// formatting. This avoids escaping newlines and quotes in string values, making
    /// telemetry data more readable.
    ///
    /// By using the [`std::fmt::Display`] trait (`"{}"`) for string fields instead. This
    /// avoids expensive [`Debug`] formatting for log messages while maintaining [`Debug`]
    /// formatting for non-string types.
    ///
    /// This has a significant performance impact, and speeds up telemetry logging by
    /// about 43% (from 17.39% to 9.86% overhead).
    fn record_str(&mut self, field: &Field, value: &str) {
        let field_name = field.name();
        let field_value = inline_string!("{}", value);
        if field_name == "message" {
            self.inner.insert(field_value, "".into());
        } else {
            self.inner.insert(field_name.into(), field_value);
        }
    }
}

#[must_use]
pub fn build_spacer(max_display_width: ColWidth) -> InlineString {
    let spacer = ENTRY_SEPARATOR_CHAR.repeat(max_display_width.as_usize());
    fg_color(tui_color!(dark_lizard_green), &spacer).to_small_str()
}

#[cfg(test)]
mod tests_tracing_custom_event_formatter {
    use super::*;
    use crate::{core::test_fixtures::StdoutMock, glyphs::SPACER_GLYPH as SPACER};
    use std::sync::Mutex;
    use tracing::{info, subscriber::set_default};
    use tracing_subscriber::fmt::SubscriberBuilder;

    #[test]
    fn test_custom_formatter_with_special_message_field_handling() {
        let mock_stdout = StdoutMock::new();
        let mock_stdout_clone = mock_stdout.clone();
        let subscriber = SubscriberBuilder::default()
            .event_format(CustomEventFormatter)
            .with_writer(Mutex::new(mock_stdout))
            .finish();

        // Note that tests, or libraries for that matter, should NOT call
        // `subscriber::set_global_default()`.
        let _drop_guard = set_default(subscriber);

        info!(
            message = "This is now the heading, not the body!",
            "foo" = "bar"
        );

        let time = Local::now().format("%I:%M%P").to_string();
        let it = mock_stdout_clone.get_copy_of_buffer_as_string();
        let it_no_ansi = mock_stdout_clone.get_copy_of_buffer_as_string_strip_ansi();

        // println!("{}", it);
        // println!("{}", it_no_ansi);

        // lolcat colorized each char in the heading, so strip the colors.
        assert!(!it_no_ansi.contains("message"));
        assert!(it_no_ansi.contains("This is now the heading, not the body!"));

        // lolcat colorized each char in the heading, so strip the colors.
        assert!(it_no_ansi.contains("foo"));
        assert!(it.contains("bar"));

        assert!(it.matches(FIRST_LINE_PREFIX).count() >= 1);
        assert!(it.matches(SPACER).count() >= 1);
        assert!(it.contains(&format!("{INFO_SIGIL}{LEVEL_SUFFIX}")));
        assert!(it.contains(&time));
        assert_eq!(it.matches('\n').count(), 5); // There are many new lines.
    }

    #[test]
    fn test_custom_formatter_no_message_field_name() {
        let mock_stdout = StdoutMock::new();
        let mock_stdout_clone = mock_stdout.clone();
        let subscriber = SubscriberBuilder::default()
            .event_format(CustomEventFormatter)
            .with_writer(Mutex::new(mock_stdout))
            .finish();

        // Note that tests, or libraries for that matter, should NOT call
        // `subscriber::set_global_default()`.
        let _drop_guard = set_default(subscriber);

        info!(my_log_message_heading = "This is a test log entry body");

        let time = Local::now().format("%I:%M%P").to_string();
        let it = mock_stdout_clone.get_copy_of_buffer_as_string();
        let it_no_ansi = mock_stdout_clone.get_copy_of_buffer_as_string_strip_ansi();

        // println!("{}", it);
        // println!("{}", it_no_ansi);

        // lolcat colorized each char in the heading, so strip the colors.
        assert!(it_no_ansi.contains("my_log_message_heading"));

        assert!(it.contains("This is a test log entry body"));

        assert!(it.matches(FIRST_LINE_PREFIX).count() >= 1);
        assert!(it.matches(SPACER).count() >= 1);
        assert!(it.contains(&format!("{INFO_SIGIL}{LEVEL_SUFFIX}")));
        assert!(it.contains(&time));
        assert_eq!(it.matches('\n').count(), 4); // There are many new lines.
    }
}
