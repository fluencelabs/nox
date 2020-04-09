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

#[allow(clippy::module_inception)]
mod node_service;
mod p2p {
    mod behaviour;
    mod transport;

    pub use behaviour::P2PBehaviour;
    pub use transport::build_transport;
}

pub mod function {
    mod router;

    mod protocol {
        pub mod message;
        pub mod upgrade;
    }

    mod call;
    mod provider;

    pub use call::{Address, FunctionCall};
    pub use protocol::message::ProtocolMessage;
    pub use provider::Provider;
    pub use router::FunctionRouter;
    pub(crate) use router::SwarmEventType;

    #[cfg(test)]
    pub use call::test::gen_function_call;
}

pub use node_service::NodeService;
pub use p2p::build_transport;
pub use p2p::P2PBehaviour;
