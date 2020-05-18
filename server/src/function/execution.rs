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
        AddCertificates, BuiltinService, DelegateProviding, GetCertificates, Identify,
    },
    FunctionRouter,
};
use faas_api::{service, Address, FunctionCall, Protocol};
use itertools::Itertools;
use libp2p::PeerId;
use serde_json::json;
use trust_graph::Certificate;

impl FunctionRouter {
    /// Execute call on builtin service: "provide" or "certificates"
    /// `ttl` – time to live, if `0`, "certificates" and "add_certificates" services
    ///  won't send call to neighborhood
    pub(super) fn execute_builtin(
        &mut self,
        service: BuiltinService,
        call: FunctionCall,
        ttl: usize,
    ) {
        use BuiltinService as BS;

        match service {
            BS::DelegateProviding(DelegateProviding { service_id }) => {
                self.provide(service!(service_id), call)
            }
            BS::GetCertificates(GetCertificates { peer_id, msg_id }) => {
                self.get_certificates(peer_id, call, msg_id, ttl)
            }
            BS::AddCertificates(AddCertificates {
                peer_id,
                certificates,
                msg_id,
            }) => self.add_certificates(peer_id, certificates, call, msg_id, ttl),
            BS::Identify(Identify { msg_id }) => self.identify(call, msg_id),
        }
    }

    fn identify(&mut self, call: FunctionCall, msg_id: Option<String>) {
        log::info!("executing identify, call: {:?}", &call);

        // Check reply_to is defined
        let reply_to = if let Some(reply_to) = call.reply_to.clone() {
            reply_to
        } else {
            log::error!("reply_to undefined on {:?}", call);
            self.send_error_on_call(call, "reply_to is undefined".into());
            return;
        };

        let addrs: Vec<_> = self
            .config
            .listening_addresses
            .iter()
            .map(ToString::to_string)
            .collect();
        let arguments = json!({ "msg_id": msg_id, "addresses": addrs });
        // Build reply
        let call = FunctionCall {
            uuid: Self::uuid(),
            target: Some(reply_to),
            reply_to: Some(self.config.local_address()),
            name: Some("reply on identify".into()),
            arguments,
        };
        self.call(call);
    }

    fn add_certificates(
        &mut self,
        peer_id: PeerId,
        certificates: Vec<Certificate>,
        call: FunctionCall,
        msg_id: Option<String>,
        ttl: usize,
    ) {
        #[rustfmt::skip]
        log::info!(
            "executing add_certificates of {} certs for {}, ttl: {}, call: {:?}", 
            certificates.len(), peer_id, ttl, call
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
                name: Some("reply on add_certificates".into()),
                arguments,
            };
            self.call(call);
        } else {
            log::warn!(
                "add_certificates reply_to is empty {}, can't send reply",
                call.uuid
            );
        }

        // Finally – broadcast that call to the neighborhood if ttl > 0
        // NOTE: not filtering errors, other nodes may succeed where we failed
        if ttl > 0 {
            self.send_to_neighborhood(peer_id, call)
        }
    }

    fn get_certificates(
        &mut self,
        peer_id: PeerId,
        call: FunctionCall,
        msg_id: Option<String>,
        ttl: usize,
    ) {
        use libp2p::identity::PublicKey;

        #[rustfmt::skip]
        log::info!("executing certificates service for {}, ttl: {}, call: {:?}", peer_id, ttl, call);

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
                    name: Some("reply on certificates".into()),
                    arguments,
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

        // If ttl == 0, don't send to neighborhood
        if ttl > 0 {
            // Query closest peers for `peer_id`, then broadcast the call to found neighborhood
            self.send_to_neighborhood(peer_id, call)
        }
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

                log::info!("Published a service {}: {:?}", name, call);
                self.publish_name(name, provider);
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
}
