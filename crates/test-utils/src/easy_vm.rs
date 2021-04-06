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

use aquamarine::{AquaRuntime, InterpreterOutcome, SendParticle, StepperEffects};
use particle_protocol::Particle;

use futures::{future::BoxFuture, FutureExt};
use itertools::Itertools;
use libp2p::PeerId;
use std::{convert::Infallible, task::Waker, time::Duration};

pub struct EasyVM {
    delay: Option<Duration>,
}

impl AquaRuntime for EasyVM {
    type Config = Option<Duration>;
    type Error = Infallible;

    fn create_runtime(
        delay: Option<Duration>,
        _: Waker,
    ) -> BoxFuture<'static, Result<Self, Self::Error>> {
        futures::future::ok(EasyVM { delay }).boxed()
    }

    fn into_effects(_: Result<InterpreterOutcome, Self::Error>, p: Particle) -> StepperEffects {
        StepperEffects {
            particles: vec![SendParticle {
                target: p.init_peer_id,
                particle: p,
            }],
        }
    }

    fn call(
        &mut self,
        init_user_id: PeerId,
        script: String,
        mut data: Vec<u8>,
        _particle_id: String,
    ) -> Result<InterpreterOutcome, Self::Error> {
        if let Some(delay) = self.delay {
            std::thread::sleep(delay);
        }

        let mut next_peer = init_user_id.to_string();
        if script.starts_with('!') {
            let next_peers = String::from_utf8_lossy(&data);
            dbg!(&next_peers);
            let mut next_peers = next_peers.split(",");
            next_peer = String::from(next_peers.next().unwrap());
            data = next_peers.join(",").into_bytes()
        }

        Ok(InterpreterOutcome {
            ret_code: 0,
            error_message: "".to_string(),
            data: data.into(),
            next_peer_pks: vec![next_peer],
        })
    }
}
