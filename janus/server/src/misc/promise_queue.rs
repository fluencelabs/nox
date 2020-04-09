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
use futures::channel::oneshot;
use futures_util::future::BoxFuture;
use oneshot::Canceled;
use oneshot::Sender;
use std::collections::{HashMap, VecDeque};

// TODO: maybe take Key type parameter
pub struct PromiseQueue<T> {
    promises: HashMap<String, VecDeque<Sender<T>>>,
}

impl<'a, T: Send + Clone + 'a> PromiseQueue<T> {
    pub fn new() -> Self {
        Self {
            promises: HashMap::new(),
        }
    }

    pub fn enqueue(&mut self, key: String) -> BoxFuture<'a, Result<T, Canceled>> {
        use std::collections::hash_map::Entry;
        use std::iter::FromIterator;

        let (sender, future) = oneshot::channel::<T>();

        match self.promises.entry(key) {
            Entry::Occupied(mut entry) => {
                entry.get_mut().push_back(sender);
                Enqueued::Existing
            }
            Entry::Vacant(entry) => {
                entry.insert(VecDeque::from_iter(std::iter::once(sender)));
                Enqueued::New
            }
        };

        Box::pin(future)
    }

    pub fn complete(&mut self, key: &String, value: T) -> Result<(), T> {
        match self.promises.remove(key) {
            Some(promises) => {
                for promise in promises {
                    promise.send(value.clone())?;
                }
            }
            None => {}
        }
        Ok(())
    }
}
