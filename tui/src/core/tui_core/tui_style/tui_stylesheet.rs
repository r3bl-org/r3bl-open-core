/*
 *   Copyright (c) 2022-2025 R3BL LLC
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

use super::{tui_style_attrib, TuiStyle};
use crate::{throws, CommonError, CommonResult, InlineVec};

#[derive(Default, Debug, Clone)]
pub struct TuiStylesheet {
    pub styles: InlineVec<TuiStyle>,
}

#[macro_export]
macro_rules! get_tui_style {
    (
        @from_result: $arg_stylesheet_result : expr, // Eg: from: stylesheet,
        $arg_style_name : expr                       // Eg: "style1"
    ) => {
        if let Ok(ref it) = $arg_stylesheet_result {
            it.find_style_by_id($arg_style_name)
        } else {
            None
        }
    };

    (
        @from: $arg_stylesheet : expr, // Eg: from: stylesheet,
        $arg_style_name : expr         // Eg: "style1"
    ) => {
        $arg_stylesheet.find_style_by_id($arg_style_name)
    };
}

#[macro_export]
macro_rules! get_tui_styles {
    (
        @from_result: $arg_stylesheet_result : expr, // Eg: from: stylesheet,
        [$($args:tt)*]                               // Eg: ["style1", "style2"]
    ) => {
        if let Ok(ref it) = $arg_stylesheet_result {
            it.find_styles_by_ids(&[$($args)*])
        } else {
            None
        }
    };

    (
        @from: $arg_stylesheet : expr, // Eg: from: stylesheet,
        [$($args:tt)*]                 // Eg: ["style1", "style2"]
    ) => {
        $arg_stylesheet.find_styles_by_ids(&[$($args)*])
    };
}

impl TuiStylesheet {
    pub fn new() -> Self { Self::default() }

    pub fn add_style(&mut self, style: TuiStyle) -> CommonResult<()> {
        throws!({
            if style.id.is_none() {
                return CommonError::new_error_result_with_only_msg(
                    "Style id must be defined",
                );
            }
            self.styles.push(style);
        });
    }

    pub fn add_styles(&mut self, styles: InlineVec<TuiStyle>) -> CommonResult<()> {
        throws!({
            for style in styles {
                self.add_style(style)?;
            }
        });
    }

    pub fn find_style_by_id(&self, arg_id: impl Into<u8>) -> Option<TuiStyle> {
        let id: u8 = arg_id.into();
        self.styles
            .iter()
            .find(|style| tui_style_attrib::Id::eq(style.id, id))
            .cloned()
    }

    /// Returns [None] if no style in `ids` [Vec] is found.
    pub fn find_styles_by_ids(&self, ids: &[u8]) -> Option<InlineVec<TuiStyle>> {
        let mut styles = InlineVec::<TuiStyle>::new();

        for id in ids {
            if let Some(style) = self.find_style_by_id(*id) {
                styles.push(style);
            }
        }

        if styles.is_empty() {
            None
        } else {
            styles.into()
        }
    }

    pub fn compute(styles: &Option<InlineVec<TuiStyle>>) -> Option<TuiStyle> {
        if let Some(styles) = styles {
            let mut computed = TuiStyle::default();
            styles.iter().for_each(|style| computed += style);
            computed.into()
        } else {
            None
        }
    }
}

/// Macro to make building [TuiStylesheet] easy.
///
/// This returns a [CommonResult] because it checks to see that all [TuiStyle]s that are
/// added have an `id`. If they don't, then an a [CommonError] is thrown. This is to
/// ensure that valid styles are added to a stylesheet. Without an `id`, they can't be
/// retrieved after they're added here, rendering them useless.
///
/// Here's an example.
/// ```
/// # use r3bl_tui::{
///     ch, ChUnit, TuiColor, RgbValue, TuiStyle, TryAdd, tui_stylesheet,
///     CommonResult, throws_with_return, TuiStylesheet, tui_style_attrib
/// };
/// fn create_tui_stylesheet() -> CommonResult<TuiStylesheet> {
///   throws_with_return!({
///     tui_stylesheet! {
///         TuiStyle {
///             id: tui_style_attrib::id(1),
///             padding: Some(ch(1)),
///             color_bg: Some(TuiColor::Rgb(RgbValue::from_u8(55, 55, 248))),
///             ..Default::default()
///         },
///         smallvec::smallvec![
///             TuiStyle {
///                 id: tui_style_attrib::id(2),
///                 padding: Some(ch(1)),
///                 color_bg: Some(TuiColor::Rgb(RgbValue::from_u8(155, 155, 48))),
///                 ..Default::default()
///             },
///             TuiStyle {
///                 id: tui_style_attrib::id(3),
///                 padding: Some(ch(1)),
///                 color_bg: Some(TuiColor::Rgb(RgbValue::from_u8(5, 5, 48))),
///                 ..Default::default()
///             },
///         ]
///     }
///   })
/// }
/// ```
#[macro_export]
macro_rules! tui_stylesheet {
    (
        $($style:expr),*
        $(,)* /* Optional trailing comma https://stackoverflow.com/a/43143459/2085356. */
    ) => {
    {
        use $crate::TryAdd as _;
        let mut stylesheet = $crate::TuiStylesheet::new();
            $(
                stylesheet.try_add($style)?;
            )*
            stylesheet
        }
    };
}

/// This trait exists to allow "pseudo operator overloading".
///
/// Rust does not support operator overloading, and the method to add a single style has a
/// different signature than the one to add a vector of styles. To get around this, the
/// [TryAdd] trait is implemented for both [TuiStyle] and [`Vec<Style>`]. Then the
/// [tui_stylesheet!] macro can "pseudo overload" them.
pub trait TryAdd<OtherType = Self> {
    fn try_add(&mut self, other: OtherType) -> CommonResult<()>;
}

impl TryAdd<TuiStyle> for TuiStylesheet {
    fn try_add(&mut self, other: TuiStyle) -> CommonResult<()> { self.add_style(other) }
}

impl TryAdd<InlineVec<TuiStyle>> for TuiStylesheet {
    fn try_add(&mut self, other: InlineVec<TuiStyle>) -> CommonResult<()> {
        self.add_styles(other)
    }
}
