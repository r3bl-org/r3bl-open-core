
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

use serde::{Deserialize, Serialize};

use crate::*;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Stylesheet {
    pub styles: Vec<Style>,
}

#[macro_export]
macro_rules! get_style {
    (
        @from_result: $arg_stylesheet_result : expr, // Eg: from: stylesheet,
        $arg_style_name : expr                      // Eg: "style1"
    ) => {
        if let Ok(ref it) = $arg_stylesheet_result {
            it.find_style_by_id($arg_style_name)
        } else {
            None
        }
    };

    (
        @from: $arg_stylesheet : expr, // Eg: from: stylesheet,
        $arg_style_name : expr        // Eg: "style1"
    ) => {
        $arg_stylesheet.find_style_by_id($arg_style_name)
    };
}

#[macro_export]
macro_rules! get_styles {
    (
        @from_result: $arg_stylesheet_result : expr, // Eg: from: stylesheet,
        [$($args:tt)*]                              // Eg: ["style1", "style2"]
    ) => {
        if let Ok(ref it) = $arg_stylesheet_result {
            it.find_styles_by_ids(vec![$($args)*])
        } else {
            None
        }
    };

    (
        @from: $arg_stylesheet : expr, // Eg: from: stylesheet,
        [$($args:tt)*]                // Eg: ["style1", "style2"]
    ) => {
        $arg_stylesheet.find_styles_by_ids(vec![$($args)*])
    };
}

impl Stylesheet {
    pub fn new() -> Self { Self::default() }

    pub fn add_style(&mut self, style: Style) -> CommonResult<()> {
        throws!({
            if style.id == u8::MAX {
                return CommonError::new_err_with_only_msg("Style id must be defined");
            }
            self.styles.push(style);
        });
    }

    pub fn add_styles(&mut self, styles: Vec<Style>) -> CommonResult<()> {
        throws!({
            for style in styles {
                self.add_style(style)?;
            }
        });
    }

    pub fn find_style_by_id(&self, id: u8) -> Option<Style> {
        self.styles.iter().find(|style| style.id == id).cloned()
    }

    /// Returns [None] if no style in `ids` [Vec] is found.
    pub fn find_styles_by_ids(&self, ids: Vec<u8>) -> Option<Vec<Style>> {
        let mut styles = Vec::new();

        for id in ids {
            if let Some(style) = self.find_style_by_id(id) {
                styles.push(style);
            }
        }

        if styles.is_empty() {
            None
        } else {
            styles.into()
        }
    }

    pub fn compute(styles: &Option<Vec<Style>>) -> Option<Style> {
        if let Some(styles) = styles {
            let mut computed = Style::default();
            styles.iter().for_each(|style| computed += style);
            computed.into()
        } else {
            None
        }
    }
}

/// Macro to make building [Stylesheet] easy. This returns a [CommonResult] because it checks to see
/// that all [Style]s that are added have an `id`. If they don't, then an a [CommonError] is thrown.
/// This is to ensure that valid styles are added to a stylesheet. Without an `id`, they can't be
/// retrieved after they're added here, rendering them useless.
///
/// Here's an example.
/// ```ignore
/// fn create_stylesheet() -> CommonResult<Stylesheet> {
///   throws_with_return!({
///     stylesheet! {
///         style! {
///           id: style1
///           padding: 1
///           color_bg: Color::Rgb { r: 55, g: 55, b: 248 }
///         },
///         vec![
///             style! {
///                 id: style1
///                 padding: 1
///                 color_bg: Color::Rgb { r: 55, g: 55, b: 248 }
///             },
///             style! {
///                 id: style2
///                 padding: 1
///                 color_bg: Color::Rgb { r: 85, g: 85, b: 255 }
///             },
///         ]
///     }
///   })
/// }
/// ```
#[macro_export]
macro_rules! stylesheet {
    (
        $($style:expr),*
        $(,)* /* Optional trailing comma https://stackoverflow.com/a/43143459/2085356. */
    ) => {
    {
        let mut stylesheet = Stylesheet::new();
            $(
                stylesheet.try_add($style)?;
            )*
            stylesheet
        }
    };
}

/// This trait exists to allow "pseudo operator overloading". Rust does not support operator
/// overloading, and the method to add a single style has a different signature than the one to add
/// a vector of styles. To get around this, the [TryAdd] trait is implemented for both [Style] and
/// [`Vec<Style>`]. Then the [stylesheet!] macro can "pseudo overload" them.
pub trait TryAdd<OtherType = Self> {
    fn try_add(&mut self, other: OtherType) -> CommonResult<()>;
}

impl TryAdd<Style> for Stylesheet {
    fn try_add(&mut self, other: Style) -> CommonResult<()> { self.add_style(other) }
}

impl TryAdd<Vec<Style>> for Stylesheet {
    fn try_add(&mut self, other: Vec<Style>) -> CommonResult<()> {
        self.add_styles(other)
    }
}
