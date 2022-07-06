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

pub trait IdentExt {
  fn create_from_string(&self, string: &str) -> Self;
  fn as_str(&self) -> String;
}

impl IdentExt for proc_macro2::Ident {
  /// Generates a new identifier using the given string template as the name and
  /// the span from the `self` [Ident]. The template string can contain `{}`
  /// placeholders for the `self` [Ident] name.
  fn create_from_string(&self, name_with_template_placeholder: &str) -> Self {
    let name = str::replace(name_with_template_placeholder, "{}", &self.to_string());
    proc_macro2::Ident::new(&name, self.span())
  }

  fn as_str(&self) -> String { std::string::ToString::to_string(&self) }
}