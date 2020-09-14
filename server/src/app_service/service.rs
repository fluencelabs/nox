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

use super::error::ServiceExecError;
use super::{ServiceCall, ServiceCallResult};

use faas_api::FunctionCall;
use fluence_app_service::{AppService, RawModuleConfig, RawModulesConfig};

use crate::config::AppServicesConfig;
use crate::function::{extract_client_id, extract_public_key};
use async_std::task;
use futures::channel::mpsc;
use futures::future::BoxFuture;
use libp2p::identity::PublicKey;
use std::collections::HashMap;
use std::path::PathBuf;
use std::task::Waker;
use uuid::Uuid;

pub(super) type Result<T> = std::result::Result<T, ServiceExecError>;
pub(super) type FutResult = (
    Option<AppService>,
    Option<FunctionCall>,
    Result<ServiceCallResult>,
);
pub(super) type Fut = BoxFuture<'static, FutResult>;

/// Behaviour that manages AppService instances: create, pass calls, poll for results
pub struct AppServiceBehaviour {
    /// Created instances
    //TODO: when to delete an instance?
    pub(super) app_services: HashMap<String, AppService>,
    /// Incoming calls waiting to be processed
    pub(super) calls: Vec<ServiceCall>,
    /// Context waker, used to trigger `poll`
    pub(super) waker: Option<Waker>,
    /// Pending futures: service_id -> future
    pub(super) futures: HashMap<String, Fut>,
    /// Config for service creation
    pub(super) config: AppServicesConfig,
    pub(super) mailbox: mpsc::UnboundedSender<ServiceCall>,
    pub(super) intra_calls: mpsc::UnboundedReceiver<ServiceCall>,
}

impl AppServiceBehaviour {
    pub fn new(config: AppServicesConfig) -> Self {
        let (outlet, inlet) = mpsc::unbounded();

        Self {
            app_services: <_>::default(),
            calls: <_>::default(),
            waker: <_>::default(),
            futures: <_>::default(),
            config,
            mailbox: outlet,
            intra_calls: inlet,
        }
    }

    /// Execute given `call`
    pub fn execute(&mut self, call: ServiceCall) {
        self.calls.push(call);
        self.wake();
    }

    fn create_app_service(
        config: AppServicesConfig,
        blueprint_id: String,
        service_id: String,
        waker: Option<Waker>,
        owner_id: Option<String>,
        owner_pk: Option<String>,
    ) -> (Option<AppService>, Result<ServiceCallResult>) {
        let to_string =
            |path: &PathBuf| -> Option<_> { path.to_string_lossy().into_owned().into() };

        // Load configs for all modules in blueprint
        let make_service = move |service_id: &str| -> Result<_> {
            // Load blueprint from disk
            let blueprint = Self::load_blueprint(&config.blueprint_dir, &blueprint_id)?;

            // Load all module configs
            let configs: Vec<RawModuleConfig> = blueprint
                .dependencies
                .iter()
                .map(|module| Self::load_module_config(&config.modules_dir, module))
                .collect::<Result<_>>()?;

            let modules = RawModulesConfig {
                modules_dir: to_string(&config.modules_dir),
                service_base_dir: to_string(&config.workdir),
                module: configs,
                default: None,
            };

            let mut envs = config.service_envs;
            if let Some(owner_id) = owner_id {
                envs.push(format!("owner_id={}", owner_id));
            };
            if let Some(owner_pk) = owner_pk {
                envs.push(format!("owner_pk={}", owner_pk));
            };
            log::info!("Creating service {}, envs: {:?}", service_id, envs);

            let service = AppService::new(modules, &service_id, envs)?;

            // Save created service to disk, so it is recreated on restart
            Self::persist_service(&config.services_dir, &service_id, &blueprint_id)?;

            Ok(service)
        };

        let service = make_service(service_id.as_str());
        let (service, result) = match service {
            Ok(service) => (
                Some(service),
                Ok(ServiceCallResult::ServiceCreated { service_id }),
            ),
            Err(e) => (None, Err(e)),
        };
        // Wake up when creation finished
        Self::call_wake(waker);
        (service, result)
    }

    /// Spawns tasks for calls execution and creates new services until an error happens
    pub(super) fn execute_calls<I>(
        &mut self,
        new_work: &mut I,
    ) -> std::result::Result<(), (FunctionCall, ServiceExecError)>
    where
        I: Iterator<Item = ServiceCall>,
    {
        new_work.try_fold((), |_, call| {
            match call {
                // Request to create app service with given module_names
                ServiceCall::Create { service_id, blueprint_id, call } => {
                    // Generate new service_id
                    let service_id = service_id.unwrap_or_else(|| Uuid::new_v4().to_string());

                    // Create service in background
                    let waker = self.waker.clone();
                    let config = self.config.clone();
                    let id = service_id.clone();
                    let owner_id = Self::owner_id(call.as_ref());
                    let owner_pk = Self::owner_pk(call.as_ref());
                    let future = task::spawn_blocking(move || {
                        let (service, result) = Self::create_app_service(
                            config, blueprint_id, id, waker, owner_id, owner_pk
                        );
                        (service, call, result)
                    });

                    // Save future in order to return its result on the next poll() 
                    self.futures.insert(service_id, Box::pin(future));
                    Ok(())
                }
                // Request to call function on an existing app service
                #[rustfmt::skip]
                ServiceCall::Call { service_id, module, function, arguments, headers, call } => {
                    // Take existing service
                    let mut service = self
                        .app_services
                        .remove(&service_id)
                        .ok_or_else(|| (call.clone(), ServiceExecError::NoSuchInstance(service_id.clone())))?;
                    let waker = self.waker.clone();
                    let call_service = self.make_call_service();
                    // Spawn a task that will call wasm function
                    let future = task::spawn_blocking(move || {
                        // TODO: pass `call_service` to fce
                        drop(call_service);
                        let result = service.call(&module, &function, arguments, headers);
                        let result = result.map(ServiceCallResult::Returned).map_err(|e| e.into());
                        // Wake when call finished to trigger poll()
                        Self::call_wake(waker);
                        (Some(service), Some(call), result)
                    });
                    // Save future for the next poll
                    self.futures.insert(service_id, Box::pin(future));

                    self.wake();

                    Ok(())
                }
            }
        })
    }

    #[allow(dead_code)]
    pub(super) fn make_call_service(&self) -> impl Fn(ServiceCall) {
        let mailbox = self.mailbox.clone();
        move |call| {
            mailbox.unbounded_send(call).expect(
                "app service behaviour's mailbox was disconnected. shouldn't happen. panic!",
            );
        }
    }

    /// Calls wake on an optional waker
    fn call_wake(waker: Option<Waker>) {
        if let Some(waker) = waker {
            waker.wake()
        }
    }

    /// Clones and calls wakers
    pub(super) fn wake(&self) {
        Self::call_wake(self.waker.clone())
    }

    fn owner_pk(call: Option<&FunctionCall>) -> Option<String> {
        let call = call?;
        if let PublicKey::Ed25519(pk) = extract_public_key(&call.sender).ok()? {
            return Some(bs58::encode(pk.encode()).into_string());
        }

        None
    }

    fn owner_id(call: Option<&FunctionCall>) -> Option<String> {
        let call = call?;
        let id = extract_client_id(&call.sender).ok()?;
        Some(id.to_string())
    }
}
