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
use fluence_faas::{FaaSError, FaaSInterface, FluenceFaaS, IValue, RawCoreModulesConfig};
use futures_util::future::BoxFuture;
use libp2p::core::connection::ConnectionId;
use libp2p::swarm::{
    protocols_handler::DummyProtocolsHandler, NetworkBehaviour, NetworkBehaviourAction,
    PollParameters,
};
use libp2p::PeerId;
use parity_multiaddr::Multiaddr;
use serde::ser::Error as SerError;
use serde::{Serialize, Serializer};
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};
use uuid::Uuid;
use void::Void;

type Result<T> = std::result::Result<T, FaaSExecError>;
type FutResult = (Option<FluenceFaaS>, FunctionCall, Result<FaaSCallResult>);
type Fut = BoxFuture<'static, FutResult>;

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
/// Result of executing FaasCall
pub enum FaaSCallResult {
    /// FaaS was created with this `service_id`
    FaaSCreated { service_id: String },
    #[serde(serialize_with = "FaaSCallResult::serialize_returned")]
    /// Call to faas returned this result
    Returned(Vec<IValue>),
}

impl FaaSCallResult {
    fn serialize_returned<S>(
        value: &[IValue],
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if value.is_empty() {
            Value::Null.serialize(serializer)
        } else {
            let value = fluence_faas::from_interface_values(&value)
                .map_err(|e| SerError::custom(format!("Failed to serialize result: {}", e)))?;

            Value::serialize(&value, serializer)
        }
    }
}

#[derive(Debug, Clone)]
/// Call to FaaS
pub enum FaaSCall {
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
    /// Request to create new FaaS instance with given `module_names`
    Create {
        /// Context: list of modules to load on creation
        module_names: Vec<String>,
        /// FunctionCall that caused this WasmCall, returned to caller as is
        call: FunctionCall,
    },
}

impl FaaSCall {
    /// Whether this call is of `Create` type
    pub fn is_create(&self) -> bool {
        matches!(self, FaaSCall::Create { .. })
    }

    pub fn service_id(&self) -> Option<&str> {
        match self {
            FaaSCall::Call { service_id, .. } => Some(service_id),
            FaaSCall::Create { .. } => None,
        }
    }
}

/// Behaviour that manages FaaS instances: create, pass calls, poll for results
pub struct FaaSBehaviour {
    /// Created instances
    //TODO: when to delete an instance?
    faases: HashMap<String, FluenceFaaS>,
    /// Incoming calls waiting to be processed
    calls: Vec<FaaSCall>,
    /// Context waker, used to trigger `poll`
    waker: Option<Waker>,
    /// Pending futures: service_id -> future
    futures: HashMap<String, Fut>,
    /// Config to create FaaS instances with
    config: RawCoreModulesConfig,
}

impl FaaSBehaviour {
    pub fn new(config: RawCoreModulesConfig) -> Self {
        Self {
            faases: <_>::default(),
            calls: <_>::default(),
            waker: <_>::default(),
            futures: <_>::default(),
            config,
        }
    }

    /// Execute given `call`
    pub fn execute(&mut self, call: FaaSCall) {
        self.calls.push(call);
        self.wake();
    }

    /// Get interface of a FaaS instance specified by `service_id`
    pub fn get_interface(&self, service_id: &str) -> Result<FaaSInterface<'_>> {
        let faas = self
            .faases
            .get(service_id)
            .ok_or_else(|| FaaSExecError::NoSuchInstance(service_id.to_string()))?;

        Ok(faas.get_interface())
    }

    #[allow(dead_code)]
    /// Get interfaces for all created FaaS instances
    pub fn get_interfaces(&self) -> HashMap<&str, FaaSInterface<'_>> {
        self.faases
            .iter()
            .map(|(k, v)| (k.as_str(), v.get_interface()))
            .collect()
    }

    /// Get available modules
    // TODO: load modules from filesystem?
    // TODO: load interfaces of these modules
    pub fn get_modules(&self) -> impl Iterator<Item = &str> {
        self.config.core_module.iter().map(|m| m.name.as_str())
    }

    /// Spawns tasks for calls execution and creates new FaaS-es until an error happens
    fn execute_calls<I>(
        &mut self,
        new_work: &mut I,
    ) -> std::result::Result<(), (FunctionCall, FaaSExecError)>
    where
        I: Iterator<Item = FaaSCall>,
    {
        new_work.try_fold((), |_, call| {
            match call {
                // Request to create FaaS instance with given module_names
                FaaSCall::Create { module_names, call } => {
                    // Convert module names into hashmap
                    let mut module_names = module_names.into_iter().collect();
                    // Generate new service_id
                    let service_id = Uuid::new_v4();

                    // Create FaaS in background
                    let config = self.config.clone();
                    let waker = self.waker.clone();
                    let future = task::spawn_blocking(move || {
                        let faas =
                            FluenceFaaS::with_module_names(&mut module_names, config).map_err(|e| e.into());
                        let (faas, result) = match faas {
                            Ok(faas) => (Some(faas), Ok(FaaSCallResult::FaaSCreated { service_id: service_id.to_string() })),
                            Err(e) => (None, Err(e))
                        };
                        // Wake up when creation finished
                        Self::call_wake(waker);
                        (faas, call, result)
                    });

                    // Save future in order to return its result on the next poll() 
                    self.futures.insert(service_id.to_string(), Box::pin(future));
                    Ok(())
                }
                // Request to call function on an existing FaaS instance
                #[rustfmt::skip]
                FaaSCall::Call { service_id, module, function, arguments, call } => {
                    // Take existing faas
                    let mut faas = self
                        .faases
                        .remove(&service_id)
                        .ok_or_else(|| (call.clone(), FaaSExecError::NoSuchInstance(service_id.clone())))?;
                    let waker = self.waker.clone();
                    // Spawn a task that will call wasm function
                    let future = task::spawn_blocking(move || {
                        let result = faas.call_module(&module, &function, &arguments);
                        let result = result.map(FaaSCallResult::Returned).map_err(|e| e.into());
                        // Wake when call finished to trigger poll()
                        Self::call_wake(waker);
                        (Some(faas), call, result)
                    });
                    // Save future for the next poll
                    self.futures.insert(service_id, Box::pin(future));

                    self.wake();

                    Ok(())
                }
            }
        })
    }

    /// Calls wake on an optional waker
    fn call_wake(waker: Option<Waker>) {
        if let Some(waker) = waker {
            waker.wake()
        }
    }

    /// Clones and calls wakers
    fn wake(&self) {
        Self::call_wake(self.waker.clone())
    }
}

impl NetworkBehaviour for FaaSBehaviour {
    type ProtocolsHandler = DummyProtocolsHandler;
    type OutEvent = (FunctionCall, Result<FaaSCallResult>);

    fn new_handler(&mut self) -> Self::ProtocolsHandler {
        Default::default()
    }

    fn addresses_of_peer(&mut self, _: &PeerId) -> Vec<Multiaddr> {
        vec![]
    }

    fn inject_connected(&mut self, _: &PeerId) {}

    fn inject_disconnected(&mut self, _: &PeerId) {}

    fn inject_event(&mut self, _: PeerId, _: ConnectionId, _: Void) {}

    /// Here you can see two thread pools are working together.
    /// First thread pool comes from libp2p, it calls this `poll`; `cx.waker` refers to that thread pool.
    /// Second thread pool comes from `async_std::task::spawn_blocking`, it executes `FaaSCall::Create` and `FaaSCall::Call`
    ///
    /// On each poll, `cx.waker` is cloned and saved to `self.waker`.
    /// On each poll, we go trough each new call in `self.calls`, and try to execute it. Execution
    /// happens in the background, on the "blocking" thread pool. Resulting future is then saved to `self.futures`.
    ///
    /// Once execution of the call is finished on the "blocking" thread pool, `self.waker.wake()` is called
    /// to signal "libp2p thread pool" to wake up and trigger this `poll()` function.
    ///
    /// On each poll, we go through each future in `self.futures`, and poll it to get result. If
    /// there's a result, we return it as `Poll::Ready(GenerateEvent(result))`.
    ///
    /// Note that each faas executes only a single call at a time. For that purpose it is removed
    /// from `self.faases` during execution, and inserted back once execution is finished.
    fn poll(
        &mut self,
        cx: &mut Context<'_>,
        _: &mut impl PollParameters,
    ) -> Poll<NetworkBehaviourAction<Void, Self::OutEvent>> {
        self.waker = Some(cx.waker().clone());

        // Check there is a completed call
        let mut result = None;
        // let mut futures = std::mem::replace(&mut self.futures, HashMap::new());
        for (service_id, fut) in self.futures.iter_mut() {
            let fut = Pin::new(fut);
            if let Poll::Ready(r) = fut.poll(cx) {
                // TODO: excess clone (possible to obtain via futures.remove_entry)
                result = Some((service_id.clone(), r));
                break;
            }
        }
        // Remove completed future, reinsert faas, return result
        if let Some((service_id, result)) = result {
            self.futures.remove(&service_id);

            let (faas, call, result): FutResult = result;
            // faas could be None if creation failed
            if let Some(faas) = faas {
                self.faases.insert(service_id, faas);
            }

            return Poll::Ready(NetworkBehaviourAction::GenerateEvent((call, result)));
        }

        // Check if there's a work and a matching faas isn't busy
        let capacity = self.calls.capacity();
        let calls = std::mem::replace(&mut self.calls, Vec::with_capacity(capacity));
        let (new_work, busy): (Vec<_>, _) = calls.into_iter().partition(|call| {
            // return true if service is to be created, or there is no existing work for that service_id
            call.is_create() || !self.futures.contains_key(call.service_id().unwrap())
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

#[derive(Debug)]
pub enum FaaSExecError {
    NoSuchInstance(String),
    FaaS(FaaSError),
}

impl Error for FaaSExecError {}
impl From<FaaSError> for FaaSExecError {
    fn from(err: FaaSError) -> Self {
        FaaSExecError::FaaS(err)
    }
}

impl std::fmt::Display for FaaSExecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FaaSExecError::NoSuchInstance(service_id) => {
                write!(f, "FaaS instance {} not found", service_id)
            }
            FaaSExecError::FaaS(err) => err.fmt(f),
        }
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
    use std::path::PathBuf;

    fn wait_result(
        mut swarm: Swarm<FaaSBehaviour>,
    ) -> ((FunctionCall, Result<FaaSCallResult>), Swarm<FaaSBehaviour>) {
        block_on(async move {
            let result = poll_fn(|ctx| {
                if let Poll::Ready(Some(r)) = swarm.poll_next_unpin(ctx) {
                    return Poll::Ready(r);
                }

                Poll::Pending
            })
            .await;

            (result, swarm)
        })
    }

    fn with_modules<P: Into<PathBuf>>(modules: Vec<(String, P)>) -> RawCoreModulesConfig {
        let mut tmp = std::env::temp_dir();
        tmp.push("wasm_modules/");
        std::fs::create_dir_all(&tmp).expect("create tmp dir");

        for (name, path) in modules {
            let path = path.into();
            std::fs::copy(&path, tmp.join(&name))
                .unwrap_or_else(|_| panic!("copy test module wasm {:?}", path));
        }

        let mut config: RawCoreModulesConfig = <_>::default();
        config.core_modules_dir = Some(tmp.to_string_lossy().into());

        config
    }

    fn make_swarm<P: Into<PathBuf>>(modules: Vec<(String, P)>) -> Swarm<FaaSBehaviour> {
        let config = with_modules(modules);
        let behaviour = FaaSBehaviour::new(config);
        let transport = DummyTransport::<(PeerId, Multiplex<DummyStream>)>::new();
        Swarm::new(transport, behaviour, PeerId::random())
    }

    fn create_faas(
        mut swarm: Swarm<FaaSBehaviour>,
        module_names: Vec<String>,
    ) -> (String, Swarm<FaaSBehaviour>) {
        swarm.execute(FaaSCall::Create {
            module_names,
            call: <_>::default(),
        });

        let ((_, created), swarm) = wait_result(swarm);
        let service_id = match &created {
            Ok(FaaSCallResult::FaaSCreated { service_id }) => service_id.clone(),
            wrong => unreachable!("wrong result: {:?}", wrong),
        };

        (service_id, swarm)
    }

    fn call_faas(
        mut swarm: Swarm<FaaSBehaviour>,
        service_id: String,
        module: &str,
        function: &str,
        argument: Option<&str>,
    ) -> (Vec<IValue>, Swarm<FaaSBehaviour>) {
        swarm.execute(FaaSCall::Call {
            service_id,
            module: module.to_string(),
            function: function.to_string(),
            arguments: argument
                .into_iter()
                .map(|s| IValue::String(s.to_string()))
                .collect(),
            call: <_>::default(),
        });

        let ((_, returned), swarm) = wait_result(swarm);
        let returned = match returned {
            Ok(FaaSCallResult::Returned(r)) => r,
            wrong => panic!("{:#?}", wrong),
        };

        (returned, swarm)
    }

    #[test]
    fn call_single_faas() {
        let test_module = "test_module.wasm".to_string();
        let swarm = make_swarm(vec![(test_module.clone(), TEST_MODULE)]);
        let (service_id, swarm) = create_faas(swarm, vec![test_module.clone()]);

        let interface = swarm
            .get_interface(service_id.as_str())
            .expect("get interface");
        assert_eq!(1, interface.modules.len());
        assert_eq!(
            &test_module,
            interface.modules.into_iter().next().unwrap().0
        );

        let payload = "Hello";
        let (returned, _) = call_faas(
            swarm,
            service_id,
            test_module.as_str(),
            "greeting",
            Some(payload),
        );
        assert_eq!(returned, vec![IValue::String(payload.to_string())]);
    }

    #[test]
    fn call_multiple_faases() {
        let test_module = "test_module.wasm".to_string();
        let test_module2 = "test_module2.wasm".to_string();
        let modules = vec![
            (test_module.clone(), TEST_MODULE),
            (test_module2.clone(), TEST_MODULE),
        ];
        let swarm = make_swarm(modules);

        let (service_id1, swarm) = create_faas(swarm, vec![test_module.clone()]);
        let (service_id2, mut swarm) =
            create_faas(swarm, vec![test_module.clone(), test_module2.clone()]);

        assert_eq!(
            2,
            swarm
                .get_interface(&service_id2)
                .expect("get interface")
                .modules
                .len()
        );

        for i in 1..10 {
            let payload = i.to_string();
            let (returned, s) = call_faas(
                swarm,
                service_id1.clone(),
                test_module.as_str(),
                "greeting",
                Some(payload.as_str()),
            );

            let (returned2, s) = call_faas(
                s,
                service_id2.clone(),
                test_module2.as_str(),
                "greeting",
                Some(payload.as_str()),
            );

            assert_eq!(returned, vec![IValue::String(payload.to_string())]);
            assert_eq!(returned2, returned);

            swarm = s;
        }
    }
}
