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

//! This module defines `invocation_handler` attribute procedural macro. It can simplify main module
//! invocation handler signature. According to Fluence backend convensions this handler can look
//! like this:
//!
//! ```
//! #[no_mangle]
//! pub unsafe fn invoke(ptr: *mut u8, len: usize) -> NonNull<u8> {
//!    let user_name = fluence::memory::read_input_from_mem(ptr, len);
//!    let user_name: String = String::from_utf8(user_name).unwrap();
//!
//!    // return a pointer to the result in memory
//!    fluence::memory::write_result_to_mem(format!("Hello from Fluence to {}", user_name).as_bytes())
//!        .expect("Putting result string to the memory was failed.")
//! }
//! ```
//!
//! We can use a neater way instead with `#[invocation_handler]`:
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
//! 1. `f` mustn't have more than one input argument.
//! 2. `f` mustn't be `unsafe`, `const`, generic, have custom abi linkage or variadic param.
//! 3. The type of `f` input (if it present) and output parameters must be one from
//!    {String, Vec<u8>} set.
//! 4. `f` mustn't have the name `invoke`.
//!
//! For troubleshooting and macros debugging [cargo expand](https://github.com/dtolnay/cargo-expand)
//! can be used.
//!
//! Internally this macros creates a new function `invoke` that converts a raw argument to
//! a appropriate format, calls `f` and then write `f` result via `memory::write_result_to_mem` to
//! module memory. So to use this crate apart from `fluence` `fluence_sdk_main` has to be imported.
//!
//! The macro also has an `init_fn` attribute that can be used for specifying initialization
//! function name. This function will be called only in the first invoke function call. It can be
//! used like this:
//!
//! ```
//! use fluence::sdk::*;
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
//! # Examples
//!
//! Please find more examples in `https://github.com/fluencelabs/fluence/tree/master/vm/examples`.
//!

#![doc(html_root_url = "https://docs.rs/fluence-sdk-macro/0.1.0")]

extern crate proc_macro;
mod macro_attr_parser;
mod macro_input_parser;

use crate::macro_attr_parser::HandlerAttrs;
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
                "Please do not make the main module invocation handler constant",
            ));
        }
        if let Some(unsafety) = unsafety {
            return Err(Error::new(
                unsafety.span,
                "Please do not make the main module invocation handler unsafe",
            ));
        }
        if let Some(abi) = abi {
            return Err(Error::new(
                abi.extern_token.span,
                "Please do not use any custom linkage with the main module invocation handler",
            ));
        }
        if !decl.generics.params.is_empty() || decl.generics.where_clause.is_some() {
            return Err(Error::new(
                decl.fn_token.span,
                "Please do not use template parameters with the main module invocation handler",
            ));
        }
        if let Some(variadic) = decl.variadic {
            return Err(Error::new(
                variadic.spans[0],
                "Please do not use variadic interface with the main module invocation handler",
            ));
        }
        Ok(())
    })() {
        return Err(e);
    }

    let input_type =
        match decl.inputs.len() {
            0 => ParsedType::Empty,
            1 => ParsedType::from_fn_arg(decl.inputs.first().unwrap().into_value())?,
            _ => return Err(Error::new(
                decl.inputs.span(),
                "Please do not use more than one argument in the main module invocation handler",
            )),
        };
    let output_type = ParsedType::from_return_type(&decl.output)?;
    if output_type == ParsedType::Empty {
        return Err(Error::new(
            decl.output.span(),
            "The main module invocation handler has to has a return value",
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
    Ok(resulted_invoke)
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
