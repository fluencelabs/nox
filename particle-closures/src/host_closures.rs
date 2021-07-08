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

use crate::error::HostClosureCallError;
use crate::error::HostClosureCallError::{DecodeBase58, DecodeUTF8};
use crate::identify::NodeInfo;
use crate::ipfs::IpfsState;

use connection_pool::{ConnectionPoolApi, ConnectionPoolT};
use host_closure::{
    from_base58, AVMEffect, Args, CallServiceArgs, ClosureDescriptor, JError, ParticleParameters,
};
use ivalue_utils::{error, into_record, into_record_opt, ok, unit, IValue};
use kademlia::{KademliaApi, KademliaApiT};
use now_millis::{now_ms, now_sec};
use particle_modules::{
    AddBlueprint, ModuleConfig, ModuleRepository, NamedModuleConfig, WASIConfig,
};
use particle_protocol::Contact;
use particle_services::ParticleAppServices;
use script_storage::ScriptStorageApi;
use server_config::ServicesConfig;

use async_std::task;
use humantime_serde::re::humantime::format_duration as pretty;
use libp2p::{core::Multiaddr, kad::kbucket::Key, kad::K_VALUE, PeerId};
use multihash::{Code, MultihashDigest, MultihashGeneric};
use parking_lot::{Mutex, MutexGuard};
use serde_json::{json, Value as JValue};
use std::borrow::Borrow;
use std::num::ParseIntError;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use std::{str::FromStr, sync::Arc};
use JValue::Array;

#[derive(Clone)]
pub struct HostClosures<C> {
    pub connectivity: C,
    pub script_storage: ScriptStorageApi,

    pub management_peer_id: String,
    pub startup_management_peer_id: String,
    pub ipfs_state: Arc<Mutex<IpfsState>>,

    pub modules: ModuleRepository,
    pub services: ParticleAppServices,
    pub node_info: NodeInfo,
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
        let modules = ModuleRepository::new(&modules_dir, &blueprint_dir);

        let management_peer_id = config.management_peer_id.to_base58();
        let startup_management_peer_id = config.startup_management_peer_id.to_base58();
        let services = ParticleAppServices::new(config, modules.clone());

        Self {
            connectivity,
            script_storage,
            management_peer_id,
            startup_management_peer_id,
            ipfs_state: Arc::new(Mutex::new(IpfsState::default())),
            modules,
            services,
            node_info,
        }
    }

    pub fn descriptor(self) -> ClosureDescriptor {
        Arc::new(move || {
            let this = self.clone();
            Box::new(move |args| this.route(args))
        })
    }

    fn route(&self, args: CallServiceArgs) -> Option<IValue> {
        let function_args = match Args::parse(args.function_args) {
            Ok(args) => args,
            Err(err) => {
                log::warn!("host function args parse error: {:?}", err);
                return ivalue_utils::error(json!(err.to_string()));
            }
        };

        let params = args.particle_parameters;

        log::trace!("Host function call, args: {:#?}", function_args);
        log::debug!(
            "Will execute host call {:?} {:?} [{}]",
            function_args.service_id,
            function_args.function_name,
            params.execution_id,
        );
        let log_args = format!(
            "Executed host call {:?} {:?} [{}]",
            function_args.service_id, function_args.function_name, params.execution_id,
        );

        let start = Instant::now();
        // TODO: maybe error handling and conversion should happen here, so it is possible to log::warn errors
        #[rustfmt::skip]
        let result = match (function_args.service_id.as_str(), function_args.function_name.as_str()) {
            ("peer", "identify")              => ok(json!(self.node_info)),
            ("peer", "timestamp_ms")          => ok(json!(now_ms() as u64)),
            ("peer", "timestamp_sec")         => ok(json!(now_sec())),
            ("peer", "is_connected")          => wrap(self.is_connected(function_args)),
            ("peer", "connect")               => wrap(self.connect(function_args)),
            ("peer", "get_contact")           => wrap_opt(self.get_contact(function_args)),

            ("kad", "neighborhood")           => wrap(self.neighborhood(function_args)),
            ("kad", "merge")                  => wrap(self.kad_merge(function_args.function_args)),

            ("srv", "list")                   => ok(self.list_services()),
            ("srv", "create")                 => wrap(self.create_service(function_args, params)),
            ("srv", "get_interface")          => wrap(self.get_interface(function_args)),
            ("srv", "resolve_alias")          => wrap(self.resolve_alias(function_args)),
            ("srv", "add_alias")              => wrap_unit(self.add_alias(function_args, params)),
            ("srv", "remove")                 => wrap_unit(self.remove_service(function_args, params)),

            ("dist", "add_module")            => wrap(self.add_module(function_args)),
            ("dist", "add_blueprint")         => wrap(self.add_blueprint(function_args)),
            ("dist", "make_module_config")    => wrap(self.make_module_config(function_args)),
            ("dist", "make_blueprint")        => wrap(self.make_blueprint(function_args)),
            ("dist", "list_modules")          => wrap(self.list_modules()),
            ("dist", "get_module_interface")  => wrap(self.get_module_interface(function_args)),
            ("dist", "list_blueprints")       => wrap(self.get_blueprints()),

            ("script", "add")                 => wrap(self.add_script(function_args, params)),
            ("script", "remove")              => wrap(self.remove_script(function_args, params)),
            ("script", "list")                => wrap(self.list_scripts()),

            ("op", "noop")                    => unit(),
            ("op", "array")                   => ok(Array(function_args.function_args)),
            ("op", "array_length")            => wrap(self.array_length(function_args.function_args)),
            ("op", "concat")                  => wrap(self.concat(function_args.function_args)),
            ("op", "string_to_b58")           => wrap(self.string_to_b58(function_args.function_args)),
            ("op", "string_from_b58")         => wrap(self.string_from_b58(function_args.function_args)),
            ("op", "bytes_from_b58")          => wrap(self.bytes_from_b58(function_args.function_args)),
            ("op", "bytes_to_b58")            => wrap(self.bytes_to_b58(function_args.function_args)),
            ("op", "sha256_string")           => wrap(self.sha256_string(function_args.function_args)),
            ("op", "identity")                => wrap_opt(self.identity(function_args.function_args)),

            ("ipfs", "get_multiaddr")         => wrap(self.ipfs().get_multiaddr()),
            ("ipfs", "clear_multiaddr")       => wrap(self.ipfs().clear_multiaddr(params, &self.management_peer_id)),
            ("ipfs", "set_multiaddr")         => wrap_opt(self.ipfs().set_multiaddr(function_args, params, &self.management_peer_id)),

            _ => wrap(self.call_service(function_args, params, args.create_vault)),
        };
        log::info!("{} ({})", log_args, pretty(start.elapsed()));
        result
    }

    fn neighborhood(&self, args: Args) -> Result<JValue, JError> {
        let mut args = args.function_args.into_iter();
        let key = from_base58("key", &mut args)?;
        let already_hashed: Option<bool> = Args::next_opt("already_hashed", &mut args)?;
        let count: Option<usize> = Args::next_opt("count", &mut args)?;
        let count = count.unwrap_or_else(|| K_VALUE.get());

        let key = if already_hashed == Some(true) {
            MultihashGeneric::from_bytes(&key)?
        } else {
            Code::Sha2_256.digest(&key)
        };
        let neighbors = task::block_on(self.kademlia().neighborhood(key, count));
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
        let addrs: Vec<Multiaddr> = Args::next_opt("addresses", &mut args)?.unwrap_or_default();

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
        let mut args = args.function_args.into_iter();

        let script: String = Args::next("script", &mut args)?;

        let interval = parse_u64("interval_sec", &mut args)?;
        let interval = interval.map(Duration::from_secs);

        let delay = parse_u64("delay_sec", &mut args)?;
        let delay = delay.map(Duration::from_secs);
        let delay = get_delay(delay, interval);

        let creator = PeerId::from_str(&params.init_user_id)?;
        let id = self
            .script_storage
            .add_script(script, interval, delay, creator)?;

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
        let digest_only: Option<bool> = Args::next_opt("digest_only", &mut args)?;
        let as_bytes: Option<bool> = Args::next_opt("as_bytes", &mut args)?;
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
        let count: Option<usize> = Args::next_opt("count", &mut args)?;

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

    fn identity(&self, args: Vec<serde_json::Value>) -> Result<Option<JValue>, JError> {
        if args.len() > 1 {
            Err(JError::new(format!(
                "identity accepts up to 1 arguments, received {} arguments",
                args.len()
            )))
        } else {
            Ok(args.into_iter().next())
        }
    }

    /// Flattens an array of arrays
    fn concat(&self, args: Vec<serde_json::Value>) -> Result<JValue, JError> {
        let flattened: Vec<JValue> =
            args.into_iter()
                .enumerate()
                .try_fold(vec![], |mut acc, (i, mut v)| match v.take() {
                    JValue::Array(mut array) => {
                        acc.append(&mut array);
                        Ok(acc)
                    }
                    _ => Err(JError::new(format!(
                        "all arguments of 'concat' must be arrays: argument #{} is not",
                        i
                    ))),
                })?;

        Ok(JValue::Array(flattened))
    }

    fn array_length(&self, args: Vec<serde_json::Value>) -> Result<JValue, JError> {
        match &args[..] {
            [JValue::Array(array)] => Ok(json!(array.len())),
            [_] => Err(JError::new("op array_length's argument must be an array")),
            arr => Err(JError::new(format!(
                "op array_length accepts exactly 1 argument: {} found",
                arr.len()
            ))),
        }
    }

    fn add_module(&self, args: Args) -> Result<JValue, JError> {
        let mut args = args.function_args.into_iter();
        let module_bytes: String = Args::next("module_bytes", &mut args)?;
        let config = Args::next("config", &mut args)?;

        let module_hash = self.modules.add_module(module_bytes, config)?;

        Ok(JValue::String(module_hash))
    }

    fn add_blueprint(&self, args: Args) -> Result<JValue, JError> {
        let mut args = args.function_args.into_iter();
        let blueprint_request: AddBlueprint = Args::next("blueprint_request", &mut args)?;

        let blueprint_id = self.modules.add_blueprint(blueprint_request)?;
        Ok(JValue::String(blueprint_id))
    }

    fn make_module_config(&self, args: Args) -> Result<JValue, JError> {
        use toml_utils::table;

        let mut args = args.function_args.into_iter();

        let name = Args::next("name", &mut args)?;
        let mem_pages_count = Args::next_opt("mem_pages_count", &mut args)?;
        let logger_enabled = Args::next_opt("logger_enabled", &mut args)?;
        let preopened_files = Args::next_opt("preopened_files", &mut args)?;
        let envs = Args::next_opt("envs", &mut args)?.map(table);
        let mapped_dirs = Args::next_opt("mapped_dirs", &mut args)?.map(table);
        let mounted_binaries = Args::next_opt("mounted_binaries", &mut args)?.map(table);
        let logging_mask = Args::next_opt("logging_mask", &mut args)?;

        let config = NamedModuleConfig {
            name,
            file_name: None,
            config: ModuleConfig {
                mem_pages_count,
                logger_enabled,
                wasi: Some(WASIConfig {
                    preopened_files,
                    envs,
                    mapped_dirs,
                }),
                mounted_binaries,
                logging_mask,
            },
        };

        let config = serde_json::to_value(config)
            .map_err(|err| JError::new(format!("Error serializing config to JSON: {}", err)))?;

        Ok(config)
    }

    fn make_blueprint(&self, args: Args) -> Result<JValue, JError> {
        let mut args = args.function_args.into_iter();
        let name = Args::next("name", &mut args)?;
        let dependencies = Args::next("dependencies", &mut args)?;
        let blueprint_request = AddBlueprint { name, dependencies };

        let blueprint_request = serde_json::to_value(blueprint_request).map_err(|err| {
            JError::new(format!(
                "Error serializing blueprint_request to JSON: {}",
                err
            ))
        })?;
        Ok(blueprint_request)
    }

    fn list_modules(&self) -> Result<JValue, JError> {
        self.modules.list_modules()
    }

    fn get_module_interface(&self, args: Args) -> Result<JValue, JError> {
        let mut args = args.function_args.into_iter();
        let hash: String = Args::next("hex_hash", &mut args)?;
        self.modules.get_interface(&hash)
    }

    fn get_blueprints(&self) -> Result<JValue, JError> {
        self.modules
            .get_blueprints()
            .into_iter()
            .map(|bp| {
                serde_json::to_value(&bp).map_err(|err| {
                    JError::new(format!("error serializing blueprint {:?}: {}", bp, err))
                })
            })
            .collect()
    }

    fn create_service(&self, args: Args, params: ParticleParameters) -> Result<JValue, JError> {
        let mut args = args.function_args.into_iter();
        let blueprint_id: String = Args::next("blueprint_id", &mut args)?;

        let service_id = self
            .services
            .create_service(blueprint_id, params.init_user_id)?;

        Ok(JValue::String(service_id))
    }

    fn remove_service(&self, args: Args, params: ParticleParameters) -> Result<(), JError> {
        let mut args = args.function_args.into_iter();
        let service_id_or_alias: String = Args::next("service_id_or_alias", &mut args)?;

        self.services
            .remove_service(service_id_or_alias, params.init_user_id)?;
        Ok(())
    }

    fn list_services(&self) -> JValue {
        JValue::Array(self.services.list_services())
    }

    fn call_service(
        &self,
        function_args: Args,
        particle_parameters: ParticleParameters,
        create_vault: AVMEffect<PathBuf>,
    ) -> Result<JValue, JError> {
        Ok(self
            .services
            .call_service(particle_services::CallServiceArgs {
                function_args,
                particle_parameters,
                create_vault,
            })?)
    }

    fn get_interface(&self, args: Args) -> Result<JValue, JError> {
        let mut args = args.function_args.into_iter();
        let service_id: String = Args::next("service_id", &mut args)?;
        Ok(self.services.get_interface(service_id)?)
    }

    fn add_alias(&self, args: Args, params: ParticleParameters) -> Result<(), JError> {
        let mut args = args.function_args.into_iter();

        let alias: String = Args::next("alias", &mut args)?;
        let service_id: String = Args::next("service_id", &mut args)?;
        self.services
            .add_alias(alias, service_id, params.init_user_id)?;
        Ok(())
    }

    fn resolve_alias(&self, args: Args) -> Result<JValue, JError> {
        let mut args = args.function_args.into_iter();

        let alias: String = Args::next("alias", &mut args)?;
        let service_id = self.services.resolve_alias(alias)?;

        Ok(JValue::String(service_id))
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

fn wrap_unit(r: Result<(), JError>) -> Option<IValue> {
    match r {
        Err(e) => error(e.into()),
        _ => unit(),
    }
}

fn wrap_opt(r: Result<Option<JValue>, JError>) -> Option<IValue> {
    into_record_opt(r.map_err(Into::into))
}

fn parse_u64(
    field: &'static str,
    mut args: &mut impl Iterator<Item = JValue>,
) -> Result<Option<u64>, JError> {
    #[derive(thiserror::Error, Debug)]
    #[error("Error while deserializing field {field_name}: not a valid u64")]
    struct Error {
        field_name: String,
        #[source]
        err: ParseIntError,
    }

    #[derive(serde::Deserialize, Debug)]
    #[serde(untagged)]
    pub enum Period {
        String(String),
        Number(u64),
    }

    let number: Option<Period> = Args::next_opt(field, &mut args)?;

    if number.is_none() {
        return Ok(None);
    }

    number
        .map(|i| match i {
            Period::String(s) => Ok(s.parse::<u64>()?),
            Period::Number(n) => Ok(n),
        })
        .transpose()
        .map_err(|err| {
            Error {
                field_name: field.to_string(),
                err,
            }
            .into()
        })
}

fn get_delay(delay: Option<Duration>, interval: Option<Duration>) -> Duration {
    use rand::prelude::*;
    let mut rng = rand::thread_rng();
    match (delay, interval) {
        (Some(delay), _) => delay,
        (None, Some(interval)) => Duration::from_secs(rng.gen_range(0..=interval.as_secs())),
        (None, None) => Duration::from_secs(0),
    }
}
