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

use async_std::task::JoinHandle;
use futures::future::FusedFuture;

use futures::FutureExt;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

/// Holds handles to spawned tasks
pub struct NetworkTasks {
    /// Task that processes particles from particle stream
    pub particles: Option<JoinHandle<()>>,
    /// Task that reconnects to disconnected bootstraps
    pub reconnect_bootstraps: Option<JoinHandle<()>>,
    /// Task that runs Kademlia::bootstrap when enough bootstrap nodes have changed
    pub run_bootstrap: Option<JoinHandle<()>>,
    /// Task that runs particles after CallRequests are processed
    pub observations: Option<JoinHandle<()>>,
}

impl NetworkTasks {
    pub fn new(
        particles: JoinHandle<()>,
        reconnect_bootstraps: JoinHandle<()>,
        run_bootstrap: JoinHandle<()>,
        observations: JoinHandle<()>,
    ) -> Self {
        Self {
            particles: Some(particles),
            reconnect_bootstraps: Some(reconnect_bootstraps),
            run_bootstrap: Some(run_bootstrap),
            observations: Some(observations),
        }
    }

    pub async fn cancel(self) {
        if let Some(run_bootstrap) = self.run_bootstrap {
            run_bootstrap.cancel().await;
        };
        if let Some(reconnect_bootstraps) = self.reconnect_bootstraps {
            reconnect_bootstraps.cancel().await;
        };
        if let Some(particles) = self.particles {
            particles.cancel().await;
        };
        if let Some(observations) = self.observations {
            observations.cancel().await;
        }
    }
}

impl Future for NetworkTasks {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        poll_opt(&mut self.particles, cx);
        poll_opt(&mut self.reconnect_bootstraps, cx);
        poll_opt(&mut self.run_bootstrap, cx);
        poll_opt(&mut self.observations, cx);

        if self.is_terminated() {
            log::warn!("FuturesHandle terminated");
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

impl FusedFuture for NetworkTasks {
    fn is_terminated(&self) -> bool {
        self.particles.is_none()
            && self.reconnect_bootstraps.is_none()
            && self.run_bootstrap.is_none()
            && self.observations.is_none()
    }
}

/// Poll the future inside Option. If future is completed, set Option to None.
fn poll_opt(future: &mut Option<JoinHandle<()>>, cx: &mut Context<'_>) {
    let mut ready = false;
    if let Some(future) = future.as_mut() {
        if future.poll_unpin(cx).is_ready() {
            ready = true;
        }
    }
    if ready {
        future.take();
    }
}
