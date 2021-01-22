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

use std::error::Error;
use std::fmt::{Display, Formatter};

pub(crate) type Result<T> = std::result::Result<T, KademliaError>;

#[derive(Debug)]
// TODO: implement Error trait
pub enum KademliaError {
    NoPeersFound,
    Timeout,
    Cancelled,
    NoKnownPeers,
}

impl Error for KademliaError {}

impl Display for KademliaError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            KademliaError::Timeout => write!(f, "KademliaError::Timeout"),
            KademliaError::Cancelled => write!(f, "KademliaError::Cancelled"),
            KademliaError::NoKnownPeers => write!(f, "KademliaError::NoKnownPeers"),
            KademliaError::NoPeersFound => write!(f, "KademliaError::NoPeersFound"),
        }
    }
}
