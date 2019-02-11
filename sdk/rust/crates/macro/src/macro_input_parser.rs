/*
 * Copyright 2018 Fluence Labs Limited
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use quote::quote;
use syn::{parse::Error, spanned::Spanned};

pub enum ParsedType {
    Utf8String,
    ByteVector,
}

impl ParsedType {
    pub fn from_type(input_type: &syn::Type) -> syn::Result<ParsedType> {
        // parses generic param T in Vec<T> to string representation
        fn parse_vec_bracket(args: &syn::PathArguments) -> syn::Result<String> {
            // checks that T is angle bracketed
            let generic_arg = match args {
                syn::PathArguments::AngleBracketed(args) => Ok(args),
                _ => Err(Error::new(
                    args.span(),
                    "It has to be a bracketed value after Vec",
                )),
            }?;

            let arg = generic_arg.args.first().ok_or_else(|| {
                Error::new(
                    generic_arg.span(),
                    "It has to be a valid generic value in Vec brackets",
                )
            })?;
            let arg_val = arg.value();

            // converts T to syn::Type
            let arg_type = match arg_val {
                syn::GenericArgument::Type(ty) => Ok(ty),
                _ => Err(Error::new(arg_val.span(), "Incorrect type in Vec brackets")),
            }?;

            // converts T to syn::path
            let arg_path = match arg_type {
                syn::Type::Path(path) => Ok(&path.path),
                _ => Err(Error::new(
                    arg_type.span(),
                    "Unsuitable type in Vec brackets - only Vec<u8> is supported",
                )),
            }?;

            // There could be situations like Vec<some_crate::some_module::u8>
            // that why we check segments count
            if arg_path.segments.len() != 1 {
                return Err(Error::new(
                    arg_path.span(),
                    "Unsuitable type in Vec brackets - only Vec<u8> is supported",
                ));
            }

            // converts T to String
            let arg_segment = arg_path.segments.first().ok_or_else(|| {
                Error::new(
                    arg_path.span(),
                    "It has to be a valid generic value in Vec brackets",
                )
            })?;
            let arg_segment = arg_segment.value();

            Ok(arg_segment.ident.to_string())
        }

        let path = match input_type {
            syn::Type::Path(path) => Ok(&path.path),
            _ => Err(Error::new(
                input_type.span(),
                "Incorrect argument type - only Vec<u8> and String are supported",
            )),
        }?;

        let type_segment = path
            .segments
            // argument can be given in full path form: ::std::string::String
            // that why the last one used
            .last()
            .ok_or_else(|| {
                Error::new(
                    path.span(),
                    "It has to have a non-empty input argument type",
                )
            })?;
        let type_segment = type_segment.value();

        match type_segment.ident.to_string().as_str() {
            "String" => Ok(ParsedType::Utf8String),
            "Vec" => match parse_vec_bracket(&type_segment.arguments) {
                Ok(value) => match value.as_str() {
                    "u8" => Ok(ParsedType::ByteVector),
                    _ => Err(Error::new(
                        value.span(),
                        "Unsuitable type in Vec brackets - only Vec<u8> is supported",
                    )),
                },
                Err(e) => Err(e),
            },
            _ => Err(Error::new(
                type_segment.span(),
                "Only String and Vec<u8> input types are supported",
            )),
        }
    }

    pub fn from_fn_arg(fn_arg: &syn::FnArg) -> syn::Result<ParsedType> {
        let fn_arg = match fn_arg {
            syn::FnArg::Captured(arg) => Ok(&arg.ty),
            _ => Err(Error::new(fn_arg.span(), "Unknown argument")),
        }?;

        ParsedType::from_type(fn_arg)
    }

    pub fn from_return_type(ret_type: &syn::ReturnType) -> syn::Result<ParsedType> {
        let ret_type = match ret_type {
            syn::ReturnType::Type(_, t) => Ok(t),
            _ => Err(Error::new(ret_type.span(), "Unknown argument")),
        }?;

        ParsedType::from_type(ret_type.as_ref())
    }
}

pub trait InputTypeGenerator {
    fn generate_fn_prolog(&self) -> proc_macro2::TokenStream;
}

pub trait ReturnTypeGenerator {
    fn generate_fn_epilog(&self) -> proc_macro2::TokenStream;
}

impl InputTypeGenerator for ParsedType {
    fn generate_fn_prolog(&self) -> proc_macro2::TokenStream {
        match self {
            ParsedType::Utf8String => quote! {
                let arg = memory::read_input_from_mem(ptr, len);
                let arg = String::from_utf8(arg).unwrap();
            },
            ParsedType::ByteVector => quote! {
                let arg = memory::read_input_from_mem(ptr, len);
            },
        }
    }
}

impl ReturnTypeGenerator for ParsedType {
    fn generate_fn_epilog(&self) -> proc_macro2::TokenStream {
        match self {
            ParsedType::Utf8String => quote! {
                memory::write_result_to_mem(
                    result.as_bytes()
                )
                .expect("Putting result string to memory has failed")
            },
            ParsedType::ByteVector => quote! {
                memory::write_result_to_mem(&result[..])
                    .expect("Putting result vector to memory has failed")
            },
        }
    }
}
