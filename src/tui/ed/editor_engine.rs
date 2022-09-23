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

/// Holds data in between render calls. This is not stored in the [EditorBuffer] struct, which lives
/// in the state.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditorEngine {
  /// The col and row offset for scrolling if active.
  pub scroll_offset: ScrollOffset,
  // TK: 💉✅ hold bounds size & origin pos
  pub origin_pos: Position,
  pub bounds_size: Size,
}

/// Private struct to help keep function signatures smaller.
#[derive(Debug)]
struct RenderArgs<'a> {
  editor_buffer: &'a EditorBuffer,
  style_adj_box_origin_pos: Position,
  style_adj_box_bounds_size: Size,
  has_focus: &'a HasFocus,
  current_box: &'a FlexBox,
}

const DEFAULT_CURSOR_CHAR: char = '▒';

#[derive(Debug)]
enum CaretPaintStyle {
  /// Using cursor show / hide.
  #[allow(dead_code)]
  GlobalCursor,
  /// Painting the editor_buffer.get_caret() position w/ reverse style.
  LocalPaintedEffect,
}

impl EditorEngine {
  // FIXME: impl apply #23
  pub async fn apply<S, A>(
    &mut self, component_registry: &mut ComponentRegistry<S, A>, editor_buffer: &EditorBuffer,
    input_event: &InputEvent, shared_tw_data: &SharedTWData, self_id: &str,
  ) -> CommonResult<Option<EditorBuffer>>
  where
    S: Default + Display + Clone + PartialEq + Debug + Sync + Send,
    A: Default + Display + Clone + Sync + Send,
  {
    // TK: 💉✅ inject bounds_size & origin_pos into editor event, then apply to editor buffer
    if let Some(editor_event) =
      EditorEvent::try_create_from(input_event, self.origin_pos, self.bounds_size)
    {
      let mut new_editor_buffer = editor_buffer.clone();
      EditorBuffer::apply_editor_event(
        &mut new_editor_buffer,
        editor_event,
        shared_tw_data,
        component_registry,
      );
      Ok(Some(new_editor_buffer))
    } else {
      Ok(None)
    }
  }

  // FIXME: impl render #23
  pub async fn render<S, A>(
    &mut self, editor_buffer: &EditorBuffer, component_registry: &mut ComponentRegistry<S, A>,
    current_box: &FlexBox, shared_tw_data: &SharedTWData, self_id: &str,
  ) -> CommonResult<RenderPipeline>
  where
    S: Default + Display + Clone + PartialEq + Debug + Sync + Send,
    A: Default + Display + Clone + Sync + Send,
  {
    throws_with_return!({
      // Create this struct to pass around fewer variables.
      let context = RenderArgs {
        editor_buffer,
        style_adj_box_origin_pos: current_box.style_adjusted_origin_pos, // Adjusted for padding (if set).
        style_adj_box_bounds_size: current_box.style_adjusted_bounds_size, // Adjusted for padding (if set).
        has_focus: &component_registry.has_focus,
        current_box,
      };

      // TK: 💉✅ SAVE current_box::{style_adjusted_origin_pos, style_adjusted_bounds_size} -> EditorEngine
      self.bounds_size = current_box.style_adjusted_bounds_size;
      self.origin_pos = current_box.style_adjusted_origin_pos;

      // TK: remove debug
      log_no_err!(DEBUG, "🟨🟨🟨 current_box -> {:?}", current_box);

      // Save a few variables for apply().
      self.bounds_size = context.style_adj_box_bounds_size;
      self.origin_pos = context.style_adj_box_origin_pos;

      if editor_buffer.is_empty() {
        render_empty_state(&context)
      } else {
        let q_content = self.render_content(&context);
        let q_caret = render_caret(CaretPaintStyle::LocalPaintedEffect, &context);
        render_pipeline!(@join_and_drop q_content, q_caret)
      }
    })
  }

  // This simply clips the content to the `style_adj_box_bounds_size`.
  fn render_content(&mut self, context_ref: &RenderArgs<'_>) -> RenderPipeline {
    let RenderArgs {
      editor_buffer,
      style_adj_box_origin_pos: origin_pos,
      style_adj_box_bounds_size: size,
      current_box,
      ..
    } = context_ref;
    let mut render_pipeline = render_pipeline!(@new_empty);

    let Size {
      col: max_content_display_cols,
      row: max_display_row_count,
    } = size;

    // TK: manage scroll here -> manage_scroll::{detect(), mutate()}
    if let Some(new_scroll_offset) = manage_scroll::detect(origin_pos, size, editor_buffer) {
      self.scroll_offset = new_scroll_offset;
    }

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
      if ch!(row_index) > *max_display_row_count {
        break;
      }

      // Clip the content to max cols.
      let truncated_line = line.truncate_to_fit_display_cols(*max_content_display_cols);
      render_pipeline! {
        @push_into render_pipeline at ZOrder::Normal =>
          RenderOp::MoveCursorPositionRelTo(
            *origin_pos, position! { col: 0 , row: ch!(@to_usize row_index) }
          ),
          RenderOp::ApplyColors(current_box.get_computed_style()),
          RenderOp::PrintTextWithAttributes(truncated_line.into(), current_box.get_computed_style()),
          RenderOp::ResetColor
      };

      // TK: remove debug
      log_no_err!(
        DEBUG,
        "🟡🟡🟡 row_index: {:?}, max_display_row_count: {:?}",
        row_index,
        **max_display_row_count
      );
    }

    render_pipeline
  }
}

/// Implement caret painting using two different strategies represented by [CaretPaintStyle].
fn render_caret(style: CaretPaintStyle, context_ref: &RenderArgs<'_>) -> RenderPipeline {
  let RenderArgs {
    style_adj_box_origin_pos,
    has_focus,
    current_box,
    editor_buffer,
    ..
  } = context_ref;
  let mut render_pipeline: RenderPipeline = RenderPipeline::default();

  if has_focus.does_current_box_have_focus(current_box) {
    // TK: Fix: caret can be painted PAST the bounds of the box!

    match style {
      CaretPaintStyle::GlobalCursor => {
        render_pipeline! {
          @push_into render_pipeline at ZOrder::Caret =>
            RenderOp::RequestShowCaretAtPositionRelTo(*style_adj_box_origin_pos, editor_buffer.get_caret())
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
          RenderOp::MoveCursorPositionRelTo(*style_adj_box_origin_pos, editor_buffer.get_caret()),
            RenderOp::PrintTextWithAttributes(
              str_at_caret,
              style! { attrib: [reverse] }.into()),
          RenderOp::MoveCursorPositionRelTo(*style_adj_box_origin_pos, editor_buffer.get_caret())
        };
      }
    }
  }

  render_pipeline
}

fn render_empty_state(context_ref: &RenderArgs<'_>) -> RenderPipeline {
  let RenderArgs {
    style_adj_box_origin_pos,
    style_adj_box_bounds_size,
    has_focus,
    current_box,
    ..
  } = context_ref;
  let mut render_pipeline: RenderPipeline = RenderPipeline::default();
  let mut content_cursor_pos = position! { col: 0 , row: 0 };

  // Paint the text.
  render_pipeline! {
    @push_into render_pipeline at ZOrder::Normal =>
      RenderOp::MoveCursorPositionRelTo(*style_adj_box_origin_pos, position! { col: 0 , row: 0 }),
      RenderOp::ApplyColors(style! {
        color_fg: TWColor::Red
      }.into()),
      RenderOp::PrintTextWithAttributes("No content added".into(), None),
      RenderOp::ResetColor
  };

  // Paint the emoji.
  if has_focus.does_current_box_have_focus(current_box) {
    render_pipeline! {
      @push_into render_pipeline at ZOrder::Normal =>
        RenderOp::MoveCursorPositionRelTo(
          *style_adj_box_origin_pos,
          content_cursor_pos.add_rows_with_bounds(ch!(1), style_adj_box_bounds_size.row)),
        RenderOp::PrintTextWithAttributes("👀".into(), None)
    };
  }

  render_pipeline
}
