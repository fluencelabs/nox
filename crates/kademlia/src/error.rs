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

use thiserror::Error;

pub(crate) type Result<T> = std::result::Result<T, KademliaError>;

#[derive(Debug, Error)]
pub enum KademliaError {
    #[error("KademliaError::NoPeersFound")]
    NoPeersFound,
    #[error("KademliaError::PeerTimedOut")]
    PeerTimedOut,
    #[error("KademliaError::QueryTimedOut")]
    QueryTimedOut,
    #[error("KademliaError::Cancelled")]
    Cancelled,
    #[error("KademliaError::NoKnownPeers")]
    NoKnownPeers,
    #[error("KademliaError::PeerBanned")]
    PeerBanned,
}
