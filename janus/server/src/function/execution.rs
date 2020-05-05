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
use super::FunctionRouter;
use faas_api::{Address, FunctionCall, Protocol};

impl FunctionRouter {
    // ####
    // ## Execution
    // ###

    pub(super) fn execute_builtin(&mut self, service: BuiltinService, call: FunctionCall) {
        use Protocol::*;

        let protocols = call.reply_to.as_ref().map(|addr| addr.protocols());
        let name: Address = service.into();

        match protocols.as_deref() {
            Some([Peer(id), c @ Client(_), rem @ ..]) if self.is_local(id) => {
                // forward_to ~ /peer/QmLocal/client/QmClient/service/QmService, or more complex
                let local: Address = Peer(self.peer_id.clone()).into();
                let provider = local.append(c).append_protos(rem);

                #[rustfmt::skip]
                let replaced = self.provided_names.insert(name.clone(), provider.clone());
                if let Some(replaced) = replaced {
                    log::warn!(
                        "Replaced name {:?} with {:?} due to call {}",
                        replaced,
                        provider,
                        &call.uuid
                    );
                }

                log::info!("Published a service {}: {:?}", name, call);
                self.publish_name(name, provider);
            }
            Some(&[Peer(_), ..]) => {
                // To avoid routing cycle (see https://gist.github.com/folex/61700dd6afa14fbe3d1168e04dfe2661 for bug postmortem)
                log::warn!("Service register error: non-local relay {:?}", call);
                self.send_error_on_call(call, "error: non-local relay".into());
            }
            Some(other) => {
                let addr = other.iter().collect::<Address>();
                log::warn!("Service register error: unsupported reply_to {}", addr);
                self.send_error_on_call(call, "error: unsupported reply_to".into());
            }
            None => {
                // If there's no `reply_to`, then we don't know where to forward, so can't register
                log::warn!("reply_to was not defined in {:?}", call);
                self.send_error_on_call(
                    call,
                    "reply_to must be defined when calling 'provide' service".into(),
                );
            }
        }
    }
}
