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

use syn::parse::{Parse, ParseStream};
use quote::quote;
use syn::export::TokenStream2;

pub struct HandlerAttrs {
    handler_attrs: Vec<HandlerAttr>,
}

pub enum HandlerAttr {
    InitFnName(String),
    SideModules(Vec<String>),
}

impl HandlerAttrs {
    pub fn init_fn_name(&self) -> Option<(&str)> {
        self.handler_attrs
            .iter()
            .filter_map(|attr| match attr {
                HandlerAttr::InitFnName(name) => Some(&name[..]),
                _ => None,
            })
            .next()
    }

    pub fn side_modules(&self) -> Option<(&Vec<String>)> {
        self.handler_attrs
            .iter()
            .filter_map(|attr| match attr {
                HandlerAttr::SideModules(modules) => Some(modules),
                _ => None,
            })
            .next()
    }
}

impl Default for HandlerAttrs {
    fn default() -> Self {
        HandlerAttrs {
            handler_attrs: Vec::new(),
        }
    }
}

impl Parse for HandlerAttrs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut attrs = HandlerAttrs::default();
        if input.is_empty() {
            return Ok(attrs);
        }

        let attr_opts =
            syn::punctuated::Punctuated::<HandlerAttr, syn::token::Comma>::parse_terminated(input)?;
        attrs.handler_attrs = attr_opts.into_iter().collect();

        Ok(attrs)
    }
}

impl Parse for HandlerAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // trying to parse the `init_fn`/`side_modules`/... tokens
        let attr_name = input.step(|cursor| match cursor.ident() {
            Some((ident, rem)) => Ok((ident, rem)),
            None => Err(cursor.error("Expected a valid ident")),
        })?;

        match attr_name.to_string().as_str() {
            "init_fn" => {
                // trying to parse `=`
                input.parse::<::syn::token::Eq>()?;

                // trying to parse a init function name
                match input.parse::<syn::Ident>() {
                    Ok(init_fn_name) => Ok(HandlerAttr::InitFnName(init_fn_name.to_string())),
                    Err(_) => Err(syn::Error::new(
                        attr_name.span(),
                        "Expected a function name",
                    )),
                }
            },

            "side_modules" => {
                // trying to parse `=`
                input.parse::<::syn::token::Eq>()?;

                let raw_side_modules_list;
                syn::parenthesized!(raw_side_modules_list in input);

                let raw_side_modules_opts =
                    syn::punctuated::Punctuated::<syn::Ident, syn::token::Comma>::parse_terminated(
                        &raw_side_modules_list,
                    )?;

                let tt = raw_side_modules_opts
                    .iter()
                    .map(|c| c.to_string())
                    .collect();

                Ok(HandlerAttr::SideModules(tt))
            },

            _ => Err(syn::Error::new(
                attr_name.span(),
                "Expected a `side_modules` token in invocation_handler macros attributes",
            )),
        }
    }
}

pub fn generate_side_modules_glue_code(side_modules_list: &Vec<String>) -> syn::Result<TokenStream2> {
    let mut modules_glue_code = quote!();
    for module_name in side_modules_list {
        let allocate_fn_name = format!("{}_allocate", module_name);
        let deallocate_fn_name = format!("{}_deallocate", module_name);
        let invoke_fn_name = format!("{}_invoke", module_name);
        let load_fn_name = format!("{}_load", module_name);
        let store_fn_name = format!("{}_store", module_name);
        let module_name_ident = syn::parse_str::<syn::Ident>(&module_name)?;

        modules_glue_code = quote! {
            mod #module_name_ident {
                #[link(wasm_import_module = #module_name)]
                extern "C" {
                    #[link_name = #allocate_fn_name]
                    pub fn allocate(size: usize) -> i32;

                    // Deallocate chunk of module memory after it's not used anymore
                    #[link_name = #deallocate_fn_name]
                    pub fn deallocate(ptr: i32, size: usize);

                    // Call module's invocation handler with data specified by pointer and size
                    #[link_name = #invoke_fn_name]
                    pub fn invoke(ptr: i32, size: usize) -> i32;

                    // Read 1 byte from ptr location of module memory
                    #[link_name = #load_fn_name]
                    pub fn load(ptr: i32) -> u8;

                    // Put 1 byte at ptr location in module memory
                    #[link_name = #store_fn_name]
                    pub fn store(ptr: i32, byte: u8);
                }

                // Execute query on module
                pub fn query(query: Vec<u8>) -> Vec<u8> {
                    unsafe {
                        // Allocate memory for the query in module
                        let query_ptr = allocate(query.len());

                        // Store query in module's memory
                        for (i, byte) in query.iter().enumerate() {
                            let ptr = query_ptr + i as i32;
                            store(ptr, *byte);
                        }

                        // Execute the query, and get pointer to the result
                        let result_ptr = invoke(query_ptr, query.len());

                        // First 4 bytes at result_ptr location encode result size, read that first
                        let mut result_size: usize = 0;
                        for byte_id in 0..3 {
                            let ptr = result_ptr + byte_id as i32;
                            let b = load(ptr) as usize;
                            result_size = result_size + (b << (8 * byte_id));
                        }
                        // Now we know exact size of the query execution result

                        // Read query execution result byte-by-byte
                        let mut result_bytes = vec![0; result_size as usize];
                        for byte_id in 0..result_size {
                            let ptr = result_ptr + (byte_id + 4) as i32;
                            let b = load(ptr);
                            result_bytes[byte_id as usize] = b;
                        }

                        // Deallocate query result
                        deallocate(result_ptr, result_size + 4);

                        result_bytes
                    }
                }
            }
        }
    }

    Ok(modules_glue_code)
}
