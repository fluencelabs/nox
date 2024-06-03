/*
 * Copyright 2024 Fluence DAO
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

use crate::peer_id;
use libp2p_identity::PeerId;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize, Debug)]
#[serde(tag = "scope_type", content = "scope_value")]
pub enum PeerScope {
    WorkerId(WorkerId),
    Host,
}

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd, Debug, Serialize, Deserialize)]
pub struct WorkerId(
    #[serde(
        serialize_with = "peer_id::serde::serialize",
        deserialize_with = "peer_id::serde::deserialize"
    )]
    PeerId,
);

impl From<PeerId> for WorkerId {
    fn from(value: PeerId) -> Self {
        WorkerId(value)
    }
}

impl From<WorkerId> for PeerId {
    fn from(value: WorkerId) -> Self {
        value.0
    }
}

impl fmt::Display for WorkerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.to_base58().fmt(f)
    }
}
