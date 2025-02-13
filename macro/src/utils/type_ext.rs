/*
 *   Copyright (c) 2022-2025 R3BL LLC
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

use quote::ToTokens;
use syn::{punctuated::Punctuated,
          token::Comma,
          GenericArgument,
          Ident,
          Path,
          PathArguments::AngleBracketed,
          Type,
          TypePath,
          TypeReference};

pub trait TypeExtHasIdent {
    fn has_ident(&self) -> bool;
    fn get_ident(&self) -> Option<Ident>;
}

pub trait TypeExtHasGenericArgs {
    fn has_angle_bracketed_generic_args(&self) -> bool;
    fn get_angle_bracketed_generic_args_result(
        &self,
    ) -> Result<Punctuated<GenericArgument, Comma>, ()>;
    fn get_angle_bracketed_generic_args_idents_result(&self) -> Result<Vec<Ident>, ()>;
    fn to_string(&self) -> String;
}

impl TypeExtHasIdent for syn::Type {
    fn has_ident(&self) -> bool {
        match self {
            Type::Path(ref it) => it.get_ident().is_some(),
            Type::Reference(ref it) => it.get_ident().is_some(),
            _ => false,
        }
    }

    fn get_ident(&self) -> Option<Ident> {
        match self {
            Type::Path(ref it) => it.get_ident(),
            Type::Reference(ref it) => it.get_ident(),
            _ => None,
        }
    }
}

impl TypeExtHasIdent for TypePath {
    fn has_ident(&self) -> bool {
        let TypePath {
            path: Path { segments, .. },
            ..
        } = self;
        {
            let ident = &segments.first();
            ident.is_some()
        }
    }

    fn get_ident(&self) -> Option<Ident> {
        if self.has_ident() {
            self.path.segments.first().map(|s| s.ident.clone())
        } else {
            None
        }
    }
}

impl TypeExtHasIdent for TypeReference {
    fn has_ident(&self) -> bool {
        match self.elem.as_ref() {
            Type::Path(ref it) => it.has_ident(),
            _ => false,
        }
    }

    fn get_ident(&self) -> Option<Ident> {
        match self.has_ident() {
            false => None,
            true => {
                let elem = self.elem.as_ref();
                match elem {
                    Type::Path(ref it) => it.get_ident(),
                    _ => None,
                }
            }
        }
    }
}

impl TypeExtHasGenericArgs for syn::Type {
    /// True if self.type_path.path.segments.first().arguments.args.len() to be >
    /// 0.
    fn has_angle_bracketed_generic_args(&self) -> bool {
        match self.get_angle_bracketed_generic_args_result() {
            Ok(generic_args) => !generic_args.is_empty(),
            Err(_) => false,
        }
    }

    /// Ok if self.type_path.path.segments.first().arguments.args exists.
    fn get_angle_bracketed_generic_args_result(
        &self,
    ) -> Result<Punctuated<GenericArgument, Comma>, ()> {
        if let Type::Path(ref type_path) = self {
            let path = &type_path.path;
            let path_arguments = &path.segments.first().unwrap().arguments;

            if let AngleBracketed(ref angle_bracketed_generic_arguments) = path_arguments
            {
                return Ok(angle_bracketed_generic_arguments.args.clone());
            }
        }

        Err(())
    }

    fn get_angle_bracketed_generic_args_idents_result(&self) -> Result<Vec<Ident>, ()> {
        match self.get_angle_bracketed_generic_args_result() {
            Ok(generic_args) => {
                let mut idents = vec![];
                for generic_arg in generic_args {
                    if let GenericArgument::Type(Type::Path(ref type_path)) = generic_arg
                    {
                        let path = &type_path.path;
                        let ident = &path.segments.first().unwrap().ident;
                        idents.push(ident.clone());
                    }
                }
                Ok(idents)
            }
            Err(_) => Err(()),
        }
    }

    fn to_string(&self) -> String { self.to_token_stream().to_string().replace(' ', "") }
}
