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

use r3bl_rs_utils::{assert_eq2, ch, ChUnit};

#[test]
fn test_from_whatever_into_ch() {
  let ch_1: ChUnit = ch!(1);
  assert_eq2!(*ch_1, 1);

  let ch_2: ChUnit = ch!(1, @inc);
  assert_eq2!(*ch_2, 2);

  let ch_3: ChUnit = ch!(1, @dec);
  assert_eq2!(*ch_3, 0);

  let ch_4: ChUnit = ch!(0, @dec);
  assert_eq2!(*ch_4, 0);
}

#[test]
fn test_from_ch_into_usize() {
  let usize_1: usize = ch!(@to_usize ch!(1));
  assert_eq2!(usize_1, 1);

  let usize_2: usize = ch!(@to_usize ch!(1), @inc);
  assert_eq2!(usize_2, 2);

  let usize_3: usize = ch!(@to_usize ch!(1), @dec);
  assert_eq2!(usize_3, 0);

  let usize_4: usize = ch!(@to_usize ch!(0), @dec);
  assert_eq2!(usize_4, 0);
}

#[test]
fn test_from_ch_into_u16() {
  let u16_1: u16 = ch!(@to_u16 ch!(1));
  assert_eq2!(u16_1, 1);

  let u16_2: u16 = ch!(@to_u16 ch!(1), @inc);
  assert_eq2!(u16_2, 2);

  let u16_3: u16 = ch!(@to_u16 ch!(1), @dec);
  assert_eq2!(u16_3, 0);

  let u16_4: u16 = ch!(@to_u16 ch!(0), @dec);
  assert_eq2!(u16_4, 0);
}
