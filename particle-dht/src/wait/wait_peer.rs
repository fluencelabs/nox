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

/// FunctionCall waiting for something happen to a peer possible states
#[derive(Debug)]
pub(super) enum WaitPeer<T> {
    /// Wait for a given peer to become routable via Kademlia and forward call there
    Routable(T),
    /// Wait for a given peer to become connected and forward call there
    Connected(T),
    /// Get neighbourhood of a given PeerId, and send call to each peer there
    Neighborhood(T),
}

impl<T> WaitPeer<T> {
    pub fn found(&self) -> bool {
        matches!(self, WaitPeer::Routable(_))
    }

    pub fn connected(&self) -> bool {
        matches!(self, WaitPeer::Connected(_))
    }

    pub fn neighborhood(&self) -> bool {
        matches!(self, WaitPeer::Neighborhood(_))
    }

    pub fn value(self) -> T {
        match self {
            WaitPeer::Routable(value) => value,
            WaitPeer::Connected(value) => value,
            WaitPeer::Neighborhood(value) => value,
        }
    }
}
