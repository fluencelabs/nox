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

extern crate proc_macro;

use proc_macro2::Span;
use quote::quote;
use syn::{parse::Error, spanned::Spanned};

pub enum ParsedType {
    Utf8String,
    ByteVector,
}

impl ParsedType {
    pub fn from_type(input_type: &syn::Type) -> syn::Result<ParsedType> {
        // parses generic param T in Vec<T> to string representation
        fn parse_vec_bracket(args: &syn::PathArguments, span: Span) -> syn::Result<String> {
            // checks that T is angle bracketed
            let generic_arg = match args {
                syn::PathArguments::AngleBracketed(args) => Some(args),
                _ => {
                    return Err(Error::new(span, "It has to be a bracketed value after Vec"));
                },
            }
            .unwrap();

            let arg = generic_arg.args.first().unwrap();
            let arg_val = arg.value();

            // converts T to syn::Type
            let arg_type = match arg_val {
                syn::GenericArgument::Type(ty) => Some(ty),
                _ => return Err(Error::new(span, "Incorrect type in Vec brackets")),
            }
            .unwrap();

            // converts T to syn::path
            let arg_path = match arg_type {
                syn::Type::Path(path) => Some(&path.path),
                _ => {
                    return Err(Error::new(
                        span,
                        "Unsuitable type in Vec brackets - only Vec<u8 is supported>",
                    ));
                },
            }
            .unwrap();

            // converts T to String
            let arg_segment = arg_path.segments.first().unwrap();
            let arg_segment = arg_segment.value();
            Ok(arg_segment.ident.to_string())
        }

        let path = match input_type {
            syn::Type::Path(path) => Some(&path.path),
            _ => {
                return Err(Error::new(
                    input_type.span(),
                    "Incorrect argument type - only Vec<u8> and String are supported",
                ));
            },
        }
        .unwrap();

        // argument can be given in full path form: ::std::string::String
        // that why the last one used
        let type_segment = path.segments.last().unwrap();
        let type_segment = type_segment.value();

        match type_segment.ident.to_string().as_str() {
            "String" => Ok(ParsedType::Utf8String),
            "Vec" => match parse_vec_bracket(&type_segment.arguments, type_segment.span()) {
                Ok(value) => match value.as_str() {
                    "u8" => Some(Ok(ParsedType::ByteVector)),
                    _ => {
                        return Err(Error::new(
                            value.span(),
                            "Unsuitable type in Vec brackets - only Vec<u8 is supported>",
                        ));
                    },
                }
                .unwrap(),
                Err(err) => Err(err),
            },
            _ => Err(Error::new(
                type_segment.span(),
                "Only String and Vec<u8> input types are supported",
            )),
        }
    }

    pub fn from_fn_arg(fn_arg: &syn::FnArg) -> syn::Result<ParsedType> {
        let fn_arg = match fn_arg {
            syn::FnArg::Captured(arg) => Some(&arg.ty),
            _ => return Err(Error::new(fn_arg.span(), "Unknown argument")),
        }
        .unwrap();

        ParsedType::from_type(fn_arg)
    }

    pub fn from_return_type(ret_type: &syn::ReturnType) -> syn::Result<ParsedType> {
        let ret_type = match ret_type {
            syn::ReturnType::Type(_, t) => Some(t),
            _ => return Err(Error::new(ret_type.span(), "Unknown argument")),
        }
        .unwrap();

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
                .expect("Putting result string to memory was failed.")
            },
            ParsedType::ByteVector => quote! {
                memory::write_result_to_mem(&result[..])
                    .expect("Putting result vector to memory was failed.")
            },
        }
    }
}
