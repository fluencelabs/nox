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

use connection_pool::{ConnectionPool, ConnectionPoolApi, Contact};
use host_closure::{
    from_base58, Args, Closure, ClosureDescriptor, JError, ParticleClosure, ParticleParameters,
};
use ivalue_utils::{into_record, into_record_opt, ok, IValue};
use kademlia::{KademliaApi, KademliaApiOutlet};
use particle_providers::ProviderRepository;
use particle_services::{ParticleAppServices, ServicesConfig};

use async_std::task;
use libp2p::PeerId;
use multihash::Code;
use multihash::MultihashDigest;
use serde_json::{json, Value as JValue};
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use JValue::Array;

#[derive(Clone)]
pub struct HostClosures<C> {
    pub create_service: ParticleClosure,
    pub call_service: ParticleClosure,
    pub add_module: Closure,
    pub add_blueprint: Closure,
    pub get_modules: Closure,
    pub get_blueprints: Closure,
    pub add_provider: Closure,
    pub get_providers: Closure,
    pub get_interface: Closure,
    pub get_active_interfaces: Closure,
    pub identify: Closure,
    pub connectivity: C,
}

impl<C: Clone + Send + Sync + 'static + AsRef<KademliaApiOutlet> + AsRef<ConnectionPoolApi>>
    HostClosures<C>
{
    pub fn new(
        connectivity: C,
        node_info: NodeInfo,
        local_peer_id: PeerId,
        services_base_dir: PathBuf,
        envs: HashMap<Vec<u8>, Vec<u8>>,
    ) -> Result<Self, std::io::Error> {
        let config = ServicesConfig::new(local_peer_id.to_string(), services_base_dir, envs)?;
        let modules_dir = config.modules_dir.clone();
        let blueprint_dir = config.blueprint_dir.clone();
        let services = ParticleAppServices::new(config);
        let providers = ProviderRepository::new(local_peer_id);

        Ok(Self {
            add_provider: providers.add_provider(),
            get_providers: providers.get_providers(),
            get_modules: particle_modules::get_modules(modules_dir.clone()),
            get_blueprints: particle_modules::get_blueprints(blueprint_dir.clone()),
            add_module: particle_modules::add_module(modules_dir),
            add_blueprint: particle_modules::add_blueprint(blueprint_dir),
            create_service: services.create_service(),
            call_service: services.call_service(),
            get_interface: services.get_interface(),
            get_active_interfaces: services.get_active_interfaces(),
            identify: identify(node_info),
            connectivity,
        })
    }

    pub fn descriptor(self) -> ClosureDescriptor {
        Arc::new(move || {
            let this = self.clone();
            Box::new(move |particle, args| this.route(particle, args))
        })
    }

    fn route(&self, particle: ParticleParameters, args: Vec<IValue>) -> Option<IValue> {
        let args = match Args::parse(args) {
            Ok(args) => args,
            Err(err) => {
                log::warn!("host function args parse error: {:?}", err);
                return ivalue_utils::error(json!(err.to_string()));
            }
        };
        log::info!(
            "Host function call {:?} {}",
            args.service_id,
            args.function_name
        );
        log::debug!("Host function call, args: {:#?}", args);

        // TODO: maybe error handling and conversion should happen here, so it is possible to log::warn errors
        #[rustfmt::skip]
        match (args.service_id.as_str(), args.function_name.as_str()) {
            ("peer", "is_connected")   => wrap(self.is_connected(args)),
            ("peer", "connect")        => wrap(self.connect(args)),
            ("peer", "get_contact")    => wrap_opt(self.get_contact(args)),

            ("dht", "neighborhood")    => wrap(self.neighborhood(args)),
            ("dht", "add_provider")    => (self.add_provider)(args),
            ("dht", "get_providers")   => (self.get_providers)(args),

            ("srv", "create")          => (self.create_service)(particle, args),
            ("srv", "get_interface")   => (self.get_interface)(args),
            ("srv", "get_interfaces")  => (self.get_active_interfaces)(args),

            ("dist", "add_module")     => (self.add_module)(args),
            ("dist", "add_blueprint")  => (self.add_blueprint)(args),
            ("dist", "get_modules")    => (self.get_modules)(args),
            ("dist", "get_blueprints") => (self.get_blueprints)(args),

            ("op", "identify") => (self.identify)(args),
            ("op", "identity") => ok(Array(args.function_args)),

            _ => (self.call_service)(particle, args),
        }
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
        let contact: Contact = Args::next("peer_id", &mut args.function_args.into_iter())?;
        let ok = task::block_on(self.connection_pool().connect(contact));
        Ok(json!(ok))
    }

    fn get_contact(&self, args: Args) -> Result<Option<JValue>, JError> {
        let peer: String = Args::next("peer_id", &mut args.function_args.into_iter())?;
        let peer = PeerId::from_str(peer.as_str())?;
        let contact = task::block_on(self.connection_pool().get_contact(peer));
        Ok(contact.map(|c| json!(c)))
    }

    fn kademlia(&self) -> &KademliaApiOutlet {
        // AsRef::<Kad>::as_ref(&self.connectivity)
        self.connectivity.as_ref()
    }

    #[allow(dead_code)]
    fn connection_pool(&self) -> &ConnectionPoolApi {
        // AsRef::<ConnectionPoolApi>::as_ref(&self.connectivity)
        self.connectivity.as_ref()
    }
}

fn wrap(r: Result<JValue, JError>) -> Option<IValue> {
    into_record(r.map_err(Into::into))
}

fn wrap_opt(r: Result<Option<JValue>, JError>) -> Option<IValue> {
    into_record_opt(r.map_err(Into::into))
}
