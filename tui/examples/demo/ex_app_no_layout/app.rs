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
use r3bl_rs_utils_core::*;
use r3bl_tui::*;

use super::*;
use crate::{DEBUG, *};

/// Async trait object that implements the [Render] trait.
#[derive(Default, Debug, Clone, Copy)]
pub struct AppNoLayout {
    pub lolcat: Lolcat,
}

macro_rules! fire {
    (@add_pop => $arg_event_consumed: ident, $arg_shared_store: ident, $arg_action: expr) => {
        spawn_and_consume_event!($arg_event_consumed, $arg_shared_store, $arg_action);
        call_if_true!(
            DEBUG,
            log_no_err!(
                INFO,
                "⛵ AppNoLayout::handle_event -> + -> dispatch_spawn: {}",
                $arg_action
            )
        );
    };
    (@sub_pop => $arg_event_consumed: ident, $arg_shared_store: ident, $arg_action: expr) => {
        spawn_and_consume_event!($arg_event_consumed, $arg_shared_store, $arg_action);
        call_if_true!(
            DEBUG,
            log_no_err!(
                INFO,
                "⛵ AppNoLayout::handle_event -> - -> dispatch_spawn: {}",
                $arg_action
            )
        );
    };
}

#[async_trait]
impl App<State, Action> for AppNoLayout {
    async fn app_render(
        &mut self,
        args: GlobalScopeArgs<'_, State, Action>,
    ) -> CommonResult<RenderPipeline> {
        throws_with_return!({
            let GlobalScopeArgs {
                state,
                shared_global_data,
                ..
            } = args;

            let content = format!("{state}");

            let content_size_col: ChUnit = content.len().into();
            let window_size: Size = shared_global_data.read().await.get_size();

            let col: ChUnit = (window_size.col_count - content_size_col) / 2;
            let row: ChUnit = window_size.row_count / 2;

            let plain_content = format!("{state}");
            let unicode_string = UnicodeString::from(plain_content);
            let colored_content =
                lolcat_each_char_in_unicode_string(&unicode_string, Some(&mut self.lolcat));
            self.lolcat.next_color();

            let mut pipeline = render_pipeline!(
              @new ZOrder::Normal
              =>
                RenderOp::ResetColor,
                RenderOp::MoveCursorPositionAbs(position!(col_index: col, row_index: row)),
                RenderOp::PrintTextWithAttributes(colored_content, None),
            );

            status_bar_helpers::create_status_bar_message(&mut pipeline, window_size);

            pipeline
        });
    }

    async fn app_handle_event(
        &mut self,
        args: GlobalScopeArgs<'_, State, Action>,
        input_event: &InputEvent,
    ) -> CommonResult<EventPropagation> {
        throws_with_return!({
            let GlobalScopeArgs { shared_store, .. } = args;

            call_if_true!(
                DEBUG,
                log_no_err!(
                    INFO,
                    "⛵ AppNoLayout::handle_event -> input_event: {}",
                    input_event
                )
            );

            let mut event_consumed = false;

            if let InputEvent::Keyboard(KeyPress::Plain { key }) = input_event {
                // Check for + or - key.
                if let Key::Character(typed_char) = key {
                    match typed_char {
                        '+' => {
                            fire!(@add_pop => event_consumed, shared_store, Action::AddPop(1));
                        }
                        '-' => {
                            fire!(@sub_pop => event_consumed, shared_store, Action::SubPop(1));
                        }
                        _ => {}
                    }
                }

                // Check for up or down arrow key.
                if let Key::SpecialKey(special_key) = key {
                    match special_key {
                        SpecialKey::Up => {
                            fire!(@add_pop => event_consumed, shared_store, Action::AddPop(1));
                        }
                        SpecialKey::Down => {
                            fire!(@sub_pop => event_consumed, shared_store, Action::SubPop(1));
                        }
                        _ => {}
                    }
                }
            }

            if event_consumed {
                EventPropagation::Consumed
            } else {
                EventPropagation::Propagate
            }
        });
    }
}

mod status_bar_helpers {
    use super::*;

    /// Shows helpful messages at the bottom row of the screen.
    pub fn create_status_bar_message(pipeline: &mut RenderPipeline, size: Size) {
        let styled_texts = styled_texts! {
          styled_text! { "Hints:",        style!(attrib: [dim])       },
          styled_text! { " x : Exit ⛔ ", style!(attrib: [bold])      },
          styled_text! { " … ",           style!(attrib: [dim])       },
          styled_text! { " ↑ / + : inc ", style!(attrib: [underline]) },
          styled_text! { " … ",           style!(attrib: [dim])       },
          styled_text! { " ↓ / - : dec ", style!(attrib: [underline]) }
        };

        let display_width = styled_texts.display_width();
        let col_center: ChUnit = (size.col_count - display_width) / 2;
        let row_bottom: ChUnit = size.row_count - 1;
        let center: Position = position!(col_index: col_center, row_index: row_bottom);

        let mut render_ops = render_ops!();
        render_ops.push(RenderOp::MoveCursorPositionAbs(center));
        styled_texts.render_into(&mut render_ops);
        pipeline.push(ZOrder::Normal, render_ops);
    }
}
