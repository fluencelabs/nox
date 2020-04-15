/*
 * Copyright 2019 Fluence Labs Limited
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

use crate::misc::enqueue_result::Enqueued;
use std::collections::hash_map::Entry;
use std::collections::{HashMap, VecDeque};
use std::hash::Hash;

pub struct WaitingQueues<K, V> {
    map: HashMap<K, VecDeque<V>>,
}

impl<K: Eq + Hash, V> WaitingQueues<K, V> {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    // Inserts `item` in the queue associated with `key`
    pub fn enqueue(&mut self, key: K, item: V) -> Enqueued {
        use std::iter::FromIterator;

        match self.map.entry(key) {
            Entry::Occupied(mut entry) => {
                entry.get_mut().push_back(item);
                Enqueued::Existing
            }
            Entry::Vacant(entry) => {
                entry.insert(VecDeque::from_iter(std::iter::once(item)));
                Enqueued::New
            }
        }
    }

    // Removes queue associated with `key`
    pub fn remove(&mut self, key: &K) -> impl Iterator<Item = V> {
        self.map.remove(key).into_iter().flatten()
    }

    // Removes items on key `k` that satisfy `remove` predicate
    // Returns removed items. Keeps other items by reinserting them to queue.
    pub fn remove_with<F>(&mut self, key: &K, remove: F) -> impl Iterator<Item = V>
    where
        F: FnMut(&V) -> bool,
    {
        self.map
            .get_mut(key)
            .map(|queue| {
                let (keep, remove) = queue.drain(..).partition::<Vec<_>, _>(remove);
                queue.extend(keep);
                remove.into_iter()
            })
            .into_iter()
            .flatten()
    }

    // Returns number of items on key `k`. Useful for debug lofs.
    pub fn count(&self, key: &K) -> usize {
        self.map.get(key).map_or(0, |q| q.len())
    }
}
