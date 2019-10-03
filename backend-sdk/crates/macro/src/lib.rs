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
//! This module defines an `invocation_handler` attribute procedural macro. It can be used to
//! simplify the signature of the main module invocation handler:
//!
//! ```
//! use fluence::sdk::*;
//!
//! #[invocation_handler]
//! fn greeting(name: String) -> String {
//!    format!("Hello from Fluence to {}", name)
//! }
//! ```
//!
//! To use this macro with a function `f` certain conditions must be met:
//! 1. `f` shouldn't have more than one input argument.
//! 2. `f` shouldn't be `unsafe`, `const`, generic, have custom ABI linkage or variadic param.
//! 3. The type of `f` input (if it presents) and output parameters should be one from
//!    {String, Vec<u8>} set.
//! 4. `f` shouldn't have the name `invoke`.
//!
//! For troubleshooting and macros debugging [cargo expand](https://github.com/dtolnay/cargo-expand)
//! can be used.
//!
//! Internally this macro creates a new function `invoke` that converts a raw argument to the
//! appropriate format, calls `f` and then writes `f` result via `memory::write_response_to_mem` to
//! module memory. So to use this crate apart from `fluence` `fluence_sdk_main` has to be imported.
//!
//! The macro also has the `init_fn` and `side_modules` attributes. The first one that can be used
//! for specifying initialization function name. This function is called only once at the first
//! call of the invoke function. It can be used like this:
//!
//! ```
//! use fluence::sdk::*;
//! use log::info;
//!
//! fn init() {
//!     logger::WasmLogger::init_with_level(log::Level::Info).is_ok()
//! }
//!
//! #[invocation_handler(init_fn = init)]
//! fn greeting(name: String) -> String {
//!     info!("{} has been successfully greeted", name);
//!     format!("Hello from Fluence to {}", name)
//! }
//! ```
//!
//! The second macro could be used for generate API to connect with side modules like SQlite and
//! Redis. It can be used like this:
//! ```
//! use fluence::sdk::*;
//!
//! #[invocation_handler(side_modules = (sqlite, redis))]
//! fn greeting(name: String) -> String {
//!     sqlite::call("SELECT * from users");
//!     sqlite::call("GET user");
//!     format!("Hello from Fluence to {}", name)
//! }
//! ```
//!
//! # Examples
//!
//! Please find more examples [here](https://github.com/fluencelabs/tutorials).

#![doc(html_root_url = "https://docs.rs/fluence-sdk-macro/0.1.9")]

#![deny(
    dead_code,
    nonstandard_style,
    unused_imports,
    unused_mut,
    unused_variables,
    unused_unsafe,
    unreachable_patterns
)]

#![recursion_limit = "128"]

extern crate proc_macro;
mod macro_attr_parser;
mod macro_input_parser;

use crate::macro_attr_parser::{generate_side_modules_glue_code, HandlerAttrs};
use crate::macro_input_parser::{InputTypeGenerator, ParsedType, ReturnTypeGenerator};
use proc_macro::TokenStream;
use quote::quote;
use syn::spanned::Spanned;
use syn::{parse::Error, parse_macro_input, ItemFn};

fn invoke_handler_impl(
    attr: proc_macro2::TokenStream,
    fn_item: syn::ItemFn,
) -> syn::Result<proc_macro2::TokenStream> {
    let ItemFn {
        constness,
        unsafety,
        abi,
        ident,
        decl,
        ..
    } = &fn_item;

    if let Err(e) = (|| {
        if let Some(constness) = constness {
            return Err(Error::new(
                constness.span,
                "The invocation handler shouldn't be constant",
            ));
        }
        if let Some(unsafety) = unsafety {
            return Err(Error::new(
                unsafety.span,
                "The invocation handler shouldn't be unsage",
            ));
        }
        if let Some(abi) = abi {
            return Err(Error::new(
                abi.extern_token.span,
                "The invocation handler shouldn't have any custom linkage",
            ));
        }
        if !decl.generics.params.is_empty() || decl.generics.where_clause.is_some() {
            return Err(Error::new(
                decl.fn_token.span,
                "The invocation handler shouldn't use template parameters",
            ));
        }
        if let Some(variadic) = decl.variadic {
            return Err(Error::new(
                variadic.spans[0],
                "The invocation handler shouldn't use variadic interface",
            ));
        }
        Ok(())
    })() {
        return Err(e);
    }

    let input_type = match decl.inputs.len() {
        0 => ParsedType::Empty,
        1 => ParsedType::from_fn_arg(decl.inputs.first().unwrap().into_value())?,
        _ => {
            return Err(Error::new(
                decl.inputs.span(),
                "The invocation handler shouldn't have more than one argument",
            ))
        },
    };
    let output_type = ParsedType::from_return_type(&decl.output)?;
    if output_type == ParsedType::Empty {
        return Err(Error::new(
            decl.output.span(),
            "The invocation handler should have the return value",
        ));
    }

    let prolog = input_type.generate_fn_prolog();
    let prolog = match input_type {
        ParsedType::Empty => quote! {
            #prolog

            let result = #ident();
        },
        _ => quote! {
            #prolog

            let result = #ident(arg);
        },
    };
    let epilog = output_type.generate_fn_epilog();

    let attrs = syn::parse2::<HandlerAttrs>(attr)?;
    let raw_init_fn_name = attrs.init_fn_name();
    let raw_side_modules_list = attrs.side_modules();

    let resulted_invoke = match raw_init_fn_name {
        Some(init_fn_name) => {
            let init_fn_name = syn::parse_str::<syn::Ident>(init_fn_name)?;
            quote! {
                #fn_item

                static mut __FLUENCE_SDK_IS_INITED_d28374a960b570e5db00dfe7a0c7b93: bool = false;

                #[no_mangle]
                pub unsafe fn invoke(ptr: *mut u8, len: usize) -> std::ptr::NonNull<u8> {
                        if !__FLUENCE_SDK_IS_INITED_d28374a960b570e5db00dfe7a0c7b93 {
                            #init_fn_name();
                            unsafe { __FLUENCE_SDK_IS_INITED_d28374a960b570e5db00dfe7a0c7b93 = true; }
                        }

                    #prolog

                    #epilog
                }
            }
        },
        None => quote! {
            #fn_item

            #[no_mangle]
            pub unsafe fn invoke(ptr: *mut u8, len: usize) -> std::ptr::NonNull<u8> {
                #prolog

                #epilog
            }
        },
    };

    match raw_side_modules_list {
        Some(side_modules) => {
            let side_modules_glue_code = generate_side_modules_glue_code(side_modules)?;
            Ok(quote! {
                #side_modules_glue_code
                #resulted_invoke
            })
        },
        _ => Ok(resulted_invoke),
    }
}

#[proc_macro_attribute]
pub fn invocation_handler(attr: TokenStream, input: TokenStream) -> TokenStream {
    let fn_item = parse_macro_input!(input as ItemFn);
    match invoke_handler_impl(attr.into(), fn_item) {
        Ok(v) => v,
        // converts syn:error to proc_macro2::TokenStream
        Err(e) => e.to_compile_error(),
    }
    // converts proc_macro2::TokenStream to proc_macro::TokenStream
    .into()
}
