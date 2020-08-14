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

use super::error::ServiceExecError::{
    self, AddModule, IncorrectBlueprint, IncorrectModuleConfig, NoModuleConfig, NoSuchBlueprint,
    SerializeConfig, WriteConfig,
};
use super::{AppServicesConfig, Blueprint, ServiceCall, ServiceCallResult};

use faas_api::FunctionCall;
use fluence_app_service::{
    AppService, FaaSInterface as AppServiceInterface, RawModuleConfig, RawModulesConfig,
};

use crate::app_service::error::ServiceExecError::{CreateServiceBaseDir, WriteBlueprint};
use crate::app_service::files;
use async_std::task;
use futures::future::BoxFuture;
use std::collections::HashMap;
use std::path::PathBuf;
use std::task::Waker;
use uuid::Uuid;

pub(super) type Result<T> = std::result::Result<T, ServiceExecError>;
pub(super) type FutResult = (Option<AppService>, FunctionCall, Result<ServiceCallResult>);
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
}

impl AppServiceBehaviour {
    pub fn new(config: AppServicesConfig) -> Self {
        Self {
            app_services: <_>::default(),
            calls: <_>::default(),
            waker: <_>::default(),
            futures: <_>::default(),
            config,
        }
    }

    /// Execute given `call`
    pub fn execute(&mut self, call: ServiceCall) {
        self.calls.push(call);
        self.wake();
    }

    /// Get interface of a service specified by `service_id`
    pub fn get_interface(&self, service_id: &str) -> Result<AppServiceInterface<'_>> {
        let service = self
            .app_services
            .get(service_id)
            .ok_or_else(|| ServiceExecError::NoSuchInstance(service_id.to_string()))?;

        Ok(service.get_interface())
    }

    /// Get interfaces for all created services
    pub fn get_interfaces(&self) -> HashMap<&str, AppServiceInterface<'_>> {
        self.app_services
            .iter()
            .map(|(k, v)| (k.as_str(), v.get_interface()))
            .collect()
    }

    /// Get available modules (intersection of modules from config + modules on filesystem)
    // TODO: load interfaces of these modules
    pub fn get_modules(&self) -> Vec<String> {
        Self::list_files(&self.config.blueprint_dir)
            .into_iter()
            .flatten()
            .filter_map(|pb| files::extract_module_name(pb.file_name()?.to_str()?))
            .collect()
    }

    /// Get available blueprints
    pub fn get_blueprints(&self) -> Vec<Blueprint> {
        Self::list_files(&self.config.blueprint_dir)
            .into_iter()
            .flatten()
            .filter(|pb| {
                pb.file_name()
                    .and_then(|f| f.to_str())
                    .filter(|s| files::is_blueprint(s))
                    .is_some()
            })
            .filter_map(|pb| toml::from_slice(std::fs::read(pb).ok()?.as_slice()).ok())
            .collect()
    }

    fn list_files(dir: &PathBuf) -> Option<impl Iterator<Item = PathBuf>> {
        let dir = std::fs::read_dir(dir).ok()?;
        Some(dir.filter_map(|p| p.ok()?.path().into()))
    }

    /// Adds a module to the filesystem, overwriting existing module.
    /// Also adds module config to the RawModuleConfig
    pub fn add_module(&mut self, bytes: Vec<u8>, config: RawModuleConfig) -> Result<()> {
        let path = PathBuf::from(&self.config.blueprint_dir);
        let module = path.join(files::module_file_name(&config.name));
        std::fs::write(&module, bytes).map_err(|err| AddModule {
            path: path.clone(),
            err,
        })?;

        // replace existing configuration with a new one
        let toml = toml::to_string_pretty(&config).map_err(|err| SerializeConfig { err })?;
        let config = path.join(files::module_config_name(config.name));
        std::fs::write(&config, toml).map_err(|err| WriteConfig { path, err })?;

        Ok(())
    }

    /// Saves new blueprint to disk
    pub fn add_blueprint(&mut self, blueprint: &Blueprint) -> Result<()> {
        let mut path = PathBuf::from(&self.config.blueprint_dir);
        path.push(files::blueprint_file_name(&blueprint));

        // Save blueprint to disk
        let bytes = toml::to_vec(&blueprint).map_err(|err| SerializeConfig { err })?;
        std::fs::write(&path, bytes).map_err(|err| WriteBlueprint { path, err })?;

        // TODO: check dependencies are satisfied?

        Ok(())
    }

    fn create_app_service(
        config: AppServicesConfig,
        blueprint_id: String,
        service_id: String,
        waker: Option<Waker>,
    ) -> (Option<AppService>, Result<ServiceCallResult>) {
        use std::fs::read;

        let to_string =
            |path: &PathBuf| -> Option<_> { path.to_string_lossy().into_owned().into() };

        // Load configs for all modules in blueprint
        let make_service = move |service_id| -> Result<_> {
            // Load blueprint from disk
            let bp_dir = PathBuf::from(&config.blueprint_dir);
            let bp_path = bp_dir.join(files::blueprint_fname(blueprint_id.as_str()));
            let blueprint = read(&bp_path).map_err(|err| NoSuchBlueprint { path: bp_path, err })?;
            let blueprint: Blueprint =
                toml::from_slice(blueprint.as_slice()).map_err(|err| IncorrectBlueprint { err })?;

            // Load all module configs
            let configs: Vec<RawModuleConfig> = blueprint
                .dependencies
                .iter()
                .map(|module| {
                    let config = bp_dir.join(files::module_config_name(module));
                    let config =
                        read(&config).map_err(|err| NoModuleConfig { path: config, err })?;
                    let config: RawModuleConfig = toml::from_slice(config.as_slice())
                        .map_err(|err| IncorrectModuleConfig { err })?;
                    Ok(config)
                })
                .collect::<Result<_>>()?;

            // Create separate base dir for the new service
            let service_base_dir = config.services_workdir.join(&service_id);
            std::fs::create_dir_all(&service_base_dir).map_err(|err| CreateServiceBaseDir {
                path: service_base_dir,
                err,
            })?;

            let modules = RawModulesConfig {
                modules_dir: to_string(&config.blueprint_dir),
                service_base_dir: to_string(&service_base_dir),
                module: configs,
                default: None,
            };

            Ok(AppService::new(modules, service_id, config.service_envs)?)
        };

        let service = make_service(&service_id);
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
                ServiceCall::Create { blueprint_id, call } => {
                    // Generate new service_id
                    let service_id = Uuid::new_v4();

                    // Create service in background
                    let waker = self.waker.clone();
                    let config = self.config.clone();
                    let future = task::spawn_blocking(move || {
                        let service_id = service_id.to_string();
                        let (service, result) = Self::create_app_service(
                            config, blueprint_id, service_id, waker
                        );
                        (service, call, result)
                    });

                    // Save future in order to return its result on the next poll() 
                    self.futures.insert(service_id.to_string(), Box::pin(future));
                    Ok(())
                }
                // Request to call function on an existing app service
                #[rustfmt::skip]
                ServiceCall::Call { service_id, module, function, arguments, call } => {
                    // Take existing service
                    let mut service = self
                        .app_services
                        .remove(&service_id)
                        .ok_or_else(|| (call.clone(), ServiceExecError::NoSuchInstance(service_id.clone())))?;
                    let waker = self.waker.clone();
                    // Spawn a task that will call wasm function
                    let future = task::spawn_blocking(move || {
                        let result = service.call(&module, &function, arguments);
                        let result = result.map(ServiceCallResult::Returned).map_err(|e| e.into());
                        // Wake when call finished to trigger poll()
                        Self::call_wake(waker);
                        (Some(service), call, result)
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
