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

use quote::{quote, ToTokens};
use syn::{Ident, PathArguments, Type, TypePath};

use crate::utils::{IdentExt, TypeExtHasIdent};

#[derive(Debug)]
pub struct IdentRef {
    pub ident: Ident,
    pub is_ref: bool,
}

pub fn get_fn_input_args_ident_ref_from_fn_ty(property_fn_type: &Type) -> Vec<IdentRef> {
    let mut args: Vec<IdentRef> = Vec::new();
    if let Type::Path(type_path) = property_fn_type {
        handle_type_path(type_path, &mut args);
    }
    args
}

pub fn handle_type_path(type_path: &TypePath, args: &mut Vec<IdentRef>) {
    if type_path.path.segments.first().is_some() {
        let path_segment = type_path.path.segments.first().unwrap();
        let path_arguments = &path_segment.arguments;
        if let PathArguments::Parenthesized(p_g_args) = path_arguments {
            let inputs = &p_g_args.inputs;
            inputs.iter().for_each(|type_item| match type_item {
                Type::Path(it) => {
                    if it.has_ident() {
                        let ident = it.get_ident().unwrap();
                        let is_ref = false;
                        args.push(IdentRef { ident, is_ref });
                    }
                }
                Type::Reference(it) => {
                    if it.has_ident() {
                        let ident = it.get_ident().unwrap();
                        let is_ref = true;
                        args.push(IdentRef { ident, is_ref });
                    }
                }
                _ => {}
            })
        }
    }
}

pub fn gen_fn_input_args_expr_list(
    fn_arg_type_list: &[IdentRef],
) -> (Vec<proc_macro2::TokenStream>, Vec<Ident>) {
    let mut count = 0;
    let mut arg_name_ident_vec: Vec<Ident> = Vec::new();
    let arg_with_type_vec: Vec<proc_macro2::TokenStream> = fn_arg_type_list
        .iter()
        .map(|arg_ty_ident_ref| {
            count += 1;
            let arg_name_ident: Ident = arg_ty_ident_ref
                .ident
                .create_from_string(&format!("arg{count}"));
            arg_name_ident_vec.push(arg_name_ident.clone());

            let arg_ty_ident = arg_ty_ident_ref.ident.clone();

            if arg_ty_ident_ref.is_ref {
                quote! { #arg_name_ident: &#arg_ty_ident }
            } else {
                quote! { #arg_name_ident: #arg_ty_ident }
            }
        })
        .collect::<Vec<proc_macro2::TokenStream>>();
    (arg_with_type_vec, arg_name_ident_vec)
}

pub fn get_fn_output_type_from(
    property_fn_type: &Type,
) -> Option<proc_macro2::TokenStream> {
    if let Type::Path(type_path) = property_fn_type {
        if type_path.path.segments.first().is_some() {
            let path_segment = type_path.path.segments.first().unwrap();
            let path_arguments = &path_segment.arguments;
            if let PathArguments::Parenthesized(p_g_args) = path_arguments {
                let output = &p_g_args.output;
                if let syn::ReturnType::Type(_, return_ty) = output {
                    return Some(return_ty.to_token_stream());
                }
            }
        }
    }
    None
}
