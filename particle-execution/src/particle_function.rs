/*
 * Copyright 2024 Fluence DAO
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

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

use futures::future::BoxFuture;
use particle_args::Args;

use crate::{FunctionOutcome, ParticleParams};

pub type Output<'a> = BoxFuture<'a, FunctionOutcome>;

pub type ServiceFunctionMut =
    Box<dyn FnMut(Args, ParticleParams) -> Output<'static> + 'static + Send + Sync>;
pub type ServiceFunctionImmut =
    Box<dyn Fn(Args, ParticleParams) -> Output<'static> + 'static + Send + Sync>;

pub enum ServiceFunction {
    Mut(tokio::sync::Mutex<ServiceFunctionMut>),
    Immut(ServiceFunctionImmut),
}

impl ServiceFunction {
    pub async fn call(&self, args: Args, particle: ParticleParams) -> FunctionOutcome {
        match self {
            ServiceFunction::Mut(f) => {
                let mut func = f.lock().await;
                func(args, particle).await
            }
            ServiceFunction::Immut(f) => f(args, particle).await,
        }
    }
}

impl From<ServiceFunctionImmut> for ServiceFunction {
    fn from(f: ServiceFunctionImmut) -> Self {
        ServiceFunction::Immut(f)
    }
}

impl From<ServiceFunctionMut> for ServiceFunction {
    fn from(f: ServiceFunctionMut) -> Self {
        ServiceFunction::Mut(tokio::sync::Mutex::new(f))
    }
}

#[async_trait]
pub trait ParticleFunction: 'static + Send + Sync {
    async fn call(&self, args: Args, particle: ParticleParams) -> FunctionOutcome;
    async fn extend(
        &self,
        service: String,
        functions: HashMap<String, ServiceFunction>,
        fallback: Option<ServiceFunction>,
    );
    async fn remove(&self, service: &str);
}

pub trait ParticleFunctionMut: 'static + Send + Sync {
    fn call_mut(&mut self, args: Args, particle: ParticleParams) -> Output<'_>;
}

#[async_trait]
pub trait ParticleFunctionStatic: Clone + 'static + Send + Sync {
    async fn call(&self, args: Args, particle: ParticleParams) -> FunctionOutcome;
    async fn extend(
        &self,
        service: String,
        functions: HashMap<String, ServiceFunction>,
        fallback: Option<ServiceFunction>,
    );
    async fn remove(&self, service: &str);
}

#[async_trait]
impl<F: ParticleFunction> ParticleFunctionStatic for Arc<F> {
    async fn call(self: &Arc<F>, args: Args, particle: ParticleParams) -> FunctionOutcome {
        let this = self.clone();
        ParticleFunction::call(this.as_ref(), args, particle).await
    }

    async fn extend(
        self: &Arc<F>,
        service: String,
        functions: HashMap<String, ServiceFunction>,
        fallback: Option<ServiceFunction>,
    ) {
        ParticleFunction::extend(self.as_ref(), service, functions, fallback).await;
    }

    async fn remove(self: &Arc<F>, service: &str) {
        ParticleFunction::remove(self.as_ref(), service).await;
    }
}
