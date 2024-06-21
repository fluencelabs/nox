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

use connection_pool::ConnectionPoolApi;
use kademlia::KademliaApi;
use particle_args::Args;
use particle_execution::{FunctionOutcome, ParticleFunction, ParticleParams, ServiceFunction};

use crate::builtins::CustomService;
use crate::Builtins;

#[async_trait]
impl<C> ParticleFunction for Builtins<C>
where
    C: Clone + Send + Sync + 'static + AsRef<KademliaApi> + AsRef<ConnectionPoolApi>,
{
    async fn call(&self, args: Args, particle: ParticleParams) -> FunctionOutcome {
        Builtins::call(self, args, particle).await
    }

    async fn extend(
        &self,
        service: String,
        functions: HashMap<String, ServiceFunction>,
        fallback: Option<ServiceFunction>,
    ) {
        self.custom_services.write().await.insert(
            service,
            CustomService {
                functions,
                fallback,
            },
        );
    }

    async fn remove(&self, service: &str) {
        self.custom_services
            .write()
            .await
            .remove(service)
            .map(|hm| (hm.functions, hm.fallback));
    }
}
