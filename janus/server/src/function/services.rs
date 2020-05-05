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
use super::router::Service::Delegated;
use super::FunctionRouter;
use faas_api::{Address, FunctionCall};
use libp2p::PeerId;
use std::collections::HashSet;

impl FunctionRouter {
    // ####
    // ## Service routing
    // ###
    pub(super) fn send_to_service(&mut self, service: String, call: FunctionCall) {
        if let Some(builtin) = BuiltinService::from(service.as_str(), call.arguments.clone()) {
            self.execute_builtin(builtin, call)
        } else if let Some(local_service) = self.local_services.get(&service).cloned() {
            log::info!(
                "Service {} was found locally. uuid {}",
                &service,
                &call.uuid
            );
            self.execute_on_service(local_service, call)
        } else {
            log::info!(
                "Service {} not found locally. uuid {}",
                &service,
                &call.uuid
            );
            self.find_service_provider(service, call);
        }
    }

    // Look for service providers, enqueue call to wait for providers
    pub(super) fn find_service_provider(&mut self, service: String, call: FunctionCall) {
        self.wait_provider.enqueue(service.clone(), call);
        // TODO: don't call get_providers if there are already calls waiting for it
        self.get_providers(service)
    }

    // Advance execution for calls waiting for this service: send them to first provider
    pub fn providers_found(&mut self, service_id: &str, providers: HashSet<Address>) {
        if providers.is_empty() {
            self.provider_search_failed(service_id, "zero providers found");
        } else {
            self.provider_search_succeeded(service_id, providers)
        }
    }

    fn provider_search_succeeded(&mut self, service_id: &str, providers: HashSet<Address>) {
        log::info!(
            "Found {} providers for service {}: {:?}",
            providers.len(),
            service_id,
            providers
        );
        let mut calls = self.wait_provider.remove(&service_id.into()).peekable();
        // Check if calls are empty without actually advancing iterator
        if calls.peek().is_none() && !providers.is_empty() {
            log::warn!(
                "Providers found for {}, but there are no calls waiting for it",
                service_id
            );
        }

        // TODO: taking only first provider here. Should we send a call to all of them?
        // TODO: weight providers according to TrustGraph
        let provider = providers.into_iter().next().unwrap();
        for mut call in calls {
            call.target = Some(provider.clone());
            self.send_to(provider.clone(), call);
        }
    }

    pub(super) fn provider_search_failed(&mut self, service_id: &str, reason: &str) {
        let mut calls = self.wait_provider.remove(&service_id.into()).peekable();
        // Check if calls are empty without actually advancing iterator
        if calls.peek().is_none() {
            log::warn!(
                "Failed to find providers for {}: {}; no calls were waiting",
                service_id,
                reason
            );
            return;
        }

        log::warn!("Failed to find providers for {}: {}", service_id, reason);
        for call in calls {
            self.send_error_on_call(
                call,
                format!("Failed to find providers for {}: {}", service_id, reason),
            );
        }
    }

    // Removes all services that are delegated by specified PeerId
    pub(super) fn remove_delegated_services(&mut self, delegate: &PeerId) {
        let delegate = delegate.to_base58();

        self.local_services.retain(|k, v| match v {
            Delegated { forward_to } => {
                let to_delegate = forward_to
                    .destination_peer()
                    .map_or(false, |p| p.to_base58() == delegate);
                if to_delegate {
                    log::info!(
                        "Removing delegated service {}. forward_to: {:?}, delegate: {}",
                        k,
                        forward_to,
                        delegate
                    );
                }
                !to_delegate
            }
        });
    }
}
