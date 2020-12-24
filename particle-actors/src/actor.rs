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

use crate::invoke::{parse_outcome, ExecutionError};

use aquamarine_vm::AquamarineVM;
use particle_protocol::Particle;

use async_std::{pin::Pin, task};
use futures::{future::BoxFuture, Future, FutureExt};
use libp2p::PeerId;
use log::LevelFilter;
use serde_json::json;
use std::{
    collections::VecDeque,
    fmt::Debug,
    task::{Context, Poll, Waker},
};

pub(super) type Fut = BoxFuture<'static, FutResult>;

pub struct FutResult {
    pub vm: AquamarineVM,
    pub effects: Vec<ActorEvent>,
}

pub enum ActorEvent {
    Forward { particle: Particle, target: PeerId },
}

pub struct Actor {
    future: Option<Fut>,
    // TODO: why keep particle here?
    #[allow(dead_code)]
    particle: Particle,
    mailbox: VecDeque<Particle>,
    waker: Option<Waker>,
}

impl Actor {
    pub fn new(particle: Particle) -> Self {
        Self {
            future: None,
            particle,
            mailbox: <_>::default(),
            waker: <_>::default(),
        }
    }

    #[allow(dead_code)]
    pub fn particle(&self) -> &Particle {
        &self.particle
    }

    pub fn ingest(&mut self, particle: Particle) {
        self.mailbox.push_back(particle);
        self.wake();
    }

    /// Polls actor for result on previously ingested particle
    pub fn poll_completed(&mut self, cx: &mut Context<'_>) -> Poll<FutResult> {
        self.waker = Some(cx.waker().clone());

        // Poll self.future
        let future = self
            .future
            .take()
            .map(|mut fut| (Pin::new(&mut fut).poll(cx), fut));

        match future {
            // If future is ready, return effects and vm
            Some((Poll::Ready(r), _)) => Poll::Ready(r),
            o => {
                // Either keep pending future or keep it None
                self.future = o.map(|t| t.1);
                Poll::Pending
            }
        }
    }

    /// Provide actor with new `vm` to execute particles, if there are any.
    ///
    /// If actor is in the middle of executing previous particle, vm is returned
    /// If actor's mailbox is empty, vm is returned
    pub fn poll_next(&mut self, vm: AquamarineVM, cx: &mut Context<'_>) -> Poll<AquamarineVM> {
        self.waker = Some(cx.waker().clone());

        // Return vm if previous particle is still executing
        if self.future.is_some() {
            return Poll::Ready(vm);
        }

        match self.mailbox.pop_front() {
            Some(p) => {
                // Take ownership of vm to process particle
                self.future = Self::execute(p, vm, cx.waker().clone()).into();
                Poll::Pending
            }
            // Mailbox is empty, return vm
            None => Poll::Ready(vm),
        }
    }

    fn execute(p: Particle, mut vm: AquamarineVM, waker: Waker) -> Fut {
        task::spawn_blocking(move || {
            log::info!("Executing particle {}", p.id);

            let result = vm.call(
                p.init_peer_id.to_string(),
                &p.script,
                p.data.clone(),
                &p.id,
            );
            if let Err(err) = &result {
                log::warn!("Error executing particle {:#?}: {}", p, err)
            }

            let effects = match parse_outcome(result) {
                Ok((data, targets)) if targets.len() > 0 => {
                    #[rustfmt::skip]
                    log::debug!("Particle {} executed, will be sent to {} targets", p.id, targets.len());
                    let particle = Particle {
                        data,
                        ..p
                    };
                    targets
                        .into_iter()
                        .map(|target| ActorEvent::Forward {
                            particle: particle.clone(),
                            target,
                        })
                        .collect::<Vec<_>>()
                }
                Ok((data, _)) => {
                    log::warn!("Executed particle {}, next_peer_pks is empty. Won't send anywhere", p.id);
                    if log::max_level() >= LevelFilter::Debug {
                        let data = String::from_utf8_lossy(data.as_slice());
                        log::debug!("particle {} next_peer_pks = [], data: {}", p.id, data);
                    }
                    vec![]
                }
                Err(ExecutionError::AquamarineError(err)) => {
                    log::warn!("Error executing particle {:#?}: {}", p, err);
                    vec![]
                }
                Err(err @ ExecutionError::StepperOutcome { .. }) => {
                    log::warn!("Error executing script: {}", err);
                    // Return error to the init peer id
                    vec![protocol_error(p, err)]
                },
                Err(err @ ExecutionError::InvalidResultField { .. }) => {
                    log::warn!("Error parsing outcome for particle {:#?}: {}", p, err);
                    // Return error to the init peer id
                    vec![protocol_error(p, err)]
                }
            };

            waker.wake();

            FutResult { vm, effects }
        })
        .boxed()
    }

    fn wake(&self) {
        if let Some(waker) = &self.waker {
            waker.wake_by_ref();
        }
    }
}

fn protocol_error(mut particle: Particle, err: impl Debug) -> ActorEvent {
    let error = format!("{:?}", err);
    particle.data = json!({"protocol!error": error, "data": base64::encode(particle.data)})
        .to_string()
        .into_bytes();
    // Return error to the init peer id
    ActorEvent::Forward {
        target: particle.init_peer_id.clone(),
        particle,
    }
}
