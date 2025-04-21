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

// XMARK: Clever Rust, use of decl macro w/ `tt` to allow any number of arguments.

/// This macro joins a collection of items into a [crate::InlineString] (which is
/// allocated and returned) with a specified delimiter and format. It iterates over the
/// collection, formats each item with the provided format, and joins them with the
/// delimiter. No heap allocation via [String] creation occurs when the `$format`
/// expression is executed.
///
/// # Arguments
///
/// * `from: $collection` - The collection to iterate over.
/// * `each: $item` - The identifier for each item in the collection.
/// * `index: $index` - The identifier for the index of each item in the collection.
/// * `delim: $delim` - The delimiter to insert between items.
/// * `format: $($format:tt)*` - The format to apply to each item. This is whatever you
///   would pass to [format!] or [write!].
///
/// # Example
///
/// ```rust
/// use r3bl_tui::join_with_index;
/// let items = vec!["apple", "banana", "cherry"];
/// let ch = "x";
/// let result = join_with_index!(
///     from: items,
///     each: item,
///     index: index,
///     delim: ", ",
///     format: "{}[{index}]: '{item}'", ch
/// );
/// assert_eq!(result, "x[0]: 'apple', x[1]: 'banana', x[2]: 'cherry'");
/// ```
#[macro_export]
macro_rules! join_with_index {
    (
        from: $collection:expr,
        each: $item:ident,
        index: $index:ident,
        delim: $delim:expr,
        format: $($format:tt)*
    ) => {{
        use std::fmt::Write as _;
        let mut acc = $crate::InlineString::new();
        let mut iter = $collection.iter().enumerate();
        // First item.
        if let Some(($index, $item)) = iter.next() {
            _ = write!(&mut acc, $($format)*);
        }
        // Rest of the items.
        for ($index, $item) in iter {
            _ = write!(&mut acc, "{}", $delim);
            _ = write!(&mut acc, $($format)*);
        }
        acc
    }};
}

#[cfg(test)]
mod tests_join_with_index {
    #[test]
    fn test_many_items() {
        let items = ["apple", "banana", "cherry"];
        let result = join_with_index!(
            from: items,
            each: item,
            index: index,
            delim: ", ",
            format: "[{index}]: '{item}'"
        );
        assert_eq!(result, "[0]: 'apple', [1]: 'banana', [2]: 'cherry'");
    }

    #[test]
    fn test_join_with_index_empty_collection() {
        let items: Vec<&str> = vec![];
        let result = join_with_index!(
            from: items,
            each: item,
            index: index,
            delim: ", ",
            format: "[{index}]: '{item}'"
        );
        assert_eq!(result, "");
    }

    #[test]
    fn test_join_with_index_single_item() {
        let items = ["apple"];
        let result = join_with_index!(
            from: items,
            each: item,
            index: index,
            delim: ", ",
            format: "[{index}]: '{item}'"
        );
        assert_eq!(result, "[0]: 'apple'");
    }

    #[test]
    fn test_join_with_index_two_items() {
        let items = ["apple", "banana"];
        let result = join_with_index!(
            from: items,
            each: item,
            index: index,
            delim: ", ",
            format: "[{index}]: '{item}'"
        );
        assert_eq!(result, "[0]: 'apple', [1]: 'banana'");
    }

    #[test]
    fn test_join_with_index_with_comma() {
        let items = ["apple", "banana", "cherry"];
        let result = join_with_index!(
            from: items,
            each: item,
            index: index,
            delim: ", ",
            format: "[{index}]: '{item}'"
        );
        assert_eq!(result, "[0]: 'apple', [1]: 'banana', [2]: 'cherry'");
    }

    #[test]
    fn test_join_with_index_with_space() {
        let items = ["apple", "banana", "cherry"];
        let result = join_with_index!(
            from: items,
            each: item,
            index: index,
            delim: " ",
            format: "[{index}]: '{item}'"
        );
        assert_eq!(result, "[0]: 'apple' [1]: 'banana' [2]: 'cherry'");
    }

    #[test]
    fn test_join_with_index_with_dash() {
        let items = ["apple", "banana", "cherry"];
        let result = join_with_index!(
            from: items,
            each: item,
            index: index,
            delim: "-",
            format: "[{index}]: '{item}'"
        );
        assert_eq!(result, "[0]: 'apple'-[1]: 'banana'-[2]: 'cherry'");
    }
}

/// A macro to join elements of a collection into a single [crate::InlineString] (which
/// is allocated and returned) with a specified delimiter and format. No heap allocation
/// via [String] creation occurs when the `$format` expression is executed.
///
/// # Arguments
///
/// * `from: $collection` - The collection to iterate over.
/// * `each: $item` - The identifier for each item in the collection.
/// * `delim: $delim` - The delimiter to insert between items.
/// * `format: $($format:tt)*` - The format to apply to each item. This is whatever you
///   would pass to [format!] or [write!].
///
/// # Example
///
/// ```rust
/// use r3bl_tui::join;
/// let vec = vec![1, 2, 3];
/// let ch = "x";
/// let result = join! {
///     from: vec,
///     each: item,
///     delim: ", ",
///     format: "{item}{}", ch
/// };
/// assert_eq!(result, "1x, 2x, 3x");
/// ```
#[macro_export]
macro_rules! join {
    (
        from: $collection:expr,
        each: $item:ident,
        delim: $delim:expr,
        format: $($format:tt)*
    ) => {{
        use std::fmt::Write as _;
        let mut acc = $crate::InlineString::new();
        let mut iter = $collection.iter();
        // First item.
        if let Some($item) = iter.next() {
            _ = write!(&mut acc, $($format)*);
        }
        // Rest of the items.
        for $item in iter {
            _ = write!(&mut acc, "{}", $delim);
            _ = write!(&mut acc, $($format)*);
        }
        acc
    }};
}

#[cfg(test)]
mod tests_join {
    #[test]
    fn test_join() {
        let vec = [1, 2, 3, 4];
        let result = join!(
            from: vec,
            each: item,
            delim: ", ",
            format: "'{item}'"
        );
        assert_eq!(result, "'1', '2', '3', '4'");
    }

    #[test]
    fn test_join_empty_collection() {
        let items: Vec<&str> = vec![];
        let result = join!(from: items, each: item, delim: ", ", format: "{item}");
        assert_eq!(result, "");
        assert_eq!(result, items.join(", "));
    }

    #[test]
    fn test_join_single_item() {
        let items = ["apple"];
        let result = join!(from: items, each: item, delim: ", ", format: "{item}");
        assert_eq!(result, "apple");
        assert_eq!(result, items.join(", "));
    }

    #[test]
    fn test_join_two_items() {
        let items = ["apple", "banana"];
        let result = join!(from: items, each: item, delim: ", ", format: "{item}");
        assert_eq!(result, "apple, banana");
        assert_eq!(result, items.join(", "));
    }

    #[test]
    fn test_join_with_comma() {
        let items = ["apple", "banana", "cherry"];
        let result = join!(from: items, each: item, delim: ", ", format: "{item}");
        assert_eq!(result, "apple, banana, cherry");
        assert_eq!(result, items.join(", "));
    }

    #[test]
    fn test_join_with_space() {
        let items = ["apple", "banana", "cherry"];
        let result = join!(from: items, each: item, delim: " ", format: "{item}");
        assert_eq!(result, "apple banana cherry");
        assert_eq!(result, items.join(" "));
    }

    #[test]
    fn test_join_with_dash() {
        let items = ["apple", "banana", "cherry"];
        let result = join!(from: items, each: item, delim: "-", format: "{item}");
        assert_eq!(result, "apple-banana-cherry");
        assert_eq!(result, items.join("-"));
    }
}
