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

use crate::app_service::service::{AppServiceBehaviour, FutResult, Result};
use crate::app_service::ServiceCallResult;

use faas_api::FunctionCall;

use libp2p::{
    core::connection::ConnectionId,
    swarm::{
        protocols_handler::DummyProtocolsHandler, NetworkBehaviour, NetworkBehaviourAction,
        PollParameters,
    },
    PeerId,
};
use parity_multiaddr::Multiaddr;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use void::Void;

impl NetworkBehaviour for AppServiceBehaviour {
    type ProtocolsHandler = DummyProtocolsHandler;
    type OutEvent = (FunctionCall, Result<ServiceCallResult>);

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
    /// Second thread pool comes from `async_std::task::spawn_blocking`, it executes `ServiceCall::Create` and `ServiceCall::Call`
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
    /// Note that each service executes only a single call at a time. For that purpose it is removed
    /// from `self.app_services` during execution, and inserted back once execution is finished.
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
        // Remove completed future, reinsert service, return result
        if let Some((service_id, result)) = result {
            self.futures.remove(&service_id);

            let (service, call, result): FutResult = result;
            // service could be None if creation failed
            if let Some(service) = service {
                self.app_services.insert(service_id, service);
            }

            // If there's a call, then someone possibly waits for a result of creation
            if let Some(call) = call {
                return Poll::Ready(NetworkBehaviourAction::GenerateEvent((call, result)));
            }
        }

        // Check if there's a work and a matching service isn't busy
        let capacity = self.calls.capacity();
        let calls = std::mem::replace(&mut self.calls, Vec::with_capacity(capacity));
        let (new_work, busy): (Vec<_>, _) = calls.into_iter().partition(|call| {
            // return true if service is to be created, or there is no existing work for that service_id
            call.is_create() || !self.futures.contains_key(call.service_id().unwrap())
        });
        self.calls.extend(busy);

        // Execute calls on services
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
    use crate::app_service::{Blueprint, ServiceCall};
    use crate::config::AppServicesConfig;
    use failure::_core::time::Duration;
    use fluence_app_service::{IValue, RawModuleConfig};
    use futures::StreamExt;
    use futures::{executor::block_on, future::poll_fn};
    use libp2p::core::transport::dummy::{DummyStream, DummyTransport};
    use libp2p::mplex::Multiplex;
    use libp2p::{PeerId, Swarm};
    use rand::Rng;
    use serde_json::json;
    use std::path::{Path, PathBuf};

    fn uuid() -> String {
        uuid::Uuid::new_v4().to_string()
    }

    fn wait_result(
        mut swarm: Swarm<AppServiceBehaviour>,
        timeout: Duration,
    ) -> (
        (FunctionCall, Result<ServiceCallResult>),
        Swarm<AppServiceBehaviour>,
    ) {
        block_on(async_std::future::timeout(timeout, async move {
            let result = poll_fn(|ctx| {
                if let Poll::Ready(Some(r)) = swarm.poll_next_unpin(ctx) {
                    return Poll::Ready(r);
                }

                Poll::Pending
            })
            .await;

            (result, swarm)
        }))
        .expect("timed out")
    }

    fn make_tmp_dir() -> PathBuf {
        use rand::distributions::Alphanumeric;

        let mut tmp = std::env::temp_dir();
        tmp.push("fluence_test/");
        let dir: String = rand::thread_rng()
            .sample_iter(Alphanumeric)
            .take(16)
            .collect();
        tmp.push(dir);

        std::fs::create_dir_all(&tmp).expect("create tmp dir");

        tmp
    }

    fn make_config() -> AppServicesConfig {
        let tmp = make_tmp_dir();

        AppServicesConfig {
            blueprint_dir: tmp.clone(),
            modules_dir: tmp.clone(),
            services_dir: tmp.clone(),
            workdir: tmp,
            service_envs: vec![],
        }
    }

    fn upload_modules<P: AsRef<Path>>(
        swarm: &mut Swarm<AppServiceBehaviour>,
        modules: &[(&str, P)],
    ) {
        for (name, path) in modules {
            let bytes = std::fs::read(&path).expect("read module");
            let config: RawModuleConfig = serde_json::from_value(json!(
                {
                    "name": name,
                    "mem_pages_count": 100,
                    "logger_enabled": true,
                    "wasi": {
                        "envs": Vec::<()>::new(),
                        "preopened_files": vec!["./tests/artifacts"],
                        "mapped_dirs": json!({}),
                    }
                }
            ))
            .unwrap();
            swarm.add_module(bytes, config).expect("add module");
        }
    }

    fn add_blueprint(swarm: &mut Swarm<AppServiceBehaviour>, modules: &[&str]) -> Blueprint {
        let blueprint = Blueprint {
            name: uuid(),
            id: uuid(),
            dependencies: modules.iter().map(|s| s.to_string()).collect(),
        };
        swarm.add_blueprint(&blueprint).expect("add blueprint");
        blueprint
    }

    fn make_swarm() -> Swarm<AppServiceBehaviour> {
        let config = make_config();
        let behaviour = AppServiceBehaviour::new(config);
        let transport = DummyTransport::<(PeerId, Multiplex<DummyStream>)>::new();
        Swarm::new(transport, behaviour, PeerId::random())
    }

    fn create_app_service(
        mut swarm: Swarm<AppServiceBehaviour>,
        module_names: &[&str],
    ) -> (String, Swarm<AppServiceBehaviour>) {
        let blueprint = add_blueprint(&mut swarm, module_names);

        swarm.execute(ServiceCall::Create {
            blueprint_id: blueprint.id,
            call: Some(FunctionCall::default()),
            service_id: None,
        });

        let ((_, created), swarm) = wait_result(swarm, Duration::from_secs(15));
        let service_id = match &created {
            Ok(ServiceCallResult::ServiceCreated { service_id }) => service_id.clone(),
            wrong => unreachable!("wrong result: {:?}", wrong),
        };

        (service_id, swarm)
    }

    fn call_service(
        mut swarm: Swarm<AppServiceBehaviour>,
        service_id: String,
        module: &str,
        function: &str,
        argument: Option<&str>,
    ) -> (Vec<IValue>, Swarm<AppServiceBehaviour>) {
        swarm.execute(ServiceCall::Call {
            service_id,
            module: module.to_string(),
            function: function.to_string(),
            arguments: serde_json::json!([argument]),
            call: <_>::default(),
        });

        let ((_, returned), swarm) = wait_result(swarm, Duration::from_millis(100));
        let returned = match returned {
            Ok(ServiceCallResult::Returned(r)) => r,
            wrong => panic!("{:#?}", wrong),
        };

        (returned, swarm)
    }

    #[test]
    fn call_single_service() {
        let test_module = "test_module";
        let mut swarm = make_swarm();
        upload_modules(&mut swarm, &[(test_module, TEST_MODULE)]);
        let (service_id, swarm) = create_app_service(swarm, &[test_module]);

        let interface = swarm
            .get_interface(service_id.as_str())
            .expect("get interface");
        assert_eq!(1, interface.modules.len());
        assert_eq!(
            &test_module,
            &interface.modules.into_iter().next().unwrap().0
        );

        let payload = "Hello";
        let (returned, _) = call_service(swarm, service_id, test_module, "greeting", Some(payload));
        assert_eq!(returned, vec![IValue::String(payload.to_string())]);
    }

    #[test]
    fn call_multiple_services() {
        let test_module = "test_module";
        let test_module2 = "test_module2";
        let mut swarm = make_swarm();
        upload_modules(
            &mut swarm,
            &[(test_module, TEST_MODULE), (test_module2, TEST_MODULE)],
        );
        let (service_id1, swarm) = create_app_service(swarm, &[test_module]);
        let (service_id2, mut swarm) = create_app_service(swarm, &[test_module, test_module2]);

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
            let (returned, s) = call_service(
                swarm,
                service_id1.clone(),
                test_module,
                "greeting",
                Some(payload.as_str()),
            );

            let (returned2, s) = call_service(
                s,
                service_id2.clone(),
                test_module2,
                "greeting",
                Some(payload.as_str()),
            );

            assert_eq!(returned, vec![IValue::String(payload.to_string())]);
            assert_eq!(returned2, returned);

            swarm = s;
        }
    }
}
