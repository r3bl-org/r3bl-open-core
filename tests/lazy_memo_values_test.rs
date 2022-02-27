/*
 Copyright 2022 R3BL LLC

 Licensed under the Apache License, Version 2.0 (the "License");
 you may not use this file except in compliance with the License.
 You may obtain a copy of the License at

      https://www.apache.org/licenses/LICENSE-2.0

 Unless required by applicable law or agreed to in writing, software
 distributed under the License is distributed on an "AS IS" BASIS,
 WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 See the License for the specific language governing permissions and
 limitations under the License.
*/

//! Integration tests for the `lazy` module.

use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};

use r3bl_rs_utils::utils::LazyMemoValues;

#[test]
fn test_lazy() {
  // These are copied in the closure below.
  let arc_atomic_count = AtomicUsize::new(0);
  let mut a_variable = 123;
  let mut a_flag = false;

  let mut generate_value_fn = LazyMemoValues::new(|it| {
    arc_atomic_count.fetch_add(1, SeqCst);
    a_variable = 12;
    a_flag = true;
    a_variable + it
  });

  assert_eq!(arc_atomic_count.load(SeqCst), 0);
  assert_eq!(generate_value_fn.get_ref(&1), &13);
  assert_eq!(arc_atomic_count.load(SeqCst), 1);
  assert_eq!(generate_value_fn.get_ref(&1), &13); // Won't regenerate the value.
  assert_eq!(arc_atomic_count.load(SeqCst), 1); // Doesn't change.

  assert_eq!(generate_value_fn.get_ref(&2), &14);
  assert_eq!(arc_atomic_count.load(SeqCst), 2);
  assert_eq!(generate_value_fn.get_ref(&2), &14);
  assert_eq!(generate_value_fn.get_copy(&2), 14);

  assert_eq!(a_variable, 12);
  assert_eq!(a_flag, true);
}
