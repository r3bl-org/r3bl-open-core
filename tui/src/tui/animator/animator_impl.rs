/*
 *   Copyright (c) 2023 R3BL LLC
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

use std::fmt::Debug;

use r3bl_redux::*;
use tokio::task::JoinHandle;

/// This is a simple animator that can be used to run a single animation task. Animators can be
/// re-used (stopped, and restarted repeatedly).
/// - Once a task is started it can be stopped, but another task can't be started.
/// - After a task is stopped, another one can be started again.
#[derive(Debug, Default)]
pub struct Animator {
    pub animation_task_handle: Option<JoinHandle<()>>,
}

impl Animator {
    /// Starts an animation task if one isn't already running. The animation task is actually
    /// started by calling the `start_animator_fn` function (the `shared_store` is passed to it).
    ///
    /// Arguments:
    /// 1. `shared_store`: An action will presumably be dispatched to the store as the animation
    ///    progresses. Essentially some property in the state will be manipulated over time and the
    ///    action is what will change this property.
    /// 2. `start_animator_task_fn`: This is a function that will start the animation task. It will
    ///    typically spawn a Tokio task and return a handle to it.
    pub fn start<S, A>(
        &mut self,
        shared_store: &SharedStore<S, A>,
        start_animator_task_fn: fn(&SharedStore<S, A>) -> JoinHandle<()>,
    ) where
        S: Default + Clone + PartialEq + Debug + Sync + Send,
        A: Debug + Default + Clone + Sync + Send,
    {
        if self.is_animation_started() {
            return;
        }
        self.animation_task_handle = Some(start_animator_task_fn(shared_store));
    }

    pub fn is_animation_started(&self) -> bool {
        matches!(&self.animation_task_handle, Some(_handle))
    }

    pub fn stop(&mut self) {
        if let Some(handle) = &self.animation_task_handle {
            handle.abort();
        }
        self.animation_task_handle = None;
    }
}
