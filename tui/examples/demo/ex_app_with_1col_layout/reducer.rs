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

use async_trait::async_trait;
use r3bl_redux::*;

use super::*;

/// Reducer.
#[derive(Default)]
pub struct Reducer;

#[async_trait]
impl AsyncReducer<State, Action> for Reducer {
  async fn run(&self, action: &Action, state: &State) -> State {
    let State { stack: stack_copy } = &mut state.clone();
    reduce_mut(stack_copy, action);
    State {
      stack: stack_copy.to_vec(),
    }
  }
}

fn reduce_mut(stack: &mut Vec<i32>, action: &Action) {
  match action {
    Action::AddPop(arg) => {
      if stack.is_empty() {
        stack.push(*arg)
      } else {
        let top = stack.pop().unwrap();
        let sum = top + arg;
        stack.push(sum);
      }
    }

    Action::SubPop(arg) => {
      if stack.is_empty() {
        stack.push(*arg)
      } else {
        let top = stack.pop().unwrap();
        let sum = top - arg;
        stack.push(sum);
      }
    }

    Action::Clear => stack.clear(),

    _ => {}
  }
}
