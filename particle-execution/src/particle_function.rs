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

use futures::future::BoxFuture;
use serde_json::Value as JValue;

use particle_args::{Args, JError};

use crate::ParticleParams;

pub type Output = BoxFuture<'static, Result<Option<JValue>, JError>>;
pub trait ParticleFunction: 'static + Send + Sync {
    fn call(&self, args: Args, particle: ParticleParams) -> Output;
    fn call_mut(&mut self, args: Args, particle: ParticleParams) -> Output;
}
