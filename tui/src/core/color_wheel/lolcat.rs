// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Lolcat-style rainbow text colorization functionality.
//!
//! This module provides the classic "lolcat" rainbow text effect:
//! - `Lolcat` - Core struct for rainbow colorization
//! - `LolcatBuilder` - Builder pattern for configuring Lolcat instances
//! - `Colorize` - Enum for foreground-only or background+foreground coloring
//! - Helper functions for easy text colorization
//!
//! The lolcat functionality creates smooth rainbow gradients across text,
//! similar to the popular command-line `lolcat` tool. This module consolidates
//! the previously separate `lolcat_api.rs` and `lolcat_impl.rs` files.

use std::{borrow::Cow,
          fmt::{Debug, Formatter, Result}};

use super::{ColorChangeSpeed, ColorWheelControl, Seed, SeedDelta, color_wheel_helpers};
use crate::{GCStringOwned, TuiStyle, TuiStyledTexts, tui_color, tui_styled_text};

/// Please use the [`LolcatBuilder`] to create this struct (lots of documentation is
/// provided here). Please do not use this struct directly.
#[derive(Clone, Copy, PartialEq)]
pub struct Lolcat {
    pub color_wheel_control: ColorWheelControl,
    pub seed_delta: SeedDelta,
}

impl Default for Lolcat {
    fn default() -> Self { LolcatBuilder::new().build() }
}

impl Debug for Lolcat {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        /// More info: <https://stackoverflow.com/questions/63214346/how-to-truncate-f64-to-2-decimal-places>
        fn pretty_print_f64(before: f64) -> f64 { f64::trunc(before * 100.0) / 100.0 }

        write!(
            f,
            "lolcat: [{}, {}, {}, {}]",
            pretty_print_f64(*self.color_wheel_control.seed),
            pretty_print_f64(*self.color_wheel_control.spread),
            pretty_print_f64(*self.color_wheel_control.frequency),
            self.color_wheel_control.color_change_speed
        )
    }
}

impl Lolcat {
    /// This function does not respect [`crate::global_color_support::detect()`]
    /// (it will always colorize to truecolor regardless of terminal limitations). Use
    /// [`crate::ColorWheel`] if you want to respect
    /// [`crate::global_color_support::detect`].
    pub fn colorize_to_styled_texts(&mut self, us: &GCStringOwned) -> TuiStyledTexts {
        let mut acc = TuiStyledTexts::default();

        for seg in us.iter() {
            let seg_str = seg.get_str(us.as_str());
            let new_color =
                color_wheel_helpers::get_color_tuple(&self.color_wheel_control);
            let derived_from_new_color = color_wheel_helpers::calc_fg_color(new_color);

            let style = if self.color_wheel_control.background_mode
                == Colorize::BothBackgroundAndForeground
            {
                TuiStyle {
                    color_fg: Some(tui_color!(
                        derived_from_new_color.0,
                        derived_from_new_color.1,
                        derived_from_new_color.2,
                    )),
                    color_bg: Some(tui_color!(new_color.0, new_color.1, new_color.2,)),
                    ..Default::default()
                }
            } else {
                TuiStyle {
                    color_fg: Some(tui_color!(new_color.0, new_color.1, new_color.2,)),
                    ..Default::default()
                }
            };

            acc += tui_styled_text!(
                @style: style,
                @text: seg_str,
            );

            self.color_wheel_control.seed +=
                Seed::from(self.color_wheel_control.color_change_speed);
        }

        acc
    }

    pub fn next_color(&mut self) { self.color_wheel_control.seed += self.seed_delta; }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Colorize {
    /// The background is colorized, and the foreground is computed for contrast. The
    /// primary effect here is that the background of the generated colors is what is
    /// being lolcat'd.
    BothBackgroundAndForeground,
    /// Only the foreground color is cycled, background is left alone.
    OnlyForeground,
}

/// A builder struct for the [Lolcat] struct. Example usage:
///
/// ```
/// use r3bl_tui::*;
///
/// let mut lolcat = LolcatBuilder::new()
///   .set_color_change_speed(ColorChangeSpeed::Rapid)
///   .set_seed(1.0)
///   .set_seed_delta(1.0)
///   .build();
///
/// let string = "Hello, world!";
/// let string_gcs: GCStringOwned = string.into();
/// let lolcat_mut = &mut lolcat;
/// let st = lolcat_mut.colorize_to_styled_texts(&string_gcs);
/// lolcat.next_color();
/// ```
///
/// This [Lolcat] that is returned by `build()` is safe to re-use.
/// - The colors it cycles through are "stable" meaning that once constructed via the
///   [builder](LolcatBuilder) (which sets the speed, seed, and delta that determine where
///   the color wheel starts when it is used). For eg, when used in a dialog box component
///   that re-uses the instance, repeated calls to the `render()` function of this
///   component will produce the same generated colors over and over again.
/// - If you want to change where the color wheel "begins", you have to change the speed,
///   seed, and delta of this [Lolcat] instance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LolcatBuilder {
    /// Rate at which the color changes when
    /// [`colorize_to_styled_texts`](Lolcat::colorize_to_styled_texts) is called.
    pub color_change_speed: ColorChangeSpeed,
    /// Initial color of the wheel.
    pub seed: Seed,
    /// Delta that should be applied to the seed for it to change colors.
    pub seed_delta: SeedDelta,
    /// Colorize the background and foreground, or just the foreground.
    pub colorization_strategy: Colorize,
}

impl Default for LolcatBuilder {
    fn default() -> Self {
        Self {
            color_change_speed: ColorChangeSpeed::Slow,
            seed: 1.0.into(),
            seed_delta: 1.0.into(),
            colorization_strategy: Colorize::OnlyForeground, /* color only the
                                                              * foreground */
        }
    }
}

impl LolcatBuilder {
    #[must_use]
    pub fn new() -> Self { Self::default() }

    #[must_use]
    pub fn set_background_mode(mut self, background_mode: Colorize) -> Self {
        self.colorization_strategy = background_mode;
        self
    }

    #[must_use]
    pub fn set_color_change_speed(
        mut self,
        color_change_speed: ColorChangeSpeed,
    ) -> Self {
        self.color_change_speed = color_change_speed;
        self
    }

    #[must_use]
    pub fn set_seed(mut self, arg_seed: impl Into<Seed>) -> Self {
        self.seed = arg_seed.into();
        self
    }

    #[must_use]
    pub fn set_seed_delta(mut self, arg_seed_delta: impl Into<SeedDelta>) -> Self {
        self.seed_delta = arg_seed_delta.into();
        self
    }

    #[must_use]
    pub fn build(self) -> Lolcat {
        let mut new_lolcat = Lolcat {
            seed_delta: self.seed_delta,
            color_wheel_control: ColorWheelControl::default(),
        };

        new_lolcat.color_wheel_control.color_change_speed = self.color_change_speed;
        new_lolcat.color_wheel_control.seed = self.seed;
        new_lolcat.color_wheel_control.background_mode = self.colorization_strategy;

        new_lolcat
    }

    pub fn apply(&self, lolcat: &mut Lolcat) {
        lolcat.color_wheel_control.color_change_speed = self.color_change_speed;
        lolcat.color_wheel_control.seed = self.seed;
        lolcat.seed_delta = self.seed_delta;
    }
}

pub fn colorize_to_styled_texts(
    lolcat: &mut Lolcat,
    arg_str: impl AsRef<str>,
) -> TuiStyledTexts {
    let str = arg_str.as_ref();
    let string_gcs: GCStringOwned = str.into();
    lolcat.colorize_to_styled_texts(&string_gcs)
}

#[must_use]
pub fn lolcat_each_char_in_unicode_string(
    us: &GCStringOwned,
    lolcat: Option<&mut Lolcat>,
) -> TuiStyledTexts {
    let mut saved_orig_speed = None;

    let mut my_lolcat: Cow<'_, Lolcat> = if let Some(lolcat_arg) = lolcat {
        saved_orig_speed = Some(lolcat_arg.color_wheel_control.color_change_speed);
        lolcat_arg.color_wheel_control.color_change_speed = ColorChangeSpeed::Rapid;
        Cow::Borrowed(lolcat_arg)
    } else {
        let lolcat_temp = LolcatBuilder::new()
            .set_color_change_speed(ColorChangeSpeed::Rapid)
            .build();
        Cow::Owned(lolcat_temp)
    };

    let it = my_lolcat.to_mut().colorize_to_styled_texts(us);

    // Restore saved_orig_speed if it was set.
    if let Some(orig_speed) = saved_orig_speed {
        my_lolcat.to_mut().color_wheel_control.color_change_speed = orig_speed;
    }

    it
}
