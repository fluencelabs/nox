/*
 * Copyright 2020 Fluence Labs Limited
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

#![allow(dead_code)]

use libp2p::PeerId;

#[derive(Debug)]
struct ParticleRouter {
    pub actors: Vec<Actor>,
}

#[derive(Debug)]
struct Actor {
    init_user_id: PeerId,
    state: ExecutionState,
    previous_particle: Particle,
}

impl Actor {
    pub fn new(init_user_id: PeerId) -> Self {
        Self {
            init_user_id,
            state: <_>::default(),
            previous_particle: <_>::default(),
        }
    }
}

#[derive(Default, Debug)]
struct ExecutionState {}

#[derive(Default, Debug)]
struct Particle {}
