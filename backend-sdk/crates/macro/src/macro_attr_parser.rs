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
