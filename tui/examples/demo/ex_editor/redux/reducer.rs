/*
 *   Copyright (c) 2022 R3BL LLC
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

use async_trait::async_trait;
use r3bl_redux::*;
use r3bl_tui::DialogBuffer;

use super::*;

#[derive(Default)]
pub struct Reducer;

#[async_trait]
impl AsyncReducer<State, Action> for Reducer {
    async fn run(&self, action: &Action, state: &State) -> State {
        match action {
            Action::Noop => state.clone(),

            Action::UpdateEditorBufferById(id, buffer) => {
                let mut new_state = state.clone();
                new_state.editor_buffers.insert(*id, buffer.clone());
                new_state
            }

            Action::SetDialogBufferTitleAndTextById(id, title, text) => {
                let mut new_state = state.clone();

                let dialog_buffer = {
                    let mut it = DialogBuffer::new_empty();
                    it.title = title.into();
                    it.editor_buffer.set_lines(vec![text.into()]);
                    it
                };

                new_state.dialog_buffers.insert(*id, dialog_buffer);

                new_state
            }

            Action::UpdateDialogBufferById(id, editor_buffer) => {
                let mut new_state = state.clone();

                new_state
                    .dialog_buffers
                    .entry(*id)
                    .and_modify(|it| it.editor_buffer = editor_buffer.clone())
                    .or_insert_with(
                        // This code path should never execute, since to update the buffer given an
                        // id, it should have already existed in the first place (created by
                        // SetDialogBufferTitleAndTextById action).
                        || {
                            let mut it = DialogBuffer::new_empty();
                            it.editor_buffer = editor_buffer.clone();
                            it
                        },
                    );

                new_state
            }
        }
    }
}
