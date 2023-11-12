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

use std::io::{Result, *};

use crossterm::{cursor::*, queue, terminal::*, style::*};
use r3bl_rs_utils_core::*;

use crate::*;

pub trait FunctionComponent<W: Write, S:ViewportHeight> {
    fn get_write(&mut self) -> &mut W;
    fn calculate_viewport_height(&self, state:&S) -> ChUnit;
    fn render(&mut self, state: &S, shared_global_data : &mut SharedGlobalData) -> Result<()>;
    fn allocate_viewport_height_space(&mut self, state: &S) -> Result<()>{
        let viewport_height =
            /* including the header */ self.calculate_viewport_height(state);

        // Allocate space. This is required so that the commands to move the cursor up and
        // down shown below will work.
        for _ in 0..*viewport_height {
            println!();
        }

        // Move the cursor back up.
        let writer = self.get_write();
        queue! (
            writer,
            MoveToPreviousLine(*viewport_height),
        )?;

        Ok(())
    }

    fn clear_viewport(&mut self, state: &S) -> Result<()> {
        let viewport_height =
            /* including the header */ self.calculate_viewport_height(state);

        let writer = self.get_write();
        // Clear the viewport.
        for _ in 0..*viewport_height {
            queue! (
                writer,
                ResetColor,
                Clear(ClearType::CurrentLine),
                MoveToNextLine(1),
            )?;
        }

        // Move the cursor back up.
        queue! (
            writer,
            MoveToPreviousLine(*viewport_height),
        )?;

        Ok(())
    }
}
