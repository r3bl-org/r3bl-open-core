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
use r3bl_rs_utils::*;

use super::*;
use crate::DEBUG;

#[derive(Debug, Clone, Default)]
pub struct ColumnRenderComponent {
  pub lolcat: Lolcat,
}

macro_rules! fire {
  (@add_pop => $arg_event_consumed: ident, $arg_shared_store: ident, $arg_action: expr) => {
    spawn_and_consume_event!($arg_event_consumed, $arg_shared_store, $arg_action);

    debug_log_action(
      "ColumnRenderComponent::handle_event".to_string(),
      $arg_action,
    );

    call_if_true!(
      DEBUG,
      log_no_err!(
        INFO,
        "⛵ ColumnRenderComponent::handle_event -> + -> dispatch_spawn: {}",
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
        "⛵ ColumnRenderComponent::handle_event -> - -> dispatch_spawn: {}",
        $arg_action
      )
    );
  };
}

#[async_trait]
impl Component<State, Action> for ColumnRenderComponent {
  /// Handle following input events (and consume them):
  /// - Up,   `+` : fire `AddPop(1)`
  /// - Down, `-` : fire `SubPop(1)`
  async fn handle_event(
    &mut self, component_registry: &mut ComponentRegistry<State, Action>, has_focus: &mut HasFocus,
    input_event: &InputEvent, _state: &State, shared_store: &SharedStore<State, Action>,
    shared_tw_data: &SharedTWData,
  ) -> CommonResult<EventPropagation> {
    throws_with_return!({
      let mut event_consumed = false;

      if let InputEvent::Keyboard(Keypress::Plain { key }) = input_event {
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

  async fn render(
    &mut self, component_registry: &mut ComponentRegistry<State, Action>, has_focus: &mut HasFocus,
    current_box: &FlexBox, _: &State, _: &SharedStore<State, Action>,
    shared_tw_data: &SharedTWData,
  ) -> CommonResult<RenderPipeline> {
    throws_with_return!({
      // Fixed strings.
      let line_1 = format!("{} - Hello", current_box.id);
      let line_2 = format!("{} - World", current_box.id);

      // Setup intermediate vars.
      let box_origin_pos = current_box.style_adjusted_origin_pos; // Adjusted for style margin (if any).
      let box_bounds_size = current_box.style_adjusted_bounds_size; // Adjusted for style margin (if any).
      let mut content_cursor_pos = position! { col: 0 , row: 0 };
      let mut render_pipeline: RenderPipeline = render_pipeline!(@new_empty);

      // Line 1.
      render_pipeline! {
        @push_into render_pipeline at ZOrder::Normal =>
          RenderOp::MoveCursorPositionRelTo(box_origin_pos, content_cursor_pos),
          RenderOp::ApplyColors(current_box.get_computed_style()),
          RenderOp::PrintTextWithAttributes(
            colorize_using_lolcat! {
              &mut self.lolcat,
              "{}",
              UnicodeString::from(line_1).truncate_to_fit_size(box_bounds_size)
            },
            current_box.get_computed_style(),
          )
      };

      // Line 2.
      render_pipeline! {
        @push_into render_pipeline at ZOrder::Normal =>
          RenderOp::MoveCursorPositionRelTo(
            box_origin_pos,
            content_cursor_pos.add_rows_with_bounds(ch!(1), box_bounds_size.row)
          ),
          RenderOp::PrintTextWithAttributes(
            colorize_using_lolcat! {
              &mut self.lolcat,
              "{}",
              UnicodeString::from(line_2).truncate_to_fit_size(box_bounds_size)
            },
            current_box.get_computed_style(),
          ),
          RenderOp::ResetColor
      };

      // Paint is_focused.
      if has_focus.does_current_box_have_focus(current_box) {
        render_pipeline! {
          @push_into render_pipeline at ZOrder::Normal =>
            RenderOp::MoveCursorPositionRelTo(
              box_origin_pos,
              content_cursor_pos.add_rows_with_bounds(ch!(1), box_bounds_size.row)
            ),
            RenderOp::PrintTextWithAttributes("👀".into(), None)
        };
      }

      call_if_true!(DEBUG, {
        log_no_err! {
          INFO,
          "\
🦜 ColumnComponent::render ->
  - current_box: {:?},
  - box_origin_pos: {:?},
  - box_bounds_size: {:?},
  - content_pos: {:?},
  - render_pipeline: {:?}",
          current_box,
          box_origin_pos,
          box_bounds_size,
          content_cursor_pos,
          render_pipeline
        };
      });

      // Return the render_pipeline.
      render_pipeline
    });
  }
}
