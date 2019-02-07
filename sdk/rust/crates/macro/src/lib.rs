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

//! This module defines invocation_handler attribute procedural macro. It allows simplifiing
//! principal module invocation handler signature. According to `Fluence Wasm backend convensions`
//! is can look like this
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
//! Instead of this you can write more pretty one
//!
//! ```
//! use fluence::sdk::*;
//!
//! #[invocation_handler]
//! fn main(name: String) -> String {
//!    format!("Hello from Fluence to {}", name)
//! }
//! ```
//!
//! To use this macro with some function `f` some conditions have to be met:
//! 1. `f` has to have one input argument.
//! 2. `f` has to don't be `unsafe`, `const`, generic or have custom abi linkage or variadic param.
//! 3. The input and output types of `f` has to be in {String, Vec<u8>} set.
//! 4. `f` has to don't have name `invoke`.
//!
//! For trubleshooting and macros debugging [carga expand](https://github.com/dtolnay/cargo-expand)
//! can be used.
//!
//! Internally this macros creates a new function `invoke` that converts a raw argument to
//! appropriate format, call `f` and then converts its result by `memory::write_result_to_mem` from
//! `fluence_sdk_main`. So to use this crate apart from `fluence` at first `fluence_sdk_main` has
//! to be imported.
//!
//! # Safety
//!
//! Nothing special: `f` recevies an argument and returns a result by value.
//!
//! # Examples
//!
//! Please find more examples in `https://github.com/fluencelabs/fluence/tree/master/vm/examples`.
//!

extern crate proc_macro;
mod parser;

use crate::parser::{InputTypeGenerator, ParsedType, ReturnTypeGenerator};
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse::Error, parse_macro_input, ItemFn};

#[warn(clippy::redundant_closure_call)]
fn invoke_handler_impl(fn_item: &syn::ItemFn) -> syn::Result<proc_macro2::TokenStream> {
    let ItemFn {
        constness,
        unsafety,
        abi,
        ident,
        decl,
        ..
    } = fn_item;

    if let Err(e) = (|| {
        if decl.inputs.len() != 1 {
            return Err(Error::new(
                decl.paren_token.span,
                "The principal module invocation handler has to have one input param",
            ));
        }
        if let Some(constness) = constness {
            return Err(Error::new(
                constness.span,
                "The principal module invocation handler has to don't be const",
            ));
        }
        if let Some(unsafety) = unsafety {
            return Err(Error::new(
                unsafety.span,
                "The principal module invocation handler has to don't be unsafe",
            ));
        }
        if let Some(abi) = abi {
            return Err(Error::new(
                abi.extern_token.span,
                "The principal module invocation handler has to don't have custom linkage",
            ));
        }
        if !decl.generics.params.is_empty() || decl.generics.where_clause.is_some() {
            return Err(Error::new(
                decl.fn_token.span,
                "The principal module invocation handler has to don't have generic params",
            ));
        }
        if let Some(variadic) = decl.variadic {
            return Err(Error::new(
                variadic.spans[0],
                "The principal module invocation handler has to don't be variadic",
            ));
        }
        Ok(())
    })() {
        return Err(e);
    }

    let input_type = ParsedType::from_fn_arg(
        decl.inputs
            .first()
            // it is already checked that there is only one input arg
            .unwrap()
            .into_value(),
    )?;
    let output_type = ParsedType::from_return_type(&decl.output)?;

    let prolog = input_type.generate_fn_prolog();
    let epilog = output_type.generate_fn_epilog();

    let resulted_invoke = quote! {
        #fn_item

        #[no_mangle]
        pub unsafe fn invoke(ptr: *mut u8, len: usize) -> std::ptr::NonNull<u8> {
            #prolog

            let result = #ident(arg);

            #epilog
        }
    };

    Ok(resulted_invoke)
}

#[proc_macro_attribute]
pub fn invocation_handler(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let fn_item = parse_macro_input!(input as ItemFn);
    match invoke_handler_impl(&fn_item) {
        Ok(v) => v,
        // converts syn:error to proc_macro2::TokenStream
        Err(e) => e.to_compile_error(),
    }
    // converts proc_macro2::TokenStream to proc_macro::TokenStream
    .into()
}
