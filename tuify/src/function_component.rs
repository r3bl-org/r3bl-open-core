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

use crossterm::{cursor::{MoveToNextLine, MoveToPreviousLine},
                queue,
                terminal::{Clear, ClearType}};
use r3bl_core::{call_if_true, throws, ChUnit, Size};

use crate::{ResizeHint, DEVELOPMENT_MODE};

pub trait CalculateResizeHint {
    fn set_size(&mut self, new_size: Size);
    fn get_resize_hint(&self) -> Option<ResizeHint>;
    fn set_resize_hint(&mut self, new_size: Size);
    fn clear_resize_hint(&mut self);
}

pub trait FunctionComponent<W: Write, S: CalculateResizeHint> {
    fn get_write(&mut self) -> &mut W;

    fn calculate_header_viewport_height(&self, state: &mut S) -> ChUnit;

    fn calculate_items_viewport_height(&self, state: &mut S) -> ChUnit;

    fn render(&mut self, state: &mut S) -> Result<()>;

    fn allocate_viewport_height_space(&mut self, state: &mut S) -> Result<()> {
        throws!({
            let viewport_height =
                /* not including the header */ self.calculate_items_viewport_height(state) +
                /* for header row(s) */ self.calculate_header_viewport_height(state);

            // Allocate space. This is required so that the commands to move the cursor up and
            // down shown below will work.
            for _ in 0..*viewport_height {
                println!();
            }

            // Move the cursor back up.
            let writer = self.get_write();
            queue! {
                writer,
                MoveToPreviousLine(*viewport_height),
            }?;
        });
    }

    fn clear_viewport_for_resize(&mut self, state: &mut S) -> Result<()> {
        throws!({
            call_if_true!(DEVELOPMENT_MODE, {
                tracing::debug!("\n🥑🥑🥑\nresize hint: {:?}", state.get_resize_hint());
            });

            let viewport_height = match state.get_resize_hint() {
                // Resize happened.
                Some(ResizeHint::GotBigger)
                | Some(ResizeHint::NoChange)
                | Some(ResizeHint::GotSmaller) => {
                    /* not including the header */
                    self.calculate_items_viewport_height(state) +
                    /* for header row(s) */
                    self.calculate_header_viewport_height(state)
                }
                // Nothing to do, since resize didn't happen.
                None => return Ok(()),
            };

            let writer = self.get_write();

            // Clear the viewport.
            for _ in 0..*viewport_height {
                queue! {
                    writer,
                    Clear(ClearType::FromCursorDown),
                    MoveToNextLine(1),
                }?;
            }

            // Move the cursor back up.
            queue! {
                writer,
                MoveToPreviousLine(*viewport_height),
            }?;

            // Clear resize hint.
            state.clear_resize_hint();
        });
    }

    fn clear_viewport(&mut self, state: &mut S) -> Result<()> {
        throws!({
            let viewport_height =
                /* not including the header */ self.calculate_items_viewport_height(state) +
                /* for header row(s) */ self.calculate_header_viewport_height(state);

            let writer = self.get_write();

            // Clear the viewport.
            for _ in 0..*viewport_height {
                queue! {
                    writer,
                    Clear(ClearType::CurrentLine),
                    MoveToNextLine(1),
                }?;
            }

            // Move the cursor back up.
            queue! {
                writer,
                MoveToPreviousLine(*viewport_height),
            }?;
        });
    }
}
