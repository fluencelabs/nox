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

use std::borrow::Borrow;
use std::num::ParseIntError;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::{Duration, Instant};

use async_std::task;
use humantime_serde::re::humantime::format_duration as pretty;
use libp2p::{core::Multiaddr, kad::kbucket::Key, kad::K_VALUE, PeerId};
use multihash::{Code, MultihashDigest, MultihashGeneric};
use serde_json::{json, Value as JValue, Value};
use JValue::Array;

use avm_server::{CallRequestParams, CallServiceResult};

use connection_pool::{ConnectionPoolApi, ConnectionPoolT};
use host_closure::{from_base58, AVMEffect, Args, ArgsError, JError};
use ivalue_utils::{error, into_record, into_record_opt, unit, IValue};
use kademlia::{KademliaApi, KademliaApiT};
use now_millis::{now_ms, now_sec};
use particle_modules::{
    AddBlueprint, ModuleConfig, ModuleRepository, NamedModuleConfig, WASIConfig,
};
use particle_protocol::{Contact, Particle};
use particle_services::ParticleAppServices;
use script_storage::ScriptStorageApi;
use server_config::ServicesConfig;

use crate::error::HostClosureCallError;
use crate::error::HostClosureCallError::{DecodeBase58, DecodeUTF8};
use crate::identify::NodeInfo;
use std::convert::{TryFrom, TryInto};

#[derive(Debug, Clone)]
pub struct HostFunctions<C> {
    pub connectivity: C,
    pub script_storage: ScriptStorageApi,

    pub management_peer_id: PeerId,
    pub builtins_management_peer_id: PeerId,

    pub modules: ModuleRepository,
    pub services: ParticleAppServices,
    pub node_info: NodeInfo,
}

impl<C: Clone + Send + Sync + 'static + AsRef<KademliaApi> + AsRef<ConnectionPoolApi>>
    HostFunctions<C>
{
    pub fn new(
        connectivity: C,
        script_storage: ScriptStorageApi,
        node_info: NodeInfo,
        config: ServicesConfig,
    ) -> Self {
        let modules_dir = &config.modules_dir;
        let blueprint_dir = &config.blueprint_dir;
        let vault_dir = &config.particles_vault_dir;
        let modules = ModuleRepository::new(modules_dir, blueprint_dir, vault_dir);

        let management_peer_id = config.management_peer_id;
        let builtins_management_peer_id = config.builtins_management_peer_id;
        let services = ParticleAppServices::new(config, modules.clone());

        Self {
            connectivity,
            script_storage,
            management_peer_id,
            builtins_management_peer_id,
            modules,
            services,
            node_info,
        }
    }

    pub async fn call(
        &self,
        call_request: CallRequestParams,
        particle: Particle,
    ) -> CallServiceResult {
        let args = match Args::try_from(call_request) {
            Ok(args) => args,
            Err(err) => {
                return CallServiceResult {
                    ret_code: 1,
                    result: format!("Failed to deserialize CallRequestParams to Args: {}", err),
                }
            }
        };

        log::trace!("Host function call, args: {:#?}", args);
        let log_args = format!("{:?} {:?}", args.service_id, args.function_name);

        let start = Instant::now();
        // TODO: maybe error handling and conversion should happen here, so it is possible to log::warn errors
        #[rustfmt::skip]
        let result = match (args.service_id.as_str(), args.function_name.as_str()) {
            ("peer", "identify")              => ok(json!(self.node_info)),
            ("peer", "timestamp_ms")          => ok(json!(now_ms() as u64)),
            ("peer", "timestamp_sec")         => ok(json!(now_sec())),
            ("peer", "is_connected")          => wrap(self.is_connected(args).await),
            ("peer", "connect")               => wrap(self.connect(args).await),
            ("peer", "get_contact")           => self.get_contact(args).await,

            ("kad", "neighborhood")           => wrap(self.neighborhood(args).await),
            ("kad", "merge")                  => wrap(self.kad_merge(args.function_args)),

            ("srv", "list")                   => ok(self.list_services()),
            ("srv", "create")                 => wrap(self.create_service(args, particle)),
            ("srv", "get_interface")          => wrap(self.get_interface(args)),
            ("srv", "resolve_alias")          => wrap(self.resolve_alias(args)),
            ("srv", "add_alias")              => wrap_unit(self.add_alias(args, particle)),
            ("srv", "remove")                 => wrap_unit(self.remove_service(args, particle)),

            ("dist", "add_module_from_vault") => wrap(self.add_module_from_vault(args, particle)),
            ("dist", "add_module")            => wrap(self.add_module(args)),
            ("dist", "add_blueprint")         => wrap(self.add_blueprint(args)),
            ("dist", "make_module_config")    => wrap(self.make_module_config(args)),
            ("dist", "load_module_config")    => wrap(self.load_module_config_from_vault(args, particle)),
            ("dist", "default_module_config") => wrap(self.default_module_config(args)),
            ("dist", "make_blueprint")        => wrap(self.make_blueprint(args)),
            ("dist", "load_blueprint")        => wrap(self.load_blueprint_from_vault(args, particle)),
            ("dist", "list_modules")          => wrap(self.list_modules()),
            ("dist", "get_module_interface")  => wrap(self.get_module_interface(args)),
            ("dist", "list_blueprints")       => wrap(self.get_blueprints()),

            ("script", "add")                 => wrap(self.add_script(args, particle)),
            ("script", "remove")              => wrap(self.remove_script(args, particle).await),
            ("script", "list")                => wrap(self.list_scripts().await),

            ("op", "noop")                    => Ok(None),
            ("op", "array")                   => ok(Array(args.function_args)),
            ("op", "array_length")            => wrap(self.array_length(args.function_args)),
            ("op", "concat")                  => wrap(self.concat(args.function_args)),
            ("op", "string_to_b58")           => wrap(self.string_to_b58(args.function_args)),
            ("op", "string_from_b58")         => wrap(self.string_from_b58(args.function_args)),
            ("op", "bytes_from_b58")          => wrap(self.bytes_from_b58(args.function_args)),
            ("op", "bytes_to_b58")            => wrap(self.bytes_to_b58(args.function_args)),
            ("op", "sha256_string")           => wrap(self.sha256_string(args.function_args)),
            ("op", "concat_strings")          => wrap(self.concat_strings(args.function_args)),
            ("op", "identity")                => self.identity(args.function_args),

            _ => self.call_service(args, particle, todo!("create vault isn't implemented")).map(Some),
        };
        let elapsed = pretty(start.elapsed());
        if let Err(err) = &result {
            log::warn!("Failed host call {} ({}): {}", log_args, elapsed, err)
        } else {
            log::info!("Executed host call {} ({})", log_args, elapsed);
        };

        match result {
            Ok(v) => CallServiceResult {
                ret_code: 0,
                result: v.map_or(json!([]), |v| json!([v])).to_string(),
            },
            Err(e) => CallServiceResult {
                ret_code: 1,
                result: json!([JValue::from(e)]).to_string(),
            },
        }
    }

    async fn neighborhood(&self, args: Args) -> Result<JValue, JError> {
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
        let neighbors = self.kademlia().neighborhood(key, count).await;
        let neighbors = neighbors
            .map(|vs| json!(vs.into_iter().map(|id| id.to_string()).collect::<Vec<_>>()))?;

        Ok(neighbors)
    }

    async fn is_connected(&self, args: Args) -> Result<JValue, JError> {
        let peer: String = Args::next("peer_id", &mut args.function_args.into_iter())?;
        let peer = PeerId::from_str(peer.as_str())?;
        let ok = self.connection_pool().is_connected(peer).await;
        Ok(json!(ok))
    }

    async fn connect(&self, args: Args) -> Result<JValue, JError> {
        let mut args = args.function_args.into_iter();

        let peer_id: String = Args::next("peer_id", &mut args)?;
        let peer_id = PeerId::from_str(peer_id.as_str())?;
        let addrs: Vec<Multiaddr> = Args::next_opt("addresses", &mut args)?.unwrap_or_default();

        let contact = Contact::new(peer_id, addrs);

        let ok = self.connection_pool().connect(contact).await;
        Ok(json!(ok))
    }

    async fn get_contact(&self, args: Args) -> Result<Option<JValue>, JError> {
        let peer: String = Args::next("peer_id", &mut args.function_args.into_iter())?;
        let peer = PeerId::from_str(peer.as_str())?;
        let contact = self.connection_pool().get_contact(peer).await;
        Ok(contact.map(|c| json!(c)))
    }

    fn add_script(&self, args: Args, params: Particle) -> Result<JValue, JError> {
        let mut args = args.function_args.into_iter();

        let script: String = Args::next("script", &mut args)?;

        let interval = parse_u64("interval_sec", &mut args)?;
        let interval = interval.map(Duration::from_secs);

        let delay = parse_u64("delay_sec", &mut args)?;
        let delay = delay.map(Duration::from_secs);
        let delay = get_delay(delay, interval);

        let creator = params.init_peer_id;
        let id = self
            .script_storage
            .add_script(script, interval, delay, creator)?;

        Ok(json!(id))
    }

    async fn remove_script(&self, args: Args, params: Particle) -> Result<JValue, JError> {
        let mut args = args.function_args.into_iter();

        let force = params.init_peer_id == self.management_peer_id;

        let uuid: String = Args::next("uuid", &mut args)?;
        let actor = params.init_peer_id;

        let ok = self
            .script_storage
            .remove_script(uuid, actor, force)
            .await?;

        Ok(json!(ok))
    }

    async fn list_scripts(&self) -> Result<JValue, JError> {
        let scripts = self.script_storage.list_scripts().await?;

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
        let count = count.unwrap_or_else(|| K_VALUE.get());

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
        keys.dedup();

        let keys = keys
            .into_iter()
            .map(|k| bs58::encode(k.into_preimage()).into_string());

        let keys: Vec<_> = keys.take(count).collect();

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
                .try_fold(vec![], |mut acc, (i, v)| match v {
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

    /// Concatenates an array of arrays
    fn concat_strings(&self, args: Vec<serde_json::Value>) -> Result<JValue, JError> {
        let string: String =
            args.into_iter()
                .enumerate()
                .try_fold(String::new(), |mut acc, (i, v)| match v {
                    JValue::String(s) => {
                        acc.push_str(&s);
                        Ok(acc)
                    }
                    _ => Err(JError::new(format!(
                        "all arguments of 'concat_strings' must be strings: argument #{} is not",
                        i
                    ))),
                })?;

        Ok(JValue::String(string))
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

        let module_hash = self.modules.add_module_base64(module_bytes, config)?;

        Ok(JValue::String(module_hash))
    }

    fn add_module_from_vault(&self, args: Args, params: Particle) -> Result<JValue, JError> {
        let mut args = args.function_args.into_iter();
        let module_path: String = Args::next("module_path", &mut args)?;
        let config = Args::next("config", &mut args)?;

        let module_hash = self
            .modules
            .add_module_from_vault(module_path, config, params)?;

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

    fn load_module_config_from_vault(
        &self,
        args: Args,
        params: Particle,
    ) -> Result<JValue, JError> {
        let mut args = args.function_args.into_iter();
        let config_path: String = Args::next("config_path", &mut args)?;

        let config = self
            .modules
            .load_module_config_from_vault(config_path, params)?;
        let config = serde_json::to_value(config)
            .map_err(|err| JError::new(format!("Error serializing config to JSON: {}", err)))?;

        Ok(config)
    }

    fn default_module_config(&self, args: Args) -> Result<JValue, JError> {
        let mut args = args.function_args.into_iter();
        let module_name: String = Args::next("module_name", &mut args)?;

        let config = NamedModuleConfig {
            name: module_name,
            file_name: None,
            config: <_>::default(),
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

    fn load_blueprint_from_vault(&self, args: Args, params: Particle) -> Result<JValue, JError> {
        let mut args = args.function_args.into_iter();
        let blueprint_path = Args::next("blueprint_path", &mut args)?;

        let blueprint_request = self
            .modules
            .load_blueprint_from_vault(blueprint_path, params)?;

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

    fn create_service(&self, args: Args, params: Particle) -> Result<JValue, JError> {
        let mut args = args.function_args.into_iter();
        let blueprint_id: String = Args::next("blueprint_id", &mut args)?;

        let service_id = self
            .services
            .create_service(blueprint_id, params.init_peer_id)?;

        Ok(JValue::String(service_id))
    }

    fn remove_service(&self, args: Args, params: Particle) -> Result<(), JError> {
        let mut args = args.function_args.into_iter();
        let service_id_or_alias: String = Args::next("service_id_or_alias", &mut args)?;

        self.services
            .remove_service(service_id_or_alias, params.init_peer_id)?;
        Ok(())
    }

    fn list_services(&self) -> JValue {
        JValue::Array(self.services.list_services())
    }

    fn call_service(
        &self,
        function_args: Args,
        particle: Particle,
        create_vault: AVMEffect<PathBuf>,
    ) -> Result<JValue, JError> {
        Ok(self
            .services
            .call_service(particle_services::CallServiceArgs {
                function_args,
                particle,
                create_vault,
            })?)
    }

    fn get_interface(&self, args: Args) -> Result<JValue, JError> {
        let mut args = args.function_args.into_iter();
        let service_id: String = Args::next("service_id", &mut args)?;
        Ok(self.services.get_interface(service_id)?)
    }

    fn add_alias(&self, args: Args, params: Particle) -> Result<(), JError> {
        let mut args = args.function_args.into_iter();

        let alias: String = Args::next("alias", &mut args)?;
        let service_id: String = Args::next("service_id", &mut args)?;
        self.services
            .add_alias(alias, service_id, params.init_peer_id)?;
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
}

fn ok(v: JValue) -> Result<Option<JValue>, JError> {
    Ok(Some(v))
}

fn wrap(r: Result<JValue, JError>) -> Result<Option<JValue>, JError> {
    r.map(Some)
}

fn wrap_unit(r: Result<(), JError>) -> Result<Option<JValue>, JError> {
    r.map(|_| Some(json!({})))
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
