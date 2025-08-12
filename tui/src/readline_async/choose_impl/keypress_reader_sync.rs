// Copyright (c) 2023-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

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
