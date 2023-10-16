/*
 *   Copyright (c) 2023 R3BL LLC
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

use std::{
    env,
    sync::atomic::{AtomicI8, Ordering},
};

/// Global variable which can be used to:
/// 1. Override the color support.
/// 2. Memoize the value of the color support result from running [color_support_get].
///
/// This is a global variable because it is used in multiple places in the codebase, and
/// it is really dependent on the environment.
static mut COLOR_SUPPORT_GLOBAL: AtomicI8 = AtomicI8::new(NOT_SET_VALUE);

const NOT_SET_VALUE: i8 = -1;

/// This is the main function that is used to determine whether color is supported. And if
/// so what type of color is supported. If the value has been set using
/// [color_support_override_set], then that value will be returned. Otherwise, the value
/// will be determined by the examining the environment variables.
pub fn detect_color_support() -> ColorSupport {
    match color_support_get() {
        ColorSupport::NotSet => {
            examine_env_vars_to_determine_color_support(Stream::Stdout)
        }
        ColorSupport::Ansi256 => ColorSupport::Ansi256,
        ColorSupport::Truecolor => ColorSupport::Truecolor,
        ColorSupport::Grayscale => ColorSupport::Grayscale,
        ColorSupport::NoColor => ColorSupport::NoColor,
    }
}

/// Override the color support. Regardless of the value of the environment variables the
/// value you set here will be used when you call [detect_color_support].
///
/// ## Testing support
///
/// The [serial_test](https://crates.io/crates/serial_test) crate is used to test this
/// function. In any test in which this function is called, please use the `#[serial]`
/// attribute to annotate that test. Otherwise there will be flakiness in the test results
/// (tests are run in parallel using many threads).
pub fn color_support_override_set(value: ColorSupport) {
    unsafe {
        let value: i8 = ColorSupport::into(value);
        COLOR_SUPPORT_GLOBAL.store(value, Ordering::SeqCst);
    };
}

/// Get the color support. If the value has been set using [color_support_override_set],
/// then that value will be returned. Otherwise, the value will be determined by the
/// examining the environment variables.
fn color_support_get() -> ColorSupport {
    let color_support_global = unsafe { COLOR_SUPPORT_GLOBAL.load(Ordering::SeqCst) };
    ColorSupport::from(color_support_global)
}

/// Determine whether color is supported heuristically. This is based on the environment
/// variables.
fn examine_env_vars_to_determine_color_support(stream: Stream) -> ColorSupport {
    if env_no_color()
        || as_str(&env::var("TERM")) == Ok("dumb")
        || !(is_a_tty(stream)
            || env::var("IGNORE_IS_TERMINAL").map_or(false, |v| v != "0"))
    {
        return ColorSupport::NoColor;
    }

    if env::consts::OS == "macos" {
        if as_str(&env::var("TERM_PROGRAM")) == Ok("Apple_Terminal")
            && env::var("TERM").map(|term| check_256_color(&term)) == Ok(true)
        {
            return ColorSupport::Ansi256;
        }

        if as_str(&env::var("TERM_PROGRAM")) == Ok("iTerm.app")
            || as_str(&env::var("COLORTERM")) == Ok("truecolor")
        {
            return ColorSupport::Truecolor;
        }
    }

    if env::consts::OS == "linux" && as_str(&env::var("COLORTERM")) == Ok("truecolor") {
        return ColorSupport::Truecolor;
    }

    if env::consts::OS == "windows" {
        return ColorSupport::Truecolor;
    }

    if env::var("COLORTERM").is_ok()
        || env::var("TERM").map(|term| check_ansi_color(&term)) == Ok(true)
        || env::var("CLICOLOR").map_or(false, |v| v != "0")
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
    Ansi256,
    Truecolor,
    NoColor,
    Grayscale,
    NotSet,
}

/// These trail implementations allow us to use `ColorSupport` and `i8` interchangeably.
mod convert_between_color_and_i8 {
    impl From<i8> for super::ColorSupport {
        fn from(value: i8) -> Self {
            match value {
                1 => super::ColorSupport::Ansi256,
                2 => super::ColorSupport::Truecolor,
                3 => super::ColorSupport::NoColor,
                4 => super::ColorSupport::Grayscale,
                _ => super::ColorSupport::NotSet,
            }
        }
    }

    impl From<super::ColorSupport> for i8 {
        fn from(value: super::ColorSupport) -> Self {
            match value {
                super::ColorSupport::Ansi256 => 1,
                super::ColorSupport::Truecolor => 2,
                super::ColorSupport::NoColor => 3,
                super::ColorSupport::Grayscale => 4,
                _ => -1,
            }
        }
    }
}

mod helpers {
    use super::*;

    pub fn is_a_tty(stream: Stream) -> bool {
        use is_terminal::*;
        match stream {
            Stream::Stdout => std::io::stdout().is_terminal(),
            Stream::Stderr => std::io::stderr().is_terminal(),
        }
    }

    pub fn check_256_color(term: &str) -> bool {
        term.ends_with("256") || term.ends_with("256color")
    }

    pub fn check_ansi_color(term: &str) -> bool {
        term.starts_with("screen")
            || term.starts_with("xterm")
            || term.starts_with("vt100")
            || term.starts_with("vt220")
            || term.starts_with("rxvt")
            || term.contains("color")
            || term.contains("ansi")
            || term.contains("cygwin")
            || term.contains("linux")
    }

    pub fn env_no_color() -> bool {
        match as_str(&env::var("NO_COLOR")) {
            Ok("0") | Err(_) => false,
            Ok(_) => true,
        }
    }
}
pub use helpers::*;

fn as_str<E>(option: &Result<String, E>) -> Result<&str, &E> {
    match option {
        Ok(inner) => Ok(inner),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use serial_test::serial;

    use super::*;

    #[test]
    #[serial]
    fn cycle_1() {
        color_support_override_set(ColorSupport::Ansi256);
        assert_eq!(color_support_get(), ColorSupport::Ansi256);
    }

    #[test]
    #[serial]
    fn cycle_2() {
        color_support_override_set(ColorSupport::Truecolor);
        assert_eq!(color_support_get(), ColorSupport::Truecolor);
    }

    #[test]
    #[serial]
    fn cycle_3() {
        color_support_override_set(ColorSupport::NoColor);
        assert_eq!(color_support_get(), ColorSupport::NoColor);
    }

    #[test]
    #[serial]
    fn cycle_4() {
        color_support_override_set(ColorSupport::Grayscale);
        assert_eq!(color_support_get(), ColorSupport::Grayscale);
    }

    #[test]
    #[serial]
    fn cycle_5() {
        color_support_override_set(ColorSupport::NotSet);
        assert_eq!(color_support_get(), ColorSupport::NotSet);
    }
}
