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
use faas_api::{Address, FunctionCall, Protocol};

impl FunctionRouter {
    // ####
    // ## Execution
    // ###

    pub(super) fn execute_builtin(&mut self, service: BuiltinService, call: FunctionCall) {
        // TODO: implement more BuiltinServices
        use Protocol::*;

        let protocols = call.reply_to.as_ref().map(|addr| addr.protocols());
        let name: Address = service.into();

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
