/*
 * Copyright 2021 Fluence Labs Limited
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

use host_closure::{Args, JError, ParticleParameters};
use libp2p::core::Multiaddr;
use serde_json::{json, Value as JValue};

#[derive(Default)]
pub struct IpfsState {
    multiaddr: Option<Multiaddr>,
}

impl IpfsState {
    pub fn new(multiaddr: impl Into<Option<Multiaddr>>) -> Self {
        Self {
            multiaddr: multiaddr.into(),
        }
    }
}

impl IpfsState {
    pub fn set_multiaddr(
        &mut self,
        args: Args,
        params: ParticleParameters,
        management_peer_id: &str,
    ) -> Result<Option<JValue>, JError> {
        let mut args = args.function_args.into_iter();

        if params.init_user_id != management_peer_id {
            let err = json!({
                "user": params.init_user_id,
                "function": "set_multiaddr",
                "reason": "only management peer id can set ipfs multiaddr",
            });
            return Err(JError(err));
        }

        let multiaddr: Multiaddr = Args::next("multiaddr", &mut args)?;
        let old = self.multiaddr.replace(multiaddr);

        Ok(old.map(|addr| json!(addr)))
    }

    pub fn get_multiaddr(&mut self) -> Option<JValue> {
        self.multiaddr.map(|addr| json!(addr))
    }

    pub fn clear_multiaddr(
        &mut self,
        params: ParticleParameters,
        management_peer_id: &str,
    ) -> Result<JValue, JError> {
        if params.init_user_id != management_peer_id {
            return Err(JError(json!({
                "user": params.init_user_id,
                "function": "clear_multiaddr",
                "reason": "only management peer id can clear ipfs multiaddr",
            })));
        }

        let old = self.multiaddr.take();

        Ok(json!(old.is_some()))
    }
}
