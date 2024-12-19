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

use std::fmt;

use chrono::Local;
use crossterm::style::Stylize;
use custom_event_formatter_constants::*;
use r3bl_ansi_color::{AnsiStyledText, Color, Style};
use r3bl_core::{ColorWheel,
                OrderedMap,
                UnicodeString,
                ch,
                get_terminal_width,
                remove_escaped_quotes,
                string_helpers_constants,
                truncate_from_right};
use r3bl_macro::tui_style;
use textwrap::{Options, WordSeparator, wrap};
use tracing::{Event,
              Subscriber,
              field::{Field, Visit}};
use tracing_subscriber::{fmt::{FormatEvent, FormatFields},
                         registry::LookupSpan};

pub struct CustomEventFormatter;

// Colors: <https://en.wikipedia.org/wiki/ANSI_escape_code>
pub mod custom_event_formatter_constants {
    use super::*;

    pub const FIRST_LINE_PREFIX: &str = " 𜱐 ";
    pub const SUBSEQUENT_LINE_PREFIX: &str = " ";
    pub const LEVEL_SUFFIX: &str = ":";

    pub const ERROR_SIGIL: &str = "E";
    pub const WARN_SIGIL: &str = "W";
    pub const INFO_SIGIL: &str = "I";
    pub const DEBUG_SIGIL: &str = "D";
    pub const TRACE_SIGIL: &str = "T";

    pub const ENTRY_SEPARATOR_CHAR: &str = "‾";

    pub const BODY_FG_COLOR: Color = Color::Rgb(175, 175, 175);
    pub const BODY_FG_COLOR_BRIGHT: Color = Color::Rgb(200, 200, 200);
    pub const HEADING_BG_COLOR: Color = Color::Rgb(70, 70, 90);

    pub const INFO_FG_COLOR: Color = Color::Rgb(233, 150, 122);
    pub const ERROR_FG_COLOR: Color = Color::Rgb(255, 182, 193); //Color::Rgb(220, 92, 92);
    pub const WARN_FG_COLOR: Color = Color::Rgb(255, 140, 0);
    pub const DEBUG_FG_COLOR: Color = Color::Rgb(255, 255, 0);
    pub const TRACE_FG_COLOR: Color = Color::Rgb(186, 85, 211);
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
    fn format_event(
        &self,
        ctx: &tracing_subscriber::fmt::FmtContext<'_, S, N>,
        mut writer: tracing_subscriber::fmt::format::Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        // Get spacer.
        let spacer = string_helpers_constants::SPACER;
        let spacer_display_width = UnicodeString::from(spacer).display_width;

        // Length accumulator (for line width calculations).
        let mut line_width_used = ch!(0);

        // Custom timestamp.
        let timestamp = Local::now();
        let timestamp_str =
            format!("{ts}{sp}", ts = timestamp.format("%I:%M%P"), sp = spacer);
        line_width_used += UnicodeString::from(&timestamp_str).display_width;
        let timestamp_str_fmt = AnsiStyledText {
            text: &timestamp_str,
            style: &[
                Style::Foreground(BODY_FG_COLOR_BRIGHT),
                Style::Background(HEADING_BG_COLOR),
            ],
        };
        write!(writer, "\n{timestamp_str_fmt}")?;

        // Custom span context.
        if let Some(scope) = ctx.lookup_current() {
            let scope_str = format!("[{}] ", scope.name());
            line_width_used += UnicodeString::from(&scope_str).display_width;
            let scope_str_fmt = AnsiStyledText {
                text: &scope_str,
                style: &[
                    Style::Foreground(BODY_FG_COLOR_BRIGHT),
                    Style::Background(HEADING_BG_COLOR),
                    Style::Italic,
                ],
            };
            write!(writer, "{scope_str_fmt}")?;
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
        let mut style_acc: Vec<Style> = vec![];
        let level_str = match *event.metadata().level() {
            tracing::Level::ERROR => {
                style_acc.push(Style::Foreground(ERROR_FG_COLOR));
                format!("{ERROR_SIGIL}{LEVEL_SUFFIX}{spacer}")
            }
            tracing::Level::WARN => {
                style_acc.push(Style::Foreground(WARN_FG_COLOR));
                format!("{WARN_SIGIL}{LEVEL_SUFFIX}{spacer}")
            }
            tracing::Level::INFO => {
                style_acc.push(Style::Foreground(INFO_FG_COLOR));
                format!("{INFO_SIGIL}{LEVEL_SUFFIX}{spacer}")
            }
            tracing::Level::DEBUG => {
                style_acc.push(Style::Foreground(DEBUG_FG_COLOR));
                format!("{DEBUG_SIGIL}{LEVEL_SUFFIX}{spacer}")
            }
            tracing::Level::TRACE => {
                style_acc.push(Style::Foreground(TRACE_FG_COLOR));
                format!("{TRACE_SIGIL}{LEVEL_SUFFIX}{spacer}")
            }
        };
        style_acc.push(Style::Background(HEADING_BG_COLOR));
        style_acc.push(Style::Bold);
        let level_str_fmt = AnsiStyledText {
            text: &level_str,
            style: &style_acc,
        };
        let level_str_display_width = UnicodeString::from(&level_str).display_width;
        line_width_used += spacer_display_width;
        line_width_used += level_str_display_width;
        write!(writer, "{level_str_fmt}")?;

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
        let mut ordered_map = OrderedMap::<String, String>::default();
        event.record(&mut VisitEventAndPopulateOrderedMapWithFields {
            inner: &mut ordered_map,
        });

        // This is actually the terminal width of the process, not necessarily the
        // terminal width of the process that is viewing the log output.
        let max_display_width = ch!(get_terminal_width());

        let text_wrap_options = Options::new(ch!(@to_usize max_display_width))
            .initial_indent(FIRST_LINE_PREFIX)
            .subsequent_indent(SUBSEQUENT_LINE_PREFIX)
            .word_separator(WordSeparator::UnicodeBreakProperties);

        for (heading, body) in ordered_map.iter() {
            // Prepare the msg and body.
            let heading = remove_escaped_quotes(heading);
            let body = remove_escaped_quotes(body);

            // Write heading line.
            line_width_used += spacer_display_width;
            let line_1_width = max_display_width - line_width_used;
            let line_1_text = format!(
                "{spacer}{heading}",
                heading =
                    truncate_from_right(&heading, ch!(@to_usize line_1_width), false)
            );
            let line_1_text_fmt = ColorWheel::lolcat_into_string(
                &line_1_text,
                Some(tui_style!(
                    attrib: [bold]
                )),
            );
            writeln!(writer, "{line_1_text_fmt}")?;

            // Write body line(s).
            let body = wrap(&body, &text_wrap_options);
            for body_line in body.iter() {
                let body_line = truncate_from_right(body_line, max_display_width, true);
                // let body_line_fmt = body_line.to_string().dark_green();
                let body_line_fmt = AnsiStyledText {
                    text: &body_line,
                    style: &[Style::Foreground(BODY_FG_COLOR)],
                };
                writeln!(writer, "{body_line_fmt}")?;
            }
        }

        // Write the terminating line separator.
        let line_separator =
            ENTRY_SEPARATOR_CHAR.repeat(ch!(@to_usize max_display_width));
        let line_separator_fmt = line_separator.dark_green();
        writeln!(writer, "{line_separator_fmt}")
    }
}

pub struct VisitEventAndPopulateOrderedMapWithFields<'a> {
    inner: &'a mut OrderedMap<String, String>,
}

impl Visit for VisitEventAndPopulateOrderedMapWithFields<'_> {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        let field_name = field.name();
        let field_value = format!("{:?}", value);
        self.inner.insert(field_name.to_string(), field_value);
    }
}

#[cfg(test)]
mod tests_tracing_custom_event_formatter {
    use std::sync::Mutex;

    use chrono::Local;
    use r3bl_test_fixtures::StdoutMock;
    use string_helpers_constants::SPACER;
    use tracing::{info, subscriber::set_global_default};
    use tracing_subscriber::fmt::SubscriberBuilder;

    use super::*;

    #[test]
    fn test_custom_formatter() {
        let mock_stdout = StdoutMock::new();
        let mock_stdout_clone = mock_stdout.clone();
        let subscriber = SubscriberBuilder::default()
            .event_format(CustomEventFormatter)
            .with_writer(Mutex::new(mock_stdout))
            .finish();

        set_global_default(subscriber).expect("Failed to set subscriber");

        info!(message = "This is a test log entry");

        let time = Local::now().format("%I:%M%P").to_string();
        let it = mock_stdout_clone.get_copy_of_buffer_as_string();
        let it_no_ansi = mock_stdout_clone.get_copy_of_buffer_as_string_strip_ansi();

        // println!("{}", it);
        // println!("{}", it_no_ansi);

        assert!(it_no_ansi.contains("message")); // lolcat colorized each char, so strip the colors.
        assert!(it.matches(FIRST_LINE_PREFIX).count() >= 1);
        assert!(it.matches(SPACER).count() >= 1);
        assert!(it.contains("This is a test log entry"));
        assert!(it.contains(&format!("{INFO_SIGIL}{LEVEL_SUFFIX}")));
        assert!(it.contains(&time));
        assert!(it.matches('\n').count() >= 4); // There are many new lines.
    }
}
