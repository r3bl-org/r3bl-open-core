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

use r3bl_rs_utils_core::{throws, CommonResult};
use tokio::sync::mpsc::Sender;

use crate::TerminalWindowMainThreadSignal;

/// This is a simple animator that can be used to run a single animation task. Animators
/// can be re-used (stopped, and restarted repeatedly).
/// - Once a task is started it can be stopped, but another task can't be started.
/// - After a task is stopped, another one can be started again.
#[derive(Debug, Default)]
pub struct Animator {
    /// This is the channel that will be used to kill the animation task.
    /// - [None] means that the animation task is not running.
    /// - When an animation task is started, this will have a [Some] value in it.
    ///
    /// The [Animator::stop](Animator::stop) function uses this channel to kill the
    /// animation task.
    pub animator_kill_channel: Option<Sender<()>>,
}

impl Animator {
    /// Starts an animation task if one isn't already running. The animation task is
    /// actually started by calling the `start_animator_task` callback function. The main
    /// thread signal channel is passed to this callback function. This allows the
    /// animator to communicate with the main thread if needed, for example, to ask for
    /// re-renders.
    ///
    /// Arguments:
    /// 1. `channel_sender`: An action will presumably be dispatched to the app as the
    ///    animation progresses. Essentially some property in the state will be
    ///    manipulated over time and the action is what will change this property.
    /// 2. `start_animator_task_fn`: This is a function that will start the animation
    ///    task. It will typically spawn a Tokio task and return a handle to it.
    pub fn start<AS>(
        &mut self,
        channel_sender: Sender<TerminalWindowMainThreadSignal<AS>>,
        start_animator_task: fn(Sender<TerminalWindowMainThreadSignal<AS>>) -> Sender<()>,
    ) where
        AS: Debug + Default + Clone + Sync + Send,
    {
        if self.is_animation_started() {
            return;
        }
        self.animator_kill_channel = Some(start_animator_task(channel_sender));
    }

    pub fn is_animation_started(&self) -> bool {
        matches!(&self.animator_kill_channel, Some(_handle))
    }

    pub fn is_animation_not_started(&self) -> bool { !self.is_animation_started() }

    pub fn stop(&mut self) -> CommonResult<()> {
        throws!({
            if let Some(kill_channel) = &self.animator_kill_channel {
                let kill_channel_clone = kill_channel.clone();
                tokio::spawn(async move {
                    let _ = kill_channel_clone.send(()).await;
                });
                self.animator_kill_channel = None;
            }
        });
    }
}
