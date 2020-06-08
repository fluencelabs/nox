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

// Copyright 2019 Parity Technologies (UK) Ltd.
//
// Permission is hereby granted, free of charge, to any person obtaining a
// copy of this software and associated documentation files (the "Software"),
// to deal in the Software without restriction, including without limitation
// the rights to use, copy, modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software, and to permit persons to whom the
// Software is furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS
// OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
// FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.

use super::record::*;
use libp2p::kad::kbucket;
use libp2p::kad::record::store::*;
use libp2p::kad::record::{Key, ProviderRecord, Record};
use libp2p::kad::K_VALUE;
use libp2p::PeerId;
use smallvec::SmallVec;
use std::borrow::Cow;
use std::collections::{hash_map, hash_set, HashMap, HashSet};
use std::hash::Hash;
use std::iter;
use std::time::Instant;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MultiRecordKind {
    // Holds multiple (value, publisher) pairs, merge on insert
    MultiRecord,
    // Holds single (value, publisher) pair, replaced on insert
    // Currently isn't used
    SimpleRecord,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MultiRecord {
    /// Key of the record.
    pub key: Key,
    /// Map of value to publisher
    pub values: HashMap<Vec<u8>, PeerId>,
    /// The expiration time as measured by a local, monotonic clock.
    pub expires: Option<Instant>,
    pub kind: MultiRecordKind,
}

impl MultiRecord {
    // TODO: this could be simplified with linked hash map
    /// Merge `right` into the `left`, overwriting existing values,
    /// and keep up to `max_values` values.
    /// Values in `right` have higher priority. Intuition behind that is:
    ///     `right` values are "newer", more recent – they just came from network.
    ///     And the `left` values we already "had" stored in this MemoryStore.
    fn merge_maps<K: Hash + Eq, V>(
        left: &mut HashMap<K, V>,
        mut right: HashMap<K, V>,
        max_values: usize,
    ) {
        // Remove intersection; these will be overwritten
        right.keys().for_each(|k| {
            left.remove(k);
        });

        // Calculate total number of values.
        // NOTE: this can be done only after removing the intersection.
        let total = left.len() + right.len();
        // Calculate if there will be over `max_values` values
        let over_limit = total.checked_sub(max_values);
        if let Some(mut over_limit) = over_limit {
            // `keep` is a predicate that says whether to keep a record.
            // It will return 'false' until `over_limit` isn't 0, and decrement `over_limit` each time.
            // When `over_limit` is 0, there would be only `max_values` values in both maps combined.
            let mut keep = || {
                let delete = over_limit > 0;
                over_limit = over_limit.saturating_sub(1);
                !delete
            };
            // First, remove values from `left`: these have lower priority (they're "older")
            left.retain(|_, _| keep());
            // Then, if we're still over limit, remove values from `right`
            right.retain(|_, _| keep());
        }

        // Finally add `right` map values to the `left` map
        left.extend(right);
    }

    /// Merge `values` and `expires` from `mrec` to `self`.
    pub fn merge(&mut self, mrec: MultiRecord, max_values: usize) {
        debug_assert_eq!(self.key, mrec.key);

        Self::merge_maps(&mut self.values, mrec.values, max_values);

        // Take max of expiration times, and set it to `self.expires`.
        // `None` means the record will never expire.
        if let Some(current) = self.expires.as_ref() {
            // If `mrec.expires` is None, mark current record to never expire
            if mrec.expires.map_or(true, |their| current.le(&their)) {
                self.expires = mrec.expires;
            }
        }
    }

    /// Return whether this record is of `SimpleRecord` kind
    pub fn is_simple(&self) -> bool {
        match self.kind {
            MultiRecordKind::SimpleRecord => true,
            MultiRecordKind::MultiRecord => false,
        }
    }

    /// Remove values by specified publisher
    pub fn remove(&mut self, publisher: &PeerId) {
        self.values.retain(|_, p| p != publisher)
    }

    pub fn is_expired(&self, now: Instant) -> bool {
        self.expires.map_or(false, |t| now >= t)
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

/// In-memory implementation of a `RecordStore`.
pub struct MemoryStore {
    /// The identity of the peer owning the store.
    local_key: kbucket::Key<PeerId>,
    /// The configuration of the store.
    config: MemoryStoreConfig,
    /// The stored records.
    records: HashMap<Key, MultiRecord>,
    /// The stored provider records.
    providers: HashMap<Key, SmallVec<[ProviderRecord; K_VALUE.get()]>>,
    /// The set of all provider records for the node identified by `local_key`.
    ///
    /// Must be kept in sync with `providers`.
    provided: HashSet<ProviderRecord>,
}

/// Configuration for a `MemoryStore`.
pub struct MemoryStoreConfig {
    /// The maximum number of records.
    pub max_records: usize,
    /// The maximum size of record values, in bytes.
    pub max_value_bytes: usize,
    /// The maximum number of providers stored for a key.
    ///
    /// This should match up with the chosen replication factor.
    pub max_providers_per_key: usize,
    /// The maximum number of provider records for which the
    /// local node is the provider.
    pub max_provided_keys: usize,
    /// The maximum number of values stored in MultiRecord
    pub max_values_per_multi_record: usize,
}

impl Default for MemoryStoreConfig {
    fn default() -> Self {
        Self {
            max_records: 1024,
            max_value_bytes: 65 * 1024,
            max_provided_keys: 1024,
            max_providers_per_key: K_VALUE.get(),
            max_values_per_multi_record: 100,
        }
    }
}

impl MemoryStore {
    /// Creates a new `MemoryRecordStore` with a default configuration.
    pub fn new(local_id: PeerId) -> Self {
        Self::with_config(local_id, Default::default())
    }

    /// Creates a new `MemoryRecordStore` with the given configuration.
    pub fn with_config(local_id: PeerId, config: MemoryStoreConfig) -> Self {
        MemoryStore {
            local_key: kbucket::Key::new(local_id),
            config,
            records: HashMap::default(),
            provided: HashSet::default(),
            providers: HashMap::default(),
        }
    }

    // TODO: limit on number of records
    fn record_to_multirecord(record: Record) -> Result<MultiRecord> {
        try_to_multirecord(record).map_err(|e| {
            log::error!("Can't parse multi record from DHT record: {:?}", e);
            Error::MaxRecords // TODO custom error?
        })
    }
}

impl<'a> RecordStore<'a> for MemoryStore {
    type RecordsIter = std::vec::IntoIter<Cow<'a, Record>>;

    #[allow(clippy::type_complexity)]
    type ProvidedIter = iter::Map<
        hash_set::Iter<'a, ProviderRecord>,
        fn(&'a ProviderRecord) -> Cow<'a, ProviderRecord>,
    >;

    fn get(&'a self, k: &Key) -> Option<Cow<'_, Record>> {
        // TODO: Performance? Was Cow::Borrowed, now Cow::Owned, and lot's of .clone()'s :(
        self.records
            .get(k)
            .map(|mrec| reduce_multirecord(mrec.clone()))
            .map(Cow::Owned)
    }

    fn put(&'a mut self, r: Record) -> Result<()> {
        if r.value.len() >= self.config.max_value_bytes {
            log::warn!(
                "value of record for key {:?} is too large: {}",
                bs58::encode(r.key).into_string(),
                r.value.len()
            );
            return Err(Error::ValueTooLarge);
        }

        let num_records = self.records.len();

        let key = r.key.clone();
        let mrec = Self::record_to_multirecord(r)?;

        match self.records.entry(key) {
            hash_map::Entry::Occupied(mut e) => {
                if mrec.is_simple() {
                    // Replace if mrec is of simple kind
                    e.insert(mrec);
                } else {
                    e.get_mut()
                        .merge(mrec, self.config.max_values_per_multi_record)
                }
            }
            hash_map::Entry::Vacant(e) => {
                if num_records >= self.config.max_records {
                    return Err(Error::MaxRecords);
                }
                e.insert(mrec);
            }
        }

        Ok(())
    }

    fn remove(&'a mut self, k: &Key) {
        if let Some(rec) = self.records.get_mut(k) {
            if rec.is_expired(Instant::now()) {
                // Whole record expired, remove it
                self.records.remove(k);
            } else {
                // Remove ourselves from multirecord
                rec.remove(self.local_key.preimage());
                if rec.is_empty() {
                    // If record is now empty, remove it
                    self.records.remove(k);
                }
            }
        }
    }

    fn records(&'a self) -> Self::RecordsIter {
        let vec = self
            .records
            .iter()
            .map(|(_, mrec)| reduce_multirecord(mrec.clone()))
            .map(Cow::Owned)
            .collect::<Vec<_>>();

        vec.into_iter()
    }

    fn add_provider(&'a mut self, record: ProviderRecord) -> Result<()> {
        let num_keys = self.providers.len();

        // Obtain the entry
        let providers = match self.providers.entry(record.key.clone()) {
            e @ hash_map::Entry::Occupied(_) => e,
            e @ hash_map::Entry::Vacant(_) => {
                if self.config.max_provided_keys == num_keys {
                    return Err(Error::MaxProvidedKeys);
                }
                e
            }
        }
        .or_insert_with(Default::default);

        if let Some(i) = providers.iter().position(|p| p.provider == record.provider) {
            // In-place update of an existing provider record.
            providers.as_mut()[i] = record;
        } else {
            // It is a new provider record for that key.
            let local_key = self.local_key.clone();
            let key = kbucket::Key::new(record.key.clone());
            let provider = kbucket::Key::new(record.provider.clone());
            if let Some(i) = providers.iter().position(|p| {
                let pk = kbucket::Key::new(p.provider.clone());
                provider.distance(&key) < pk.distance(&key)
            }) {
                // Insert the new provider.
                if local_key.preimage() == &record.provider {
                    self.provided.insert(record.clone());
                }
                providers.insert(i, record);
                // Remove the excess provider, if any.
                if providers.len() > self.config.max_providers_per_key {
                    if let Some(p) = providers.pop() {
                        self.provided.remove(&p);
                    }
                }
            } else if providers.len() < self.config.max_providers_per_key {
                // The distance of the new provider to the key is larger than
                // the distance of any existing provider, but there is still room.
                if local_key.preimage() == &record.provider {
                    self.provided.insert(record.clone());
                }
                providers.push(record);
            }
        }
        Ok(())
    }

    fn providers(&'a self, key: &Key) -> Vec<ProviderRecord> {
        self.providers
            .get(key)
            .map_or_else(Vec::new, |ps| ps.clone().into_vec())
    }

    fn provided(&'a self) -> Self::ProvidedIter {
        self.provided.iter().map(Cow::Borrowed)
    }

    fn remove_provider(&'a mut self, key: &Key, provider: &PeerId) {
        if let hash_map::Entry::Occupied(mut e) = self.providers.entry(key.clone()) {
            let providers = e.get_mut();
            if let Some(i) = providers.iter().position(|p| &p.provider == provider) {
                let p = providers.remove(i);
                self.provided.remove(&p);
            }
            if providers.is_empty() {
                e.remove();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fluence_libp2p::RandomPeerId;
    use multihash::{wrap, Code, Multihash};
    use quickcheck::*;
    use rand::Rng;
    use std::time::{Duration, Instant};

    #[derive(Clone, Debug)]
    struct NewProviderRecord(ProviderRecord);
    #[derive(Clone, Debug)]
    struct NewRecord(Record);
    #[derive(Clone, Debug)]
    struct NewKey(Key);
    #[derive(Clone, Debug)]
    struct NewKBucketKey(kbucket::Key<PeerId>);

    impl Arbitrary for NewKBucketKey {
        fn arbitrary<G: Gen>(_: &mut G) -> NewKBucketKey {
            NewKBucketKey(kbucket::Key::from(PeerId::random()))
        }
    }

    impl Arbitrary for NewKey {
        fn arbitrary<G: Gen>(_: &mut G) -> NewKey {
            let hash = rand::thread_rng().gen::<[u8; 32]>();
            NewKey(Key::from(wrap(Code::Sha2_256, &hash)))
        }
    }

    impl Arbitrary for NewRecord {
        fn arbitrary<G: Gen>(g: &mut G) -> NewRecord {
            fn gen_rec<G: Gen>(g: &mut G) -> Record {
                Record {
                    key: NewKey::arbitrary(g).0,
                    value: Vec::arbitrary(g),
                    publisher: Some(PeerId::random()),
                    expires: if g.gen() {
                        Some(Instant::now() + Duration::from_secs(g.gen_range(0, 60)))
                    } else {
                        None
                    },
                }
            }

            let rec = if g.gen() {
                gen_rec(g)
            } else {
                // 25 chosen without any reason, you can change it up to max_values_per_multi_record
                let max_values = 25;
                let mut rec = try_to_multirecord(gen_rec(g)).expect("reduced");
                for _ in 1..g.gen_range(1, max_values) {
                    let mut new_rec = try_to_multirecord(gen_rec(g)).expect("reduced");
                    new_rec.key = rec.key.clone();
                    rec.merge(new_rec, max_values);
                }

                reduce_multirecord(rec)
            };

            NewRecord(rec)
        }
    }

    impl Arbitrary for NewProviderRecord {
        fn arbitrary<G: Gen>(g: &mut G) -> NewProviderRecord {
            NewProviderRecord(ProviderRecord {
                key: NewKey::arbitrary(g).0,
                provider: RandomPeerId::random(),
                expires: if g.gen() {
                    Some(Instant::now() + Duration::from_secs(g.gen_range(0, 60)))
                } else {
                    None
                },
            })
        }
    }

    fn random_multihash() -> Multihash {
        wrap(Code::Sha2_256, &rand::thread_rng().gen::<[u8; 32]>())
    }

    fn distance(r: &ProviderRecord) -> kbucket::Distance {
        kbucket::Key::new(r.key.clone()).distance(&kbucket::Key::new(r.provider.clone()))
    }

    #[test]
    fn put_get_remove_unexpired_record() {
        fn prop(r: NewRecord) {
            let r = r.0;
            let key = &r.key;

            let mut mrec = try_to_multirecord(r.clone()).expect("reduce ok");
            // Set record to never expire
            mrec.expires = None;
            let reduced = reduce_multirecord(mrec.clone());
            let mut store = MemoryStore::new(PeerId::random());
            assert!(store.put(reduced.clone()).is_ok());

            let from_store = store.get(key).expect("contains").into_owned();
            assert_eq!(from_store.key, reduced.key);
            assert_eq!(from_store.publisher, reduced.publisher);
            assert_eq!(from_store.expires, reduced.expires);

            store.remove(key);
            let after_remove = store.get(key);

            let local_values: Vec<_> = mrec
                .values
                .values()
                .filter(|&v| v == store.local_key.preimage())
                .collect();

            if local_values.len() == mrec.values.len() {
                // If all values stored in multirecord were published by store.local_key
                // then record would be removed completely
                assert_eq!(after_remove, None)
            } else {
                // There were non-local values in mrecord, can't be empty
                let after_remove = after_remove.expect("non empty").into_owned();
                let after_remove = try_to_multirecord(after_remove).expect("must be a multirecord");
                // only local values should be deleted
                assert_eq!(
                    after_remove.values.len(),
                    mrec.values.len() - local_values.len(),
                    "local values must be deleted"
                );
            }
        }
        quickcheck(prop as fn(_))
    }

    #[test]
    fn put_get_remove_expired_record() {
        fn prop(r: NewRecord) {
            let r = r.0;
            let key = &r.key;

            let mut mrec = try_to_multirecord(r.clone()).expect("reduce ok");
            // Set record to expire right now
            let now = Instant::now();
            mrec.expires = Some(now);
            let reduced = reduce_multirecord(mrec.clone());

            let mut store = MemoryStore::new(PeerId::random());
            assert!(store.put(reduced.clone()).is_ok());

            let from_store = store.get(key).expect("contains").into_owned();

            assert_eq!(from_store.key, reduced.key);
            assert_eq!(from_store.publisher, reduced.publisher);
            // We can't compare expires directly due to serialization quirks
            assert!(from_store.expires.expect("expires defined") >= now);

            store.remove(key);
            assert!(store.get(key).is_none());
        }
        quickcheck(prop as fn(_))
    }

    #[test]
    fn add_get_remove_provider() {
        fn prop(r: NewProviderRecord) {
            let r = r.0;
            let mut store = MemoryStore::new(PeerId::random());
            assert!(store.add_provider(r.clone()).is_ok());
            assert!(store.providers(&r.key).contains(&r));
            store.remove_provider(&r.key, &r.provider);
            assert!(!store.providers(&r.key).contains(&r));
        }
        quickcheck(prop as fn(_))
    }

    #[test]
    fn providers_ordered_by_distance_to_key() {
        fn prop(providers: Vec<NewKBucketKey>) -> bool {
            let mut store = MemoryStore::new(PeerId::random());
            let key = Key::from(random_multihash());

            let providers = providers.into_iter().map(|v| v.0).collect::<Vec<_>>();
            let mut records = providers
                .into_iter()
                .map(|p| ProviderRecord::new(key.clone(), p.into_preimage()))
                .collect::<Vec<_>>();

            for r in &records {
                assert!(store.add_provider(r.clone()).is_ok());
            }

            records.sort_by(|r1, r2| distance(r1).cmp(&distance(r2)));
            records.truncate(store.config.max_providers_per_key);

            records == store.providers(&key).to_vec()
        }

        quickcheck(prop as fn(_) -> _)
    }

    #[test]
    fn provided() {
        let id = RandomPeerId::random();
        let mut store = MemoryStore::new(id.clone());
        let key = random_multihash();
        let rec = ProviderRecord::new(key, id.clone());
        assert!(store.add_provider(rec.clone()).is_ok());
        assert_eq!(
            vec![Cow::Borrowed(&rec)],
            store.provided().collect::<Vec<_>>()
        );
        store.remove_provider(&rec.key, &id);
        assert_eq!(store.provided().count(), 0);
    }

    #[test]
    fn update_provider() {
        let mut store = MemoryStore::new(PeerId::random());
        let key = random_multihash();
        let prv = RandomPeerId::random();
        let mut rec = ProviderRecord::new(key, prv);
        assert!(store.add_provider(rec.clone()).is_ok());
        assert_eq!(vec![rec.clone()], store.providers(&rec.key).to_vec());
        rec.expires = Some(Instant::now());
        assert!(store.add_provider(rec.clone()).is_ok());
        assert_eq!(vec![rec.clone()], store.providers(&rec.key).to_vec());
    }

    #[test]
    fn max_provided_keys() {
        let mut store = MemoryStore::new(PeerId::random());
        for _ in 0..store.config.max_provided_keys {
            let key = random_multihash();
            let prv = RandomPeerId::random();
            let rec = ProviderRecord::new(key, prv);
            let _ = store.add_provider(rec);
        }
        let key = random_multihash();
        let prv = RandomPeerId::random();
        let rec = ProviderRecord::new(key, prv);
        match store.add_provider(rec) {
            Err(Error::MaxProvidedKeys) => {}
            _ => panic!("Unexpected result"),
        }
    }

    #[test]
    // check that `max_values_per_multi_record` holds
    fn multirecord_limit() {
        let mut store = MemoryStore::new(PeerId::random());
        let limit = store.config.max_values_per_multi_record;

        let key: Key = random_multihash().to_vec().into();
        let rec = || Record {
            key: key.clone(),
            value: random_multihash().to_vec(),
            publisher: Some(PeerId::random()),
            expires: None,
        };
        for _ in 0..limit {
            store.put(rec()).expect("put success")
        }

        let check_mrec = |mrec: &MultiRecord| {
            assert_eq!(mrec.kind, MultiRecordKind::MultiRecord);
            assert_eq!(mrec.values.len(), limit);
        };

        let mrec = store.records.get(&key).expect("get record");
        check_mrec(mrec);
        for _ in 0..limit {
            store.put(rec()).expect("put record");
            let mrec = store.records.get(&key).expect("get record");
            check_mrec(mrec);
        }
    }

    #[test]
    // check that values inside multirecord are deduplicated
    fn multirecord_unique_values() {
        let mut store = MemoryStore::new(PeerId::random());

        let key: Key = random_multihash().to_vec().into();
        for i in 0..100 {
            let value = (i % 10).to_string().as_bytes().to_vec();
            let rec = Record {
                key: key.clone(),
                value,
                publisher: Some(PeerId::random()),
                expires: None,
            };
            store.put(rec).expect("put success");
        }

        let mrec = store.records.get(&key).expect("get record");
        assert_eq!(mrec.kind, MultiRecordKind::MultiRecord);
        assert_eq!(mrec.values.len(), 10);

        let publisher = RandomPeerId::random();
        for i in 0..10 {
            let value = i.to_string().as_bytes().to_vec();
            let rec = Record {
                key: key.clone(),
                value,
                publisher: Some(publisher.clone()),
                expires: None,
            };
            store.put(rec).expect("put success");
        }

        let mrec = store.records.get(&key).expect("get record");
        assert_eq!(mrec.kind, MultiRecordKind::MultiRecord);
        assert_eq!(mrec.values.len(), 10);
        for p in mrec.values.values() {
            assert_eq!(p, &publisher);
        }
    }
}
