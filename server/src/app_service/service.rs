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

use super::error::ServiceExecError::{self, *};
use super::{Blueprint, ServiceCall, ServiceCallResult};

use faas_api::FunctionCall;
use fluence_app_service::{
    AppService, FaaSInterface as AppServiceInterface, RawModuleConfig, RawModulesConfig,
};

use crate::app_service::error::ServiceExecError::{
    CreateServicesDir, ReadPersistedService, WriteBlueprint,
};
use crate::app_service::files;
use crate::app_service::persisted_service::PersistedService;
use crate::config::AppServicesConfig;
use async_std::task;
use futures::future::BoxFuture;
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
        Self::list_files(&self.config.modules_dir)
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

    /// Adds a module to the filesystem, overwriting existing module.
    /// Also adds module config to the RawModuleConfig
    pub fn add_module(&mut self, bytes: Vec<u8>, config: RawModuleConfig) -> Result<()> {
        let path = PathBuf::from(&self.config.modules_dir);
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

            let service = AppService::new(modules, &service_id, config.service_envs)?;

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
                    let future = task::spawn_blocking(move || {
                        let (service, result) = Self::create_app_service(
                            config, blueprint_id, id, waker
                        );
                        (service, call, result)
                    });

                    // Save future in order to return its result on the next poll() 
                    self.futures.insert(service_id, Box::pin(future));
                    Ok(())
                }
                // Request to call function on an existing app service
                #[rustfmt::skip]
                ServiceCall::Call { service_id, module, function, arguments, call_parameters, call } => {
                    // Take existing service
                    let mut service = self
                        .app_services
                        .remove(&service_id)
                        .ok_or_else(|| (call.clone(), ServiceExecError::NoSuchInstance(service_id.clone())))?;
                    let waker = self.waker.clone();
                    // Spawn a task that will call wasm function
                    let future = task::spawn_blocking(move || {
                        let result = service.call(&module, &function, arguments, call_parameters);
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

    /// Load info about persisted services from disk, and create `AppService` for each of them
    pub fn create_persisted_services(&mut self) -> Vec<ServiceExecError> {
        // Load all persisted service file names
        let files = match Self::list_files(&self.config.services_dir) {
            Some(files) => files,
            None => {
                // Attempt to create directory and exit
                return std::fs::create_dir_all(&self.config.services_dir)
                    .map_err(|err| CreateServicesDir {
                        path: self.config.services_dir.clone(),
                        err,
                    })
                    .err()
                    .into_iter()
                    .collect();
            }
        };

        files.filter(files::is_service).map(|file| {
            // Load service's persisted info
            let bytes =
                std::fs::read(&file).map_err(|err| ReadPersistedService { err, path: file.clone() })?;
            let PersistedService { service_id, blueprint_id } =
                toml::from_slice(bytes.as_slice()).map_err(|err| IncorrectModuleConfig { err })?;

            // Don't overwrite existing services
            if self.app_services.contains_key(service_id) {
                log::warn!(
                    "Won't load persisted service {}: there's already a service with such service id",
                    service_id
                );

                return Ok(());
            }

            // Schedule creation of the service
            self.execute(ServiceCall::Create {
                service_id: Some(service_id.to_string()),
                blueprint_id: blueprint_id.to_string(),
                call: None
            });

            self.wake();

            Ok(())
        }).filter_map(|r| r.err()).collect()
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

    /// Load blueprint from disk
    fn load_blueprint(bp_dir: &PathBuf, blueprint_id: &str) -> Result<Blueprint> {
        let bp_path = bp_dir.join(files::blueprint_fname(blueprint_id));
        let blueprint =
            std::fs::read(&bp_path).map_err(|err| NoSuchBlueprint { path: bp_path, err })?;
        let blueprint: Blueprint =
            toml::from_slice(blueprint.as_slice()).map_err(|err| IncorrectBlueprint { err })?;

        Ok(blueprint)
    }

    /// Load RawModuleConfig from disk, for a given module name
    fn load_module_config(modules_dir: &PathBuf, module: &str) -> Result<RawModuleConfig> {
        let config = modules_dir.join(files::module_config_name(module));
        let config = std::fs::read(&config).map_err(|err| NoModuleConfig { path: config, err })?;
        let config =
            toml::from_slice(config.as_slice()).map_err(|err| IncorrectModuleConfig { err })?;

        Ok(config)
    }

    /// Persist service info to disk, so it is recreated after restart
    fn persist_service(services_dir: &PathBuf, service_id: &str, blueprint_id: &str) -> Result<()> {
        let config = PersistedService::new(service_id, blueprint_id);
        let bytes = toml::to_vec(&config).map_err(|err| SerializeConfig { err })?;
        let path = services_dir.join(files::service_file_name(service_id));
        std::fs::write(&path, bytes).map_err(|err| WriteConfig { path, err })
    }

    /// List files in directory
    fn list_files(dir: &PathBuf) -> Option<impl Iterator<Item = PathBuf>> {
        let dir = std::fs::read_dir(dir).ok()?;
        Some(dir.filter_map(|p| p.ok()?.path().into()))
    }
}
