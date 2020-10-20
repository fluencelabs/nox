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

type ProviderMap = Arc<RwLock<ProviderMapInner>>;
type ProviderMapInner = HashMap<String, RwLock<HashSet<Provider>>>;

#[derive(Debug, Serialize, Deserialize, Hash, PartialEq, Eq)]
/// A representation of a service provider
pub struct Provider {
    #[serde(with = "peerid_serializer")]
    /// Peer id of the node that acts as a provider
    pub peer: PeerId,
    /// Optional service_id
    /// If defined, an app service is considered to be a provider
    /// If None, the whole node (i.e., `peer`) is considered to be a provider
    pub service_id: Option<String>,
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
        let provider_map = self.providers.clone();
        closure_opt(move |mut args| {
            let key: String = Args::next("key", &mut args)?;
            let provider_map = map_ref(&provider_map)?;
            let provider_set = ok_get!(provider_map.get(&key));
            let provider_set = set_ref(&provider_set, &key)?;

            let provider_set = json!(provider_set.deref());
            Ok(Some(provider_set))
        })
    }

    /// Creates a closure that
    /// takes a `Provider` instance as an argument, adds it to the `ProviderRepository`,
    /// and returns nothing
    pub fn add_provider(&self) -> Closure {
        let provider_map = self.providers.clone();
        closure_opt(move |mut args| {
            let key: String = Args::next("key", &mut args)?;
            let provider: Provider = Args::next("provider", &mut args)?;

            // check if there are providers for that key
            let empty = {
                let provider_map = map_ref(&provider_map)?;
                provider_map.get(&key).is_none()
            };

            // if there's no providers for that key, insert empty collection into the map
            if empty {
                let mut provider_map = map_ref_mut(&provider_map)?;
                let provider_set = <_>::default();
                provider_map.insert(key.to_string(), provider_set);
            }

            let provider_map = map_ref(&provider_map)?;
            let provider_set = ok_get!(provider_map.get(&key));
            let mut provider_set = set_ref_mut(&provider_set, &key)?;
            provider_set.insert(provider);

            Ok(None)
        })
    }
}

/// Obtains a read lock on the `ProviderMap` (analogous to having a `&`)
fn map_ref<'a>(
    map: &'a ProviderMap,
) -> Result<RwLockReadGuard<'a, ProviderMapInner>, ProviderError<'static>> {
    map.read().map_err(|_| Poisoned)
}

/// Obtains a write lock on the `ProviderMap` (analogous to having a `&mut`)
fn map_ref_mut(
    map: &ProviderMap,
) -> Result<RwLockWriteGuard<'_, ProviderMapInner>, ProviderError<'static>> {
    map.write().map_err(|_| Poisoned)
}

/// Obtains a read lock on the providers set (analogous to having a `&`)
fn set_ref<'a, 'b>(
    set: &'a RwLock<HashSet<Provider>>,
    key: &'b str,
) -> Result<RwLockReadGuard<'a, HashSet<Provider>>, ProviderError<'b>> {
    set.read().map_err(|_| KeyPoisoned(key))
}

/// Obtains a write lock on the providers set (analogous to having a `&mut`)
fn set_ref_mut<'a, 'b>(
    set: &'a RwLock<HashSet<Provider>>,
    key: &'b str,
) -> Result<RwLockWriteGuard<'a, HashSet<Provider>>, ProviderError<'b>> {
    set.write().map_err(|_| KeyPoisoned(key))
}
