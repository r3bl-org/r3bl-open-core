/*
 *   Copyright (c) 2025 R3BL LLC
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

/// A macro to create a [crate::InlineString] (which is allocated and returned) with a
/// specified format. No heap allocation via [String] creation occurs when the `$format`
/// expression is executed.
///
/// # Arguments
///
/// - `$format` - The format to apply to the string storage. This is whatever you would
///   pass to [format!] or [write!].
#[macro_export]
macro_rules! inline_string {
    (
        $($format:tt)*
    ) => {{
        let mut acc = $crate::InlineString::new();
        use std::fmt::Write as _;
        _ = write!(&mut acc, $($format)*);
        acc
    }};
}

#[cfg(test)]
mod inline_string_tests {
    #[test]
    fn test_inline_string() {
        let result = inline_string!("{}, {}", "Hello", "world!");
        assert_eq!(result, "Hello, world!");
    }
}

/// A macro to create a [crate::TinyInlineString] (which is allocated and returned) with a
/// specified format. No heap allocation via [String] creation occurs when the `$format`
/// expression is executed.
///
/// # Arguments
///
/// - `$format` - The format to apply to the char storage. This is whatever you would
///   pass to [format!] or [write!].
#[macro_export]
macro_rules! tiny_inline_string {
    (
        $($format:tt)*
    ) => {{
        let mut acc = $crate::TinyInlineString::new();
        use std::fmt::Write as _;
        _ = write!(&mut acc, $($format)*);
        acc
    }};
}

#[cfg(test)]
mod tiny_inline_string_tests {
    #[test]
    fn test_tiny_inline_string() {
        let result = tiny_inline_string!("{}, {}", "X", "Y");
        assert_eq!(result, "X, Y");
    }
}
