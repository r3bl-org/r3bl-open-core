/*
 *   Copyright (c) 2022-2025 R3BL LLC
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

use std::fmt::{Debug, Formatter};

/// Action.
#[derive(Default, Clone, Debug)]
#[non_exhaustive]
#[allow(dead_code)]
pub enum AppSignal {
    Add,
    Sub,
    Clear,
    #[default]
    Noop,
}

/// State.
#[derive(Clone, PartialEq, Eq, Default)]
pub struct State {
    pub counter: isize,
}

impl Debug for State {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "State {{ counter: {:?} }}", self.counter)
    }
}
