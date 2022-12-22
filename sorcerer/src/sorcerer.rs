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
use libp2p::PeerId;
use maplit::hashmap;

use aquamarine::AquamarineApi;
use fluence_libp2p::types::Inlet;
use particle_args::JError;
use particle_builtins::{wrap, wrap_unit};
use particle_execution::ServiceFunction;
use particle_modules::ModuleRepository;
use particle_services::ParticleAppServices;
use server_config::ResolvedConfig;
use spell_event_bus::api::{from_user_config, SpellEventBusApi, TriggerEvent};
use spell_storage::SpellStorage;

use crate::spells::{
    get_spell_arg, get_spell_id, spell_install, spell_list, spell_remove, store_error,
    store_response,
};
use crate::utils::process_func_outcome;

#[derive(Clone)]
pub struct Sorcerer {
    pub aquamarine: AquamarineApi,
    pub services: ParticleAppServices,
    pub spell_storage: SpellStorage,
    pub spell_event_bus_api: SpellEventBusApi,
    /// TODO: use owner-specific spell keypairs
    pub node_peer_id: PeerId,
    pub spell_script_particle_ttl: Duration,
}

type CustomService = (
    // service id
    String,
    // functions
    HashMap<String, ServiceFunction>,
    // unhandled
    Option<ServiceFunction>,
);

impl Sorcerer {
    pub fn new(
        services: ParticleAppServices,
        modules: ModuleRepository,
        aquamarine: AquamarineApi,
        config: ResolvedConfig,
        local_peer_id: PeerId,
        spell_event_bus_api: SpellEventBusApi,
    ) -> (Self, Vec<CustomService>) {
        let spell_storage =
            SpellStorage::create(&config.dir_config.spell_base_dir, &services, &modules)
                .expect("Spell storage creation");

        let sorcerer = Self {
            aquamarine,
            services,
            spell_storage,
            spell_event_bus_api,
            node_peer_id: local_peer_id,
            spell_script_particle_ttl: config.max_spell_particle_ttl,
        };

        let spell_service_functions = sorcerer.get_spell_service_functions();

        (sorcerer, spell_service_functions)
    }

    async fn resubscribe_spells(&self) {
        for spell_id in self.spell_storage.get_registered_spells() {
            log::info!("Rescheduling spell {}", spell_id);
            let result: Result<(), JError> = try {
                let result = process_func_outcome::<TriggerConfigValue>(
                    self.services.call_function(
                        &spell_id,
                        "get_trigger_config",
                        vec![],
                        None,
                        self.node_peer_id,
                        self.spell_script_particle_ttl,
                    ),
                    &spell_id,
                    "get_trigger_config",
                )?;
                let config = from_user_config(result.config)?;
                self.spell_event_bus_api
                    .subscribe(spell_id.clone(), config.clone())
                    .await?;
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
                        sorcerer.execute_script(spell_event.spell_id).await;
                    }
                })
                .await;
        })
    }

    fn get_spell_service_functions(&self) -> Vec<CustomService> {
        let mut service_functions: Vec<CustomService> = vec![];
        let services = self.services.clone();
        let storage = self.spell_storage.clone();
        let spell_event_bus = self.spell_event_bus_api.clone();
        let install_closure: ServiceFunction = Box::new(move |args, params| {
            let storage = storage.clone();
            let services = services.clone();
            let spell_event_bus_api = spell_event_bus.clone();
            async move {
                wrap(spell_install(args, params, storage, services, spell_event_bus_api).await)
            }
            .boxed()
        });

        let services = self.services.clone();
        let storage = self.spell_storage.clone();
        let spell_event_bus_api = self.spell_event_bus_api.clone();
        let remove_closure: ServiceFunction = Box::new(move |args, params| {
            let storage = storage.clone();
            let services = services.clone();
            let api = spell_event_bus_api.clone();
            async move { wrap_unit(spell_remove(args, params, storage, services, api).await) }
                .boxed()
        });

        let storage = self.spell_storage.clone();
        let list_closure: ServiceFunction = Box::new(move |_args, _params| {
            let storage = storage.clone();
            async move { wrap(spell_list(storage)) }.boxed()
        });

        let functions = hashmap! {
            "install".to_string() => install_closure,
            "remove".to_string() => remove_closure,
            "list".to_string() => list_closure
        };
        service_functions.push(("spell".to_string(), functions, None));

        let get_spell_id_closure: ServiceFunction =
            Box::new(move |args, params| async move { wrap(get_spell_id(args, params)) }.boxed());

        let services = self.services.clone();
        let get_spell_arg_closure: ServiceFunction = Box::new(move |args, params| {
            let services = services.clone();
            async move { wrap(get_spell_arg(args, params, services)) }.boxed()
        });
        service_functions.push((
            "getDataSrv".to_string(),
            hashmap! {"spell_id".to_string() => get_spell_id_closure},
            Some(get_spell_arg_closure),
        ));

        let services = self.services.clone();
        let error_handler_closure: ServiceFunction = Box::new(move |args, params| {
            let services = services.clone();
            async move { wrap_unit(store_error(args, params, services)) }.boxed()
        });

        service_functions.push((
            "errorHandlingSrv".to_string(),
            hashmap! {"error".to_string() => error_handler_closure},
            None,
        ));

        let services = self.services.clone();
        let response_handler_closure: ServiceFunction = Box::new(move |args, params| {
            let services = services.clone();
            async move { wrap_unit(store_response(args, params, services)) }.boxed()
        });

        service_functions.push((
            "callbackSrv".to_string(),
            hashmap! {"response".to_string() => response_handler_closure},
            None,
        ));

        service_functions
    }
}
