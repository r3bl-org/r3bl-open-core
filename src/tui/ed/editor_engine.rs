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

/// Private struct to help keep function signatures smaller.
#[derive(Debug)]
struct RenderArgs<'a, S, A>
where
  S: Default + Display + Clone + PartialEq + Debug + Sync + Send,
  A: Default + Display + Clone + Sync + Send,
{
  editor_buffer: &'a EditorBuffer,
  component_registry: &'a ComponentRegistry<S, A>,
}

/// Holds data related to rendering in between render calls. This is not stored in the
/// [EditorBuffer] struct, which lives in the [Store]. The store provides the underlying document or
/// buffer struct that holds the actual document.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditorEngine {
  /// The col and row offset for scrolling if active.
  pub scroll_offset: ScrollOffset,
  /// Set by [render](EditorEngine::render).
  pub origin_pos: Position,
  /// Set by [render](EditorEngine::render).
  pub bounds_size: Size,
  /// Set by [render](EditorEngine::render).
  pub current_box: FlexBox,
}

impl EditorEngine {
  // FIXME: impl apply #23
  pub async fn apply<S, A>(
    &mut self,
    component_registry: &mut ComponentRegistry<S, A>,
    editor_buffer: &EditorBuffer,
    input_event: &InputEvent,
    shared_tw_data: &SharedTWData,
    self_id: &str,
  ) -> CommonResult<Option<EditorBuffer>>
  where
    S: Default + Display + Clone + PartialEq + Debug + Sync + Send,
    A: Default + Display + Clone + Sync + Send,
  {
    if let Ok(editor_event) = EditorBufferCommand::try_from(input_event) {
      let mut new_editor_buffer = editor_buffer.clone();
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
    editor_buffer: &EditorBuffer,
    component_registry: &mut ComponentRegistry<S, A>,
    current_box: &FlexBox,
    shared_tw_data: &SharedTWData,
    self_id: &str,
  ) -> CommonResult<RenderPipeline>
  where
    S: Default + Display + Clone + PartialEq + Debug + Sync + Send,
    A: Default + Display + Clone + Sync + Send,
  {
    throws_with_return!({
      self.bounds_size = current_box.style_adjusted_bounds_size;
      self.origin_pos = current_box.style_adjusted_origin_pos;
      self.current_box = current_box.clone();

      // TK: remove debug
      log_no_err!(
        DEBUG,
        "🟨🟨🟨 self.bounds_size -> {:?}, self.origin_pos -> {:?}",
        self.bounds_size,
        self.origin_pos
      );

      // Create this struct to pass around fewer variables.
      let render_args = RenderArgs {
        editor_buffer,
        component_registry,
      };

      if editor_buffer.is_empty() {
        self.render_empty_state(&render_args)
      } else {
        let q_content = self.render_content(&render_args);
        let q_caret = self.render_caret(CaretPaintStyle::LocalPaintedEffect, &render_args);
        render_pipeline!(@join_and_drop q_content, q_caret)
      }
    })
  }

  // This simply clips the content to the `style_adj_box_bounds_size`.
  // TK: 📜✅ scroll enable render_content
  fn render_content<S, A>(&mut self, render_args: &RenderArgs<'_, S, A>) -> RenderPipeline
  where
    S: Default + Display + Clone + PartialEq + Debug + Sync + Send,
    A: Default + Display + Clone + Sync + Send,
  {
    let RenderArgs { editor_buffer, .. } = render_args;
    let mut render_pipeline = render_pipeline!(@new_empty);

    let Size {
      col: max_content_display_cols,
      row: max_display_row_count,
    } = self.bounds_size;

    // TK: manage scroll here -> manage_scroll::{detect(), mutate()}
    if let Some(new_scroll_offset) =
      manage_scroll::detect(&self.origin_pos, &self.bounds_size, editor_buffer)
    {
      self.scroll_offset = new_scroll_offset;
    }

    // TK: remove debug
    log_no_err!(
      DEBUG,
      "🟨🟨🟨 self.scroll_offset -> {:?}",
      self.scroll_offset
    );

    // TK: handle vert scroll
    // TK: handle horiz scroll

    // Paint each line in the buffer (skipping the scroll_offset.row).
    // https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.skip
    for (row_index, line) in editor_buffer
      .get_lines()
      .iter()
      .skip(ch!(@to_usize self.scroll_offset.row))
      .enumerate()
    {
      // Clip the content to max rows.
      if ch!(row_index) > max_display_row_count {
        // TK: remove debug
        log_no_err!(
          DEBUG,
          "🟥🟥🟥 row_index {:?} > max_display_row_count {:?}",
          row_index,
          *max_display_row_count
        );
        break;
      }

      // Clip the content to max cols.
      let truncated_line = line.truncate_to_fit_display_cols(max_content_display_cols);
      render_pipeline! {
        @push_into render_pipeline at ZOrder::Normal =>
          RenderOp::MoveCursorPositionRelTo(
            self.origin_pos, position! { col: 0 , row: ch!(@to_usize row_index) }
          ),
          RenderOp::ApplyColors(self.current_box.get_computed_style()),
          RenderOp::PrintTextWithAttributes(truncated_line.into(), self.current_box.get_computed_style()),
          RenderOp::ResetColor
      };

      // TK: remove debug
      log_no_err!(
        DEBUG,
        "🟡🟡🟡 row_index: {:?}, max_display_row_count: {:?}",
        row_index,
        *max_display_row_count
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
      editor_buffer,
      ..
    } = render_args;
    let mut render_pipeline: RenderPipeline = RenderPipeline::default();

    if component_registry
      .has_focus
      .does_current_box_have_focus(&self.current_box)
    {
      // TK: Fix: caret can be painted PAST the bounds of the box!

      match style {
        CaretPaintStyle::GlobalCursor => {
          render_pipeline! {
            @push_into render_pipeline at ZOrder::Caret =>
              RenderOp::RequestShowCaretAtPositionRelTo(self.origin_pos, editor_buffer.get_caret())
          };
        }
        CaretPaintStyle::LocalPaintedEffect => {
          let str_at_caret: String = if let Some(UnicodeStringSegmentSliceResult {
            unicode_string_seg: str_seg,
            ..
          }) = line_buffer_content::string_at_caret(editor_buffer)
          {
            str_seg.string
          } else {
            DEFAULT_CURSOR_CHAR.into()
          };
          render_pipeline! {
            @push_into render_pipeline at ZOrder::Caret =>
            RenderOp::MoveCursorPositionRelTo(self.origin_pos, editor_buffer.get_caret()),
              RenderOp::PrintTextWithAttributes(
                str_at_caret,
                style! { attrib: [reverse] }.into()),
            RenderOp::MoveCursorPositionRelTo(self.origin_pos, editor_buffer.get_caret())
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
        RenderOp::MoveCursorPositionRelTo(self.origin_pos, position! { col: 0 , row: 0 }),
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
            self.origin_pos,
            content_cursor_pos.add_rows_with_bounds(ch!(1), self.bounds_size.row)),
          RenderOp::PrintTextWithAttributes("👀".into(), None)
      };
    }

    render_pipeline
  }
}
