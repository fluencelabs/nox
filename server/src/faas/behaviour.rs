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
use fluence_faas::{FaaSError, FluenceFaaS, IValue, RawCoreModulesConfig};
use futures_util::future::BoxFuture;
use libp2p::core::connection::ConnectionId;
use libp2p::swarm::{
    protocols_handler::DummyProtocolsHandler, NetworkBehaviour, NetworkBehaviourAction,
    PollParameters,
};
use libp2p::PeerId;
use parity_multiaddr::Multiaddr;
use std::collections::HashMap;
use std::error::Error;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};
use uuid::Uuid;
use void::Void;

type Result<T> = std::result::Result<T, FaasExecError>;
type FaasResult = Vec<IValue>;
type FutResult = (FluenceFaaS, FunctionCall, Result<WasmResult>);
type Fut = BoxFuture<'static, FutResult>;

#[derive(Debug)]
pub enum FaasExecError {
    NoSuchInstance(String),
    FaaS(FaaSError),
}

impl Error for FaasExecError {}
impl From<FaaSError> for FaasExecError {
    fn from(err: FaaSError) -> Self {
        FaasExecError::FaaS(err)
    }
}

impl std::fmt::Display for FaasExecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FaasExecError::NoSuchInstance(service_id) => {
                write!(f, "FaaS instance {} not found", service_id)
            }
            FaasExecError::FaaS(err) => err.fmt(f),
        }
    }
}

pub enum WasmResult {
    FaaSCreated { service_id: String },
    Returned(FaasResult),
}

#[allow(dead_code)]
pub enum WasmCall {
    /// Call to the FaaS instance specified by `service_id`
    Call {
        /// UUID of the FaaS instance
        service_id: String,
        /// Module to call function on
        module: String,
        /// Function name to call
        function: String,
        /// Arguments for the function
        arguments: Vec<IValue>,
        /// FunctionCall that caused this WasmCall, returned to caller as is
        call: FunctionCall,
    },
    Create {
        /// Context: list of modules to load
        module_names: Vec<String>,
        /// FunctionCall that caused this WasmCall, returned to caller as is
        call: FunctionCall,
    },
}

impl WasmCall {
    pub fn is_create(&self) -> bool {
        match self {
            WasmCall::Create { .. } => true,
            WasmCall::Call { .. } => false,
        }
    }

    pub fn service_id(&self) -> Option<&str> {
        match self {
            WasmCall::Call { service_id, .. } => Some(service_id),
            WasmCall::Create { .. } => None,
        }
    }
}

pub struct FaaSBehaviour {
    faases: HashMap<String, FluenceFaaS>,
    calls: Vec<WasmCall>,
    waker: Option<Waker>,
    futures: HashMap<String, Fut>,
    config: RawCoreModulesConfig,
}

impl FaaSBehaviour {
    #[allow(dead_code)]
    pub fn new(config: RawCoreModulesConfig) -> Self {
        Self {
            faases: <_>::default(),
            calls: <_>::default(),
            waker: <_>::default(),
            futures: <_>::default(),
            config,
        }
    }

    #[allow(dead_code)]
    pub fn execute(&mut self, call: WasmCall) {
        self.calls.push(call);

        if let Some(waker) = self.waker.take() {
            waker.wake()
        }
    }

    fn create_faas(&self, module_names: Vec<String>) -> Result<(String, FluenceFaaS)> {
        let mut module_names = module_names.into_iter().collect();
        let uuid = Uuid::new_v4().to_string();
        let config = self.config.clone();

        let faas = FluenceFaaS::with_module_names(&mut module_names, config)?;
        Ok((uuid, faas))
    }

    /// Spawns tasks for calls execution and creates new FaaS-es until an error happens
    fn execute_calls<I>(
        &mut self,
        new_work: &mut I,
    ) -> std::result::Result<(), (FunctionCall, FaasExecError)>
    where
        I: Iterator<Item = WasmCall>,
    {
        new_work.try_fold((), |_, call| {
            match call {
                // Request to create FaaS instance with given module_names
                WasmCall::Create { module_names, call } => {
                    let (service_id, faas) = self
                        .create_faas(module_names)
                        .map_err(|e| (call.clone(), e))?;

                    let result = WasmResult::FaaSCreated {
                        // TODO: excess clone, data duplication:
                        //  service_id stored in futures as key and as value in FaaSCreated :(
                        service_id: service_id.clone(),
                    };
                    let result = (faas, call, Ok(result));

                    // Save completed future so result is returned on the next poll
                    let future = async_std::future::ready(result);
                    self.futures.insert(service_id, Box::pin(future));

                    Ok(())
                }
                // Request to call function on an existing FaaS instance
                #[rustfmt::skip]
                WasmCall::Call { service_id, module, function, arguments, call } => {
                    // Take existing faas
                    let mut faas = self
                        .faases
                        .remove(&service_id)
                        .ok_or_else(|| (call.clone(), FaasExecError::NoSuchInstance(service_id.clone())))?;
                    // Spawn a task that will call wasm function
                    let future = task::spawn_blocking(move || {
                        let result = faas.call_module(&module, &function, &arguments);
                        let result = result.map(|r| WasmResult::Returned(r)).map_err(|e| e.into());
                        (faas, call, result)
                    });
                    // Save future for the next poll
                    self.futures.insert(service_id, Box::pin(future));

                    Ok(())
                }
            }
        })
    }
}

impl NetworkBehaviour for FaaSBehaviour {
    type ProtocolsHandler = DummyProtocolsHandler;
    type OutEvent = (FunctionCall, Result<WasmResult>);

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
                // TODO: excess clone (possible to obtain via futures.remove_entry
                result = Some((key.clone(), r));
                break;
            }
        }
        // Remove completed future, reinsert faas, return result
        if let Some((service_id, result)) = result {
            self.futures.remove(&service_id);

            let (faas, call, result): FutResult = result;
            self.faases.insert(service_id, faas);

            return Poll::Ready(NetworkBehaviourAction::GenerateEvent((call, result)));
        }

        // Check if there's a work and a matching faas isn't busy
        let capacity = self.calls.capacity();
        let calls = std::mem::replace(&mut self.calls, Vec::with_capacity(capacity));
        let (busy, new_work): (Vec<_>, _) = calls.into_iter().partition(|call| {
            // return true if service is to be created, or there is no existing work for that service_id
            call.is_create() || self.futures.contains_key(call.service_id().unwrap())
        });
        self.calls.extend(busy);

        // Execute calls on faases
        let mut new_work = new_work.into_iter();
        // Iterate until an error is "found"
        let err = self.execute_calls(&mut new_work);

        // If error happened during call execution
        if let Err((call, err)) = err {
            // Put left work back to the queue
            self.calls.extend(new_work);
            // Return the error
            return Poll::Ready(NetworkBehaviourAction::GenerateEvent((call, Err(err))));
        }

        Poll::Pending
    }
}

#[cfg(test)]
mod tests {
    static TEST_MODULE: &str = "./tests/artifacts/test_module_wit.wasi.wasm";

    use super::*;
    // use async_std::task;
    use fluence_faas::RawCoreModulesConfig;
    use futures::StreamExt;
    use futures::{executor::block_on, future::poll_fn};
    use libp2p::core::transport::dummy::{DummyStream, DummyTransport};
    use libp2p::mplex::Multiplex;
    use libp2p::{PeerId, Swarm};

    #[test]
    fn call_multiple_faases() {
        let mut tmp = std::env::temp_dir();
        tmp.push("wasm_modules/");
        std::fs::create_dir_all(&tmp).expect("create tmp dir");
        println!("test module: {:?}", std::fs::canonicalize(TEST_MODULE));
        std::fs::copy(TEST_MODULE, tmp.join("test_module.wasm")).expect("copy test module wasm");

        let mut config: RawCoreModulesConfig = <_>::default();
        config.core_modules_dir = Some(tmp.to_string_lossy().into());

        let behaviour = FaaSBehaviour::new(config);
        let transport = DummyTransport::<(PeerId, Multiplex<DummyStream>)>::new();
        let mut swarm = Swarm::new(transport, behaviour, PeerId::random());

        let call = FunctionCall {
            uuid: "uuid".to_string(),
            target: None,
            reply_to: None,
            module: None,
            fname: None,
            arguments: Default::default(),
            name: None,
            sender: Default::default(),
        };

        swarm.execute(WasmCall::Create {
            module_names: vec!["test_module.wasm".to_string()],
            call: call.clone(),
        });

        let (returned_call, result) = block_on(async move {
            poll_fn(|ctx| {
                loop {
                    match swarm.poll_next_unpin(ctx) {
                        Poll::Ready(Some((call, result))) => return Poll::Ready((call, result)),
                        _ => break,
                    }
                }
                Poll::Pending
            })
            .await
        });

        assert_eq!(returned_call, call);
        assert!(result.is_ok());
    }
}
