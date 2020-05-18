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

use faas_api::FunctionCall;

/// Possible waiting states of the FunctionCall
#[derive(Debug)]
pub(super) enum WaitPeer {
    /// Wait for a given peer to become routable via Kademlia and forward call there
    Routable(FunctionCall),
    /// Wait for a given peer to become connected and forward call there
    Connected(FunctionCall),
    /// Get neighbourhood of a given PeerId, and send call to each peer there
    Neighborhood(FunctionCall),
}

impl WaitPeer {
    pub fn found(&self) -> bool {
        matches!(self, WaitPeer::Routable(_))
    }

    pub fn connected(&self) -> bool {
        matches!(self, WaitPeer::Connected(_))
    }

    pub fn neighborhood(&self) -> bool {
        matches!(self, WaitPeer::Neighborhood(_))
    }
}

impl Into<FunctionCall> for WaitPeer {
    fn into(self) -> FunctionCall {
        match self {
            WaitPeer::Routable(call) => call,
            WaitPeer::Connected(call) => call,
            WaitPeer::Neighborhood(call) => call,
        }
    }
}
