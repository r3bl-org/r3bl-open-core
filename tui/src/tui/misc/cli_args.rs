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

/// Helper trait and impl to convert [std::env::Args] to a [`Vec<String>`] after removing the first
/// item (which is the path to the executable).
pub trait ArgsToStrings {
    fn filter_and_convert_to_strings(&self) -> Vec<String>;
    fn as_str(my_vec: &[String]) -> Vec<&str>;
}

impl ArgsToStrings for std::env::Args {
    fn filter_and_convert_to_strings(&self) -> Vec<String> {
        let mut list = std::env::args().collect::<Vec<String>>();
        if !list.is_empty() {
            list.remove(0);
        }
        list
    }

    fn as_str(my_vec: &[String]) -> Vec<&str> {
        my_vec.iter().map(String::as_str).collect::<Vec<&str>>()
    }
}
