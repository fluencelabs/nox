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

use std::ops::Div;
use std::sync::Arc;
use std::{
    collections::{HashMap, VecDeque},
    task::{Context, Poll},
};

use futures::task::Waker;

/// For tests, mocked time is used
#[cfg(test)]
use mock_time::now_ms;
use particle_execution::{ParticleFunctionStatic, ParticleParams};
use particle_protocol::Particle;
use peer_metrics::ParticleExecutorMetrics;
/// Get current time from OS
#[cfg(not(test))]
use real_time::now_ms;

use crate::actor::{Actor, ActorPoll};
use crate::aqua_runtime::AquaRuntime;
use crate::deadline::Deadline;
use crate::error::AquamarineApiError;
use crate::particle_effects::NetworkEffects;
use crate::particle_functions::{Function, Functions};
use crate::vm_pool::VmPool;

pub struct Plumber<RT: AquaRuntime, F> {
    events: VecDeque<Result<NetworkEffects, AquamarineApiError>>,
    actors: HashMap<String, Actor<RT, F>>,
    vm_pool: VmPool<RT>,
    builtins: Arc<F>,
    waker: Option<Waker>,
    metrics: Option<ParticleExecutorMetrics>,
}

impl<RT: AquaRuntime, F: ParticleFunctionStatic> Plumber<RT, F> {
    pub fn new(vm_pool: VmPool<RT>, builtins: F, metrics: Option<ParticleExecutorMetrics>) -> Self {
        Self {
            vm_pool,
            builtins: Arc::new(builtins),
            events: <_>::default(),
            actors: <_>::default(),
            waker: <_>::default(),
            metrics,
        }
    }

    /// Receives and ingests incoming particle: creates a new actor or forwards to the existing mailbox
    pub fn ingest(&mut self, particle: Particle, function: Option<Function>) {
        self.wake();

        let deadline = Deadline::from(&particle);
        if deadline.is_expired(now_ms()) {
            log::info!("Particle {} is expired, ignoring", particle.id);
            self.events
                .push_back(Err(AquamarineApiError::ParticleExpired {
                    particle_id: particle.id,
                }));
            return;
        }

        let builtins = &self.builtins;
        let actor = self.actors.entry(particle.id.clone()).or_insert_with(|| {
            let params = ParticleParams::clone_from(&particle);
            let functions = Functions::new(params, builtins.clone());
            Actor::new(&particle, functions)
        });

        actor.ingest(particle);
        if let Some(function) = function {
            actor.set_function(function);
        }
    }

    pub fn poll(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Result<NetworkEffects, AquamarineApiError>> {
        self.waker = Some(cx.waker().clone());

        self.vm_pool.poll(cx);

        if let Some(event) = self.events.pop_front() {
            return Poll::Ready(event);
        }

        // Gather effects and put VMs back
        let mut effects = vec![];
        let mut mailbox_size = 0;
        for actor in self.actors.values_mut() {
            if let Poll::Ready(result) = actor.poll_completed(cx) {
                effects.push((result.effects, result.stats));
                self.vm_pool.put_vm(result.vm);
            }
            mailbox_size += actor.mailbox_size();
        }

        // Remove expired actors
        if let Some(mut vm) = self.vm_pool.get_vm() {
            let now = now_ms();
            self.actors.retain(|particle_id, actor| {
                // if actor hasn't yet expired or is still executing, keep it
                // TODO: if actor is expired, cancel execution and return VM back to pool
                //       https://github.com/fluencelabs/fluence/issues/1212
                if !actor.is_expired(now) || actor.is_executing() {
                    return true; // keep actor
                }

                log::debug!("Reaping particle's actor {}", particle_id);
                // cleanup files and dirs after particle processing (vault & prev_data)
                // TODO: do not pass vm https://github.com/fluencelabs/fluence/issues/1216
                if let Err(err) = actor.cleanup(particle_id, &mut vm) {
                    log::warn!(
                        "Error cleaning up after particle {}: {:?}",
                        particle_id,
                        err
                    )
                }
                false // remove actor
            });

            self.vm_pool.put_vm(vm);
        }

        // Execute next messages
        let mut stats = vec![];
        for actor in self.actors.values_mut() {
            if let Some(vm) = self.vm_pool.get_vm() {
                match actor.poll_next(vm, cx) {
                    ActorPoll::Vm(vm) => self.vm_pool.put_vm(vm),
                    ActorPoll::Executing(mut s) => stats.append(&mut s),
                }
            } else {
                // TODO: calculate deviations from normal mailbox_size
                if mailbox_size > 11 {
                    log::warn!(
                        "{} particles waiting in mailboxes, but all interpreters busy",
                        mailbox_size
                    );
                }
                break;
            }
        }

        self.meter(|m| {
            for (_, stat) in &effects {
                // count particle interpretations
                if stat.success {
                    m.interpretation_successes.inc();
                } else {
                    m.interpretation_failures.inc();
                }

                let time = stat.interpretation_time.as_secs_f64();
                m.interpretation_time_sec.observe(time);
                if let Some(new_data_len) = stat.new_data_len {
                    // divide interpretation time by data size
                    let normalized = time.div(new_data_len as f64);
                    m.normalized_interpretation_time_sec.observe(normalized);
                }

                m.total_actors_mailbox.set(mailbox_size as u64);
                m.alive_actors.set(self.actors.len() as u64);
                m.free_interpreters.set(self.vm_pool.free_vms() as u64);
            }

            for stat in &stats {
                m.service_call(stat.success, stat.builtin, stat.run_time)
            }
        });

        // Turn effects into events, and buffer them
        for (effect, _) in effects {
            self.events.push_back(Ok(effect));
        }

        // Return a new event if there is some
        if let Some(event) = self.events.pop_front() {
            return Poll::Ready(event);
        }

        Poll::Pending
    }

    fn wake(&self) {
        if let Some(waker) = &self.waker {
            waker.wake_by_ref();
        }
    }

    fn meter<U, FF: Fn(&ParticleExecutorMetrics) -> U>(&self, f: FF) {
        self.metrics.as_ref().map(f);
    }
}

/// Implements `now` by taking number of non-leap seconds from `Utc::now()`
mod real_time {
    #[allow(dead_code)]
    pub fn now_ms() -> u64 {
        (chrono::Utc::now().timestamp() * 1000) as u64
    }
}

#[cfg(test)]
// mod tests {
//     use crate::actor::Deadline;
//     use crate::plumber::mock_time::set_mock_time;
//     use crate::plumber::{is_expired, now, real_time};
//     use crate::Plumber;
//
//     use particle_protocol::Particle;
//
//     use futures::task::noop_waker_ref;
//     use std::{sync::Arc, task::Context};
//
//     fn plumber() -> Plumber {
//         let config = <_>::default();
//         let host_closure = Arc::new(|| panic!("no host_closure no no no"));
//         Plumber::new(config, host_closure)
//     }
//
//     fn particle(ts: u64, ttl: u32) -> Particle {
//         let mut particle = Particle::default();
//         particle.timestamp = ts;
//         particle.ttl = ttl;
//
//         particle
//     }
//
//     fn context() -> Context<'static> {
//         Context::from_waker(noop_waker_ref())
//     }
//
//     /// Checks that expired actor will be removed
//     #[test]
//     fn remove_expired() {
//         set_mock_time(real_time::now());
//
//         let mut plumber = plumber();
//
//         let particle = particle(now(), 1);
//         let deadline = Deadline::from(&particle);
//         assert!(!deadline.is_expired(now()));
//
//         plumber.ingest(particle);
//
//         assert_eq!(plumber.actors.len(), 1);
//         let mut cx = context();
//         assert!(plumber.poll(&mut cx).is_pending());
//         assert_eq!(plumber.actors.len(), 1);
//
//         set_mock_time(now() + 2);
//         assert!(plumber.poll(&mut cx).is_pending());
//         assert_eq!(plumber.actors.len(), 0);
//     }
//
//     /// Checks that expired particle won't create an actor
//     #[test]
//     fn ignore_expired() {
//         set_mock_time(real_time::now());
//
//         let mut plumber = plumber();
//         let particle = particle(now() - 100, 99);
//         assert!(is_expired(now(), &particle));
//
//         plumber.ingest(particle);
//
//         assert_eq!(plumber.actors.len(), 0);
//
//         // Check actor doesn't appear after poll somehow
//         set_mock_time(now() + 1000);
//         assert!(plumber.poll(&mut context()).is_pending());
//         assert_eq!(plumber.actors.len(), 0);
//     }
// }

/// Code taken from https://blog.iany.me/2019/03/how-to-mock-time-in-rust-tests-and-cargo-gotchas-we-met/
/// And then modified to use u64 instead of `SystemTime`
#[cfg(test)]
pub mod mock_time {
    #![allow(dead_code)]

    use std::cell::RefCell;

    thread_local! {
        static MOCK_TIME: RefCell<u64> = RefCell::new(0);
    }

    pub fn now_ms() -> u64 {
        MOCK_TIME.with(|cell| cell.borrow().clone())
    }

    pub fn set_mock_time(time: u64) {
        MOCK_TIME.with(|cell| *cell.borrow_mut() = time);
    }
}
