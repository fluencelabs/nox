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

use fluence_spell_dtos::trigger_config::TriggerConfigValue;
use futures::{FutureExt, StreamExt};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::spells::{
    get_spell_arg, get_spell_id, spell_install, spell_list, spell_remove, spell_update_config,
    store_error, store_response,
};
use crate::utils::process_func_outcome;
use crate::worker_builins::{create_worker, get_worker_peer_id, remove_worker, worker_list};
use aquamarine::AquamarineApi;
use key_manager::KeyManager;
use particle_args::JError;
use particle_builtins::{wrap, wrap_unit, CustomService};
use particle_execution::ServiceFunction;
use particle_modules::ModuleRepository;
use particle_services::ParticleAppServices;
use serde_json::Value;
use server_config::ResolvedConfig;
use spell_event_bus::api::{from_user_config, SpellEventBusApi, TriggerEvent};
use spell_storage::SpellStorage;

#[derive(Clone)]
pub struct Sorcerer {
    pub aquamarine: AquamarineApi,
    pub services: ParticleAppServices,
    pub spell_storage: SpellStorage,
    pub spell_event_bus_api: SpellEventBusApi,
    pub spell_script_particle_ttl: Duration,
    pub key_manager: KeyManager,
}

impl Sorcerer {
    pub fn new(
        services: ParticleAppServices,
        modules: ModuleRepository,
        aquamarine: AquamarineApi,
        config: ResolvedConfig,
        spell_event_bus_api: SpellEventBusApi,
        key_manager: KeyManager,
    ) -> (Self, HashMap<String, CustomService>, String) {
        let (spell_storage, spell_version) =
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

        let mut builtin_functions = sorcerer.make_spell_builtins();
        builtin_functions.extend_one(sorcerer.make_worker_builtin());

        (sorcerer, builtin_functions, spell_version)
    }

    async fn resubscribe_spells(&self) {
        for spell_id in self
            .spell_storage
            .get_registered_spells()
            .values()
            .flatten()
        {
            log::info!("Rescheduling spell {}", spell_id);
            let result: Result<(), JError> = try {
                let spell_owner = self
                    .services
                    .get_service_owner(spell_id.clone(), self.key_manager.get_host_peer_id())?;
                let result = process_func_outcome::<TriggerConfigValue>(
                    self.services.call_function(
                        spell_owner,
                        spell_id,
                        "get_trigger_config",
                        vec![],
                        None,
                        spell_owner,
                        self.spell_script_particle_ttl,
                        false,
                    ),
                    spell_id,
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

    pub fn start(
        self,
        spell_events_receiver: mpsc::UnboundedReceiver<TriggerEvent>,
    ) -> JoinHandle<()> {
        tokio::task::Builder::new()
            .name("sorcerer")
            .spawn(async {
                self.resubscribe_spells().await;
                let spell_events_stream = UnboundedReceiverStream::new(spell_events_receiver);
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
            .expect("Could not spawn task")
    }

    fn make_spell_builtins(&self) -> HashMap<String, CustomService> {
        let mut spell_builtins: HashMap<String, CustomService> = HashMap::new();

        spell_builtins.insert(
            "spell".to_string(),
            CustomService::new(
                vec![
                    ("install", self.make_spell_install_closure()),
                    ("remove", self.make_spell_remove_closure()),
                    ("list", self.make_spell_list_closure()),
                    (
                        "update_trigger_config",
                        self.make_spell_update_config_closure(),
                    ),
                ],
                None,
            ),
        );

        spell_builtins.insert(
            "getDataSrv".to_string(),
            CustomService::new(
                vec![
                    ("spell_id", self.make_get_spell_id_closure()),
                    ("-relay-", self.make_get_relay_closure()),
                ],
                Some(self.make_get_spell_arg_closure()),
            ),
        );

        spell_builtins.insert(
            "errorHandlingSrv".to_string(),
            CustomService::new(vec![("error", self.make_error_handler_closure())], None),
        );

        spell_builtins.insert(
            "callbackSrv".to_string(),
            CustomService::new(
                vec![("response", self.make_response_handler_closure())],
                None,
            ),
        );

        spell_builtins
    }

    fn make_worker_builtin(&self) -> (String, CustomService) {
        (
            "worker".to_string(),
            CustomService::new(
                vec![
                    ("create", self.make_worker_create_closure()),
                    ("get_peer_id", self.make_worker_get_peer_id_closure()),
                    ("remove", self.make_worker_remove_closure()),
                    ("list", self.make_worker_list_closure()),
                ],
                None,
            ),
        )
    }

    fn make_spell_install_closure(&self) -> ServiceFunction {
        let services = self.services.clone();
        let storage = self.spell_storage.clone();
        let spell_event_bus = self.spell_event_bus_api.clone();
        let key_manager = self.key_manager.clone();
        ServiceFunction::Immut(Box::new(move |args, params| {
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
        }))
    }

    fn make_spell_remove_closure(&self) -> ServiceFunction {
        let services = self.services.clone();
        let storage = self.spell_storage.clone();
        let spell_event_bus_api = self.spell_event_bus_api.clone();
        let key_manager = self.key_manager.clone();

        ServiceFunction::Immut(Box::new(move |args, params| {
            let storage = storage.clone();
            let services = services.clone();
            let api = spell_event_bus_api.clone();
            let key_manager = key_manager.clone();
            async move {
                wrap_unit(spell_remove(args, params, storage, services, api, key_manager).await)
            }
            .boxed()
        }))
    }

    fn make_spell_list_closure(&self) -> ServiceFunction {
        let storage = self.spell_storage.clone();
        ServiceFunction::Immut(Box::new(move |_, params| {
            let storage = storage.clone();
            async move { wrap(spell_list(params, storage)) }.boxed()
        }))
    }

    fn make_spell_update_config_closure(&self) -> ServiceFunction {
        let api = self.spell_event_bus_api.clone();
        let services = self.services.clone();
        let key_manager = self.key_manager.clone();
        ServiceFunction::Immut(Box::new(move |args, params| {
            let api = api.clone();
            let services = services.clone();
            let key_manager = key_manager.clone();
            async move { wrap_unit(spell_update_config(args, params, services, api, key_manager).await) }.boxed()
        }))
    }

    fn make_get_spell_id_closure(&self) -> ServiceFunction {
        ServiceFunction::Immut(Box::new(move |_, params| {
            async move { wrap(get_spell_id(params)) }.boxed()
        }))
    }

    fn make_get_relay_closure(&self) -> ServiceFunction {
        let relay_peer_id = self.key_manager.get_host_peer_id().to_base58();
        ServiceFunction::Immut(Box::new(move |_, _| {
            let relay = relay_peer_id.clone();
            async move { wrap(Ok(Value::String(relay))) }.boxed()
        }))
    }

    fn make_get_spell_arg_closure(&self) -> ServiceFunction {
        let services = self.services.clone();
        ServiceFunction::Immut(Box::new(move |args, params| {
            let services = services.clone();
            async move { wrap(get_spell_arg(args, params, services)) }.boxed()
        }))
    }

    fn make_error_handler_closure(&self) -> ServiceFunction {
        let services = self.services.clone();
        ServiceFunction::Immut(Box::new(move |args, params| {
            let services = services.clone();
            async move { wrap_unit(store_error(args, params, services)) }.boxed()
        }))
    }

    fn make_response_handler_closure(&self) -> ServiceFunction {
        let services = self.services.clone();
        ServiceFunction::Immut(Box::new(move |args, params| {
            let services = services.clone();
            async move { wrap_unit(store_response(args, params, services)) }.boxed()
        }))
    }

    fn make_worker_create_closure(&self) -> ServiceFunction {
        let key_manager = self.key_manager.clone();
        ServiceFunction::Immut(Box::new(move |args, params| {
            let key_manager = key_manager.clone();
            async move { wrap(create_worker(args, params, key_manager)) }.boxed()
        }))
    }

    fn make_worker_get_peer_id_closure(&self) -> ServiceFunction {
        let key_manager = self.key_manager.clone();
        ServiceFunction::Immut(Box::new(move |args, params| {
            let key_manager = key_manager.clone();
            async move { wrap(get_worker_peer_id(args, params, key_manager)) }.boxed()
        }))
    }

    fn make_worker_list_closure(&self) -> ServiceFunction {
        let key_manager = self.key_manager.clone();
        ServiceFunction::Immut(Box::new(move |_, _| {
            let key_manager = key_manager.clone();
            async move { wrap(worker_list(key_manager)) }.boxed()
        }))
    }

    fn make_worker_remove_closure(&self) -> ServiceFunction {
        let services = self.services.clone();
        let storage = self.spell_storage.clone();
        let spell_event_bus_api = self.spell_event_bus_api.clone();
        let key_manager = self.key_manager.clone();

        ServiceFunction::Immut(Box::new(move |args, params| {
            let storage = storage.clone();
            let services = services.clone();
            let api = spell_event_bus_api.clone();
            let key_manager = key_manager.clone();
            async move {
                wrap_unit(remove_worker(args, params, key_manager, services, storage, api).await)
            }
            .boxed()
        }))
    }
}
