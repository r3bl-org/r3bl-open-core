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

use std::io::Result;

use crossterm::{cursor::{MoveToNextLine, MoveToPreviousLine},
                terminal::{Clear, ClearType}};

use crate::{queue_commands,
            throws,
            ChUnit,
            OutputDevice,
            ResizeHint,
            Size,
            DEVELOPMENT_MODE};

pub trait CalculateResizeHint {
    fn set_size(&mut self, new_size: Size);
    fn get_resize_hint(&self) -> Option<ResizeHint>;
    fn set_resize_hint(&mut self, new_size: Size);
    fn clear_resize_hint(&mut self);
}

pub trait FunctionComponent<S: CalculateResizeHint> {
    fn get_output_device(&mut self) -> OutputDevice;

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
            queue_commands! {
                self.get_output_device(),
                MoveToPreviousLine(*viewport_height),
            };
        });
    }

    fn clear_viewport_for_resize(&mut self, state: &mut S) -> Result<()> {
        throws!({
            DEVELOPMENT_MODE.then(|| {
                // % is Display, ? is Debug.
                tracing::debug!(
                    message = "🥑🥑🥑 clear viewport for resize",
                    resize_hint = ?state.get_resize_hint()
                );
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

            // Clear the viewport.
            for _ in 0..*viewport_height {
                queue_commands! {
                    self.get_output_device(),
                    Clear(ClearType::FromCursorDown),
                    MoveToNextLine(1),
                };
            }

            // Move the cursor back up.
            queue_commands! {
                self.get_output_device(),
                MoveToPreviousLine(*viewport_height),
            };

            // Clear resize hint.
            state.clear_resize_hint();
        });
    }

    fn clear_viewport(&mut self, state: &mut S) -> Result<()> {
        throws!({
            let viewport_height =
                /* not including the header */ self.calculate_items_viewport_height(state) +
                /* for header row(s) */ self.calculate_header_viewport_height(state);

            // Clear the viewport.
            for _ in 0..*viewport_height {
                queue_commands! {
                    self.get_output_device(),
                    Clear(ClearType::CurrentLine),
                    MoveToNextLine(1),
                };
            }

            // Move the cursor back up.
            queue_commands! {
                self.get_output_device(),
                MoveToPreviousLine(*viewport_height),
            };
        });
    }
}
