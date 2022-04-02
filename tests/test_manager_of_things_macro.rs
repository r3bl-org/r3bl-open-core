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

//! # Watch macro expansion
//!
//! To watch for changes run this script:
//! `./cargo-watch-macro-expand-one-test.fish test_manager_of_things_macro`
//!
//! # Watch test output
//!
//! To watch for test output run this script:
//! `./cargo-watch-one-test.fish test_manager_of_things_macro`

use std::collections::HashMap;

use r3bl_rs_utils_macro::make_struct_safe_to_share_and_mutate;

#[tokio::test]
async fn test_custom_syntax_full() {
  make_struct_safe_to_share_and_mutate! {
    named MyMapManager<K, V>
    where K: Default + Send + Sync + 'static, V: Default + Send + Sync + 'static
    containing my_map
    of_type std::collections::HashMap<K, V>
  }

  // Create an instance of the "manager" struct.
  let my_manager: MyMapManager<String, String> = MyMapManager::default();

  // 🔒 Each of the locked objects need to be wrapped in a block, or call `drop()` so the
  // mutex guard can be dropped and the tests won't deadlock.

  // 1. Test that `my_map` is created.
  let locked_map = my_manager.get_value().await;
  assert_eq!(locked_map.len(), 0);
  drop(locked_map);

  // 2. Test that `get_ref()` => works
  //    - 🔒 `with_arc_get_locked_thing()`
  //    - 🔒 `with_ref_ge_lock_readable()`
  //    - `with_ref_set_value()`
  let arc_clone = my_manager.get_ref();

  let locked_map = MyMapManager::with_ref_get_value_w_lock(&arc_clone).await;
  assert_eq!(locked_map.len(), 0);
  drop(locked_map); // 🔒 Prevents deadlock below.

  let map: HashMap<String, String> = HashMap::new();
  MyMapManager::with_ref_set_value(&arc_clone, map).await;
  assert_eq!(
    MyMapManager::with_ref_get_value_r_lock(&arc_clone)
      .await
      .len(),
    0
  );

  let map: HashMap<String, String> = HashMap::new();
  my_manager.set_value(map).await;
  assert_eq!(
    my_manager
      .my_map
      .read()
      .await
      .len(),
    0
  );
}

#[tokio::test]
async fn test_custom_syntax_no_where_clause() {
  make_struct_safe_to_share_and_mutate! {
    named StringMap<K, V>
    // where is optional and is missing here.
    containing my_map
    of_type std::collections::HashMap<K, V>
  }

  let my_manager: StringMap<String, String> = StringMap::default();
  let locked_map = my_manager.my_map.read().await;
  assert_eq!(locked_map.len(), 0);
  drop(locked_map);
}

#[test]
fn test_simple_expansion() {
  make_struct_safe_to_share_and_mutate! {
    named MyMapManager<K, V>
    where K: Default + Send + Sync + 'static, V: Default + Send + Sync + 'static
    containing my_map
    of_type std::collections::HashMap<K, V>
  }
}
