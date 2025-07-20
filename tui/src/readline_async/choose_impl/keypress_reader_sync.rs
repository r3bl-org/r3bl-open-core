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

use crate::InputEvent;

pub trait KeyPressReader {
    fn read_key_press(&mut self) -> Option<InputEvent>;
}

#[derive(Debug)]
pub struct CrosstermKeyPressReader;

impl KeyPressReader for CrosstermKeyPressReader {
    fn read_key_press(&mut self) -> Option<InputEvent> {
        let maybe_read = crossterm::event::read().ok()?;
        InputEvent::try_from(maybe_read).ok()
    }
}

#[derive(Debug)]
pub struct TestVecKeyPressReader {
    pub key_press_vec: Vec<InputEvent>,
    pub index: Option<usize>,
}

impl KeyPressReader for TestVecKeyPressReader {
    #[allow(clippy::unwrap_in_result)] /* This is only used in tests */
    fn read_key_press(&mut self) -> Option<InputEvent> {
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

        Some(self.key_press_vec[index].clone())
    }
}
