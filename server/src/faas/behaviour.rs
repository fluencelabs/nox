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

use async_std::task;
use faas_api::FunctionCall;
use fluence_faas::FluenceFaaS;
use libp2p::swarm::{
    protocols_handler::DummyProtocolsHandler, NetworkBehaviour, NetworkBehaviourAction,
    PollParameters,
};
use libp2p::PeerId;
use parity_multiaddr::Multiaddr;
use std::collections::HashMap;
use std::future::Future;
use std::task::{Context, Poll, Waker};
use std::thread::JoinHandle;
use void::Void;

type FaasResult = Vec<InterfaceValue>;

pub struct FaaSBehaviour {
    faases: HashMap<String, FluenceFaaS>,
    calls: Vec<FunctionCall>,
    waker: Option<Waker>,
    futures: HashMap<String, JoinHandle<(FluenceFaaS, FunctionCall, FaasResult)>>,
}

impl FaaSBehaviour {
    pub fn execute(call: FunctionCall) {
        assert!(call.module.is_some())
    }

    pub fn create_faas(_call: &FunctionCall) -> FluenceFaaS {
        unimplemented!()
    }
}

impl NetworkBehaviour for FaaSBehaviour {
    type ProtocolsHandler = DummyProtocolsHandler;
    type OutEvent = (FunctionCall, FaasResult);

    fn new_handler(&mut self) -> Self::ProtocolsHandler {
        Default::default()
    }

    fn addresses_of_peer(&mut self, _: &PeerId) -> Vec<Multiaddr> {
        vec![]
    }

    fn inject_connected(&mut self, _: &PeerId) {}

    fn inject_disconnected(&mut self, _: &PeerId) {}

    fn inject_event(&mut self, _: PeerId, _: ConnectionId, _: Void) {}

    fn poll(
        &mut self,
        cx: &mut Context<'_>,
        _: &mut impl PollParameters,
    ) -> Poll<NetworkBehaviourAction<_, Self::OutEvent>> {
        self.waker = Some(cx.waker().clone());

        // Check there is a completed call
        let mut result = None;
        let mut uuid = None;
        for (key, fut) in self.futures.iter_mut() {
            if let Poll::Ready(r) = fut.poll() {
                uuid = Some(key);
                result = Some(r);
                break;
            }
        }
        // Remove completed future, reinsert faas, return result
        if idx >= 0 {
            self.futures.remove(idx);

            let (faas, call, result) = result.unwrap();
            self.faases.insert(call.module, faas);

            return Poll::Ready(NetworkBehaviourAction::GenerateEvent((call, result)));
        }

        // Check if there's a work and a matching faas isn't busy
        let futures = &self.futures;
        let mut new_work = vec![];
        self.calls.retain(|call| {
            let uuid = call.module.expect("module must be defined");
            let busy = futures.contains_key(&uuid);
            if !busy {
                new_work.push(call)
            }

            busy
        });

        // Execute calls on faases
        for call in new_work {
            let uuid = call.module.expect("module must be defined");
            let mut faas = self
                .faases
                .remove(&uuid)
                .unwrap_or_else(|| Self::create_faas(&call));
            let future = task::spawn_blocking(move || faas.execute(call));
            self.futures.insert(uuid, future);
        }

        Poll::Pending
    }
}
