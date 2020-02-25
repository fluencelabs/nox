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

use serde::{Deserialize, Serialize};

/// Relay event is just a data that need to be relayed from a peer of `src_id` to a peer of `dst_id`.
#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq)]
pub struct RelayEvent {
    // TODO: use PeerId instead of Vec<u8>. Currently it's blocked by implementing serde traits.
    pub src_id: Vec<u8>,
    // TODO: use PeerId instead of Vec<u8>. Currently it's blocked by implementing serde traits.
    pub dst_id: Vec<u8>,
    pub data: Vec<u8>,
}
