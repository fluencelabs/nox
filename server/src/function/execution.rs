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

use super::{
    address_signature::verify_address_signatures,
    builtin_service::{
        AddCertificates, BuiltinService, GetCertificates, GetInterface, Identify, Provide,
    },
    FunctionRouter,
};
use crate::function::errors::CallErrorKind::{InvalidArguments, ResultSerializationFailed};
use crate::function::CallError;
use faas_api::{provider, Address, FunctionCall, Protocol};
use fluence_faas::IValue;
use itertools::Itertools;
use libp2p::PeerId;
use serde::Serialize;
use serde_json::{json, Value};
use trust_graph::Certificate;

impl FunctionRouter {
    /// Execute call on builtin service: "provide", "certificates", etc
    /// `ttl` – time to live, if `0`, then "certificates" and "add_certificates" services
    ///  won't send call to neighborhood
    pub(super) fn execute_builtin(&mut self, service: BuiltinService, call: FunctionCall) {
        use BuiltinService as BS;

        match service {
            BS::Provide(Provide { service_id }) => self.provide(provider!(service_id), call),
            BS::GetCertificates(GetCertificates { peer_id, msg_id }) => {
                self.get_certificates(peer_id, call, msg_id)
            }
            BS::AddCertificates(AddCertificates {
                peer_id,
                certificates,
                msg_id,
            }) => self.add_certificates(peer_id, certificates, call, msg_id),
            BS::Identify(Identify { msg_id }) => {
                let addrs = &self.config.external_addresses;
                let addrs: Vec<_> = addrs.iter().map(ToString::to_string).collect();
                self.reply_with(call, msg_id, ("addresses", addrs))
            }
            BS::GetInterface(GetInterface { msg_id }) => {
                match serde_json::to_value(self.faas.get_interface()) {
                    Ok(interface) => self.reply_with(call, msg_id, ("interface", interface)),
                    Err(err) => log::error!(
                        "Totally unexpected: can't serialize FaaS interface to json: {}",
                        err
                    ),
                }
            }
        }
    }

    /// Execute `function` on `module` in the FluenceFaaS
    pub(super) fn execute_wasm(
        &mut self,
        module: String,
        function: String,
        call: FunctionCall,
    ) -> Result<(), CallError<'static>> {
        let arguments = if call.arguments != Value::Null {
            fluence_faas::to_interface_value(&call.arguments).map_err(|e| {
                InvalidArguments {
                    error: format!("can't parse arguments as array of interface types: {}", e),
                }
                .of_call(call.clone())
            })?
        } else {
            IValue::Record(<_>::default())
        };

        let arguments = match &arguments {
            IValue::Record(arguments) => Ok(arguments.as_ref()),
            other => Err(InvalidArguments {
                error: format!("expected array of interface values: got {:?}", other),
            }
            .of_call(call.clone())),
        }?;

        let result = self
            .faas
            .call_module(&module, &function, arguments)
            .map_err(|e| CallError::make(call.clone(), e))?;
        let result = fluence_faas::from_interface_values(&result)
            .map_err(|e| ResultSerializationFailed(e.to_string()).of_call(call.clone()))?;

        Ok(self.reply_with(call, None, ("result", result)))
    }

    fn add_certificates(
        &mut self,
        peer_id: PeerId,
        certificates: Vec<Certificate>,
        call: FunctionCall,
        msg_id: Option<String>,
    ) {
        #[rustfmt::skip]
        log::info!(
            "executing add_certificates of {} certs for {}, call: {:?}", 
            certificates.len(), peer_id, call
        );

        // Calculate current time in Duration
        let time = trust_graph::current_time();
        // Add each certificate, and collect errors
        let errors: Vec<_> = certificates
            .iter()
            .flat_map(|cert| {
                self.kademlia
                    .trust
                    .add(cert.clone(), time)
                    .map_err(|err| (cert, err))
                    .err()
            })
            .collect();

        // If there are any errors, send these errors in a single message
        if !errors.is_empty() {
            #[rustfmt::skip]
            let error = errors.into_iter().map(
                |(cert, err)| format!("cert error {:?}: {}", cert, err)
            ).join("\n");
            #[rustfmt::skip]
            log::warn!("add_certificates error uuid {}: {} call {:?}", call.uuid, error, call);
            self.send_error_on_call(call.clone(), error);
        } else if let Some(reply_to) = call.reply_to.as_ref() {
            // If there are no errors, send reply marking success
            log::debug!("add_certificates success uuid {}", call.uuid);

            let status = format!("{} certs added", certificates.len());
            let arguments = json!({ "msg_id": msg_id, "status": status });
            // Build reply
            let call = FunctionCall {
                uuid: Self::uuid(),
                target: Some(reply_to.clone()),
                reply_to: Some(self.config.local_address()),
                module: None,
                fname: None,
                arguments,
                name: Some("reply on add_certificates".into()),
                sender: self.config.local_address(),
            };
            self.call(call);
        } else {
            log::warn!(
                "add_certificates reply_to is empty {}, can't send reply",
                call.uuid
            );
        }

        // Finally – broadcast that call to the neighborhood if it wasn't already a replication
        // NOTE: not filtering errors, other nodes may succeed where we failed
        self.replicate_to_neighbors(peer_id, call);
    }

    fn get_certificates(&mut self, peer_id: PeerId, call: FunctionCall, msg_id: Option<String>) {
        use libp2p::identity::PublicKey;

        #[rustfmt::skip]
        log::info!("executing certificates service for {}, call: {:?}", peer_id, call);

        // Check reply_to is defined
        let reply_to = if let Some(reply_to) = call.reply_to.clone() {
            reply_to
        } else {
            log::error!("reply_to undefined on {:?}", call);
            self.send_error_on_call(call, "reply_to is undefined".into());
            return;
        };

        // Extract public key from peer_id
        match peer_id.as_public_key() {
            Some(PublicKey::Ed25519(public_key)) => {
                // Load certificates from trust graph; TODO: are empty roots OK?
                let certs = self.kademlia.trust.get_all_certs(public_key, &[]);
                // Serialize certs to string
                let certs = certs.into_iter().map(|c| c.to_string()).collect::<Vec<_>>();
                let arguments = json!({"certificates": certs, "msg_id": msg_id });
                // Build reply
                let call = FunctionCall {
                    uuid: Self::uuid(),
                    target: Some(reply_to),
                    reply_to: Some(self.config.local_address()),
                    module: None,
                    fname: None,
                    arguments,
                    name: Some("reply on certificates".into()),
                    sender: self.config.local_address(),
                };
                // Send reply with certificates and msg_id
                self.call(call);
            }
            Some(pk) => {
                #[rustfmt::skip]
                log::error!("unsupported public key: expected ed25519, got {:?} {:?}", pk, call);
                self.send_error_on_call(call, "unsupported public key: expected ed25519".into());
                return;
            }

            None => {
                log::error!("can't extract public key from peer_id {:?}", call);
                self.send_error_on_call(call, "can't extract public key from peer_id".into());
                return;
            }
        }

        // Query closest peers for `peer_id`, then broadcast the call to found neighborhood
        self.replicate_to_neighbors(peer_id, call);
    }

    fn provide(&mut self, name: Address, call: FunctionCall) {
        use Protocol::*;

        let protocols = call.reply_to.as_ref().map(|addr| addr.protocols());

        match protocols.as_deref() {
            Some([Peer(p), cl @ Client(_), sig @ Signature(_), rem @ ..]) if self.is_local(p) => {
                // Verify signatures in address
                if let Err(err) = verify_address_signatures(call.reply_to.as_ref().unwrap()) {
                    log::warn!("Service register error {:?}: {:?}", call, err);
                    self.send_error_on_call(call, format!("signature error: {:?}", err));
                    return;
                }

                // provider ~ /peer/QmLocal/client/QmClient/signature/0xSig/service/QmService, or more complex
                let provider = self.config.local_address() / cl / sig / rem;

                // Insert provider to local hashmap
                let replaced = self.provided_names.insert(name.clone(), provider.clone());
                if let Some(replaced) = replaced {
                    #[rustfmt::skip]
                    log::warn!("Replaced name {:?} with {:?}, call: {}", replaced, provider, &call.uuid);
                }

                if let Err(err) = self.publish_name(&name, &provider) {
                    log::warn!("Service register error {:?} store error: {:?}", call, err);
                    self.send_error_on_call(call, format!("store error: {:?}", err));
                    return;
                }
                log::info!("Published a service {}: {:?}", name, call);
            }
            Some([Peer(p), ..]) if !self.is_local(p) => {
                // To avoid routing cycle (see discussion https://fluencelabs.slack.com/archives/C8FDH536W/p1588333361404100?thread_ts=1588331102.398600&cid=C8FDH536W)
                log::warn!("Service register error: non-local relay {:?}", call);
                self.send_error_on_call(call, "error: non-local relay".into());
            }
            Some([Peer(_), Client(_), ..]) => {
                // Peer is local, but no signature was specified
                log::warn!("Service register error: missing signature {:?}", call);
                self.send_error_on_call(call, "error: missing signature".into());
            }
            Some(other) => {
                let addr = other.iter().collect::<Address>();
                log::warn!("Service register error: unsupported reply_to {}", addr);
                self.send_error_on_call(call, "error: unsupported reply_to".into());
            }
            None => {
                // If there's no `reply_to`, then we don't know where to forward, so can't register
                log::warn!("Service register error: missing reply_to  in {:?}", call);
                self.send_error_on_call(call, "missing reply_to".into());
            }
        }
    }

    /// Find neighborhood of the key `peer_id`, then broadcast given call to found neighborhood
    fn replicate_to_neighbors(&mut self, peer_id: PeerId, mut call: FunctionCall) {
        if call.fname.as_ref().map_or(true, |m| m != "replicate") {
            // Mark call as a replication
            call.fname = Some("replicate".into());
            self.send_to_neighborhood(peer_id, call)
        }
    }

    fn reply_with<T: Serialize>(
        &mut self,
        call: FunctionCall,
        msg_id: Option<String>,
        data: (&str, T),
    ) {
        // Check reply_to is defined
        let reply_to = if let Some(reply_to) = call.reply_to.clone() {
            reply_to
        } else {
            log::error!("reply_to undefined on {:?}", call);
            self.send_error_on_call(call, "reply_to is undefined".into());
            return;
        };

        let arguments = json!({ "msg_id": msg_id, data.0: data.1 });
        // Build reply
        let call = FunctionCall {
            uuid: Self::uuid(),
            target: Some(reply_to),
            reply_to: Some(self.config.local_address()),
            fname: None,
            module: None,
            arguments,
            name: None,
            sender: self.config.local_address(),
        };
        self.call(call);
    }
}
