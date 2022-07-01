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

use core::panic;
pub trait NestedMeta {
  fn is_meta(&self) -> bool;
  fn get_meta(&self) -> &syn::Meta;
}

/// Can be either a 👉 [syn::NestedMeta::Meta] or a [syn::NestedMeta::Lit].
impl NestedMeta for syn::NestedMeta {
  fn is_meta(&self) -> bool {
    match self {
      syn::NestedMeta::Meta(_) => true,
      syn::NestedMeta::Lit(_) => false,
    }
  }

  fn get_meta(&self) -> &syn::Meta {
    match self {
      syn::NestedMeta::Meta(meta) => meta,
      syn::NestedMeta::Lit(_) => panic!("Lit found"),
    }
  }
}
