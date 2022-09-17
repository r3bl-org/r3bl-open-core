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

use r3bl_rs_utils::*;

#[test]
fn test_create_render_pipeline() {
  // Single pipeline.
  let mut pipeline = render_pipeline!(@new_empty);

  render_pipeline!(@push_into pipeline at ZOrder::Normal =>
    RenderOp::ClearScreen,
    RenderOp::ResetColor
  );

  assert_eq2!(pipeline.len(), 1);
  let z_order_len = pipeline.get(&ZOrder::Normal).unwrap().list.len();
  assert_eq2!(z_order_len, 2);

  let first_item = pipeline.get(&ZOrder::Normal).unwrap().list.first().unwrap();
  assert_eq2!(first_item, &RenderOp::ClearScreen);

  let last_item = pipeline.get(&ZOrder::Normal).unwrap().list.last().unwrap();
  assert_eq2!(last_item, &RenderOp::ResetColor);

  // Merge multiple pipelines.
  let pipeline_1: RenderPipeline = {
    let mut p = render_pipeline!(@new ZOrder::Normal
      =>
        RenderOp::ClearScreen,
        RenderOp::ResetColor
    );
    render_pipeline!(@push_into p at ZOrder::Caret =>
      RenderOp::ResetColor
    );
    p
  };

  let pipeline_2: RenderPipeline = render_pipeline!(@new ZOrder::Normal
    =>
      RenderOp::ClearScreen,
      RenderOp::ResetColor
  );

  let pipeline_merged = render_pipeline!(@join_and_drop pipeline_1, pipeline_2);
  assert_eq2!(pipeline_merged.len(), 2);
  assert_eq2!(pipeline_merged.get(&ZOrder::Normal).unwrap().list.len(), 4);
  assert_eq2!(pipeline_merged.get(&ZOrder::Caret).unwrap().list.len(), 1);
}
