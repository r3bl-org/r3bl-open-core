/*
 *   Copyright (c) 2022 Nazmul Idris
 *   All rights reserved.

 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at

 *   http://www.apache.org/licenses/LICENSE-2.0

 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
*/

use core::panic;

use syn::Ident;

pub trait MetaExt {
  fn is_meta_name_value(&self) -> bool;
  fn get_meta_name_value_str(&self) -> String;
  fn get_meta_name_value_ident(&self) -> Ident;
}

/// Can be either a 👉 [syn::Meta::NameValue], [syn::Meta::List], or [syn::Meta::Path].
impl MetaExt for syn::Meta {
  fn is_meta_name_value(&self) -> bool {
    match self {
      syn::Meta::Path(_) => false,
      syn::Meta::List(_) => false,
      syn::Meta::NameValue(_) => true,
    }
  }

  fn get_meta_name_value_str(&self) -> String {
    match self {
      syn::Meta::Path(_) => panic!("Path found"),
      syn::Meta::List(_) => panic!("List found"),
      syn::Meta::NameValue(meta_name_value) => {
        let lit_str = match &meta_name_value.lit {
          syn::Lit::Str(lit_str) => lit_str.value(),
          _ => panic!("Expected a string literal"),
        };
        lit_str
      }
    }
  }

  /// ```no_run
  /// Path {
  ///   leading_colon: None,
  ///   segments: [
  ///       PathSegment {
  ///           ident: Ident {
  ///               ident: "key",
  ///               span: #0 bytes(510..513),
  ///           },
  ///           arguments: None,
  ///       },
  ///   ],
  /// }
  /// ```
  fn get_meta_name_value_ident(&self) -> Ident {
    match self {
      syn::Meta::Path(_) => panic!("Path found"),
      syn::Meta::List(_) => panic!("List found"),
      syn::Meta::NameValue(meta_name_value) => {
        if let Some(ident) = meta_name_value.path.get_ident() {
          ident.clone()
        } else {
          panic!("Expected an ident")
        }
      }
    }
  }
}
