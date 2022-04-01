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

use my_proc_macros_lib::make_a_manager_named;

#[tokio::test]
async fn test_custom_syntax_full() {
  make_a_manager_named! {
    MyMapManager<K, V>
    where K: Default + Send + Sync + 'static, V: Default + Send + Sync + 'static
    for my_map
    as type std::collections::HashMap<K, V>
  }

  // Create an instance of the "manager" struct.
  let my_manager: MyMapManager<String, String> = MyMapManager::default();

  // 🔒 Each of the locked objects need to be wrapped in a block, or call `drop()` so the
  // mutex guard can be dropped and the tests won't deadlock.

  // 1. Test that `my_map` is created.
  let locked_map = my_manager.my_map.read().await;
  assert_eq!(locked_map.len(), 0);
  drop(locked_map);

  // 2. Test that `get_arc()` => works
  //    - 🔒 `with_arc_get_locked_thing()`
  //    - 🔒 `with_arc_get_locked_thing_r()`
  //    - `with_arc_set_value_of_wrapped_thing()`
  let arc_clone = my_manager.get_arc();

  let locked_map = MyMapManager::with_arc_get_locked_thing_w(&arc_clone).await;
  assert_eq!(locked_map.len(), 0);
  drop(locked_map); // 🔒 Prevents deadlock below.

  let map: HashMap<String, String> = HashMap::new();
  MyMapManager::with_arc_set_value_of_wrapped_thing(&arc_clone, map).await;
  assert_eq!(
    MyMapManager::with_arc_get_locked_thing_r(&arc_clone)
      .await
      .len(),
    0
  );
}

#[tokio::test]
async fn test_custom_syntax_no_where_clause() {
  make_a_manager_named! {
    StringMap<K, V>
    // where is optional and is missing here.
    for my_map
    as type std::collections::HashMap<K, V>
  }

  // Create an instance of the "manager" struct.
  let my_manager: StringMap<String, String> = StringMap::default();

  // 🔒 Each of the locked objects need to be wrapped in a block, or call `drop()` so the
  // mutex guard can be dropped and the tests won't deadlock.

  // 1. Test that `my_map` is created.
  let locked_map = my_manager.my_map.read().await;
  assert_eq!(locked_map.len(), 0);
  drop(locked_map);

  // 2. Test that `get_arc()` => works
  //    - 🔒 `with_arc_get_locked_thing()`
  //    - 🔒 `with_arc_get_locked_thing_r()`
  //    - `with_arc_set_value_of_wrapped_thing()`
  let arc_clone = my_manager.get_arc();

  let locked_map = StringMap::with_arc_get_locked_thing_w(&arc_clone).await;
  assert_eq!(locked_map.len(), 0);
  drop(locked_map); // 🔒 Prevents deadlock below.

  let map: HashMap<String, String> = HashMap::new();
  StringMap::with_arc_set_value_of_wrapped_thing(&arc_clone, map).await;
  assert_eq!(
    StringMap::with_arc_get_locked_thing_r(&arc_clone)
      .await
      .len(),
    0
  );
}
