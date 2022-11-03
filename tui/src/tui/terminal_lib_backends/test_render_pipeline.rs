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

#[cfg(test)]
mod tests {
  use r3bl_rs_utils_core::*;

  use crate::*;

  #[test]
  fn render_ops_macro() {
    let render_ops = render_ops!(
      @new
      RenderOp::ClearScreen, RenderOp::ResetColor
    );
    assert_eq2!(render_ops.len(), 2);
  }

  #[test]
  fn render_pipeline_macro() {
    // Single pipeline.
    let mut pipeline = render_pipeline!();

    render_pipeline!(
      @push_into pipeline
      at ZOrder::Normal =>
        RenderOp::ClearScreen,
        RenderOp::ResetColor
    );
    assert_eq2!(pipeline.len(), 1);

    let render_ops_set = pipeline.get(&ZOrder::Normal).unwrap();
    assert_eq2!(render_ops_set.len(), 1);

    let render_op_vec = pipeline.get_all_render_op_in(ZOrder::Normal).unwrap();
    assert_eq2!(render_op_vec.len(), 2);
    assert_eq2!(render_op_vec, vec![RenderOp::ClearScreen, RenderOp::ResetColor]);
  }

  #[test]
  fn merge_pipelines() {
    // Merge multiple pipelines.
    let pipeline_1: RenderPipeline = {
      let mut it = render_pipeline!(@new ZOrder::Normal
        =>
          RenderOp::ClearScreen,
          RenderOp::ResetColor
      );

      render_pipeline!(@push_into it at ZOrder::Caret =>
        RenderOp::ResetColor
      );

      assert_eq2!(
        it.get_all_render_op_in(ZOrder::Normal).unwrap(),
        vec![RenderOp::ClearScreen, RenderOp::ResetColor]
      );
      assert_eq2!(
        it.get_all_render_op_in(ZOrder::Caret).unwrap(),
        vec![RenderOp::ResetColor]
      );

      it
    };

    // This is a duplicate of the above pipeline.
    let pipeline_2: RenderPipeline = {
      let it = render_pipeline!(@new ZOrder::Normal
        =>
          RenderOp::ClearScreen,
          RenderOp::ResetColor
      );

      assert_eq2!(
        it.get_all_render_op_in(ZOrder::Normal).unwrap(),
        vec![RenderOp::ClearScreen, RenderOp::ResetColor]
      );

      it
    };

    let _pipeline_merged: RenderPipeline = {
      let pipeline_merged = render_pipeline!(@join_and_drop pipeline_1, pipeline_2);
      assert_eq2!(pipeline_merged.len(), 2);

      let normal_set = pipeline_merged.get(&ZOrder::Normal).unwrap();
      let caret_set = pipeline_merged.get(&ZOrder::Caret).unwrap();

      assert_eq2!(normal_set.len(), 2);
      assert_eq2!(caret_set.len(), 1);

      assert_eq2!(
        pipeline_merged.get_all_render_op_in(ZOrder::Normal).unwrap(),
        vec![
          RenderOp::ClearScreen,
          RenderOp::ResetColor,
          RenderOp::ClearScreen,
          RenderOp::ResetColor
        ]
      );
      assert_eq2!(
        pipeline_merged.get_all_render_op_in(ZOrder::Caret).unwrap(),
        vec![RenderOp::ResetColor]
      );

      pipeline_merged
    };
  }

  #[test]
  fn hoist_z_order_in_pipeline() {
    let mut pipeline = render_pipeline!();

    render_pipeline!(@push_into pipeline at ZOrder::Normal =>
      RenderOp::ClearScreen,
      RenderOp::ResetColor
    );

    pipeline.hoist(ZOrder::Normal, ZOrder::Glass);

    assert_eq2!(pipeline.len(), 1);
    assert_eq2!(pipeline.get(&ZOrder::Normal), None);
    assert_eq2!(pipeline.get_all_render_op_in(ZOrder::Glass).unwrap().len(), 2);
  }
}
