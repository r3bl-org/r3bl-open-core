/*
 *   Copyright (c) 2024-2025 R3BL LLC
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

//! The reason for the strange logic in the
//! [VisitEventAndPopulateOrderedMapWithFields::record_debug] function and the
//! [CustomEventFormatter::format_event] skipping empty field values (ie, empty body
//! lines) is that we wanted to be able to have a `message` field where a String can be
//! used instead of "stringify!" which just dumps the string literal. This does not allow
//! the message to be a variable, which means it can't be composed using other glyphs,
//! such as the ones from [crate::glyphs]. To get around this limitation, the
//! following logic was implemented.
//!
//! The tracing crate deals with records that have fields. Each field has a name and
//! value. The `message` field name is special and is automatically injected in cases
//! where a call to `info!`, `warn!`, `error!`, etc. only has 1 expression, eg:
//! `info!(foobar);`, `info!("foobar");` or `info!(format!("{}{}", "foo", "bar"));`.
//!
//! So in order to be able to create "dynamic" headings or field names, you have to
//! explicitly use the `message` field name. Its value can then be any expression. There
//! are lots of examples in the tests below.
use std::fmt::{self};

use chrono::Local;
use const_format::formatcp;
use custom_event_formatter_constants::*;
use textwrap::{Options, WordSeparator, wrap};
use tracing::{Event,
              Subscriber,
              field::{Field, Visit}};
use tracing_subscriber::{fmt::{FormatEvent, FormatFields},
                         registry::LookupSpan};

use crate::{AnsiStyledText,
            ColWidth,
            ColorWheel,
            GCString,
            InlineString,
            OrderedMap,
            RgbValue,
            TuiColor,
            fg_rgb_color,
            get_terminal_width,
            glyphs,
            inline_string,
            new_style,
            pad_fmt,
            remove_escaped_quotes,
            rgb_value,
            truncate_from_right,
            usize,
            width};

#[derive(Debug, Default)]
pub struct CustomEventFormatter;

pub fn build_spacer(max_display_width: ColWidth) -> InlineString {
    let mut acc_padding = InlineString::with_capacity(max_display_width.as_usize());
    pad_fmt!(
        fmt: acc_padding,
        pad_str: ENTRY_SEPARATOR_CHAR,
        repeat_count: max_display_width.as_usize()
    );

    // Format spacer.
    fg_rgb_color(rgb_value!(dark_lizard_green), &acc_padding).to_small_str()
}

// Colors: <https://en.wikipedia.org/wiki/ANSI_escape_code>
#[rustfmt::skip]
pub mod custom_event_formatter_constants {
    use super::*;

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

impl<S, N> FormatEvent<S, N> for CustomEventFormatter
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    /// Format the event into 2 lines:
    /// 1. Timestamp, span context, level, and message truncated to the available visible
    ///    width.
    /// 2. Body that is text wrapped to the visible width.
    ///
    /// This function takes into account text that can contain emoji.
    ///
    /// The reason for the strange logic in
    /// [VisitEventAndPopulateOrderedMapWithFields::record_debug] and the
    /// [CustomEventFormatter::format_event] skipping empty field value lines is that we
    /// wanted to be able to have a `message` field where a String can be used instead of
    /// "stringify!" which just dumps the string literal. This does not allow the message
    /// to be a variable, which means it can't be composed using other glyphs, such as the
    /// ones from [crate::glyphs]. To get around this limitation, the following logic
    /// was implemented.
    fn format_event(
        &self,
        ctx: &tracing_subscriber::fmt::FmtContext<'_, S, N>,
        mut f: tracing_subscriber::fmt::format::Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        // Get spacer.
        let spacer = glyphs::SPACER_GLYPH;
        let spacer_display_width = GCString::width(spacer);

        // Length accumulator (for line width calculations).
        let mut line_width_used = width(0);

        // Custom timestamp.
        let timestamp = Local::now();
        let timestamp_str = inline_string!(
            "{sp}{ts}{sp}",
            ts = timestamp.format("%I:%M%P"),
            sp = spacer
        );
        line_width_used += GCString::width(&timestamp_str);
        let timestamp_str_fmt = AnsiStyledText {
            text: &timestamp_str,
            style: new_style!(
                italic
                color_fg: {TuiColor::Rgb(BODY_FG_COLOR_BRIGHT)}
                color_bg: {TuiColor::Rgb(HEADING_BG_COLOR)}
            )
            .into(),
        };
        write!(f, "\n{timestamp_str_fmt}")?;

        // Custom span context.
        if let Some(scope) = ctx.lookup_current() {
            let scope_str = inline_string!("[{}] ", scope.name());
            line_width_used += GCString::width(&scope_str);
            let scope_str_fmt = AnsiStyledText {
                text: &scope_str,
                style: new_style!(
                    italic
                    color_fg: {TuiColor::Rgb(BODY_FG_COLOR_BRIGHT)}
                    color_bg: {TuiColor::Rgb(HEADING_BG_COLOR)}
                )
                .into(),
            };
            write!(f, "{scope_str_fmt}")?;
        }

        // Custom metadata formatting. For eg:
        //
        // metadata: Metadata {
        //     name: "event src/bin/gen-certs.rs:110",
        //     target: "gen_certs",
        //     level: Level(
        //         Debug,
        //     ),
        //     module_path: "gen_certs",
        //     location: src/bin/gen-certs.rs:110,
        //     fields: {msg, body},
        //     callsite: Identifier(0x5a46fb928d40),
        //     kind: Kind(EVENT),
        // }

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
        style.bold = true;

        let level_str_fmt = AnsiStyledText {
            text: &level_str,
            style: style.into(),
        };

        let level_str_display_width = GCString::width(&level_str);
        line_width_used += spacer_display_width;
        line_width_used += level_str_display_width;
        write!(f, "{level_str_fmt}")?;

        // Custom field formatting. For eg:
        //
        // fields: ValueSet {
        //     msg: "pwd at end",
        //     body: "Ok(\"/home/nazmul/github/rust-scratch/tls\")",
        //     callsite: Identifier(0x5a46fb928d40),
        // }
        //
        // Instead of:
        // ctx.field_format().format_fields(writer.by_ref(), event)?;

        let mut order_map = OrderedMap::<InlineString, InlineString>::default();
        event.record(&mut VisitEventAndPopulateOrderedMapWithFields {
            inner: &mut order_map,
        });

        // This is actually the terminal width of the process, not necessarily the
        // terminal width of the process that is viewing the log output.
        let max_display_width = get_terminal_width();

        let text_wrap_options = Options::new(usize(*max_display_width))
            .initial_indent(FIRST_LINE_PREFIX)
            .subsequent_indent(SUBSEQUENT_LINE_PREFIX)
            .word_separator(WordSeparator::UnicodeBreakProperties);

        for (heading, body) in order_map.iter() {
            // Write heading line.
            let heading = remove_escaped_quotes(heading);
            line_width_used += spacer_display_width;
            let line_1_width = {
                let it = max_display_width - line_width_used;
                width(*it)
            };
            let line_1_text = inline_string!(
                "{spacer}{heading}",
                heading = truncate_from_right(&heading, line_1_width, false)
            );
            let line_1_text_fmt =
                ColorWheel::lolcat_into_string(&line_1_text, Some(new_style!(bold)));
            writeln!(f, "{line_1_text_fmt}")?;

            // Write body line(s).
            if !body.is_empty() {
                let body = remove_escaped_quotes(body);
                let body = wrap(&body, &text_wrap_options);
                for body_line in body.iter() {
                    let body_line =
                        truncate_from_right(body_line, max_display_width, true);
                    let body_line_fmt = AnsiStyledText {
                        text: &body_line,
                        style: new_style!(
                            color_fg: {TuiColor::Rgb(BODY_FG_COLOR)}
                        )
                        .into(),
                    };
                    writeln!(f, "{body_line_fmt}")?;
                }
            }
        }

        // Write the terminating line separator.
        writeln!(f, "{}", build_spacer(max_display_width))
    }
}

pub struct VisitEventAndPopulateOrderedMapWithFields<'a> {
    pub inner: &'a mut OrderedMap<InlineString, InlineString>,
}

impl Visit for VisitEventAndPopulateOrderedMapWithFields<'_> {
    /// The reason for the strange logic in
    /// [VisitEventAndPopulateOrderedMapWithFields::record_debug] and the
    /// [CustomEventFormatter::format_event] skipping empty field value lines is that we
    /// wanted to be able to have a `message` field where a String can be used instead of
    /// "stringify!" which just dumps the string literal. This does not allow the message
    /// to be a variable, which means it can't be composed using other glyphs, such as the
    /// ones from [crate::glyphs]. To get around this limitation, the following logic
    /// was implemented.
    ///
    /// There is some strange logic in here to handle the `message` field. The `message`
    /// field is a special field that is added by the `tracing` crate. In the example
    /// below, the statements are identical:
    ///
    /// ```rust
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
    /// [CustomEventFormatter::format_event] to skip writing the body line.
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        let field_name = field.name();
        let field_value = inline_string!("{:?}", value);
        if field_name == "message" {
            self.inner.insert(field_value, "".into());
        } else {
            self.inner.insert(field_name.into(), field_value);
        }
    }
}

#[cfg(test)]
mod tests_tracing_custom_event_formatter {
    use std::sync::Mutex;

    use chrono::Local;
    use tracing::{info, subscriber::set_default};
    use tracing_subscriber::fmt::SubscriberBuilder;

    use super::*;
    use crate::{glyphs::SPACER_GLYPH as SPACER, test_fixtures::StdoutMock};

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
