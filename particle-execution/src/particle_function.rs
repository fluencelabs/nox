/*
 * Nox Fluence Peer
 *
 * Copyright (C) 2024 Fluence DAO
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation version 3 of the
 * License.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
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
