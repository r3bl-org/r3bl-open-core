// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::TuiStyle;
use crate::{InlineVec, TuiStyleId};

/// Error returned when adding a [`TuiStyle`] without a defined [`TuiStyleId`] to a
/// [`TuiStylesheet`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error, miette::Diagnostic)]
#[error("Style id must be defined")]
pub struct StylesheetError;

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
    #[must_use]
    pub fn new() -> Self { Self::default() }

    /// # Errors
    ///
    /// Returns [`StylesheetError`] if the style id is not defined.
    pub fn try_add_style(&mut self, style: TuiStyle) -> Result<(), StylesheetError> {
        throws!({
            if style.id.is_none() {
                return Err(StylesheetError);
            }
            self.styles.push(style);
        });
    }

    /// # Errors
    ///
    /// Returns [`StylesheetError`] if any of the styles have undefined ids.
    pub fn try_add_styles(
        &mut self,
        styles: InlineVec<TuiStyle>,
    ) -> Result<(), StylesheetError> {
        throws!({
            for style in styles {
                self.try_add_style(style)?;
            }
        });
    }

    pub fn find_style_by_id(&self, arg_id: impl Into<u8>) -> Option<TuiStyle> {
        let id: u8 = arg_id.into();
        self.styles
            .iter()
            .find(|style| TuiStyleId::eq(style.id, id))
            .copied()
    }

    /// Returns [None] if no style in `ids` [Vec] is found.
    #[must_use]
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

    #[must_use]
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

/// Macro to make building [`TuiStylesheet`] easy.
///
/// This uses `?` to propagate [`StylesheetError`] when any added [`TuiStyle`] is missing
/// an `id`. Without an `id`, styles can't be retrieved after they're added here, rendering
/// them useless.
///
/// Here's an example.
/// ```
/// # use r3bl_tui::{
///     ch, ChUnit, TuiColor, RgbValue, TuiStyle, TryAdd, tui_stylesheet,
///     throws_with_return, TuiStylesheet, tui_style_attrib, TuiStyleId
/// };
/// fn create_tui_stylesheet() -> miette::Result<TuiStylesheet> {
///   throws_with_return!({
///     tui_stylesheet! {
///         TuiStyle {
///             id: Some(TuiStyleId(1)),
///             padding: Some(ch(1)),
///             color_bg: Some(TuiColor::Rgb(RgbValue::from_u8(55, 55, 248))),
///             ..Default::default()
///         },
///         smallvec::smallvec![
///             TuiStyle {
///                 id: Some(TuiStyleId(2)),
///                 padding: Some(ch(1)),
///                 color_bg: Some(TuiColor::Rgb(RgbValue::from_u8(155, 155, 48))),
///                 ..Default::default()
///             },
///             TuiStyle {
///                 id: Some(TuiStyleId(3)),
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
        use $crate::TryAdd;
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
/// [`TryAdd`] trait is implemented for both [`TuiStyle`] and [`Vec<Style>`]. Then the
/// [`tui_stylesheet`!] macro can "pseudo overload" them.
pub trait TryAdd<OtherType = Self> {
    /// # Errors
    ///
    /// Returns [`StylesheetError`] if the operation fails (e.g., missing style id).
    fn try_add(&mut self, other: OtherType) -> Result<(), StylesheetError>;
}

impl TryAdd<TuiStyle> for TuiStylesheet {
    #[allow(clippy::missing_errors_doc)]
    fn try_add(&mut self, other: TuiStyle) -> Result<(), StylesheetError> {
        self.try_add_style(other)
    }
}

impl TryAdd<InlineVec<TuiStyle>> for TuiStylesheet {
    #[allow(clippy::missing_errors_doc)]
    fn try_add(&mut self, other: InlineVec<TuiStyle>) -> Result<(), StylesheetError> {
        self.try_add_styles(other)
    }
}
