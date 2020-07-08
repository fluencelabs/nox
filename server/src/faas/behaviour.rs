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
use async_std::task::JoinHandle;
use faas_api::FunctionCall;
use fluence_faas::{FaaSError, FluenceFaaS, IValue};
use libp2p::core::connection::ConnectionId;
use libp2p::swarm::{
    protocols_handler::DummyProtocolsHandler, NetworkBehaviour, NetworkBehaviourAction,
    PollParameters,
};
use libp2p::PeerId;
use parity_multiaddr::Multiaddr;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};
use void::Void;

type FaasResult = Result<Vec<IValue>, FaaSError>;
type FutResult = (FluenceFaaS, WasmCall, FaasResult);
//noinspection RsUnresolvedReference
type Fut = JoinHandle<FutResult>;

struct WasmCall {
    module: String,
    function: String,
    arguments: Vec<IValue>,
    call: FunctionCall,
}

pub struct FaaSBehaviour {
    faases: HashMap<String, FluenceFaaS>,
    calls: Vec<WasmCall>,
    waker: Option<Waker>,
    futures: HashMap<String, Fut>,
}

impl FaaSBehaviour {
    #[allow(unused_variables, dead_code)]
    pub fn execute(module: String, function: String, arguments: Vec<IValue>, call: FunctionCall) {}

    fn create_faas(_call: &WasmCall) -> FluenceFaaS {
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
    ) -> Poll<NetworkBehaviourAction<Void, Self::OutEvent>> {
        self.waker = Some(cx.waker().clone());

        // Check there is a completed call
        let mut result = None;
        // let mut futures = std::mem::replace(&mut self.futures, HashMap::new());
        for (key, fut) in self.futures.iter_mut() {
            let fut = Pin::new(fut);
            if let Poll::Ready(r) = fut.poll(cx) {
                result = Some((key.clone(), r));
                break;
            }
        }
        // Remove completed future, reinsert faas, return result
        if let Some((uuid, result)) = result {
            self.futures.remove(&uuid);

            let (faas, call, result): FutResult = result;
            self.faases.insert(uuid, faas);

            return Poll::Ready(NetworkBehaviourAction::GenerateEvent((call.call, result)));
        }

        // Check if there's a work and a matching faas isn't busy
        let futures = &self.futures;
        let capacity = self.calls.capacity();
        let calls = std::mem::replace(&mut self.calls, Vec::with_capacity(capacity));
        let (busy, new_work): (Vec<_>, _) = calls
            .into_iter()
            .partition(|call| futures.contains_key(&call.module));
        self.calls.extend(busy);

        // Execute calls on faases
        for call in new_work {
            let uuid = call.module.clone();
            let mut faas = self
                .faases
                .remove(&uuid)
                .unwrap_or_else(|| Self::create_faas(&call));
            let future = task::spawn_blocking(move || {
                let result = faas.call_module(&call.module, &call.function, &call.arguments);
                (faas, call, result)
            });
            self.futures.insert(uuid, future);
        }

        Poll::Pending
    }
}
