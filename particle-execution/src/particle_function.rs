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

use async_std::sync::Mutex;
use std::collections::HashMap;
use std::sync::Arc;

use futures::future::BoxFuture;
use futures::FutureExt;

use particle_args::Args;

use crate::{FunctionOutcome, ParticleParams};

pub type Output<'a> = BoxFuture<'a, FunctionOutcome>;

pub type ServiceFunction =
    Box<dyn FnMut(Args, ParticleParams) -> Output<'static> + 'static + Send + Sync>;

pub trait ParticleFunction: 'static + Send + Sync {
    fn call(&self, args: Args, particle: ParticleParams) -> Output<'_>;
    fn extend(&mut self, service: String, functions: HashMap<String, Mutex<ServiceFunction>>);
    fn remove(&mut self, service: &str) -> Option<HashMap<String, Mutex<ServiceFunction>>>;
}

pub trait ParticleFunctionMut: 'static + Send + Sync {
    fn call_mut(&mut self, args: Args, particle: ParticleParams) -> Output<'_>;
}

pub trait ParticleFunctionStatic: 'static + Send + Sync {
    fn call(&self, args: Args, particle: ParticleParams) -> Output<'static>;
}

impl<F: ParticleFunction> ParticleFunctionStatic for Arc<F> {
    fn call(self: &Arc<F>, args: Args, particle: ParticleParams) -> Output<'static> {
        let this = self.clone();
        async move { ParticleFunction::call(this.as_ref(), args, particle).await }.boxed()
    }
}
