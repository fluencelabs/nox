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
use super::router::Service;
use super::router::Service::Delegated;
use super::FunctionRouter;
use faas_api::{Address, FunctionCall};

impl FunctionRouter {
    // ####
    // ## Execution
    // ###

    // Send call to local service
    pub(super) fn execute_on_service(&mut self, service: Service, call: FunctionCall) {
        log::debug!("Got call for local service {:?}: {:?}", service, call);
        match service {
            Delegated { forward_to } => {
                log::info!("Forwarding service call {} to {}", call.uuid, forward_to);
                self.send_to(forward_to, call)
            }
        }
    }

    pub(super) fn execute_builtin(&mut self, service: BuiltinService, call: FunctionCall) {
        match service {
            BuiltinService::DelegateProviding { service_id } => match &call.reply_to {
                // To avoid routing cycle (see https://gist.github.com/folex/61700dd6afa14fbe3d1168e04dfe2661 for bug postmortem)
                Some(Address::Relay { relay, .. }) if !self.is_local(&relay) => {
                    log::warn!(
                        "Declined to register service targeted to a different relay: {:?}",
                        call
                    );
                    self.send_error_on_call(
                        call,
                        "declined to register service targeted to a different relay".into(),
                    );
                }
                Some(Address::Service {
                    service_id: forward_to,
                }) if &service_id == forward_to => {
                    log::warn!("Declined to register cyclic service delegation: {:?}", call);
                    // TODO: return error to sender
                }
                // Happy path – registering delegated service. TODO: is it sound to allow forward_to = Address::Service?
                Some(forward_to) => {
                    let new = Delegated {
                        forward_to: forward_to.clone(),
                    };
                    let replaced = self.local_services.insert(service_id.clone(), new.clone());
                    if let Some(replaced) = replaced {
                        log::warn!(
                            "Replaced service {:?} with {:?} due to call {}",
                            replaced,
                            new,
                            &call.uuid
                        );
                    }

                    log::info!("Published a service {}: {:?}", service_id, call);
                    self.publish_provider(service_id, &new);
                }
                // If there's no `reply_to`, then we don't know where to forward, so can't register
                None => {
                    log::warn!("reply_to was not defined in {:?}", call);
                    self.send_error_on_call(
                        call,
                        "reply_to must be defined when calling 'provide' service".into(),
                    )
                }
            },
        }
    }

    pub(super) fn handle_local_call(&mut self, call: FunctionCall) {
        log::warn!("Got call {:?}, don't know what to do with that", call);
        self.send_error_on_call(call, "Don't know how to handle that".to_string())
    }
}
