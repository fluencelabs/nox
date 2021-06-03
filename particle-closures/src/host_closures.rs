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

use crate::error::HostClosureCallError::{DecodeBase58, DecodeUTF8};
use crate::identify::{identify, NodeInfo};

use connection_pool::{ConnectionPoolApi, ConnectionPoolT};
use host_closure::{
    from_base58, Args, Closure, ClosureDescriptor, JError, ParticleClosure, ParticleParameters,
};
use ivalue_utils::{into_record, into_record_opt, ok, unit, IValue};
use kademlia::{KademliaApi, KademliaApiT};
use now_millis::{now_ms, now_sec};
use particle_modules::ModuleRepository;
use particle_protocol::Contact;
use particle_providers::ProviderRepository;
use particle_services::ParticleAppServices;
use script_storage::ScriptStorageApi;
use server_config::ServicesConfig;

use crate::error::HostClosureCallError;
use crate::ipfs::IpfsState;
use async_std::task;
use humantime_serde::re::humantime::format_duration as pretty;
use libp2p::kad::kbucket::Key;
use libp2p::{core::Multiaddr, PeerId};
use multihash::{Code, MultihashDigest, MultihashGeneric};
use parking_lot::{Mutex, MutexGuard};
use serde_json::{json, Value as JValue};
use std::borrow::Borrow;
use std::num::ParseIntError;
use std::time::{Duration, Instant};
use std::{str::FromStr, sync::Arc};
use JValue::Array;

#[derive(Clone)]
pub struct HostClosures<C> {
    pub create_service: ParticleClosure,
    pub remove_service: ParticleClosure,
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
    pub resolve_alias: Closure,
    pub connectivity: C,
    pub script_storage: ScriptStorageApi,

    // deprecated
    pub add_provider: Closure,
    pub get_providers: Closure,

    pub management_peer_id: String,
    pub ipfs_state: Arc<Mutex<IpfsState>>,
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
            remove_service: services.remove_service(),
            call_service: services.call_service(),
            get_interface: services.get_interface(),
            list_services: services.list_services(),
            identify: identify(node_info),
            add_alias: services.add_alias(),
            resolve_alias: services.resolve_alias(),
            connectivity,
            script_storage,
            management_peer_id,
            ipfs_state: Arc::new(Mutex::new(IpfsState::default())),
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
            ("peer", "timestamp_ms")          => ok(json!(now_ms() as u64)),
            ("peer", "timestamp_sec")         => ok(json!(now_sec())),

            ("kad", "neighborhood")           => wrap(self.neighborhood(args)),
            ("kad", "merge")                  => wrap(self.kad_merge(args.function_args)),

            ("srv", "create")                 => (self.create_service)(params, args),
            ("srv", "remove")                 => (self.remove_service)(params, args),
            ("srv", "list")                   => (self.list_services)(args),
            ("srv", "get_interface")          => (self.get_interface)(args),
            ("srv", "add_alias")              => (self.add_alias)(params, args),
            ("srv", "resolve_alias")          => (self.resolve_alias)(args),

            ("dist", "add_module")            => (self.add_module)(args),
            ("dist", "list_modules")          => (self.list_modules)(args),
            ("dist", "get_module_interface")  => (self.get_module_interface)(args),
            ("dist", "add_blueprint")         => (self.add_blueprint)(args),
            ("dist", "list_blueprints")       => (self.get_blueprints)(args),

            ("script", "add")                 => wrap(self.add_script(args, params)),
            ("script", "remove")              => wrap(self.remove_script(args, params)),
            ("script", "list")                => wrap(self.list_scripts()),

            ("op", "noop")                    => unit(),
            ("op", "array")                   => ok(Array(args.function_args)),
            ("op", "identity")                => wrap_opt(Ok(args.function_args.into_iter().next())),
            ("op", "concat")                  => wrap(self.concat(args.function_args)),
            ("op", "string_to_b58")           => wrap(self.string_to_b58(args.function_args)),
            ("op", "string_from_b58")         => wrap(self.string_from_b58(args.function_args)),
            ("op", "bytes_from_b58")          => wrap(self.bytes_from_b58(args.function_args)),
            ("op", "bytes_to_b58")            => wrap(self.bytes_to_b58(args.function_args)),
            ("op", "sha256_string")           => wrap(self.sha256_string(args.function_args)),

            ("ipfs", "get_multiaddr")         => wrap(self.ipfs().get_multiaddr()),
            ("ipfs", "set_multiaddr")         => wrap_opt(self.ipfs().set_multiaddr(args, params, &self.management_peer_id)),
            ("ipfs", "clear_multiaddr")       => wrap(self.ipfs().clear_multiaddr(params, &self.management_peer_id)),

            ("deprecated", "add_provider")    => (self.add_provider)(args),
            ("deprecated", "get_providers")   => (self.get_providers)(args),

            _ => (self.call_service)(params, args),
        };
        log::info!("{} ({})", log_args, pretty(start.elapsed()));
        result
    }

    fn neighborhood(&self, args: Args) -> Result<JValue, JError> {
        let mut args = args.function_args.into_iter();
        let key = from_base58("key", &mut args)?;
        let already_hashed: Option<bool> = Args::maybe_next("already_hashed", &mut args)?;
        let key = if already_hashed == Some(true) {
            MultihashGeneric::from_bytes(&key)?
        } else {
            Code::Sha2_256.digest(&key)
        };
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

        let force = params.init_user_id == self.management_peer_id;

        let uuid: String = Args::next("uuid", &mut args)?;
        let actor = PeerId::from_str(&params.init_user_id)?;

        let ok = task::block_on(self.script_storage.remove_script(uuid, actor, force))?;

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

    fn string_to_b58(&self, args: Vec<serde_json::Value>) -> Result<JValue, JError> {
        let mut args = args.into_iter();
        let string: String = Args::next("string", &mut args)?;
        let b58 = bs58::encode(string).into_string();
        Ok(JValue::String(b58))
    }

    /// Attempts to decode UTF8 string from a given base58 string
    /// May fail at base58 decoding and on UTF8 decoding
    fn string_from_b58(&self, args: Vec<serde_json::Value>) -> Result<JValue, JError> {
        let mut args = args.into_iter();
        let string: String = Args::next("b58_string", &mut args)?;
        let vec = bs58::decode(string).into_vec().map_err(DecodeBase58)?;
        let string = String::from_utf8(vec).map_err(DecodeUTF8)?;
        Ok(JValue::String(string))
    }

    fn bytes_from_b58(&self, args: Vec<serde_json::Value>) -> Result<JValue, JError> {
        let mut args = args.into_iter();
        let string: String = Args::next("b58_string", &mut args)?;
        let vec = bs58::decode(string).into_vec().map_err(DecodeBase58)?;
        Ok(json!(vec))
    }

    fn bytes_to_b58(&self, args: Vec<serde_json::Value>) -> Result<JValue, JError> {
        let mut args = args.into_iter();
        let bytes: Vec<u8> = Args::next("bytes", &mut args)?;
        let string = bs58::encode(bytes).into_string();
        Ok(JValue::String(string))
    }

    /// Returns SHA256 of the passed string
    /// Accepts 3 arguments:
    /// `string` – string to hash
    /// `digest_only` boolean – if set to true, return only SHA256 digest, otherwise (by default) – full multihash
    /// `as_bytes` boolean - if set to true, return result as array of bytes, otherwise (by default) – as base58 string
    fn sha256_string(&self, args: Vec<serde_json::Value>) -> Result<JValue, JError> {
        let mut args = args.into_iter();
        let string: String = Args::next("string", &mut args)?;
        let digest_only: Option<bool> = Args::maybe_next("digest_only", &mut args)?;
        let as_bytes: Option<bool> = Args::maybe_next("as_bytes", &mut args)?;
        let multihash: MultihashGeneric<_> = Code::Sha2_256.digest(string.as_bytes());

        let result = if digest_only == Some(true) {
            multihash.digest().to_vec()
        } else {
            multihash.to_bytes()
        };

        if as_bytes == Some(true) {
            Ok(json!(result))
        } else {
            let b58 = bs58::encode(result).into_string();
            Ok(JValue::String(b58))
        }
    }

    /// Merge, sort by distance to first key, return top K
    /// K is optional. If not passed, all elements are returned.
    fn kad_merge(&self, args: Vec<serde_json::Value>) -> Result<JValue, JError> {
        let mut args = args.into_iter();
        let target: String = Args::next("target", &mut args)?;
        let left: Vec<String> = Args::next("left", &mut args)?;
        let right: Vec<String> = Args::next("right", &mut args)?;
        let count: Option<usize> = Args::maybe_next("count", &mut args)?;

        let target = bs58::decode(target).into_vec().map_err(DecodeBase58)?;
        let target = Key::from(target);
        let left = left.into_iter();
        let right = right.into_iter();

        let mut keys: Vec<Key<_>> = left
            .chain(right)
            .map(|b58_str| {
                Ok(Key::from(
                    bs58::decode(b58_str).into_vec().map_err(DecodeBase58)?,
                ))
            })
            .collect::<Result<Vec<_>, HostClosureCallError>>()?;
        keys.sort_by_cached_key(|k| target.distance(k.as_ref()));

        let keys = keys
            .into_iter()
            .map(|k| bs58::encode(k.into_preimage()).into_string());

        let keys: Vec<_> = if let Some(count) = count {
            keys.take(count).collect()
        } else {
            keys.collect()
        };

        Ok(json!(keys))
    }

    /// Flattens an array of arrays
    fn concat(&self, args: Vec<serde_json::Value>) -> Result<JValue, JError> {
        let flattened: Vec<JValue> = args.into_iter().enumerate().flat_map().try_fold(
            vec![],
            |mut acc, (i, mut v)| match v.take() {
                JValue::Array(mut array) => {
                    acc.append(&mut array);
                    Ok(acc)
                }
                _ => Err(JError::new(format!(
                    "all arguments of 'concat' must be arrays: argument #{} is not",
                    i
                ))),
            },
        )?;

        Ok(JValue::Array(flattened))
    }

    fn kademlia(&self) -> &KademliaApi {
        self.connectivity.as_ref()
    }

    fn connection_pool(&self) -> &ConnectionPoolApi {
        self.connectivity.as_ref()
    }

    fn ipfs(&self) -> MutexGuard<'_, IpfsState> {
        self.ipfs_state.lock()
    }
}

fn wrap(r: Result<JValue, JError>) -> Option<IValue> {
    into_record(r.map_err(Into::into))
}

fn wrap_opt(r: Result<Option<JValue>, JError>) -> Option<IValue> {
    into_record_opt(r.map_err(Into::into))
}
