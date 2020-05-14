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

use super::address_signature::verify_address_signatures;
use super::builtin_service::BuiltinService;
use super::FunctionRouter;
use faas_api::{service, Address, FunctionCall, Protocol};
use libp2p::PeerId;

impl FunctionRouter {
    // ####
    // ## Execution
    // ###

    /// Execute call on builtin service: "provide" or "certificates"
    /// `ttl` – time to live, if `0`, "certficiates" service won't send call to neighborhood
    pub(super) fn execute_builtin(
        &mut self,
        service: BuiltinService,
        call: FunctionCall,
        ttl: usize,
    ) {
        match service {
            BuiltinService::DelegateProviding { service_id } => {
                self.provide(service!(service_id), call)
            }
            BuiltinService::GetCertificates { peer_id, msg_id } => {
                self.get_certificates(peer_id, call, msg_id, ttl)
            }
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
        use serde_json::json;
        use Protocol::Peer;

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
        if let Some(public_key) = peer_id.as_public_key() {
            if let PublicKey::Ed25519(public_key) = public_key {
                // Load certificates from trust graph; TODO: are empty roots OK?
                let certs = self.kademlia.trust.get_all_certs(public_key, &[]);
                // Serialize certs to string
                let certs = certs.into_iter().map(|c| c.to_string()).collect::<Vec<_>>();
                let arguments = json!({"certificates": certs, "msg_id": msg_id });
                // Build reply
                let call = FunctionCall {
                    uuid: Self::uuid(),
                    target: Some(reply_to),
                    reply_to: Some(Peer(self.peer_id.clone()).into()),
                    name: Some("reply on certificates".into()),
                    arguments,
                };
                // Send reply with certificates and msg_id
                self.call(call);
            } else {
                log::error!("unsupported public key: expected ed25519 {:?}", call);
                self.send_error_on_call(call, "unsupported public key: expected ed25519".into());
                return;
            }
        } else {
            log::error!("can't extract public key from peer_id {:?}", call);
            self.send_error_on_call(call, "can't extract public key from peer_id".into());
            return;
        }

        // If ttl == 0, don't send to neighborhood
        if ttl > 0 {
            // Query closest peers for `peer_id`, then broadcast the call to found neighborhood
            self.send_to_neighborhood(peer_id, call)
        }
    }

    fn provide(&mut self, name: Address, call: FunctionCall) {
        // TODO: implement more BuiltinServices
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

                // Build provider address
                let local: Address = Peer(self.peer_id.clone()).into();
                // provider ~ /peer/QmLocal/client/QmClient/service/QmService, or more complex
                let provider = local.append(cl).append(sig).append_protos(rem);

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
