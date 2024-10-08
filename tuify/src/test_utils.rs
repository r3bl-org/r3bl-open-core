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

use std::io::{Result, Write};

use crate::{KeyPress, KeyPressReader};

pub struct TestStringWriter {
    buffer: String,
}

impl Default for TestStringWriter {
    fn default() -> Self { Self::new() }
}

impl TestStringWriter {
    pub fn new() -> Self {
        TestStringWriter {
            buffer: String::new(),
        }
    }

    pub fn get_buffer(&self) -> &str { &self.buffer }
}

impl Write for TestStringWriter {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let result = std::str::from_utf8(buf);
        match result {
            Ok(value) => {
                self.buffer.push_str(value);
                Ok(buf.len())
            }
            Err(_) => Ok(0),
        }
    }

    fn flush(&mut self) -> Result<()> { Ok(()) }
}

pub struct TestVecKeyPressReader {
    pub key_press_vec: Vec<KeyPress>,
    pub index: Option<usize>,
}

impl KeyPressReader for TestVecKeyPressReader {
    fn read_key_press(&mut self) -> KeyPress {
        // Increment index every time this function is called until the end of the vector
        // and then wrap around.
        match self.index {
            Some(index) => {
                if index < self.key_press_vec.len() - 1 {
                    self.index = Some(index + 1);
                } else {
                    self.index = Some(0);
                }
            }
            None => {
                self.index = Some(0);
            }
        }

        let index = self.index.unwrap();

        self.key_press_vec[index]
    }
}

pub fn contains_ansi_escape_sequence(text: &str) -> bool {
    text.chars().any(|it| it == '\x1b')
}

#[test]
fn test_is_ansi_escape_sequence() {
    use r3bl_ansi_color::{AnsiStyledText, Color, Style};
    use r3bl_core::assert_eq2;

    assert_eq2!(
        contains_ansi_escape_sequence(
            "\x1b[31mThis is red text.\x1b[0m And this is normal text."
        ),
        true
    );

    assert_eq2!(contains_ansi_escape_sequence("This is normal text."), false);

    assert_eq2!(
        contains_ansi_escape_sequence(
            &AnsiStyledText {
                text: "Print a formatted (bold, italic, underline) string w/ ANSI color codes.",
                style: &[
                    Style::Bold,
                    Style::Italic,
                    Style::Underline,
                    Style::Foreground(Color::Rgb(50, 50, 50)),
                    Style::Background(Color::Rgb(100, 200, 1)),
                ],
            }
            .to_string()
        ),
        true
    );
}
