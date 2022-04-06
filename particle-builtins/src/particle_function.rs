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

use futures::FutureExt;

use connection_pool::ConnectionPoolApi;
use kademlia::KademliaApi;
use particle_args::Args;
use particle_execution::{ParticleFunction, ParticleFunctionOutput, ParticleParams};

use crate::Builtins;

impl<C> ParticleFunction for Builtins<C>
where
    C: Clone + Send + Sync + 'static + AsRef<KademliaApi> + AsRef<ConnectionPoolApi>,
{
    fn call(&self, args: Args, particle: ParticleParams) -> ParticleFunctionOutput<'_> {
        Builtins::call(self, args, particle).boxed()
    }
}
