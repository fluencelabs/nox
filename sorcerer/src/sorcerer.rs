/*
 * Copyright 2022 Fluence Labs Limited
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

use std::collections::HashMap;
use std::time::Duration;

use async_std::task::{spawn, JoinHandle};
use fluence_spell_dtos::trigger_config::TriggerConfigValue;
use futures::{FutureExt, StreamExt};
use serde_json::Value;

use aquamarine::AquamarineApi;
use fluence_libp2p::types::Inlet;
use key_manager::KeyManager;
use particle_args::JError;
use particle_builtins::{wrap, wrap_unit};
use particle_execution::{ServiceFunction, ServiceFunctionImmut};
use particle_modules::ModuleRepository;
use particle_services::ParticleAppServices;
use server_config::ResolvedConfig;
use spell_event_bus::api::{from_user_config, SpellEventBusApi, TriggerEvent};
use spell_storage::SpellStorage;

use crate::spells::{
    get_spell_arg, get_spell_id, spell_install, spell_list, spell_remove, spell_update_config,
    store_error, store_response,
};
use crate::utils::process_func_outcome;

#[derive(Clone)]
pub struct Sorcerer {
    pub aquamarine: AquamarineApi,
    pub services: ParticleAppServices,
    pub spell_storage: SpellStorage,
    pub spell_event_bus_api: SpellEventBusApi,
    pub spell_script_particle_ttl: Duration,
    pub key_manager: KeyManager,
}

pub struct SpellBuiltin {
    pub service_id: String,
    pub functions: HashMap<String, ServiceFunction>,
    pub unhandled: Option<ServiceFunction>,
}

impl SpellBuiltin {
    pub fn new(service_id: &str) -> Self {
        Self {
            service_id: service_id.to_string(),
            functions: Default::default(),
            unhandled: None,
        }
    }

    pub fn append(&mut self, function_name: &str, service_function: ServiceFunction) {
        self.functions
            .insert(function_name.to_string(), service_function);
    }

    pub fn set_unhandled(&mut self, unhandled: ServiceFunction) {
        self.unhandled = Some(unhandled)
    }
}

impl Sorcerer {
    pub fn new(
        services: ParticleAppServices,
        modules: ModuleRepository,
        aquamarine: AquamarineApi,
        config: ResolvedConfig,
        spell_event_bus_api: SpellEventBusApi,
        key_manager: KeyManager,
    ) -> (Self, Vec<SpellBuiltin>) {
        let spell_storage =
            SpellStorage::create(&config.dir_config.spell_base_dir, &services, &modules)
                .expect("Spell storage creation");

        let sorcerer = Self {
            aquamarine,
            services,
            spell_storage,
            spell_event_bus_api,
            spell_script_particle_ttl: config.max_spell_particle_ttl,
            key_manager,
        };

        let spell_service_functions = sorcerer.make_spell_builtins();

        (sorcerer, spell_service_functions)
    }

    async fn resubscribe_spells(&self) {
        for spell_id in self.spell_storage.get_registered_spells() {
            log::info!("Rescheduling spell {}", spell_id);
            let result: Result<(), JError> = try {
                let spell_owner = self.services.get_service_owner(spell_id.clone(), None)?;
                let result = process_func_outcome::<TriggerConfigValue>(
                    self.services.call_function(
                        spell_owner,
                        &spell_id,
                        "get_trigger_config",
                        vec![],
                        None,
                        spell_owner,
                        self.spell_script_particle_ttl,
                    ),
                    &spell_id,
                    "get_trigger_config",
                )?;
                let config = from_user_config(result.config)?;
                if let Some(config) = config.and_then(|c| c.into_rescheduled()) {
                    self.spell_event_bus_api
                        .subscribe(spell_id.clone(), config.clone())
                        .await?;
                } else {
                    log::warn!("Spell {spell_id} is not rescheduled since its config is either not found or not reschedulable");
                }
            };
            if let Err(e) = result {
                // 1. We do not remove the spell we aren't able to reschedule. Users should be able to rerun it manually when updating trigger config.
                // 2. Maybe we should somehow register which spell are running and which are not and notify user about it.
                log::warn!("Failed to reschedule spell {}: {}.", spell_id, e);
            }
        }
    }

    pub fn start(self, spell_events_stream: Inlet<TriggerEvent>) -> JoinHandle<()> {
        spawn(async {
            self.resubscribe_spells().await;

            spell_events_stream
                .for_each_concurrent(None, move |spell_event| {
                    let sorcerer = self.clone();
                    // Note that the event that triggered the spell is in `spell_event.event`
                    async move {
                        sorcerer.execute_script(spell_event).await;
                    }
                })
                .await;
        })
    }

    fn make_spell_builtins(&self) -> Vec<SpellBuiltin> {
        let mut spell_builtins: Vec<SpellBuiltin> = vec![];

        let mut spell_service = SpellBuiltin::new("spell");
        spell_service.append("install", self.make_spell_install_closure().into());
        spell_service.append("remove", self.make_spell_remove_closure().into());
        spell_service.append("list", self.make_spell_list_closure().into());
        spell_service.append(
            "update_trigger_config",
            self.make_spell_update_config_closure().into(),
        );
        spell_builtins.push(spell_service);

        let mut get_data_srv = SpellBuiltin::new("getDataSrv");
        get_data_srv.append("spell_id", self.make_get_spell_id_closure().into());
        get_data_srv.append("-relay-", self.make_get_relay_closure().into());
        get_data_srv.set_unhandled(self.make_get_spell_arg_closure().into());
        spell_builtins.push(get_data_srv);

        let mut error_handler_srv = SpellBuiltin::new("errorHandlingSrv");
        error_handler_srv.append("error", self.make_error_handler_closure().into());
        spell_builtins.push(error_handler_srv);

        let mut callback_srv = SpellBuiltin::new("callbackSrv");
        callback_srv.append("response", self.make_response_handler_closure().into());
        spell_builtins.push(callback_srv);

        spell_builtins
    }

    fn make_spell_install_closure(&self) -> ServiceFunctionImmut {
        let services = self.services.clone();
        let storage = self.spell_storage.clone();
        let spell_event_bus = self.spell_event_bus_api.clone();
        let key_manager = self.key_manager.clone();
        Box::new(move |args, params| {
            let storage = storage.clone();
            let services = services.clone();
            let spell_event_bus_api = spell_event_bus.clone();
            let key_manager = key_manager.clone();
            async move {
                wrap(
                    spell_install(
                        args,
                        params,
                        storage,
                        services,
                        spell_event_bus_api,
                        key_manager,
                    )
                    .await,
                )
            }
            .boxed()
        })
    }

    fn make_spell_remove_closure(&self) -> ServiceFunctionImmut {
        let services = self.services.clone();
        let storage = self.spell_storage.clone();
        let spell_event_bus_api = self.spell_event_bus_api.clone();
        let key_manager = self.key_manager.clone();

        Box::new(move |args, params| {
            let storage = storage.clone();
            let services = services.clone();
            let api = spell_event_bus_api.clone();
            let key_manager = key_manager.clone();
            async move {
                wrap_unit(spell_remove(args, params, storage, services, api, key_manager).await)
            }
            .boxed()
        })
    }

    fn make_spell_list_closure(&self) -> ServiceFunctionImmut {
        let storage = self.spell_storage.clone();
        Box::new(move |_, _| {
            let storage = storage.clone();
            async move { wrap(spell_list(storage)) }.boxed()
        })
    }

    fn make_spell_update_config_closure(&self) -> ServiceFunctionImmut {
        let api = self.spell_event_bus_api.clone();
        let services = self.services.clone();
        let key_manager = self.key_manager.clone();
        Box::new(move |args, params| {
            let api = api.clone();
            let services = services.clone();
            let key_manager = key_manager.clone();
            async move { wrap_unit(spell_update_config(args, params, services, api, key_manager).await) }.boxed()
        })
    }

    fn make_get_spell_id_closure(&self) -> ServiceFunctionImmut {
        Box::new(move |_, params| async move { wrap(get_spell_id(params)) }.boxed())
    }

    fn make_get_relay_closure(&self) -> ServiceFunctionImmut {
        let relay_peer_id = self.key_manager.get_host_peer_id().to_base58();
        Box::new(move |_, _| {
            let relay = relay_peer_id.clone();
            async move { wrap(Ok(Value::String(relay))) }.boxed()
        })
    }

    fn make_get_spell_arg_closure(&self) -> ServiceFunctionImmut {
        let services = self.services.clone();
        Box::new(move |args, params| {
            let services = services.clone();
            async move { wrap(get_spell_arg(args, params, services)) }.boxed()
        })
    }

    fn make_error_handler_closure(&self) -> ServiceFunctionImmut {
        let services = self.services.clone();
        Box::new(move |args, params| {
            let services = services.clone();
            async move { wrap_unit(store_error(args, params, services)) }.boxed()
        })
    }

    fn make_response_handler_closure(&self) -> ServiceFunctionImmut {
        let services = self.services.clone();
        Box::new(move |args, params| {
            let services = services.clone();
            async move { wrap_unit(store_response(args, params, services)) }.boxed()
        })
    }
}
