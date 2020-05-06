/*
 *   MIT License
 *
 *   Copyright (c) 2020 Fluence Labs Limited
 *
 *   Permission is hereby granted, free of charge, to any person obtaining a copy
 *   of this software and associated documentation files (the "Software"), to deal
 *   in the Software without restriction, including without limitation the rights
 *   to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 *   copies of the Software, and to permit persons to whom the Software is
 *   furnished to do so, subject to the following conditions:
 *
 *   The above copyright notice and this permission notice shall be included in all
 *   copies or substantial portions of the Software.
 *
 *   THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 *   IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 *   FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 *   AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 *   LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 *   OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 *   SOFTWARE.
 */

use super::builtin_service::BuiltinService;
//use super::router::Service::Delegated;
use super::FunctionRouter;
use crate::function::waiting_queues::Enqueued;
use faas_api::{Address, FunctionCall, Protocol};
use libp2p::PeerId;
use std::collections::HashSet;

impl FunctionRouter {
    // ####
    // ## Service routing
    // ###

    pub(super) fn service_available_locally(&self, service: &Protocol) -> bool {
        BuiltinService::is_builtin(service) || self.provided_names.contains_key(&service.into())
    }

    pub(super) fn pass_to_local_service(&mut self, name: Address, mut call: FunctionCall) {
        if let Some(builtin) = BuiltinService::from(&name, call.arguments.clone()) {
            self.execute_builtin(builtin, call)
        } else if let Some(provider) = self.provided_names.get(&name).cloned() {
            log::info!("Service {} was found locally. uuid {}", &name, &call.uuid);
            log::info!("Forwarding service call {} to {}", call.uuid, provider);
            call.target = Some(
                call.target
                    .map_or(provider.clone(), |target| provider.extend(&target)),
            ); // TODO: write tests on that, it's a very complex decision
            self.call(call);
        } else {
            let err_msg = "unroutable message: no local provider found for target".to_string();
            log::warn!("Error on {}: {}", &call.uuid, err_msg);
            self.send_error_on_call(call, err_msg);
        }
    }

    // Look for service providers, enqueue call to wait for providers
    pub(super) fn find_service_provider(&mut self, name: Address, call: FunctionCall) {
        log::info!("Finding service provider for {}, call: {:?}", name, call);
        if let Enqueued::New = self.wait_name_resolved.enqueue(name.clone(), call) {
            // won't call get_providers if there are already calls waiting for it
            self.resolve_name(name)
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
                    call.target
                        .map_or(provider.clone(), |target| provider.clone().extend(target)),
                );
                // TODO: write tests on that, it's a very complex decision
                // call.target = Some(provider.clone());
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

    // Removes all names that resolve to an address containing `resolvee`
    pub(super) fn remove_halted_names(&mut self, resolvee: &PeerId) {
        use Protocol::*;

        self.provided_names.retain(|k, v| {
            let protocols = v.protocols();
            let halted = protocols.iter().any(|p| match p {
                Peer(id) if id == resolvee => true,
                Client(id) if id == resolvee => true,
                _ => false,
            });
            if halted {
                log::info!(
                    "Removing halted name {}. forward_to: {} due to peer {} disconnection",
                    k,
                    v,
                    resolvee
                );
            }
            !halted
        });
    }
}
