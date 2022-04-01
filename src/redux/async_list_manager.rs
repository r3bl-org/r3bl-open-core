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

use my_proc_macros_lib::make_struct_safe_to_share_and_mutate;

make_struct_safe_to_share_and_mutate! {
  named SafeListManager<T>
  where T: Sync + Send + 'static
  containing list
  of_type Vec<T>
}

impl<T> SafeListManager<T>
where
  T: Sync + Send + 'static,
{
  pub async fn push(
    &mut self,
    item: T,
  ) {
    let arc = self.get_ref();
    let mut locked_list = arc.write().await;
    locked_list.push(item);
  }

  pub async fn clear(&mut self) {
    let arc = self.get_ref();
    let mut locked_list = arc.write().await;
    locked_list.clear();
  }
}

// Define macros.
// https://stackoverflow.com/questions/28953262/pass-member-function-body-as-macro-parameter
// https://cheats.rs/#tooling-directives
// https://dhghomon.github.io/easy_rust/Chapter_61.html
// https://stackoverflow.com/questions/26731243/how-do-i-use-a-macro-across-module-files

/// The `$lambda` expression is not async.
macro_rules! iterate_over_vec_with {
  ($this:ident, $locked_list_arc:expr, $lambda:expr) => {
    let locked_list = $locked_list_arc.get_ref();
    let list_r = locked_list.read().await;
    for item_fn in list_r.iter() {
      $lambda(&item_fn);
    }
  };
}

/// The `$lambda` expression is async.
macro_rules! iterate_over_vec_with_async {
  ($this:ident, $locked_list_arc:expr, $lambda:expr) => {
    let locked_list = $locked_list_arc.get_ref();
    let list_r = locked_list.read().await;
    for item_fn in list_r.iter() {
      $lambda(&item_fn).await;
    }
  };
}

/// The `$lambda` expression is async.
macro_rules! iterate_over_vec_with_results_async {
  ($locked_list_arc:expr, $lambda:expr, $results:ident) => {
    let locked_list = $locked_list_arc.get_ref();
    let list_r = locked_list.read().await;
    for item_fn in list_r.iter() {
      let result = $lambda(&item_fn).await;
      if let Ok(action) = result {
        if let Some(action) = action {
          $results.push(action);
        }
      };
    }
    // The following line would create a function that returns from the block where its used.
    // return $results;
  };
}

pub(crate) use iterate_over_vec_with;
pub(crate) use iterate_over_vec_with_async;
pub(crate) use iterate_over_vec_with_results_async;
