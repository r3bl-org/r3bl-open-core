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

#[tokio::test]
async fn test_add_box_size_to_pos() {
  // [10, 10] + [30, 10] = [40, 20]
  let pos = position!(col: 10, row: 10);
  let size = size!(col: 30, row: 10);
  let new_pos = pos + size; // `size + pos` is not defined.
  assert_eq!(new_pos.col, 40);
  assert_eq!(new_pos.row, 20);
}

#[tokio::test]
async fn test_mul_box_pos_to_pair() {
  // [30, 10] * [1, 0] = [30, 0]
  {
    let pos: Position = position!(col: 30, row: 10);
    let pair_cancel_row = (1, 0);
    let new_pair = pos * pair_cancel_row;
    assert_eq!(new_pair.col, 30);
    assert_eq!(new_pair.row, 0);
  }

  // [30, 10] * [0, 1] = [0, 10]
  {
    let pos: Position = position!(col: 30, row: 10);
    let pair_cancel_col = (0, 1);
    let new_pair = pos * pair_cancel_col;
    assert_eq!(new_pair.col, 0);
    assert_eq!(new_pair.row, 10);
  }
}

#[test]
fn test_percent_works_as_expected() {
  let maybe_pc_100: Result<Percent, String> = percent!(100i32);
  if let Ok(pc_100) = maybe_pc_100 {
    assert_eq!(*pc_100, 100);
    let result = calc_percentage(pc_100, 500);
    assert_eq!(result, 500);
  } else {
    panic!("Failed to create Percent from 100");
  }

  let pc_50 = Percent::try_from(50i32).unwrap();
  assert_eq!(*pc_50, 50);
  let result = calc_percentage(pc_50, 500);
  assert_eq!(result, 250);

  let pc_0 = Percent::try_from(0i32).unwrap();
  assert_eq!(*pc_0, 0);
  let result = calc_percentage(pc_0, 500);
  assert_eq!(result, 0);
}

#[test]
fn test_percent_parsing_fails_as_expected() {
  Percent::try_from(-1i32).unwrap_err();

  Percent::try_from(0i32).unwrap();
  Percent::try_from(0u16).unwrap();

  Percent::try_from(100i32).unwrap();
  Percent::try_from(100u16).unwrap();

  Percent::try_from(101i32).unwrap_err();
  Percent::try_from(101u16).unwrap_err();
}
