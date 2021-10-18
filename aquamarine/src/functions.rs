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

use async_std::sync::Arc;
use avm_server::CallServiceResult;
use futures::future::BoxFuture;
use futures::stream::FuturesUnordered;
use serde_json::Value as JValue;

use host_closure::{Args, JError};
use particle_protocol::Particle;

type ParticleId = String;
type FunctionDuplet = (String, String);
pub type Function = Arc<Box<dyn FnMut(Args, Particle) -> BoxFuture<Result<JValue, JError>>>>;

pub struct Functions<F> {
    function_calls: FuturesUnordered<(u32, CallServiceResult)>,
    particle_functions: HashMap<FunctionDuplet, Function>,
    host_functions: F,
}

impl<F> Functions<F> {
    pub fn new(host_functions: F) -> Self {}
}
