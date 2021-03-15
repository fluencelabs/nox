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

use crate::identify::{identify, NodeInfo};

use connection_pool::{ConnectionPoolApi, ConnectionPoolT};
use host_closure::{
    from_base58, Args, Closure, ClosureDescriptor, JError, ParticleClosure, ParticleParameters,
};
use ivalue_utils::{into_record, into_record_opt, ok, IValue};
use kademlia::{KademliaApi, KademliaApiT};
use now_millis::{now_ms, now_sec};
use particle_protocol::Contact;
use particle_providers::ProviderRepository;
use particle_services::ParticleAppServices;
use particle_services::ServiceError::Forbidden;
use script_storage::ScriptStorageApi;
use server_config::ServicesConfig;

use async_std::task;
use humantime_serde::re::humantime::format_duration as pretty;
use libp2p::{core::Multiaddr, PeerId};
use multihash::{Code, MultihashDigest};
use particle_modules::ModuleRepository;
use serde_json::{json, Value as JValue};
use std::borrow::Borrow;
use std::num::ParseIntError;
use std::time::{Duration, Instant};
use std::{str::FromStr, sync::Arc};
use JValue::Array;

#[derive(Clone)]
pub struct HostClosures<C> {
    pub create_service: ParticleClosure,
    pub call_service: ParticleClosure,

    pub add_module: Closure,
    pub add_blueprint: Closure,
    pub list_modules: Closure,
    pub get_module_interface: Closure,
    pub get_blueprints: Closure,

    pub get_interface: Closure,
    pub list_services: Closure,

    pub identify: Closure,
    pub add_alias: ParticleClosure,
    pub connectivity: C,
    pub script_storage: ScriptStorageApi,

    // deprecated
    pub add_provider: Closure,
    pub get_providers: Closure,

    pub management_peer_id: String,
}

impl<C: Clone + Send + Sync + 'static + AsRef<KademliaApi> + AsRef<ConnectionPoolApi>>
    HostClosures<C>
{
    pub fn new(
        connectivity: C,
        script_storage: ScriptStorageApi,
        node_info: NodeInfo,
        config: ServicesConfig,
    ) -> Self {
        let modules_dir = config.modules_dir.clone();
        let blueprint_dir = config.blueprint_dir.clone();
        let providers = ProviderRepository::new(config.local_peer_id);
        let modules = ModuleRepository::new(&modules_dir, &blueprint_dir);

        let management_peer_id = config.management_peer_id.to_base58();
        let services = ParticleAppServices::new(config, modules.clone());

        Self {
            add_provider: providers.add_provider(),
            get_providers: providers.get_providers(),
            list_modules: modules.list_modules(),
            get_module_interface: modules.get_interface(),
            get_blueprints: modules.get_blueprints(),
            add_module: modules.add_module(),
            add_blueprint: modules.add_blueprint(),
            create_service: services.create_service(),
            call_service: services.call_service(),
            get_interface: services.get_interface(),
            list_services: services.list_services(),
            identify: identify(node_info),
            add_alias: services.add_alias(),
            connectivity,
            script_storage,
            management_peer_id,
        }
    }

    pub fn descriptor(self) -> ClosureDescriptor {
        Arc::new(move || {
            let this = self.clone();
            Box::new(move |particle, args| this.route(particle, args))
        })
    }

    fn route(&self, params: ParticleParameters, args: Vec<IValue>) -> Option<IValue> {
        let args = match Args::parse(args) {
            Ok(args) => args,
            Err(err) => {
                log::warn!("host function args parse error: {:?}", err);
                return ivalue_utils::error(json!(err.to_string()));
            }
        };

        log::trace!("Host function call, args: {:#?}", args);
        let log_args = format!(
            "Executed host call {:?} {:?}",
            args.service_id, args.function_name
        );

        let start = Instant::now();
        // TODO: maybe error handling and conversion should happen here, so it is possible to log::warn errors
        #[rustfmt::skip]
        let result = match (args.service_id.as_str(), args.function_name.as_str()) {
            ("peer", "is_connected")          => wrap(self.is_connected(args)),
            ("peer", "connect")               => wrap(self.connect(args)),
            ("peer", "get_contact")           => wrap_opt(self.get_contact(args)),
            ("peer", "identify")              => (self.identify)(args),
            ("peer", "timestamp_ms")          => ok(json!(now_ms())),
            ("peer", "timestamp_sec")         => ok(json!(now_sec())),

            ("kad", "neighborhood")           => wrap(self.neighborhood(args)),

            ("srv", "create")                 => (self.create_service)(params, args),
            ("srv", "list")                   => (self.list_services)(args),
            ("srv", "get_interface")          => (self.get_interface)(args),
            ("srv", "add_alias")              => (self.add_alias)(params, args),

            ("dist", "add_module")            => (self.add_module)(args),
            ("dist", "list_modules")          => (self.list_modules)(args),
            ("dist", "get_module_interface")  => (self.get_module_interface)(args),
            ("dist", "add_blueprint")         => (self.add_blueprint)(args),
            ("dist", "list_blueprints")       => (self.get_blueprints)(args),

            ("script", "add")                 => wrap(self.add_script(args, params)),
            ("script", "remove")              => wrap(self.remove_script(args, params)),
            ("script", "list")                => wrap(self.list_scripts()),

            ("op", "identity")                => ok(Array(args.function_args)),

            ("deprecated", "add_provider")    => (self.add_provider)(args),
            ("deprecated", "get_providers")   => (self.get_providers)(args),

            _ => (self.call_service)(params, args),
        };
        log::info!("{} ({})", log_args, pretty(start.elapsed()));
        result
    }

    fn neighborhood(&self, args: Args) -> Result<JValue, JError> {
        let key = from_base58("key", &mut args.function_args.into_iter())?;
        let key = Code::Sha2_256.digest(&key);
        let neighbors = task::block_on(self.kademlia().neighborhood(key));
        let neighbors = neighbors
            .map(|vs| json!(vs.into_iter().map(|id| id.to_string()).collect::<Vec<_>>()))?;

        Ok(neighbors)
    }

    fn is_connected(&self, args: Args) -> Result<JValue, JError> {
        let peer: String = Args::next("peer_id", &mut args.function_args.into_iter())?;
        let peer = PeerId::from_str(peer.as_str())?;
        let ok = task::block_on(self.connection_pool().is_connected(peer));
        Ok(json!(ok))
    }

    fn connect(&self, args: Args) -> Result<JValue, JError> {
        let mut args = args.function_args.into_iter();

        let peer_id: String = Args::next("peer_id", &mut args)?;
        let peer_id = PeerId::from_str(peer_id.as_str())?;
        let addrs: Vec<Multiaddr> = Args::maybe_next("addresses", &mut args)?.unwrap_or_default();

        let contact = Contact::new(peer_id, addrs);

        let ok = task::block_on(self.connection_pool().connect(contact));
        Ok(json!(ok))
    }

    fn get_contact(&self, args: Args) -> Result<Option<JValue>, JError> {
        let peer: String = Args::next("peer_id", &mut args.function_args.into_iter())?;
        let peer = PeerId::from_str(peer.as_str())?;
        let contact = task::block_on(self.connection_pool().get_contact(peer));
        Ok(contact.map(|c| json!(c)))
    }

    fn add_script(&self, args: Args, params: ParticleParameters) -> Result<JValue, JError> {
        #[derive(thiserror::Error, Debug)]
        #[error("Error while deserializing field interval_sec: not a valid u64")]
        struct Error(#[source] ParseIntError);
        let mut args = args.function_args.into_iter();

        let script: String = Args::next("script", &mut args)?;
        let interval = Args::maybe_next("interval_sec", &mut args)?;
        let interval = interval
            .map(|s: String| s.parse::<u64>())
            .transpose()
            .map_err(Error)?;
        let interval = interval.map(Duration::from_secs);
        let creator = PeerId::from_str(&params.init_user_id)?;
        let id = self.script_storage.add_script(script, interval, creator)?;

        Ok(json!(id))
    }

    fn remove_script(&self, args: Args, params: ParticleParameters) -> Result<JValue, JError> {
        let mut args = args.function_args.into_iter();

        if params.init_user_id != self.management_peer_id {
            return Err(Forbidden(
                params.init_user_id + " " + self.management_peer_id.as_str(),
                "remove_script".to_string(),
            )
            .into());
        };

        let uuid: String = Args::next("uuid", &mut args)?;
        let actor = PeerId::from_str(&params.init_user_id)?;

        let ok = task::block_on(self.script_storage.remove_script(uuid, actor))?;

        Ok(json!(ok))
    }

    fn list_scripts(&self) -> Result<JValue, JError> {
        let scripts = task::block_on(self.script_storage.list_scripts())?;

        Ok(JValue::Array(
            scripts
                .into_iter()
                .map(|(id, script)| {
                    let id: &String = id.borrow();
                    json!({
                        "id": id,
                        "src": script.src,
                        "failures": script.failures,
                        "interval": script.interval.map(|i| pretty(i).to_string()),
                        "owner": script.owner.to_string(),
                    })
                })
                .collect(),
        ))
    }

    fn kademlia(&self) -> &KademliaApi {
        self.connectivity.as_ref()
    }

    fn connection_pool(&self) -> &ConnectionPoolApi {
        self.connectivity.as_ref()
    }
}

fn wrap(r: Result<JValue, JError>) -> Option<IValue> {
    into_record(r.map_err(Into::into))
}

fn wrap_opt(r: Result<Option<JValue>, JError>) -> Option<IValue> {
    into_record_opt(r.map_err(Into::into))
}
