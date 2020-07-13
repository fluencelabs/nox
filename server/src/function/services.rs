/*
 * Copyright 2020 Fluence Labs Limited
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

use super::builtin_service::BuiltinService;
use super::FunctionRouter;
use crate::function::waiting_queues::Enqueued;
use crate::function::{CallError, CallErrorKind, CallErrorKind::*, ErrorData};
use faas_api::{Address, FunctionCall, Protocol};
use libp2p::PeerId;
use std::collections::HashSet;

type CallResult<'a, T> = std::result::Result<T, CallError<'a>>;

impl FunctionRouter {
    // ####
    // ## Service routing
    // ###

    /// Execute call locally: on builtin service or forward to provided name
    pub(super) fn execute_locally<'a>(
        &mut self,
        module: &'a str,
        call: FunctionCall,
        hashtag: Option<String>,
    ) -> CallResult<'a, ()> {
        if BuiltinService::is_builtin(module) {
            let builtin = BuiltinService::from(module, call.arguments.clone())
                .map_err(|e| call.clone().error(e))?;
            return self.execute_builtin(builtin, call);
        }

        let found = self
            .find_in_faas(module, call.fname.as_deref(), hashtag)
            .map_err(|e| call.clone().error(e))?;

        if let Some((module, function)) = found {
            return self.execute_wasm(module, function, call);
        }

        Err(UnroutableCall(format!("module {} not found", module)).of_call(call))
    }

    /// Find a matching module with a matching function, and return their names
    fn find_in_faas<'a>(
        &mut self,
        module: &'a str,
        function: Option<&str>,
        service_id: Option<String>,
    ) -> Result<Option<(String, String)>, CallErrorKind<'a>> {
        let service_id = service_id.ok_or(MissingServiceId)?;
        let interface = self.faas.get_interface(&service_id)?;
        let functions = interface
            .modules
            .get(module)
            .ok_or(NoSuchModule { module, service_id })?;
        let function = function.ok_or_else(|| MissingFunctionName {
            module: module.to_string(),
        })?;
        if !functions.contains_key(function) {
            return Err(CallErrorKind::FunctionNotFound {
                module: module.to_string(),
                function: function.to_string(),
            });
        }
        Ok(Some((module.to_string(), function.to_string())))
    }

    // Look for service providers, enqueue call to wait for providers
    pub(super) fn find_providers(&mut self, name: Address, call: FunctionCall) {
        log::info!("Finding service provider for {}, call: {:?}", name, call);
        if let Enqueued::New = self.wait_name_resolved.enqueue(name.clone(), call) {
            // won't call get_providers if there are already calls waiting for it
            self.resolve_name(&name)
        } else {
            log::debug!(
                "won't call resolve_name because there are already promises waiting for {}",
                name
            )
        }
    }

    // Advance execution for calls waiting for this service: send them to first provider
    pub fn providers_found(&mut self, name: &Address, providers: HashSet<Address>) {
        if providers.is_empty() {
            self.provider_search_failed(name, "zero providers found");
        } else {
            self.provider_search_succeeded(name, providers)
        }
    }

    fn provider_search_succeeded(&mut self, name: &Address, providers: HashSet<Address>) {
        log::info!(
            "Found {} providers for name {}: {:?}",
            providers.len(),
            name,
            providers
        );
        let mut calls = self.wait_name_resolved.remove(&name).peekable();
        // Check if calls are empty without actually advancing iterator
        if calls.peek().is_none() && !providers.is_empty() {
            log::warn!(
                "Providers found for {}, but there are no calls waiting for it",
                name
            );
        }

        // TODO: Sending call to all providers here,
        //       implement and use ProviderSelector::All, ProviderSelector::Latest, ProviderSelector::MaxWeight
        // TODO: weight providers according to TrustGraph
        for call in calls {
            for provider in providers.iter() {
                let mut call = call.clone();
                call.target = Some(
                    // TODO: write tests on that, it's a very complex decision
                    call.target
                        .map_or(provider.clone(), |target| provider.clone().extend(target)),
                );
                log::debug!("Sending call to provider {:?}", call);
                self.call(call);
            }
        }
    }

    pub(super) fn provider_search_failed(&mut self, name: &Address, reason: &str) {
        let mut calls = self.wait_name_resolved.remove(name).peekable();
        // Check if calls are empty without actually advancing iterator
        if calls.peek().is_none() {
            log::warn!("Failed to find providers for {}: {}; 0 calls", name, reason);
            return;
        } else {
            log::warn!("Failed to find providers for {}: {}", name, reason);
        }
        for call in calls {
            self.send_error_on_call(
                call,
                format!("Failed to find providers for {}: {}", name, reason),
            );
        }
    }

    /// Removes all names that resolve to an address containing `resolvee`
    pub(super) fn remove_halted_names(&mut self, resolvee: &PeerId) {
        use Protocol::*;

        // TODO: use drain_filter once available https://github.com/rust-lang/rust/issues/59618
        let mut removed = vec![];

        self.provided_names.retain(|k, v| {
            let protocols = v.protocols();
            let halted = protocols.iter().any(|p| match p {
                Peer(id) if id == resolvee => true,
                Client(id) if id == resolvee => true,
                _ => false,
            });
            if halted {
                log::info!(
                    "Unpublishing halted name {}. forward_to: {} due to peer {} disconnection",
                    k,
                    v,
                    resolvee
                );
                // TODO: avoid clone?
                removed.push(k.clone())
            }
            !halted
        });

        for name in removed {
            self.unpublish_name(name);
        }
    }
}
