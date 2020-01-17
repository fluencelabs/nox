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

/// The main responsibility of the p2p layer of Janus is to keep the network state updated.
/// At the moment, it uses floodsub for that - all nodes are subscribed to the same topic and
/// notifies each other about any change on their side (like adding/removing new peers). In future
/// should be refactored to smth more scalable (like gossipsub when it will be merged tp libp2p).
pub mod behaviour;
mod message;
mod swarm_state_behaviour;
pub mod transport;
