// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Tests for character encoding support - UTF-8, emojis, and wide characters.

use vte::Perform;

use super::tests_fixtures::*;
use crate::{AnsiToOfsBufPerformer, offscreen_buffer::ofs_buf_test_fixtures::*};

#[test]
fn test_utf8_characters() {
    let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

    // Process UTF-8 characters including emojis
    let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf);

    // Print various UTF-8 characters
    performer.print('H');
    performer.print('é'); // Latin character with accent
    performer.print('中'); // Chinese character
    performer.print('🦀'); // Emoji (Rust crab)
    performer.print('!');

    // Verify all UTF-8 characters are in the buffer
    assert_plain_char_at(&ofs_buf, 0, 0, 'H');
    assert_plain_char_at(&ofs_buf, 0, 1, 'é');
    assert_plain_char_at(&ofs_buf, 0, 2, '中');
    assert_plain_char_at(&ofs_buf, 0, 3, '🦀');
    assert_plain_char_at(&ofs_buf, 0, 4, '!');

    // Verify rest of line is empty
    for col in 5..10 {
        assert_empty_at(&ofs_buf, 0, col);
    }

    // Verify the rest of the buffer is empty
    for row in 1..10 {
        for col in 0..10 {
            assert_empty_at(&ofs_buf, row, col);
        }
    }
}
