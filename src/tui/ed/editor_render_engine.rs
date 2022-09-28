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

use std::fmt::{Debug, Display};

use serde::*;

use crate::*;

const DEFAULT_CURSOR_CHAR: char = '▒';

#[derive(Debug)]
enum CaretPaintStyle {
  /// Using cursor show / hide.
  #[allow(dead_code)]
  GlobalCursor,
  /// Painting the editor_buffer.get_caret() position w/ reverse style.
  LocalPaintedEffect,
}

/// Holds data related to rendering in between render calls. This is not stored in the
/// [EditorBuffer] struct, which lives in the [Store]. The store provides the underlying document or
/// buffer struct that holds the actual document.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditorRenderEngine {
  /// Set by [render](EditorRenderEngine::render).
  pub current_box: FlexBox,
}

impl EditorRenderEngine {
  // FIXME: impl apply #23
  pub async fn apply<S, A>(
    &mut self,
    args: EditorEngineArgs<'_, S, A>,
    input_event: &InputEvent,
  ) -> CommonResult<Option<EditorBuffer>>
  where
    S: Default + Display + Clone + PartialEq + Debug + Sync + Send,
    A: Default + Display + Clone + Sync + Send,
  {
    let EditorEngineArgs {
      buffer,
      component_registry,
      shared_tw_data,
      self_id,
      ..
    } = args;

    // TK: 🚨🔮 resize -> caret + scroll fix in editor buffer; need to handle resize event
    // scroll::validate_caret_in_viewport_activate_scroll_if_needed(EditorArgsMut {
    //   buffer,
    //   engine: self,
    // });

    if let Ok(editor_event) = EditorBufferCommand::try_from(input_event) {
      let mut new_editor_buffer = buffer.clone();
      EditorBuffer::apply_editor_event(
        self,
        &mut new_editor_buffer,
        editor_event,
        shared_tw_data,
        component_registry,
        self_id,
      );
      Ok(Some(new_editor_buffer))
    } else {
      Ok(None)
    }
  }

  // FIXME: impl render #23
  pub async fn render<S, A>(
    &mut self,
    args: EditorEngineArgs<'_, S, A>,
    current_box: &FlexBox,
  ) -> CommonResult<RenderPipeline>
  where
    S: Default + Display + Clone + PartialEq + Debug + Sync + Send,
    A: Default + Display + Clone + Sync + Send,
  {
    throws_with_return!({
      let EditorEngineArgs {
        buffer,
        component_registry,
        ..
      } = args;

      self.current_box = current_box.clone();

      // Create reusable args for render functions.
      let render_args = RenderArgs {
        buffer,
        component_registry,
      };

      if buffer.is_empty() {
        self.render_empty_state(&render_args)
      } else {
        let q_content = self.render_content(&render_args);
        let q_caret = self.render_caret(CaretPaintStyle::LocalPaintedEffect, &render_args);
        render_pipeline!(@join_and_drop q_content, q_caret)
      }
    })
  }

  // This simply clips the content to the `style_adj_box_bounds_size`.
  fn render_content<S, A>(&mut self, render_args: &RenderArgs<'_, S, A>) -> RenderPipeline
  where
    S: Default + Display + Clone + PartialEq + Debug + Sync + Send,
    A: Default + Display + Clone + Sync + Send,
  {
    let RenderArgs { buffer, .. } = render_args;
    let mut render_pipeline = render_pipeline!(@new_empty);

    let Size {
      cols: max_display_col_count,
      rows: max_display_row_count,
    } = self.current_box.style_adjusted_bounds_size;

    // Paint each line in the buffer (skipping the scroll_offset.row).
    // https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.skip
    for (row_index, line) in buffer
      .get_lines()
      .iter()
      .skip(ch!(@to_usize buffer.get_scroll_offset().row))
      .enumerate()
    {
      // Clip the content to max rows.
      if ch!(row_index) > max_display_row_count {
        // TK: ‼️ remove debug
        log_no_err!(
          DEBUG,
          "🟥🟥🟥 row_index {:?} > max_display_row_count {:?}, line: {:?}",
          row_index,
          *max_display_row_count,
          line.string,
        );
        break;
      }

      // Clip the content to max cols.
      let truncated_line = line.truncate_to_fit_display_cols(max_display_col_count);
      render_pipeline! {
        @push_into render_pipeline at ZOrder::Normal =>
          RenderOp::MoveCursorPositionRelTo(
            self.current_box.style_adjusted_origin_pos, position! { col: 0 , row: ch!(@to_usize row_index) }
          ),
          RenderOp::ApplyColors(self.current_box.get_computed_style()),
          RenderOp::PrintTextWithAttributes(truncated_line.into(), self.current_box.get_computed_style()),
          RenderOp::ResetColor
      };

      // TK: ‼️ remove debug
      log_no_err!(
        DEBUG,
        "👉🟡🟡🟡 row_index: {:?}, max_display_row_count: {:?}, line: {:?}",
        row_index,
        *max_display_row_count,
        line.string,
      );
    }

    render_pipeline
  }

  /// Implement caret painting using two different strategies represented by [CaretPaintStyle].
  fn render_caret<S, A>(
    &mut self,
    style: CaretPaintStyle,
    render_args: &RenderArgs<'_, S, A>,
  ) -> RenderPipeline
  where
    S: Default + Display + Clone + PartialEq + Debug + Sync + Send,
    A: Default + Display + Clone + Sync + Send,
  {
    let RenderArgs {
      component_registry,
      buffer,
      ..
    } = render_args;
    let mut render_pipeline: RenderPipeline = RenderPipeline::default();

    if component_registry
      .has_focus
      .does_current_box_have_focus(&self.current_box)
    {
      match style {
        CaretPaintStyle::GlobalCursor => {
          render_pipeline! {
            @push_into render_pipeline at ZOrder::Caret =>
              RenderOp::RequestShowCaretAtPositionRelTo(
                self.current_box.style_adjusted_origin_pos, buffer.get_caret(CaretKind::Raw))
          };
        }
        CaretPaintStyle::LocalPaintedEffect => {
          let str_at_caret: String = if let Some(UnicodeStringSegmentSliceResult {
            unicode_string_seg: str_seg,
            ..
          }) = editor_ops_get_content::string_at_caret(buffer, self)
          {
            str_seg.string
          } else {
            DEFAULT_CURSOR_CHAR.into()
          };

          // TK: ‼️ remove debug
          log_no_err!(
            DEBUG,
            "👆🔵🔵🔵 str_at_caret: {:?}, caret(Raw): {:?}, scroll_offset: {:?}",
            str_at_caret,
            buffer.get_caret(CaretKind::Raw),
            buffer.get_scroll_offset(),
          );

          render_pipeline! {
            @push_into render_pipeline at ZOrder::Caret =>
            RenderOp::MoveCursorPositionRelTo(
              self.current_box.style_adjusted_origin_pos, buffer.get_caret(CaretKind::Raw)),
              RenderOp::PrintTextWithAttributes(
                str_at_caret,
                style! { attrib: [reverse] }.into()),
            RenderOp::MoveCursorPositionRelTo(
              self.current_box.style_adjusted_origin_pos, buffer.get_caret(CaretKind::Raw))
          };
        }
      }
    }

    render_pipeline
  }

  fn render_empty_state<S, A>(&mut self, render_args: &RenderArgs<'_, S, A>) -> RenderPipeline
  where
    S: Default + Display + Clone + PartialEq + Debug + Sync + Send,
    A: Default + Display + Clone + Sync + Send,
  {
    let RenderArgs {
      component_registry, ..
    } = render_args;
    let mut render_pipeline: RenderPipeline = RenderPipeline::default();
    let mut content_cursor_pos = position! { col: 0 , row: 0 };

    // Paint the text.
    render_pipeline! {
      @push_into render_pipeline at ZOrder::Normal =>
        RenderOp::MoveCursorPositionRelTo(
          self.current_box.style_adjusted_origin_pos, position! { col: 0 , row: 0 }),
        RenderOp::ApplyColors(style! {
          color_fg: TWColor::Red
        }.into()),
        RenderOp::PrintTextWithAttributes("No content added".into(), None),
        RenderOp::ResetColor
    };

    // Paint the emoji.
    if component_registry
      .has_focus
      .does_current_box_have_focus(&self.current_box)
    {
      render_pipeline! {
        @push_into render_pipeline at ZOrder::Normal =>
          RenderOp::MoveCursorPositionRelTo(
            self.current_box.style_adjusted_origin_pos,
            content_cursor_pos.add_row_with_bounds(
              ch!(1), self.current_box.style_adjusted_bounds_size.rows)),
          RenderOp::PrintTextWithAttributes("👀".into(), None)
      };
    }

    render_pipeline
  }
}
