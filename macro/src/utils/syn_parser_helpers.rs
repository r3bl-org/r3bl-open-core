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

use quote::quote;
use syn::{Data::Struct, DataStruct, Fields::Named};

/// Returns [proc_macro2::TokenStream] (not [proc_macro::TokenStream]).
pub fn transform_named_fields_into_ts(
  data_struct: &DataStruct, transform_named_field_fn: &dyn Fn(&syn::Field) -> proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
  match data_struct.fields {
    Named(ref fields) => {
      // Create iterator over named fields, holding generated props token streams.
      let props_ts_iter = fields.named.iter().map(|named_field| transform_named_field_fn(named_field));

      // Unwrap iterator into a [proc_macro2::TokenStream].
      quote! {
        #(#props_ts_iter)*
      }
    }
    _ => quote! {},
  }
}

/// If [syn::Data] contains [syn::DataStruct] then parse it, and generate a
/// [proc_macro2::TokenStream] and return it.
pub fn with_data_struct_make_ts(
  data: &syn::Data, data_struct_transform_fn: &dyn Fn(&syn::DataStruct) -> proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
  match data {
    Struct(ref data_struct) => data_struct_transform_fn(data_struct),
    _ => quote! {},
  }
}
