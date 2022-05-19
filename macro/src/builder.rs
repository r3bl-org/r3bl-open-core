/*
 *   Copyright (c) 2022 R3BL LLC
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

use quote::quote;
use syn::{parse_macro_input, parse_str, DataStruct, DeriveInput, Type};

use super::utils::{data_ext::DataExt,
                   ident_ext::IdentExt,
                   syn_parser_helpers::{transform_named_fields_into_ts,
                                        with_data_struct_make_ts}};

const BUILDER_DOC_URL: &str = "https://rust-lang.github.io/api-guidelines/type-safety.html#builders-enable-construction-of-complex-values-c-builder";

/// Example #1: <https://github.com/dtolnay/syn/blob/master/examples/heapsize/heapsize_derive/src/lib.rs>
/// Example #2: <https://github.com/jonhoo/proc-macro-workshop/blob/master/builder/src/lib.rs>
pub fn derive_proc_macro_impl(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
  let DeriveInput {
    ident: struct_name_ident,
    data,
    generics,
    ..
  }: DeriveInput = parse_macro_input!(input);

  let required_trait_bounds: Vec<&str> = vec!["std::default::Default", "std::fmt::Debug"];

  // Only generate code for struct.
  if data.is_struct() {
    with_data_struct_make_ts(&data, &|data_struct| {
      let builder_name_ident = struct_name_ident.from_string("{}Builder");

      let gen_props_setter_fns_ts =
        transform_named_fields_into_setter_fns_ts(data_struct);

      let gen_props_ts = transform_named_fields_to_props_ts(data_struct);

      let doc_struct_str = format!(
        " Implements the [builder pattern] for [`{}`].\n [builder pattern]: {}",
        &struct_name_ident, BUILDER_DOC_URL
      );

      let gen_props_with_defaults_ts =
        transform_named_fields_to_props_with_defaults_ts(data_struct);

      let new_or_modified_where_clause_ts =
        if does_where_clause_exist(&generics.where_clause) {
          add_trait_bounds_to_existing_where_clause_ts(
            &generics.where_clause,
            &required_trait_bounds,
          )
        } else {
          make_new_where_clause_with_default_trait_bounds_for_named_fields(data_struct)
        };

      let build_set_named_fields_ts = build_fn_set_named_fields_ts(data_struct);

      quote! {
        #[doc = #doc_struct_str]
        impl #generics #builder_name_ident #generics #new_or_modified_where_clause_ts {
          pub fn new() -> Self {
            Self {
              #gen_props_with_defaults_ts
            }
          }

          pub fn build(mut self) -> #struct_name_ident #generics {
            #struct_name_ident {
              #build_set_named_fields_ts
            }
          }

          #gen_props_setter_fns_ts
        }

        pub struct #builder_name_ident #generics #new_or_modified_where_clause_ts {
          #gen_props_ts
        }
      }
    })
  } else {
    quote! {}
  }
  .into()
}

fn build_fn_set_named_fields_ts(data_struct: &DataStruct) -> proc_macro2::TokenStream {
  let build_set_named_fields =
    transform_named_fields_into_ts(data_struct, &|named_field| {
      let field_ident = named_field.ident.as_ref().unwrap();
      // let field_ty = &named_field.ty;
      quote! {
        #field_ident: self.#field_ident,
      }
    });
  build_set_named_fields
}

fn make_new_where_clause_with_default_trait_bounds_for_named_fields(
  data_struct: &DataStruct
) -> proc_macro2::TokenStream {
  let trait_bound_list = transform_named_fields_into_ts(&data_struct, &|named_field| {
    // let field_ident = named_field.ident.as_ref().unwrap();
    let field_ty = &named_field.ty;
    quote! {
      #field_ty: std::default::Default,
    }
  });
  quote! {
    where #trait_bound_list
  }
}

/// Add the `std::default::Default` trait bounds (passed via the `Vec<String>` to the
/// where clause for each type parameter.
///
/// Here's an example of a where clause token stream:
/// ```no_run
/// [
///     Type(
///       PredicateType {
///           lifetimes: None,
///           bounded_ty: Path(
///               TypePath {
///                   qself: None,
///                   path: Path {
///                       leading_colon: None,
///                       segments: [
///                           PathSegment {
///                               ident: Ident {
///                                   ident: "X",
///                                   span: #0 bytes(684..685),
///                               },
///                               arguments: None,
///                           },
///                       ],
///                   },
///               },
///           ),
///           colon_token: Colon,
///           bounds: [...],
///       },
///     ),
///     Comma,
///     Type(
///       PredicateType {...},
///     ),
///     Comma,
/// ]
/// ```
fn add_trait_bounds_to_existing_where_clause_ts(
  where_clause: &Option<syn::WhereClause>,
  traits: &Vec<&str>,
) -> proc_macro2::TokenStream {
  // Must parse the `traits.join("+")` string into a [syn::Type].
  let joined_traits: Type = parse_str(&traits.join(" + ")).unwrap();

  let where_clause_ts = match where_clause {
    Some(where_clause) => {
      let where_predicate_punctuated_list = &where_clause.predicates;

      let modified_where_predicates_ts = where_predicate_punctuated_list
        .iter()
        .map(
          |where_predicate| match where_predicate {
            syn::WherePredicate::Type(_) => {
              quote! { #where_predicate + #joined_traits }
            }
            _ => quote! {},
          },
        )
        .collect::<Vec<_>>();

      quote! { where #(#modified_where_predicates_ts),* }
    }
    None => {
      quote! {}
    }
  };

  return where_clause_ts;
}

fn does_where_clause_exist(where_clause: &Option<syn::WhereClause>) -> bool {
  match where_clause {
    Some(_) => true,
    None => false,
  }
}

/// Given named fields, generate props w/ defaults for the <Foo>Builder impl block.
/// Returns [proc_macro2::TokenStream] (not [proc_macro::TokenStream]).
fn transform_named_fields_to_props_with_defaults_ts(
  data_struct: &DataStruct
) -> proc_macro2::TokenStream {
  transform_named_fields_into_ts(data_struct, &|named_field| {
    let field_ident = named_field.ident.as_ref().unwrap();
    // let field_ty = &named_field.ty;
    quote! {
      #field_ident: Default::default(),
    }
  })
}

/// Given named fields, generate props for the <Foo>Builder struct block.
/// Returns [proc_macro2::TokenStream] (not [proc_macro::TokenStream]).
fn transform_named_fields_to_props_ts(
  data_struct: &DataStruct
) -> proc_macro2::TokenStream {
  transform_named_fields_into_ts(data_struct, &|named_field| {
    let field_ident = named_field.ident.as_ref().unwrap();
    let field_ty = &named_field.ty;
    quote! {
      pub #field_ident: #field_ty,
    }
  })
}

/// Given named fields, generate functions for the <Foo>Builder impl block.
/// Returns [proc_macro2::TokenStream] (not [proc_macro::TokenStream]).
fn transform_named_fields_into_setter_fns_ts(
  data_struct: &DataStruct
) -> proc_macro2::TokenStream {
  transform_named_fields_into_ts(data_struct, &|named_field| {
    let field_ident = named_field.ident.as_ref().unwrap();
    let fn_name_ident = field_ident.from_string("set_{}");
    let arg_ty = &named_field.ty;
    quote! {
      pub fn #fn_name_ident(mut self, value: #arg_ty) -> Self {
        self.#field_ident = value;
        self
      }
    }
  })
}
