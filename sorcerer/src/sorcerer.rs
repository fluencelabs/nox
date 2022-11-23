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

use crate::services::{get_spell_id, spell_install, spell_list, spell_remove};

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use futures::future::BoxFuture;
use futures::FutureExt;
use libp2p::PeerId;
use maplit::hashmap;
use serde_json::{json, Value as JValue};
use JValue::Array;

use aquamarine::AquamarineApi;
use connection_pool::ConnectionPoolApi;
use events_dispatcher::scheduler::api::{SchedulerApi, TimerConfig};
use events_dispatcher::scheduler::{Scheduler, SchedulerConfig};
use kademlia::KademliaApi;
use now_millis::now_ms;
use particle_args::{Args, JError};
use particle_builtins::{wrap, wrap_unit, Builtins};
use particle_execution::{
    FunctionOutcome, ParticleFunction, ParticleFunctionStatic, ParticleParams, ServiceFunction,
};
use particle_modules::ModuleRepository;
use particle_services::ParticleAppServices;
use server_config::ResolvedConfig;
use spell_storage::SpellStorage;
use uuid_utils::uuid;

#[derive(Clone)]
pub struct Sorcerer<C> {
    pub aquamarine: AquamarineApi,
    pub builtins: Arc<Builtins<C>>,
    pub spell_storage: SpellStorage,
    pub scheduler: Scheduler,
    pub spell_scheduler_api: SchedulerApi,
    /// it is temporary, later we will use spell keypairs
    pub node_peer_id: PeerId,
}

impl<C> Sorcerer<C>
where
    C: Clone + Send + Sync + 'static + AsRef<KademliaApi> + AsRef<ConnectionPoolApi>,
{
    pub fn new(
        builtins: Arc<Builtins<C>>,
        aquamarine: AquamarineApi,
        config: ResolvedConfig,
        local_peer_id: PeerId,
    ) -> Self {
        let spell_storage = Self::restore_spells(
            config.dir_config.spell_base_dir.clone(),
            &builtins.services,
            &builtins.modules,
        );
        let (scheduler, spell_scheduler_api) = Scheduler::new(
            SchedulerConfig {
                timer_resolution: config.script_storage_timer_resolution,
            },
            |id| log::warn!("Sending spell: {}", id),
        );

        let mut sorcerer = Self {
            aquamarine,
            builtins,
            spell_storage,
            scheduler,
            spell_scheduler_api,
            node_peer_id: local_peer_id,
        };

        sorcerer.register_service_functions();
        let sorcerer_clone = sorcerer.clone();
        sorcerer.scheduler.set_callback(move |id| {
            sorcerer_clone.execute_script(id.to_string());
        });
        sorcerer
    }

    fn register_service_functions(&self) {
        let services_install = self.builtins.services.clone();
        let storage_install = self.spell_storage.clone();
        let scheduler_api_install = self.spell_scheduler_api.clone();
        let install_closure: ServiceFunction = Box::new(move |args, params| {
            let storage = storage_install.clone();
            let services = services_install.clone();
            let scheduler_api = scheduler_api_install.clone();
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

        let services_remove = self.builtins.services.clone();
        let storage_remove = self.spell_storage.clone();
        let remove_closure: ServiceFunction = Box::new(move |args, params| {
            let storage = storage_remove.clone();
            let services = services_remove.clone();
            async move { wrap_unit(spell_remove(storage, services, args, params)) }.boxed()
        });

        let storage_list = self.spell_storage.clone();
        let list_closure: ServiceFunction = Box::new(move |_args, _params| {
            let storage = storage_list.clone();
            async move { wrap(spell_list(storage)) }.boxed()
        });

        let functions = hashmap! {
            "install".to_string() => install_closure,
            "remove".to_string() => remove_closure,
            "list".to_string() => list_closure
        };
        self.builtins.extend("spell".to_string(), functions);

        let get_data_srv_closure: ServiceFunction =
            Box::new(move |args, params| async move { wrap(get_spell_id(args, params)) });

        self.builtins.extend(
            "getDataSrv".to_string(),
            hashmap! {"spell_id".to_string() => get_data_srv_closure},
        )
    }

    fn restore_spells(
        spells_base_dir: PathBuf,
        services: &ParticleAppServices,
        modules: &ModuleRepository,
    ) -> SpellStorage {
        // Load the up-to-date spell service and save its blueprint_id to create spell services from it.
        let spell_blueprint_id = services.load_spell_service(spells_base_dir).unwrap();
        // Find blueprint ids of the already existing spells. They might be of older versions of the spell service.
        // These blueprint ids marked with work "spell" to differ from other blueprints.
        let all_spell_blueprint_ids = modules
            .get_blueprints()
            .into_iter()
            .filter(|blueprint| blueprint.name == "spell")
            .map(|x| x.id)
            .collect::<HashSet<_>>();
        // Find already created spells by corresponding blueprint_ids.
        let registered_spells = services
            .list_service_with_blueprints()
            .into_iter()
            .filter(|(_, blueprint)| all_spell_blueprint_ids.contains(blueprint))
            .map(|(id, _)| id)
            .collect::<_>();
        // TODO: read spells configs
        // TODO: reschedule spells

        SpellStorage::new(
            spell_blueprint_id,
            all_spell_blueprint_ids,
            registered_spells,
        )
    }
}
