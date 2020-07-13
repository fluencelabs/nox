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

use super::address_signature::SignatureError;
use super::{
    address_signature::verify_address_signatures,
    builtin_service::{
        AddCertificates, BuiltinService, GetCertificates, GetInterface, Identify, Provide,
    },
    errors::CallErrorKind::*,
    CallError, ErrorData, FunctionRouter, ResolvedFunction,
};
use crate::faas::FaaSCall;
use faas_api::{provider, Address, FunctionCall, Protocol};
use fluence_faas::IValue;
use libp2p::PeerId;
use serde::Serialize;
use serde_json::{json, Value};
use trust_graph::Certificate;

impl FunctionRouter {
    /// Execute call on builtin service: "provide", "certificates", etc
    /// `ttl` – time to live, if `0`, then "certificates" and "add_certificates" services
    ///  won't send call to neighborhood
    pub(super) fn execute_builtin(
        &mut self,
        service: BuiltinService,
        call: FunctionCall,
    ) -> Result<(), CallError> {
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
                unimplemented!("get_interface not implemented");
                Err(call.error(InvalidArguments {
                    error: "get_interface not implemented".to_string(),
                }))
                // match serde_json::to_value(self.faas.get_interface()) {
                //     Ok(interface) => self.reply_with(call, msg_id, ("interface", interface)),
                //     Err(err) => Err(call.error(FaasInterfaceSerialization(err))),
                // }
            }
        }
    }

    /// Execute `function` on `module` in the FluenceFaaS
    pub(super) fn execute_wasm(
        &mut self,
        function: ResolvedFunction,
        call: FunctionCall,
    ) -> Result<(), CallError> {
        let ResolvedFunction {
            module,
            function,
            service_id,
        } = function;

        let is_null = call.arguments.is_null();
        let is_empty_arr = call.arguments.as_array().map_or(false, |a| a.is_empty());
        let is_empty_obj = call.arguments.as_object().map_or(false, |m| m.is_empty());
        let arguments = if !is_null && !is_empty_arr && !is_empty_obj {
            Some(
                fluence_faas::to_interface_value(&call.arguments).map_err(|e| {
                    call.clone().error(InvalidArguments {
                        error: format!("can't parse arguments as array of interface types: {}", e),
                    })
                })?,
            )
        } else {
            None
        };

        let arguments = match arguments {
            Some(IValue::Record(arguments)) => Ok(arguments.into_vec()),
            // Convert null, [] and {} into vec![]
            None => Ok(vec![]),
            other => Err(call.clone().error(InvalidArguments {
                error: format!("expected array of interface values: got {:?}", other),
            })),
        }?;

        let faas_call = FaaSCall::Call {
            service_id,
            module,
            function,
            arguments,
            call,
        };

        self.faas.execute(faas_call);
        Ok(())

        // TODO: retrieve results from faas

        // // Handle empty result manually because `from_interface_values` doesn't support empty vec
        // let result = if !result.is_empty() {
        //     fluence_faas::from_interface_values(&result)
        //         .map_err(|e| call.clone().error(ResultSerializationFailed(e.to_string())))?
        // } else {
        //     Value::Null
        // };
        //
        // self.reply_with(call, None, ("result", result))
    }

    fn add_certificates(
        &mut self,
        peer_id: PeerId,
        certificates: Vec<Certificate>,
        call: FunctionCall,
        msg_id: Option<String>,
    ) -> Result<(), CallError> {
        #[rustfmt::skip]
        log::info!(
            "executing add_certificates of {} certs for {}, call: {:?}", 
            certificates.len(), peer_id, call
        );

        // Calculate current time in Duration
        let time = trust_graph::current_time();
        // Add each certificate, and collect errors
        let results: Vec<_> = certificates
            .into_iter()
            .map(|cert| match self.kademlia.trust.add(cert.clone(), time) {
                Ok(_) => Ok(cert),
                Err(e) => Err((cert, e)),
            })
            .collect();

        // If there are any errors, send these errors in a single message
        if results.iter().any(Result::is_err) {
            let failed = results.into_iter().flat_map(Result::err).collect();
            return Err(call.error(AddCertificates(failed)));
        }

        // If there are no errors, send reply marking success
        let reply_to = ok_get!(call.reply_to.as_ref()).clone();
        log::debug!("add_certificates success uuid {}", call.uuid);

        let status = format!("{} certs added", results.len());
        let arguments = json!({ "msg_id": msg_id, "status": status });
        let name = "reply on add_certificates".to_string();
        // Send reply
        let reply = FunctionCall::reply(reply_to, self.config.local_address(), arguments, name);
        self.call(reply);

        // Finally – broadcast that call to the neighborhood if it wasn't already a replication
        // NOTE: not filtering errors, other nodes may succeed where we failed
        Ok(self.replicate_to_neighbors(peer_id, call))
    }

    fn get_certificates(
        &mut self,
        peer_id: PeerId,
        call: FunctionCall,
        msg_id: Option<String>,
    ) -> Result<(), CallError> {
        use libp2p::identity::PublicKey;

        #[rustfmt::skip]
        log::info!("executing certificates service for {}, call: {:?}", peer_id, call);

        let reply_to = call.reply_to.clone();
        let reply_to = reply_to.ok_or_else(|| call.clone().error(MissingReplyTo))?;

        // Extract public key from peer_id
        let public_key = peer_id
            .as_public_key()
            .ok_or_else(|| call.clone().error(MissingPublicKey))
            .and_then(|pk| match pk {
                PublicKey::Ed25519(public_key) => Ok(public_key),
                _ => Err(call.clone().error(UnsupportedPublicKey)),
            })?;

        // Load certificates from trust graph; TODO: are empty roots OK?
        let certs = self.kademlia.trust.get_all_certs(public_key, &[]);
        // Serialize certs to string
        let certs = certs.into_iter().map(|c| c.to_string()).collect::<Vec<_>>();
        let arguments = json!({"certificates": certs, "msg_id": msg_id });
        let name = "reply on certificates".to_string();
        // Send reply with certificates and msg_id
        let reply = FunctionCall::reply(reply_to, self.config.local_address(), arguments, name);
        self.call(reply);

        // Query closest peers for `peer_id`, then broadcast the call to found neighborhood
        self.replicate_to_neighbors(peer_id, call);

        Ok(())
    }

    fn provide(&mut self, name: Address, call: FunctionCall) -> Result<(), CallError> {
        use Protocol::*;

        let protocols = call.reply_to.as_ref().map(|addr| addr.protocols());

        match protocols.as_deref() {
            Some([Peer(p), cl @ Client(_), sig @ Signature(_), rem @ ..]) if self.is_local(p) => {
                // Verify signatures in address
                let reply_to = call.reply_to.as_ref().unwrap();
                verify_address_signatures(reply_to).map_err(|e| call.clone().error(e))?;

                // provider ~ /peer/QmLocal/client/QmClient/signature/0xSig/service/QmService, or more complex
                let provider = self.config.local_address() / cl / sig / rem;

                // Insert provider to local hashmap
                let replaced = self.provided_names.insert(name.clone(), provider.clone());
                if let Some(replaced) = replaced {
                    #[rustfmt::skip]
                    log::warn!("Replaced name {:?} with {:?}, call: {}", replaced, provider, &call.uuid);
                }

                self.publish_name(&name, &provider)
                    .map_err(|e| call.clone().error(e))?;

                log::info!("Published a service {}: {:?}", name, call);
                Ok(())
            }
            // To avoid routing cycle (see discussion https://fluencelabs.slack.com/archives/C8FDH536W/p1588333361404100?thread_ts=1588331102.398600&cid=C8FDH536W)
            Some([Peer(p), ..]) if !self.is_local(p) => Err(call.error(NonLocalRelay)),
            // Peer is local, but no signature was specified
            Some([Peer(_), Client(_), ..]) => Err(call.error(SignatureError::MissingSignature)),
            Some(other) => Err(call.error(UnsupportedReplyTo(other.iter().collect()))),
            // If there's no `reply_to`, then we don't know where to forward, so can't register
            None => Err(call.error(MissingReplyTo)),
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

    /// Send reply on a given call with given msg_id and data in arguments
    /// If data or msg_id is empty, it's not included in the arguments
    fn reply_with<T: Serialize>(
        &mut self,
        call: FunctionCall,
        msg_id: Option<String>,
        data: (&str, T),
    ) -> Result<(), CallError> {
        let reply_to = call.reply_to.clone();
        let reply_to = reply_to.ok_or_else(|| call.error(MissingReplyTo))?;

        let mut args = serde_json::Map::new();

        // Include `data` if it's not empty
        let value = json!(data.1);
        if !value.is_null() {
            args.insert(data.0.into(), value);
        }

        // Include `msg_id` if it's not empty
        if let Some(msg_id) = msg_id {
            args.insert("msg_id".into(), msg_id.into());
        }

        // Build JSON object and send in reply
        let args = Value::Object(args);
        let call = FunctionCall::reply(reply_to, self.config.local_address(), args, None);
        Ok(self.call(call))
    }
}
