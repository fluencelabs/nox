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

use crate::providers::ProviderError::{KeyPoisoned, Poisoned};

use control_macro::ok_get;
use fluence_libp2p::{peerid_serializer, PeerId};
use host_closure::{closure_opt, Args, Closure};
use json_utils::err_as_value;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::ops::Deref;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

/*macro_rules! map_get {
    ($providers:expr, $key:expr) => {{
        {
            let map = read_map(&$providers)?;
            map.get(&$key)
        }
    }};
}
*/

type ProviderMap = Arc<RwLock<ProviderMapInner>>;
type ProviderMapInner = HashMap<String, RwLock<HashSet<Provider>>>;

#[derive(Debug, Serialize, Deserialize, Hash, PartialEq, Eq)]
/// A representation of a service provider
pub struct Provider {
    #[serde(with = "peerid_serializer")]
    /// Peer id of the node that acts as a provider
    peer: PeerId,
    /// Optional service_id
    /// If defined, an app service is considered to be a provider
    /// If None, the whole node (i.e., `peer`) is considered to be a provider
    service_id: Option<String>,
}

#[derive(Debug, Default)]
/// Thread-safe storage of providers
// TODO: remove on TTL?
pub struct ProviderRepository {
    providers: ProviderMap,
}

#[derive(Serialize, Debug)]
/// Errors that happen during `ProviderRepository` reading or writing
pub enum ProviderError<'a> {
    /// The whole repository was poisoned by a panic
    Poisoned,
    /// Only a single key was poisoned by a panic
    KeyPoisoned(&'a str),
}

impl<'a> From<ProviderError<'a>> for Value {
    fn from(err: ProviderError<'a>) -> Self {
        err_as_value(err)
    }
}

impl ProviderRepository {
    /// Creates a closure that
    /// takes `key` as a parameter, and returns an array of `Provider` for that key
    pub fn get_providers(&self) -> Closure {
        let providers = self.providers.clone();
        closure_opt(move |mut args| {
            let key: String = Args::next("key", &mut args)?;
            println!("get_providers {:?}", key);
            let providers = read_map(&providers)?;
            let providers = ok_get!(providers.get(&key));
            let providers = read_set(&providers, &key)?;

            let providers = json!(providers.deref());
            Ok(Some(providers))
        })
    }

    /// Creates a closure that
    /// takes a `Provider` instance as an argument, adds it to the `ProviderRepository`,
    /// and returns nothing
    pub fn add_provider(&self) -> Closure {
        let providers = self.providers.clone();
        closure_opt(move |mut args| {
            let key: String = Args::next("key", &mut args)?;
            let provider: Provider = Args::next("provider", &mut args)?;

            println!("add_providers {:?} {:?}", key, provider);

            let providers_r = read_map(&providers)?;
            let empty = providers_r.get(&key).is_none();
            drop(providers_r);
            if empty {
                let mut providers = write_map(&providers)?;
                providers.insert(key.to_string(), <_>::default());
            }

            let providers = read_map(&providers)?;
            let providers = ok_get!(providers.get(&key));
            let mut providers = write_set(&providers, &key)?;
            providers.insert(provider);

            Ok(None)
        })
    }
}

fn read_map<'a>(
    providers: &'a ProviderMap,
) -> Result<RwLockReadGuard<'a, ProviderMapInner>, ProviderError<'static>> {
    providers.read().map_err(|_| Poisoned)
}

fn write_map(
    providers: &ProviderMap,
) -> Result<RwLockWriteGuard<'_, ProviderMapInner>, ProviderError<'static>> {
    providers.write().map_err(|_| Poisoned)
}

// fn map_get<'a, 'b>(
//     providers: &'a ProviderMap,
//     key: &'b str,
// ) -> Result<Option<&'a RwLock<HashSet<Provider>>>, ProviderError<'b>> {
//     Ok(read_map(providers)?.get(key))
// }

fn read_set<'a, 'b>(
    providers: &'a RwLock<HashSet<Provider>>,
    key: &'b str,
) -> Result<RwLockReadGuard<'a, HashSet<Provider>>, ProviderError<'b>> {
    providers.read().map_err(|_| KeyPoisoned(key))
}

fn write_set<'a, 'b>(
    providers: &'a RwLock<HashSet<Provider>>,
    key: &'b str,
) -> Result<RwLockWriteGuard<'a, HashSet<Provider>>, ProviderError<'b>> {
    providers.write().map_err(|_| KeyPoisoned(key))
}
