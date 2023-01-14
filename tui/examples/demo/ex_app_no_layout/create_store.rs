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

use std::fmt::{Display, Formatter};

use async_trait::async_trait;
use r3bl_redux::*;

// Create a new store and attach the reducer.
pub async fn create_store() -> Store<State, Action> {
    let mut store: Store<State, Action> = Store::default();
    store.add_reducer(AppReducer::new()).await;
    store
}

/// Action.
#[derive(Default, Clone, Debug)]
#[non_exhaustive]
#[allow(dead_code)]
pub enum Action {
    Startup,
    AddPop(i32),
    SubPop(i32),
    Clear,
    #[default]
    Noop,
}

impl Display for Action {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { write!(f, "{self:?}") }
}

/// State.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct State {
    pub stack: Vec<i32>,
}

impl Default for State {
    fn default() -> Self { Self { stack: vec![0] } }
}

impl Display for State {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "State {{ stack: {:?} }}", self.stack)
    }
}

/// Reducer.
#[derive(Default)]
pub struct AppReducer;

#[async_trait]
impl AsyncReducer<State, Action> for AppReducer {
    async fn run(&self, action: &Action, state: &mut State) {
        match action {
            Action::AddPop(arg) => {
                if state.stack.is_empty() {
                    state.stack.push(*arg)
                } else {
                    let top = state.stack.pop().unwrap();
                    let sum = top + arg;
                    state.stack.push(sum);
                }
            }

            Action::SubPop(arg) => {
                if state.stack.is_empty() {
                    state.stack.push(*arg)
                } else {
                    let top = state.stack.pop().unwrap();
                    let sum = top - arg;
                    state.stack.push(sum);
                }
            }

            Action::Clear => state.stack = vec![],

            _ => {}
        }
    }
}
