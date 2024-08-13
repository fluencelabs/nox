/*
 * Nox Fluence Peer
 *
 * Copyright (C) 2024 Fluence DAO
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation version 3 of the
 * License.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::ops::Try;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use derivative::Derivative;
use fluence_app_service::TomlMarineNamedModuleConfig;
use fluence_keypair::Signature;
use libp2p::core::Multiaddr;
use libp2p::kad::KBucketKey;
use libp2p::kad::K_VALUE;
use libp2p::PeerId;
use multihash::Multihash;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JValue, Value};
use tokio::sync::RwLock;
use JValue::Array;

use connection_pool::ConnectionPoolApi;
use connection_pool::ConnectionPoolT;
use fs_utils::create_dirs;
use fs_utils::to_abs_path;
use health::HealthCheckRegistry;
use kademlia::KademliaApi;
use kademlia::KademliaApiT;
use now_millis::now_ms;
use now_millis::now_sec;
use particle_args::from_base58;
use particle_args::Args;
use particle_args::ArgsError;
use particle_args::JError;
use particle_execution::FunctionOutcome;
use particle_execution::ParticleParams;
use particle_execution::ServiceFunction;
use particle_modules::AddBlueprint;
use particle_modules::EffectorsMode;
use particle_modules::ModuleConfig;
use particle_modules::ModuleRepository;
use particle_modules::NamedModuleConfig;
use particle_modules::WASIConfig;
use particle_protocol::Contact;
use particle_services::ParticleAppServices;
use particle_services::ParticleAppServicesConfig;
use particle_services::PeerScope;
use particle_services::ServiceInfo;
use particle_services::ServiceType;
use peer_metrics::ServicesMetrics;
use service_modules::Hash;
use types::peer_id;
use types::peer_scope::WorkerId;
use uuid_utils::uuid;
use workers::KeyStorage;
use workers::PeerScopes;
use workers::Workers;

use crate::debug::fmt_custom_services;
use crate::error::HostClosureCallError;
use crate::error::HostClosureCallError::DecodeBase58;
use crate::error::HostClosureCallError::DecodeUTF8;
use crate::func::binary;
use crate::func::unary;
use crate::json;
use crate::math;
use crate::outcome::ok;
use crate::outcome::wrap;
use crate::outcome::wrap_unit;

pub struct CustomService {
    /// (function_name -> service function)
    pub functions: HashMap<String, ServiceFunction>,
    /// if set, all `function_name` mismatches with `custom_service.functions` will be routed to `fallback`
    pub fallback: Option<ServiceFunction>,
}

impl CustomService {
    pub fn new(funcs: Vec<(&str, ServiceFunction)>, fallback: Option<ServiceFunction>) -> Self {
        Self {
            functions: funcs
                .into_iter()
                .map(|(name, f)| (name.to_string(), f))
                .collect(),
            fallback,
        }
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Builtins<C> {
    pub connectivity: C,
    pub modules: ModuleRepository,
    pub services: ParticleAppServices,
    #[derivative(Debug(format_with = "fmt_custom_services"))]
    pub custom_services: RwLock<HashMap<String, CustomService>>,
    #[derivative(Debug = "ignore")]
    workers: Arc<Workers>,
    #[derivative(Debug = "ignore")]
    key_storage: Arc<KeyStorage>,
    #[derivative(Debug = "ignore")]
    scopes: PeerScopes,
    config: BuiltinsConfig,
}

#[derive(Debug)]
pub struct BuiltinsConfig {
    pub particle_app_services: ParticleAppServicesConfig,
    /// Dir to store .wasm modules and their configs
    pub modules_dir: PathBuf,
    /// Path of the blueprint directory containing blueprints and wasm modules
    pub blueprint_dir: PathBuf,
    /// Is in the developer mode
    pub is_dev_mode: bool,
    /// Mapping of binary names to their paths for mounted binaries used in developer mode
    pub mounted_binaries_mapping: HashMap<String, PathBuf>,
    /// List of allowed effector modules by CID
    pub allowed_effectors: HashMap<Hash, HashMap<String, PathBuf>>,
}

impl BuiltinsConfig {
    pub fn new(
        particle_app_services: ParticleAppServicesConfig,
        persistent_dir: PathBuf,
        allowed_effectors: HashMap<Hash, HashMap<String, PathBuf>>,
        mounted_binaries_mapping: HashMap<String, PathBuf>,
        is_dev_mode: bool,
    ) -> Result<Self, std::io::Error> {
        let persistent_dir = to_abs_path(persistent_dir);

        let blueprint_dir = config_utils::blueprint_dir(&persistent_dir);
        let modules_dir = config_utils::modules_dir(&persistent_dir);

        create_dirs(&[&blueprint_dir, &modules_dir])?;

        let allowed_effectors = allowed_effectors
            .into_iter()
            .map(|(cid, effector)| {
                let effector = effector
                    .into_iter()
                    .map(|(name, path)| {
                        let path_str = path.display();
                        match path.try_exists() {
                            Err(err) => tracing::warn!(
                                "cannot check binary `{path_str}` for effector `{cid}`: {err}"
                            ),
                            Ok(false) => tracing::warn!(
                                "binary `{path_str}` for effector `{cid}` does not exist"
                            ),
                            _ => {}
                        };
                        (name, path.to_path_buf())
                    })
                    .collect::<_>();
                (cid, effector)
            })
            .collect::<_>();

        let mounted_binaries_mapping = if !is_dev_mode {
            HashMap::new()
        } else {
            mounted_binaries_mapping
                .into_iter()
                .map(|(name, path)| {
                    let path_str = path.display();
                    match path.try_exists() {
                        Err(err) => tracing::warn!("cannot check binary `{path_str}`: {err}"),
                        Ok(false) => tracing::warn!("binary `{path_str}` does not exist"),
                        _ => {}
                    };
                    (name, path.to_path_buf())
                })
                .collect::<_>()
        };
        Ok(Self {
            particle_app_services,
            blueprint_dir,
            modules_dir,
            allowed_effectors,
            mounted_binaries_mapping,
            is_dev_mode,
        })
    }
}

impl<C> Builtins<C>
where
    C: Clone + Send + Sync + 'static + AsRef<KademliaApi> + AsRef<ConnectionPoolApi>,
{
    pub fn new(
        config: BuiltinsConfig,
        connectivity: C,
        services_metrics: ServicesMetrics,
        key_storage: Arc<KeyStorage>,
        workers: Arc<Workers>,
        scope: PeerScopes,
        health_registry: Option<&mut HealthCheckRegistry>,
    ) -> Self {
        let effectors_mode = if config.is_dev_mode {
            EffectorsMode::all_effectors(
                config.allowed_effectors.clone(),
                config.mounted_binaries_mapping.clone(),
            )
        } else {
            EffectorsMode::restricted_effectors(config.allowed_effectors.clone())
        };
        let modules_dir = &config.modules_dir;
        let blueprint_dir = &config.blueprint_dir;

        let modules = ModuleRepository::new(modules_dir, blueprint_dir, effectors_mode);
        let services = ParticleAppServices::new(
            config.particle_app_services.clone(),
            modules.clone(),
            Some(services_metrics),
            health_registry,
            workers.clone(),
            scope.clone(),
        )
        .expect("TODO async-marine: handle error from ParticleAppServices");

        Self {
            connectivity,
            modules,
            services,
            workers,
            custom_services: <_>::default(),
            key_storage,
            scopes: scope,
            config,
        }
    }

    pub async fn call(&self, args: Args, particle: ParticleParams) -> FunctionOutcome {
        let mut start = Instant::now();
        let result = self.builtins_call(args, particle).await;
        let result = match result {
            FunctionOutcome::NotDefined { args, params } => {
                start = Instant::now();
                self.custom_service_call(args, params).await
            }
            result => result,
        };
        let end = start.elapsed().as_secs();

        match result {
            FunctionOutcome::NotDefined { args, params } => self.call_service(args, params).await,
            result => {
                if let Some(metrics) = self.services.metrics.as_ref() {
                    metrics.observe_builtins(result.not_err(), end as f64);
                }
                result
            }
        }
    }

    pub async fn custom_service_call(
        &self,
        args: Args,
        particle: ParticleParams,
    ) -> FunctionOutcome {
        if let Some(function) = self
            .custom_services
            .read()
            .await
            .get(&args.service_id)
            .and_then(|fs| {
                fs.functions
                    .get(&args.function_name)
                    .or(fs.fallback.as_ref())
            })
        {
            function.call(args, particle).await
        } else {
            FunctionOutcome::NotDefined {
                args,
                params: particle,
            }
        }
    }

    // TODO: get rid of all blocking methods (std::fs and such)
    pub async fn builtins_call(&self, args: Args, particle: ParticleParams) -> FunctionOutcome {
        use Result as R;

        #[rustfmt::skip]
        match (args.service_id.as_str(), args.function_name.as_str()) {
            ("peer", "timestamp_ms") => ok(json!(now_ms() as u64)),
            ("peer", "timestamp_sec") => ok(json!(now_sec())),
            ("peer", "is_connected") => wrap(self.is_connected(args).await),
            ("peer", "connect") => wrap(self.connect(args, particle).await),
            ("peer", "get_contact") => self.get_contact(args).await,
            ("peer", "timeout") => self.timeout(args).await,

            ("kad", "neighborhood") => wrap(self.neighborhood(args).await),
            ("kad", "neigh_with_addrs") => wrap(self.neighborhood_with_addresses(args).await),
            ("kad", "merge") => wrap(self.kad_merge(args.function_args)),

            ("srv", "list") => ok(self.list_services(particle).await),
            ("srv", "create") => wrap(self.create_service(args, particle).await),
            ("srv", "get_interface") => wrap(self.get_interface(args, particle).await),
            ("srv", "resolve_alias") => wrap(self.resolve_alias(args, particle).await),
            ("srv", "resolve_alias_opt") => wrap(self.resolve_alias_opt(args, particle).await),
            ("srv", "add_alias") => wrap_unit(self.add_alias(args, particle).await),
            ("srv", "remove") => wrap_unit(self.remove_service(args, particle).await),
            ("srv", "info") => wrap(self.get_service_info(args, particle).await),

            ("dist", "add_module_from_vault") => wrap(self.add_module_from_vault(args, particle).await),
            ("dist", "add_module") => wrap(self.add_module(args, particle).await),
            ("dist", "add_module_bytes_from_vault") => wrap(self.add_module_bytes_from_vault(args, particle).await),
            ("dist", "add_blueprint") => wrap(self.add_blueprint(args, particle).await),
            ("dist", "make_module_config") => wrap(make_module_config(args)),
            ("dist", "load_module_config") => wrap(self.load_module_config_from_vault(args, particle)),
            ("dist", "default_module_config") => wrap(self.default_module_config(args)),
            ("dist", "make_blueprint") => wrap(self.make_blueprint(args)),
            ("dist", "load_blueprint") => wrap(self.load_blueprint_from_vault(args, particle)),
            ("dist", "list_modules") => wrap(self.list_modules()),
            ("dist", "get_module_interface") => wrap(self.get_module_interface(args)),
            ("dist", "list_blueprints") => wrap(self.get_blueprints()),
            ("dist", "get_blueprint") => wrap(self.get_blueprint(args)),

            ("op", "noop") => FunctionOutcome::Empty,
            ("op", "array") => ok(Array(args.function_args)),
            ("op", "array_length") => wrap(self.array_length(args.function_args)),
            ("op", "concat") => wrap(self.concat(args.function_args)),
            ("op", "string_to_b58") => wrap(self.string_to_b58(args.function_args)),
            ("op", "string_from_b58") => wrap(self.string_from_b58(args.function_args)),
            ("op", "bytes_from_b58") => wrap(self.bytes_from_b58(args.function_args)),
            ("op", "bytes_to_b58") => wrap(self.bytes_to_b58(args.function_args)),
            ("op", "sha256_string") => wrap(self.sha256_string(args.function_args)),
            ("op", "concat_strings") => wrap(self.concat_strings(args.function_args)),
            ("op", "identity") => self.identity(args.function_args),

            ("debug", "stringify") => self.stringify(args.function_args),

            ("stat", "service_memory") => wrap(self.service_mem_stats(args, particle).await),
            ("stat", "service_stat") => wrap(self.service_stat(args, particle).await),

            ("math", "add") => binary(args, |x: i64, y: i64| -> R<i64, _> { math::add(x, y) }),
            ("math", "sub") => binary(args, |x: i64, y: i64| -> R<i64, _> { math::sub(x, y) }),
            ("math", "mul") => binary(args, |x: i64, y: i64| -> R<i64, _> { math::mul(x, y) }),
            ("math", "fmul") => binary(args, |x: f64, y: f64| -> R<i64, _> { math::fmul_floor(x, y) }),
            ("math", "div") => binary(args, |x: i64, y: i64| -> R<i64, _> { math::div(x, y) }),
            ("math", "rem") => binary(args, |x: i64, y: i64| -> R<i64, _> { math::rem(x, y) }),
            ("math", "pow") => binary(args, |x: i64, y: u32| -> R<i64, _> { math::pow(x, y) }),
            ("math", "log") => binary(args, |x: i64, y: i64| -> R<u32, _> { math::log(x, y) }),

            ("cmp", "gt") => binary(args, |x: i64, y: i64| -> R<bool, _> { math::gt(x, y) }),
            ("cmp", "gte") => binary(args, |x: i64, y: i64| -> R<bool, _> { math::gte(x, y) }),
            ("cmp", "lt") => binary(args, |x: i64, y: i64| -> R<bool, _> { math::lt(x, y) }),
            ("cmp", "lte") => binary(args, |x: i64, y: i64| -> R<bool, _> { math::lte(x, y) }),
            ("cmp", "cmp") => binary(args, |x: i64, y: i64| -> R<i8, _> { math::cmp(x, y) }),

            ("array", "sum") => unary(args, |xs: Vec<i64>| -> R<i64, _> { math::array_sum(xs) }),
            ("array", "dedup") => unary(args, |xs: Vec<String>| -> R<Vec<String>, _> { math::dedup(xs) }),
            ("array", "intersect") => binary(args, |xs: HashSet<String>, ys: HashSet<String>| -> R<Vec<String>, _> { math::intersect(xs, ys) }),
            ("array", "diff") => binary(args, |xs: HashSet<String>, ys: HashSet<String>| -> R<Vec<String>, _> { math::diff(xs, ys) }),
            ("array", "sdiff") => binary(args, |xs: HashSet<String>, ys: HashSet<String>| -> R<Vec<String>, _> { math::sdiff(xs, ys) }),
            ("array", "slice") => wrap(self.array_slice(args.function_args)),
            ("array", "length") => wrap(self.array_length(args.function_args)),

            ("sig", "sign") => wrap(self.sign(args, particle)),
            ("sig", "verify") => wrap(self.verify(args, particle)),
            ("sig", "get_peer_id") => wrap(self.get_peer_id(particle)),

            ("json", "obj") => wrap(json::obj(args)),
            ("json", "put") => wrap(json::put(args)),
            ("json", "puts") => wrap(json::puts(args)),
            ("json", "parse") => unary(args, |s: String| -> R<JValue, _> { json::parse(&s) }),
            ("json", "stringify") => unary(args, |v: JValue| -> R<String, _> { Ok(json::stringify(v)) }),
            ("json", "obj_pairs") => unary(args, |vs: Vec<(String, JValue)>| -> R<JValue, _> { json::obj_from_pairs(vs) }),
            ("json", "puts_pairs") => binary(args, |obj: JValue, vs: Vec<(String, JValue)>| -> R<JValue, _> { json::puts_from_pairs(obj, vs) }),

            ("vault", "put") => wrap(self.vault_put(args, particle)),
            ("vault", "cat") => wrap(self.vault_cat(args, particle)),

            ("vm", "create") => wrap(self.create_vm(args, particle).await),
            ("vm", "reboot") => wrap(self.reboot_vm(args, particle)),
            ("vm", "reset") => wrap(self.reset_vm(args, particle)),
            ("vm", "stop") => wrap(self.stop_vm(args, particle)),
            ("vm", "start") => wrap(self.start_vm(args, particle)),
            ("vm", "status") => wrap(self.status_vm(args, particle)),

            ("run-console", "print") => {
                self.guard_protected(&particle).await?;

                let function_args = args.function_args.iter();
                let decider = function_args.filter_map(JValue::as_str).any(|s| s.contains("decider"));
                if decider {
                    // if log comes from decider, log it as INFO
                    log::info!(target: "run-console", "{}", json!(args.function_args));
                } else {
                    // log everything else as DEBUG
                    log::debug!(target: "run-console", "{}", json!(args.function_args));
                }
                wrap_unit(Ok(()))
            }
            // NOTE: must be the same as in ServiceKey::AquaIpfs
            // TODO: come up with some better way of restricting service access like aqua-ipfs
            ("aqua-ipfs", _) => {
                // If the call is on Host, we check that the caller is a host, a manager or
                // a worker spell. Otherwise we allow the call to go and find an aqua-ipfs service
                // since it can be a user-defined service which isn't the same as system aqua-ipfs.
                // Allow anything when in the Dev Mode
                if !self.config.is_dev_mode && matches!(particle.peer_scope, PeerScope::Host) {
                    self.guard_protected(&particle).await?;
                }
                FunctionOutcome::NotDefined { args, params: particle }
            }
            _ => FunctionOutcome::NotDefined { args, params: particle },
        }
    }

    async fn neighbor_peers(&self, args: Args) -> Result<Vec<PeerId>, JError> {
        let mut args = args.function_args.into_iter();
        let key = from_base58("key", &mut args)?;
        let already_hashed: Option<bool> = Args::next_opt("already_hashed", &mut args)?;
        let count: Option<usize> = Args::next_opt("count", &mut args)?;
        let count = count.unwrap_or_else(|| K_VALUE.get());

        let key = if already_hashed == Some(true) {
            Multihash::from_bytes(&key)?
        } else {
            Multihash::wrap(0x12, &key[..])?
        };
        let neighbors = self.kademlia().neighborhood(key, count).await?;

        Ok(neighbors)
    }

    async fn neighborhood(&self, args: Args) -> Result<JValue, JError> {
        let neighbors = self.neighbor_peers(args).await?;
        let neighbors = json!(neighbors
            .into_iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>());

        Ok(neighbors)
    }

    async fn neighborhood_with_addresses(&self, args: Args) -> Result<JValue, JError> {
        use futures::stream::FuturesUnordered;
        use futures::StreamExt;

        let neighbors = self.neighbor_peers(args).await?;
        let neighbors = neighbors
            .into_iter()
            .map(|peer| async move {
                let contact = self.connection_pool().get_contact(peer).await;
                (peer, contact)
            })
            .collect::<FuturesUnordered<_>>()
            .map(|(peer_id, contact)| {
                json!({
                    "peer_id": peer_id.to_string(),
                    "addresses": contact.map(|c| c.addresses).unwrap_or_default()
                })
            })
            .collect::<Vec<_>>()
            .await;
        let neighbors = json!(neighbors);

        Ok(neighbors)
    }

    async fn is_connected(&self, args: Args) -> Result<JValue, JError> {
        let peer: String = Args::next("peer_id", &mut args.function_args.into_iter())?;
        let peer = PeerId::from_str(peer.as_str())?;
        let ok = self.connection_pool().is_connected(peer).await;
        Ok(json!(ok))
    }

    async fn connect(&self, args: Args, params: ParticleParams) -> Result<JValue, JError> {
        let mut args = args.function_args.into_iter();

        let peer_id: String = Args::next("peer_id", &mut args)?;
        let peer_id = PeerId::from_str(peer_id.as_str())?;
        let addrs: Vec<Multiaddr> = Args::next_opt("addresses", &mut args)?.unwrap_or_default();

        self.guard_protected(&params).await?;

        let contact = Contact::new(peer_id, addrs);

        let ok = self.connection_pool().connect(contact).await;
        Ok(json!(ok))
    }

    async fn get_contact(&self, args: Args) -> FunctionOutcome {
        let peer: String = Args::next("peer_id", &mut args.function_args.into_iter())?;
        let peer = PeerId::from_str(peer.as_str())?;
        let contact = self.connection_pool().get_contact(peer).await;
        match contact {
            Some(c) => FunctionOutcome::Ok(json!(c)),
            None => FunctionOutcome::Empty,
        }
    }

    async fn timeout(&self, args: Args) -> FunctionOutcome {
        use std::future::pending;

        let mut args = args.function_args.into_iter();

        let dur_field = "duration_ms";
        let duration = parse_from_str(dur_field, &mut args)?;
        let duration = duration.ok_or(ArgsError::MissingField(dur_field))?;
        let duration = Duration::from_millis(duration);

        let message = Args::next_opt("message", &mut args)?;

        // sleep for `duration`
        tokio::time::timeout(duration, pending::<()>()).await.ok();

        message
            .map(|msg: String| FunctionOutcome::Ok(msg.into()))
            .unwrap_or(FunctionOutcome::Empty)
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
        let multihash: Multihash<64> = Multihash::wrap(0x12, string.as_bytes())?;

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
        let target = KBucketKey::from(target);
        let left = left.into_iter();
        let right = right.into_iter();

        let mut keys: Vec<KBucketKey<_>> = left
            .chain(right)
            .map(|b58_str| {
                Ok(KBucketKey::from(
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

    fn identity(&self, args: Vec<serde_json::Value>) -> FunctionOutcome {
        if args.len() > 1 {
            FunctionOutcome::Err(JError::new(format!(
                "identity accepts up to 1 arguments, received {} arguments",
                args.len()
            )))
        } else {
            Try::from_output(args.into_iter().next())
        }
    }

    fn stringify(&self, args: Vec<serde_json::Value>) -> FunctionOutcome {
        let debug = if args.is_empty() {
            // return valid JSON string
            r#""<empty argument list>""#.to_string()
        } else if args.len() == 1 {
            args[0].to_string()
        } else {
            JValue::Array(args).to_string()
        };

        FunctionOutcome::Ok(JValue::String(debug))
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
                        "all arguments of 'concat' must be arrays: argument #{i} is not"
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
                        "all arguments of 'concat_strings' must be strings: argument #{i} is not"
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

    /// takes a range of values from an array
    /// slice(array: []JValue, start: usize, end: usize) -> []JValue
    fn array_slice(&self, args: Vec<serde_json::Value>) -> Result<JValue, JError> {
        let (array, start, end) = if let [array, start, end] = &args[..] {
            (array, start, end)
        } else {
            return Err(JError::new(
                "invalid number of parameters. need array, start index and end index",
            ));
        };

        let array = match array {
            JValue::Array(arr) if arr.is_empty() => return Ok(json!([])),
            JValue::Array(arr) => arr,
            e => {
                return Err(JError::new(format!(
                    "first argument must be an array, was {e}"
                )));
            }
        };

        let start = match start.as_u64() {
            Some(n) => n as usize,
            _ => {
                return Err(JError::new(format!(
                    "second argument (start index) must be an unsigned integer, was {start}"
                )));
            }
        };

        let end = match end.as_u64() {
            Some(n) => n as usize,
            _ => {
                return Err(JError::new(format!(
                    "third argument (end index) must be an unsigned integer, was {end}"
                )));
            }
        };

        if start > end || end > array.len() {
            return Err(JError::new(format!(
                "slice indexes out of bounds. start index: {:?}, end index: {:?}, array length: {:?}",
                start, end, array.len())
            ));
        }

        let slice: Vec<JValue> = array[start..end].to_vec();
        Ok(JValue::Array(slice))
    }

    async fn add_module(&self, args: Args, params: ParticleParams) -> Result<JValue, JError> {
        let mut args = args.function_args.into_iter();
        let module_bytes: String = Args::next("module_bytes", &mut args)?;
        let config = Args::next("config", &mut args)?;

        self.guard_protected(&params).await?;
        let hash = self.modules.add_module_base64(module_bytes, config)?;

        Ok(json!(hash))
    }

    async fn add_module_from_vault(
        &self,
        args: Args,
        params: ParticleParams,
    ) -> Result<JValue, JError> {
        let mut args = args.function_args.into_iter();
        let module_path: String = Args::next("module_path", &mut args)?;
        let config: TomlMarineNamedModuleConfig = Args::next("config", &mut args)?;

        self.guard_protected(&params).await?;

        let module_hash = self.modules.add_module_from_vault(
            &self.services.vault,
            self.scopes.to_peer_id(params.peer_scope),
            config.name,
            module_path,
            params,
        )?;

        Ok(json!(module_hash))
    }

    async fn add_module_bytes_from_vault(
        &self,
        args: Args,
        params: ParticleParams,
    ) -> Result<Value, JError> {
        let mut args = args.function_args.into_iter();
        let module_name: String = Args::next("module_name", &mut args)?;
        let module_path: String = Args::next("module_path", &mut args)?;

        self.guard_protected(&params).await?;

        let module_hash = self.modules.add_module_from_vault(
            &self.services.vault,
            self.scopes.to_peer_id(params.peer_scope),
            module_name,
            module_path,
            params,
        )?;

        Ok(json!(module_hash))
    }

    async fn add_blueprint(&self, args: Args, params: ParticleParams) -> Result<JValue, JError> {
        let mut args = args.function_args.into_iter();
        let blueprint: String = Args::next("blueprint", &mut args)?;

        self.guard_protected(&params).await?;

        let blueprint = AddBlueprint::decode(blueprint.as_bytes()).map_err(|err| {
            JError::new(format!("Error deserializing blueprint from IPLD: {err}"))
        })?;
        let blueprint_id = self.modules.add_blueprint(blueprint)?;
        Ok(JValue::String(blueprint_id))
    }

    fn load_module_config_from_vault(
        &self,
        args: Args,
        params: ParticleParams,
    ) -> Result<JValue, JError> {
        let mut args = args.function_args.into_iter();
        let config_path: String = Args::next("config_path", &mut args)?;

        let config = ModuleRepository::load_module_config_from_vault(
            &self.services.vault,
            self.scopes.to_peer_id(params.peer_scope),
            config_path,
            params,
        )?;
        let config = serde_json::to_value(config)
            .map_err(|err| JError::new(format!("Error serializing config to JSON: {err}")))?;

        Ok(config)
    }
    fn default_module_config(&self, args: Args) -> Result<JValue, JError> {
        let mut args = args.function_args.into_iter();
        let module_name: String = Args::next("module_name", &mut args)?;

        let config = NamedModuleConfig {
            name: module_name,
            load_from: None,
            file_name: None,
            config: <_>::default(),
        };
        let config = serde_json::to_value(config)
            .map_err(|err| JError::new(format!("Error serializing config to JSON: {err}")))?;

        Ok(config)
    }

    fn make_blueprint(&self, args: Args) -> Result<JValue, JError> {
        let mut args = args.function_args.into_iter();
        let name = Args::next("name", &mut args)?;
        let dependencies = Args::next("dependencies", &mut args)?;
        let blueprint = AddBlueprint { name, dependencies };

        let blueprint = blueprint
            .to_string()
            .map_err(|err| JError::new(format!("Error serializing blueprint to IPLD: {err}")))?;
        Ok(json!(blueprint))
    }

    fn load_blueprint_from_vault(
        &self,
        args: Args,
        params: ParticleParams,
    ) -> Result<JValue, JError> {
        let mut args = args.function_args.into_iter();
        let blueprint_path: String = Args::next("blueprint_path", &mut args)?;

        let current_peer_id = self.scopes.to_peer_id(params.peer_scope);
        let data =
            self.services
                .vault
                .cat_slice(current_peer_id, &params, Path::new(&blueprint_path))?;
        let blueprint = AddBlueprint::decode(&data).map_err(|err| {
            JError::new(format!(
                "Error parsing blueprint from vault {blueprint_path:?}: {err}"
            ))
        })?;

        let blueprint = blueprint
            .to_string()
            .map_err(|err| JError::new(format!("Error serializing blueprint to IPLD: {err}")))?;
        Ok(json!(blueprint))
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
                    JError::new(format!("error serializing blueprint {bp:?}: {err}"))
                })
            })
            .collect()
    }

    fn get_blueprint(&self, args: Args) -> Result<JValue, JError> {
        let mut args = args.function_args.into_iter();
        let blueprint_id: String = Args::next("blueprint_id", &mut args)?;

        let blueprint = self.modules.get_blueprint_from_cache(&blueprint_id)?;

        Ok(json!(blueprint))
    }

    async fn create_service(&self, args: Args, params: ParticleParams) -> Result<JValue, JError> {
        let mut args = args.function_args.into_iter();
        let blueprint_id: String = Args::next("blueprint_id", &mut args)?;

        self.guard_protected(&params).await?;

        let service_id = self
            .services
            .create_service(
                params.peer_scope,
                ServiceType::Service,
                blueprint_id,
                params.init_peer_id,
            )
            .await?;

        Ok(JValue::String(service_id))
    }

    async fn remove_service(&self, args: Args, params: ParticleParams) -> Result<(), JError> {
        let mut args = args.function_args.into_iter();
        let service_id_or_alias: String = Args::next("service_id_or_alias", &mut args)?;

        self.guard_protected(&params).await?;

        self.services
            .remove_service(
                params.peer_scope,
                &params.id,
                &service_id_or_alias,
                params.init_peer_id,
                false,
            )
            .await?;

        Ok(())
    }

    async fn list_services(&self, params: ParticleParams) -> JValue {
        Array(
            self.services
                .list_services(params.peer_scope)
                .await
                .iter()
                .map(|info| json!(Service::from(info, self.scopes.clone())))
                .collect(),
        )
    }

    async fn call_service(&self, function_args: Args, particle: ParticleParams) -> FunctionOutcome {
        self.services
            .call_service(function_args, particle, true)
            .await
    }

    async fn get_interface(&self, args: Args, params: ParticleParams) -> Result<JValue, JError> {
        let mut args = args.function_args.into_iter();
        let service_id: String = Args::next("service_id", &mut args)?;
        Ok(self
            .services
            .get_interface(params.peer_scope, service_id, &params.id)
            .await?)
    }

    async fn add_alias(&self, args: Args, params: ParticleParams) -> Result<(), JError> {
        let mut args = args.function_args.into_iter();

        let alias: String = Args::next("alias", &mut args)?;
        let service_id: String = Args::next("service_id", &mut args)?;

        self.guard_protected(&params).await?;

        self.services
            .add_alias(
                params.peer_scope,
                alias.clone(),
                service_id.clone(),
                params.init_peer_id,
            )
            .await?;

        log::debug!(
            "Added alias {} for service {:?} {}",
            alias,
            params.peer_scope,
            service_id
        );

        Ok(())
    }

    async fn resolve_alias(&self, args: Args, params: ParticleParams) -> Result<JValue, JError> {
        let mut args = args.function_args.into_iter();
        let alias: String = Args::next("alias", &mut args)?;
        let service_id = self
            .services
            .resolve_alias(params.peer_scope, alias.clone(), &params.id)
            .await?;

        log::debug!(
            "Resolved alias {} to service {:?} {}",
            alias,
            params.peer_scope,
            service_id
        );

        Ok(JValue::String(service_id))
    }

    async fn resolve_alias_opt(
        &self,
        args: Args,
        params: ParticleParams,
    ) -> Result<JValue, JError> {
        let mut args = args.function_args.into_iter();
        let alias: String = Args::next("alias", &mut args)?;
        let service_id_opt = self
            .services
            .resolve_alias(params.peer_scope, alias, &params.id)
            .await
            .map(|id| vec![JValue::String(id)])
            .unwrap_or_default();

        Ok(Array(service_id_opt))
    }

    async fn get_service_info(&self, args: Args, params: ParticleParams) -> Result<JValue, JError> {
        let mut args = args.function_args.into_iter();
        let service_id_or_alias: String = Args::next("service_id_or_alias", &mut args)?;
        let info = self
            .services
            .get_service_info(params.peer_scope, service_id_or_alias, &params.id)
            .await?;

        Ok(json!(Service::from(&info, self.scopes.clone())))
    }

    fn kademlia(&self) -> &KademliaApi {
        self.connectivity.as_ref()
    }

    fn connection_pool(&self) -> &ConnectionPoolApi {
        self.connectivity.as_ref()
    }

    async fn service_mem_stats(
        &self,
        args: Args,
        params: ParticleParams,
    ) -> Result<JValue, JError> {
        let mut args = args.function_args.into_iter();
        let service_id_or_alias: String = Args::next("service_id", &mut args)?;

        self.services
            .get_service_mem_stats(params.peer_scope, service_id_or_alias, &params.id)
            .await
            .map(Array)
    }

    async fn service_stat(&self, args: Args, params: ParticleParams) -> Result<JValue, JError> {
        let mut args = args.function_args.into_iter();
        let service_id_or_alias: String = Args::next("service_id", &mut args)?;
        // Resolve aliases; also checks that the requested service exists.
        let service_id = self
            .services
            .to_service_id(params.peer_scope, service_id_or_alias, &params.id)
            .await?;
        let metrics = self
            .services
            .metrics
            .as_ref()
            .ok_or_else(|| JError::new("Service stats collection is disabled"))?;
        if let Some(result) = metrics.builtin.read(&service_id) {
            Ok(json!({
                "status": true,
                "error": "",
                "result": vec![result],
            }))
        } else {
            Ok(json!({
                "status": false,
                "error": format!("No stats were collected for the `{service_id}` service"),
                "result": [],
            }))
        }
    }

    fn sign(&self, args: Args, params: ParticleParams) -> Result<JValue, JError> {
        let tetraplets = args.tetraplets;
        let mut args = args.function_args.into_iter();

        // The `sign` is allowed only for a **worker** to sign on **worker** and for a **host** to sign on **host**.
        // The manager is able to sign only on the host.
        let init_peer_scope = self.scopes.scope(params.init_peer_id);
        let is_manager_on_host = self.scopes.is_management(params.init_peer_id)
            && matches!(params.peer_scope, PeerScope::Host);
        if !init_peer_scope.map_or(is_manager_on_host, |result| params.peer_scope == result) {
            return Err(JError::new(format!(
                "peer '{}' is not allowed to sign data on '{}'",
                params.init_peer_id,
                self.scopes.to_peer_id(params.peer_scope)
            )));
        }

        let result: Result<JValue, JError> = try {
            let data: Vec<u8> = Args::next("data", &mut args)?;

            let tetraplet = tetraplets.first().map(|v| v.as_slice());
            if let Some([t]) = tetraplet {
                if self.scopes.scope(PeerId::from_str(&t.peer_pk)?).is_err() {
                    return Err(JError::new(format!(
                        "data is expected to be produced by service 'registry' on peer '{}', was from peer '{}'",
                        self.scopes.get_host_peer_id(), t.peer_pk
                    )));
                }

                let duplet = (t.service_id.as_str(), t.function_name.as_str());
                let metadata_bytes = ("registry", "get_record_metadata_bytes");
                let record_bytes = ("registry", "get_record_bytes");

                if duplet != record_bytes && duplet != metadata_bytes {
                    return Err(JError::new(format!(
                        "data is expected to result from a call to 'registry.get_record_bytes' or 'registry.get_record_metadata_bytes', was from '{}.{}'",
                        t.service_id, t.function_name
                    )));
                }

                if !t.lens.is_empty() {
                    return Err(JError::new(
                        "lens for data tetraplet is expected to be empty",
                    ));
                }
            } else {
                return Err(JError::new(format!("expected tetraplet for a scalar argument, got tetraplet for an array: {tetraplet:?}, tetraplets")));
            }

            let keypair = self.key_storage.get_keypair(params.peer_scope).unwrap(); //TODO: fix unwrap
            json!(keypair.sign(&data)?.to_vec())
        };

        match result {
            Ok(sig) => Ok(json!({
                "success": true,
                "error": [],
                "signature": vec![sig]
            })),

            Err(error) => Ok(json!({
                "success": false,
                "error": vec![JValue::from(error)],
                "signature": []
            })),
        }
    }

    fn verify(&self, args: Args, params: ParticleParams) -> Result<JValue, JError> {
        let mut args = args.function_args.into_iter();
        let signature: Vec<u8> = Args::next("signature", &mut args)?;
        let data: Vec<u8> = Args::next("data", &mut args)?;
        let pk = self
            .key_storage
            .get_keypair(params.peer_scope)
            .ok_or(JError::new(format!(
                "Not found key pair for scope {:?}",
                params.peer_scope
            )))?
            .public();
        let signature = Signature::from_bytes(pk.get_key_format(), signature);

        Ok(JValue::Bool(pk.verify(&data, &signature).is_ok()))
    }

    fn get_peer_id(&self, params: ParticleParams) -> Result<JValue, JError> {
        let peer_id = match params.peer_scope {
            PeerScope::WorkerId(worker_id) => worker_id.to_string(),
            PeerScope::Host => self.scopes.get_host_peer_id().to_string(),
        };

        Ok(JValue::String(peer_id))
    }
    fn vault_put(&self, args: Args, params: ParticleParams) -> Result<JValue, JError> {
        let mut args = args.function_args.into_iter();
        let data: String = Args::next("data", &mut args)?;
        let name = uuid();
        let current_peer_id = self.scopes.to_peer_id(params.peer_scope);
        let virtual_path = self
            .services
            .vault
            .put(current_peer_id, &params, name, &data)?;

        Ok(JValue::String(virtual_path.display().to_string()))
    }

    fn vault_cat(&self, args: Args, params: ParticleParams) -> Result<JValue, JError> {
        let mut args = args.function_args.into_iter();
        let path: String = Args::next("path", &mut args)?;
        let current_peer_id = self.scopes.to_peer_id(params.peer_scope);
        self.services
            .vault
            .cat(current_peer_id, &params, Path::new(&path))
            .map(JValue::String)
            .map_err(|_| JError::new(format!("Error reading vault file `{path}`")))
    }

    async fn guard_protected(&self, particle: &ParticleParams) -> Result<(), JError> {
        if self.is_worker_spell(particle).await
            || self.scopes.is_host(particle.init_peer_id)
            || self.scopes.is_management(particle.init_peer_id)
        {
            Ok(())
        } else {
            Err(JError::new(
                "This function is only available to the host or worker spells",
            ))
        }
    }

    // Check that the particle is from a worker spell from a worker installed on the host
    // 1. Particle ID must be in spell_{spell_id}_{n} format
    // 2. init_peer_id must be a local peer id for the host (either host id or a worker id)
    // 3. There must be a spell service with alias "worker-spell" on the init_peer_id
    // 4. The spell service must have the same spell_id as in the particle and be of type "spell"
    async fn is_worker_spell(&self, particle: &ParticleParams) -> bool {
        let result: Option<_> = try {
            let local_scope = self.scopes.scope(particle.init_peer_id).ok()?;
            let spell_id = ParticleParams::get_spell_id(&particle.id)?;
            let (worker_service, _) = self
                .services
                .get_service(local_scope, "worker-spell".to_string(), &particle.id)
                .await
                .ok()?;
            worker_service.service_type == ServiceType::Spell
                && worker_service.service_id == spell_id
        };
        result.unwrap_or(false)
    }

    fn allow_only_worker(&self, params: &ParticleParams) -> Result<WorkerId, JError> {
        match params.peer_scope {
            PeerScope::WorkerId(worker_id) => {
                let is_worker = params.init_peer_id == worker_id.into();
                let worker_creator = self.workers.get_worker_creator(worker_id)?;
                let is_worker_creator = params.init_peer_id == worker_creator;
                if !is_worker && !is_worker_creator {
                    Err(JError::new(format!("Failed to create VM on {worker_id:?}, can be installed by worker creator {worker_creator} or worker itself {worker_id}")))
                } else {
                    Ok(worker_id)
                }
            }
            PeerScope::Host => Err(JError::new("This function is only available for workers")),
        }
    }

    // Create a VM from the image
    // - `image` - path to the image in the *vault*
    // Returns the name of the created VM
    // Throws on errors
    async fn create_vm(&self, args: Args, params: ParticleParams) -> Result<JValue, JError> {
        let worker_id = self.allow_only_worker(&params)?;
        let mut function_args = args.function_args.into_iter();

        let image_arg: String = Args::next("image", &mut function_args)?;
        let image_arg: PathBuf = image_arg.into();

        let vault_image = self
            .services
            .vault
            .to_real_path(worker_id.into(), &params, image_arg.as_path())
            .map_err(|_| JError::new(format!("Image {} not found", image_arg.display())))?;

        let vm_name = self
            .workers
            .create_vm(worker_id, vault_image.as_path())
            .await
            .map_err(|err| JError::new(format!("Failed to create vm: {err}")))?;

        Ok(JValue::String(vm_name))
    }

    // Stop a VM assigned to a Worker if any
    // Throws on errors
    fn stop_vm(&self, _args: Args, params: ParticleParams) -> Result<JValue, JError> {
        let worker_id = self.allow_only_worker(&params)?;
        self.workers
            .stop_vm(worker_id)
            .map_err(|err| JError::new(err.to_string()))?;
        Ok(JValue::Null)
    }

    // Start a VM assigned to a Worker if any
    // Throws on errors
    fn start_vm(&self, _args: Args, params: ParticleParams) -> Result<JValue, JError> {
        let worker_id = self.allow_only_worker(&params)?;
        self.workers
            .start_vm(worker_id)
            .map_err(|err| JError::new(err.to_string()))?;
        Ok(JValue::Null)
    }

    // Reboot a VM assigned to a Worker if any
    // Throws on errors
    fn reboot_vm(&self, _args: Args, params: ParticleParams) -> Result<JValue, JError> {
        let worker_id = self.allow_only_worker(&params)?;
        self.workers
            .reboot_vm(worker_id)
            .map_err(|err| JError::new(err.to_string()))?;
        Ok(JValue::Null)
    }

    // Reset a VM assigned to a Worker if any
    // Throws on errors
    fn reset_vm(&self, _args: Args, params: ParticleParams) -> Result<JValue, JError> {
        let worker_id = self.allow_only_worker(&params)?;
        self.workers
            .reset_vm(worker_id)
            .map_err(|err| JError::new(err.to_string()))?;
        Ok(JValue::Null)
    }

    // Reset a VM assigned to a Worker if any
    // Throws on errors
    // Note: it's okay to allow anyone to get the VM status
    fn status_vm(&self, _args: Args, params: ParticleParams) -> Result<JValue, JError> {
        let worker_id = match params.peer_scope {
            PeerScope::WorkerId(worker_id) => worker_id,
            PeerScope::Host => {
                return Err(JError::new("This function is only available on workers"));
            }
        };
        let status = self
            .workers
            .status_vm(worker_id)
            .map_err(|err| JError::new(err.to_string()))?;
        Ok(JValue::String(status.to_string()))
    }
}

fn make_module_config(args: Args) -> Result<JValue, JError> {
    use toml_utils::table;

    let mut args = args.function_args.into_iter();

    let name = Args::next("name", &mut args)?;
    // These are not used anymore, keep them for backward compatibility, because args are positional
    let _mem_pages_count: Option<u32> = Args::next_opt("mem_pages_count", &mut args)?;
    let _max_heap_size: Option<String> = Args::next_opt("max_heap_size", &mut args)?;

    let logger_enabled = Args::next_opt("logger_enabled", &mut args)?;
    // These are not used anymore, keep them for backward compatibility, because args are positional
    let _preopened_files: Option<Vec<String>> = Args::next_opt("preopened_files", &mut args)?;
    let envs = Args::next_opt("envs", &mut args)?.map(table);
    let mapped_dirs = Args::next_opt("mapped_dirs", &mut args)?.map(table);
    let mounted_binaries = Args::next_opt("mounted_binaries", &mut args)?.map(table);
    let logging_mask = Args::next_opt("logging_mask", &mut args)?;

    let config = NamedModuleConfig {
        name,
        load_from: None,
        file_name: None,
        config: ModuleConfig {
            logger_enabled,
            wasi: Some(WASIConfig { envs, mapped_dirs }),
            mounted_binaries,
            logging_mask,
        },
    };

    let config = serde_json::to_value(config)
        .map_err(|err| JError::new(format!("Error serializing config to JSON: {err}")))?;

    Ok(config)
}

fn parse_from_str<T>(
    field: &'static str,
    mut args: &mut impl Iterator<Item = JValue>,
) -> Result<Option<T>, JError>
where
    T: FromStr + for<'a> Deserialize<'a>,
    <T as FromStr>::Err: std::error::Error + 'static,
{
    #[derive(thiserror::Error, Debug)]
    #[error("Error while deserializing field {field_name}")]
    struct Error<E: std::error::Error> {
        field_name: String,
        #[source]
        err: E,
    }

    #[derive(serde::Deserialize, Debug)]
    #[serde(untagged)]
    pub enum Either<T> {
        String(String),
        Target(T),
    }

    let number: Option<Either<T>> = Args::next_opt(field, &mut args)?;

    if number.is_none() {
        return Ok(None);
    }

    number
        .map(|i| match i {
            Either::String(s) => Ok(s.parse::<T>()?),
            Either::Target(n) => Ok(n),
        })
        .transpose()
        .map_err(|err: T::Err| {
            Error {
                field_name: field.to_string(),
                err,
            }
            .into()
        })
}

#[derive(Debug, Serialize)]
struct Service {
    pub id: String,
    pub blueprint_id: String,
    pub service_type: ServiceType,
    #[serde(serialize_with = "peer_id::serde::serialize")]
    pub owner_id: PeerId,
    pub aliases: Vec<String>,
    #[serde(serialize_with = "peer_id::serde::serialize")]
    pub worker_id: PeerId,
}

impl Service {
    fn from(service_info: &ServiceInfo, peer_scopes: PeerScopes) -> Self {
        let worker_id: PeerId = match service_info.peer_scope {
            PeerScope::WorkerId(worker_id) => worker_id.into(),
            PeerScope::Host => peer_scopes.get_host_peer_id(),
        };

        Service {
            id: service_info.id.clone(),
            blueprint_id: service_info.blueprint_id.clone(),
            service_type: service_info.service_type.clone(),
            owner_id: service_info.owner_id,
            aliases: service_info.aliases.clone(),
            worker_id,
        }
    }
}

#[cfg(test)]
mod prop_tests {
    use prop::collection::vec;
    use proptest::arbitrary::StrategyFor;
    use proptest::collection::{SizeRange, VecStrategy};
    use proptest::prelude::*;
    use serde_json::json;

    use particle_args::Args;

    use crate::builtins::make_module_config;

    prop_compose! {
      /// Generates ByteSize strings
      fn heap_size
        ()
        // FIXME: limit is 100k GB because ByteSize can't handle exabytes. terabytes and petabytes are missing for the same reason.
        (n in prop::option::of(0..100_000), si in "(?i)([kmg]i)?B")
        -> Vec<String>
      {
        n.map(|n| vec![format!("{n} {si}")]).unwrap_or_default()
      }
    }

    /// Wraps value into AIR-style option (vec of length 0 or 1)
    pub fn air_opt<T: Strategy>(element: T) -> proptest::collection::VecStrategy<T> {
        vec(element, 0..1)
    }

    /// Generates an associative array of strings of a given size
    pub fn assoc_vec(size: impl Into<SizeRange>) -> VecStrategy<VecStrategy<StrategyFor<String>>> {
        vec(vec(any::<String>(), 2..=2), size)
    }

    proptest! {
        #[test]
        fn module_config(
            name in any::<String>(),
            mem_pages in air_opt(any::<u32>()),
            logger_enabled in air_opt(proptest::bool::ANY),
            heap in heap_size(),
            preopened_files in air_opt(any::<String>()),
            envs in air_opt(assoc_vec(0..10)),
            mapped_dirs in air_opt(assoc_vec(0..10)),
            mounted_binaries in air_opt(assoc_vec(0..10)),
            logging_mask in air_opt(any::<i32>()),
        ) {
            let mem_pages: Vec<u32> = mem_pages;
            let heap: Vec<String> = heap;
            let preopened_files: Vec<String> = preopened_files;
            let envs: Vec<Vec<Vec<String>>> = envs;
            let mapped_dirs: Vec<Vec<Vec<String>>> = mapped_dirs;
            let mounted_binaries: Vec<Vec<Vec<String>>> = mounted_binaries;
            let logging_mask: Vec<i32> = logging_mask;

            let args = vec![
                json!(name),              // required: name
                json!(mem_pages),         // mem_pages_count = optional: None
                json!(heap),              // optional: max_heap_size
                json!(logger_enabled),    // optional: logger_enabled
                json!(preopened_files),   // optional: preopened_files
                json!(envs),              // optional: envs
                json!(mapped_dirs),       // optional: mapped_dirs
                json!(mounted_binaries),  // optional: mounted_binaries
                json!(logging_mask),      // optional: logging_mask
            ];
            let args = Args {
                service_id: "".to_string(),
                function_name: "".to_string(),
                function_args: args,
                tetraplets: vec![],
            };

            let config = make_module_config(args).expect("parse config via make_module_config");
            let config_name = config.get("name").and_then(|n| n.as_str()).expect("'name' field in the config");
            prop_assert_eq!(config_name, name);
        }
    }
}
