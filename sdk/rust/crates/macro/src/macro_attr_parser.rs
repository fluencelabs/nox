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
    // There could be some other attributes in future
    handler_attrs: Vec<HandlerAttr>
}

pub enum HandlerAttr {
    InitFnName(String)
}

impl HandlerAttrs {
    pub fn init_fn_name(&self) -> Option<(&str)> {
        self.handler_attrs
            .iter()
            .filter_map(|attr| match attr {
                HandlerAttr::InitFnName(name) => {
                    Some(&name[..])
                }
            })
            .next()
    }
}

impl Default for HandlerAttrs {
    fn default() -> Self {
        HandlerAttrs { handler_attrs: Vec::new() }
    }
}

impl Parse for HandlerAttrs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut attrs = HandlerAttrs::default();
        if input.is_empty() {
            return Ok(attrs);
        }

        // trying to parse the first part
        let attr = input.step(|cursor| match cursor.ident() {
            Some((ident, rem)) => Ok((ident, rem)),
            None => Err(cursor.error("expected a valid ident")),
        })?;

        let init_fn_name = match attr.to_string().as_str() {
            "init_fn" => {
                    input.parse::<::syn::token::Eq>()?;
                    match input.parse::<syn::Ident>() {
                        Ok(init_fn_name) => Ok(init_fn_name.to_string()),
                        Err(_) => {
                            Err(syn::Error::new(attr.span(), "expected function name"))
                        }
                }
            },
            _ => Err(syn::Error::new(attr.span(), "expected a `init_fn` token in invocation_handler macros attributes"))
        }?;

        attrs.handler_attrs.push(HandlerAttr::InitFnName(init_fn_name));
        Ok(attrs)
    }
}
