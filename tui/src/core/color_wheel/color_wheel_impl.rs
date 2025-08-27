// Copyright (c) 2023-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Main `ColorWheel` implementation.
//!
//! This module contains the primary `ColorWheel` struct and its implementation,
//! providing the main interface for color wheel functionality:
//! - Text colorization with various policies
//! - Gradient generation and management
//! - Support for RGB, ANSI 256, and Lolcat color modes
//! - Iterator interface for color cycling
//!
//! The `ColorWheel` automatically adapts to terminal capabilities and provides
//! a unified interface for all color wheel operations. Previously located
//! in `color_wheel/color_wheel_impl.rs`.
//!
//! # Caching Strategy
//!
//! The `ColorWheel` implements intelligent caching to optimize performance, particularly
//! for repetitive colorization tasks like dialog border rendering (which showed 11.71%
//! CPU usage in flamegraph analysis).
//!
//! ## Cacheable Policies (Deterministic Output)
//!
//! ### `ReuseExistingGradientAndResetIndex`
//! - Always resets the gradient index to 0 before colorizing
//! - Same input → same output (deterministic)
//! - Used by: Dialog borders, UI decorations
//! - Performance: Eliminates redundant computation for repeated strings
//!
//! ### `RegenerateGradientAndIndexBasedOnTextLength`
//! - Generates a gradient sized to text length, starting at index 0
//! - Deterministic for same text
//! - Used by: One-off colorizations where gradient should fit text exactly
//!
//! ## Non-Cacheable Policy
//!
//! ### `ReuseExistingGradientAndIndex` (NOT CACHED)
//! - Maintains gradient index between calls (stateful)
//! - Creates flowing rainbow effects across multiple strings
//! - Same input → different output depending on prior calls
//! - Caching would break the continuous color flow
//!
//! Example of why this can't be cached:
//! ```text
//! // First call: "Hello" uses colors 0-4
//! colorize("Hello") → [H=red, e=orange, l=yellow, l=green, o=blue]
//!
//! // Second call: "World" continues from color 5
//! colorize("World") → [W=purple, o=red, r=orange, l=yellow, d=green]
//!
//! // Third call: "Hello" now starts from color 10!
//! colorize("Hello") → [H=cyan, e=magenta, l=red, l=orange, o=yellow]
//! ```
//!
//! # Hashing Strategy
//!
//! This module uses a dual hashing strategy optimized for different use cases:
//!
//! ## `FxHashMap` for Cache Storage
//! - The cache uses `rustc_hash::FxHashMap` instead of `std::collections::HashMap`
//! - `FxHash` provides 3-5x faster lookups compared to the default `SipHash`
//! - This directly addresses the 38M samples spent in `COLORIZATION_CACHE` operations
//!   shown in flamegraph
//! - `FxHash` is ideal here because:
//!   - Cache keys are trusted internal data, not user input
//!   - No cryptographic security requirements
//!   - Small keys (text + config data) that `FxHash` handles efficiently
//!
//! ## `DefaultHasher` for Config Hashing
//! - `gradient_config_hash` computation continues to use
//!   `std::collections::hash_map::DefaultHasher`
//! - This provides better hash distribution when condensing multiple `ColorWheelConfig`
//!   into a single u64
//! - `DefaultHasher` is retained here because:
//!   - It only runs once per colorization request (not in the hot path)
//!   - Better distribution reduces collision risk for complex config structures
//!   - The one-time cost (~50-100 cycles) is negligible compared to cache operation
//!     savings
//!
//! This dual approach gives us the best of both worlds: maximum speed for frequent cache
//! operations while maintaining hash quality where it matters most.
//!
//! ## Performance Results
//!
//! Flamegraph analysis after `FxHashMap` implementation shows dramatic improvement:
//! - **Before**: 38M samples in `COLORIZATION_CACHE` hash operations
//! - **After**: Only 5M samples in `lolcat_into_string` (including all operations)
//! - **Result**: 87% reduction in ColorWheel-related CPU usage
//!
//! The optimization successfully eliminated the hash operation bottleneck, making
//! `ColorWheel` operations negligible in the overall performance profile.
use std::{collections::hash_map::DefaultHasher,
          hash::{Hash, Hasher},
          sync::LazyLock};

use sizing::VecConfigs;
use smallvec::SmallVec;

use super::{Ansi256GradientIndex, ColorWheelConfig, ColorWheelDirection,
            ColorWheelSpeed, GradientKind, GradientLengthKind, Lolcat, LolcatBuilder,
            Seed,
            color_wheel_config::{defaults::{Defaults, get_default_gradient_stops},
                                 sizing::VecSteps},
            color_wheel_helpers, generate_random_truecolor_gradient,
            generate_truecolor_gradient, get_gradient_array_for};
use crate::{ChUnit, GCStringOwned, GradientGenerationPolicy, RgbValue,
            TextColorizationPolicy, TuiColor, TuiStyle, TuiStyledText, TuiStyledTexts,
            WriteToBuf, ast, ch, glyphs::SPACER_GLYPH as SPACER, tui_color,
            tui_styled_text, u8, usize};

/// These are sized to allow for stack allocation rather than heap allocation. If for some
/// reason these are exceeded, then they will [`smallvec::SmallVec::spilled`] over into
/// the heap.
pub(in crate::core) mod sizing {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    pub type VecConfigs = SmallVec<[ColorWheelConfig; MAX_CONFIGS]>;
    const MAX_CONFIGS: usize = 2;
}

/// Cache for `ColorWheel` colorization to avoid repeated computation.
mod color_wheel_cache {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    const CACHE_SIZE: usize = 1000;

    /// Key for caching `ColorWheel` operations.
    /// Only includes fields that affect the colorization output.
    #[derive(Hash, Clone, PartialEq, Eq, Debug)]
    pub(super) struct ColorWheelCacheKey {
        /// The text to be colorized
        text: String,
        /// Hash of the `ColorWheel` configuration (gradient colors, speed, etc.)
        gradient_config_hash: u64,
        /// The gradient generation policy
        gradient_generation_policy: GradientGenerationPolicy,
        /// The text colorization policy
        text_colorization_policy: TextColorizationPolicy,
        /// Whether a default style is present (we don't hash the style itself)
        has_default_style: bool,
    }

    /// Helper functions for hashing `ColorWheel` configurations.
    ///
    /// We need manual hashing because `ColorWheelConfig` cannot derive `Hash` due to
    /// the `Lolcat` variant containing `Seed` and `SeedDelta` which wrap `f64` values.
    ///
    /// Floating-point numbers don't implement `Hash` because:
    /// - NaN != NaN (violates reflexivity required by Hash/Eq).
    /// - -0.0 == 0.0 but have different bit representations.
    /// - Denormalized numbers can have multiple representations.
    ///
    /// To work around this, we convert f64 to bits for hashing, which gives us
    /// deterministic hashing at the cost of treating -0.0 and 0.0 as different.
    pub(super) mod hashing_helpers {
        #[allow(clippy::wildcard_imports)]
        use super::*;

        /// Hash a single `ColorWheelConfig` into the hasher.
        pub(super) fn hash_color_wheel_config<H: Hasher>(
            config: &ColorWheelConfig,
            hasher: &mut H,
        ) {
            match config {
                ColorWheelConfig::Rgb(color_stops, wheel_speed, gradient_steps) => {
                    0u8.hash(hasher); // discriminant
                    color_stops.hash(hasher);
                    wheel_speed.hash(hasher);
                    gradient_steps.hash(hasher);
                }
                ColorWheelConfig::RgbRandom(speed) => {
                    1u8.hash(hasher); // discriminant
                    speed.hash(hasher);
                }
                ColorWheelConfig::Ansi256(index, speed) => {
                    2u8.hash(hasher); // discriminant
                    index.hash(hasher);
                    speed.hash(hasher);
                }
                ColorWheelConfig::Lolcat(builder) => {
                    3u8.hash(hasher); // discriminant
                    hash_lolcat_builder(builder, hasher);
                }
            }
        }

        /// Hash a `LolcatBuilder`, handling the f64 fields.
        fn hash_lolcat_builder<H: Hasher>(
            builder: &crate::lolcat::LolcatBuilder,
            hasher: &mut H,
        ) {
            builder.color_change_speed.hash(hasher);
            // Convert f64 to bits for deterministic hashing
            builder.seed.0.to_bits().hash(hasher);
            builder.seed_delta.0.to_bits().hash(hasher);
            hash_colorize_strategy(builder.colorization_strategy, hasher);
        }

        /// Hash the Colorize enum discriminant.
        fn hash_colorize_strategy<H: Hasher>(
            strategy: crate::lolcat::Colorize,
            hasher: &mut H,
        ) {
            match strategy {
                crate::lolcat::Colorize::BothBackgroundAndForeground => {
                    0u8.hash(hasher);
                }
                crate::lolcat::Colorize::OnlyForeground => {
                    1u8.hash(hasher);
                }
            }
        }
    }

    impl ColorWheelCacheKey {
        pub(super) fn new(
            text: &str,
            configs: &VecConfigs,
            gradient_generation_policy: GradientGenerationPolicy,
            text_colorization_policy: TextColorizationPolicy,
            maybe_default_style: Option<TuiStyle>,
        ) -> Self {
            // Hash all configs to create a unique identifier for the gradient state
            let mut hasher = DefaultHasher::new();
            for config in configs {
                hashing_helpers::hash_color_wheel_config(config, &mut hasher);
            }
            let gradient_config_hash = hasher.finish();

            Self {
                text: text.to_string(),
                gradient_config_hash,
                gradient_generation_policy,
                text_colorization_policy,
                has_default_style: maybe_default_style.is_some(),
            }
        }
    }

    /// Global cache instance for `ColorWheel` operations.
    ///
    /// This cache stores the results of text colorization operations, significantly
    /// improving performance for repetitive tasks like dialog border rendering.
    /// The cache is thread-safe and uses LRU eviction when full.
    pub(super) static COLORIZATION_CACHE: LazyLock<
        crate::ThreadSafeLruCache<ColorWheelCacheKey, TuiStyledTexts>,
    > = LazyLock::new(|| crate::new_threadsafe_lru_cache(CACHE_SIZE));

    /// Check if a gradient policy is cacheable.
    ///
    /// Only policies that reset the index produce deterministic output:
    /// - `ReuseExistingGradientAndResetIndex`: Always starts from index 0
    /// - `RegenerateGradientAndIndexBasedOnTextLength`: Regenerates and starts from 0
    ///
    /// The `ReuseExistingGradientAndIndex` policy is NOT cacheable because it
    /// maintains state between calls, producing different output for the same input.
    pub(super) fn is_cacheable_policy(policy: GradientGenerationPolicy) -> bool {
        matches!(
            policy,
            GradientGenerationPolicy::ReuseExistingGradientAndResetIndex
                | GradientGenerationPolicy::RegenerateGradientAndIndexBasedOnTextLength
        )
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn compute_hash<T: Hash>(value: &T) -> u64 {
            let mut hasher = DefaultHasher::new();
            value.hash(&mut hasher);
            hasher.finish()
        }

        #[test]
        fn test_cache_key_hash_deterministic() {
            // Test that same inputs produce same hash
            let configs = smallvec::smallvec![crate::ColorWheelConfig::Rgb(
                smallvec::smallvec!["#ff0000".into(), "#00ff00".into()],
                crate::ColorWheelSpeed::Fast,
                crate::u8(100),
            ),];

            let key1 = ColorWheelCacheKey::new(
                "test text",
                &configs,
                GradientGenerationPolicy::ReuseExistingGradientAndResetIndex,
                TextColorizationPolicy::ColorEachCharacter(None),
                None,
            );

            let key2 = ColorWheelCacheKey::new(
                "test text",
                &configs,
                GradientGenerationPolicy::ReuseExistingGradientAndResetIndex,
                TextColorizationPolicy::ColorEachCharacter(None),
                None,
            );

            assert_eq!(compute_hash(&key1), compute_hash(&key2));
            assert_eq!(key1, key2);
        }

        #[test]
        fn test_cache_key_hash_different_for_different_inputs() {
            let configs = smallvec::smallvec![crate::ColorWheelConfig::Rgb(
                smallvec::smallvec!["#ff0000".into()],
                crate::ColorWheelSpeed::Fast,
                crate::u8(100),
            ),];

            let key1 = ColorWheelCacheKey::new(
                "text1",
                &configs,
                GradientGenerationPolicy::ReuseExistingGradientAndResetIndex,
                TextColorizationPolicy::ColorEachCharacter(None),
                None,
            );

            let key2 = ColorWheelCacheKey::new(
                "text2",
                &configs,
                GradientGenerationPolicy::ReuseExistingGradientAndResetIndex,
                TextColorizationPolicy::ColorEachCharacter(None),
                None,
            );

            assert_ne!(compute_hash(&key1), compute_hash(&key2));
            assert_ne!(key1, key2);
        }

        #[test]
        fn test_lolcat_config_hash_handles_f64() {
            use crate::{Seed, SeedDelta,
                        lolcat::{Colorize, LolcatBuilder}};

            // Test that different f64 values produce different hashes
            let config1 = crate::ColorWheelConfig::Lolcat(LolcatBuilder {
                color_change_speed: crate::ColorChangeSpeed::Slow,
                seed: Seed(1.0),
                seed_delta: SeedDelta(0.1),
                colorization_strategy: Colorize::OnlyForeground,
            });

            let config2 = crate::ColorWheelConfig::Lolcat(LolcatBuilder {
                color_change_speed: crate::ColorChangeSpeed::Slow,
                seed: Seed(2.0),
                seed_delta: SeedDelta(0.1),
                colorization_strategy: Colorize::OnlyForeground,
            });

            let mut hasher1 = DefaultHasher::new();
            let mut hasher2 = DefaultHasher::new();

            hashing_helpers::hash_color_wheel_config(&config1, &mut hasher1);
            hashing_helpers::hash_color_wheel_config(&config2, &mut hasher2);

            assert_ne!(hasher1.finish(), hasher2.finish());
        }

        #[test]
        fn test_hash_handles_negative_zero() {
            use crate::{Seed, SeedDelta,
                        lolcat::{Colorize, LolcatBuilder}};

            // Test that -0.0 and 0.0 produce different hashes (as documented)
            let config1 = crate::ColorWheelConfig::Lolcat(LolcatBuilder {
                color_change_speed: crate::ColorChangeSpeed::Slow,
                seed: Seed(0.0),
                seed_delta: SeedDelta(0.1),
                colorization_strategy: Colorize::OnlyForeground,
            });

            let config2 = crate::ColorWheelConfig::Lolcat(LolcatBuilder {
                color_change_speed: crate::ColorChangeSpeed::Slow,
                seed: Seed(-0.0),
                seed_delta: SeedDelta(0.1),
                colorization_strategy: Colorize::OnlyForeground,
            });

            let mut hasher1 = DefaultHasher::new();
            let mut hasher2 = DefaultHasher::new();

            hashing_helpers::hash_color_wheel_config(&config1, &mut hasher1);
            hashing_helpers::hash_color_wheel_config(&config2, &mut hasher2);

            // This is expected behavior - -0.0 and 0.0 have different bit representations
            assert_ne!(hasher1.finish(), hasher2.finish());
        }

        #[test]
        fn test_all_config_variants_hash_differently() {
            use crate::Ansi256GradientIndex;

            let configs = vec![
                crate::ColorWheelConfig::Rgb(
                    smallvec::smallvec!["#ff0000".into()],
                    crate::ColorWheelSpeed::Fast,
                    crate::u8(100),
                ),
                crate::ColorWheelConfig::RgbRandom(crate::ColorWheelSpeed::Medium),
                crate::ColorWheelConfig::Ansi256(
                    Ansi256GradientIndex::LightGreenToLightBlue,
                    crate::ColorWheelSpeed::Slow,
                ),
            ];

            let mut hashes = Vec::new();
            for config in &configs {
                let mut hasher = DefaultHasher::new();
                hashing_helpers::hash_color_wheel_config(config, &mut hasher);
                hashes.push(hasher.finish());
            }

            // All different variants should produce different hashes
            for i in 0..hashes.len() {
                for j in (i + 1)..hashes.len() {
                    assert_ne!(hashes[i], hashes[j]);
                }
            }
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct ColorWheel {
    pub configs: VecConfigs,
    pub gradient_kind: GradientKind,
    pub gradient_length_kind: GradientLengthKind,
    pub index: ChUnit,
    pub index_direction: ColorWheelDirection,
    pub counter: ChUnit,
}

impl Default for ColorWheel {
    fn default() -> Self {
        let mut acc = VecConfigs::new();

        let config_1 = {
            ColorWheelConfig::Rgb(
                get_default_gradient_stops(),
                ColorWheelSpeed::Medium,
                Defaults::Steps as u8,
            )
        };

        let config_2 = ColorWheelConfig::Ansi256(
            Ansi256GradientIndex::MediumGreenToMediumBlue,
            ColorWheelSpeed::Medium,
        );

        acc.push(config_1);
        acc.push(config_2);

        Self::new(acc)
    }
}

impl ColorWheel {
    /// This will lazily create a color wheel. It does not compute the gradient and
    /// memoize it when this function is called.
    ///
    /// 1. The heavy lifting is done when
    ///    [`generate_color_wheel`](ColorWheel::generate_color_wheel) is called.
    /// 2. When you use
    ///    [`colorize_into_styled_texts`](ColorWheel::colorize_into_styled_texts) it will
    ///    also also call this method.
    ///
    /// # Arguments
    /// 1. `configs`: A list of color wheel configs. The order of the configs is
    ///    unimportant. However, at the very least, one Truecolor config & one ANSI 256
    ///    config should be provided. The fallback is always grayscale. See
    ///    [`ColorWheelConfig::narrow_config_based_on_color_support`],
    ///    [`crate::global_color_support::detect`] for more info.
    #[must_use]
    pub fn new(configs: VecConfigs) -> Self {
        Self {
            configs,
            gradient_kind: GradientKind::NotCalculatedYet,
            gradient_length_kind: GradientLengthKind::NotCalculatedYet,
            index: ch(0),
            index_direction: ColorWheelDirection::Forward,
            counter: ch(0),
        }
    }

    /// This method will return the index of the current color in the gradient. If the
    /// color wheel is a lolcat, then the seed * 1000 is returned. If the gradient has
    /// not been computed yet, then 0 is returned.
    #[must_use]
    pub fn get_index(&self) -> ChUnit {
        match self.gradient_kind {
            GradientKind::ColorWheel(_) => self.index,
            GradientKind::Lolcat(lolcat) => lolcat_helper::convert_lolcat_seed_to_index(
                lolcat.color_wheel_control.seed,
            ),
            GradientKind::NotCalculatedYet => ch(0),
        }
    }

    /// This method will return the length of the gradient. This is
    /// [`GradientLengthKind::NotCalculatedYet`] if the gradient has not been computed &
    /// memoized yet via a call to
    /// [`generate_color_wheel`](ColorWheel::generate_color_wheel).
    #[must_use]
    pub fn get_gradient_len(&self) -> GradientLengthKind { self.gradient_length_kind }

    pub fn get_gradient_kind(&mut self) -> &mut GradientKind { &mut self.gradient_kind }

    /// Every time this method is called, it will generate the gradient & memoize it.
    ///
    /// # Arguments
    /// * `steps_override` - If `Some` then the number of steps will be overridden. If
    ///   `None` then the number of steps will be determined by the `ColorWheelConfig`.
    ///
    /// Here's the priority order of how `steps` is determined:
    /// 1. If `steps_override` is `Some` then use that.
    /// 2. If `steps_override` is `None` then use the steps from the `ColorWheelConfig`.
    /// 3. If nothing is found in `ColorWheelConfig` then use `DEFAULT_STEPS`.
    ///
    /// # Errors
    /// If the RGB color is invalid, then this method will panic.
    pub fn generate_color_wheel(
        &mut self,
        maybe_steps_override: Option<u8>,
    ) -> &GradientKind {
        let my_config =
            ColorWheelConfig::narrow_config_based_on_color_support(&self.configs);

        let steps: u8 =
            gradient_generation_helper::determine_steps(maybe_steps_override, &my_config);

        // Generate new gradient and replace the old one.
        // More info: https://github.com/Ogeon/palette/tree/master/palette#gradients
        match &my_config {
            ColorWheelConfig::Lolcat(builder) => {
                gradient_generation_helper::set_lolcat_gradient(self, builder);
            }

            ColorWheelConfig::Rgb(color_stops, _, _) => {
                // Generate new gradient.
                let new_gradient = generate_truecolor_gradient(color_stops, steps);
                gradient_generation_helper::set_gradient_and_length(self, new_gradient);
            }

            ColorWheelConfig::RgbRandom(_) => {
                // Generate new random gradient.
                let new_gradient = generate_random_truecolor_gradient(steps);
                gradient_generation_helper::set_gradient_and_length(self, new_gradient);
            }

            ColorWheelConfig::Ansi256(index, _) => {
                let gradient_vec =
                    gradient_generation_helper::generate_ansi256_gradient(*index);
                gradient_generation_helper::set_gradient_and_length(self, gradient_vec);
            }
        }

        &self.gradient_kind
    }

    /// This method will return the next color in the gradient. It updates the index. When
    /// it reaches the end of the gradient, it will flip direction and go in reverse.
    /// And then flip again when it reaches the start. And so on.
    pub fn next_color(&mut self) -> Option<TuiColor> {
        // Early return if the following can't be found.
        if let GradientKind::NotCalculatedYet = self.gradient_kind {
            return None;
        }

        // Get the gradient.
        let my_config =
            ColorWheelConfig::narrow_config_based_on_color_support(&self.configs);

        // Early return if lolcat.
        if let ColorWheelConfig::Lolcat(_) = &my_config {
            return if let GradientKind::Lolcat(lolcat) = &mut self.gradient_kind {
                Some(lolcat_helper::generate_next_lolcat_color(lolcat))
            } else {
                None
            };
        }

        // Determine if the index should be changed (depending on the speed).
        let (should_change_index, new_counter) =
            color_wheel_navigation::should_update_index(&my_config, self.counter);
        self.counter = new_counter;

        let GradientKind::ColorWheel(gradient) = &mut self.gradient_kind else {
            return None;
        };

        // Actually change the index if it should be changed.
        if should_change_index {
            color_wheel_navigation::update_index_with_direction(
                &mut self.index,
                &mut self.index_direction,
                gradient.len(),
            );
        }

        // Return the color for the correct index.
        color_wheel_navigation::get_color_at_index(gradient, self.index)
    }

    /// This method will reset the index to zero.
    fn reset_index(&mut self) {
        // If this is a lolcat, reset the seed, and early return.
        if let GradientLengthKind::Lolcat(seed) = self.get_gradient_len()
            && let GradientKind::Lolcat(lolcat) = self.get_gradient_kind()
        {
            lolcat.color_wheel_control.seed = seed;
            return;
        }

        // Not a lolcat so reset the index and direction.
        self.index = ch(0);
        self.index_direction = ColorWheelDirection::Forward;
    }

    /// Simplified version of [`ColorWheel::colorize_into_string`] with some defaults.
    /// This method is optimized for repeated calls with the same text (like logging).
    /// It uses caching internally since it always uses the
    /// `ReuseExistingGradientAndResetIndex` policy.
    #[must_use]
    pub fn lolcat_into_string(
        text: &str,
        maybe_default_style: Option<TuiStyle>,
    ) -> String {
        let mut color_wheel = ColorWheel::default();
        let string_gcs: GCStringOwned = text.into();

        // Get cached styled texts using the generalized cache.
        // Since we use ReuseExistingGradientAndResetIndex here, the result
        // will be cached. This is perfect for logging where the same
        // messages are colorized repeatedly.
        let styled_texts = color_wheel.colorize_into_styled_texts(
            &string_gcs,
            GradientGenerationPolicy::ReuseExistingGradientAndResetIndex,
            TextColorizationPolicy::ColorEachCharacter(None),
        );

        // Convert styled texts to string using WriteToBuf for better performance
        let mut buffer = String::new();
        for TuiStyledText { mut style, text } in styled_texts.inner {
            if let Some(default_style) = maybe_default_style {
                style += default_style;
            }
            let ansi_styled_text = ast(text, style);
            // Use WriteToBuf trait for better performance
            let _ = ansi_styled_text.write_to_buf(&mut buffer);
        }

        buffer
    }

    /// See [`ColorWheel::lolcat_into_string`] for an easy to use version of this
    /// function.
    pub fn colorize_into_string(
        &mut self,
        string: &str,
        gradient_generation_policy: GradientGenerationPolicy,
        text_colorization_policy: TextColorizationPolicy,
        maybe_default_style: Option<TuiStyle>,
    ) -> String {
        let string_gcs: GCStringOwned = string.into();
        let spans_in_line = self.colorize_into_styled_texts(
            &string_gcs,
            gradient_generation_policy,
            text_colorization_policy,
        );

        let mut buffer = String::new();

        for TuiStyledText { mut style, text } in spans_in_line.inner {
            if let Some(default_style) = maybe_default_style {
                style += default_style;
            }
            let ansi_styled_text = ast(text, style);
            // Use WriteToBuf trait for better performance
            let _ = ansi_styled_text.write_to_buf(&mut buffer);
        }

        buffer
    }

    /// This method gives you fine grained control over the color wheel. It returns a
    /// gradient-colored string. It respects the [`crate::ColorSupport`]
    /// restrictions for the terminal.
    ///
    /// # Colorization Policy
    ///
    /// - [`GradientGenerationPolicy::RegenerateGradientAndIndexBasedOnTextLength`] - The
    ///   first time this method is called it will generate a gradient w/ the number of
    ///   steps. Subsequent calls will use the same gradient and index **if** the number
    ///   of steps is the same. However, if the number of steps are different, then a new
    ///   gradient will be generated & the index reset.
    ///
    /// - [`GradientGenerationPolicy::ReuseExistingGradientAndResetIndex`] - The first
    ///   time this method is called it will generate a gradient w/ the number of steps.
    ///   Subsequent calls will use the same gradient and the index will be reset to 0.
    ///
    /// - [`GradientGenerationPolicy::ReuseExistingGradientAndIndex`] - The first time
    ///   this method is called it will generate a gradient w/ the number of steps.
    ///   Subsequent calls will use the same gradient and index.
    ///
    /// # Caching
    ///
    /// This method uses intelligent caching for policies that produce deterministic
    /// output:
    /// - `ReuseExistingGradientAndResetIndex`: Cached (always starts from index 0)
    /// - `RegenerateGradientAndIndexBasedOnTextLength`: Cached (deterministic for same
    ///   text)
    /// - `ReuseExistingGradientAndIndex`: NOT cached (stateful, produces different
    ///   output)
    pub fn colorize_into_styled_texts(
        &mut self,
        us: &GCStringOwned,
        gradient_generation_policy: GradientGenerationPolicy,
        text_colorization_policy: TextColorizationPolicy,
    ) -> TuiStyledTexts {
        // Use cache for deterministic policies.
        // The cache check happens inside get_cached_or_compute, which will:
        // 1. Check if the policy is cacheable (reset policies only)
        // 2. If not cacheable, immediately compute without caching
        // 3. If cacheable, check cache and return cached result or compute and cache
        let text = us.string.as_ref();

        // Check if result is cacheable and already cached
        if color_wheel_cache::is_cacheable_policy(gradient_generation_policy) {
            let key = color_wheel_cache::ColorWheelCacheKey::new(
                text,
                &self.configs,
                gradient_generation_policy,
                text_colorization_policy,
                None,
            );

            // Try to get from cache
            let cache = color_wheel_cache::COLORIZATION_CACHE.clone();
            if let Ok(mut cache_guard) = cache.lock()
                && let Some(cached) = cache_guard.get(&key)
            {
                return cached.clone();
            }

            // Not in cache, compute and store
            self.generate_gradient(us, gradient_generation_policy);
            let result = self.generate_styled_texts(text_colorization_policy, us);

            if let Ok(mut cache_guard) = cache.lock() {
                cache_guard.insert(key, result.clone());
            }

            return result;
        }

        // Not cacheable - compute directly
        // This closure is only called if:
        // - Policy is not cacheable (ReuseExistingGradientAndIndex)
        self.generate_gradient(us, gradient_generation_policy);
        self.generate_styled_texts(text_colorization_policy, us)
    }

    fn generate_styled_texts(
        &mut self,
        text_colorization_policy: TextColorizationPolicy,
        us: &GCStringOwned,
    ) -> TuiStyledTexts {
        if ColorWheelConfig::config_contains_bg_lolcat(&self.configs) {
            self.generate_styled_texts_for_lolcat_bg(text_colorization_policy, us)
        } else {
            self.generate_styled_texts_regular(text_colorization_policy, us)
        }
    }

    /// Handle special case for lolcat background mode.
    fn generate_styled_texts_for_lolcat_bg(
        &mut self,
        text_colorization_policy: TextColorizationPolicy,
        us: &GCStringOwned,
    ) -> TuiStyledTexts {
        let mut acc = TuiStyledTexts::default();
        let maybe_style =
            lolcat_bg_helper::extract_style_from_policy(text_colorization_policy);

        // Loop: Colorize each (next) character w/ (next) color.
        for next_seg in us {
            let next_seg_str = next_seg.get_str(us);
            let styled_text =
                self.create_lolcat_bg_styled_text(maybe_style, next_seg_str);
            acc += styled_text;
        }

        acc
    }

    /// Handle regular colorization cases.
    fn generate_styled_texts_regular(
        &mut self,
        text_colorization_policy: TextColorizationPolicy,
        us: &GCStringOwned,
    ) -> TuiStyledTexts {
        match text_colorization_policy {
            TextColorizationPolicy::ColorEachCharacter(maybe_style) => {
                self.colorize_each_character(maybe_style, us)
            }
            TextColorizationPolicy::ColorEachWord(maybe_style) => {
                self.colorize_each_word(maybe_style, us)
            }
        }
    }

    /// Colorize each character with a different color.
    fn colorize_each_character(
        &mut self,
        maybe_style: Option<TuiStyle>,
        us: &GCStringOwned,
    ) -> TuiStyledTexts {
        let mut acc = TuiStyledTexts::default();
        for next_seg in us {
            let next_seg_str = next_seg.get_str(us);
            acc += tui_styled_text!(
                @style: generate_styled_texts_helper::gen_style_fg_color_for(maybe_style, self.next_color()),
                @text: next_seg_str,
            );
        }
        acc
    }

    /// Colorize each word with a different color.
    fn colorize_each_word(
        &mut self,
        maybe_style: Option<TuiStyle>,
        us: &GCStringOwned,
    ) -> TuiStyledTexts {
        let mut acc = TuiStyledTexts::default();
        // More info on peekable: https://stackoverflow.com/a/67872822/2085356
        let mut peekable = us.string.split_ascii_whitespace().peekable();
        while let Some(next_word) = peekable.next() {
            // Loop: Colorize each (next) word w/ (next) color.
            acc += tui_styled_text!(
                @style: generate_styled_texts_helper::gen_style_fg_color_for(maybe_style, self.next_color()),
                @text: next_word,
            );
            if peekable.peek().is_some() {
                acc += tui_styled_text!(
                    @style: TuiStyle::default(),
                    @text: SPACER,
                );
            }
        }
        acc
    }

    /// Create a styled text for lolcat background mode.
    fn create_lolcat_bg_styled_text(
        &mut self,
        maybe_style: Option<TuiStyle>,
        text: &str,
    ) -> TuiStyledText {
        let maybe_next_bg_color = self.next_color();

        if let Some(next_bg_color) = maybe_next_bg_color {
            let maybe_bg_color =
                lolcat_bg_helper::convert_tui_color_to_rgb_tuple(next_bg_color);

            if let Some((bg_red, bg_green, bg_blue)) = maybe_bg_color {
                let (fg_red, fg_green, fg_blue) =
                    color_wheel_helpers::calc_fg_color((bg_red, bg_green, bg_blue));
                tui_styled_text!(
                    @style: generate_styled_texts_helper::gen_style_fg_bg_color_for(
                        maybe_style,
                        Some(tui_color!(fg_red, fg_green, fg_blue)),
                        Some(tui_color!(bg_red, bg_green, bg_blue)),
                    ),
                    @text: text,
                )
            } else {
                tui_styled_text!(
                    @style: generate_styled_texts_helper::gen_style_fg_bg_color_for(maybe_style, None, None,),
                    @text: text,
                )
            }
        } else {
            tui_styled_text!(
                @style: generate_styled_texts_helper::gen_style_fg_bg_color_for(maybe_style, None, None,),
                @text: text,
            )
        }
    }

    fn generate_gradient(
        &mut self,
        us: &GCStringOwned,
        gradient_generation_policy: GradientGenerationPolicy,
    ) {
        match gradient_generation_policy {
            GradientGenerationPolicy::RegenerateGradientAndIndexBasedOnTextLength => {
                let steps = u8(*us.len());

                // Generate a new gradient if one doesn't exist.
                if let GradientLengthKind::NotCalculatedYet = self.get_gradient_len() {
                    self.generate_color_wheel(Some(steps));
                    return;
                }

                // Re-use gradient if possible.
                if let GradientLengthKind::ColorWheel(length) = self.get_gradient_len()
                    && u8(ch(length)) != steps
                {
                    self.generate_color_wheel(Some(steps));
                }

                // ALWAYS reset index for this policy.
                // This ensures deterministic output - same text always produces
                // same colors starting from the beginning of the gradient.
                // This is why this policy is CACHEABLE.
                self.reset_index();
            }

            GradientGenerationPolicy::ReuseExistingGradientAndIndex => {
                if let GradientLengthKind::NotCalculatedYet = self.get_gradient_len() {
                    self.generate_color_wheel(None);
                }
                // NO index reset - maintains state between calls.
                // This means the same text gets different colors each time
                // depending on where we left off in the gradient.
                // This is why this policy is NOT CACHEABLE.
            }

            GradientGenerationPolicy::ReuseExistingGradientAndResetIndex => {
                if let GradientLengthKind::NotCalculatedYet = self.get_gradient_len() {
                    self.generate_color_wheel(None);
                }

                // ALWAYS reset index for this policy.
                // Like RegenerateGradientAndIndexBasedOnTextLength, this
                // ensures deterministic output, making it CACHEABLE.
                self.reset_index();
            }
        }
    }
}

mod generate_styled_texts_helper {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    // Inner function.
    pub fn gen_style_fg_color_for(
        maybe_style: Option<TuiStyle>,
        next_color: Option<TuiColor>,
    ) -> TuiStyle {
        let mut it = TuiStyle {
            color_fg: next_color,
            ..Default::default()
        };
        it += &maybe_style;
        it
    }

    // Inner function.
    pub fn gen_style_fg_bg_color_for(
        maybe_style: Option<TuiStyle>,
        next_fg_color: Option<TuiColor>,
        next_background_color: Option<TuiColor>,
    ) -> TuiStyle {
        let mut it = TuiStyle {
            color_fg: next_fg_color,
            color_bg: next_background_color,
            ..Default::default()
        };
        it += &maybe_style;
        it
    }
}

/// Helper module for color wheel index management and navigation.
mod color_wheel_navigation {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Determine if the color wheel index should be updated based on speed settings.
    pub fn should_update_index(
        config: &ColorWheelConfig,
        counter: ChUnit,
    ) -> (bool, ChUnit) {
        let speed_threshold = match config {
            ColorWheelConfig::Rgb(_, ColorWheelSpeed::Fast, _)
            | ColorWheelConfig::RgbRandom(ColorWheelSpeed::Fast)
            | ColorWheelConfig::Ansi256(_, ColorWheelSpeed::Fast) => {
                ColorWheelSpeed::Fast as u8
            }
            ColorWheelConfig::Rgb(_, ColorWheelSpeed::Medium, _)
            | ColorWheelConfig::RgbRandom(ColorWheelSpeed::Medium)
            | ColorWheelConfig::Ansi256(_, ColorWheelSpeed::Medium) => {
                ColorWheelSpeed::Medium as u8
            }
            ColorWheelConfig::Rgb(_, ColorWheelSpeed::Slow, _)
            | ColorWheelConfig::RgbRandom(ColorWheelSpeed::Slow)
            | ColorWheelConfig::Ansi256(_, ColorWheelSpeed::Slow) => {
                ColorWheelSpeed::Slow as u8
            }
            _ => return (false, counter),
        };

        if counter == ch(speed_threshold) {
            (true, ch(1)) // Reset counter and update index
        } else {
            (false, counter + 1) // Increment counter, don't update index
        }
    }

    /// Update the color wheel index and handle direction changes.
    pub fn update_index_with_direction(
        index: &mut ChUnit,
        direction: &mut ColorWheelDirection,
        gradient_len: usize,
    ) -> Option<TuiColor> {
        match *direction {
            ColorWheelDirection::Forward => {
                *index += 1;

                // Hit the end of the gradient, reverse direction
                if *index == ch(gradient_len) {
                    *direction = ColorWheelDirection::Reverse;
                    *index -= 2;
                }
            }
            ColorWheelDirection::Reverse => {
                *index -= 1;

                // Hit the start of the gradient, forward direction
                if *index == ch(0) {
                    *direction = ColorWheelDirection::Forward;
                }
            }
        }
        None // Color will be retrieved separately
    }

    /// Get color at the current index from the gradient.
    pub fn get_color_at_index(gradient: &VecSteps, index: ChUnit) -> Option<TuiColor> {
        gradient.get(usize(index)).copied()
    }
}

/// Helper module for lolcat-specific operations.
mod lolcat_helper {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Handle lolcat color generation and seed advancement.
    pub fn generate_next_lolcat_color(lolcat: &mut Lolcat) -> TuiColor {
        let new_color = color_wheel_helpers::get_color_tuple(&lolcat.color_wheel_control);
        lolcat.color_wheel_control.seed +=
            Seed::from(lolcat.color_wheel_control.color_change_speed);
        tui_color!(new_color.0, new_color.1, new_color.2)
    }

    /// Convert lolcat seed to [`ChUnit`] for indexing.
    ///
    /// This function converts a Seed value to a `ChUnit` for use as an index.
    /// The implementation has been simplified to use integer operations where possible,
    /// which improves performance and reduces floating point precision issues.
    pub fn convert_lolcat_seed_to_index(seed: Seed) -> ChUnit {
        // Early return for invalid seed values
        if !(*seed).is_finite() || *seed < 0.0 {
            return ch(0);
        }

        // Convert seed to integer directly, using multiplication by 1000
        // to preserve precision of small fractional values
        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        let seed_int = (*seed * 1000.0).round() as u64;

        // Convert to usize safely
        let converted_seed = usize::try_from(seed_int).unwrap_or(0);

        ch(converted_seed)
    }
}

/// Helper module for gradient generation operations.
mod gradient_generation_helper {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Determine the number of steps for gradient generation.
    pub fn determine_steps(
        maybe_steps_override: Option<u8>,
        config: &ColorWheelConfig,
    ) -> u8 {
        match maybe_steps_override {
            // 1. Try use steps from `steps_override`.
            Some(steps_override) => steps_override,
            None => {
                // 2. Try use steps from `ColorWheelConfig`.
                if let ColorWheelConfig::Rgb(_, _, steps_from_config) = config {
                    *steps_from_config
                }
                // 3. Otherwise use the default.
                else {
                    Defaults::Steps as u8
                }
            }
        }
    }

    /// Generate gradient for Ansi256 color configuration.
    pub fn generate_ansi256_gradient(index: Ansi256GradientIndex) -> VecSteps {
        let gradient_array = get_gradient_array_for(index);
        let size_hint = gradient_array.len();
        let mut gradient_vec: VecSteps = VecSteps::with_capacity(size_hint);
        for color_u8 in gradient_array {
            gradient_vec.push(tui_color!(ansi * color_u8));
        }
        gradient_vec
    }

    /// Set the gradient and length based on the generated gradient.
    pub fn set_gradient_and_length(color_wheel: &mut ColorWheel, gradient: VecSteps) {
        color_wheel.gradient_length_kind = GradientLengthKind::ColorWheel(gradient.len());
        color_wheel.gradient_kind = GradientKind::ColorWheel(gradient);
        color_wheel.index = ch(0);
    }

    /// Set up lolcat gradient configuration.
    pub fn set_lolcat_gradient(color_wheel: &mut ColorWheel, builder: &LolcatBuilder) {
        color_wheel.gradient_kind = GradientKind::Lolcat(builder.build());
        color_wheel.index = ch(0);
        color_wheel.gradient_length_kind = GradientLengthKind::Lolcat(builder.seed);
    }
}

/// Helper module for lolcat background functionality.
mod lolcat_bg_helper {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Extract the style from the text colorization policy for lolcat background mode.
    pub fn extract_style_from_policy(
        text_colorization_policy: TextColorizationPolicy,
    ) -> Option<TuiStyle> {
        match text_colorization_policy {
            TextColorizationPolicy::ColorEachCharacter(maybe_style)
            | TextColorizationPolicy::ColorEachWord(maybe_style) => maybe_style,
        }
    }

    /// Convert a [`TuiColor`] to an RGB tuple for use as background color.
    pub fn convert_tui_color_to_rgb_tuple(color: TuiColor) -> Option<(u8, u8, u8)> {
        match color {
            TuiColor::Rgb(RgbValue {
                red: bg_red,
                green: bg_green,
                blue: bg_blue,
            }) => Some((bg_red, bg_green, bg_blue)),
            TuiColor::Ansi(ansi_value) => {
                let rgb_value = RgbValue::from(ansi_value);
                Some((rgb_value.red, rgb_value.green, rgb_value.blue))
            }
            TuiColor::Basic(basic_color) => {
                match RgbValue::try_from_tui_color(TuiColor::Basic(basic_color)) {
                    Ok(RgbValue { red, green, blue }) => Some((red, green, blue)),
                    Err(_) => None,
                }
            }
            TuiColor::Reset => None,
        }
    }
}

#[cfg(test)]
mod tests_color_wheel_rgb {
    use serial_test::serial;

    use super::*;
    use crate::{ColorSupport, assert_eq2, global_color_support,
                tui_style::tui_style_attrib::{Bold, Dim},
                tui_style_attribs};

    #[test]
    fn test_convert_lolcat_seed_to_index() {
        // Test with zero seed
        let seed_zero = Seed(0.0);
        assert_eq2!(
            lolcat_helper::convert_lolcat_seed_to_index(seed_zero),
            ch(0)
        );

        // Test with positive seed
        let seed_positive = Seed(1.5);
        assert_eq2!(
            lolcat_helper::convert_lolcat_seed_to_index(seed_positive),
            ch(1500)
        );

        // Test with small positive seed
        let seed_small = Seed(0.001);
        assert_eq2!(
            lolcat_helper::convert_lolcat_seed_to_index(seed_small),
            ch(1)
        );

        // Test with negative seed (should return 0)
        let seed_negative = Seed(-1.0);
        assert_eq2!(
            lolcat_helper::convert_lolcat_seed_to_index(seed_negative),
            ch(0)
        );

        // Test with NaN seed (should return 0)
        let seed_nan = Seed(f64::NAN);
        assert_eq2!(lolcat_helper::convert_lolcat_seed_to_index(seed_nan), ch(0));

        // Test with very large seed
        let seed_large = Seed(1_000_000.0);
        assert_eq2!(
            lolcat_helper::convert_lolcat_seed_to_index(seed_large),
            ch(1_000_000_000)
        );
    }

    mod test_helper {
        use smallvec::smallvec;

        use super::*;

        pub fn create_color_wheel_rgb() -> ColorWheel {
            let config_1 = ColorWheelConfig::Rgb(
                smallvec::smallvec!["#000000".into(), "#ffffff".into()],
                ColorWheelSpeed::Fast,
                10,
            );
            ColorWheel::new(smallvec![config_1])
        }
    }

    /// This strange test is needed because the color wheel uses a global variable to
    /// determine color support. This test ensures that the global variable is reset
    /// to its original value after each test.
    ///
    /// Additionally, since Rust runs tests in a multi-threaded environment, we need to
    /// ensure that the global variable is reset to its original value before each
    /// test. This is why `test_color_wheel_config_narrowing`,
    /// `test_color_wheel_iterator`, etc. are wrapped in a single test.
    ///
    /// If these two are left as separate tests, then these tests will be flaky.
    #[serial]
    #[test]
    fn test_color_wheel_config_narrowing() {
        let default_color_wheel = ColorWheel::default();
        let configs = &default_color_wheel.configs;

        // Set ColorSupport override to: Ansi 256.
        {
            global_color_support::set_override(ColorSupport::Ansi256);
            let config = ColorWheelConfig::narrow_config_based_on_color_support(configs);
            assert_eq2!(
                config,
                ColorWheelConfig::Ansi256(
                    Ansi256GradientIndex::MediumGreenToMediumBlue,
                    ColorWheelSpeed::Medium,
                ),
            );
            global_color_support::clear_override();
        }

        // Set ColorSupport override to: Truecolor.
        {
            global_color_support::set_override(ColorSupport::Truecolor);
            let config = ColorWheelConfig::narrow_config_based_on_color_support(configs);
            assert_eq2!(
                config,
                ColorWheelConfig::Rgb(
                    get_default_gradient_stops(),
                    ColorWheelSpeed::Medium,
                    Defaults::Steps as u8,
                ),
            );
            global_color_support::clear_override();
        }

        // Set ColorSupport override to: Grayscale.
        {
            global_color_support::set_override(ColorSupport::Grayscale);
            let config = ColorWheelConfig::narrow_config_based_on_color_support(configs);
            assert_eq2!(
                config,
                ColorWheelConfig::Ansi256(
                    Ansi256GradientIndex::GrayscaleMediumGrayToWhite,
                    ColorWheelSpeed::Medium,
                ),
            );
            global_color_support::clear_override();
        }
    }

    /// This strange test is needed because the color wheel uses a global variable to
    /// determine color support. This test ensures that the global variable is reset
    /// to its original value after each test.
    ///
    /// Additionally, since Rust runs tests in a multi-threaded environment, we need to
    /// ensure that the global variable is reset to its original value before each
    /// test. This is why `test_color_wheel_config_narrowing`,
    /// `test_color_wheel_iterator`, etc. are wrapped in a single test.
    ///
    /// If these two are left as separate tests, then these tests will be flaky.
    #[serial]
    #[test]
    fn test_color_wheel_iterator() {
        global_color_support::set_override(ColorSupport::Truecolor);

        let color_wheel = &mut test_helper::create_color_wheel_rgb();

        // Didn't call generate_color_wheel() yet, so it should return the start color.
        assert!(color_wheel.next_color().is_none());

        // Call generate_color_wheel() with 10 steps.
        let gradient_kind = color_wheel.generate_color_wheel(None);
        let GradientKind::ColorWheel(lhs) = gradient_kind else {
            panic!()
        };
        let rhs = &[
            (0, 0, 0),
            (26, 26, 26),
            (51, 51, 51),
            (77, 77, 77),
            (102, 102, 102),
            (128, 128, 128),
            (153, 153, 153),
            (179, 179, 179),
            (204, 204, 204),
            (230, 230, 230),
        ]
        .iter()
        .map(|(r, g, b)| tui_color!(*r, *g, *b))
        .collect::<VecSteps>();
        assert_eq2!(lhs, rhs);

        // Call to next() should return the start_color.
        assert_eq2!(
            // 1st call to next(), index is 0
            color_wheel.next_color().unwrap(),
            tui_color!(0, 0, 0)
        );
        assert_eq2!(
            // 2nd call to next(), index is 0
            color_wheel.next_color().unwrap(),
            tui_color!(0, 0, 0)
        );
        assert_eq2!(
            // 3rd call to next(), index is 1
            color_wheel.next_color().unwrap(),
            tui_color!(26, 26, 26)
        );
        assert_eq2!(
            // # 4th call to next(), index is 1
            color_wheel.next_color().unwrap(),
            tui_color!(26, 26, 26)
        );
        assert_eq2!(
            // # 5th call to next(), index is 2
            color_wheel.next_color().unwrap(),
            tui_color!(51, 51, 51)
        );
        assert_eq2!(
            // # 6th call to next(), index is 2
            color_wheel.next_color().unwrap(),
            tui_color!(51, 51, 51)
        );

        // Advance color wheel to index = 8.
        for _ in 0..13 {
            color_wheel.next_color();
        }

        // Next call to next() which is the 20th call should return the end_color.
        assert_eq2!(color_wheel.next_color().unwrap(), tui_color!(230, 230, 230));

        // Next call to next() should return the end_color - 1.
        assert_eq2!(color_wheel.next_color().unwrap(), tui_color!(204, 204, 204));

        // Reverse color wheel to index = 0.
        for _ in 0..16 {
            color_wheel.next_color();
        }

        // Next call to next() should return the start_color.
        assert_eq2!(color_wheel.next_color().unwrap(), tui_color!(0, 0, 0));

        // Next call to next() should advance the index again to 1.
        assert_eq2!(color_wheel.next_color().unwrap(), tui_color!(26, 26, 26));

        global_color_support::clear_override();
    }

    /// This strange test is needed because the color wheel uses a global variable to
    /// determine color support. This test ensures that the global variable is reset
    /// to its original value after each test.
    ///
    /// Additionally, since Rust runs tests in a multi-threaded environment, we need to
    /// ensure that the global variable is reset to its original value before each
    /// test. This is why `test_color_wheel_config_narrowing`,
    /// `test_color_wheel_iterator`, etc. are wrapped in a single test.
    ///
    /// If these two are left as separate tests, then these tests will be flaky.
    #[serial]
    #[test]
    fn test_colorize_into_styled_texts_color_each_word() {
        let color_wheel_rgb = &mut test_helper::create_color_wheel_rgb();

        global_color_support::set_override(ColorSupport::Truecolor);

        let string = "HELLO WORLD";
        let string_gcs: GCStringOwned = string.into();
        let styled_texts = color_wheel_rgb.colorize_into_styled_texts(
            &string_gcs,
            GradientGenerationPolicy::RegenerateGradientAndIndexBasedOnTextLength,
            TextColorizationPolicy::ColorEachWord(None),
        );
        assert_eq2!(styled_texts.len(), 3);

        // [0]: "HELLO", color_fg: Rgb(0, 0, 0)
        assert_eq2!(styled_texts[0].get_text(), "HELLO");
        assert_eq2!(
            styled_texts[0].get_style().color_fg,
            Some(tui_color!(0, 0, 0))
        );
        assert!(styled_texts[0].get_style().attribs.dim.is_none());
        assert!(styled_texts[0].get_style().attribs.bold.is_none());

        // [1]: " ", color_fg: None
        assert_eq2!(styled_texts[1].get_text(), " ");
        assert_eq2!(styled_texts[1].get_style().color_fg, None);

        // [2]: "WORLD", color_fg: Rgb(0, 0, 0)
        assert_eq2!(styled_texts[2].get_text(), "WORLD");
        assert_eq2!(
            styled_texts[2].get_style().color_fg,
            Some(tui_color!(0, 0, 0))
        );

        global_color_support::clear_override();
    }

    /// This strange test is needed because the color wheel uses a global variable to
    /// determine color support. This test ensures that the global variable is reset
    /// to its original value after each test.
    ///
    /// Additionally, since Rust runs tests in a multi-threaded environment, we need to
    /// ensure that the global variable is reset to its original value before each
    /// test. This is why `test_color_wheel_config_narrowing`,
    /// `test_color_wheel_iterator`, etc. are wrapped in a single test.
    ///
    /// If these two are left as separate tests, then these tests will be flaky.
    #[serial]
    #[test]
    fn test_colorize_to_styled_texts_color_each_character() {
        let color_wheel_rgb = &mut test_helper::create_color_wheel_rgb();

        global_color_support::set_override(ColorSupport::Truecolor);

        let style = TuiStyle {
            attribs: tui_style_attribs(Dim + Bold),
            ..Default::default()
        };

        let string = "HELLO";
        let string_gcs: GCStringOwned = string.into();
        let styled_texts = color_wheel_rgb.colorize_into_styled_texts(
            &string_gcs,
            GradientGenerationPolicy::RegenerateGradientAndIndexBasedOnTextLength,
            TextColorizationPolicy::ColorEachCharacter(Some(style)),
        );
        assert_eq2!(styled_texts.len(), 5);

        // [0]: "H", color_fg: Rgb(0, 0, 0)
        assert_eq2!(styled_texts[0].get_text(), "H");
        assert_eq2!(
            styled_texts[0].get_style().color_fg,
            Some(tui_color!(0, 0, 0))
        );
        assert!(styled_texts[0].get_style().attribs.dim.is_some());
        assert!(styled_texts[0].get_style().attribs.bold.is_some());

        // [1]: "E", color_fg: Rgb(0, 0, 0)
        assert_eq2!(styled_texts[1].get_text(), "E");
        assert_eq2!(
            styled_texts[1].get_style().color_fg,
            Some(tui_color!(0, 0, 0))
        );

        // [2]: "L", color_fg: Rgb(51, 51, 51)
        assert_eq2!(styled_texts[2].get_text(), "L");
        assert_eq2!(
            styled_texts[2].get_style().color_fg,
            Some(tui_color!(51, 51, 51))
        );

        // [3]: "L", color_fg: Rgb(51, 51, 51)
        assert_eq2!(styled_texts[3].get_text(), "L");
        assert_eq2!(
            styled_texts[3].get_style().color_fg,
            Some(tui_color!(51, 51, 51))
        );

        // [4]: "O", color_fg: Rgb(102,102,102)
        assert_eq2!(styled_texts[4].get_text(), "O");
        assert_eq2!(
            styled_texts[4].get_style().color_fg,
            Some(tui_color!(102, 102, 102))
        );

        global_color_support::clear_override();
    }

    /// This strange test is needed because the color wheel uses a global variable to
    /// determine color support. This test ensures that the global variable is reset
    /// to its original value after each test.
    ///
    /// Additionally, since Rust runs tests in a multi-threaded environment, we need to
    /// ensure that the global variable is reset to its original value before each
    /// test. This is why `test_color_wheel_config_narrowing`,
    /// `test_color_wheel_iterator`, etc. are wrapped in a single test.
    ///
    /// If these two are left as separate tests, then these tests will be flaky.
    #[serial]
    #[test]
    fn test_colorize_into_ansi_styled_string_each_character() {
        let color_wheel_rgb = &mut test_helper::create_color_wheel_rgb();

        global_color_support::set_override(ColorSupport::Truecolor);

        let string = "HELLO WORLD";

        let ansi_styled_string = color_wheel_rgb.colorize_into_string(
            string,
            GradientGenerationPolicy::RegenerateGradientAndIndexBasedOnTextLength,
            TextColorizationPolicy::ColorEachCharacter(None),
            None,
        );

        println!("ansi_styled_string: {ansi_styled_string}");
        println!("ansi_styled_string: {ansi_styled_string:?}");

        assert_eq2!(
            ansi_styled_string,
            "\u{1b}[38;2;0;0;0mH\u{1b}[0m\u{1b}[38;2;0;0;0mE\u{1b}[0m\u{1b}[38;2;23;23;23mL\u{1b}[0m\u{1b}[38;2;23;23;23mL\u{1b}[0m\u{1b}[38;2;46;46;46mO\u{1b}[0m\u{1b}[38;2;46;46;46m \u{1b}[0m\u{1b}[38;2;70;70;70mW\u{1b}[0m\u{1b}[38;2;70;70;70mO\u{1b}[0m\u{1b}[38;2;93;93;93mR\u{1b}[0m\u{1b}[38;2;93;93;93mL\u{1b}[0m\u{1b}[38;2;116;116;116mD\u{1b}[0m"
        );

        global_color_support::clear_override();
    }
}

#[cfg(test)]
mod bench {
    extern crate test;
    use test::Bencher;

    use super::*;

    /// Benchmark: `lolcat_into_string` with short ASCII text
    #[bench]
    fn bench_lolcat_into_string_ascii_short(b: &mut Bencher) {
        let text = "Hello, world!";
        b.iter(|| {
            let _result = ColorWheel::lolcat_into_string(text, None);
        });
    }

    /// Benchmark: `lolcat_into_string` with longer ASCII text
    #[bench]
    fn bench_lolcat_into_string_ascii_long(b: &mut Bencher) {
        let text =
            "The quick brown fox jumps over the lazy dog. Lorem ipsum dolor sit amet.";
        b.iter(|| {
            let _result = ColorWheel::lolcat_into_string(text, None);
        });
    }

    /// Benchmark: `lolcat_into_string` with typical log message
    #[bench]
    fn bench_lolcat_into_string_log_message(b: &mut Bencher) {
        let text = "main_event_loop → Startup 🎉";
        b.iter(|| {
            let _result = ColorWheel::lolcat_into_string(text, None);
        });
    }

    /// Benchmark: `lolcat_into_string` with Unicode text
    #[bench]
    fn bench_lolcat_into_string_unicode(b: &mut Bencher) {
        let text = "Hello, 世界! こんにちは";
        b.iter(|| {
            let _result = ColorWheel::lolcat_into_string(text, None);
        });
    }

    /// Benchmark: `lolcat_into_string` with repeated text (cache benefit test)
    #[bench]
    fn bench_lolcat_into_string_repeated(b: &mut Bencher) {
        let text = "AppManager::render_app() ok 🎨";
        b.iter(|| {
            // Simulate repeated calls with same text
            for _ in 0..10 {
                let _result = ColorWheel::lolcat_into_string(text, None);
            }
        });
    }

    /// Benchmark: Instance method `colorize_into_string` for comparison
    #[bench]
    fn bench_colorize_into_string_instance(b: &mut Bencher) {
        let mut color_wheel = ColorWheel::default();
        let text = "Hello, world!";
        b.iter(|| {
            let _result = color_wheel.colorize_into_string(
                text,
                GradientGenerationPolicy::ReuseExistingGradientAndResetIndex,
                TextColorizationPolicy::ColorEachCharacter(None),
                None,
            );
        });
    }

    /// Benchmark: Dialog border top line colorization (typical dialog border)
    #[bench]
    fn bench_dialog_border_top(b: &mut Bencher) {
        let mut color_wheel = ColorWheel::default();
        let border_text = "┌─────────────────────────────────────────┐";
        let border_gcs: GCStringOwned = border_text.into();

        b.iter(|| {
            let _result = color_wheel.colorize_into_styled_texts(
                &border_gcs,
                GradientGenerationPolicy::ReuseExistingGradientAndResetIndex,
                TextColorizationPolicy::ColorEachCharacter(None),
            );
        });
    }

    /// Benchmark: Dialog border side colorization (repeated frequently)
    #[bench]
    fn bench_dialog_border_side(b: &mut Bencher) {
        let mut color_wheel = ColorWheel::default();
        let border_text = "│                                         │";
        let border_gcs: GCStringOwned = border_text.into();

        b.iter(|| {
            let _result = color_wheel.colorize_into_styled_texts(
                &border_gcs,
                GradientGenerationPolicy::ReuseExistingGradientAndResetIndex,
                TextColorizationPolicy::ColorEachCharacter(None),
            );
        });
    }

    /// Benchmark: Dialog border repeated (simulates multiple borders in same render)
    #[bench]
    fn bench_dialog_border_repeated(b: &mut Bencher) {
        let mut color_wheel = ColorWheel::default();
        let top_border = "┌─────────────────────────────────────────┐";
        let side_border = "│                                         │";
        let bottom_border = "└─────────────────────────────────────────┘";

        let top_gcs: GCStringOwned = top_border.into();
        let side_gcs: GCStringOwned = side_border.into();
        let bottom_gcs: GCStringOwned = bottom_border.into();

        b.iter(|| {
            // Simulate rendering a complete dialog border
            let _top = color_wheel.colorize_into_styled_texts(
                &top_gcs,
                GradientGenerationPolicy::ReuseExistingGradientAndResetIndex,
                TextColorizationPolicy::ColorEachCharacter(None),
            );

            // Multiple side borders (typical dialog has many rows)
            for _ in 0..10 {
                let _side = color_wheel.colorize_into_styled_texts(
                    &side_gcs,
                    GradientGenerationPolicy::ReuseExistingGradientAndResetIndex,
                    TextColorizationPolicy::ColorEachCharacter(None),
                );
            }

            let _bottom = color_wheel.colorize_into_styled_texts(
                &bottom_gcs,
                GradientGenerationPolicy::ReuseExistingGradientAndResetIndex,
                TextColorizationPolicy::ColorEachCharacter(None),
            );
        });
    }

    /// Benchmark: `colorize_into_styled_texts` with reset policy (cacheable)
    #[bench]
    fn bench_colorize_into_styled_texts_reset_policy(b: &mut Bencher) {
        let mut color_wheel = ColorWheel::default();
        let text = "Test string for caching";
        let text_gcs: GCStringOwned = text.into();

        b.iter(|| {
            let _result = color_wheel.colorize_into_styled_texts(
                &text_gcs,
                GradientGenerationPolicy::ReuseExistingGradientAndResetIndex,
                TextColorizationPolicy::ColorEachCharacter(None),
            );
        });
    }

    /// Benchmark: `colorize_into_styled_texts` with maintain policy (NOT cacheable)
    #[bench]
    fn bench_colorize_into_styled_texts_maintain_policy(b: &mut Bencher) {
        let mut color_wheel = ColorWheel::default();
        let text = "Test string for no caching";
        let text_gcs: GCStringOwned = text.into();

        b.iter(|| {
            let _result = color_wheel.colorize_into_styled_texts(
                &text_gcs,
                GradientGenerationPolicy::ReuseExistingGradientAndIndex, // NOT cached!
                TextColorizationPolicy::ColorEachCharacter(None),
            );
        });
    }

    /// Benchmark: Hash computation for `ColorWheelCacheKey`
    #[bench]
    fn bench_hash_cache_key_creation(b: &mut Bencher) {
        let configs = smallvec::smallvec![crate::ColorWheelConfig::Rgb(
            smallvec::smallvec!["#ff0000".into(), "#00ff00".into(), "#0000ff".into()],
            crate::ColorWheelSpeed::Fast,
            crate::u8(100),
        ),];

        b.iter(|| {
            let _key = color_wheel_cache::ColorWheelCacheKey::new(
                "Hello, world! This is a test string.",
                &configs,
                GradientGenerationPolicy::ReuseExistingGradientAndResetIndex,
                TextColorizationPolicy::ColorEachCharacter(None),
                None,
            );
        });
    }

    /// Benchmark: Hash computation for Lolcat config (with f64)
    #[bench]
    fn bench_hash_lolcat_config(b: &mut Bencher) {
        use crate::{Seed, SeedDelta,
                    lolcat::{Colorize, LolcatBuilder}};

        let config = crate::ColorWheelConfig::Lolcat(LolcatBuilder {
            color_change_speed: crate::ColorChangeSpeed::Slow,
            seed: Seed(42.0),
            seed_delta: SeedDelta(0.1),
            colorization_strategy: Colorize::OnlyForeground,
        });

        b.iter(|| {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            // Use the same hashing logic as the cache key
            match &config {
                crate::ColorWheelConfig::Lolcat(builder) => {
                    3u8.hash(&mut hasher); // discriminant
                    builder.color_change_speed.hash(&mut hasher);
                    builder.seed.0.to_bits().hash(&mut hasher);
                    builder.seed_delta.0.to_bits().hash(&mut hasher);
                    match builder.colorization_strategy {
                        Colorize::BothBackgroundAndForeground => 0u8.hash(&mut hasher),
                        Colorize::OnlyForeground => 1u8.hash(&mut hasher),
                    }
                }
                _ => panic!("Expected Lolcat config"),
            }
            let _hash = hasher.finish();
        });
    }

    /// Benchmark: Hash computation for multiple configs
    #[bench]
    fn bench_hash_multiple_configs(b: &mut Bencher) {
        use crate::Ansi256GradientIndex;

        let configs: VecConfigs = smallvec::smallvec![
            crate::ColorWheelConfig::Rgb(
                smallvec::smallvec!["#ff0000".into(), "#00ff00".into()],
                crate::ColorWheelSpeed::Fast,
                crate::u8(100),
            ),
            crate::ColorWheelConfig::Ansi256(
                Ansi256GradientIndex::LightGreenToLightBlue,
                crate::ColorWheelSpeed::Medium,
            ),
        ];

        b.iter(|| {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            for config in &configs {
                // Hash each config manually to test performance
                match config {
                    crate::ColorWheelConfig::Rgb(stops, speed, steps) => {
                        0u8.hash(&mut hasher);
                        stops.hash(&mut hasher);
                        speed.hash(&mut hasher);
                        steps.hash(&mut hasher);
                    }
                    crate::ColorWheelConfig::Ansi256(index, speed) => {
                        2u8.hash(&mut hasher);
                        index.hash(&mut hasher);
                        speed.hash(&mut hasher);
                    }
                    _ => {}
                }
            }
            let _hash = hasher.finish();
        });
    }

    /// Benchmark: `FxHashMap` vs `HashMap` - Cache insertion performance
    ///
    /// Real-world flamegraph results:
    /// - Before `FxHashMap`: 38M samples in `COLORIZATION_CACHE` hash operations
    /// - After `FxHashMap`: Only 5M samples in `lolcat_into_string` (all operations)
    /// - Performance improvement: 87% reduction in `ColorWheel` CPU usage
    #[bench]
    fn bench_fxhashmap_vs_hashmap_insert(b: &mut Bencher) {
        use std::collections::HashMap;

        use rustc_hash::{FxBuildHasher, FxHashMap};

        // Create test keys
        let mut keys = Vec::new();
        for i in 0..100 {
            let key = color_wheel_cache::ColorWheelCacheKey::new(
                &format!("Test string number {i}"),
                &VecConfigs::new(),
                GradientGenerationPolicy::ReuseExistingGradientAndResetIndex,
                TextColorizationPolicy::ColorEachCharacter(None),
                None,
            );
            keys.push(key);
        }

        // Test FxHashMap
        let fx_time = {
            let start = std::time::Instant::now();
            b.iter(|| {
                let mut map: FxHashMap<
                    color_wheel_cache::ColorWheelCacheKey,
                    TuiStyledTexts,
                > = FxHashMap::with_capacity_and_hasher(100, FxBuildHasher);
                for key in &keys {
                    map.insert(key.clone(), TuiStyledTexts::default());
                }
                test::black_box(map)
            });
            start.elapsed()
        };

        // Test standard HashMap
        let hash_time = {
            let start = std::time::Instant::now();
            b.iter(|| {
                let mut map: HashMap<
                    color_wheel_cache::ColorWheelCacheKey,
                    TuiStyledTexts,
                > = HashMap::with_capacity(100);
                for key in &keys {
                    map.insert(key.clone(), TuiStyledTexts::default());
                }
                test::black_box(map)
            });
            start.elapsed()
        };

        // The benchmark framework will show the actual performance
        // This is just to ensure both paths are tested
        test::black_box((fx_time, hash_time));
    }

    /// Benchmark: `LruCache` lookup performance
    #[bench]
    fn bench_lru_cache_lookup(b: &mut Bencher) {
        // Pre-populate cache
        let mut cache: crate::LruCache<
            color_wheel_cache::ColorWheelCacheKey,
            TuiStyledTexts,
        > = crate::LruCache::new(1000);

        let mut keys = Vec::new();
        for i in 0..1000 {
            let key = color_wheel_cache::ColorWheelCacheKey::new(
                &format!("Dialog border string {}", i % 10), /* Simulate repeated
                                                              * patterns */
                &VecConfigs::new(),
                GradientGenerationPolicy::ReuseExistingGradientAndResetIndex,
                TextColorizationPolicy::ColorEachCharacter(None),
                None,
            );
            cache.insert(key.clone(), TuiStyledTexts::default());
            if i < 100 {
                keys.push(key); // Keep some keys for lookup
            }
        }

        b.iter(|| {
            let mut hits = 0;
            for key in &keys {
                if cache.contains_key(key) {
                    hits += 1;
                }
            }
            test::black_box(hits)
        });
    }

    /// Benchmark: Cache hit rate for dialog borders
    #[bench]
    fn bench_cache_dialog_border_patterns(b: &mut Bencher) {
        let mut color_wheel = ColorWheel::default();

        // Common dialog border patterns
        let border_patterns = vec![
            "┌─────────────────────────────────────────┐",
            "│                                         │",
            "├─────────────────────────────────────────┤",
            "└─────────────────────────────────────────┘",
        ];

        b.iter(|| {
            // Simulate multiple dialog renders
            for _ in 0..10 {
                for pattern in &border_patterns {
                    let gcs: GCStringOwned = pattern.into();
                    let _result = color_wheel.colorize_into_styled_texts(
                        &gcs,
                        GradientGenerationPolicy::ReuseExistingGradientAndResetIndex,
                        TextColorizationPolicy::ColorEachCharacter(None),
                    );
                }
            }
        });
    }
}
