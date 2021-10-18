/*
 * Copyright 2021 Fluence Labs Limited
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

use std::collections::HashMap;
use std::sync::Arc;
use std::task::Poll;

use avm_server::CallRequestParams;
use futures::channel::mpsc::{TrySendError, UnboundedReceiver, UnboundedSender};
use libp2p::futures::StreamExt;
use serde_json::Value as JValue;

use host_closure::{Args, JError};
use particle_protocol::Particle;

use crate::particle_functions::FunctionApiError::{SendAdd, SendRemove};
use futures::future::BoxFuture;

type ParticleId = String;
type FunctionDuplet = (String, String);
pub type Function = Arc<Box<dyn FnMut(Args, Particle) -> BoxFuture<Result<JValue, JError>>>>;

enum Command {
    Add(ParticleId, FunctionDuplet, Function),
    Remove(ParticleId),
}

#[derive(Clone)]
pub struct ParticleFunctions {
    map: HashMap<ParticleId, HashMap<FunctionDuplet, Function>>,
    inlet: UnboundedReceiver<Command>,
}

impl ParticleFunctions {
    pub fn new(inlet: UnboundedReceiver<Command>) -> Self {
        Self {
            map: <_>::default(),
            inlet,
        }
    }

    /// Check if there are new commands and execute them
    pub fn poll(&mut self) {
        // TODO: check for Ok(None) and report that the channel has closed?
        while let Ok(Some(command)) = self.inlet.try_next() {
            match command {
                Command::Add(id, duplet, function) => {
                    let functions = self.map.entry(id).or_default();
                    functions.insert(duplet, function);
                }
                Command::Remove(id) => self.map.remove(&id),
            }
        }
    }

    pub fn get(&mut self, id: &ParticleId) -> Option<&mut HashMap<FunctionDuplet, Function>> {
        self.map.get_mut(id)
    }
}

pub struct ParticleFunctionsApi {
    inlet: UnboundedSender<Command>,
}

pub enum FunctionApiError {
    SendAdd,
    SendRemove,
}

impl ParticleFunctionsApi {
    pub fn new(inlet: UnboundedSender<Command>) -> Self {
        Self { inlet }
    }

    pub fn add(
        &self,
        id: ParticleId,
        function_name: FunctionDuplet,
        function: Function,
    ) -> Result<(), FunctionApiError> {
        self.inlet
            .unbounded_send(Command::Add(id, function_name, function))
            .map_err(|_| SendAdd)
    }

    pub fn remove(&self, id: ParticleId) -> Result<(), FunctionApiError> {
        self.inlet
            .unbounded_send(Command::Remove(id))
            .map_err(|_| SendRemove)
    }
}
