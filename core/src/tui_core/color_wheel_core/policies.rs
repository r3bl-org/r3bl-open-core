/*
 *   Copyright (c) 2024 R3BL LLC
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

use crate::TuiStyle;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum GradientGenerationPolicy {
    /// The first time this method is called it will generate a gradient w/ the number
    /// of steps. Subsequent calls will use the same gradient and index **if** the
    /// number of steps is the same. However, if the number of steps are different,
    /// then a new gradient will be generated & the index reset.
    RegenerateGradientAndIndexBasedOnTextLength,
    /// The first time this method is called it will generate a gradient w/ the number
    /// of steps. Subsequent calls will use the same gradient and index.
    ReuseExistingGradientAndIndex,
    ReuseExistingGradientAndResetIndex,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum TextColorizationPolicy {
    ColorEachCharacter(Option<TuiStyle>),
    ColorEachWord(Option<TuiStyle>),
}
