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
  fn test_serde_tw_color_simple() {
    let color: TWColor = TWColor::Red;
    let ser_str = serde_json::to_string(&color).unwrap();
    let og_color: TWColor = serde_json::from_str(&ser_str).unwrap();
    assert_eq2!(color, og_color);
  }

  #[test]
  fn test_serde_tw_color_rgb() {
    let color = TWColor::Rgb { r: 0, g: 0, b: 0 };
    let ser_str = serde_json::to_string(&color).unwrap();
    let og_color: TWColor = serde_json::from_str(&ser_str).unwrap();
    assert_eq2!(color, og_color);
  }

  #[test]
  fn test_serde_tw_command_queue() {
    let mut q = RenderPipeline::default();
    q.push(&ZOrder::Normal, RenderOp::ClearScreen);
    q.push(&ZOrder::Normal, RenderOp::SetBgColor(TWColor::Red));
    let ser_str = serde_json::to_string_pretty(&q).unwrap();
    println!("{ser_str}");
    let og_q: RenderPipeline = serde_json::from_str(&ser_str).unwrap();
    assert_eq2!(q, og_q);
  }

  #[test]
  fn test_serde_position() {
    let position = position!(col: 0, row:0);
    let ser_str = position.ser_to_string().unwrap();
    let og_position = Position::deser_from_str(&ser_str).unwrap();
    assert_eq2!(position, og_position);
  }

  #[test]
  fn test_serde_size() {
    let size = size!(cols: 0, rows:0);
    let ser_str = size.ser_to_string().unwrap();
    let og_size = Size::deser_from_str(&ser_str).unwrap();
    assert_eq2!(size, og_size);
  }
}
