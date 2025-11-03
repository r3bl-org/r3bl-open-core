// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Core word boundary detection logic.

use unicode_segmentation::UnicodeSegmentation;

/// Returns true if the grapheme cluster is a word boundary (whitespace or punctuation).
///
/// ## Examples
///
/// ```
/// use r3bl_tui::core::graphemes::word_boundaries::is_word_boundary;
///
/// assert_eq!(is_word_boundary(" "), true);   // Space
/// assert_eq!(is_word_boundary("\t"), true);  // Tab
/// assert_eq!(is_word_boundary("-"), true);   // Hyphen
/// assert_eq!(is_word_boundary("."), true);   // Period
/// assert_eq!(is_word_boundary("a"), false);  // Letter
/// assert_eq!(is_word_boundary("5"), false);  // Digit
/// assert_eq!(is_word_boundary("世"), false); // Unicode char
/// ```
#[must_use]
pub fn is_word_boundary(grapheme: &str) -> bool {
    grapheme
        .chars()
        .all(|c| c.is_whitespace() || c.is_ascii_punctuation())
}

/// Returns true if the grapheme cluster is a word character (not a boundary).
///
/// This is the inverse of `is_word_boundary()`.
#[must_use]
pub fn is_word_char(grapheme: &str) -> bool { !is_word_boundary(grapheme) }

/// Finds the start position of the previous word from the cursor position.
///
/// This function moves backward from the cursor, skipping any boundary characters,
/// then skipping word characters until it finds the start of the previous word.
///
/// ## Algorithm
///
/// 1. Start from cursor position (or end of text if cursor is beyond)
/// 2. Skip any boundary characters (whitespace/punctuation) moving backward
/// 3. Skip word characters moving backward
/// 4. Return the position at the start of the word
///
/// ## Edge Cases
///
/// - If cursor is at position 0, returns 0
/// - If text is empty, returns 0
/// - If only boundary characters before cursor, returns 0
///
/// ## Examples
///
/// ```
/// use r3bl_tui::core::graphemes::word_boundaries::find_prev_word_start;
///
/// assert_eq!(find_prev_word_start("hello world", 11), 6);  // "world" → "hello"
/// assert_eq!(find_prev_word_start("hello-world", 11), 6);  // "world" → "hello"
/// assert_eq!(find_prev_word_start("hello  world", 12), 7); // Skip multiple spaces
/// assert_eq!(find_prev_word_start("hello", 5), 0);         // Start of text
/// assert_eq!(find_prev_word_start("", 0), 0);              // Empty string
/// ```
#[must_use]
pub fn find_prev_word_start(text: &str, cursor_grapheme_idx: usize) -> usize {
    let graphemes: Vec<(usize, &str)> = text.grapheme_indices(true).collect();
    let count = graphemes.len();

    if count == 0 || cursor_grapheme_idx == 0 {
        return 0;
    }

    // Start from cursor - 1 (or last grapheme if cursor is beyond end)
    let mut idx = cursor_grapheme_idx.saturating_sub(1).min(count - 1);

    // Skip any boundary characters at/before cursor
    while idx > 0 && is_word_boundary(graphemes[idx].1) {
        idx -= 1;
    }

    // If we're at position 0 and it's a boundary, stay at 0
    if idx == 0 && is_word_boundary(graphemes[0].1) {
        return 0;
    }

    // Now skip word characters to find the start of this word
    while idx > 0 && is_word_char(graphemes[idx].1) {
        idx -= 1;
    }

    // If we stopped on a boundary (and not at position 0), move forward one
    if idx > 0 && is_word_boundary(graphemes[idx].1) {
        idx += 1;
    } else if idx == 0 && is_word_char(graphemes[0].1) {
        // We're at the start and it's a word char, this is the start
        return 0;
    } else if idx == 0 && is_word_boundary(graphemes[0].1) {
        // We're at the start and it's a boundary, move forward
        idx += 1;
    }

    idx
}

/// Finds the end position of the next word from the cursor position.
///
/// This function moves forward from the cursor, skipping any boundary characters,
/// then skipping word characters until it finds the end of the next word.
///
/// ## Algorithm
///
/// 1. Start from cursor position
/// 2. Skip any boundary characters (whitespace/punctuation) moving forward
/// 3. Skip word characters moving forward
/// 4. Return the position after the last word character
///
/// ## Edge Cases
///
/// - If cursor is at end of text, returns text length
/// - If text is empty, returns 0
/// - If only boundary characters after cursor, returns text length
///
/// ## Examples
///
/// ```
/// use r3bl_tui::core::graphemes::word_boundaries::find_next_word_end;
///
/// assert_eq!(find_next_word_end("hello world", 0), 5);   // "hello"
/// assert_eq!(find_next_word_end("hello-world", 0), 5);   // "hello"
/// assert_eq!(find_next_word_end("hello  world", 0), 5);  // Skip to "hello"
/// assert_eq!(find_next_word_end("hello", 0), 5);         // End of text
/// assert_eq!(find_next_word_end("", 0), 0);              // Empty string
/// ```
#[must_use]
pub fn find_next_word_end(text: &str, cursor_grapheme_idx: usize) -> usize {
    let graphemes: Vec<(usize, &str)> = text.grapheme_indices(true).collect();
    let count = graphemes.len();

    if count == 0 {
        return 0;
    }

    if cursor_grapheme_idx >= count {
        return count;
    }

    let mut idx = cursor_grapheme_idx;

    // Skip any boundary characters at/after cursor
    while idx < count && is_word_boundary(graphemes[idx].1) {
        idx += 1;
    }

    // If we reached the end, return count
    if idx >= count {
        return count;
    }

    // Now skip word characters to find the end of this word
    while idx < count && is_word_char(graphemes[idx].1) {
        idx += 1;
    }

    idx
}

/// Finds the start position of the next word from the cursor position.
///
/// This function moves forward from the cursor, skipping any word characters
/// (to finish the current word), then skipping boundary characters, until it
/// finds the start of the next word.
///
/// ## Algorithm
///
/// 1. Start from cursor position
/// 2. Skip any word characters at/after cursor (finish current word)
/// 3. Skip boundary characters (whitespace/punctuation) moving forward
/// 4. Return the position at the start of the next word
///
/// ## Edge Cases
///
/// - If cursor is at end of text, returns text length
/// - If text is empty, returns 0
/// - If only boundary characters after cursor, returns text length
///
/// ## Examples
///
/// ```
/// use r3bl_tui::core::graphemes::word_boundaries::find_next_word_start;
///
/// // "one two three"
/// //  012 456 89ABC (C=12)
/// assert_eq!(find_next_word_start("one two three", 0), 4);   // "one" → "two"
/// assert_eq!(find_next_word_start("one two three", 4), 8);   // "two" → "three"
/// assert_eq!(find_next_word_start("one-two", 0), 4);         // "one" → "two" (skip hyphen)
/// assert_eq!(find_next_word_start("one  two", 0), 5);        // Skip multiple spaces
/// assert_eq!(find_next_word_start("one", 0), 3);             // End of text
/// assert_eq!(find_next_word_start("", 0), 0);                // Empty string
/// ```
#[must_use]
pub fn find_next_word_start(text: &str, cursor_grapheme_idx: usize) -> usize {
    let graphemes: Vec<(usize, &str)> = text.grapheme_indices(true).collect();
    let count = graphemes.len();

    if count == 0 {
        return 0;
    }

    if cursor_grapheme_idx >= count {
        return count;
    }

    let mut idx = cursor_grapheme_idx;

    // Skip any word characters at/after cursor (finish current word)
    while idx < count && is_word_char(graphemes[idx].1) {
        idx += 1;
    }

    // If we reached the end, return count
    if idx >= count {
        return count;
    }

    // Skip any boundary characters to find the start of next word
    while idx < count && is_word_boundary(graphemes[idx].1) {
        idx += 1;
    }

    idx
}

#[cfg(test)]
mod tests {
    use super::*;

    // Boundary detection tests

    #[test]
    fn test_is_word_boundary_whitespace() {
        assert!(is_word_boundary(" "));
        assert!(is_word_boundary("\t"));
        assert!(is_word_boundary("\n"));
        assert!(is_word_boundary("\r"));
    }

    #[test]
    fn test_is_word_boundary_punctuation() {
        assert!(is_word_boundary("."));
        assert!(is_word_boundary(","));
        assert!(is_word_boundary(";"));
        assert!(is_word_boundary(":"));
        assert!(is_word_boundary("!"));
        assert!(is_word_boundary("?"));
        assert!(is_word_boundary("-"));
        assert!(is_word_boundary("_"));
        assert!(is_word_boundary("("));
        assert!(is_word_boundary(")"));
        assert!(is_word_boundary("["));
        assert!(is_word_boundary("]"));
        assert!(is_word_boundary("{"));
        assert!(is_word_boundary("}"));
    }

    #[test]
    fn test_is_word_char() {
        assert!(is_word_char("a"));
        assert!(is_word_char("Z"));
        assert!(is_word_char("0"));
        assert!(is_word_char("9"));
        assert!(is_word_char("世")); // Unicode
        assert!(is_word_char("🌍")); // Emoji
    }

    #[test]
    fn test_is_word_boundary_not_word_chars() {
        assert!(!is_word_boundary("a"));
        assert!(!is_word_boundary("Z"));
        assert!(!is_word_boundary("0"));
        assert!(!is_word_boundary("9"));
        assert!(!is_word_boundary("世"));
        assert!(!is_word_boundary("🌍"));
    }

    // find_prev_word_start tests

    #[test]
    fn test_find_prev_word_start_simple() {
        // "hello world"
        //  01234 56789A (A=10)
        // Cursor at 11 (end), should find start of "world" at 6
        assert_eq!(find_prev_word_start("hello world", 11), 6);
    }

    #[test]
    fn test_find_prev_word_start_with_punctuation() {
        // "hello-world"
        //  01234 56789A
        // Cursor at 11, should find start of "world" at 6
        assert_eq!(find_prev_word_start("hello-world", 11), 6);
    }

    #[test]
    fn test_find_prev_word_start_multiple_spaces() {
        // "hello  world"
        //  01234  6789AB (B=11)
        // Cursor at 12, should skip spaces and find start of "world" at 7
        assert_eq!(find_prev_word_start("hello  world", 12), 7);
    }

    #[test]
    fn test_find_prev_word_start_at_beginning() {
        // "hello"
        //  01234
        // Cursor at 5, should return 0 (start of text)
        assert_eq!(find_prev_word_start("hello", 5), 0);
    }

    #[test]
    fn test_find_prev_word_start_at_position_zero() {
        // Cursor already at 0, should stay at 0
        assert_eq!(find_prev_word_start("hello", 0), 0);
    }

    #[test]
    fn test_find_prev_word_start_empty_string() {
        assert_eq!(find_prev_word_start("", 0), 0);
    }

    #[test]
    fn test_find_prev_word_start_only_spaces() {
        assert_eq!(find_prev_word_start("   ", 3), 0);
    }

    #[test]
    fn test_find_prev_word_start_unicode() {
        // "hello 世界"
        // Cursor after "世界", should find start at position after space
        let text = "hello 世界";
        let graphemes: Vec<_> = text.graphemes(true).collect();
        let cursor = graphemes.len(); // After "世界"
        let result = find_prev_word_start(text, cursor);
        assert_eq!(result, 6); // Start of "世界"
    }

    #[test]
    fn test_find_prev_word_start_emoji() {
        // "hello 🌍"
        let text = "hello 🌍";
        let graphemes: Vec<_> = text.graphemes(true).collect();
        let cursor = graphemes.len();
        let result = find_prev_word_start(text, cursor);
        assert_eq!(result, 6); // Start of "🌍"
    }

    #[test]
    fn test_find_prev_word_start_consecutive_punctuation() {
        // "hello...world"
        //  01234   56789A (A=10)
        // Cursor at 13, should find start of "world" at 8
        assert_eq!(find_prev_word_start("hello...world", 13), 8);
    }

    // find_next_word_end tests

    #[test]
    fn test_find_next_word_end_simple() {
        // "hello world"
        //  01234 56789A
        // Cursor at 0, should find end of "hello" at 5
        assert_eq!(find_next_word_end("hello world", 0), 5);
    }

    #[test]
    fn test_find_next_word_end_with_punctuation() {
        // "hello-world"
        //  01234 56789A
        // Cursor at 0, should find end of "hello" at 5
        assert_eq!(find_next_word_end("hello-world", 0), 5);
    }

    #[test]
    fn test_find_next_word_end_multiple_spaces() {
        // "hello  world"
        //  01234  6789AB
        // Cursor at 0, should skip to end of "hello" at 5
        assert_eq!(find_next_word_end("hello  world", 0), 5);
    }

    #[test]
    fn test_find_next_word_end_from_middle() {
        // "hello world"
        //  01234 56789A
        // Cursor at 6 (start of "world"), should find end at 11
        assert_eq!(find_next_word_end("hello world", 6), 11);
    }

    #[test]
    fn test_find_next_word_end_at_end() {
        // "hello"
        //  01234
        // Cursor at 5 (end), should return 5
        let text = "hello";
        let len = text.graphemes(true).count();
        assert_eq!(find_next_word_end(text, len), len);
    }

    #[test]
    fn test_find_next_word_end_empty_string() {
        assert_eq!(find_next_word_end("", 0), 0);
    }

    #[test]
    fn test_find_next_word_end_only_spaces() {
        let text = "   ";
        let len = text.graphemes(true).count();
        assert_eq!(find_next_word_end(text, 0), len);
    }

    #[test]
    fn test_find_next_word_end_unicode() {
        // "hello 世界"
        // Cursor at 0, should find end of "hello" at 5
        let text = "hello 世界";
        assert_eq!(find_next_word_end(text, 0), 5);
    }

    #[test]
    fn test_find_next_word_end_emoji() {
        // "hello 🌍"
        let text = "hello 🌍";
        assert_eq!(find_next_word_end(text, 0), 5);
    }

    #[test]
    fn test_find_next_word_end_from_space() {
        // "hello world"
        //  01234 56789A
        // Cursor at 5 (the space), should skip space and find end of "world" at 11
        assert_eq!(find_next_word_end("hello world", 5), 11);
    }

    #[test]
    fn test_find_next_word_end_consecutive_punctuation() {
        // "hello...world"
        //  01234   89ABC (C=12)
        // Cursor at 0, should find end of "hello" at 5
        assert_eq!(find_next_word_end("hello...world", 0), 5);
    }

    // find_next_word_start tests

    #[test]
    fn test_find_next_word_start_simple() {
        // "one two three"
        //  012 456 89ABC (C=12)
        // Cursor at 0, should find start of "two" at 4
        assert_eq!(find_next_word_start("one two three", 0), 4);
    }

    #[test]
    fn test_find_next_word_start_from_middle() {
        // "one two three"
        //  012 456 89ABC
        // Cursor at 4 (start of "two"), should find start of "three" at 8
        assert_eq!(find_next_word_start("one two three", 4), 8);
    }

    #[test]
    fn test_find_next_word_start_with_punctuation() {
        // "one-two"
        //  012 456
        // Cursor at 0, should skip hyphen and find start of "two" at 4
        assert_eq!(find_next_word_start("one-two", 0), 4);
    }

    #[test]
    fn test_find_next_word_start_multiple_spaces() {
        // "one  two"
        //  012  567
        // Cursor at 0, should skip multiple spaces and find start of "two" at 5
        assert_eq!(find_next_word_start("one  two", 0), 5);
    }

    #[test]
    fn test_find_next_word_start_at_end() {
        // "one"
        //  012
        // Cursor at 0, no next word, should return end (3)
        assert_eq!(find_next_word_start("one", 0), 3);
    }

    #[test]
    fn test_find_next_word_start_cursor_at_end() {
        // Cursor already at end, should stay at end
        let text = "one";
        let len = text.graphemes(true).count();
        assert_eq!(find_next_word_start(text, len), len);
    }

    #[test]
    fn test_find_next_word_start_empty_string() {
        assert_eq!(find_next_word_start("", 0), 0);
    }

    #[test]
    fn test_find_next_word_start_only_spaces() {
        let text = "   ";
        let len = text.graphemes(true).count();
        assert_eq!(find_next_word_start(text, 0), len);
    }

    #[test]
    fn test_find_next_word_start_from_inside_word() {
        // "hello world"
        //  01234 56789A
        // Cursor at 2 (inside "hello"), should skip to end of "hello" then find "world"
        // at 6
        assert_eq!(find_next_word_start("hello world", 2), 6);
    }

    #[test]
    fn test_find_next_word_start_from_space() {
        // "one two"
        //  012 456
        // Cursor at 3 (the space), should skip space and find start of "two" at 4
        assert_eq!(find_next_word_start("one two", 3), 4);
    }

    #[test]
    fn test_find_next_word_start_unicode() {
        // "hello 世界"
        // Cursor at 0, should find start of "世界" at position 6
        let text = "hello 世界";
        assert_eq!(find_next_word_start(text, 0), 6);
    }

    #[test]
    fn test_find_next_word_start_emoji() {
        // "hello 🌍"
        let text = "hello 🌍";
        assert_eq!(find_next_word_start(text, 0), 6);
    }

    #[test]
    fn test_find_next_word_start_consecutive_punctuation() {
        // "one...two"
        //  012   567
        // Cursor at 0, should skip "one" and "..." and find start of "two" at 6
        assert_eq!(find_next_word_start("one...two", 0), 6);
    }

    // Edge case tests

    #[test]
    fn test_combined_operations() {
        let text = "foo bar-baz qux";
        //          012 456 89A BCD (D=13)

        // From end of "bar": prev should be "foo", next should be "baz"
        let cursor_after_bar = 7;
        assert_eq!(find_prev_word_start(text, cursor_after_bar), 4); // Start of "bar"
        assert_eq!(find_next_word_end(text, cursor_after_bar), 11); // End of "baz"
    }

    #[test]
    fn test_single_word() {
        let text = "hello";
        assert_eq!(find_prev_word_start(text, 5), 0);
        assert_eq!(find_next_word_end(text, 0), 5);
    }

    #[test]
    fn test_word_at_boundaries() {
        let text = " hello ";
        // Start at 1 (beginning of "hello")
        // Previous word from here goes to beginning (0)
        assert_eq!(find_prev_word_start(text, 1), 0);
        // Next word end from 'h' goes to end of "hello" (6)
        assert_eq!(find_next_word_end(text, 1), 6);
    }
}
