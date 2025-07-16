/*
 *   Copyright (c) 2023-2025 R3BL LLC
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

use std::{env,
          sync::atomic::{AtomicI8, Ordering}};

/// # Terminal Color Support Detection with Performance Optimization
///
/// This module provides efficient color support detection for terminal applications with
/// critical performance optimizations to prevent render loop bottlenecks.
///
/// ## Performance Critical Implementation
///
/// **CRITICAL**: This implementation includes memoization to prevent a severe performance
/// bottleneck identified through flamegraph analysis. Without caching, color support
/// detection can consume ~24% of execution time in the main event loop.
///
/// ### The Problem (Before Optimization)
/// - `examine_env_vars_to_determine_color_support()` was called thousands of times per
///   render
/// - Each call performed expensive environment variable lookups
/// - Flamegraph analysis showed 24% of CPU time spent in color detection
/// - Caused significant editor lag during typing/editing operations
///
/// ### The Solution (Current Implementation)
/// - Added `COLOR_SUPPORT_CACHED` static variable for memoization
/// - Detection runs once and caches result for subsequent calls
/// - Provides ~24% reduction in execution time for editor operations
/// - Maintains thread-safety with atomic operations
///
/// ## Usage Patterns
///
/// This module supports two primary use cases:
/// 1. **Override color support** - For testing or user preferences
/// 2. **Cached detection** - For production performance (default behavior)
///
/// ### Correct Usage (Performance Optimized)
///
/// ```rust
/// use r3bl_tui::{global_color_support, ColorSupport};
///
/// // ✅ CORRECT: Use the cached detect() function
/// let color_support = global_color_support::detect();
///
/// // ✅ CORRECT: For testing, use overrides
/// global_color_support::set_override(ColorSupport::NoColor);
/// let overridden = global_color_support::detect(); // Returns NoColor
/// global_color_support::clear_override();
/// ```
///
/// ### Incorrect Usage (Performance Killer)
///
/// ```rust
/// use r3bl_tui::{examine_env_vars_to_determine_color_support, Stream};
///
/// // ❌ WRONG: Direct calls bypass caching and kill performance
/// let color_support = examine_env_vars_to_determine_color_support(Stream::Stdout);
/// ```
///
/// ## Global Variables
///
/// Two global atomic variables manage the color detection state:
/// - `COLOR_SUPPORT_GLOBAL`: Explicit override values (highest priority)
/// - `COLOR_SUPPORT_CACHED`: Memoized detection results (performance optimization)
pub mod global_color_support {
    use super::{examine_env_vars_to_determine_color_support, AtomicI8, ColorSupport,
                Ordering, Stream};

    /// Global override for color support detection.
    ///
    /// This variable stores an explicit override value that takes precedence over all
    /// automatic color detection logic. When set via [`set_override()`], the [`detect()`]
    /// function will always return this value instead of examining environment variables
    /// or using cached detection results.
    ///
    /// # Usage Flow in [`detect()`]
    /// 1. **First priority**: Check this override value
    /// 2. If override is set, return it immediately (skip cache and detection)
    /// 3. If not set, proceed to check [`COLOR_SUPPORT_CACHED`]
    ///
    /// # Use Cases
    /// - Testing: Force specific color support levels for unit tests
    /// - User preference: Allow applications to override automatic detection
    /// - Debugging: Temporarily disable colors regardless of terminal capabilities
    ///
    /// # Thread Safety
    /// Uses `AtomicI8` with `Release`/`Acquire` ordering to ensure thread-safe access
    /// across the application.
    static mut COLOR_SUPPORT_GLOBAL: AtomicI8 = AtomicI8::new(NOT_SET_VALUE);

    /// Cached result of automatic color support detection.
    ///
    /// This variable stores the memoized result from
    /// [`examine_env_vars_to_determine_color_support()`] to avoid repeatedly checking
    /// environment variables and terminal capabilities on every call to [`detect()`].
    ///
    /// # Usage Flow in [`detect()`]
    /// 1. First priority: Check [`COLOR_SUPPORT_GLOBAL`] for overrides
    /// 2. **Second priority**: Check this cached value
    /// 3. If cache hit, return the cached result immediately
    /// 4. If cache miss, run detection, store result here, then return it
    ///
    /// # Caching Behavior
    /// - Initially set to [`NOT_SET_VALUE`] (-1)
    /// - Populated on first call to [`detect()`] when no override is set
    /// - Remains valid until explicitly cleared via [`clear_cache()`]
    /// - Can be manually set via [`set_cached()`] for testing purposes
    ///
    /// # Performance Benefits
    /// Eliminates expensive environment variable lookups and terminal capability
    /// checks on subsequent calls, providing O(1) color support detection after
    /// the initial detection run.
    static mut COLOR_SUPPORT_CACHED: AtomicI8 = AtomicI8::new(NOT_SET_VALUE);

    const NOT_SET_VALUE: i8 = -1;

    /// This is the main function that is used to determine whether color is supported.
    /// And if so what type of color is supported.
    ///
    /// ## Performance-Critical Implementation
    ///
    /// This function implements a three-tier detection strategy optimized for
    /// performance:
    ///
    /// 1. **Override Check**: If [`set_override`] was called, return that value
    ///    immediately
    /// 2. **Cache Check**: If detection was previously run, return cached result (O(1))
    /// 3. **Detection**: Only run expensive environment detection on cache miss
    ///
    /// ## Why Caching is Critical
    ///
    /// Without caching, this function was identified as a major performance bottleneck:
    /// - Called thousands of times during editor operations
    /// - Each call performed expensive environment variable lookups
    /// - Consumed ~24% of total execution time in flamegraph analysis
    /// - Caused noticeable lag during typing and editing
    ///
    /// With caching, detection runs once per application lifetime, providing dramatic
    /// performance improvements for interactive applications.
    ///
    /// ## Thread Safety
    ///
    /// Uses atomic operations with `Acquire`/`Release` ordering for thread-safe access
    /// across multiple threads without requiring external synchronization.
    #[must_use]
    pub fn detect() -> ColorSupport {
        // First check for explicit override
        match try_get_override() {
            Ok(it) => it,
            Err(()) => {
                // Check if we've already cached the detection result
                if let Ok(cached) = try_get_cached() { cached } else {
                    // Not cached yet, so detect once and cache the result
                    let detected =
                        examine_env_vars_to_determine_color_support(Stream::Stdout);
                    set_cached(detected);
                    detected
                }
            }
        }
    }

    /// Override the color support. Regardless of the value of the environment variables
    /// the value you set here will be used when you call [`detect()`].
    ///
    /// # Testing support
    ///
    /// The [serial_test](https://crates.io/crates/serial_test) crate is used to test this
    /// function. In any test in which this function is called, please use the `#[serial]`
    /// attribute to annotate that test. Otherwise there will be flakiness in the test
    /// results (tests are run in parallel using many threads).
    #[allow(clippy::result_unit_err, static_mut_refs)]
    pub fn set_override(value: ColorSupport) {
        let it = i8::from(value);
        unsafe { COLOR_SUPPORT_GLOBAL.store(it, Ordering::Release) }
    }

    #[allow(static_mut_refs)]
    pub fn clear_override() {
        unsafe { COLOR_SUPPORT_GLOBAL.store(NOT_SET_VALUE, Ordering::Release) };
    }

    /// Clear the cached color support detection result, forcing re-detection on next
    /// call. This is useful for testing or when environment might have changed.
    #[allow(static_mut_refs)]
    pub fn clear_cache() {
        unsafe { COLOR_SUPPORT_CACHED.store(NOT_SET_VALUE, Ordering::Release) };
    }

    /// Get the cached color support detection result.
    /// - If detection has been run and cached, that value will be returned.
    /// - Otherwise, an error will be returned.
    #[allow(clippy::result_unit_err, static_mut_refs)]
    pub fn try_get_cached() -> Result<ColorSupport, ()> {
        let it = unsafe { COLOR_SUPPORT_CACHED.load(Ordering::Acquire) };
        ColorSupport::try_from(it)
    }

    /// Set the cached color support detection result.
    #[allow(static_mut_refs)]
    pub fn set_cached(value: ColorSupport) {
        let it = i8::from(value);
        unsafe { COLOR_SUPPORT_CACHED.store(it, Ordering::Release) };
    }

    /// Get the color support override value.
    /// - If the value has been set using [`crate::global_color_support::set_override`],
    ///   then that value will be returned.
    /// - Otherwise, an error will be returned.
    #[allow(clippy::result_unit_err, static_mut_refs)]
    pub fn try_get_override() -> Result<ColorSupport, ()> {
        let it = unsafe { COLOR_SUPPORT_GLOBAL.load(Ordering::Acquire) };
        ColorSupport::try_from(it)
    }
}

/// Determine whether color is supported heuristically. This is based on the environment
/// variables.
///
/// ## Performance Warning
///
/// **This function is expensive and should not be called repeatedly!**
///
/// This function performs multiple environment variable lookups (`env::var()` calls)
/// which involve system calls and are computationally expensive:
///
/// - `NO_COLOR` - Check for color disabling
/// - `TERM` - Terminal type detection
/// - `TERM_PROGRAM` - Specific terminal application detection (macOS)
/// - `COLORTERM` - Modern color support indication
/// - `CLICOLOR` - Legacy color support flag
/// - `IGNORE_IS_TERMINAL` - Override for non-TTY environments
///
/// When called thousands of times per render (as was happening before caching),
/// this function consumed ~24% of total execution time in flamegraph analysis.
///
/// ## Caching Strategy
///
/// This function should only be called through [`global_color_support::detect()`]
/// which implements proper memoization. Direct calls to this function bypass
/// the performance optimization and should be avoided in production code.
///
/// ## Detection Logic
///
/// The function implements a comprehensive heuristic strategy:
/// 1. Check for explicit color disabling (`NO_COLOR`, `TERM=dumb`)
/// 2. Verify TTY capability (unless overridden)
/// 3. Apply platform-specific detection logic (macOS, Linux, Windows)
/// 4. Fallback to generic environment variable checks
#[must_use]
pub fn examine_env_vars_to_determine_color_support(stream: Stream) -> ColorSupport {
    if helpers::env_no_color()
        || env::var("TERM").is_ok_and(|v| v == "dumb")
        || !(helpers::is_a_tty(stream)
            || env::var("IGNORE_IS_TERMINAL").is_ok_and(|v| v != "0"))
    {
        return ColorSupport::NoColor;
    }

    if env::consts::OS == "macos" {
        if env::var("TERM_PROGRAM").is_ok_and(|v| v == "Apple_Terminal")
            && env::var("TERM").is_ok_and(|term| helpers::check_256_color(&term))
        {
            return ColorSupport::Ansi256;
        }

        if env::var("TERM_PROGRAM").is_ok_and(|v| v == "iTerm.app")
            || env::var("COLORTERM").is_ok_and(|v| v == "truecolor")
        {
            return ColorSupport::Truecolor;
        }
    }

    if env::consts::OS == "linux" && env::var("COLORTERM").is_ok_and(|v| v == "truecolor")
    {
        return ColorSupport::Truecolor;
    }

    if env::consts::OS == "windows" {
        return ColorSupport::Truecolor;
    }

    if env::var("COLORTERM").is_ok()
        || env::var("TERM").is_ok_and(|term| helpers::check_ansi_color(&term))
        || env::var("CLICOLOR").is_ok_and(|v| v != "0")
        || is_ci::uncached()
    {
        return ColorSupport::Truecolor;
    }

    ColorSupport::NoColor
}

/// The stream to check for color support.
#[derive(Clone, Copy, Debug)]
pub enum Stream {
    Stdout,
    Stderr,
}

/// The result of the color support check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorSupport {
    Truecolor,
    Ansi256,
    Grayscale,
    NoColor,
}

/// These trail implementations allow us to use `ColorSupport` and `i8` interchangeably.
mod convert_between_color_and_i8 {
    impl TryFrom<i8> for super::ColorSupport {
        type Error = ();

        #[rustfmt::skip]
        fn try_from(value: i8) -> Result<Self, Self::Error> {
            match value {
                1 => Ok(super::ColorSupport::Ansi256),
                2 => Ok(super::ColorSupport::Truecolor),
                3 => Ok(super::ColorSupport::NoColor),
                4 => Ok(super::ColorSupport::Grayscale),
                _ => Err(()),
            }
        }
    }

    impl From<super::ColorSupport> for i8 {
        #[rustfmt::skip]
        fn from(value: super::ColorSupport) -> Self {
            match value {
                super::ColorSupport::Ansi256   => 1,
                super::ColorSupport::Truecolor => 2,
                super::ColorSupport::NoColor   => 3,
                super::ColorSupport::Grayscale => 4,
            }
        }
    }
}

mod helpers {
    use super::{as_str, env, Stream};

    #[must_use]
    pub fn is_a_tty(stream: Stream) -> bool {
        use std::io::IsTerminal;
        match stream {
            Stream::Stdout => std::io::stdout().is_terminal(),
            Stream::Stderr => std::io::stderr().is_terminal(),
        }
    }

    #[must_use]
    pub fn check_256_color(term: &str) -> bool {
        term.ends_with("256") || term.ends_with("256color")
    }

    #[must_use]
    pub fn check_ansi_color(term: &str) -> bool {
        term.starts_with("screen")
            || term.starts_with("vscode")
            || term.starts_with("xterm")
            || term.starts_with("vt100")
            || term.starts_with("vt220")
            || term.starts_with("rxvt")
            || term.contains("color")
            || term.contains("ansi")
            || term.contains("cygwin")
            || term.contains("linux")
    }

    #[must_use]
    pub fn env_no_color() -> bool {
        match as_str(&env::var("NO_COLOR")) {
            Ok("0") | Err(_) => false,
            Ok(_) => true,
        }
    }
}

fn as_str<E>(option: &Result<String, E>) -> Result<&str, &E> {
    match option {
        Ok(inner) => Ok(inner),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    //! Tests for color support detection with performance optimizations.
    //!
    //! These tests verify both the correctness of color detection and the
    //! caching behavior that prevents performance bottlenecks in the main
    //! event loop. The `#[serial]` annotations ensure thread-safe testing
    //! of global state.
    use serial_test::serial;

    use super::*;

    #[test]
    #[serial]
    fn cycle_1() {
        global_color_support::set_override(ColorSupport::Ansi256);
        assert_eq!(
            global_color_support::try_get_override(),
            Ok(ColorSupport::Ansi256)
        );
    }

    #[test]
    #[serial]
    fn cycle_2() {
        global_color_support::set_override(ColorSupport::Truecolor);
        assert_eq!(
            global_color_support::try_get_override(),
            Ok(ColorSupport::Truecolor)
        );
    }

    #[test]
    #[serial]
    fn cycle_3() {
        global_color_support::set_override(ColorSupport::NoColor);
        assert_eq!(
            global_color_support::try_get_override(),
            Ok(ColorSupport::NoColor)
        );
    }

    #[test]
    #[serial]
    fn cycle_4() {
        global_color_support::set_override(ColorSupport::Grayscale);
        assert_eq!(
            global_color_support::try_get_override(),
            Ok(ColorSupport::Grayscale)
        );
    }

    #[test]
    #[serial]
    fn test_caching_behavior() {
        // Clear any existing state
        global_color_support::clear_override();
        global_color_support::clear_cache();

        // First call should detect and cache
        let first_result = global_color_support::detect();

        // Verify that cache now has a value
        assert_eq!(global_color_support::try_get_cached(), Ok(first_result));

        // Second call should return the same cached result
        let second_result = global_color_support::detect();
        assert_eq!(first_result, second_result);

        // Clear cache and verify it's cleared
        global_color_support::clear_cache();
        assert!(global_color_support::try_get_cached().is_err());
    }

    #[test]
    #[serial]
    fn cycle_5() {
        global_color_support::clear_override();
        assert_eq!(global_color_support::try_get_override(), Err(()));
    }
}
