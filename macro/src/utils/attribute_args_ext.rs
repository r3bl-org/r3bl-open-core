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

use crate::utils::{meta_ext::MetaExt, nested_meta_ext::NestedMeta};

pub trait AttributeArgsExt {
    fn get_key_value_pair(&self) -> (String, String);
}

/// The args take a key value pair like `#[attrib_macro_logger(key = "value")]`,
/// which evaluates to:
/// ```ignore
/// &args = [
///     Meta(
///         NameValue(
///             MetaNameValue {
///                 path: Path {
///                     leading_colon: None,
///                     segments: [
///                         PathSegment {
///                             ident: Ident {
///                                 ident: "key",
///                                 span: #0 bytes(510..513),
///                             },
///                             arguments: None,
///                         },
///                     ],
///                 },
///                 eq_token: Eq,
///                 lit: Str(
///                     LitStr {
///                         token: "value",
///                     },
///                 ),
///             },
///         ),
///     ),
/// ]
/// ```
impl AttributeArgsExt for syn::AttributeArgs {
    fn get_key_value_pair(&self) -> (String, String) {
        for nested_meta in self.iter() {
            if nested_meta.is_meta() {
                let meta = nested_meta.get_meta();
                if meta.is_meta_name_value() {
                    let key = meta.get_meta_name_value_ident().to_string();
                    let value = meta.get_meta_name_value_str();
                    return (key, value);
                }
            }
        }
        panic!("Expected a key value pair");
    }
}
