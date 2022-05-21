/*
 *   Copyright (c) 2022 R3BL LLC
 *   All rights reserved.

 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at

 *   http://www.apache.org/licenses/LICENSE-2.0

 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
*/

use r3bl_rs_utils::unwrap_option_or_compute_if_none;

#[test]
fn test_unwrap_option_or_compute_if_none() {
  struct MyStruct {
    field: Option<i32>,
  }
  let mut my_struct = MyStruct { field: None };
  assert_eq!(my_struct.field, None);
  unwrap_option_or_compute_if_none!(my_struct.field, { || 1 });
  assert_eq!(my_struct.field, Some(1));
}
