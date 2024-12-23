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

use std::borrow::Cow;

use crate::{SmallStringBackingStore, UnicodeString};

/// UnicodeStringExt trait.
pub trait UnicodeStringExt {
    fn unicode_string(&self) -> UnicodeString;
}

// PERF: [ ] perf
impl UnicodeStringExt for SmallStringBackingStore {
    fn unicode_string(&self) -> UnicodeString { UnicodeString::new(self.as_str()) }
}

// PERF: [ ] perf
impl UnicodeStringExt for &SmallStringBackingStore {
    fn unicode_string(&self) -> UnicodeString { UnicodeString::new(self.as_str()) }
}

impl UnicodeStringExt for Cow<'_, str> {
    fn unicode_string(&self) -> UnicodeString { UnicodeString::new(self) }
}

impl UnicodeStringExt for &str {
    fn unicode_string(&self) -> UnicodeString { UnicodeString::new(self) }
}

impl UnicodeStringExt for &&str {
    fn unicode_string(&self) -> UnicodeString { UnicodeString::new(self) }
}

impl UnicodeStringExt for String {
    fn unicode_string(&self) -> UnicodeString { UnicodeString::new(self) }
}
