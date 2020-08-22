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
use crate::app_service::{ServiceCall, ServiceCallResult, ServiceExecError};
use crate::function::wait_address::WaitAddress;
use crate::function::{CallError, CallErrorKind::*, ErrorData};
use faas_api::{Address, FunctionCall, Protocol};
use libp2p::PeerId;
use std::collections::HashSet;

type CallResult<T> = std::result::Result<T, CallError>;

impl FunctionRouter {
    // ####
    // ## Service routing
    // ###

    /// Execute call locally: on builtin service or forward to provided name
    pub(super) fn execute_locally(
        &mut self,
        module: String,
        call: FunctionCall,
        hashtag: Option<String>,
    ) -> CallResult<()> {
        if BuiltinService::is_builtin(&module) {
            let builtin = BuiltinService::from(module, call.arguments.clone())
                .map_err(|e| call.clone().error(e))?;
            return self.execute_builtin(builtin, call);
        }

        let call = self.prepare_call(module, hashtag, call)?;
        self.app_service.execute(call);

        Ok(())
    }

    /// Create `ServiceCall` from a `FunctionCall`
    fn prepare_call(
        &mut self,
        module: String,
        service_id: Option<String>,
        call: FunctionCall,
    ) -> Result<ServiceCall, CallError> {
        // DSL-like function to save letters on error handling
        let e = |e| call.clone().error(e);

        let service_id = match service_id {
            Some(service_id) => service_id,
            None => return Err(e(MissingServiceId)),
        };

        let interface = self
            .app_service
            .get_interface(&service_id)
            .map_err(|e| call.clone().error(e))?;
        let functions = interface.modules.get(module.as_str()).ok_or_else(|| {
            e(NoSuchModule {
                module: module.clone(),
                service_id: service_id.clone(),
            })
        })?;
        let function = call.fname.as_ref().ok_or_else(|| {
            e(MissingFunctionName {
                module: module.to_string(),
            })
        })?;
        if !functions.contains_key(function.as_str()) {
            return Err(e(FunctionNotFound {
                module: module.to_string(),
                function: function.to_string(),
            }));
        }

        let call_parameters = call
            .call_parameters
            .as_ref()
            .ok_or_else(|| e(MissingCallParameters))?
            .clone();

        let faas_call = ServiceCall::Call {
            service_id,
            module,
            function: function.to_string(),
            arguments: call.arguments.clone(),
            call_parameters,
            call,
        };

        Ok(faas_call)
    }

    // Look for service providers, enqueue call to wait for providers
    pub(super) fn find_providers(&mut self, name: Address, call: FunctionCall) {
        log::info!("Finding service provider for {}, call: {:?}", name, call);
        self.wait_address
            .enqueue(name.clone(), WaitAddress::ProviderFound(call));
        // TODO: don't call resolve_name if there is the same request in progress
        self.resolve_name(&name)
    }

    // Advance execution for calls waiting for this service: send them to first provider
    pub fn providers_found(&mut self, name: Address, providers: HashSet<Address>) {
        if providers.is_empty() {
            self.provider_search_failed(name, "zero providers found");
        } else {
            self.provider_search_succeeded(name, providers)
        }
    }

    fn provider_search_succeeded(&mut self, name: Address, providers: HashSet<Address>) {
        log::info!(
            "Found {} providers for name {}: {:?}",
            providers.len(),
            name,
            providers
        );
        let mut calls = self
            .wait_address
            .remove_with(name.clone(), WaitAddress::provider_found)
            .peekable();
        // Check if calls are empty without actually advancing iterator
        if calls.peek().is_none() && !providers.is_empty() {
            log::warn!(
                "Providers found for {}, but there are no calls waiting for it",
                name
            );
        }

        let mut providers = providers.into_iter().collect::<Vec<_>>();
        // Sort providers by weight in trust graph
        providers.sort_by_cached_key(|addr| {
            // Take last public key in the address
            let pk = addr.iter().rev().find_map(|p| p.public_key())?;
            let pk = match pk {
                libp2p::identity::PublicKey::Ed25519(pk) => pk,
                _ => return None,
            };
            // Look it up in the trust graph
            self.kademlia.trust.weight(pk)
        });

        // TODO: Sending call to all providers here,
        //       implement and use ProviderSelector::All, ProviderSelector::Latest, ProviderSelector::MaxWeight
        for call in calls.map(|c| c.call()) {
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

    pub(super) fn provider_search_failed(&mut self, name: Address, reason: &str) {
        let mut calls = self
            .wait_address
            .remove_with(name.clone(), WaitAddress::provider_found)
            .peekable();

        // Check if calls are empty without actually advancing iterator
        if calls.peek().is_none() {
            log::warn!("Failed to find providers for {}: {}; 0 calls", name, reason);
            return;
        }
        log::warn!("Failed to find providers for {}: {}", name, reason);

        for c in calls {
            self.send_error_on_call(
                c.call(),
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

    /// Serialize and send app service result as a reply
    pub(super) fn send_app_service_result(
        &mut self,
        call: FunctionCall,
        result: Result<ServiceCallResult, ServiceExecError>,
    ) -> Result<(), CallError> {
        use serde_json::Value;

        let data = match result {
            Ok(result) => {
                let result = serde_json::to_value(result)
                    .map_err(|e| call.clone().error(ResultSerializationFailed(e.to_string())))?;
                ("result", result)
            }
            Err(error) => ("error", Value::String(error.to_string())),
        };
        self.reply_with(call, None, data)?;

        Ok(())
    }
}
