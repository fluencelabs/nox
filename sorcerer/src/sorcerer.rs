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

use aquamarine::AquamarineApi;
use connection_pool::ConnectionPoolApi;
use events_dispatcher::scheduler::api::{SchedulerApi, TimerConfig};
use events_dispatcher::scheduler::{Scheduler, SchedulerConfig};
use futures::future::BoxFuture;
use futures::FutureExt;
use kademlia::KademliaApi;
use maplit::hashmap;
use now_millis::now_ms;
use particle_args::{Args, JError};
use particle_builtins::{wrap, wrap_unit, Builtins};
use particle_execution::{
    FunctionOutcome, ParticleFunction, ParticleFunctionStatic, ParticleParams, ServiceFunction,
};
use particle_modules::ModuleRepository;
use particle_services::ParticleAppServices;
use serde_json::{json, Value as JValue};
use server_config::ResolvedConfig;
use spell_storage::SpellStorage;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use uuid_utils::uuid;
use JValue::Array;

pub struct Sorcerer<C> {
    pub aquamarine: AquamarineApi,
    pub builtins: Arc<Builtins<C>>,
    pub spell_storage: SpellStorage,
    pub scheduler: Scheduler,
    pub spell_scheduler_api: SchedulerApi,
}

impl<C> Sorcerer<C>
where
    C: Clone + Send + Sync + 'static + AsRef<KademliaApi> + AsRef<ConnectionPoolApi>,
{
    pub fn new(
        builtins: Arc<Builtins<C>>,
        aquamarine: AquamarineApi,
        config: ResolvedConfig,
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

        let sorcerer = Self {
            aquamarine,
            builtins,
            spell_storage,
            scheduler,
            spell_scheduler_api,
        };

        sorcerer.register_service_functions();
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
                wrap(Sorcerer::<C>::spell_install(
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
            async move { wrap_unit(Sorcerer::<C>::spell_remove(storage, services, args, params)) }
                .boxed()
        });

        let storage_list = self.spell_storage.clone();
        let list_closure: ServiceFunction = Box::new(move |args, params| {
            let storage = storage_list.clone();
            async move { wrap(Sorcerer::<C>::spell_list(storage, args, params)) }.boxed()
        });

        let functions = hashmap! {
            "install".to_string() => install_closure,
            "remove".to_string() => remove_closure,
            "list".to_string() => list_closure
        };
        self.builtins.extend("spell".to_string(), functions);

        let get_data_srv_closure: ServiceFunction = Box::new(move |args, params| async move {
            wrap({
                if params.id.starts_with("spell_") {
                    Ok(json!(params.id.split('_').next().next())))
                } else {
                    Err()
                }
            })
        });

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

    fn spell_install(
        spell_storage: SpellStorage,
        services: ParticleAppServices,
        spell_scheduler_api: SchedulerApi,
        sargs: Args,
        params: ParticleParams,
    ) -> Result<JValue, JError> {
        let mut args = sargs.function_args.clone().into_iter();
        let script: String = Args::next("script", &mut args)?;
        // TODO: redo config when other event are supported
        let period: u64 = Args::next("period", &mut args)?;

        let service_id =
            services.create_service(spell_storage.get_blueprint(), params.init_peer_id)?;
        spell_storage.register_spell(service_id.clone());

        // Save the script to the spell.
        let script_arg = JValue::String(script);
        let script_tetraplet = sargs.tetraplets[0].clone();
        let spell_args = Args {
            service_id: service_id.clone(),
            function_name: "set_script_source_to_file".to_string(),
            function_args: vec![script_arg],
            tetraplets: vec![script_tetraplet],
        };
        let particle = ParticleParams {
            id: uuid(),
            init_peer_id: params.init_peer_id,
            timestamp: now_ms() as u64,
            ttl: params.ttl,
            script: "".to_string(),
            signature: vec![],
        };
        services.call_service(spell_args, particle);
        // TODO: also save config

        // Scheduling the spell
        spell_scheduler_api.add(
            service_id.clone(),
            TimerConfig {
                period: Duration::from_secs(period),
            },
        )?;
        Ok(JValue::String(service_id))
    }

    fn spell_list(
        spell_storage: SpellStorage,
        _args: Args,
        _params: ParticleParams,
    ) -> Result<JValue, JError> {
        Ok(Array(
            spell_storage
                .get_registered_spells()
                .into_iter()
                .map(JValue::String)
                .collect(),
        ))
    }
    fn spell_remove(
        spell_storage: SpellStorage,
        services: ParticleAppServices,
        args: Args,
        params: ParticleParams,
    ) -> Result<(), JError> {
        let mut args = args.function_args.into_iter();
        let spell_id: String = Args::next("spell_id", &mut args)?;

        spell_storage.unregister_spell(&spell_id);
        services.remove_service(spell_id, params.init_peer_id)?;
        Ok(())
    }
}
