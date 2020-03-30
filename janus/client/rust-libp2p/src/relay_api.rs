/*
 * Copyright 2019 Fluence Labs Limited
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

use libp2p::PeerId;
use multihash::Multihash;

// TODO: move to some package
// TODO: separate into RelayApi & DhtApi
pub trait RelayApi {
    fn relay_message(&mut self, src: PeerId, dst: PeerId, message: Vec<u8>);
    fn provide(&mut self, relay: PeerId, key: Multihash);
    fn find_providers(&mut self, relay: PeerId, client_id: PeerId, key: Multihash);
}
