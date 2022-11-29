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

use crate::spells::{get_spell_id, spell_install, spell_list, spell_remove};

use async_std::task::{spawn, JoinHandle};
use std::collections::HashMap;
use std::time::Duration;

use futures::{FutureExt, StreamExt};
use libp2p::PeerId;
use maplit::hashmap;

use aquamarine::AquamarineApi;
use fluence_libp2p::types::Inlet;
use particle_builtins::{wrap, wrap_unit};
use particle_execution::ServiceFunction;
use particle_modules::ModuleRepository;
use particle_services::ParticleAppServices;
use server_config::ResolvedConfig;
use spell_event_bus::scheduler::api::{Event, SchedulerApi};
use spell_storage::SpellStorage;

#[derive(Clone)]
pub struct Sorcerer {
    pub aquamarine: AquamarineApi,
    pub services: ParticleAppServices,
    pub spell_storage: SpellStorage,
    pub scheduler_api: SchedulerApi,
    /// it is temporary, later we will use spell keypairs
    pub node_peer_id: PeerId,
    pub spell_script_particle_ttl: Duration,
}

impl Sorcerer {
    pub fn new(
        services: ParticleAppServices,
        modules: ModuleRepository,
        aquamarine: AquamarineApi,
        config: ResolvedConfig,
        local_peer_id: PeerId,
        scheduler_api: SchedulerApi,
    ) -> (Self, Vec<(String, HashMap<String, ServiceFunction>)>) {
        let spell_storage = SpellStorage::create(
            config.dir_config.spell_base_dir.clone(),
            &services,
            &modules,
        )
        .expect("Spell storage creation");

        let sorcerer = Self {
            aquamarine,
            services,
            spell_storage,
            scheduler_api,
            node_peer_id: local_peer_id,
            spell_script_particle_ttl: config.spell_script_particle_ttl,
        };

        let spell_service_functions = sorcerer.get_spell_service_functions();

        // TODO: reschedule spells
        // sorcerer.spell_storage.get_registered_spells()

        (sorcerer, spell_service_functions)
    }

    pub fn start(self, spell_events_stream: Inlet<Event>) -> JoinHandle<()> {
        spawn(async {
            spell_events_stream
                .for_each_concurrent(None, move |event| {
                    let sorcerer = self.clone();
                    match event {
                        Event::TimeTrigger { id } => {
                            async move {
                                sorcerer.execute_script(id).await;
                            }
                        }
                    }
                })
                .await;
        })
    }

    fn get_spell_service_functions(&self) -> Vec<(String, HashMap<String, ServiceFunction>)> {
        let mut service_functions: Vec<(String, HashMap<String, ServiceFunction>)> = vec![];
        let services = self.services.clone();
        let storage = self.spell_storage.clone();
        let scheduler = self.scheduler_api.clone();
        let install_closure: ServiceFunction = Box::new(move |args, params| {
            let storage = storage.clone();
            let services = services.clone();
            let scheduler_api = scheduler.clone();
            async move {
                wrap(spell_install(
                    storage,
                    services,
                    scheduler_api,
                    args,
                    params,
                ))
            }
            .boxed()
        });

        let services = self.services.clone();
        let storage = self.spell_storage.clone();
        let remove_closure: ServiceFunction = Box::new(move |args, params| {
            let storage = storage.clone();
            let services = services.clone();
            async move { wrap_unit(spell_remove(storage, services, args, params)) }.boxed()
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
        service_functions.push(("spell".to_string(), functions));

        let get_data_srv_closure: ServiceFunction =
            Box::new(move |args, params| async move { wrap(get_spell_id(args, params)) }.boxed());

        service_functions.push((
            "getDataSrv".to_string(),
            hashmap! {"spell_id".to_string() => get_data_srv_closure},
        ));

        service_functions
    }
}
