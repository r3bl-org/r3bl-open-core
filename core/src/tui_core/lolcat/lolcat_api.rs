/*
 *   Copyright (c) 2022 R3BL LLC
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

use std::{borrow::Cow, mem};

use crate::*;

/// Given a mutable [Lolcat], colorize the token tree that follows.
///
/// ```rust
/// use r3bl_rs_utils_core::*;
///
/// let mut lolcat = LolcatBuilder::new().build();
/// let content = "Hello, world!";
/// let colored_content = colorize_using_lolcat!(
///   &mut lolcat, "{}", content
/// );
/// lolcat.next_color();
/// ```
///
/// See [my_print!] for more information on how this macro is written.
#[macro_export]
macro_rules! colorize_using_lolcat {
  ($lolcat: expr, $($arg:tt)*) => {
    format!("{}", std::format_args!($($arg)*)).color_with($lolcat);
  };
}

/// Given a mutable [Lolcat] and [Style] that has `lolcat` field set to true:
/// - Colorize `text_content` and replace it w/ this colored content.
///
/// This function does nothing:
/// - If `maybe_style` is [None], then `text_content` will not be colored.
/// - If [Style] has `lolcat` field set to false, then `text_content` will not be colored.
pub fn apply_lolcat_from_style<'a>(
    maybe_style: &Option<Style>,
    lolcat: &'a mut Lolcat,
    text_content: &'a mut Cow<str>,
) {
    if let Some(Style { lolcat: true, .. }) = maybe_style {
        let unicode_string = UnicodeString::from(text_content.as_ref());
        let mut colorized_string =
            lolcat_each_char_in_unicode_string(&unicode_string, Some(lolcat));
        mem::swap(&mut colorized_string, text_content.to_mut());
    }
}

/// Given a mutable [Lolcat] reference, colorize each character of the given [UnicodeString].
///
/// ```rust
/// use r3bl_rs_utils_core::*;
///
/// let mut lolcat = LolcatBuilder::new().build();
/// let content = "Hello, world!";
/// let content_us = UnicodeString::from(content);
/// let colored_content = lolcat_each_char_in_unicode_string(
///   &content_us, Some(&mut lolcat)
/// );
/// lolcat.next_color();
/// ```
///
/// Colorizes each of the [GraphemeClusterSegment]s in the [UnicodeString] with a rapidly changing
/// color. If you don't pass in your own [Lolcat] then a random one will be created for you, which
/// changes the color rapidly between each segment.
pub fn lolcat_each_char_in_unicode_string(
    unicode_string: &UnicodeString,
    lolcat: Option<&mut Lolcat>,
) -> String {
    let mut saved_orig_speed = None;

    let mut my_lolcat: Cow<Lolcat> = match lolcat {
        Some(lolcat_arg) => {
            saved_orig_speed = Some(lolcat_arg.color_wheel_control.color_change_speed);
            lolcat_arg.color_wheel_control.color_change_speed = ColorChangeSpeed::Rapid;
            Cow::Borrowed(lolcat_arg)
        }
        None => {
            let lolcat_temp = LolcatBuilder::new()
                .set_color_change_speed(ColorChangeSpeed::Rapid)
                .build();
            Cow::Owned(lolcat_temp)
        }
    };

    let mut return_vec: Vec<String> = vec![];
    for letter in unicode_string.vec_segment.iter() {
        let colored_letter = letter.string.color_with(my_lolcat.to_mut());
        return_vec.push(colored_letter);
    }

    // Restore saved_orig_speed if it was set.
    if let Some(orig_speed) = saved_orig_speed {
        my_lolcat.to_mut().color_wheel_control.color_change_speed = orig_speed;
    }

    return_vec.join("")
}
