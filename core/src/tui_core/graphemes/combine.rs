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

use std::ops::Add;

use crate::{UnicodeString, UnicodeStringExt};

impl Add<&str> for UnicodeString {
    type Output = Self;

    fn add(self, rhs: &str) -> Self::Output {
        let mut new_string = self.string;
        new_string.push_str(rhs);
        // PERF: [ ] perf
        new_string.unicode_string()
    }
}

impl Add<&UnicodeString> for UnicodeString {
    type Output = Self;

    fn add(self, rhs: &UnicodeString) -> Self::Output {
        let mut new_string = self.string;
        new_string.push_str(&rhs.string);
        // PERF: [ ] perf
        new_string.unicode_string()
    }
}
