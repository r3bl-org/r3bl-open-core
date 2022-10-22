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

use std::fmt::Debug;

use r3bl_rs_utils_core::*;
use r3bl_rs_utils_macro::style;

use super::*;
use crate::*;
const DEFAULT_CURSOR_CHAR: char = '▒';

// ┏━━━━━━━━━━━━━━━━━━━━━━━━━┓
// ┃ EditorEngine render API ┃
// ┛                         ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
/// Things you can do with editor engine.
pub struct EditorEngineRenderApi;

impl EditorEngineRenderApi {
  /// Event based interface for the editor. This converts the [InputEvent] into an [EditorEvent] and
  /// then executes it. Returns a new [EditorBuffer] if the operation was applied otherwise returns
  /// [None].
  pub async fn apply_event<S, A>(
    args: EditorEngineArgs<'_, S, A>,
    input_event: &InputEvent,
  ) -> CommonResult<ApplyResponse<EditorBuffer>>
  where
    S: Default + Clone + PartialEq + Debug + Sync + Send,
    A: Default + Clone + Sync + Send,
  {
    let EditorEngineArgs {
      editor_buffer,
      component_registry,
      shared_tw_data,
      self_id,
      editor_engine,
      ..
    } = args;

    if let Ok(editor_event) = EditorEvent::try_from(input_event) {
      let mut new_editor_buffer = editor_buffer.clone();
      EditorEvent::apply_editor_event(
        editor_engine,
        &mut new_editor_buffer,
        editor_event,
        shared_tw_data,
        component_registry,
        self_id,
      );
      Ok(ApplyResponse::Applied(new_editor_buffer))
    } else {
      Ok(ApplyResponse::NotApplied)
    }
  }

  pub async fn render_engine<S, A>(
    args: EditorEngineArgs<'_, S, A>,
    current_box: &FlexBox,
  ) -> CommonResult<RenderPipeline>
  where
    S: Default + Clone + PartialEq + Debug + Sync + Send,
    A: Default + Clone + Sync + Send,
  {
    throws_with_return!({
      let EditorEngineArgs {
        editor_buffer,
        component_registry,
        editor_engine,
        ..
      } = args;

      editor_engine.current_box = current_box.clone();

      // Create reusable args for render functions.
      let render_args = RenderArgs {
        editor_buffer,
        component_registry,
        editor_engine,
      };

      if editor_buffer.is_empty() {
        EditorEngineRenderApi::render_empty_state(&render_args)
      } else {
        let q_content = EditorEngineRenderApi::render_content(&render_args);
        let q_caret = EditorEngineRenderApi::render_caret(CaretPaintStyle::LocalPaintedEffect, &render_args);
        render_pipeline!(@join_and_drop q_content, q_caret)
      }
    })
  }

  // This simply clips the content to the `style_adj_box_bounds_size`.
  fn render_content<S, A>(render_args: &RenderArgs<'_, S, A>) -> RenderPipeline
  where
    S: Default + Clone + PartialEq + Debug + Sync + Send,
    A: Default + Clone + Sync + Send,
  {
    let RenderArgs {
      editor_buffer,
      editor_engine,
      ..
    } = render_args;
    let mut render_pipeline = render_pipeline!(@new_empty);

    let Size {
      cols: max_display_col_count,
      rows: max_display_row_count,
    } = editor_engine.current_box.style_adjusted_bounds_size;

    // Paint each line in the buffer (skipping the scroll_offset.row).
    // https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.skip
    for (row_index, line) in editor_buffer
      .get_lines()
      .iter()
      .skip(ch!(@to_usize editor_buffer.get_scroll_offset().row))
      .enumerate()
    {
      // Clip the content to max rows.
      if ch!(row_index) > max_display_row_count {
        break;
      }

      // Clip the content [scroll_offset.col .. max cols].
      let truncated_line = line.truncate_start_by_n_col(editor_buffer.get_scroll_offset().col);
      let truncated_line = UnicodeString::from(truncated_line);
      let truncated_line = truncated_line.truncate_end_to_fit_display_cols(max_display_col_count);

      render_pipeline! {
        @push_into render_pipeline at ZOrder::Normal =>
          RenderOp::MoveCursorPositionRelTo(
            editor_engine.current_box.style_adjusted_origin_pos, position! { col: 0 , row: ch!(@to_usize row_index) }
          ),
          RenderOp::ApplyColors(editor_engine.current_box.get_computed_style()),
          RenderOp::PrintTextWithAttributes(truncated_line.into(), editor_engine.current_box.get_computed_style()),
          RenderOp::ResetColor
      };
    }

    render_pipeline
  }

  /// Implement caret painting using two different strategies represented by [CaretPaintStyle].
  fn render_caret<S, A>(style: CaretPaintStyle, render_args: &RenderArgs<'_, S, A>) -> RenderPipeline
  where
    S: Default + Clone + PartialEq + Debug + Sync + Send,
    A: Default + Clone + Sync + Send,
  {
    let RenderArgs {
      component_registry,
      editor_buffer,
      editor_engine,
      ..
    } = render_args;
    let mut render_pipeline: RenderPipeline = RenderPipeline::default();

    if component_registry
      .has_focus
      .does_current_box_have_focus(&editor_engine.current_box)
    {
      match style {
        CaretPaintStyle::GlobalCursor => {
          render_pipeline! {
            @push_into render_pipeline at ZOrder::Caret =>
              RenderOp::RequestShowCaretAtPositionRelTo(
                editor_engine.current_box.style_adjusted_origin_pos, editor_buffer.get_caret(CaretKind::Raw))
          };
        }
        CaretPaintStyle::LocalPaintedEffect => {
          let str_at_caret: String = if let Some(UnicodeStringSegmentSliceResult {
            unicode_string_seg: str_seg,
            ..
          }) = EditorEngineDataApi::string_at_caret(editor_buffer, editor_engine)
          {
            str_seg.string
          } else {
            DEFAULT_CURSOR_CHAR.into()
          };

          render_pipeline! {
            @push_into render_pipeline at ZOrder::Caret =>
            RenderOp::MoveCursorPositionRelTo(
              editor_engine.current_box.style_adjusted_origin_pos, editor_buffer.get_caret(CaretKind::Raw)),
              RenderOp::PrintTextWithAttributes(
                str_at_caret,
                style! { attrib: [reverse] }.into()),
            RenderOp::MoveCursorPositionRelTo(
              editor_engine.current_box.style_adjusted_origin_pos, editor_buffer.get_caret(CaretKind::Raw))
          };
        }
      }
    }

    render_pipeline
  }

  pub fn render_empty_state<S, A>(render_args: &RenderArgs<'_, S, A>) -> RenderPipeline
  where
    S: Default + Clone + PartialEq + Debug + Sync + Send,
    A: Default + Clone + Sync + Send,
  {
    let RenderArgs {
      component_registry,
      editor_engine,
      ..
    } = render_args;
    let mut render_pipeline: RenderPipeline = RenderPipeline::default();
    let mut content_cursor_pos = position! { col: 0 , row: 0 };

    // Paint the text.
    render_pipeline! {
      @push_into render_pipeline at ZOrder::Normal =>
        RenderOp::MoveCursorPositionRelTo(
          editor_engine.current_box.style_adjusted_origin_pos, position! { col: 0 , row: 0 }),
        RenderOp::ApplyColors(style! {
          color_fg: TWColor::Red
        }.into()),
        RenderOp::PrintTextWithAttributes("No content added".into(), None),
        RenderOp::ResetColor
    };

    // Paint the emoji.
    if component_registry
      .has_focus
      .does_current_box_have_focus(&editor_engine.current_box)
    {
      render_pipeline! {
        @push_into render_pipeline at ZOrder::Normal =>
          RenderOp::MoveCursorPositionRelTo(
            editor_engine.current_box.style_adjusted_origin_pos,
            content_cursor_pos.add_row_with_bounds(
              ch!(1), editor_engine.current_box.style_adjusted_bounds_size.rows)),
          RenderOp::PrintTextWithAttributes("👀".into(), None)
      };
    }

    render_pipeline
  }
}

mod misc {
  use super::*;

  #[derive(Debug)]
  pub(super) enum CaretPaintStyle {
    /// Using cursor show / hide.
    #[allow(dead_code)]
    GlobalCursor,
    /// Painting the editor_buffer.get_caret() position w/ reverse style.
    LocalPaintedEffect,
  }

  pub enum ApplyResponse<T>
  where
    T: Debug,
  {
    Applied(T),
    NotApplied,
  }
}
pub use misc::*;
