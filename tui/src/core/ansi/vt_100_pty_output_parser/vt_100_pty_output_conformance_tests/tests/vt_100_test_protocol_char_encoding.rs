// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Tests for character encoding support - [`UTF-8`], emojis, and wide characters.
//!
//! [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8

use super::super::test_fixtures_vt_100_ansi_conformance::*;
use crate::{AnsiToOfsBufPerformer, PixelChar, col, ofs_buf::test_fixtures_ofs_buf::*,
            row};
use vte::Perform;

#[test]
fn test_utf8_characters() {
    let mut ofs_buf_vt_100 = create_test_ofs_buf_10r_by_10c();

    // Process UTF-8 characters including emojis.
    let mut performer = AnsiToOfsBufPerformer::new(&mut ofs_buf_vt_100);

    // Print various UTF-8 characters.
    performer.print('H'); // width 1
    performer.print('é'); // Latin character with accent, width 1
    performer.print('中'); // Chinese character, width 2
    performer.print('🦀'); // Emoji (Rust crab), width 2
    performer.print('!'); // width 1

    // Verify all UTF-8 characters are in the buffer. Wide characters (中, 🦀)
    // occupy their own cell plus a trailing `PixelChar::Void` for the extra
    // display column, so the narrow chars that follow them are pushed right.
    assert_plain_char_at(&ofs_buf_vt_100, 0, 0, 'H');
    assert_plain_char_at(&ofs_buf_vt_100, 0, 1, 'é');
    assert_plain_char_at(&ofs_buf_vt_100, 0, 2, '中');
    assert!(
        matches!(
            ofs_buf_vt_100.get_char(row(0) + col(3)),
            Some(PixelChar::Void)
        ),
        "expected Void trailing '中' at col 3"
    );
    assert_plain_char_at(&ofs_buf_vt_100, 0, 4, '🦀');
    assert!(
        matches!(
            ofs_buf_vt_100.get_char(row(0) + col(5)),
            Some(PixelChar::Void)
        ),
        "expected Void trailing '🦀' at col 5"
    );
    assert_plain_char_at(&ofs_buf_vt_100, 0, 6, '!');

    // Verify rest of line is empty.
    for col_idx in 7..10 {
        assert_empty_at(&ofs_buf_vt_100, 0, col_idx);
    }

    // Verify the rest of the buffer is empty.
    for row_idx in 1..10 {
        for col_idx in 0..10 {
            assert_empty_at(&ofs_buf_vt_100, row_idx, col_idx);
        }
    }
}
