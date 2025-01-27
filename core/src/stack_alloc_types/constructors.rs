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

/// A macro to create a [crate::StringStorage] (which is allocated and returned) with a
/// specified format. No heap allocation via [String] creation occurs when the `$format`
/// expression is executed.
///
/// # Arguments
///
/// - `$format` - The format to apply to the string storage. This is whatever you would
///   pass to [format!] or [write!].
#[macro_export]
macro_rules! string_storage {
    (
        $($format:tt)*
    ) => {{
        let mut acc = $crate::StringStorage::new();
        use std::fmt::Write as _;
        _ = write!(&mut acc, $($format)*);
        acc
    }};
}

#[cfg(test)]
mod string_storage_tests {
    #[test]
    fn test_string_storage() {
        let result = string_storage!("{}, {}", "Hello", "world!");
        assert_eq!(result, "Hello, world!");
    }
}

/// A macro to create a [crate::CharStorage] (which is allocated and returned) with a
/// specified format. No heap allocation via [String] creation occurs when the `$format`
/// expression is executed.
///
/// # Arguments
///
/// - `$format` - The format to apply to the char storage. This is whatever you would
///   pass to [format!] or [write!].
#[macro_export]
macro_rules! char_storage {
    (
        $($format:tt)*
    ) => {{
        let mut acc = $crate::CharStorage::new();
        use std::fmt::Write as _;
        _ = write!(&mut acc, $($format)*);
        acc
    }};
}

#[cfg(test)]
mod char_storage_tests {
    #[test]
    fn test_char_storage() {
        let result = char_storage!("{}, {}", "X", "Y");
        assert_eq!(result, "X, Y");
    }
}
