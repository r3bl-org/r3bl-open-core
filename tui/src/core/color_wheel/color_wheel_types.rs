// Copyright (c) 2023-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Core data types for the color wheel functionality.
//!
//! This module contains the fundamental types used throughout the color wheel system:
//! - Basic value types: `Seed`, `Spread`, `Frequency`, `SeedDelta`
//! - Speed control: `ColorChangeSpeed`
//! - Central control structure: `ColorWheelControl`
//!
//! These types were previously split between `color_wheel_types.rs` and
//! `color_wheel_control.rs` but have been consolidated for better organization.

use std::{fmt::{Display, Formatter},
          ops::{AddAssign, Deref, DerefMut}};

use rand::random;

use super::lolcat::Colorize;

// ================================================================================================
// Basic types
// ================================================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Seed(pub f64);

mod seed {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl Deref for Seed {
        type Target = f64;

        fn deref(&self) -> &Self::Target { &self.0 }
    }

    impl DerefMut for Seed {
        fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
    }

    impl From<f64> for Seed {
        fn from(f: f64) -> Self { Self(f) }
    }

    impl AddAssign<SeedDelta> for Seed {
        fn add_assign(&mut self, delta: SeedDelta) { self.0 += delta.0; }
    }

    impl AddAssign<Seed> for Seed {
        fn add_assign(&mut self, other: Seed) { self.0 += other.0; }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spread(pub f64);

mod spread {
    use super::{Deref, DerefMut, Spread};

    impl Deref for Spread {
        type Target = f64;

        fn deref(&self) -> &Self::Target { &self.0 }
    }

    impl DerefMut for Spread {
        fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
    }

    impl From<f64> for Spread {
        fn from(f: f64) -> Self { Self(f) }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frequency(pub f64);

mod frequency {
    use super::{Deref, DerefMut, Frequency};

    impl Deref for Frequency {
        type Target = f64;

        fn deref(&self) -> &Self::Target { &self.0 }
    }

    impl DerefMut for Frequency {
        fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
    }

    impl From<f64> for Frequency {
        fn from(f: f64) -> Self { Self(f) }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SeedDelta(pub f64);

mod seed_delta {
    use super::{Deref, DerefMut, SeedDelta};

    impl Deref for SeedDelta {
        type Target = f64;

        fn deref(&self) -> &Self::Target { &self.0 }
    }

    impl DerefMut for SeedDelta {
        fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
    }

    impl From<f64> for SeedDelta {
        fn from(f: f64) -> Self { Self(f) }
    }
}

// ================================================================================================
// ColorChangeSpeed
// ================================================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ColorChangeSpeed {
    #[default]
    Rapid,
    Slow,
}

impl Display for ColorChangeSpeed {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ColorChangeSpeed::Rapid => write!(f, "Rapid"),
            ColorChangeSpeed::Slow => write!(f, "Slow"),
        }
    }
}

impl From<ColorChangeSpeed> for Seed {
    /// The float is added to seed in [`crate::Lolcat`] after every iteration. If the
    /// number is `Rapid` then the changes in color between new lines is quite abrupt.
    /// If it is `Slow` then the changes are much much smoother. And so this is the
    /// default.
    fn from(value: ColorChangeSpeed) -> Seed {
        match value {
            ColorChangeSpeed::Rapid => 1.0.into(),
            ColorChangeSpeed::Slow => 0.1.into(),
        }
    }
}

// ================================================================================================
// ColorWheelControl
// ================================================================================================

/// A struct to contain info we need to print with every character.
#[derive(Debug, Clone, Copy)]
pub struct ColorWheelControl {
    pub seed: Seed,
    pub spread: Spread,
    pub frequency: Frequency,
    pub background_mode: Colorize,
    pub color_change_speed: ColorChangeSpeed,
}

impl PartialEq for ColorWheelControl {
    /// More info:
    /// 1. <https://stackoverflow.com/questions/67951688/comparing-structs-with-floating-point-numbers-in-rust>
    /// 2. <https://doc.rust-lang.org/std/primitive.f64.html#associatedconstant.EPSILON>
    /// 3. <https://rust-lang.github.io/rust-clippy/master/index.html#float_equality_without_abs>
    fn eq(&self, other: &Self) -> bool {
        (*self.seed - *other.seed).abs() < f64::EPSILON // self.seed == other.seed
            && *self.spread == *other.spread
            && *self.frequency == *other.frequency
            && self.background_mode == other.background_mode
            && self.color_change_speed == other.color_change_speed
    }
}

impl ColorWheelControl {
    pub fn new(
        arg_seed: impl Into<Seed>,
        arg_spread: impl Into<Spread>,
        arg_frequency: impl Into<Frequency>,
        color_change: ColorChangeSpeed,
    ) -> ColorWheelControl {
        let mut seed: Seed = arg_seed.into();
        if *seed == 0.0 {
            *seed = random::<f64>() * 10e9;
        }
        let spread: Spread = arg_spread.into();
        let frequency: Frequency = arg_frequency.into();

        ColorWheelControl {
            seed,
            spread,
            frequency,
            background_mode: Colorize::OnlyForeground,
            color_change_speed: color_change,
        }
    }
}

impl Default for ColorWheelControl {
    fn default() -> Self { Self::new(0.0, 3.0, 0.1, ColorChangeSpeed::Slow) }
}
