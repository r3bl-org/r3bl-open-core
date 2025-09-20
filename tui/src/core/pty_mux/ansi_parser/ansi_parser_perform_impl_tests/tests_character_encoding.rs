// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Tests for character encoding support - UTF-8, emojis, and wide characters.

use vte::Perform;

use super::create_test_offscreen_buffer_10r_by_10c;
use crate::{AnsiToBufferProcessor,
            offscreen_buffer::test_fixtures_offscreen_buffer::*};

#[test]
fn test_utf8_characters() {
    let mut ofs_buf = create_test_offscreen_buffer_10r_by_10c();

    // Process UTF-8 characters including emojis
    {
        let mut processor = AnsiToBufferProcessor::new(&mut ofs_buf);

        // Print various UTF-8 characters
        processor.print('H');
        processor.print('é'); // Latin character with accent
        processor.print('中'); // Chinese character
        processor.print('🦀'); // Emoji (Rust crab)
        processor.print('!');
    }

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
}